//! Application core: [`AppState`], [`Msg`], and [`update`] — a toolkit-independent state
//! machine. Only [`crate::settings::Settings`] crosses in from `main`; no CLI or raw config
//! types, and no `ratatui`/`crossterm` types anywhere in this module tree.

#[cfg(feature = "cheat-list")]
mod cheatlist;
mod focus;
mod msg;
mod process_list;
mod state;

#[cfg(feature = "cheat-list")]
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

use libscanmem::error::ScanmemError;
use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::session::{ScanCriterion, ScanExpr, ScanStats, Session};
use libscanmem::value::{self, UserValue, Value};

pub use focus::{Direction, Focus};
pub use msg::Msg;
use state::ScanJob;
pub use state::{AppState, AttachedProcess, MatchSortColumn, ProcessEntry, Status, StatusLevel};
#[cfg(feature = "cheat-list")]
pub use state::{CheatEntry, PathPromptKind};

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
        Msg::Snapshot => Some(spawn_scan(state, Session::run_snapshot)),
        Msg::PollScan => poll_scan(state),
        Msg::ResetScan => Some(reset_scan(state)),
        Msg::Write { address, value } => Some(with_session(state, |session| {
            session.write(address, &value).map(|()| "ok".to_owned())
        })),
        Msg::FocusHexView(address) => Some(focus_hex_view(state, address)),
        Msg::MoveHexCursor(delta) => {
            move_hex_cursor(state, delta);
            None
        }
        Msg::SetHexEditInput(input) => {
            state.hex_edit_input = input;
            None
        }
        Msg::CommitHexEdit => Some(commit_hex_edit(state)),
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
        #[cfg(feature = "cheat-list")]
        Msg::BeginEditCheatValue(index) => Some(begin_edit_cheat_value(state, index)),
        #[cfg(feature = "cheat-list")]
        Msg::SetCheatValueInput(input) => {
            state.cheat_value_input = input;
            None
        }
        #[cfg(feature = "cheat-list")]
        Msg::ConfirmCheatValueEdit => Some(confirm_cheat_value_edit(state)),
        #[cfg(feature = "cheat-list")]
        Msg::SaveCheatList => Some(save_cheat_list(state)),
        #[cfg(feature = "cheat-list")]
        Msg::LoadCheatList => {
            state.path_prompt = Some(PathPromptKind::Load);
            state.path_input.clear();
            Some(Status::info("enter a path to load the cheat list"))
        }
        #[cfg(feature = "cheat-list")]
        Msg::SetPathInput(input) => {
            state.path_input = input;
            None
        }
        #[cfg(feature = "cheat-list")]
        Msg::ConfirmPathPrompt => Some(confirm_path_prompt(state)),
        #[cfg(feature = "cheat-list")]
        Msg::Tick => {
            rewrite_frozen_cheats(state);
            None
        }
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
                #[cfg(feature = "cheat-list")]
                Focus::CheatView => select_cheat(state, 1),
                _ => {}
            }
            None
        }
        Msg::SelectPrev => {
            match state.focus {
                Focus::ProcessPicker => select_process(state, -1),
                Focus::MatchView => select_match(state, -1),
                #[cfg(feature = "cheat-list")]
                Focus::CheatView => select_cheat(state, -1),
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
        Msg::FocusDirection(dir) => {
            state.focus = state.focus.towards(dir);
            None
        }
        Msg::ToggleExpand => {
            state.expanded = !state.expanded;
            None
        }
        Msg::ShowHelp => {
            state.help_visible = !state.help_visible;
            None
        }
        Msg::Dismiss => {
            if state.help_visible {
                state.help_visible = false;
                None
            } else if close_path_prompt_if_open(state) {
                None
            } else if let Some(job) = &state.scan_job {
                // The scan itself is on a background thread; this only asks it to stop early —
                // `Msg::PollScan` picks up the (partial) result once it actually finishes.
                job.stop_flag.request();
                Some(Status::info("cancelling scan…"))
            } else if state.focus == Focus::HexView && !state.hex_edit_input.is_empty() {
                state.hex_edit_input.clear();
                None
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
                    Focus::CheatView => {
                        state.cheat_value_input.clear();
                        state.cheat_editing_index = None;
                    }
                    Focus::HexView => {}
                }
                None
            } else {
                None
            }
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
        None => not_attached(state),
    }
}

