//! [`AppState`] — attached session, recorded cheats, and focus/help/quit flags; the
//! toolkit-independent state the `ui/` shell renders from each frame.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use libscanmem::error::ScanmemError;
use libscanmem::interrupt::{ScanProgress, StopFlag};
use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::session::{MatchView, ScanStats, Session};
use libscanmem::value::Value;
use rustix::process::Pid;
#[cfg(feature = "cheat-list")]
use std::path::PathBuf;

use crate::app::focus::Focus;

/// Severity of a status-bar message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Error,
}

/// A one-line status-bar message produced by the most recent [`super::update`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub level: StatusLevel,
    pub text: String,
}

impl Status {
    pub(super) fn info(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Info,
            text: text.into(),
        }
    }

    pub(super) fn error(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Error,
            text: text.into(),
        }
    }
}

/// One recorded cheat-list entry: an address, a user description, the value to (re)write, and
/// whether it is currently frozen (continuously rewritten).
#[cfg(feature = "cheat-list")]
#[derive(Debug, Clone, PartialEq)]
pub struct CheatEntry {
    pub address: usize,
    pub description: String,
    pub value: Value,
    pub frozen: bool,
}

/// Which cheat-list operation [`AppState::path_input`] is currently being typed for.
#[cfg(feature = "cheat-list")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPromptKind {
    Save,
    Load,
}

/// One running process visible under `/proc`: a pid and its display name (`"?"` if the name
/// could not be read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
}

/// The currently attached target's pid and display name, tracked independent of whether
/// [`AppState::session`] itself is populated — while a scan runs on a background thread the
/// `Session` is temporarily taken out of `AppState`, but the UI must keep showing what it's
/// attached to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedProcess {
    pub pid: u32,
    pub name: String,
}

/// A scan or refresh running on a background thread, started by `Msg::RunScan`/`Msg::NewScan`/
/// `Msg::RefreshMatches` so a slow one can't freeze the UI. Owns the `Session` for the duration —
/// it's moved out of `AppState` when the job starts and moved back once `Msg::PollScan` observes
/// a result on `rx` — plus handles to watch its progress and request it stop early without
/// needing the `Session` itself (which isn't available to the UI thread while the job is running).
#[derive(Debug)]
pub(super) struct ScanJob {
    pub(super) rx: mpsc::Receiver<(Session, Result<ScanStats, ScanmemError>)>,
    pub(super) progress: ScanProgress,
    pub(super) stop_flag: StopFlag,
}

/// Which column the Match View is currently sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchSortColumn {
    Address,
    Value,
}

impl MatchSortColumn {
    /// The other column, cycled via `Msg::CycleMatchSort`.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            MatchSortColumn::Address => MatchSortColumn::Value,
            MatchSortColumn::Value => MatchSortColumn::Address,
        }
    }
}

