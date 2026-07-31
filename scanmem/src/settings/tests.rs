use clap::Parser;
use tracing::level_filters::LevelFilter;

use super::{Settings, resolve};
use crate::cli::CliArgs;

fn cli(args: &[&str]) -> CliArgs {
    let mut full = vec!["scanmem"];
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
    assert_eq!(settings.pid.unwrap().as_raw_pid(), 1234);
}

#[test]
fn cli_verbosity_sets_log_level() {
    let settings = resolve(&cli(&["-vv"]));
    assert_eq!(settings.log_level, LevelFilter::DEBUG);
}