/// Bytes of session memory loaded into the Hex View on either side of the focused address.
const HEX_VIEW_BUFFER_LEN: usize = 256;

/// Loads [`HEX_VIEW_BUFFER_LEN`] bytes of session memory centered on `address` into the Hex View
/// buffer and switches focus to it. Falls back to reading forward from `address` (rather than
/// centered) if the centered window crosses into unmapped memory, since a match sitting near the
/// start of its region would otherwise always fail to open.
fn focus_hex_view(state: &mut AppState, address: usize) -> Status {
    let Some(session) = state.session.as_mut() else {
        return not_attached(state);
    };

    let centered_base = address.saturating_sub(HEX_VIEW_BUFFER_LEN / 2);
    let (base, bytes) = match session.read(centered_base, HEX_VIEW_BUFFER_LEN) {
        Ok(bytes) => (centered_base, bytes),
        Err(_) => match session.read(address, HEX_VIEW_BUFFER_LEN) {
            Ok(bytes) => (address, bytes),
            Err(err) => return Status::error(err.to_string()),
        },
    };

    state.hex_cursor = address
        .saturating_sub(base)
        .min(bytes.len().saturating_sub(1));
    state.hex_base_address = base;
    state.hex_buffer = bytes;
    state.hex_edit_input.clear();
    state.focus = Focus::HexView;
    Status::info(format!("hex view @ {address:#x}"))
}

/// Moves `state.hex_cursor` by `delta` bytes, clamped to `state.hex_buffer`'s bounds; a no-op if
/// the buffer is empty.
fn move_hex_cursor(state: &mut AppState, delta: isize) {
    if state.hex_buffer.is_empty() {
        return;
    }
    let max = state.hex_buffer.len() as isize - 1;
    let next = (state.hex_cursor as isize + delta).clamp(0, max);
    state.hex_cursor = next as usize;
}

/// Parses `state.hex_edit_input` as a hex byte and writes it to the address under the Hex View
/// cursor, updating `state.hex_buffer` on success.
fn commit_hex_edit(state: &mut AppState) -> Status {
    let input = std::mem::take(&mut state.hex_edit_input);
    if input.is_empty() {
        return Status::error("no byte value entered");
    }
    let Ok(byte) = u8::from_str_radix(&input, 16) else {
        return Status::error(format!("{input:?} is not a valid hex byte"));
    };
    let Some(address) = state.hex_cursor_address() else {
        return if state.session.is_none() {
            not_attached(state)
        } else {
            Status::error("hex view has no bytes loaded")
        };
    };
    let Some(session) = state.session.as_mut() else {
        return not_attached(state);
    };

    match session.write(address, &Value::U8(byte)) {
        Ok(()) => {
            state.hex_buffer[state.hex_cursor] = byte;
            Status::info(format!("wrote {byte:#04x} @ {address:#x}"))
        }
        Err(err) => Status::error(err.to_string()),
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
        return not_attached(state);
    };
    match session.write(address, &value) {
        Ok(()) => {
            state.cheats[index].value = value;
            Status::info("cheat value updated")
        }
        Err(err) => Status::error(err.to_string()),
    }
}

#[cfg(feature = "cheat-list")]
fn begin_edit_cheat_value(state: &mut AppState, index: usize) -> Status {
    match state.cheats.get(index) {
        Some(entry) => {
            state.cheat_value_input = entry.value.to_string();
            state.cheat_editing_index = Some(index);
            state.search_active = true;
            Status::info("editing cheat value — Enter to confirm, Esc to cancel")
        }
        None => cheat_index_out_of_range(index),
    }
}

#[cfg(feature = "cheat-list")]
fn confirm_cheat_value_edit(state: &mut AppState) -> Status {
    let Some(index) = state.cheat_editing_index.take() else {
        return Status::error("no cheat value is being edited");
    };
    state.search_active = false;
    let input = std::mem::take(&mut state.cheat_value_input);

    let Some(entry) = state.cheats.get(index) else {
        return cheat_index_out_of_range(index);
    };
    match value_from_input(&entry.value, input.trim()) {
        Ok(value) => edit_cheat_value(state, index, value),
        Err(err) => Status::error(err),
    }
}

