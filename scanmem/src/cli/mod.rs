//! Argument and subcommand definitions — parsed only in `main`.

use clap::Parser;

/// `scanmem` command-line arguments.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// Attach to this pid immediately on startup.
    #[arg(long)]
    pub pid: Option<u32>,

    /// Increase log verbosity; may be repeated (-v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[cfg(test)]
mod tests;