/// Current application state: the attached session (if any), recorded cheats, the Process
/// Picker's list/filter/selection, the Scan Panel's data type/match type/value input, the Match
/// View's sort/filter/selection, the focused panel, help/quit flags, and the last status message.
/// No `ratatui`/`crossterm` types appear anywhere in this module.
#[derive(Debug)]
pub struct AppState {
    pub(super) session: Option<Session>,
    pub(super) attached: Option<AttachedProcess>,
    pub(super) scan_job: Option<ScanJob>,
    #[cfg(feature = "cheat-list")]
    pub(super) cheats: Vec<CheatEntry>,
    #[cfg(feature = "cheat-list")]
    pub(super) cheat_selected: usize,
    #[cfg(feature = "cheat-list")]
    pub(super) cheat_value_input: String,
    #[cfg(feature = "cheat-list")]
    pub(super) cheat_editing_index: Option<usize>,
    #[cfg(feature = "cheat-list")]
    pub(super) cheat_list_path: Option<PathBuf>,
    #[cfg(feature = "cheat-list")]
    pub(super) path_prompt: Option<PathPromptKind>,
    #[cfg(feature = "cheat-list")]
    pub(super) path_input: String,
    pub(super) processes: Vec<ProcessEntry>,
    pub(super) process_filter: String,
    pub(super) process_selected: usize,
    pub(super) search_active: bool,
    pub(super) scan_data_type: ScanDataType,
    pub(super) scan_match_type: MatchType,
    pub(super) scan_input: String,
    pub(super) match_sort: MatchSortColumn,
    pub(super) match_filter: String,
    pub(super) match_selected: usize,
    /// Every currently tracked match's value as of the most recent scan/refresh — the baseline
    /// [`Self::match_changed_addresses`] is computed against on the *next* one.
    pub(super) match_previous_values: HashMap<usize, Value>,
    /// Addresses whose value differed from [`Self::match_previous_values`] as of the most recent
    /// scan/refresh — drives Match View's "just changed" highlight. Recomputed (and
    /// `match_previous_values` updated to the new snapshot) every time a scan/refresh completes;
    /// an address absent from the *previous* snapshot (a newly discovered match) is never
    /// considered changed, only one whose value actually differs from what it was last time.
    pub(super) match_changed_addresses: HashSet<usize>,
    #[cfg(feature = "hex-view")]
    pub(super) hex_buffer: Vec<u8>,
    #[cfg(feature = "hex-view")]
    pub(super) hex_base_address: usize,
    #[cfg(feature = "hex-view")]
    pub(super) hex_cursor: usize,
    #[cfg(feature = "hex-view")]
    pub(super) hex_edit_input: String,
    /// Bytes of session memory loaded into the Hex View on either side of the focused address —
    /// seeded from `Settings::hex_view_buffer_len` at startup (`app::apply_settings_to_state`).
    #[cfg(feature = "hex-view")]
    pub(super) hex_view_buffer_len: usize,
    pub(super) focus: Focus,
    pub(super) expanded: bool,
    pub(super) help_visible: bool,
    pub(super) quit: bool,
    pub(super) status: Option<Status>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: None,
            attached: None,
            scan_job: None,
            #[cfg(feature = "cheat-list")]
            cheats: Vec::new(),
            #[cfg(feature = "cheat-list")]
            cheat_selected: 0,
            #[cfg(feature = "cheat-list")]
            cheat_value_input: String::new(),
            #[cfg(feature = "cheat-list")]
            cheat_editing_index: None,
            #[cfg(feature = "cheat-list")]
            cheat_list_path: None,
            #[cfg(feature = "cheat-list")]
            path_prompt: None,
            #[cfg(feature = "cheat-list")]
            path_input: String::new(),
            processes: Vec::new(),
            process_filter: String::new(),
            process_selected: 0,
            search_active: false,
            scan_data_type: ScanDataType::Integer32,
            scan_match_type: MatchType::EqualTo,
            scan_input: String::new(),
            match_sort: MatchSortColumn::Address,
            match_filter: String::new(),
            match_selected: 0,
            match_previous_values: HashMap::new(),
            match_changed_addresses: HashSet::new(),
            #[cfg(feature = "hex-view")]
            hex_buffer: Vec::new(),
            #[cfg(feature = "hex-view")]
            hex_base_address: 0,
            #[cfg(feature = "hex-view")]
            hex_cursor: 0,
            #[cfg(feature = "hex-view")]
            hex_edit_input: String::new(),
            #[cfg(feature = "hex-view")]
            hex_view_buffer_len: 256,
            focus: Focus::default(),
            expanded: false,
            help_visible: false,
            quit: false,
            status: None,
        }
    }
}

impl AppState {
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// The currently attached target's pid/name, if any — stays populated for as long as a
    /// session is attached, even while [`Self::session`] is temporarily unavailable because a
    /// background scan currently owns it.
    pub fn attached(&self) -> Option<&AttachedProcess> {
        self.attached.as_ref()
    }

    /// `true` while a scan/refresh started by `Msg::RunScan`/`Msg::NewScan`/`Msg::RefreshMatches`
    /// is running on a background thread. [`Self::session`] is unavailable for the duration.
    pub fn is_scanning(&self) -> bool {
        self.scan_job.is_some()
    }

    /// `(bytes scanned so far, total bytes considered)` for the in-progress scan, or `None` if
    /// none is running.
    pub fn scan_progress(&self) -> Option<(usize, usize)> {
        self.scan_job.as_ref().map(|job| job.progress.get())
    }

    #[cfg(feature = "cheat-list")]
    pub fn cheats(&self) -> &[CheatEntry] {
        &self.cheats
    }

    #[cfg(feature = "cheat-list")]
    pub fn cheat_selected(&self) -> usize {
        self.cheat_selected
    }

    /// The cheat currently highlighted in [`Self::cheats`], if any.
    #[cfg(feature = "cheat-list")]
    pub fn selected_cheat(&self) -> Option<&CheatEntry> {
        self.cheats.get(self.cheat_selected)
    }

    #[cfg(feature = "cheat-list")]
    pub fn cheat_value_input(&self) -> &str {
        &self.cheat_value_input
    }

    #[cfg(feature = "cheat-list")]
    pub fn cheat_list_path(&self) -> Option<&std::path::Path> {
        self.cheat_list_path.as_deref()
    }

    #[cfg(feature = "cheat-list")]
    pub fn path_prompt(&self) -> Option<PathPromptKind> {
        self.path_prompt
    }

