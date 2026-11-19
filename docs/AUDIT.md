# Audit

A whole-application audit of libscanmem, scanmem and gameconqueror
(2026-11-19) in five parts: bugs (§2), performance (§3), security and
robustness (§4), usability (§5), and code quality, tests and project hygiene
(§6). §7 lists what was left open, and §8 is the checklist to re-run the
audit. The baseline is 9b37a15.

Each finding says what was found, how serious it is, and what was done:
**fixed** (with the commit), **accepted** (with the reason), or **OK**
(checked, nothing to do). Every fix is its own commit, with a regression test
where one can be written.

Severity: **high** (wrong results, a target left frozen, or an interface that
stops responding in normal use), **medium** (wrong behavior in a reachable
case, a panic in debug builds, an operation too slow to use), **low**
(hardening, polish, or needs an unusual setup).

## 1. Method

Every source file was read in full. Findings were then reproduced in three
ways, always against target processes the audit started itself (Python
processes that call `prctl(PR_SET_PTRACER, PR_SET_PTRACER_ANY)`, so
`ptrace_scope` 1 allows them), never against other running programs:

- **Idle targets** of 16, 64 and 256 MiB of patterned data with a known
  `u32` at 100 places. The base and new release builds ran the same
  `scanmem --exec` scripts on the same target, and their full output
  (match lists included) was compared byte for byte. Wall time, CPU and peak
  RSS came from `wait4`.
- **Signal targets** that count the SIGUSR1s they receive and print a
  heartbeat, to see whether an attached target keeps running and gets its
  signals.
- **The release binaries in a pty**: the scanmem REPL (Ctrl-C, quitting
  mid-scan) and gameconqueror (scan through the keys, then idle CPU from
  `/proc/<pid>/stat` over 5 s and the time for a Down arrow to redraw).

Builds checked: default, `--no-default-features`, `--all-features`. Static
checks: `cargo clippy` at the three gate levels, `cargo deny check`, `typos`,
`cargo llvm-cov`. The `#[ignore]`d integration tests, which spawn targets,
were run on every commit with `--ignored`, and every test binary was run 20
times in parallel to look for flaky tests.

## 2. Bugs

| Area | Finding | Status |
| ---- | ------- | ------ |
| Target frozen by signals | Attaching made the target a ptrace tracee for the whole session. Its first signal (SIGCHLD, SIGALRM, a terminal resize, Ctrl-C in its terminal) put it in signal-delivery-stop. Nothing ever waited for it, so it stayed frozen without the signal, and was left stopped (`T`) after scanmem or gameconqueror quit. Attaching now only opens `/proc/<pid>/mem`. A scan stops the target with `PTRACE_SEIZE` + `PTRACE_INTERRUPT` (no SIGSTOP) and detaches afterwards, delivering any signal that arrived meanwhile. Checked with a signal target: before, frozen at the first SIGUSR1; after, it runs and counts every signal. | Fixed, high (37c2b71) |
| Narrowing overlapping matches | A match's bytes were taken to run up to the next match. Matches overlap constantly (four zero bytes are an `i32 = 0` match at every offset), so narrowing tested most of them on one or two bytes. On an idle 16 MiB target, `scan i32 = 0` twice went from 3,774,915 matches to 121,501, and `scan i64 = 0; scan i64 unchanged` from 3,311,565 to 58,398. Overlapping string matches were dropped, and `list` showed an overlapped f64 as an integer ("= 215"). Matches are now re-tested on their full width with their full old value, never gain a width, and keep their filler bytes across chunks. After the fix: 3,774,907 and 3,311,565 (the Python heap moves a little). | Fixed, high (05a42ff) |
| Ctrl-C in the REPL | Ctrl-C during a scan killed scanmem (and with it the session). One press now cancels the scan and keeps the partial matches ("stopped early: partial results"); a second press exits. Checked in a pty. | Fixed, medium (a2dc169) |
| "Increased by" overflow | `old + delta` on the tested width panicked in debug builds (an `i8` of 120 increased by 10). Integers wrap, as release builds and C scanmem did; floats are unchanged. | Fixed, medium (72d4e28) |
| Frozen cheats | gameconqueror rewrote frozen values only after a full poll interval (100 ms) with no key pressed. Key repeat never allows that, so holding an arrow key let the game change a "frozen" value. Ticks now run on schedule whatever the input. | Fixed, medium (fb7f26d) |
| Quitting mid-scan | Killing scanmem or gameconqueror during a scan leaves the target running (`S`), before and after the fixes: the kernel detaches tracees of a tracer that exits. | OK |
| Cheat lists | Save and load round-trip `u64::MAX`, `i64::MIN`, NaN, infinity, and strings with quotes, newlines and NUL. | OK |

