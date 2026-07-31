//! Argument definitions — parsed only in `main`.

use clap::Parser;

/// `gameconqueror` command-line arguments.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// Attach to this pid immediately on startup.
    #[arg(long)]
    pub pid: Option<u32>,
}

#[cfg(test)]
mod tests;
