//! Application core: [`AppState`], [`Msg`], and [`update`] — a toolkit-independent state
//! machine. Only [`crate::settings::Settings`] crosses in from `main`; no CLI or raw config
//! types, and no `ratatui`/`crossterm` types anywhere in this module tree.

mod focus;
mod msg;
mod process_list;
mod state;

use std::process::ExitCode;

use libscanmem::error::ScanmemError;
use libscanmem::session::Session;
use libscanmem::value::Value;

pub use focus::Focus;
pub use msg::Msg;
pub use state::{AppState, CheatEntry, ProcessEntry, Status, StatusLevel};

use crate::settings::Settings;

/// Applies `msg` to `state`, calling into the attached [`Session`] as needed. This is the only
/// place `AppState` is mutated.
pub fn update(state: &mut AppState, msg: Msg) {
    let status = match msg {
        Msg::Attach(pid) => Some(attach(state, pid)),
        Msg::Detach => Some(detach(state)),
        Msg::Scan(expr) => Some(with_session(state, |session| {
            session
                .scan(&expr)
                .map(|stats| format!("{} match(es)", stats.matches))
        })),
        Msg::Snapshot => Some(with_session(state, |session| {
            session
                .snapshot()
                .map(|stats| format!("{} match(es)", stats.matches))
        })),
        Msg::ResetScan => Some(reset_scan(state)),
        Msg::Write { address, value } => Some(with_session(state, |session| {
            session.write(address, &value).map(|()| "ok".to_owned())
        })),
        Msg::AddCheat {
            address,
            description,
            value,
        } => Some(add_cheat(state, address, description, value)),
        Msg::RemoveCheat(index) => Some(remove_cheat(state, index)),
        Msg::ToggleFreeze(index) => Some(toggle_freeze(state, index)),
        Msg::EditCheatValue { index, value } => Some(edit_cheat_value(state, index, value)),
        Msg::RefreshProcessList => Some(refresh_process_list(state)),
        Msg::FilterProcesses(query) => {
            state.process_filter = query;
            state.process_selected = 0;
            None
        }
        Msg::ToggleSearch => {
            state.search_active = !state.search_active;
            None
        }
        Msg::SelectNext => {
            if state.focus == Focus::ProcessPicker {
                select_process(state, 1);
            }
            None
        }
        Msg::SelectPrev => {
            if state.focus == Focus::ProcessPicker {
                select_process(state, -1);
            }
            None
        }
        Msg::FocusNext => {
            state.focus = state.focus.next();
            None
        }
        Msg::FocusPrev => {
            state.focus = state.focus.prev();
            None
        }
        Msg::ShowHelp => {
            state.help_visible = !state.help_visible;
            None
        }
        Msg::Dismiss => {
            if state.help_visible {
                state.help_visible = false;
            } else if state.search_active {
                state.search_active = false;
                state.process_filter.clear();
                state.process_selected = 0;
            }
            None
        }
        Msg::Quit => {
            state.quit = true;
            None
        }
    };

    if let Some(status) = status {
        state.status = Some(status);
    }
}

fn attach(state: &mut AppState, pid: u32) -> Status {
    let Some(pid) = rustix::process::Pid::from_raw(pid as i32) else {
        return Status::error("pid must not be zero");
    };
    match state.attach(pid) {
        Ok(region_count) => Status::info(format!(
            "attached to pid {}: {region_count} region(s)",
            pid.as_raw_pid()
        )),
        Err(err) => Status::error(err.to_string()),
    }
}

fn detach(state: &mut AppState) -> Status {
    match state.detach() {
        Ok(()) => Status::info("detached"),
        Err(err) => Status::error(err.to_string()),
    }
}

fn reset_scan(state: &mut AppState) -> Status {
    match state.session.as_mut() {
        Some(session) => {
            session.delete_in_range(0..usize::MAX);
            Status::info("scan reset")
        }
        None => not_attached(),
    }
}

fn add_cheat(state: &mut AppState, address: usize, description: String, value: Value) -> Status {
    state.cheats.push(CheatEntry {
        address,
        description,
        value,
        frozen: false,
    });
    Status::info("cheat added")
}

fn remove_cheat(state: &mut AppState, index: usize) -> Status {
    if index >= state.cheats.len() {
        return cheat_index_out_of_range(index);
    }
    state.cheats.remove(index);
    Status::info("cheat removed")
}

fn toggle_freeze(state: &mut AppState, index: usize) -> Status {
    match state.cheats.get_mut(index) {
        Some(entry) => {
            entry.frozen = !entry.frozen;
            Status::info(if entry.frozen {
                "cheat frozen"
            } else {
                "cheat unfrozen"
            })
        }
        None => cheat_index_out_of_range(index),
    }
}

fn edit_cheat_value(state: &mut AppState, index: usize, value: Value) -> Status {
    let Some(entry) = state.cheats.get(index) else {
        return cheat_index_out_of_range(index);
    };
    let address = entry.address;
    let Some(session) = state.session.as_mut() else {
        return not_attached();
    };
    match session.write(address, &value) {
        Ok(()) => {
            state.cheats[index].value = value;
            Status::info("cheat value updated")
        }
        Err(err) => Status::error(err.to_string()),
    }
}

fn refresh_process_list(state: &mut AppState) -> Status {
    state.processes = process_list::list_processes();
    state.process_selected = 0;
    Status::info(format!("{} process(es)", state.processes.len()))
}

/// Moves `state.process_selected` by `delta` (`1` or `-1`), wrapping within the current
/// filtered-process count; a no-op if the filtered list is empty.
fn select_process(state: &mut AppState, delta: isize) {
    let len = state.filtered_processes().len();
    if len == 0 {
        return;
    }
    let current = state.process_selected as isize;
    state.process_selected = (current + delta).rem_euclid(len as isize) as usize;
}

fn with_session(
    state: &mut AppState,
    f: impl FnOnce(&mut Session) -> Result<String, ScanmemError>,
) -> Status {
    match state.session.as_mut() {
        Some(session) => match f(session) {
            Ok(text) => Status::info(text),
            Err(err) => Status::error(err.to_string()),
        },
        None => not_attached(),
    }
}

fn not_attached() -> Status {
    Status::error(ScanmemError::NotAttached.to_string())
}

fn cheat_index_out_of_range(index: usize) -> Status {
    Status::error(format!("cheat index {index} is out of range"))
}

/// Runs the application: attaches to `settings.pid` first if given, then hands off to the
/// `ratatui` shell.
#[cfg(feature = "tui")]
pub fn run(settings: Settings) -> ExitCode {
    let mut state = AppState::default();
    attach_from_settings(&mut state, &settings);
    crate::ui::run(&mut state, &settings)
}

#[cfg(not(feature = "tui"))]
pub fn run(settings: Settings) -> ExitCode {
    let mut state = AppState::default();
    attach_from_settings(&mut state, &settings);
    let _ = state;
    eprintln!("gameconqueror: built without the `tui` feature; nothing to run");
    ExitCode::FAILURE
}

fn attach_from_settings(state: &mut AppState, settings: &Settings) {
    if let Some(pid) = settings.pid {
        update(state, Msg::Attach(pid));
    }
}

#[cfg(test)]
mod tests;