/// Parses `input` into a [`Value`] of the same variant (and width) as `template`, mirroring
/// [`build_scan_expr`]'s numeric handling but resolving to a single concrete width instead of a
/// scan criterion.
#[cfg(feature = "cheat-list")]
fn value_from_input(template: &Value, input: &str) -> Result<Value, String> {
    match template {
        Value::Bytes(_) => parse_concrete_bytes(input).map(Value::Bytes),
        Value::Str(_) => Ok(Value::Str(input.to_owned())),
        _ => {
            let number = value::parse_number(input).map_err(|err| err.to_string())?;
            match template {
                Value::U8(_) => number.u8.map(Value::U8),
                Value::I8(_) => number.i8.map(Value::I8),
                Value::U16(_) => number.u16.map(Value::U16),
                Value::I16(_) => number.i16.map(Value::I16),
                Value::U32(_) => number.u32.map(Value::U32),
                Value::I32(_) => number.i32.map(Value::I32),
                Value::U64(_) => number.u64.map(Value::U64),
                Value::I64(_) => number.i64.map(Value::I64),
                Value::F32(_) => number.f32.map(Value::F32),
                Value::F64(_) => number.f64.map(Value::F64),
                Value::Bytes(_) | Value::Str(_) => unreachable!(),
            }
            .ok_or_else(|| "value does not fit the cheat's data width".to_owned())
        }
    }
}

/// Parses whitespace-separated two-hex-digit bytes (as printed by [`Value`]'s `Display` impl for
/// [`Value::Bytes`]) back into concrete bytes — unlike [`value::parse_bytearray`], wildcards
/// (`??`) aren't valid here since a cheat's stored value must be a concrete byte string.
#[cfg(feature = "cheat-list")]
fn parse_concrete_bytes(input: &str) -> Result<Vec<u8>, String> {
    let bytes = input
        .split_whitespace()
        .map(|token| {
            u8::from_str_radix(token, 16).map_err(|_| format!("{token:?} is not two hex digits"))
        })
        .collect::<Result<Vec<u8>, String>>()?;

    if bytes.is_empty() {
        return Err("byte value must not be empty".to_owned());
    }
    Ok(bytes)
}

#[cfg(feature = "cheat-list")]
fn save_cheat_list(state: &mut AppState) -> Status {
    match state.cheat_list_path.clone() {
        Some(path) => match cheatlist::save(&path, &state.cheats) {
            Ok(()) => Status::info(format!("cheat list saved to {}", path.display())),
            Err(err) => Status::error(err.to_string()),
        },
        None => {
            state.path_prompt = Some(PathPromptKind::Save);
            state.path_input.clear();
            Status::info("enter a path to save the cheat list")
        }
    }
}

#[cfg(feature = "cheat-list")]
fn confirm_path_prompt(state: &mut AppState) -> Status {
    let Some(kind) = state.path_prompt.take() else {
        return Status::error("no path is being entered");
    };
    let input = std::mem::take(&mut state.path_input);
    let path = PathBuf::from(input.trim());

    match kind {
        PathPromptKind::Save => match cheatlist::save(&path, &state.cheats) {
            Ok(()) => {
                let text = format!("cheat list saved to {}", path.display());
                state.cheat_list_path = Some(path);
                Status::info(text)
            }
            Err(err) => Status::error(err.to_string()),
        },
        PathPromptKind::Load => match cheatlist::load(&path) {
            Ok(cheats) => {
                let text = format!("loaded {} cheat(s) from {}", cheats.len(), path.display());
                state.cheats = cheats;
                state.cheat_selected = 0;
                state.cheat_list_path = Some(path);
                Status::info(text)
            }
            Err(err) => Status::error(err.to_string()),
        },
    }
}

/// Rewrites every frozen cheat-list entry's stored value back to its address, ignoring
/// individual write failures (a target may unmap the page between ticks; the next tick retries).
#[cfg(feature = "cheat-list")]
fn rewrite_frozen_cheats(state: &mut AppState) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    for entry in state.cheats.iter().filter(|entry| entry.frozen) {
        let _ = session.write(entry.address, &entry.value);
    }
}

