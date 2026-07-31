//! [`AppState`] — attached session, recorded cheats, and focus/help/quit flags; the
//! toolkit-independent state the `ui/` shell renders from each frame.

use libscanmem::error::ScanmemError;
use libscanmem::session::Session;
use libscanmem::value::Value;
use rustix::process::Pid;

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
#[derive(Debug, Clone, PartialEq)]
pub struct CheatEntry {
    pub address: usize,
    pub description: String,
    pub value: Value,
    pub frozen: bool,
}

/// One running process visible under `/proc`: a pid and its `comm` name (`"?"` if the name
/// could not be read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
}

/// Current application state: the attached session (if any), recorded cheats, the Process
/// Picker's list/filter/selection, the focused panel, help/quit flags, and the last status
/// message. No `ratatui`/`crossterm` types appear anywhere in this module.
#[derive(Debug, Default)]
pub struct AppState {
    pub(super) session: Option<Session>,
    pub(super) cheats: Vec<CheatEntry>,
    pub(super) processes: Vec<ProcessEntry>,
    pub(super) process_filter: String,
    pub(super) process_selected: usize,
    pub(super) search_active: bool,
    pub(super) focus: Focus,
    pub(super) help_visible: bool,
    pub(super) quit: bool,
    pub(super) status: Option<Status>,
}

impl AppState {
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn cheats(&self) -> &[CheatEntry] {
        &self.cheats
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
