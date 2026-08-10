//! `app::update()` end-to-end against `libscanmem`'s `fake_target` helper — no terminal/`ratatui`
//! involved, mirroring the intent of `libscanmem`'s and `scanmem`'s own `fake_target` integration
//! tests.
//!
//! Requires `CAP_SYS_PTRACE` (or root, or a permissive
//! `/proc/sys/kernel/yama/ptrace_scope`) and is therefore `#[ignore]`d by default. Run manually
//! with:
//!
//! ```sh
//! cargo build --workspace && cargo test -p gameconqueror --test integration -- --ignored
//! ```

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use gameconqueror::app::{AppState, Focus, Msg, StatusLevel, update};
#[cfg(feature = "cheat-list")]
use libscanmem::scanroutines::{MatchType, ScanDataType};
#[cfg(feature = "cheat-list")]
use libscanmem::session::{ScanCriterion, ScanExpr};
use libscanmem::value::Value;
#[cfg(feature = "cheat-list")]
use libscanmem::value::{UserValue, parse_int};

/// `fake_target` is a `libscanmem` binary, not `gameconqueror`'s own, so Cargo doesn't expose a
/// `CARGO_BIN_EXE_fake_target` env var for it here; it lands next to `gameconqueror`'s own
/// executable in the same workspace target directory, since both are built for the same profile.
fn fake_target_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_gameconqueror"))
        .parent()
        .expect("gameconqueror executable has a parent directory")
        .join("fake_target")
}

