# scanmem — implementation plan

This document is the **human roadmap** and **agent playbook** for **`scanmem`**: the interactive
terminal front-end for the [`libscanmem`](./libscanmem-plan.md) engine — a clean-slate command design
first shipped as a classic REPL, with a **ratatui TUI** as the "perfect world" follow-up.

Background analysis: [scanmem/RUST_PORT_ANALYSIS.md](../../scanmem/RUST_PORT_ANALYSIS.md).
Generic workspace/settings/testing/quality-gate rules: [ARCHITECTURE.md](./ARCHITECTURE.md). Repo-wide
renaming/licensing/CI jobs that must land first: [PLAN.md](./PLAN.md). Sibling plans:
[libscanmem-plan.md](./libscanmem-plan.md) (engine, must land first), [gameconqueror-plan.md](./gameconqueror-plan.md) (GUI).

**Priority order for every design call in this plan: code quality > performance > safety > 1:1 behavioral
compatibility with the C `scanmem` CLI.**

---

## 0. Decisions locked for this plan

| Question | Decision | Rationale |
| -------- | -------- | --------- |
| Text-command scripting compat (`scanmem -c "pid 1234;list"`) | **Clean-slate design, no compat requirement** | Old grammar was fused parse/act/format C code; a fresh command set designed against `libscanmem::Session` directly is higher quality and avoids dragging legacy quirks forward |
| Front-end delivery order | **REPL first (v0.1), ratatui TUI later (v0.2 "perfect world")** | De-risks `Session` integration and terminal I/O early; reuses the spirit of [test/sm_test.sh](../../scanmem/test/sm_test.sh) as native integration tests before adding TUI complexity |

Familiar verb names (`pid`, `scan`/`list`, `dump`, `write`, `option`, `delete`, `reset`, `snapshot`) are kept
for user muscle-memory where they still make sense, but **grammar, output format, and one-shot scripting
syntax are redesigned** — not byte-compatible with the old protocol.

---

## 1. Goals and constraints

### 1.1 Goals

- **Depends on `libscanmem::Session` only** — no ptrace, no memory-scanning logic in this crate; this
  crate owns terminal I/O, command parsing/dispatch, and (later) TUI rendering only.
- **Two front-ends behind one binary**: a scripting/plain-output mode (default-buildable with
  `--no-default-features`) and an interactive ratatui mode (`tui` feature, default-on) — auto-selected via
  `is_terminal`, overridable with `--no-tui`.
