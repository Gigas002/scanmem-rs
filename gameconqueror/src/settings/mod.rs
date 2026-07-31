//! Merges CLI + defaults into a single resolved [`Settings`] — the only type passed below this
//! module, per `ARCHITECTURE.md` §3.

use std::path::PathBuf;

use tracing::level_filters::LevelFilter;

use crate::cli::CliArgs;

/// Fully resolved `gameconqueror` configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Pid to attach to immediately on startup, if any.
    pub pid: Option<u32>,
    /// Path the file-only `tracing` subscriber appends to.
    pub log_file: PathBuf,
    /// Level filter for the `tracing` subscriber.
    pub log_level: LevelFilter,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pid: None,
            log_file: std::env::temp_dir().join("gameconqueror.log"),
            log_level: LevelFilter::WARN,
        }
    }
}

/// Resolves `Settings` from `cli`, merging it over the built-in defaults: CLI arguments win.
pub fn resolve(cli: &CliArgs) -> Settings {
    let mut settings = Settings::default();

    if cli.pid.is_some() {
        settings.pid = cli.pid;
    }

    settings
}

#[cfg(test)]
mod tests;
