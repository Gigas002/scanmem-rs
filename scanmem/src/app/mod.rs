//! Application state and the REPL entry point — owns the `Session` and CLI-local state; no CLI
//! parsing, no raw config types (only [`Settings`] crosses into this module).

pub mod repl;
mod script;

use std::process::ExitCode;

use libscanmem::error::ScanmemError;
use libscanmem::session::Session;
use rustix::process::Pid;

use crate::settings::Settings;

/// One attached (or not-yet-attached) session plus REPL-local state — the structured
/// replacement for `globals_t`, owned here instead of global.
#[derive(Default)]
pub struct AppState {
    session: Option<Session>,
}

impl AppState {
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }

    /// Attaches to `pid`, replacing any previously attached session, and returns how many
    /// memory regions the target currently has mapped.
    pub fn attach(&mut self, pid: Pid) -> Result<usize, ScanmemError> {
        let session = Session::attach(pid)?;
        let region_count = session.region_count()?;
        self.session = Some(session);
        Ok(region_count)
    }

    /// Drops the current session, if any.
    pub fn reset(&mut self) {
        self.session = None;
    }
}

/// Runs the interactive REPL, or the `--exec` script if `settings.exec` is set, optionally
/// attaching to `settings.pid` first either way.
pub fn run(settings: Settings) -> ExitCode {
    let mut state = AppState::default();

    if let Some(pid) = settings.pid {
        match state.attach(pid) {
            Ok(region_count) => {
                println!(
                    "attached to pid {}: {region_count} region(s)",
                    pid.as_raw_pid()
                );
            }
            Err(err) => eprintln!("error: failed to attach to pid {}: {err}", pid.as_raw_pid()),
        }
    }

    match settings.exec {
        Some(script) => script::run(state, &script),
        None => {
            repl::run(state);
            ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests;
