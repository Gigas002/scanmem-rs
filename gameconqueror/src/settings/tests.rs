use clap::Parser;

use super::{Settings, resolve};
use crate::cli::CliArgs;

fn cli(args: &[&str]) -> CliArgs {
    let mut full = vec!["gameconqueror"];
    full.extend_from_slice(args);
    CliArgs::parse_from(full)
}

#[test]
fn defaults_when_nothing_is_provided() {
    let settings = resolve(&cli(&[]));
    assert_eq!(settings, Settings::default());
}

#[test]
fn cli_pid_is_resolved() {
    let settings = resolve(&cli(&["--pid", "1234"]));
    assert_eq!(settings.pid, Some(1234));
}
