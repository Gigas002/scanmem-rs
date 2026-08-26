//! Argument definitions — parsed only in `main`.

#[cfg(feature = "config")]
use std::path::PathBuf;

use clap::Parser;

/// `gameconqueror` command-line arguments.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// Attach to this pid immediately on startup.
    #[arg(long)]
    pub pid: Option<u32>,
    /// Path to a `config.toml`, overriding the conventional
    /// `$XDG_CONFIG_HOME/gameconqueror/config.toml`.
    #[cfg(feature = "config")]
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Path to a `theme.toml`, overriding the conventional
    /// `$XDG_CONFIG_HOME/gameconqueror/theme.toml`.
    #[cfg(feature = "config")]
    #[arg(long)]
    pub theme: Option<PathBuf>,
}

#[cfg(test)]
mod tests;
