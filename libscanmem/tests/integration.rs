//! End-to-end ptrace attach/read/write against `fake_target`, mirroring the intent of
//! `test/sm_test.sh` as native `cargo test`.
//!
//! Requires `CAP_SYS_PTRACE` (or root, or a permissive
//! `/proc/sys/kernel/yama/ptrace_scope`) and is therefore `#[ignore]`d by default. Run manually
//! with:
//!
//! ```sh
//! cargo test -p libscanmem --test integration -- --ignored
//! ```

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use libscanmem::process::Process;
use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::session::{ScanCriterion, ScanExpr, Session};
use libscanmem::value::{UserValue, Value, parse_int};
use rustix::process::Pid;

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
fn attach_read_write_and_verify_via_target_stdout() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fake_target"))
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

    let pid = Pid::from_raw(child.id() as i32).expect("child pid is non-zero");
    let process = Process::attach(pid).expect("attach failed");

    let original = process.read(address, 4).expect("read failed");
    assert_eq!(original, 0xdead_beef_u32.to_ne_bytes());

    let new_value = 0x1234_5678_u32;
    process
        .write(address, &new_value.to_ne_bytes())
        .expect("write failed");
    process.detach().expect("detach failed");

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

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
fn session_attach_scan_narrow_write_and_verify_via_target_stdout() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fake_target"))
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

    let pid = Pid::from_raw(child.id() as i32).expect("child pid is non-zero");
    let mut session = Session::attach(pid).expect("attach failed");

    let initial = parse_int("0xdeadbeef").expect("valid literal");
    let stats = session
        .scan(&ScanExpr {
            data_type: ScanDataType::Integer32,
            match_type: MatchType::EqualTo,
            criterion: ScanCriterion::Value(UserValue::Number(initial)),
        })
        .expect("first scan failed");
    assert_eq!(stats.matches, 1);
    assert_eq!(
        session.nth_match(0).expect("expected one match").address,
        address
    );

    let new_value = 0x1234_5678_u32;
    session
        .write(address, &Value::U32(new_value))
        .expect("write failed");

    let narrowed = parse_int("0x12345678").expect("valid literal");
    let stats = session
        .scan(&ScanExpr {
            data_type: ScanDataType::Integer32,
            match_type: MatchType::EqualTo,
            criterion: ScanCriterion::Value(UserValue::Number(narrowed)),
        })
        .expect("narrow scan failed");
    assert_eq!(stats.matches, 1);
    assert_eq!(
        session
            .nth_match(0)
            .expect("expected one match after narrowing")
            .address,
        address
    );

    session.detach().expect("detach failed");

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

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
fn run_new_scan_discards_existing_matches_and_rescans_from_scratch() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fake_target"))
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn fake_target");

    let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut address_line = String::new();
    stdout
        .read_line(&mut address_line)
        .expect("failed to read address line");

    let pid = Pid::from_raw(child.id() as i32).expect("child pid is non-zero");
    let mut session = Session::attach(pid).expect("attach failed");

    // 0xdeadbeef is a common poison value and may coincidentally appear more than once in the
    // target's writable memory, same caveat as the sibling scan test above — this test only
    // cares that the *next* scan finds substantially more, not the exact starting count.
    let initial = parse_int("0xdeadbeef").expect("valid literal");
    let stats = session
        .run_scan(&ScanExpr {
            data_type: ScanDataType::Integer32,
            match_type: MatchType::EqualTo,
            criterion: ScanCriterion::Value(UserValue::Number(initial)),
        })
        .expect("first scan failed");
    let initial_matches = stats.matches;
    assert!(initial_matches >= 1);

    // `run_scan` with a broad `Any` criterion would only narrow these existing matches (and keep
    // them, since `Any` always matches); `run_new_scan` must instead discard them and rescan
    // every byte from scratch, finding far more candidates.
    let stats = session
        .run_new_scan(&ScanExpr {
            data_type: ScanDataType::Integer8,
            match_type: MatchType::Any,
            criterion: ScanCriterion::None,
        })
        .expect("new scan failed");
    assert!(
        stats.matches > initial_matches * 100,
        "expected run_new_scan to rescan every byte, not just narrow the previous {} match(es); \
         got {} match(es)",
        initial_matches,
        stats.matches
    );

    session.detach().expect("detach failed");
    child.kill().expect("failed to kill fake_target");
    child.wait().expect("fake_target did not exit cleanly");
}

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
fn refresh_matches_updates_values_in_place_without_dropping_any() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fake_target"))
        .arg("ffffffff") // keep the target alive regardless of what this test writes
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

    let pid = Pid::from_raw(child.id() as i32).expect("child pid is non-zero");
    let mut session = Session::attach(pid).expect("attach failed");

    // Same 0xdeadbeef-may-coincidentally-repeat caveat as the sibling scan tests — only the
    // known `address`'s match is asserted on below, not the total count.
    let initial = parse_int("0xdeadbeef").expect("valid literal");
    let stats = session
        .run_scan(&ScanExpr {
            data_type: ScanDataType::Integer32,
            match_type: MatchType::EqualTo,
            criterion: ScanCriterion::Value(UserValue::Number(initial)),
        })
        .expect("first scan failed");
    let initial_matches = stats.matches;
    assert!(initial_matches >= 1);

    let new_value = 0x1234_5678_u32;
    session
        .write(address, &Value::U32(new_value))
        .expect("write failed");

    let stats = session.refresh_matches().expect("refresh failed");
    assert_eq!(
        stats.matches, initial_matches,
        "refresh_matches must never drop a match just because its value changed"
    );
    let refreshed = session
        .matches()
        .find(|entry| entry.address == address)
        .expect("expected the known address to still be a match");
    assert_eq!(refreshed.old_value, Value::U32(new_value));

    session.detach().expect("detach failed");
    child.kill().expect("failed to kill fake_target");
    child.wait().expect("fake_target did not exit cleanly");
}
