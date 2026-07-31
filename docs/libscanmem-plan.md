# libscanmem — implementation plan

This document is the **human roadmap** and **agent playbook** for **`libscanmem`**: a Rust port of
scanmem's engine (`scanmem.c`, `commands.c`, `handlers.c`, `ptrace.c`, `maps.c`, `targetmem.c`, `value.c`,
`scanroutines.c`, `sets.c`, `common.c`, `interrupt.c`, `show_message.c`) as a **library crate with a
structured, typed API** — no text-command protocol, no CLI, no UI concerns.

Background analysis: [scanmem/RUST_PORT_ANALYSIS.md](../../scanmem/RUST_PORT_ANALYSIS.md).
Generic workspace/testing/quality-gate rules: [ARCHITECTURE.md](./ARCHITECTURE.md) — follow that document for
every change; this plan covers **engine module split, API shape, unsafe boundaries, and phased delivery**
only. Repo-wide renaming/licensing/CI jobs that must land first: [PLAN.md](./PLAN.md). Sibling plans:
[scanmem-plan.md](./scanmem-plan.md) (CLI/TUI), [gameconqueror-plan.md](./gameconqueror-plan.md) (GUI).

**Priority order for every design call in this plan (per project direction): code quality > performance >
safety > 1:1 behavioral compatibility with the C `scanmem`.** This is a betterfication, not a transliteration.

---

## 0. Decisions locked for this plan

| Question | Decision | Rationale |
| -------- | -------- | --------- |
| Match storage (`targetmem.c` replacement) | **Safe `Vec<Swath>` redesign**, not a faithful unsafe packed port | Simpler, fully safe, no Miri/fuzz-to-trust unsafe core; some extra bytes/swath is an acceptable cost per the stated priority order |
| ptrace / `process_vm_readv`/`writev` | **Raw `libc` calls confined to one small unsafe module** (`process::ptrace`); `rustix` for everything else (fs/proc parsing, `waitpid`, `kill`, `mm`) | Neither `rustix` nor `nix` expose a real safe wrapper for `ptrace(2)` or `process_vm_readv`/`writev` — `rustix` has neither at all, and `nix`'s versions are the same unsafe raw call with no real safety gain, plus `nix` is out of scope per the "prefer rustix" direction |
| Platform scope | **Linux-only** | No FreeBSD `cfg` branching, no `PT_ATTACH`-style legacy API, no `process_vm_readv` fallback path to maintain |
| Command/text protocol | **Not part of `libscanmem`** | Confirmed by [RUST_PORT_ANALYSIS.md](../../scanmem/RUST_PORT_ANALYSIS.md): parse/act/format concerns split — parsing and formatting move to the `scanmem` CLI crate; `libscanmem` exposes typed functions/structs only |

