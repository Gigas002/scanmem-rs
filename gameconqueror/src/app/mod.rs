//! Application entry point behind the settings boundary — only [`Settings`] crosses in here, no
//! CLI or raw config types. The `Msg`/`update()` state machine described in
//! `docs/gameconqueror-plan.md` lands in a later phase; this is an empty skeleton for now.

use std::process::ExitCode;

use crate::settings::Settings;

/// Placeholder application state, grown into the full state machine in a later phase.
#[derive(Debug)]
pub struct AppState;

#[cfg(feature = "tui")]
pub fn run(settings: Settings) -> ExitCode {
    let mut state = AppState;
    crate::ui::run(&mut state, &settings)
}

#[cfg(not(feature = "tui"))]
pub fn run(settings: Settings) -> ExitCode {
    let _state = AppState;
    let _ = settings;
    eprintln!("gameconqueror: built without the `tui` feature; nothing to run");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests;
