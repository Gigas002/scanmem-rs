//! Known-memory-layout helper process for `libscanmem`/`gameconqueror` integration tests —
//! replaces `test/memfake.c`.
//!
//! Protocol: prints the address of a known `u32` local, waits (polling the actual memory, not a
//! cached register) for that value to change from its initial value or for a timeout, then
//! prints the value it observed and exits. Since attaching no longer freezes the target for the
//! whole session (only actual scans briefly pause it — see `libscanmem::process::Process::stop`),
//! this process keeps running and polling the whole time it's attached, so a harness that writes
//! more than one value to it races the plain "changed from initial" check below; pass the final
//! expected value as `argv[1]` (bare hex, no `0x` prefix) to wait for that exact value instead
//! and ignore intermediate writes.
use std::io::Write;
use std::ptr;
use std::time::{Duration, Instant};

const INITIAL: u32 = 0xdead_beef;
const TIMEOUT: Duration = Duration::from_secs(10);

fn main() {
    let mut value: u32 = INITIAL;
    let address = ptr::addr_of!(value) as usize;
    println!("{address}");
    std::io::stdout().flush().ok();

    let expected = std::env::args()
        .nth(1)
        .map(|arg| u32::from_str_radix(&arg, 16).expect("argv[1] must be a bare-hex u32"));

    let ptr = ptr::addr_of_mut!(value);
    let deadline = Instant::now() + TIMEOUT;
    loop {
        // SAFETY: `ptr` stays valid for the whole function; a volatile read forces an actual
        // memory access instead of an optimizer-cached register value, since a harness process
        // may rewrite this address via `/proc/<pid>/mem` at any time.
        let current = unsafe { ptr::read_volatile(ptr) };
        let observed = match expected {
            Some(expected) => current == expected,
            None => current != INITIAL,
        };
        if observed || Instant::now() >= deadline {
            println!("{current}");
            std::io::stdout().flush().ok();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