- **`Settings` resolution** follows [ARCHITECTURE.md §3](./ARCHITECTURE.md#3-settings-resolution-unified-resolver):
  CLI (`clap`) > optional TOML config > defaults.
- **Slim `main`**: parse CLI, resolve `Settings`, init `tracing` subscriber, hand off to `app/`.

### 1.2 Non-goals / deferred

- **No** byte-for-byte replication of old `scanmem` stdout format or `-c` scripting grammar.
- **No** memory-scanning logic duplicated here — always through `Session`.
- **No** GTK/iced — that is [gameconqueror-plan.md](./gameconqueror-plan.md)'s concern.
- **Deferred to v0.2 (TUI phase)**: live-updating match table, in-TUI hex preview, mouse support.

### 1.3 Definitions

- **Command**: one REPL/scripted user action, parsed into a typed `Command` enum, dispatched to `Session`,
  then formatted for the active front-end (plain text or TUI widget) — the parse/act/format split the
  analysis recommends, with **act** entirely in `libscanmem` and **parse**/**format** here.
- **Session state (CLI-local)**: current `Session`, output verbosity, alignment/endianness options —
  mirrors `globals_t.options` but owned by `app/`, not global.

---

## 2. Crate layout

Generic module conventions: [ARCHITECTURE.md §2, §4](./ARCHITECTURE.md#2-repository-layout).

```text
scanmem/
  src/
    main.rs             # slim entry point
    cli/                # clap: --pid, --exec <script>, --no-tui, --config, -v/--verbose
      mod.rs
      tests.rs
    config/              # optional TOML: default alignment, color, history file path
      mod.rs
      tests.rs
    settings/            # merge cli + config + defaults -> Settings
      mod.rs
      tests.rs
    logger/               # tracing subscriber init from Settings
      mod.rs
      tests.rs
    commands/             # Command enum, parser (clean-slate grammar), formatter (plain-text)
      mod.rs
      parser.rs
      formatter.rs
      tests.rs
    app/                  # REPL loop / one-shot script runner, owns Session + CLI-local state
      mod.rs
      repl.rs
      script.rs
      tests.rs
    ui/                   # ratatui widgets (Phase B, feature "tui"), one file per element
      mod.rs
      process_picker.rs
      match_table.rs
      scan_bar.rs
      status_bar.rs
      tests.rs
    utils/
      mod.rs
      tests.rs
  tests/
    integration.rs        # assert_cmd-driven end-to-end tests against libscanmem's fake_target
```

### 2.1 Cargo features

| Feature | Default | Gates |
| ------- | ------- | ----- |
| `tui` | on | `ratatui` + `crossterm`, `ui/` module, interactive event loop |
| `serde` | off | Passthrough to `libscanmem/serde` if config/export commands need it |

`--no-default-features` yields a **scripting-only** binary (no `ratatui`/`crossterm` in the dependency
tree) — this maps naturally onto [ARCHITECTURE.md §1.4](./ARCHITECTURE.md#14-cargo-features-for-slim-builds)'s
three-level CI matrix and gives users a genuinely slimmer build for headless/CI use.

---

## 3. Command set (clean-slate grammar)

| Verb | Maps to `Session` method | Notes |
| ---- | ------------------------ | ----- |
| `pid <n>` / `attach <n>` | `Session::attach` | |
| `scan <type> <match> [value]` | `Session::scan` | e.g. `scan i32 = 100`, `scan i32 range 10 20`, `scan i32 increased` |
| `snapshot` | `Session::snapshot` | first/reset scan over all regions |
| `list [range]` | `Session::matches` / `Session::nth_match` | prints match table |
| `dump <addr> <len>` | `Session::read` | hex + ascii, reuses the future TUI's hex formatting code |
| `write <addr> <type> <value>` | `Session::write` | |
| `delete <range>` | `Session::delete_in_range` | uses `libscanmem::sets` range grammar for selection |
| `option <key> <value>` | `Session::set_option` | alignment, endianness, region scan level |
| `reset` | new `Session` | |
| `help`, `quit`/`exit` | local to `app/` | |

One-shot scripting: `scanmem --exec 'attach 1234; scan i32 = 100; scan i32 increased; list'` — new syntax
(`;`-separated commands via `--exec`), explicitly **not** claiming compatibility with old `-c`.

---

## 4. Front-end behavior

### 4.1 REPL (v0.1)

- `rustyline` for line editing, history (`~/.local/state/scanmem/history` or `$XDG_STATE_HOME`), and
  command completion (verb names, then context-sensitive: match indices after `list`, etc.).
- Plain-text formatter in `commands/formatter.rs` — human-readable tables, no ANSI color when stdout is
  not a tty (respects `NO_COLOR`).
- `Session::request_stop()` wired to `Ctrl-C` during a long `scan`, not process termination (mirrors
  `interrupt.c`'s intent through the new safe API).

### 4.2 TUI (v0.2, feature `tui`)

- `ratatui` + `crossterm`; `ui/` widgets: process picker, live match table (virtualized for large match
  counts), scan input bar, status/help bar.
- `app/` owns an event loop bridging terminal events → `Command` → `Session` → re-render; no `Session`
  calls happen inside `ui/` widget code (mirrors [ARCHITECTURE.md §4.2](./ARCHITECTURE.md#42-supporting-modules):
  `app/` coordinates, `ui/` renders).
- Auto-select TUI when stdout `is_terminal()` and `tui` feature is enabled and `--no-tui`/`--exec` was not
  passed; otherwise fall back to REPL/script mode.

---

## 5. Testing strategy

Generic rules: [ARCHITECTURE.md §6](./ARCHITECTURE.md#6-testing-and-coverage).

- **Unit tests**: `commands::parser` grammar (valid/invalid commands, whitespace/edge cases),
  `commands::formatter` output shape, `settings` precedence merging.
- **Integration tests** (`tests/integration.rs`): `assert_cmd` drives the built `scanmem` binary
  against `libscanmem`'s `fake_target` helper binary (via `env!("CARGO_BIN_EXE_fake_target")` from the
  `libscanmem` dev-dependency), using `--exec` scripts — replaces
  [test/sm_test.sh](../../scanmem/test/sm_test.sh)'s intent as native `cargo test`.
- **TUI**: `ratatui`'s `TestBackend` for widget-level snapshot tests (`ui/*::tests`) — no real terminal
  needed in CI.

---

## 6. Dependencies

Generic policy: [ARCHITECTURE.md §7](./ARCHITECTURE.md#7-dependencies).

| Area | Crate | Notes |
| ---- | ----- | ----- |
| Engine | `libscanmem` | workspace path dependency |
| CLI | `clap` (derive) | `--pid`, `--exec`, `--no-tui`, `--config`, `-v` |
| REPL editing | `rustyline` | history + completion, portable pure-Rust line editor |
| TUI | `ratatui`, `crossterm` | feature `tui`, default on |
| Config | `serde`, `toml` | optional `config/` TOML |
| Logging | `tracing`, `tracing-subscriber` | binary-only, per [ARCHITECTURE.md §5](./ARCHITECTURE.md#5-logging) |
| Errors | `thiserror` | CLI-local error type wrapping `ScanmemError` |
| Testing | `assert_cmd`, `predicates` | integration tests |

---

## 7. Phased steps

### Phase 0 — Crate bootstrap

Repo-wide renaming/licensing/CI fixes are tracked once in [PLAN.md](./PLAN.md) — complete that first.
`scanmem`-specific bootstrap:

- [ ] `cli/`, `config/`, `settings/`, `logger/`, `app/` skeletons; depend on `libscanmem`.
- [ ] Vertical slice: `scanmem --pid <n>` attaches and prints region count, nothing else.

**Verify**: `cargo run -p scanmem -- --pid $$` prints something sane against the current shell's own pid.

### Phase 1 — Command core (REPL, no TUI yet)

- [ ] `commands/`: `Command` enum, parser, plain-text formatter for the verb table in §3.
- [ ] `app/repl.rs`: `rustyline`-backed loop, dispatch to `Session`, print via formatter.
- [ ] Unit tests for parser/formatter.

**Verify**: manual REPL session against `fake_target` — attach, scan, narrow, list, write, verify.

### Phase 2 — One-shot scripting + integration tests

- [ ] `app/script.rs`: `--exec '<commands>;...'` runner, same `Command`/`Session` path as the REPL.
- [ ] `tests/integration.rs`: `assert_cmd` end-to-end tests against `fake_target`.

**Verify**: `cargo test -p scanmem` green; covers attach/scan/narrow/write/verify without a live terminal.

### Phase 3 — REPL polish (v0.1 release)

- [ ] History file, completion (verb names, context-sensitive match indices), colored output honoring
  `NO_COLOR`/non-tty stdout.
- [ ] `Ctrl-C` → `Session::request_stop()` during long scans.
- [ ] README, CHANGELOG; tag **v0.1.0** (REPL-only front-end, `tui` feature exists but is not yet the
  primary documented experience).

**Verify**: [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit) gates at
all three feature levels; dogfood against `fake_target` and a real long-running process.

### Phase 4 — TUI foundation (feature `tui`)

- [ ] `ui/` skeleton: main layout, status bar, `ratatui::TestBackend`-based widget tests.
- [ ] `app/` event loop: terminal events → `Command` → `Session` → re-render, auto-select vs. `--no-tui`.
- [ ] Process picker widget (`ui/process_picker.rs`) — list `/proc` PIDs with a name filter.

**Verify**: manual — launch `scanmem` in an interactive terminal, confirm auto-TUI + `--no-tui` fallback.

### Phase 5 — TUI match workflow

- [ ] Scan input bar (`ui/scan_bar.rs`): type/match-type/value entry, submits a `Command::Scan`.
- [ ] Match table (`ui/match_table.rs`): virtualized list for large match counts, selection → `dump`/`write`.
- [ ] Snapshot widget-level tests via `TestBackend`.

**Verify**: manual dogfood — full first-scan → narrow → write loop entirely inside the TUI.

### Phase 6 — Polish + v0.2 release

- [ ] Help overlay, keybinding cheat-sheet, mouse support (optional).
- [ ] Update README to document TUI as the primary interactive experience; REPL/`--exec` documented as
  scripting/headless mode.
- [ ] CHANGELOG; tag **v0.2.0**.

**Verify**: full [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit) gate
set, including `--no-default-features` (scripting-only) build staying `ratatui`/`crossterm`-free.

---

## 8. Definition of done

### v0.1.0 (REPL)

- [ ] `scanmem` depends on `libscanmem::Session` only — no engine logic duplicated.
- [ ] Clean-slate command grammar documented in README; explicitly not `-c`-compatible.
- [ ] `assert_cmd` integration tests cover attach/scan/narrow/write/verify against `fake_target`.
- [ ] `--no-default-features` build has no `ratatui`/`crossterm` in its dependency tree.

### v0.2.0 (TUI)

- [ ] Full first-scan → narrow → write → verify workflow available without leaving the TUI.
- [ ] REPL/`--exec` remain fully functional (`--no-tui` / non-tty auto-fallback).
- [ ] CI green per [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit)
  across `--no-default-features`, default, `--all-features`.
