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

/// Current application state: the attached session (if any), recorded cheats, the focused
/// panel, help/quit flags, and the last status message. No `ratatui`/`crossterm` types appear
/// anywhere in this module.
#[derive(Debug, Default)]
pub struct AppState {
    pub(super) session: Option<Session>,
    pub(super) cheats: Vec<CheatEntry>,
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
