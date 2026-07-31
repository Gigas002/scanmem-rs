# scanmem — implementation plan

This document is the **human roadmap** and **agent playbook** for **`scanmem`**: the interactive
terminal front-end for the [`libscanmem`](./libscanmem-plan.md) engine. It is a **CLI/REPL only** —
working the way the original C `scanmem` works (attach, scan, narrow, list, dump, write, one-shot
scripting) — with **no TUI**.

Background analysis: [scanmem/RUST_PORT_ANALYSIS.md](../../scanmem/RUST_PORT_ANALYSIS.md).
Generic workspace/settings/testing/quality-gate rules: [ARCHITECTURE.md](./ARCHITECTURE.md). Repo-wide
Sibling plans:
[libscanmem-plan.md](./libscanmem-plan.md) (engine, must land first), [gameconqueror-plan.md](./gameconqueror-plan.md) (GUI).

**Priority order for every design call in this plan: code quality > performance > safety > 1:1 behavioral
compatibility with the C `scanmem` CLI.**

---

## 0. Decisions locked for this plan

| Question | Decision | Rationale |
| -------- | -------- | --------- |
| Front-end shape | **CLI/REPL only, no TUI** | Matches how the original C `scanmem` is actually used day to day; a full-screen TUI is unnecessary complexity for this crate |
| Text-command scripting compat (`scanmem -c "pid 1234;list"`) | **Clean-slate design, no compat requirement** | Old grammar was fused parse/act/format C code; a fresh command set designed against `libscanmem::Session` directly is higher quality and avoids dragging legacy quirks forward |

Familiar verb names (`pid`, `scan`/`list`, `dump`, `write`, `option`, `delete`, `reset`, `snapshot`) are kept
for user muscle-memory where they still make sense, but **grammar, output format, and one-shot scripting
syntax are redesigned** — not byte-compatible with the old protocol.

---

## 1. Goals and constraints

### 1.1 Goals

- **Depends on `libscanmem::Session` only** — no ptrace, no memory-scanning logic in this crate; this
  crate owns terminal I/O and command parsing/dispatch only.
- **One front-end**: an interactive REPL plus one-shot scripting (`--exec`), matching the interactive
  shell experience of the original C `scanmem` — no TUI, no `ratatui`/`crossterm` dependency at all.