## 3. Performance

Release builds; idle targets as in §1; median of 3 runs unless noted.

| Area | Finding | Status |
| ---- | ------- | ------ |
| First scans | For each byte, the scan stored a 16-byte `Option` in a vector (64 MiB per 4 MiB chunk) and picked each width's value out of the user's input again. Scans now stream, resolve the widths once, and for equal-to scans jump to the bytes a match can start with. On 256 MiB, `scan i32 = v` 3.12 s → **0.15 s** and peak RSS 103 → **14 MiB**; `scan any = v` 7.97 → **0.32 s**; `snapshot` 9.93 → **3.03 s** (1.19 → 1.11 GiB). Output was byte-identical to the previous build for 12–15 scripts covering every data type family, swapped byte order, narrowing and snapshots. | Fixed (3afa1d2, cef6b64) |
| Narrowing | Snapshot then `scan any unchanged` on 64 MiB (73 million matches): 4.58 → **3.69 s**, peak RSS 655 → **571 MiB**, now with correct results (§2). | Fixed (3afa1d2, 05a42ff) |
| `delete` | Each index was found by walking from the first match and removed by rebuilding the whole store: O(k·N). With 4.8 million matches, `delete 0-999` took **49.7 s → 0.28 s** (scan included), and `delete !0` didn't finish in 2 minutes (now 0.55 s). The REPL also decoded every match after each command just to count them. | Fixed, medium (ad689ce) |
| gameconqueror with many matches | Every frame (one per 100 ms, and one per key) decoded, filtered and sorted all matches and formatted a row for each. With 3.7 million matches: idle CPU **95.6 % → 0.4 %**, a Down arrow **0.7–2.0 s → 1 ms**, RSS **3.9 GiB → 565 MiB**. The list is now rebuilt only when the matches, filter or sort change, and only visible rows are drawn. | Fixed, high (da46d8c) |
| Snapshot memory | A snapshot keeps 4 bytes per byte of memory scanned (old value and width flags), like C scanmem: 1.1 GiB for a 256 MiB target. | Accepted |
| Change highlighting | gameconqueror keeps every match's previous value in a `HashMap` to highlight changes: most of the remaining 565 MiB with 3.7 million matches. It's rebuilt once per scan, not per frame. | Accepted |

Not measured: real games (only idle Python targets), targets over 256 MiB,
how long a scan keeps a multi-threaded target stopped, gameconqueror's
value sort and filter typing with millions of matches (each keystroke in the
filter rebuilds the list once, see §7), and the hex view.

## 4. Security and robustness

| Area | Finding | Status |
| ---- | ------- | ------ |
| gameconqueror log in `/tmp` | The default log was `/tmp/gameconqueror.log`, opened following symlinks, and the docs suggest `sudo gameconqueror`. Another local user could create that file first, as a symlink for root to append to or as their own file to read every address and value written. `fs.protected_symlinks` and `fs.protected_regular` block both on most distributions (logging then silently stopped). The log is now `$XDG_STATE_HOME/gameconqueror/gameconqueror.log`, created `0600` in a `0700` directory, and a symlink in its place is refused (`O_NOFOLLOW`). | Fixed, low (c471a21) |
| Scan thread panic | A panic in gameconqueror's background scan dropped the session on the worker thread. Only the thread that stopped the target can resume it, so the target stayed stopped until exit, with no session to detach. The panic is now caught and the session returned with it as an error. | Fixed, low (147c613) |
| Unsafe code | Confined to `process/ptrace.rs` (`ptrace` with integer arguments, `waitpid` into a local); every call has a SAFETY comment. Memory reads and writes go through `/proc/<pid>/mem`, which the kernel checks against `ptrace_scope` when it's opened. | OK |
| Stop and resume on one thread | Both front ends stop and resume the target on the main thread; gameconqueror moves only the `/proc/<pid>/mem` reading to its worker. Documented on `Session::prepare_scan`. | OK |
| Dependencies | `cargo deny check` passes and the lockfile is current. | OK |

## 5. Usability

| Area | Finding | Status |
| ---- | ------- | ------ |
| Attach errors | A refused attach printed "Permission denied (os error 13)", a zombie target "ptrace failed (errno 1)" or "No such process (os error 3)", and `ScanmemError::PermissionDenied` was never produced. The permission error now points at `ptrace_scope`, root and `CAP_SYS_PTRACE`; a missing pid is "no such process" (new `NoSuchProcess`), and a dead target "the target process has exited". | Fixed, low (7d15863) |
| `--exec` without a target | When `--pid` failed to attach, the script still ran and every command failed with "no process is attached". scanmem now exits with status 1 straight away. | Fixed, low (7d15863) |
| Log location docs | `docs/gameconqueror.md` and the example config describe the new default; the example notes that `~` isn't expanded in `log-file`. | Fixed, docs (c471a21) |

