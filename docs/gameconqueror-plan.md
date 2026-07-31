# gameconqueror — implementation plan

This document is the **human roadmap** and **agent playbook** for **`gameconqueror`**: a Rust port of the
GTK GUI (`gui/*.py` + `gui/GameConqueror.ui`) as a **terminal UI (TUI) built on `ratatui`**, keeping the
same conceptual panels (process picker, scan controls, match/cheat grids, hex view) but navigated entirely
through **hotkeys** — no mouse, no windowing toolkit.

Background analysis: [scanmem/RUST_PORT_ANALYSIS.md](../../scanmem/RUST_PORT_ANALYSIS.md). That analysis
was written when this crate was scoped as a GTK4-then-`iced` GUI; its backend-integration conclusion
(`gameconqueror` calls `libscanmem::Session` directly, no FFI/text-protocol hack) still holds, but its
GTK/`iced` widget notes are superseded by §0/§4 below.

Generic workspace/settings/testing/quality-gate rules: [ARCHITECTURE.md](./ARCHITECTURE.md). Sibling plans:
[libscanmem-plan.md](./libscanmem-plan.md) (engine, must land first), [scanmem-plan.md](./scanmem-plan.md)
(REPL CLI).

**Priority order for every design call in this plan: code quality > performance > safety > 1:1 behavioral
compatibility with the Python `GameConqueror`.**

---

## 0. Decisions locked for this plan

| Question | Decision | Rationale |
| -------- | -------- | --------- |
| UI paradigm | **TUI, not GUI** — no GTK, no `iced`, no windowing toolkit of any kind | Removes an entire class of native dependencies (GTK4 system libs, display server) and installs as a single portable binary; a terminal UI can run over SSH, in a container, or headless — a strict superset of environments a GTK window supports |
| TUI framework | **`ratatui`** + **`crossterm`** for the terminal backend | `ratatui` is the actively maintained, de-facto standard immediate-mode Rust TUI framework (successor to `tui-rs`); `crossterm` is cross-platform (Linux/macOS/Windows terminals) and pure-Rust, unlike `termion` |
| Interaction model | **Keyboard-only, hotkey-first**; mouse support is optional/deferred (§1.2) | The whole point of this pivot: every action has a discoverable, memorable key binding, and the UI is fully usable without ever touching a pointing device — matches how power users actually drive scanners/debuggers in a terminal |
| List/grid widget | **`ratatui::widgets::Table`** with manual sort/filter state in `app::state`, not a toolkit-provided sortable grid | `ratatui` is immediate-mode: there is no persistent `ColumnView`/`TreeView` object to hold sort/filter state, so that state lives in `AppState` (already the toolkit-independent source of truth) and `ui/` just renders it each frame |
| Hex editor widget | **Hand-built 3-pane `ratatui` widget** (offset / hex / ascii columns) with cursor state in `AppState` | No terminal-native hex-editor widget exists in `ratatui`; this is a from-scratch immediate-mode port of `hexview.py`'s concept, not its GTK implementation |
| Backend integration | **Direct typed calls into `libscanmem::Session`** | No FFI, no stdout-capture/regex-parsing hack — the single biggest win identified by the background analysis, and unaffected by the GUI→TUI pivot |

---

## 1. Goals and constraints

### 1.1 Goals

- **Toolkit-independent core, unchanged in spirit**: `app/` owns `AppState`, a `Msg` enum describing every
  user action, and an `update(state, msg)` function calling `libscanmem::Session`. Zero `ratatui`/
  `crossterm` types here — fully unit-testable without a terminal attached (the current Python
  `GameConqueror` class cannot do this today, since it mixes state and GTK widget calls throughout).
- **Hotkey-first, discoverable bindings**: every `Msg`-producing action is reachable from a single,
  documented key or key chord (§4). A built-in help overlay (bound to `?`/`F1`) lists every binding active
  in the current focused panel — there is no menu bar or toolbar to fall back on, so discoverability has to
  be built in from the start, not bolted on later.