- **`Settings` resolution** follows [ARCHITECTURE.md §3](./ARCHITECTURE.md#3-settings-resolution-unified-resolver):
  CLI (`clap`) > optional TOML config > defaults.
- **Slim `main`**: parse CLI, resolve `Settings`, init `tracing` subscriber, hand off to `app/`.

### 1.2 Non-goals / deferred

- **No** byte-for-byte replication of old `scanmem` stdout format or `-c` scripting grammar.
- **No** TUI, ever, for this crate — full-screen/widget-based interaction is
  [gameconqueror-plan.md](./gameconqueror-plan.md)'s concern (or a future crate), not this one.
- **No** memory-scanning logic duplicated here — always through `Session`.
- **No** GTK/iced — that is [gameconqueror-plan.md](./gameconqueror-plan.md)'s concern.

### 1.3 Definitions

- **Command**: one REPL/scripted user action, parsed into a typed `Command` enum, dispatched to `Session`,
  then formatted as plain text — the parse/act/format split the analysis recommends, with **act** entirely
  in `libscanmem` and **parse**/**format** here.
- **Session state (CLI-local)**: current `Session`, output verbosity, alignment/endianness options —
  mirrors `globals_t.options` but owned by `app/`, not global.

---

## 2. Crate layout

Generic module conventions: [ARCHITECTURE.md §2, §4](./ARCHITECTURE.md#2-repository-layout).

```text
scanmem/
  src/
    main.rs             # slim entry point
    cli/                # clap: --pid, --exec <script>, --config, -v/--verbose
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
    utils/
      mod.rs
      tests.rs
  tests/
    integration.rs        # assert_cmd-driven end-to-end tests against libscanmem's fake_target
```

### 2.1 Cargo features

| Feature | Default | Gates |
| ------- | ------- | ----- |
| `serde` | off | Passthrough to `libscanmem/serde` if config/export commands need it |

No UI-toolkit feature exists in this crate — it is CLI/REPL only, so there is no `--no-default-features`
vs. `--all-features` distinction driven by a UI dependency; the three-level CI matrix from
[ARCHITECTURE.md §1.4](./ARCHITECTURE.md#14-cargo-features-for-slim-builds) still applies for the `serde`
feature.

---

## 3. Command set (clean-slate grammar)

| Verb | Maps to `Session` method | Notes |
| ---- | ------------------------ | ----- |
| `pid <n>` / `attach <n>` | `Session::attach` | |
| `scan <type> <match> [value]` | `Session::scan` | e.g. `scan i32 = 100`, `scan i32 range 10 20`, `scan i32 increased` |
| `snapshot` | `Session::snapshot` | first/reset scan over all regions |
| `list [range]` | `Session::matches` / `Session::nth_match` | prints match table |
| `dump <addr> <len>` | `Session::read` | hex + ascii |
| `write <addr> <type> <value>` | `Session::write` | |
| `delete <range>` | `Session::delete_in_range` | uses `libscanmem::sets` range grammar for selection |
| `option <key> <value>` | `Session::set_option` | alignment, endianness, region scan level |
| `reset` | new `Session` | |
| `help`, `quit`/`exit` | local to `app/` | |

One-shot scripting: `scanmem --exec 'attach 1234; scan i32 = 100; scan i32 increased; list'` — new syntax
(`;`-separated commands via `--exec`), explicitly **not** claiming compatibility with old `-c`.

---

## 4. Front-end behavior

- `rustyline` for line editing, history (`~/.local/state/scanmem/history` or `$XDG_STATE_HOME`), and
  command completion (verb names, then context-sensitive: match indices after `list`, etc.).
- Plain-text formatter in `commands/formatter.rs` — human-readable tables, no ANSI color when stdout is
  not a tty (respects `NO_COLOR`).
- `Session::request_stop()` wired to `Ctrl-C` during a long `scan`, not process termination (mirrors
  `interrupt.c`'s intent through the new safe API).

---

## 5. Testing strategy

Generic rules: [ARCHITECTURE.md §6](./ARCHITECTURE.md#6-testing-and-coverage).

- **Unit tests**: `commands::parser` grammar (valid/invalid commands, whitespace/edge cases),
  `commands::formatter` output shape, `settings` precedence merging.
- **Integration tests** (`tests/integration.rs`): `assert_cmd` drives the built `scanmem` binary
  against `libscanmem`'s `fake_target` helper binary (via `env!("CARGO_BIN_EXE_fake_target")` from the
  `libscanmem` dev-dependency), using `--exec` scripts — replaces
  [test/sm_test.sh](../../scanmem/test/sm_test.sh)'s intent as native `cargo test`.

---

## 6. Dependencies

Generic policy: [ARCHITECTURE.md §7](./ARCHITECTURE.md#7-dependencies).

| Area | Crate | Notes |
| ---- | ----- | ----- |
| Engine | `libscanmem` | workspace path dependency |
| CLI | `clap` (derive) | `--pid`, `--exec`, `--config`, `-v` |
| REPL editing | `rustyline` | history + completion, portable pure-Rust line editor |
| Config | `serde`, `toml` | optional `config/` TOML |
| Logging | `tracing`, `tracing-subscriber` | binary-only, per [ARCHITECTURE.md §5](./ARCHITECTURE.md#5-logging) |
| Errors | `thiserror` | CLI-local error type wrapping `ScanmemError` |
| Testing | `assert_cmd`, `predicates` | integration tests |

---

## 7. Phased steps

### Phase 0 — Crate bootstrap

Repo-wide renaming/licensing/CI fixes are already done.
`scanmem`-specific bootstrap:

- [ ] `cli/`, `config/`, `settings/`, `logger/`, `app/` skeletons; depend on `libscanmem`.
- [ ] Vertical slice: `scanmem --pid <n>` attaches and prints region count, nothing else.

**Verify**: `cargo run -p scanmem -- --pid $$` prints something sane against the current shell's own pid.

### Phase 1 — Command core (REPL)

- [ ] `commands/`: `Command` enum, parser, plain-text formatter for the verb table in §3.
- [ ] `app/repl.rs`: `rustyline`-backed loop, dispatch to `Session`, print via formatter.
- [ ] Unit tests for parser/formatter.

**Verify**: manual REPL session against `fake_target` — attach, scan, narrow, list, write, verify.

### Phase 2 — One-shot scripting + integration tests

- [ ] `app/script.rs`: `--exec '<commands>;...'` runner, same `Command`/`Session` path as the REPL.
- [ ] `tests/integration.rs`: `assert_cmd` end-to-end tests against `fake_target`.

**Verify**: `cargo test -p scanmem` green; covers attach/scan/narrow/write/verify without a live terminal.

### Phase 3 — REPL polish + release

- [ ] History file, completion (verb names, context-sensitive match indices), colored output honoring
  `NO_COLOR`/non-tty stdout.
- [ ] `Ctrl-C` → `Session::request_stop()` during long scans.
- [ ] README, CHANGELOG; tag **v0.1.0**.

**Verify**: [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit) gates at
all three feature levels; dogfood against `fake_target` and a real long-running process.

---

## 8. Definition of done

- [ ] `scanmem` depends on `libscanmem::Session` only — no engine logic duplicated.
- [ ] Clean-slate command grammar documented in README; explicitly not `-c`-compatible.
- [ ] `assert_cmd` integration tests cover attach/scan/narrow/write/verify against `fake_target`.
- [ ] No TUI, no `ratatui`/`crossterm` anywhere in this crate's dependency tree.
- [ ] CI green per [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit)
  across `--no-default-features`, default, `--all-features`.

