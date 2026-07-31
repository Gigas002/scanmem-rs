//! `rustyline`-backed REPL loop: read a line, parse it into a [`Command`], dispatch, print.

use std::cell::Cell;
use std::rc::Rc;

use libscanmem::error::ScanmemError;
use libscanmem::session::{Session, SessionOption};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

use super::AppState;
use crate::commands::{Command, formatter, parser};

/// Verb names offered as first-word completions.
const VERBS: &[&str] = &[
    "pid", "attach", "scan", "snapshot", "list", "dump", "write", "delete", "option", "reset",
    "help", "quit", "exit",
];

/// `rustyline` helper wiring up tab-completion: verb names for the first word, then match
/// indices for `list`/`delete` once the session has recorded matches. Hinting/highlighting/
/// validation are left at their no-op defaults.
pub(super) struct ScanmemHelper {
    match_count: Rc<Cell<usize>>,
}

impl ScanmemHelper {
    pub(super) fn new(match_count: Rc<Cell<usize>>) -> Self {
        Self { match_count }
    }
}

impl Helper for ScanmemHelper {}

impl Hinter for ScanmemHelper {
    type Hint = String;
}

impl Highlighter for ScanmemHelper {}

impl Validator for ScanmemHelper {}

impl Completer for ScanmemHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        let (start, prefix) = current_word(line, pos);
        let candidates = if start == 0 {
            VERBS
                .iter()
                .filter(|verb| verb.starts_with(prefix))
                .map(|verb| (*verb).to_owned())
                .collect()
        } else if matches!(first_word(line), "list" | "delete") {
            (0..self.match_count.get())
                .map(|index| index.to_string())
                .filter(|candidate| candidate.starts_with(prefix))
                .collect()
        } else {
            Vec::new()
        };
        Ok((start, candidates))
    }
}

/// Returns the byte offset and text of the whitespace-delimited word ending at `pos`.
pub(super) fn current_word(line: &str, pos: usize) -> (usize, &str) {
    let start = line[..pos]
        .rfind(char::is_whitespace)
        .map_or(0, |index| index + 1);
    (start, &line[start..pos])
}

/// Returns the first whitespace-delimited word in `line`, or `""` if `line` is empty.
pub(super) fn first_word(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

/// Runs the interactive loop against `state` until the user quits or stdin closes.
pub fn run(mut state: AppState) {
    let match_count = Rc::new(Cell::new(0));
    let mut editor =
        Editor::<ScanmemHelper, DefaultHistory>::new().expect("failed to initialize line editor");
    editor.set_helper(Some(ScanmemHelper::new(Rc::clone(&match_count))));

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
                    Ok(command) => {
                        println!("{}", dispatch(&mut state, command));
                        update_match_count(&state, &match_count);
                    }
                    Err(err) => eprintln!("{}", formatter::error(err)),
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
                eprintln!("{}", formatter::error(err));
                break;
            }
        }
    }
}

/// Refreshes the completer's view of how many matches are recorded, if any session is attached.
fn update_match_count(state: &AppState, match_count: &Rc<Cell<usize>>) {
    let count = state
        .session()
        .map_or(0, |session| session.matches().count());
    match_count.set(count);
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
            session
                .write(address, &value)
                .map(|()| formatter::info("ok"))
        }),
        Command::Delete(selector) => delete(state, &selector),
        Command::SetOption(option) => set_option(state, option),
        Command::Reset => {
            state.reset();
            formatter::info("session reset")
        }
        Command::Help => formatter::help(),
        Command::Quit => unreachable!("Command::Quit is handled by the caller"),
    }
}

fn attach(state: &mut AppState, pid: u32) -> String {
    let Some(pid) = rustix::process::Pid::from_raw(pid as i32) else {
        return formatter::error("pid must not be zero");
    };
    match state.attach(pid) {
        Ok(region_count) => formatter::info(&format!(
            "attached to pid {}: {region_count} region(s)",
            pid.as_raw_pid()
        )),
        Err(err) => formatter::error(err),
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
        Err(err) => formatter::error(err),
    }
}

fn set_option(state: &mut AppState, option: SessionOption) -> String {
    let Some(session) = state.session_mut() else {
        return not_attached();
    };
    session.set_option(option);
    formatter::info("option updated")
}

fn with_session(
    state: &mut AppState,
    f: impl FnOnce(&mut Session) -> Result<String, ScanmemError>,
) -> String {
    match state.session_mut() {
        Some(session) => match f(session) {
            Ok(text) => text,
            Err(err) => formatter::error(err),
        },
        None => not_attached(),
    }
}

fn not_attached() -> String {
    formatter::error(ScanmemError::NotAttached)
}