- **Explicit focus model**: since there is no pointer, exactly one panel has *focus* at any time
  (`AppState::focus`), and `Tab`/`Shift+Tab` cycle it. All other key bindings are interpreted relative to
  the focused panel. This mirrors [ARCHITECTURE.md §4.2](./ARCHITECTURE.md#42-supporting-modules): `app/`
  coordinates, `ui/` renders only.
- **`ratatui` first iteration, and only iteration**: `ui/` widgets are thin — read `AppState`, render a
  `Frame`; `ui/input.rs` translates a `crossterm::event::KeyEvent` to a `Msg` via the active keymap, calls
  `app::update`, and the next frame re-renders from the new `AppState`. Unlike the original GTK-then-`iced`
  plan, there is **no second front-end to keep `app/` compatible with** — one less constraint on the core.
- **Two hard widgets carried over concept-for-concept, not code-for-code**: the 3-pane hex editor
  (`HexView`) and the sortable/filterable/inline-editable scan-result and cheat-list grids, both rebuilt as
  immediate-mode `ratatui` widgets driven entirely by keys.

### 1.2 Non-goals / deferred

- **No GUI toolkit of any kind** — GTK3, GTK4, `iced`, and any other windowing toolkit are permanently out
  of scope for this crate, not just deferred. If a graphical front-end is ever wanted again, it is a new,
  separate plan, not a phase of this one.
- **No mouse support in v0.1.** `crossterm` can report mouse events and `ratatui` widgets can hit-test them,
  so this is not architecturally blocked — it is simply not worth the complexity budget while every action
  already has a keyboard binding. Revisit only if user feedback specifically asks for it, as a strictly
  additive `ui/input.rs` change.
- **No 1:1 port of `misc.py`'s process-picker heuristics, cheat-list file format, or polkit**
  (`org.freedesktop.gameconqueror.policy`) integration — revisit the privilege-escalation UX after v0.1
  (a terminal app still needs elevated ptrace/`/proc/<pid>/mem` access) using whatever the current
  best-practice `sudo`/`pkexec`-wrapper pattern is at that time.
- **No configurable/remappable keymap in v0.1.** The keymap (§4) is a fixed table for the first release;
  making it user-configurable via `config/` TOML is a natural, additive follow-up once the fixed set has
  proven itself, not a v0.1 requirement.

### 1.3 Definitions

- **`AppState`**: current pid, attached `Session`, scan settings, match-result cache (with sort/filter
  state, since `ratatui::widgets::Table` holds none itself), cheat list, current `focus`, hex-view cursor —
  the toolkit-independent replacement for the Python `GameConqueror` class's instance state.
- **`Msg`**: one user-triggered action (`Attach(pid)`, `StartScan(expr)`, `AddCheat(row)`,
  `ToggleFreeze(row)`, `WriteValue(row, value)`, `FocusNext`, `FocusPrev`, `ShowHelp`, …). Focus/navigation
  actions are `Msg` variants too — `app/` remains the single place that mutates `AppState`.
- **`Keymap`**: a fixed table mapping `(Focus, KeyEvent) -> Msg`, owned by `ui/keymap.rs`. `ui/input.rs`
  looks up the incoming key against the table for the current focus (plus any global bindings) and emits
  the resulting `Msg`.
- **`ui/` element**: one distinct visual/interactive piece, one file, per
  [ARCHITECTURE.md §4.1](./ARCHITECTURE.md#41-ui-module) — unchanged from the generic rule, just rendering
  `ratatui` widgets instead of GTK ones.

---

## 2. Crate layout

```text
gameconqueror/
  src/
    main.rs               # slim entry point: init terminal, run event loop, restore terminal on exit
    cli/                  # clap: --pid
      mod.rs
      tests.rs
    settings/
      mod.rs
      tests.rs
    logger/                 # tracing to a file, never stdout/stderr (would corrupt the TUI frame)
      mod.rs
      tests.rs
    app/                    # AppState, Msg, update() — no ratatui/crossterm types, fully unit-testable
      mod.rs
      state.rs
      msg.rs
      focus.rs               # Focus enum + cycling order
      cheatlist.rs            # cheat-list persistence (serde + TOML/JSON, not the legacy format)
      tests.rs
    ui/                      # ratatui — one file per panel/element
      mod.rs
      input.rs                # crossterm KeyEvent -> Msg via keymap.rs
      keymap.rs                # fixed (Focus, KeyEvent) -> Msg table + per-focus help text
      layout.rs                # top-level Frame layout: panel rects, status/help bar
      process_picker.rs        # Table over /proc, incremental search-filter
      scan_panel.rs            # scan type/match-type/value input line
      match_view.rs             # Table over scan results, sortable/filterable via AppState state
      cheat_view.rs             # Table over cheat list, freeze toggle, inline value edit
      hex_view.rs               # 3-pane hex editor: offset/hex/ascii columns, cursor-driven
      help_overlay.rs           # `?`/F1 popup listing active bindings for the focused panel
      tests.rs
    utils/
      mod.rs
      tests.rs
  tests/
    integration.rs            # app::update() end-to-end tests against libscanmem's fake_target, no
                               # terminal/ratatui involved
```

### 2.1 Cargo features

| Feature | Default | Gates |
| ------- | ------- | ----- |
| `tui` | on | `ratatui`, `crossterm`, the entire `ui/` module — the shipped v0.1 experience |

`app/` and `tests/integration.rs` must build and pass under `--no-default-features` (no terminal UI at all)
per [ARCHITECTURE.md §1.4](./ARCHITECTURE.md#14-cargo-features-for-slim-builds) — this is what guarantees
the core is genuinely UI-independent, not just structured to look that way. A single `tui` feature (rather
than the previous `gtk4`/`iced` pair) reflects that there is now only one front-end to gate.

---

## 3. Module responsibilities (Python source → Rust module)

| Python source | Rust module | Notes |
| -------------- | ----------- | ----- |
| `scanmem.py` (ctypes + stdout regex parsing) | *(removed entirely)* | Replaced by direct `libscanmem::Session` calls from `app/` — the core integration win of the whole port, unaffected by the GUI→TUI pivot |
| `GameConqueror.py` (state + widget calls mixed) | `app/state.rs` + `app/msg.rs` + `app/focus.rs` | Split into toolkit-independent `AppState`/`Msg`/`update()`; focus tracking is new — the Python GUI relied on the window manager/pointer for that instead |
| `GameConqueror.ui` (Glade) | `ui/layout.rs` + per-panel files | Hand-built `ratatui` layout, not a ported XML file — GTK's widget tree has no terminal-UI equivalent to reuse |
| `hexview.py` | `ui/hex_view.rs` | Offset/hex/ascii columns rendered as `ratatui` `Paragraph`/`Table` cells; cursor and selection live in `AppState`, not in the widget — most self-contained panel; develop against a fake in-memory payload independent of the rest of the app, same approach `hexview.py`'s own `__main__` demo uses |
| `misc.py` (scan-command validation, cheat-list helpers) | `app/cheatlist.rs` + validation folded into `Msg` handling | Cheat-list persistence via `serde` + TOML/JSON, not the legacy on-disk format |
| `consts.py.in` | build-time constants or `app/` | Version/paths resolved via `settings/` |
| *(new, no Python source)* | `ui/keymap.rs`, `ui/input.rs`, `ui/help_overlay.rs` | The Python GUI got discoverability for free from GTK's menu bar/tooltips; a hotkey-first TUI has to build a keymap and an in-app help overlay explicitly |

---

## 4. Hotkeys and focus model

Generic layout/rendering rules: [ARCHITECTURE.md §4](./ARCHITECTURE.md#4-userspace-applications-cli--tui--gui).

### 4.1 Focus cycling

`Tab` / `Shift+Tab` cycle `AppState::focus` through, in order: **Process Picker → Scan Panel → Match View →
Cheat View → Hex View** (only reachable once a match/cheat row is selected) and back. The status/help bar
(`ui/layout.rs`) always shows the current focus name and a one-line hint of its most-used bindings; the
full list is one `?` away (`ui/help_overlay.rs`).

### 4.2 Global bindings (any focus)

| Key | `Msg` | Notes |
| --- | ----- | ----- |
| `Tab` / `Shift+Tab` | `FocusNext` / `FocusPrev` | Cycle panel focus |
| `?` / `F1` | `ShowHelp` | Toggle the help overlay for the current focus |
| `Ctrl+Q` | `Quit` | Confirm-then-quit if there are unsaved cheat-list changes |
| `Ctrl+S` | `SaveCheatList` | Save using the last-used path, or prompt once |
| `Ctrl+L` | `LoadCheatList` | Prompt for a path |
| `Esc` | `Dismiss` | Close a popup/overlay/prompt without acting |

### 4.3 Per-panel bindings (examples, full table lives in `ui/keymap.rs` + the in-app help overlay)

| Panel | Key | `Msg` |
| ----- | --- | ----- |
| Process Picker | `/` | `FilterProcesses(query)` — incremental search |
| Process Picker | `Enter` | `Attach(pid)` |
| Scan Panel | `s` | `StartScan(expr)` |
| Scan Panel | `n` | `NarrowScan(expr)` |
| Scan Panel | `r` | `ResetScan` |
| Match View | `↑`/`↓` or `j`/`k` | `SelectNext` / `SelectPrev` |
| Match View | `o` | `CycleSort(column)` |
| Match View | `/` | `FilterMatches(query)` |
| Match View | `a` | `AddCheat(selected_row)` |
| Cheat View | `Space` | `ToggleFreeze(row)` |
| Cheat View | `e` | `EditValue(row)` (opens an inline edit prompt) |
| Cheat View | `h` | `FocusHexView(row)` |
| Hex View | `←`/`→`/`↑`/`↓` | move cursor within the buffer |
| Hex View | `Enter` | commit an in-progress byte edit |

The table above is illustrative, not exhaustive — `ui/keymap.rs` is the single source of truth and
`help_overlay.rs` renders directly from it, so the two never drift.

---

## 5. `ratatui` widget notes

- **Process picker** (`ui/process_picker.rs`): `ratatui::widgets::Table` rendered from an
  `AppState`-owned, already-filtered `Vec<ProcessRow>`; the `/` incremental filter mutates that state
  through a `Msg`, same as every other action — replaces the Python process-list `TreeView` + GTK filter
  callback with a normal `update()` step.
- **Match/cheat grids** (`ui/match_view.rs`, `ui/cheat_view.rs`): `ratatui::widgets::Table` with sort
  column/direction and filter query stored in `AppState`; `app::update` re-sorts/re-filters the backing
  `Vec` on the relevant `Msg` rather than a widget maintaining a live model — the immediate-mode equivalent
  of `SortListModel`/`FilterListModel`. Inline editing is a modal single-line prompt (a small `Paragraph`
  overlay) rather than an in-cell GTK `Entry`/`CheckButton`, since a terminal cell can't host a live widget.
- **`HexView`** (`ui/hex_view.rs`): three `ratatui` columns (offset / hex / ascii) built from one buffer of
  bytes plus a cursor position and selection range in `AppState`; the “TextTag” highlighting `hexview.py`
  used for the selection becomes styled `Span`s picked per-cell at render time — concept-for-concept port,
  not a line-by-line translation.
- **Help overlay** (`ui/help_overlay.rs`): a centered `ratatui` popup (`Clear` + `Block` + `Table`) listing
  every binding active for `AppState::focus`, generated from `ui/keymap.rs` so it can never fall out of
  sync with what actually works.
- **Key → `Msg` shim rule**: `ui/input.rs` is the only place `crossterm::event::KeyEvent`s are read; it
  looks the key up in `ui/keymap.rs` for the current focus, builds a `Msg`, and calls
  `app::update(&mut state, msg)`. `ui/` panels never read input directly — this keeps the discipline the
  original plan established for GTK signal handlers, just retargeted at terminal key events.

---

## 6. Testing strategy

Generic rules: [ARCHITECTURE.md §6](./ARCHITECTURE.md#6-testing-and-coverage).

- **`app/` unit tests**: `update()` for every `Msg` variant (including focus-cycling and keymap-triggered
  actions) against a mocked/attached `Session` (or the real `fake_target` helper from `libscanmem`) — no
  terminal involved, runs under `--no-default-features`.
- **`ui/keymap.rs` tests**: table-driven — for every documented binding in §4, assert the lookup produces
  the expected `Msg`; catches drift between the keymap and the help overlay/docs.
- **Cheat-list persistence**: round-trip serialize/deserialize tests in `app/cheatlist.rs`.
- **`ui/` tests**: widget construction/render smoke tests using `ratatui::backend::TestBackend` (an
  in-memory backend, no real terminal or CI display server needed, unlike the GTK plan's headless-Xvfb
  requirement) — keep these minimal; the bulk of logic coverage lives in `app/`.
- **`HexView`**: unit-testable against a fake in-memory byte buffer independent of any attached process,
  per §0/§5.

---

## 7. Dependencies

Generic policy: [ARCHITECTURE.md §7](./ARCHITECTURE.md#7-dependencies).

| Area | Crate | Notes |
| ---- | ----- | ----- |
| Engine | `libscanmem` | workspace path dependency |
| TUI | `ratatui` | feature `tui`, default on |
| Terminal backend | `crossterm` | feature `tui`, default on; cross-platform, pure Rust |
| CLI | `clap` (derive) | `--pid` |
| Cheat-list persistence | `serde`, `serde_json` (optional) | |
| Logging | `tracing`, `tracing-subscriber` | file-only sink; never stdout/stderr while the TUI owns the terminal |
| Errors | `thiserror` | |

**Explicitly rejected for this crate, permanently**: GTK3/GTK4 (`gtk`/`gtk4`, `gio`, `glib`, `gdk4`),
`iced`, and any other GUI/windowing toolkit.

---

## 8. Phased steps

### Phase 0 — Crate bootstrap

Repo-wide renaming/licensing/CI fixes, including adding `gameconqueror/` as a new workspace member, are
already done. `gameconqueror`-specific bootstrap:

- [x] `cli/`, `settings/`, `logger/` (file-only sink), empty `app/` and `ui/` skeletons.
- [x] Vertical slice: a `ratatui` shell that draws an empty frame with a status bar and quits cleanly on
  `Ctrl+Q`, restoring the terminal on both normal exit and panic (a `Drop` guard or panic hook around
  `crossterm::terminal::disable_raw_mode`/leaving the alternate screen — a raw terminal left broken on
  panic is the single most common TUI footgun).

**Verify**: `cargo run -p gameconqueror` opens a blank TUI that quits cleanly; terminal is left in a normal
state after both clean quit and a simulated panic; `cargo build -p gameconqueror --no-default-features`
still compiles (`app/` only, no `ratatui`/`crossterm`).

### Phase 1 — Core (`app/`)

- [ ] `AppState`, `Msg` enum covering attach/scan/narrow/write/cheat-list/focus/help operations.
- [ ] `app/focus.rs`: `Focus` enum and cycling order.
- [ ] `update(state, msg)` calling `libscanmem::Session`; no `ratatui`/`crossterm` types anywhere in `app/`.
- [ ] Unit tests for every `Msg` variant against `fake_target`.

**Verify**: `cargo test -p gameconqueror --no-default-features` green — the core works with zero terminal
UI compiled in.

### Phase 2 — Shell, keymap, and process picker

- [ ] `ui/layout.rs`: top-level frame layout, status/help bar showing current focus.
- [ ] `ui/keymap.rs` + `ui/input.rs`: global bindings (§4.2) wired end-to-end.
- [ ] `ui/process_picker.rs`: `Table` over `/proc` with incremental `/`-filter.
- [ ] Wire picker selection → `Msg::Attach` → `app::update`.

**Verify**: manual — pick a real process with keys alone, confirm attach succeeds/fails visibly in the
status bar.

### Phase 3 — Scan panel + match grid

- [ ] `ui/scan_panel.rs`: data-type/match-type/value input line → `Msg::StartScan`/`Msg::NarrowScan`.
- [ ] `ui/match_view.rs`: `Table` over scan results, sort/filter state in `AppState`, driven by keys (§4.3).

**Verify**: manual first-scan → narrow loop against `fake_target`, matches shown and narrow correctly,
using only the keyboard.

### Phase 4 — Cheat list

- [ ] `ui/cheat_view.rs`: add-from-match-view, freeze toggle, inline value edit via prompt, description
  field.
- [ ] `app/cheatlist.rs` persistence (TOML/JSON via `serde`); `Ctrl+S`/`Ctrl+L` wired to it.

**Verify**: manual — freeze a value, confirm it's rewritten repeatedly; save/reload a cheat list file, all
via hotkeys.

### Phase 5 — HexView + help overlay

- [ ] `ui/hex_view.rs`: 3-pane offset/hex/ascii view over a byte buffer, cursor/selection in `AppState`,
  styled `Span`s for highlighting.
- [ ] Develop/test against a fake in-memory payload first, then wire to `Session::read`/`write` for a
  selected match's surrounding bytes.
- [ ] `ui/help_overlay.rs`: `?`/`F1` popup generated from `ui/keymap.rs`.

**Verify**: manual — open hex view on a known address, edit a byte via the keyboard, confirm the write
round-trips through `Session`; help overlay shows accurate bindings in every focus.

### Phase 6 — Polish + first release (TUI v0.1)

- [ ] Color theme respecting `NO_COLOR`/terminal capability, consistent with the `scanmem` CLI's approach
  (see repo conventions — no new color-detection logic, reuse the same `std::io::IsTerminal` pattern).
- [ ] Document the privilege-escalation story (running as root / a documented `sudo`/`pkexec`-wrapper)
  without committing to porting the old polkit policy file verbatim.
- [ ] README, CHANGELOG; tag **v0.1.0**.

**Verify**: [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit) gates at
all three feature levels; dogfood full attach → scan → cheat → hex-edit workflow using only the keyboard.

---

## 9. Definition of done (v0.1.0, TUI)

- [ ] `app/` compiles and passes tests under `--no-default-features` — zero `ratatui`/`crossterm` types
  outside `ui/`.
- [ ] No FFI/ctypes-style integration — `gameconqueror` depends on `libscanmem::Session` as an ordinary
  typed Rust dependency.
- [ ] No GTK, no `iced`, no windowing toolkit anywhere in the crate or its dependency tree.
- [ ] Every action reachable through a documented hotkey; `?`/`F1` help overlay matches `ui/keymap.rs`
  exactly (enforced by the table-driven test in §6).
- [ ] Full attach → scan → narrow → cheat-list → hex-edit workflow works end-to-end, keyboard-only.
- [ ] Terminal is restored to a normal state on clean exit **and** on panic.
- [ ] CI green per [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit)
  across `--no-default-features`, default, `--all-features`.