    #[cfg(feature = "cheat-list")]
    pub fn path_input(&self) -> &str {
        &self.path_input
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// `true` while `ui/layout.rs` shows only the focused panel fullscreen instead of the
    /// multi-panel grid — toggled by `Msg::ToggleExpand` (`Ctrl+E`).
    pub fn expanded(&self) -> bool {
        self.expanded
    }

    pub fn help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn status(&self) -> Option<&Status> {
        self.status.as_ref()
    }

    pub fn processes(&self) -> &[ProcessEntry] {
        &self.processes
    }

    pub fn process_filter(&self) -> &str {
        &self.process_filter
    }

    pub fn search_active(&self) -> bool {
        self.search_active
    }

    pub fn process_selected(&self) -> usize {
        self.process_selected
    }

    /// Processes whose pid or name contains [`Self::process_filter`] (case-insensitive), in the
    /// order returned by [`Self::processes`].
    pub fn filtered_processes(&self) -> Vec<&ProcessEntry> {
        if self.process_filter.is_empty() {
            return self.processes.iter().collect();
        }

        let needle = self.process_filter.to_lowercase();
        self.processes
            .iter()
            .filter(|process| {
                process.name.to_lowercase().contains(&needle)
                    || process.pid.to_string().contains(&needle)
            })
            .collect()
    }

    /// The process currently highlighted in [`Self::filtered_processes`], if any.
    pub fn selected_process(&self) -> Option<&ProcessEntry> {
        self.filtered_processes()
            .into_iter()
            .nth(self.process_selected)
    }

    pub fn scan_data_type(&self) -> ScanDataType {
        self.scan_data_type
    }

    pub fn scan_match_type(&self) -> MatchType {
        self.scan_match_type
    }

    pub fn scan_input(&self) -> &str {
        &self.scan_input
    }

    pub fn match_sort(&self) -> MatchSortColumn {
        self.match_sort
    }

    pub fn match_filter(&self) -> &str {
        &self.match_filter
    }

    pub fn match_selected(&self) -> usize {
        self.match_selected
    }

    /// The attached session's current match set, filtered by [`Self::match_filter`] (against the
    /// hex address or decimal value, case-insensitively) and sorted by [`Self::match_sort`].
    /// Empty if no session is attached.
    pub fn filtered_matches(&self) -> Vec<MatchView> {
        let Some(session) = &self.session else {
            return Vec::new();
        };
        let mut matches: Vec<MatchView> = session.matches().collect();

        if !self.match_filter.is_empty() {
            let needle = self.match_filter.to_lowercase();
            matches.retain(|entry| {
                format!("{:#x}", entry.address).contains(&needle)
                    || entry.old_value.to_string().contains(&needle)
            });
        }

        match self.match_sort {
            MatchSortColumn::Address => matches.sort_by_key(|entry| entry.address),
            // `Value` can't be `Ord` (its float variants are only `PartialOrd`), so sort by its
            // numeric widening instead of the raw value.
            MatchSortColumn::Value => matches.sort_by(|a, b| {
                a.old_value
                    .numeric_key()
                    .partial_cmp(&b.old_value.numeric_key())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }

        matches
    }

    /// The match currently highlighted in [`Self::filtered_matches`], if any.
    pub fn selected_match(&self) -> Option<MatchView> {
        self.filtered_matches().into_iter().nth(self.match_selected)
    }

    /// Whether the match at `address` had a different value on the most recent scan/refresh than
    /// it did on the one before that — drives Match View's "just changed" highlight. Not `true`
    /// for a newly discovered match (nothing to compare it against yet), only one that was
    /// already tracked and whose value actually differs from what it was last time.
    pub fn match_recently_changed(&self, address: usize) -> bool {
        self.match_changed_addresses.contains(&address)
    }

    /// The bytes currently loaded into the Hex View, starting at [`Self::hex_base_address`].
    #[cfg(feature = "hex-view")]
    pub fn hex_buffer(&self) -> &[u8] {
        &self.hex_buffer
    }

    /// The address [`Self::hex_buffer`]'s first byte is loaded from.
    #[cfg(feature = "hex-view")]
    pub fn hex_base_address(&self) -> usize {
        self.hex_base_address
    }

    /// The Hex View cursor's offset within [`Self::hex_buffer`].
    #[cfg(feature = "hex-view")]
    pub fn hex_cursor(&self) -> usize {
        self.hex_cursor
    }

    /// The address of the byte currently under the Hex View cursor, or `None` if no bytes are
    /// loaded.
    #[cfg(feature = "hex-view")]
    pub fn hex_cursor_address(&self) -> Option<usize> {
        if self.hex_cursor < self.hex_buffer.len() {
            Some(self.hex_base_address + self.hex_cursor)
        } else {
            None
        }
    }

    /// The Hex View's in-progress byte-edit input at the cursor, if any.
    #[cfg(feature = "hex-view")]
    pub fn hex_edit_input(&self) -> &str {
        &self.hex_edit_input
    }

    /// Attaches to `pid`, replacing any previously attached session, and returns how many
    /// memory regions the target currently has mapped.
    pub(super) fn attach(&mut self, pid: Pid) -> Result<usize, ScanmemError> {
        let session = Session::attach(pid)?;
        let region_count = session.region_count()?;
        self.session = Some(session);
        let pid = pid.as_raw_pid() as u32;
        let name = self
            .processes
            .iter()
            .find(|process| process.pid == pid)
            .map_or_else(|| "?".to_owned(), |process| process.name.clone());
        self.attached = Some(AttachedProcess { pid, name });
        Ok(region_count)
    }

    /// Detaches the current session, resuming the target's execution.
    pub(super) fn detach(&mut self) -> Result<(), ScanmemError> {
        let session = self.session.as_mut().ok_or(ScanmemError::NotAttached)?;
        session.detach()?;
        self.session = None;
        self.attached = None;
        self.match_previous_values.clear();
        self.match_changed_addresses.clear();
        Ok(())
    }
}
