//! Application core: [`AppState`], [`Msg`], and [`update`] — a toolkit-independent state
//! machine. Only [`crate::settings::Settings`] crosses in from `main`; no CLI or raw config
//! types, and no `ratatui`/`crossterm` types anywhere in this module tree.

mod focus;
mod msg;
mod process_list;
mod state;

use std::process::ExitCode;

use libscanmem::error::ScanmemError;
use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::session::{ScanCriterion, ScanExpr, Session};
#[cfg(feature = "cheat-list")]
use libscanmem::value::Value;
use libscanmem::value::{self, UserValue};

pub use focus::Focus;
pub use msg::Msg;
#[cfg(feature = "cheat-list")]
pub use state::CheatEntry;
pub use state::{AppState, MatchSortColumn, ProcessEntry, Status, StatusLevel};

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
        #[cfg(feature = "cheat-list")]
        Msg::AddCheat {
            address,
            description,
            value,
        } => Some(add_cheat(state, address, description, value)),
        #[cfg(feature = "cheat-list")]
        Msg::RemoveCheat(index) => Some(remove_cheat(state, index)),
        #[cfg(feature = "cheat-list")]
        Msg::ToggleFreeze(index) => Some(toggle_freeze(state, index)),
        #[cfg(feature = "cheat-list")]
        Msg::EditCheatValue { index, value } => Some(edit_cheat_value(state, index, value)),
        Msg::RefreshProcessList => Some(refresh_process_list(state)),
        Msg::FilterProcesses(query) => {
            state.process_filter = query;
            state.process_selected = 0;
            None
        }
        Msg::CycleScanDataType => {
            state.scan_data_type = next_data_type(state.scan_data_type);
            None
        }
        Msg::CycleScanMatchType => {
            state.scan_match_type = next_match_type(state.scan_match_type);
            None
        }
        Msg::SetScanInput(input) => {
            state.scan_input = input;
            None
        }
        Msg::RunScan => Some(run_scan(state)),
        Msg::CycleMatchSort => {
            state.match_sort = state.match_sort.next();
            state.match_selected = 0;
            None
        }
        Msg::FilterMatches(query) => {
            state.match_filter = query;
            state.match_selected = 0;
            None
        }
        Msg::ToggleSearch => {
            state.search_active = !state.search_active;
            None
        }
        Msg::SelectNext => {
            match state.focus {
                Focus::ProcessPicker => select_process(state, 1),
                Focus::MatchView => select_match(state, 1),
                _ => {}
            }
            None
        }
        Msg::SelectPrev => {
            match state.focus {
                Focus::ProcessPicker => select_process(state, -1),
                Focus::MatchView => select_match(state, -1),
                _ => {}
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
                match state.focus {
                    Focus::ProcessPicker => {
                        state.process_filter.clear();
                        state.process_selected = 0;
                    }
                    Focus::ScanPanel => state.scan_input.clear(),
                    Focus::MatchView => {
                        state.match_filter.clear();
                        state.match_selected = 0;
                    }
                    #[cfg(feature = "cheat-list")]
                    Focus::CheatView => {}
                    Focus::HexView => {}
                }
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

#[cfg(feature = "cheat-list")]
fn add_cheat(state: &mut AppState, address: usize, description: String, value: Value) -> Status {
    state.cheats.push(CheatEntry {
        address,
        description,
        value,
        frozen: false,
    });
    Status::info("cheat added")
}

#[cfg(feature = "cheat-list")]
fn remove_cheat(state: &mut AppState, index: usize) -> Status {
    if index >= state.cheats.len() {
        return cheat_index_out_of_range(index);
    }
    state.cheats.remove(index);
    Status::info("cheat removed")
}

#[cfg(feature = "cheat-list")]
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

#[cfg(feature = "cheat-list")]
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

/// Moves `state.match_selected` by `delta` (`1` or `-1`), wrapping within the current
/// filtered-match count; a no-op if the filtered list is empty.
fn select_match(state: &mut AppState, delta: isize) {
    let len = state.filtered_matches().len();
    if len == 0 {
        return;
    }
    let current = state.match_selected as isize;
    state.match_selected = (current + delta).rem_euclid(len as isize) as usize;
}

/// Every [`ScanDataType`] variant, in the order [`next_data_type`] cycles through.
const SCAN_DATA_TYPES: [ScanDataType; 11] = [
    ScanDataType::AnyNumber,
    ScanDataType::AnyInteger,
    ScanDataType::AnyFloat,
    ScanDataType::Integer8,
    ScanDataType::Integer16,
    ScanDataType::Integer32,
    ScanDataType::Integer64,
    ScanDataType::Float32,
    ScanDataType::Float64,
    ScanDataType::ByteArray,
    ScanDataType::String,
];

/// Every [`MatchType`] variant, in the order [`next_match_type`] cycles through.
const SCAN_MATCH_TYPES: [MatchType; 13] = [
    MatchType::Any,
    MatchType::EqualTo,
    MatchType::NotEqualTo,
    MatchType::GreaterThan,
    MatchType::LessThan,
    MatchType::Range,
    MatchType::Update,
    MatchType::NotChanged,
    MatchType::Changed,
    MatchType::Increased,
    MatchType::Decreased,
    MatchType::IncreasedBy,
    MatchType::DecreasedBy,
];

/// The next [`ScanDataType`] after `current`, wrapping around [`SCAN_DATA_TYPES`]. Neither
/// `ScanDataType` nor `MatchType` are defined in this crate, so cycling can't be an inherent
/// method on either.
fn next_data_type(current: ScanDataType) -> ScanDataType {
    let index = SCAN_DATA_TYPES
        .iter()
        .position(|&data_type| data_type == current)
        .unwrap_or(0);
    SCAN_DATA_TYPES[(index + 1) % SCAN_DATA_TYPES.len()]
}

/// The next [`MatchType`] after `current`, wrapping around [`SCAN_MATCH_TYPES`].
fn next_match_type(current: MatchType) -> MatchType {
    let index = SCAN_MATCH_TYPES
        .iter()
        .position(|&match_type| match_type == current)
        .unwrap_or(0);
    SCAN_MATCH_TYPES[(index + 1) % SCAN_MATCH_TYPES.len()]
}

fn run_scan(state: &mut AppState) -> Status {
    match build_scan_expr(state) {
        Ok(expr) => with_session(state, |session| {
            session
                .scan(&expr)
                .map(|stats| format!("{} match(es)", stats.matches))
        }),
        Err(err) => Status::error(err),
    }
}

/// Builds a `ScanExpr` from the Scan Panel's current data type/match type/free-text input,
/// mirroring `scanmem`'s REPL `scan` grammar but reading from already-stored `AppState` fields
/// instead of splitting a command line into tokens.
fn build_scan_expr(state: &AppState) -> Result<ScanExpr, String> {
    let data_type = state.scan_data_type;
    let match_type = state.scan_match_type;
    let input = state.scan_input.trim();

    if matches!(data_type, ScanDataType::ByteArray | ScanDataType::String) {
        if match_type != MatchType::EqualTo {
            return Err("byte/string scans only support the equal-to match type".to_owned());
        }
        let criterion = if data_type == ScanDataType::ByteArray {
            let pattern =
                value::parse_bytearray(input.split_whitespace()).map_err(|err| err.to_string())?;
            ScanCriterion::Value(UserValue::Bytes(pattern))
        } else if input.is_empty() {
            return Err("string scan requires a value".to_owned());
        } else {
            ScanCriterion::Value(value::parse_string(input))
        };
        return Ok(ScanExpr {
            data_type,
            match_type,
            criterion,
        });
    }

    let criterion = match match_type {
        MatchType::Any
        | MatchType::Update
        | MatchType::NotChanged
        | MatchType::Changed
        | MatchType::Increased
        | MatchType::Decreased => {
            if input.is_empty() {
                ScanCriterion::None
            } else {
                return Err("this match type takes no value".to_owned());
            }
        }
        MatchType::Range => {
            let mut tokens = input.split_whitespace();
            match (tokens.next(), tokens.next(), tokens.next()) {
                (Some(low), Some(high), None) => ScanCriterion::Range(
                    value::parse_number(low).map_err(|err| err.to_string())?,
                    value::parse_number(high).map_err(|err| err.to_string())?,
                ),
                _ => return Err("range match requires a low and high bound".to_owned()),
            }
        }
        MatchType::EqualTo
        | MatchType::NotEqualTo
        | MatchType::GreaterThan
        | MatchType::LessThan
        | MatchType::IncreasedBy
        | MatchType::DecreasedBy => {
            if input.is_empty() {
                return Err("this match type requires a value".to_owned());
            }
            ScanCriterion::Value(UserValue::Number(
                value::parse_number(input).map_err(|err| err.to_string())?,
            ))
        }
    };

    Ok(ScanExpr {
        data_type,
        match_type,
        criterion,
    })
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

#[cfg(feature = "cheat-list")]
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
