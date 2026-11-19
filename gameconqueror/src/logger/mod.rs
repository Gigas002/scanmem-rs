//! `tracing` subscriber initialization — file-only sink, since stdout/stderr are owned by the
//! `ratatui` alternate screen while the TUI is running.

use std::fs::{DirBuilder, File, OpenOptions};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use crate::settings::Settings;

/// Initializes the global `tracing` subscriber to append to `settings.log_file`.
///
/// Uses `try_init` so calling this more than once (e.g. across tests in the same process) does
/// not panic; only the first call takes effect. Silently does nothing if the log file cannot be
/// opened, since a logging failure must never block the TUI from starting.
pub fn init(settings: &Settings) {
    let Ok(file) = open_log_file(&settings.log_file) else {
        return;
    };

    let _ = tracing_subscriber::fmt()
        .with_max_level(settings.log_level)
        .with_target(false)
        .with_ansi(false)
        .with_writer(move || file.try_clone().expect("failed to clone log file handle"))
        .try_init();
}

/// Opens `path` for appending, creating it readable by its owner only (the log records addresses
/// and values written) and its missing parent directories private to the owner. A symlink at
/// `path` is refused rather than followed, so the log can't be redirected into another file.
pub(crate) fn open_log_file(path: &Path) -> std::io::Result<File> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(test)]
mod tests;