## 6. Code quality, tests and hygiene

| Area | Finding | Status |
| ---- | ------- | ------ |
| Ignored integration tests | The `#[ignore]`d suites, which CI never runs, failed on the base. They assumed exactly one match where `fake_target` also has a stack copy of the value. The scanmem test needed `CAP_SYS_PTRACE` because `fake_target` is not scanmem's child. `fake_target` could exit before the narrowing scan. Fixed with known addresses, `PR_SET_PTRACER_ANY` in `fake_target`, and a separate "done" value. They all pass now, with the ones this audit added. | Fixed (1784b71) |
| Shared temp path | The logger test wrote a fixed `/tmp` file, so parallel runs deleted each other's. | Fixed (bc1b64a) |
| New tests | Signal delivery while attached, Ctrl-C cancelling, wrapping deltas, the scan shortcuts against testing every width, overlapping narrowing, chunked narrowing, one-pass delete, scrolling, tick scheduling, error mapping, log permissions and symlinks, and the panicking scan. Every test binary passed 20 parallel runs after each commit. None depends on timing. | Fixed |
| Coverage | 73.0 % → 72.6 % of lines without the ignored tests (`cargo llvm-cov --workspace --all-features`); the new ptrace code runs only in the ignored suites. With them, **81.8 %**. The base has no comparable figure, because its ignored suite fails. `ui/mod.rs` (18 %) and the `main.rs` files run only as real binaries and were checked in a pty. | Accepted |
| Cargo warnings | `package.homepage` repeated `package.repository` in all three crates, and `workspace.package.description` was used by none; every build warned. | Fixed (808bad5) |
| Gates | fmt, typos, deny, clippy and tests at the three feature levels, the ignored suites, and docs, pass on every commit. | OK |

## 7. Left open

- **CI doesn't run the ignored suites.** They need `ptrace_scope` ≤ 1 and a
  workspace build (for `fake_target`); Ubuntu's default `ptrace_scope` is 1.
  A job running `cargo build --workspace && cargo test --workspace
  --all-features -- --ignored` would have caught §6's broken suites and
  §2's signal freeze.
- **`fake_target` ships with libscanmem.** It's a test helper in
  `libscanmem/src/bin/`, so it's packaged with the library and built by every
  `cargo build --workspace`. A `publish = false` helper crate, or an example,
  would keep it out of the package.
- **Large floats print every digit.** `list` shows an f64 of 7.2e104 as a
  105-digit number. Scientific notation above some magnitude would read
  better, but it changes the output format.
- **Filtering millions of matches.** Each keystroke in gameconqueror's match
  filter formats every address and value once. Filtering the already
  filtered list while the query only grows would make typing cheaper.
- **Change highlighting memory** (§3). Keeping previous values for visible
  rows only, or in the address-sorted list itself, would save most of it.

## 8. Re-running the audit

```sh
# Gates (as in CI), plus the ignored suites that spawn their own targets
cargo fmt --all -- --check && typos && cargo deny check
for f in --no-default-features "" --all-features; do
  cargo clippy --workspace --all-targets $f -- -D warnings
  cargo test --workspace $f
done
cargo build --workspace && cargo test --workspace --all-features -- --ignored
cargo doc --workspace --no-deps

# Coverage (system LLVM tools on Arch: LLVM_COV / LLVM_PROFDATA)
LLVM_COV=/usr/bin/llvm-cov LLVM_PROFDATA=/usr/bin/llvm-profdata \
  cargo llvm-cov --workspace --all-features --summary-only -- --include-ignored
```

For timings, start a target that allows any tracer and holds a known value,
for example:

```python
import ctypes, os, signal
ctypes.CDLL(None).prctl(0x59616D61, ctypes.c_ulong(-1), 0, 0, 0)  # PR_SET_PTRACER_ANY
data = bytearray(os.urandom(64 << 20))
for i in range(100):
    data[i * 4096 + 40 : i * 4096 + 44] = (0x13579BDF).to_bytes(4, "little")
print(os.getpid(), flush=True)
signal.pause()
```

and time `scanmem --pid <pid> --exec "scan i32 = 324508639"` (or
`snapshot`, `scan any = 0; delete 0-999`). Compare match counts of a scan
repeated on unchanged memory (`scan i32 = 0; scan i32 = 0`): they should be
equal.
