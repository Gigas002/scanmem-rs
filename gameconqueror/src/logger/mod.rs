//! `tracing` subscriber initialization — file-only sink, since stdout/stderr are owned by the
//! `ratatui` alternate screen while the TUI is running.

use std::fs::OpenOptions;

use crate::settings::Settings;

/// Initializes the global `tracing` subscriber to append to `settings.log_file`.
///
/// Uses `try_init` so calling this more than once (e.g. across tests in the same process) does
/// not panic; only the first call takes effect. Silently does nothing if the log file cannot be
/// opened, since a logging failure must never block the TUI from starting.
pub fn init(settings: &Settings) {
    let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&settings.log_file)
    else {
        return;
    };

    let _ = tracing_subscriber::fmt()
        .with_max_level(settings.log_level)
        .with_target(false)
        .with_ansi(false)
        .with_writer(move || file.try_clone().expect("failed to clone log file handle"))
        .try_init();
}

#[cfg(test)]
mod tests;
