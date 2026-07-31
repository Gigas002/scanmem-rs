//! End-to-end `--exec` scripting tests driven by `assert_cmd`, against the built `scanmem`
//! binary — replaces `test/sm_test.sh`'s intent as native `cargo test`, per the crate plan.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};

use assert_cmd::Command;
use predicates::prelude::*;

/// `fake_target` is a `libscanmem` binary, not `scanmem`'s own, so Cargo doesn't expose a
/// `CARGO_BIN_EXE_fake_target` env var for it here; it lands next to `scanmem`'s own executable
/// in the same workspace target directory, since both are built for the same profile.
fn fake_target_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_scanmem"))
        .parent()
        .expect("scanmem executable has a parent directory")
        .join("fake_target")
}

#[test]
fn exec_help_prints_the_verb_table() {
    Command::cargo_bin("scanmem")
        .unwrap()
        .args(["--exec", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("quit"));
}

#[test]
fn exec_runs_multiple_semicolon_separated_commands() {
    Command::cargo_bin("scanmem")
        .unwrap()
        .args(["--exec", "help; help"])
        .assert()
        .success();
}

#[test]
fn exec_stops_at_quit_and_ignores_trailing_commands() {
    Command::cargo_bin("scanmem")
        .unwrap()
        .args(["--exec", "help; quit; bogus"])
        .assert()
        .success();
}

#[test]
fn exec_reports_failure_on_unknown_command() {
    Command::cargo_bin("scanmem")
        .unwrap()
        .args(["--exec", "bogus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown command"));
}

#[test]
fn exec_reports_failure_when_a_command_is_not_attached() {
    Command::cargo_bin("scanmem")
        .unwrap()
        .args(["--exec", "list"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("no process is attached"));
}

#[test]
#[ignore = "requires CAP_SYS_PTRACE and a workspace build (for fake_target); run manually with \
            `cargo build --workspace && cargo test -p scanmem --test integration -- --ignored`"]
fn exec_attaches_scans_writes_and_verifies_via_target_stdout() {
    let mut child = StdCommand::new(fake_target_path())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn fake_target");

    let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut address_line = String::new();
    stdout
        .read_line(&mut address_line)
        .expect("failed to read address line");
    let address: usize = address_line
        .trim()
        .parse()
        .expect("fake_target did not print a valid address");

    let initial = 0xdead_beef_u32;
    let new_value = 0x1234_5678_u32;
    let script = format!(
        "attach {pid}; scan i32 = {initial}; list; write {address} i32 {new_value}; scan i32 = {new_value}; list",
        pid = child.id(),
    );

    Command::cargo_bin("scanmem")
        .unwrap()
        .args(["--exec", &script])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 match(es)").count(2))
        .stdout(predicate::str::contains("no matches").not());

    let mut result_line = String::new();
    stdout
        .read_line(&mut result_line)
        .expect("failed to read result line");
    let observed: u32 = result_line
        .trim()
        .parse()
        .expect("fake_target did not print a valid value");
    assert_eq!(observed, new_value);

    let status = child.wait().expect("fake_target did not exit cleanly");
    assert!(status.success());
}