**Licensing**: tracked as a repo-wide job in [PLAN.md §1](./PLAN.md#1-licensing-decision--fix), not decided
per-crate here — resolve it before Phase 7's release tag.

---

## 1. Goals and constraints

### 1.1 Goals

- **Structured API only**: `Session` type (attach, scan, matches, read, write, options) returning
  `Result<_, ScanmemError>` — no text parsing, no stdout formatting inside the library. This is the
  single highest-leverage decision from the analysis: it removes the FFI/text-protocol hack that today
  forces the Python GUI to regex-parse `libscanmem.so` stdout.
- **Safe by default**: unsafe code confined to the smallest possible surface (`process::ptrace` module
  only) — everything else (value model, scan routines, match storage, maps/sets parsing) is safe Rust.
- **Generic scan routines**: collapse the ~700 lines of repetitive C function-pointer dispatch
  (`scanroutines.c`) into generic code over integer/float types via traits (`num-traits`), not a
  mechanical 1:1 port.
- **No** consumer-facing dependency should leak into the public API surface unnecessarily — `bitflags`,
  `thiserror` types are fine to expose; no `clap`, `toml`, or UI crate ever appears in `libscanmem`.

### 1.2 Non-goals / deferred

- **No** text-command REPL/parser/formatter — belongs to the `scanmem` crate.
- **No** CLI argument parsing, no logging subscriber initialization (library emits `tracing` events only,
  per [ARCHITECTURE.md §5](./ARCHITECTURE.md#5-logging)).
- **No** FreeBSD support (see §0). Revisit only if there is concrete demand.
- **No** faithful unsafe packed swath port for v0.1 — may be revisited later as an optional, feature-gated,
  fuzzed/Miri-verified alternative backend *if* profiling on real large-process scans shows the safe
  redesign's overhead actually matters (mirrors the analysis's own recommendation).

### 1.3 Definitions

- **Region**: one entry from `/proc/<pid>/maps` — address range, permissions, path.
- **Swath**: a contiguous run of matched addresses plus their old values/match flags — the safe
  replacement for `matches_and_old_values_swath`.
- **Session**: one attached target process plus its current match set and options — the structured
  replacement for `globals_t` + `sm_backend_exec_cmd`.

---

## 2. Crate layout

Generic module conventions (`mod.rs` + `tests.rs`, feature-flag rules, crate boundaries):
[ARCHITECTURE.md §1, §9](./ARCHITECTURE.md#1-principles).

```text
libscanmem/
  src/
    lib.rs
    error/            # ScanmemError (thiserror), Result alias
    value/             # Value enum-with-payload, MatchFlags (bitflags), parsing (int/float/bytearray/string)
    sets/               # index-set parsing (comma/range lists), e.g. "1,3-5,8"
    maps/               # /proc/<pid>/maps parsing -> Vec<Region>
    scanroutines/       # generic comparator engine (traits + num-traits) over ScanDataType x MatchType
    swath/              # Vec<Swath>-based match storage (safe targetmem.c replacement)
    process/
      mod.rs
      tests.rs
      ptrace.rs         # ONLY unsafe module: raw libc ptrace/process_vm_readv/process_vm_writev calls
    interrupt/          # SIGINT-driven stop flag (signal-hook), used to abort long scans
    session/            # Session: attach/scan/matches/read/write/options — the public facade
  tests/
    fixtures/
    integration.rs
  src/bin/
    fake_target.rs      # tiny known-memory-layout helper process for integration tests (memfake.c replacement)
```

**Crate boundaries**: `libscanmem` depends on nothing UI/CLI-shaped. Every module above (except
`process::ptrace`) must build and pass tests under `--no-default-features` per the CI feature matrix.

### 2.1 Cargo features

| Feature | Default | Gates |
| ------- | ------- | ----- |
| `signals` | on | `interrupt/` (SIGINT stop-flag via `signal-hook`) — embedders with their own event loop (e.g. a future GUI) can disable this and drive `Session::request_stop()` themselves |
| `serde` | off | `Value`/`MatchFlags`/region types gain `Serialize`/`Deserialize` — useful for a future GUI/IPC boundary, not needed for the CLI (in-process calls) |

Default feature set stays the smallest useful subset (`signals` only) per
[ARCHITECTURE.md §1.4](./ARCHITECTURE.md#14-cargo-features-for-slim-builds).

---

## 3. Module catalog (C source → Rust module)

| C source | Rust module | Notes |
| -------- | ----------- | ----- |
| `value.c`/`value.h` | `value/` | `Value` enum-with-payload (`I8`/`U8`/…/`F64`/`Bytes`/`Str`) replaces the tagged union; `MatchFlags` via `bitflags!` replaces the packed `match_flags` enum — same bit layout, safe API |
| `sets.c`/`sets.h` | `sets/` | Comma/range integer set parser (match-index lists) — plain string parsing, ports mechanically |
| `common.c`/`common.h` | `error/` + small helpers where needed | Privilege-check helpers fold into `Session::attach` error paths |
| `show_message.c` | *(removed)* | Replaced by `tracing` events (`error!`/`warn!`/`debug!`) — no bespoke print shim, per [ARCHITECTURE.md §5](./ARCHITECTURE.md#5-logging) |
| `maps.c`/`maps.h` | `maps/` | `/proc/<pid>/maps` parsing into `Vec<Region>`; plain `String` parsing, no unsafe |
| `scanroutines.c`/`.h` | `scanroutines/` | Generic comparator over `ScanDataType`/`MatchType` via `num-traits`-bounded generics — collapses the C dispatch table instead of porting it 1:1 |
| `targetmem.c`/`.h` | `swath/` | **Safe redesign**: `Vec<Swath>` where `Swath { first_byte_in_child: usize, entries: Vec<SwathEntry> }`, `SwathEntry { old_value: u8, flags: MatchFlags }` — no flexible array members, no manual `realloc` |
| `ptrace.c` | `process/` (`ptrace.rs` unsafe core, rest safe) | Attach/detach, peek/poke, `process_vm_readv`/`writev`, region scan level. Peek-data caching (the old static buffer trick) becomes an explicit, safely-owned cache struct, not a `static mut` |
| `interrupt.c` | `interrupt/` | `signal-hook::flag` registering an `Arc<AtomicBool>`, polled between scan chunks — no raw `signal()`/`sigaction` calls |
| `commands.c`/`handlers.c` | `session/` (structured part only) | Attach/scan/list/dump/write/delete/reset/snapshot/options become typed `Session` methods; **no text parsing or stdout formatting lives here** — that moves entirely to `scanmem` |
| `list.h` (linked list) | *(removed)* | `Vec`/`HashMap` replace it everywhere; no direct port needed |

---

## 4. Public API sketch

```rust
pub struct Session { /* pid, regions, swaths, options — no globals_t-style singleton */ }

impl Session {
    pub fn attach(pid: Pid) -> Result<Self>;
    pub fn detach(&mut self) -> Result<()>;

    pub fn scan(&mut self, expr: &ScanExpr) -> Result<ScanStats>;   // first scan or narrowing scan
    pub fn snapshot(&mut self) -> Result<ScanStats>;                // MATCHANY equivalent
    pub fn matches(&self) -> impl Iterator<Item = MatchView> + '_;
    pub fn nth_match(&self, n: usize) -> Option<MatchView>;
    pub fn delete_in_range(&mut self, range: AddrRange) -> usize;

    pub fn read(&mut self, addr: usize, len: usize) -> Result<Vec<u8>>;
    pub fn write(&mut self, addr: usize, value: &Value) -> Result<()>;

    pub fn set_option(&mut self, option: SessionOption) ;
    pub fn request_stop(&self);                                     // cooperative long-scan abort
}

pub struct ScanExpr { pub data_type: ScanDataType, pub match_type: MatchType, pub value: Option<UserValue> }
```

`ScanmemError` (via `thiserror`) covers: `NotAttached`, `PermissionDenied`, `ProcessExited`, `Io(#[from] std::io::Error)`,
`InvalidExpr(String)`, `Ptrace(i32)` (raw errno passthrough for the unsafe core).

---

## 5. Testing strategy

Generic rules: [ARCHITECTURE.md §6](./ARCHITECTURE.md#6-testing-and-coverage).

- **Unit tests** co-located per module (`tests.rs`): `value` parsing round-trips, `sets` range parsing,
  `scanroutines` comparator correctness per type/match-type combination, `swath` insertion/iteration/
  `delete_in_range` invariants, `maps` parsing against fixture `/proc/*/maps` text blobs.
- **`process::ptrace` unsafe core**: smallest possible surface, exercised only through integration tests
  (§5.1) — never unit-tested in isolation against a fake buffer, since the whole point is real
  attach/peek/poke behavior.
- **Integration tests** (`tests/integration.rs`): spawn `src/bin/fake_target` (replaces `test/memfake.c`),
  attach, scan for a known value, narrow, write, verify — mirrors the intent of
  [test/sm_test.sh](../../scanmem/test/sm_test.sh) but as native `cargo test`, no shell harness.
- **Fuzzing (post-v0.1, optional)**: `cargo-fuzz` targets for `sets::parse` and `value` user-input parsing
  (the only untrusted-input parsers in the crate) — not for `swath`, since its design is now safe by
  construction.
- Tests requiring a real target process are **not** `#[ignore]`d by default (no `nix`/`ptrace` in CI
  sandboxes is a real risk) — mark ptrace-integration tests `#[ignore]` with a documented manual/CI-with-
  `CAP_SYS_PTRACE` checklist, matching [ARCHITECTURE.md §9](./ARCHITECTURE.md#9-crate-boundary-guidelines)'s
  "must not require a running binary unless `#[ignore]`d" rule.

---

## 6. Dependencies

Generic policy (latest stable, active-only, `cargo deny check`): [ARCHITECTURE.md §7](./ARCHITECTURE.md#7-dependencies).

| Area | Crate | Notes |
| ---- | ----- | ----- |
| Process/fs syscalls | `rustix` (`fs`, `process` features) | `/proc` reads, `waitpid`, `kill`, `mm` — everything except ptrace/`process_vm_*` |
| ptrace / cross-process memory | `libc` | Raw `ptrace()`, `process_vm_readv`/`writev` — confined to `process::ptrace`; official rust-lang crate, actively maintained, acceptable for a concentrated unsafe core |
| Match flags | `bitflags` | Packed 16-bit flag set, same semantics as C `match_flags` |
| Generic scan routines | `num-traits` | Bounds for the generic comparator engine |
| Errors | `thiserror` | `ScanmemError` |
| Logging | `tracing` | Events only, no subscriber (library rule, [ARCHITECTURE.md §5](./ARCHITECTURE.md#5-logging)) |
| SIGINT stop flag | `signal-hook` | `signal-hook::flag::register` on an `Arc<AtomicBool>` — feature `signals` |
| Optional (de)serialization | `serde` | Feature `serde`, off by default |

**Explicitly rejected**: `nix` (superseded by `rustix` + a minimal `libc` core per §0); any GTK/iced/clap/toml
crate (UI/CLI concern, belongs to sibling crates).

---

## 7. Phased steps

### Phase 0 — Workspace bootstrap

Repo-wide renaming, licensing, and CI fixes are tracked once in [PLAN.md](./PLAN.md) — complete that
first. `libscanmem`-specific bootstrap:

- [ ] Add `rustix`, `libc`, `bitflags`, `num-traits`, `thiserror`, `tracing`, `signal-hook` to
  `[workspace.dependencies]` (added here, as the first crate that needs them).
- [ ] `error/`, `value/`, `session/` as empty modules with `mod.rs` + `tests.rs` stubs.

**Verify**: `cargo build --workspace`, `cargo fmt --all -- --check`, `cargo deny check` green.

### Phase 1 — Value model + sets

- [ ] `value/`: `Value`, `UserValue`, `MatchFlags` (`bitflags`), parse int/float/bytearray/string, `Value` ↔
  display string conversions.
- [ ] `sets/`: comma/range index-set parser.
- [ ] Unit tests: parse round-trips, edge cases (overflow, empty, malformed ranges).

**Verify**: `cargo test -p libscanmem --no-default-features`, `--all-features` both green.

### Phase 2 — Maps

- [ ] `maps/`: parse `/proc/<pid>/maps` lines into `Region { start, end, perms, path }`.
- [ ] Region filtering helpers (writable-only, heap/stack/anonymous classification) replacing
  `region_scan_level_t` semantics.
- [ ] Unit tests against fixture text blobs in `tests/fixtures/maps/*.txt`.

**Verify**: no live process needed; pure parsing tests.

### Phase 3 — Scan routines

- [ ] `scanroutines/`: generic comparator trait(s) bounded by `num-traits`, covering `ANYNUMBER`/`INTEGER*`/
  `FLOAT*`/`BYTEARRAY`/`STRING` × `MATCHEQUALTO`/`..RANGE`/`..UPDATE`/`..CHANGED`/`..INCREASED(BY)`/etc.
- [ ] Endianness handling as an explicit parameter, not a global flag.
- [ ] Exhaustive unit tests per (data type × match type) combination against known byte patterns.

**Verify**: table-driven tests cover every `ScanDataType`/`MatchType` pair actually reachable from `Session`.

### Phase 4 — Swath storage (safe redesign)

- [ ] `swath/`: `Swath`, `SwathEntry`, `Vec<Swath>`-backed store; `add`, `iter`, `nth_match`,
  `delete_in_range`, `to_printable_string`/`to_bytearray_text` equivalents.
- [ ] Property tests (or thorough unit tests) for insertion order, swath-splitting/merging boundaries,
  `delete_in_range` invariants (matches count stays consistent).

**Verify**: no `unsafe` in this module; `cargo clippy` clean at all three feature levels.

### Phase 5 — Process/ptrace core

- [ ] `process::ptrace` (isolated unsafe): attach/detach, `PTRACE_PEEKDATA`/`POKEDATA` fallback,
  `process_vm_readv`/`writev` fast path, explicit (non-`static`) peek-data cache struct.
- [ ] `process/` safe wrapper: region-level read/write orchestration, calls into `maps/` for target layout.
- [ ] `interrupt/`: `signal-hook`-based stop flag (feature `signals`).
- [ ] `src/bin/fake_target.rs`: known-memory-layout helper process for integration tests.
- [ ] Integration tests (§5) against `fake_target`, `#[ignore]`d where `CAP_SYS_PTRACE` may be unavailable
  in CI, with a documented manual run command.

**Verify**: manual — attach to `fake_target`, read/write a known offset, confirm via the helper's own
stdout assertion.

### Phase 6 — `Session` facade

- [ ] `session/`: wire `attach`/`scan`/`snapshot`/`matches`/`nth_match`/`delete_in_range`/`read`/`write`/
  `set_option`/`request_stop` on top of Phases 1–5.
- [ ] `ScanmemError` finalized; no `panic!`/`unwrap()` on attacker- or target-controlled input paths.
- [ ] End-to-end integration test: attach → scan → narrow → write → verify, all through `Session` only.

**Verify**: this is the crate's public contract — `cargo doc --workspace --no-deps` renders cleanly;
`scanmem`/`gameconqueror` plans can start consuming `Session` after this phase.

### Phase 7 — Polish + first release

- [ ] `cargo-fuzz` targets for `sets`/`value` user-input parsers (optional, documented, not CI-blocking).
- [ ] README: crate purpose, `Session` quick example, explicit "Linux-only, requires `CAP_SYS_PTRACE`/root"
  note.
- [ ] CHANGELOG; confirm license per [PLAN.md §1](./PLAN.md#1-licensing-decision--fix); tag **v0.1.0**.

**Verify**: all [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit) gates
pass at all three feature levels.

---

## 8. Definition of done (v0.1.0)

- [ ] `Session` is the only public entry point; no text-command protocol anywhere in the crate.
- [ ] `unsafe` exists only in `process::ptrace`.
- [ ] `swath/` is fully safe, no `realloc`/flexible-array tricks.
- [ ] Linux-only; no FreeBSD `cfg` branches.
- [ ] `nix` does not appear in the dependency tree (`cargo tree | grep nix` clean).
- [ ] `scanmem`/`gameconqueror` can depend on `libscanmem` as an ordinary typed Rust dependency — no FFI,
  no stdout capture, no regex parsing of engine output.
- [ ] CI green per [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit)
  across `--no-default-features`, default, `--all-features`.
