use clap::Parser;

use super::CliArgs;

#[test]
fn parses_pid() {
    let args = CliArgs::parse_from(["gameconqueror", "--pid", "1234"]);
    assert_eq!(args.pid, Some(1234));
}

#[test]
fn defaults_are_empty() {
    let args = CliArgs::parse_from(["gameconqueror"]);
    assert_eq!(args.pid, None);
}

#[test]
fn rejects_unknown_flags() {
    assert!(CliArgs::try_parse_from(["gameconqueror", "--bogus"]).is_err());
}
