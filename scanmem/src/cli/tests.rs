use clap::Parser;

use super::CliArgs;

#[test]
fn parses_pid_and_verbose() {
    let args = CliArgs::parse_from(["scanmem", "--pid", "1234", "-vv"]);
    assert_eq!(args.pid, Some(1234));
    assert_eq!(args.verbose, 2);
}

#[test]
fn defaults_are_empty() {
    let args = CliArgs::parse_from(["scanmem"]);
    assert_eq!(args.pid, None);
    assert_eq!(args.exec, None);
    assert_eq!(args.verbose, 0);
}

#[test]
fn parses_exec_script() {
    let args = CliArgs::parse_from(["scanmem", "--exec", "attach 1234;list"]);
    assert_eq!(args.exec.as_deref(), Some("attach 1234;list"));
}

#[test]
fn rejects_unknown_flags() {
    assert!(CliArgs::try_parse_from(["scanmem", "--bogus"]).is_err());
}
