//! Merges CLI + defaults into a single resolved [`Settings`] — the only type passed below this
//! module, per `ARCHITECTURE.md` §3.

use rustix::process::Pid;
use tracing::level_filters::LevelFilter;

use crate::cli::CliArgs;

/// Fully resolved `scanmem` configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Pid to attach to immediately on startup, if any.
    pub pid: Option<Pid>,
    /// One-shot `;`-separated command script to run instead of the interactive REPL, if any.
    pub exec: Option<String>,
    /// Level filter for the `tracing` subscriber.
    pub log_level: LevelFilter,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pid: None,
            exec: None,
            log_level: LevelFilter::WARN,
        }
    }
}

/// Merges `cli` over the built-in defaults: CLI arguments win.
pub fn resolve(cli: &CliArgs) -> Settings {
    let mut settings = Settings::default();

    if cli.verbose > 0 {
        settings.log_level = level_from_verbosity(cli.verbose);
    }

    settings.pid = cli.pid.and_then(|raw| Pid::from_raw(raw as i32));
    settings.exec = cli.exec.clone();

    settings
}

fn level_from_verbosity(verbose: u8) -> LevelFilter {
    match verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

#[cfg(test)]
mod tests;
