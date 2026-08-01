//! [`AppState`] — attached session, recorded cheats, and focus/help/quit flags; the
//! toolkit-independent state the `ui/` shell renders from each frame.

use libscanmem::error::ScanmemError;
use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::session::{MatchView, Session};
#[cfg(feature = "cheat-list")]
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

/// One running process visible under `/proc`: a pid and its `comm` name (`"?"` if the name
/// could not be read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
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
    pub(super) focus: Focus,
    pub(super) help_visible: bool,
    pub(super) quit: bool,
    pub(super) status: Option<Status>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: None,
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
            focus: Focus::default(),
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
            MatchSortColumn::Value => matches.sort_by_key(|entry| entry.old_value),
        }

        matches
    }

    /// The match currently highlighted in [`Self::filtered_matches`], if any.
    pub fn selected_match(&self) -> Option<MatchView> {
        self.filtered_matches().into_iter().nth(self.match_selected)
    }

    /// Attaches to `pid`, replacing any previously attached session, and returns how many
    /// memory regions the target currently has mapped.
    pub(super) fn attach(&mut self, pid: Pid) -> Result<usize, ScanmemError> {
        let session = Session::attach(pid)?;
        let region_count = session.region_count()?;
        self.session = Some(session);
        Ok(region_count)
    }

    /// Detaches the current session, resuming the target's execution.
    pub(super) fn detach(&mut self) -> Result<(), ScanmemError> {
        let session = self.session.as_mut().ok_or(ScanmemError::NotAttached)?;
        session.detach()?;
        self.session = None;
        Ok(())
    }
}
