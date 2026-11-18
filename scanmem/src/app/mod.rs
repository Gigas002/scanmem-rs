//! Application state and the REPL entry point — owns the `Session` and CLI-local state; no CLI
//! parsing, no raw config types (only [`Settings`] crosses into this module).

pub mod repl;
mod script;

use std::process::ExitCode;

use libscanmem::error::ScanmemError;
use libscanmem::interrupt::StopFlag;
use libscanmem::session::Session;
use rustix::process::Pid;

use crate::commands::formatter;
use crate::settings::Settings;

/// One attached (or not-yet-attached) session plus REPL-local state — the structured
/// replacement for `globals_t`, owned here instead of global.
#[derive(Default)]
pub struct AppState {
    session: Option<Session>,
    /// Set by Ctrl-C while a command runs (REPL only); every session scans with it.
    interrupt: Option<StopFlag>,
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
        let mut session = Session::attach(pid)?;
        if let Some(flag) = &self.interrupt {
            session.set_stop_handle(flag.clone());
        }
        let region_count = session.region_count()?;
        self.session = Some(session);
        Ok(region_count)
    }

    /// Makes Ctrl-C cancel a running scan instead of ending the process (a second Ctrl-C
    /// still does). Returns the flag, to be reset after each command.
    pub fn cancel_scans_on_ctrl_c(&mut self) -> std::io::Result<StopFlag> {
        let flag = StopFlag::register_sigint()?;
        if let Some(session) = &mut self.session {
            session.set_stop_handle(flag.clone());
        }
        self.interrupt = Some(flag.clone());
        Ok(flag)
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
                    "{}",
                    formatter::info(&format!(
                        "attached to pid {}: {region_count} region(s)",
                        pid.as_raw_pid()
                    ))
                );
            }
            Err(err) => eprintln!(
                "{}",
                formatter::error(format!(
                    "failed to attach to pid {}: {err}",
                    pid.as_raw_pid()
                ))
            ),
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