#[cfg(feature = "cheat-list")]
fn close_path_prompt_if_open(state: &mut AppState) -> bool {
    if state.path_prompt.is_some() {
        state.path_prompt = None;
        state.path_input.clear();
        true
    } else {
        false
    }
}

#[cfg(not(feature = "cheat-list"))]
fn close_path_prompt_if_open(_state: &mut AppState) -> bool {
    false
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

/// Moves `state.cheat_selected` by `delta` (`1` or `-1`), wrapping within the current cheat-list
/// length; a no-op if the list is empty.
#[cfg(feature = "cheat-list")]
fn select_cheat(state: &mut AppState, delta: isize) {
    let len = state.cheats.len();
    if len == 0 {
        return;
    }
    let current = state.cheat_selected as isize;
    state.cheat_selected = (current + delta).rem_euclid(len as isize) as usize;
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
        Ok(expr) => spawn_scan(state, move |session| session.run_scan(&expr)),
        Err(err) => Status::error(err),
    }
}

/// Starts `run` (a first/narrowing scan or a snapshot) on a background thread so the UI keeps
/// rendering and responding to input while it works — either can easily take longer than a
/// frame over a large address space. Moves `state.session` into the thread for the duration;
/// `Msg::PollScan` moves it back once the thread sends a result. `state.attached` is untouched,
/// so the UI keeps showing what it's attached to throughout.
///
/// Stops the target (`Session::prepare_scan`) here, on the caller's thread, *before* handing the
/// session to the background thread, and only resumes it (`Session::resume_after_scan`) back on
/// the caller's thread once `Msg::PollScan` sees the result — never inside the background thread
/// itself. Ptrace ties the tracer relationship to the specific thread that called
/// `Session::attach` (this one, since attach/detach are never threaded); stopping or resuming
/// from any other thread would send the signal fine but then hang forever in `waitpid` waiting
/// for a stop notification only the tracer thread ever receives. `run` itself doesn't touch
/// ptrace — only `/proc/<pid>/mem`, safe from any thread — so it's the only part actually moved
/// off this one.
fn spawn_scan(
    state: &mut AppState,
    run: impl FnOnce(&mut Session) -> Result<ScanStats, ScanmemError> + Send + 'static,
) -> Status {
    let Some(mut session) = state.session.take() else {
        return not_attached(state);
    };

    if let Err(err) = session.prepare_scan() {
        state.session = Some(session);
        return Status::error(err.to_string());
    }

    let progress = session.progress_handle();
    let stop_flag = session.stop_handle();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = run(&mut session);
        let _ = tx.send((session, result));
    });

    state.scan_job = Some(ScanJob {
        rx,
        progress,
        stop_flag,
    });
    Status::info("scanning…")
}

/// Checks whether the in-progress background scan/snapshot has finished; if so, resumes the
/// target (see [`spawn_scan`] for why that must happen here rather than on the background
/// thread), restores `state.session`, and reports the scan's outcome. Dispatched every
/// event-loop iteration via `Msg::PollScan`; a no-op if no scan is running or it hasn't sent a
/// result yet.
fn poll_scan(state: &mut AppState) -> Option<Status> {
    let job = state.scan_job.as_ref()?;
    match job.rx.try_recv() {
        Ok((session, result)) => {
            session.resume_after_scan();
            state.scan_job = None;
            state.session = Some(session);
            Some(match result {
                Ok(stats) => Status::info(format!("{} match(es)", stats.matches)),
                Err(err) => Status::error(err.to_string()),
            })
        }
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => {
            state.scan_job = None;
            Some(Status::error("scan thread terminated unexpectedly"))
        }
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
        None => not_attached(state),
    }
}

/// The "no session available" status: distinguishes truly not being attached from `state.session`
/// being temporarily unavailable because a background scan currently owns it (see
/// `AppState::scan_job`) — both leave `state.session` empty, but the latter needs a different
/// message since `state.attached` says otherwise.
fn not_attached(state: &AppState) -> Status {
    if state.scan_job.is_some() {
        return Status::error("a scan is currently running");
    }
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
