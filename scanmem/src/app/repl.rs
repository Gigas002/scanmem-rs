//! `rustyline`-backed REPL loop: read a line, parse it into a [`Command`], dispatch, print.

use libscanmem::error::ScanmemError;
use libscanmem::session::{Session, SessionOption};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use super::AppState;
use crate::commands::{Command, formatter, parser};

/// Runs the interactive loop against `state` until the user quits or stdin closes.
pub fn run(mut state: AppState) {
    let mut editor = DefaultEditor::new().expect("failed to initialize line editor");

    loop {
        match editor.readline("scanmem> ") {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = editor.add_history_entry(trimmed);

                match parser::parse(trimmed) {
                    Ok(Command::Quit) => break,
                    Ok(command) => println!("{}", dispatch(&mut state, command)),
                    Err(err) => eprintln!("error: {err}"),
                }
            }
            // Cooperatively abort an in-progress scan rather than terminating the REPL.
            Err(ReadlineError::Interrupted) => {
                if let Some(session) = state.session() {
                    session.request_stop();
                }
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("error: {err}");
                break;
            }
        }
    }
}

/// Executes one already-parsed `command` against `state`, returning the plain-text result.
///
/// [`Command::Quit`] is handled by [`run`] before reaching here.
pub(super) fn dispatch(state: &mut AppState, command: Command) -> String {
    match command {
        Command::Attach(pid) => attach(state, pid),
        Command::Scan(expr) => with_session(state, |session| {
            session.scan(&expr).map(formatter::scan_stats)
        }),
        Command::Snapshot => with_session(state, |session| {
            session.snapshot().map(formatter::scan_stats)
        }),
        Command::List(range) => list(state, range),
        Command::Dump { address, len } => with_session(state, |session| {
            session
                .read(address, len)
                .map(|bytes| formatter::dump(address, &bytes))
        }),
        Command::Write { address, value } => with_session(state, |session| {
            session.write(address, &value).map(|()| "ok".to_owned())
        }),
        Command::Delete(selector) => delete(state, &selector),
        Command::SetOption(option) => set_option(state, option),
        Command::Reset => {
            state.reset();
            "session reset".to_owned()
        }
        Command::Help => formatter::help(),
        Command::Quit => unreachable!("Command::Quit is handled by the caller"),
    }
}

fn attach(state: &mut AppState, pid: u32) -> String {
    let Some(pid) = rustix::process::Pid::from_raw(pid as i32) else {
        return "error: pid must not be zero".to_owned();
    };
    match state.attach(pid) {
        Ok(region_count) => format!(
            "attached to pid {}: {region_count} region(s)",
            pid.as_raw_pid()
        ),
        Err(err) => format!("error: {err}"),
    }
}

fn list(state: &AppState, range: Option<std::ops::Range<usize>>) -> String {
    let Some(session) = state.session() else {
        return not_attached();
    };
    match range {
        Some(range) => formatter::match_table(
            range.filter_map(|index| session.nth_match(index).map(|view| (index, view))),
        ),
        None => formatter::match_table(session.matches().enumerate()),
    }
}

fn delete(state: &mut AppState, selector: &str) -> String {
    let Some(session) = state.session_mut() else {
        return not_attached();
    };
    let size = session.matches().count();
    match libscanmem::sets::parse_index_set(selector, size) {
        Ok(indices) => {
            // Descending order so removing one match never invalidates the indices still queued.
            let addresses: Vec<usize> = indices
                .iter()
                .rev()
                .filter_map(|&index| session.nth_match(index))
                .map(|view| view.address)
                .collect();
            let deleted = addresses
                .into_iter()
                .map(|address| session.delete_in_range(address..address + 1))
                .sum();
            formatter::deleted(deleted)
        }
        Err(err) => format!("error: {err}"),
    }
}

fn set_option(state: &mut AppState, option: SessionOption) -> String {
    let Some(session) = state.session_mut() else {
        return not_attached();
    };
    session.set_option(option);
    "option updated".to_owned()
}

fn with_session(
    state: &mut AppState,
    f: impl FnOnce(&mut Session) -> Result<String, ScanmemError>,
) -> String {
    match state.session_mut() {
        Some(session) => match f(session) {
            Ok(text) => text,
            Err(err) => format!("error: {err}"),
        },
        None => not_attached(),
    }
}

fn not_attached() -> String {
    format!("error: {}", ScanmemError::NotAttached)
}
