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