/// `Msg::RunScan`/`Msg::Snapshot` run on a background thread (so the real TUI never blocks on
/// one), so unlike every other `Msg` here their result isn't available the instant `update()`
/// returns — dispatch `Msg::PollScan` (what the real event loop does every tick) until
/// `AppState::is_scanning` clears, standing in for that loop.
fn wait_for_scan(state: &mut AppState) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while state.is_scanning() {
        assert!(Instant::now() < deadline, "scan did not finish in time");
        update(state, Msg::PollScan);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
#[cfg(feature = "cheat-list")]
fn attach_scan_narrow_write_and_verify_via_target_stdout() {
    // The target now keeps running while attached (only an actual scan briefly pauses it), so it
    // observes every write this test makes, not just the last one; tell it the final expected
    // value up front so it ignores the intermediate `new_value` write below and waits for this.
    let frozen_value = 0x2468_ace0_u32;
    let mut child = Command::new(fake_target_path())
        .arg(format!("{frozen_value:x}"))
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

    let mut state = AppState::default();
    update(&mut state, Msg::Attach(child.id()));
    assert!(
        state.session().is_some(),
        "attach failed: {:?}",
        state.status()
    );
    assert_eq!(state.attached().unwrap().pid, child.id());

    let initial = parse_int("0xdeadbeef").expect("valid literal");
    update(
        &mut state,
        Msg::Scan(ScanExpr {
            data_type: ScanDataType::Integer32,
            match_type: MatchType::EqualTo,
            criterion: ScanCriterion::Value(UserValue::Number(initial)),
        }),
    );
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    assert_eq!(
        state.session().unwrap().nth_match(0).unwrap().address,
        address
    );

    let new_value = 0x1234_5678_u32;
    update(
        &mut state,
        Msg::Write {
            address,
            value: Value::U32(new_value),
        },
    );
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);

    let narrowed = parse_int("0x12345678").expect("valid literal");
    update(
        &mut state,
        Msg::Scan(ScanExpr {
            data_type: ScanDataType::Integer32,
            match_type: MatchType::EqualTo,
            criterion: ScanCriterion::Value(UserValue::Number(narrowed)),
        }),
    );
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    assert_eq!(
        state.session().unwrap().nth_match(0).unwrap().address,
        address
    );

    update(
        &mut state,
        Msg::AddCheat {
            address,
            description: "narrowed value".to_owned(),
            value: Value::U32(new_value),
        },
    );
    assert_eq!(state.cheats().len(), 1);

    update(
        &mut state,
        Msg::EditCheatValue {
            index: 0,
            value: Value::U32(frozen_value),
        },
    );
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    assert_eq!(state.cheats()[0].value, Value::U32(frozen_value));

    update(&mut state, Msg::Detach);
    assert!(state.session().is_none());
    assert!(state.attached().is_none());
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);

    let mut result_line = String::new();
    stdout
        .read_line(&mut result_line)
        .expect("failed to read result line");
    let observed: u32 = result_line
        .trim()
        .parse()
        .expect("fake_target did not print a valid value");
    assert_eq!(observed, frozen_value);

    let status = child.wait().expect("fake_target did not exit cleanly");
    assert!(status.success());
}

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
#[cfg(feature = "cheat-list")]
fn freezing_a_cheat_rewrites_it_on_every_tick() {
    // See the comment in `attach_scan_narrow_write_and_verify_via_target_stdout`: the target
    // keeps running while attached, so it could in principle observe the intermediate
    // `0x1111_1111` write below instead of the tick's rewrite; tell it the final expected value
    // up front so it waits for that regardless of scheduling.
    let frozen_value = 0x2468_ace0_u32;
    let mut child = Command::new(fake_target_path())
        .arg(format!("{frozen_value:x}"))
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

    let mut state = AppState::default();
    update(&mut state, Msg::Attach(child.id()));
    assert!(
        state.session().is_some(),
        "attach failed: {:?}",
        state.status()
    );

    update(
        &mut state,
        Msg::AddCheat {
            address,
            description: "frozen".to_owned(),
            value: Value::U32(frozen_value),
        },
    );
    update(&mut state, Msg::ToggleFreeze(0));
    assert!(state.cheats()[0].frozen);

    // Simulate the target (or another tool) overwriting the frozen address between ticks.
    update(
        &mut state,
        Msg::Write {
            address,
            value: Value::U32(0x1111_1111),
        },
    );
    update(&mut state, Msg::Tick);

    update(&mut state, Msg::Detach);
    assert!(state.session().is_none());

    let mut result_line = String::new();
    stdout
        .read_line(&mut result_line)
        .expect("failed to read result line");
    let observed: u32 = result_line
        .trim()
        .parse()
        .expect("fake_target did not print a valid value");
    assert_eq!(observed, frozen_value);

    let status = child.wait().expect("fake_target did not exit cleanly");
    assert!(status.success());
}

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
fn scan_panel_run_scan_first_scan_and_narrow_via_typed_input() {
    // The target now keeps running while attached (only an actual scan briefly pauses it), so
    // its watch loop would otherwise notice this test's `Write` below and exit on its own before
    // the second scan/detach — this test manages the child's lifetime itself (`child.kill()`
    // below) and doesn't care what value fake_target ends up observing, so give it a sentinel
    // it'll never actually see written to keep it running until then.
    let mut child = Command::new(fake_target_path())
        .arg("ffffffff")
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

    let mut state = AppState::default();
    update(&mut state, Msg::Attach(child.id()));
    assert!(
        state.session().is_some(),
        "attach failed: {:?}",
        state.status()
    );

    // Default Scan Panel state is Integer32/EqualTo; `fake_target` seeds 0xdeadbeef. The exact
    // match count isn't asserted: 0xdeadbeef is a common poison value and may coincidentally
    // appear elsewhere in the target's writable memory, same as the sibling test above only
    // checks that the known address is present rather than the total count.
    update(&mut state, Msg::SetScanInput("0xdeadbeef".to_owned()));
    update(&mut state, Msg::RunScan);
    wait_for_scan(&mut state);
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    let matches = state.filtered_matches();
    assert!(matches.iter().any(|entry| entry.address == address));

    let new_value = 0x1234_5678_u32;
    update(
        &mut state,
        Msg::Write {
            address,
            value: Value::U32(new_value),
        },
    );
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);

    update(&mut state, Msg::SetScanInput("0x12345678".to_owned()));
    update(&mut state, Msg::RunScan);
    wait_for_scan(&mut state);
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    let narrowed = state.filtered_matches();
    assert!(narrowed.iter().any(|entry| entry.address == address));

    update(&mut state, Msg::Detach);
    assert!(
        state.session().is_none(),
        "detach status: {:?}",
        state.status()
    );

    child.kill().expect("failed to kill fake_target");
    child.wait().expect("fake_target did not exit cleanly");
}

#[test]
#[ignore = "requires CAP_SYS_PTRACE; run manually with `--ignored`"]
fn hex_view_focus_and_commit_edit_writes_a_single_byte() {
    let mut child = Command::new(fake_target_path())
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

    let mut state = AppState::default();
    update(&mut state, Msg::Attach(child.id()));
    assert!(
        state.session().is_some(),
        "attach failed: {:?}",
        state.status()
    );

    update(&mut state, Msg::FocusHexView(address));
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    assert_eq!(state.focus(), Focus::HexView);
    assert_eq!(state.hex_cursor_address(), Some(address));

    // `fake_target` seeds 0xdeadbeef little-endian; overwriting the low byte with 0x42 yields
    // 0xdeadbe42, which is enough to trip its "value changed" watch loop.
    update(&mut state, Msg::SetHexEditInput("42".to_owned()));
    update(&mut state, Msg::CommitHexEdit);
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    assert_eq!(state.hex_buffer()[state.hex_cursor()], 0x42);

    update(&mut state, Msg::Detach);
    assert!(state.session().is_none());

    let mut result_line = String::new();
    stdout
        .read_line(&mut result_line)
        .expect("failed to read result line");
    let observed: u32 = result_line
        .trim()
        .parse()
        .expect("fake_target did not print a valid value");
    assert_eq!(observed, 0xdeadbe42);

    let status = child.wait().expect("fake_target did not exit cleanly");
    assert!(status.success());
}
