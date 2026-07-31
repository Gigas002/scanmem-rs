# gameconqueror — implementation plan

This document is the **human roadmap** and **agent playbook** for **`gameconqueror`**: a Rust port of the
GTK GUI (`gui/*.py` + `gui/GameConqueror.ui`) — first iteration on **GTK4 via `gtk4-rs`**, later migrated
to **`iced`**, both sharing one toolkit-independent core.

Background analysis: [scanmem/RUST_PORT_ANALYSIS.md](../../scanmem/RUST_PORT_ANALYSIS.md).
Generic workspace/settings/testing/quality-gate rules: [ARCHITECTURE.md](./ARCHITECTURE.md). Repo-wide
Sibling plans:
[libscanmem-plan.md](./libscanmem-plan.md) (engine, must land first), [scanmem-plan.md](./scanmem-plan.md) (CLI).

**Priority order for every design call in this plan: code quality > performance > safety > 1:1 behavioral
compatibility with the Python `GameConqueror`.**

---

## 0. Decisions locked for this plan

| Question | Decision | Rationale |
| -------- | -------- | --------- |
| GTK version for iteration 1 | **GTK4 via `gtk4-rs`** (`gtk4`/`gio`/`glib`/`gdk4` crates) | More actively maintained gtk-rs target today than the GTK3 `gtk` crate; aligns with "only fresh/non-deprecated deps" |
| UI definition source | **Hand-built widgets in Rust** (`ui/` modules), not a ported `GameConqueror.ui` Glade file | GTK4's widget set differs enough from GTK3 that the old Glade file cannot be reused as-is; hand-built widgets keep each element in its own file per [ARCHITECTURE.md §4.1](./ARCHITECTURE.md#41-ui-module) without adding a Glade/Blueprint toolchain dependency. Revisit Blueprint (`blueprint-compiler`) only if hand-written construction becomes unwieldy |
| List/grid widget | **`gtk::ColumnView` + `gio::ListStore`/`SortListModel`/`FilterListModel`**, not `gtk::TreeView` | GTK4 soft-deprecates `TreeView`/`ListStore`-as-model in favor of `ListView`/`ColumnView` over `gio::ListModel` — the fresh, recommended approach |
| Backend integration | **Direct typed calls into `libscanmem::Session`** | No FFI, no stdout-capture/regex-parsing hack — the single biggest win identified by the analysis |

---

## 1. Goals and constraints

### 1.1 Goals

- **Toolkit-independent core**: `app/` owns `AppState`, a `Msg` enum describing every user action, and an
  `update(state, msg)` function calling `libscanmem::Session`. Zero GTK types here — fully unit-testable
  without any GUI toolkit loaded (the current Python `GameConqueror` class cannot do this today, since it
  mixes state and GTK widget calls throughout).
- **GTK4 first iteration**: `ui/` widgets are thin — build a `Msg`, call `app::update`, re-render from the
  new `AppState`. This mirrors [ARCHITECTURE.md §4.2](./ARCHITECTURE.md#42-supporting-modules): `app/`
  coordinates, `ui/` renders only.
- **Two hard widgets carried over concept-for-concept, not code-for-code**: the 3-pane hex editor
  (`HexView`) and the sortable/filterable/inline-editable scan-result and cheat-list grids.
- **Structure for the future `iced` migration from day one**: 100% reuse of `app/` and the `libscanmem`
  integration is the goal; 0% reuse of widget code between GTK4 and `iced` is accepted as unavoidable
  (retained-mode `ColumnView`/`TextBuffer` vs. `iced`'s immediate-mode `view()`).

### 1.2 Non-goals / deferred

- **No** GTK3 (`gtk` crate) anywhere in this plan.
- **No** Glade/`.ui` XML porting for v0.1 — hand-built widgets only (see §0 for the Blueprint fallback).
- **`iced` migration**: tracked as a later, explicitly separate phase (§7 Phase 7) — not part of the v0.1
  definition of done. Expect to rebuild `HexView` and the grids from `iced::widget::canvas` primitives and
  hand-write the layout `GameConqueror.ui` currently provides for free.
- **No** 1:1 port of `misc.py`'s process-picker heuristics, cheat-list file format, or polkit
  (`org.freedesktop.gameconqueror.policy`) integration — revisit privilege escalation UX after v0.1 using
  whatever the current best-practice `pkexec`/polkit pattern is at that time.

### 1.3 Definitions

- **`AppState`**: current pid, attached `Session`, scan settings, match-result cache, cheat list — the
  toolkit-independent replacement for the Python `GameConqueror` class's instance state.
- **`Msg`**: one user-triggered action (`Attach(pid)`, `StartScan(expr)`, `AddCheat(row)`,
  `ToggleFreeze(row)`, `WriteValue(row, value)`, …).
- **`ui/` element**: one distinct visual/interactive piece, one file, per
  [ARCHITECTURE.md §4.1](./ARCHITECTURE.md#41-ui-module).

---

## 2. Crate layout

```text
gameconqueror/
  src/
    main.rs               # slim entry point
    cli/                  # clap: --pid, --config (rare for a GUI, but kept per ARCHITECTURE §4.3)
      mod.rs
      tests.rs
    config/                 # optional TOML: last window size, cheat-list save path, theme preference
      mod.rs
      tests.rs
    settings/
      mod.rs
      tests.rs
    logger/
      mod.rs
      tests.rs
    app/                    # AppState, Msg, update() — no GTK/iced types, fully unit-testable
      mod.rs
      state.rs
      msg.rs
      cheatlist.rs          # cheat-list persistence (serde + TOML/JSON, not the legacy format)
      tests.rs
    ui/                      # GTK4 first iteration — one file per element
      mod.rs
      window.rs              # main window shell, menu/header bar
      process_picker.rs       # ColumnView + ListStore + FilterListModel over /proc
      scan_panel.rs           # scan type/match-type/value inputs
      match_view.rs           # ColumnView over scan results, sortable/filterable
      cheat_view.rs           # ColumnView over cheat list, inline-editable freeze/value
      hex_view.rs             # 3-pane hex editor: TextView/TextBuffer/TextTag/TextMark
      tests.rs
    ui_iced/                  # feature "iced", added in Phase 7 — same Msg/AppState, iced::Application
    utils/
      mod.rs
      tests.rs
  tests/
    integration.rs            # app::update() end-to-end tests against libscanmem's fake_target, no GTK loaded
```

### 2.1 Cargo features

| Feature | Default | Gates |
| ------- | ------- | ----- |
| `gtk4` | on | `gtk4`/`gio`/`glib`/`gdk4`, `ui/` module — the shipped v0.1 experience |
| `iced` | off | `iced`, `ui_iced/` module — Phase 7, mutually exclusive with `gtk4` in a given binary invocation but both may compile under `--all-features` for CI |

`app/` and `tests/integration.rs` must build and pass under `--no-default-features` (no GUI toolkit at
all) per [ARCHITECTURE.md §1.4](./ARCHITECTURE.md#14-cargo-features-for-slim-builds) — this is what
guarantees the core is genuinely toolkit-independent, not just structured to look that way.

---

## 3. Module responsibilities (Python source → Rust module)

| Python source | Rust module | Notes |
| -------------- | ----------- | ----- |
| `scanmem.py` (ctypes + stdout regex parsing) | *(removed entirely)* | Replaced by direct `libscanmem::Session` calls from `app/` — the core integration win of the whole port |
| `GameConqueror.py` (state + widget calls mixed) | `app/state.rs` + `app/msg.rs` | Split into toolkit-independent `AppState`/`Msg`/`update()` |
| `GameConqueror.ui` (Glade) | `ui/window.rs` + per-element files | Hand-built GTK4 widgets, not a ported XML file (§0) |
| `hexview.py` | `ui/hex_view.rs` | `TextView`/`TextBuffer`/`TextTag`/`TextMark`-based 3-pane editor; most self-contained widget — develop against a fake in-memory payload independent of the rest of the app, same approach `hexview.py`'s own `__main__` demo uses |
| `misc.py` (scan-command validation, cheat-list helpers) | `app/cheatlist.rs` + validation folded into `Msg` handling | Cheat-list persistence via `serde` + TOML/JSON, not the legacy on-disk format |
| `consts.py.in` | build-time constants or `app/` | Version/paths resolved via `settings/` |

---

## 4. GTK4 widget notes

- **Process picker** (`ui/process_picker.rs`): `gtk::ColumnView` over a `gio::ListStore<ProcessRow>`, with
  a `gtk::FilterListModel` for name/pid search — replaces the Python process-list `TreeView` + manual
  filter callback.
- **Match/cheat grids** (`ui/match_view.rs`, `ui/cheat_view.rs`): `gtk::ColumnView` +
  `gtk::SortListModel`/`FilterListModel`, `gtk::SignalListItemFactory` per column (address, type, value,
  previous value, freeze checkbox, description). Inline editing via per-column custom widget factories
  (`gtk::Entry`/`gtk::CheckButton` bound through the factory's `bind`/`unbind` signals), not
  `CellRendererText`-style APIs (GTK3-era, not part of the `ColumnView` model).
- **`HexView`** (`ui/hex_view.rs`): three synchronized `gtk::TextView`s (offset/hex/ascii) or one
  `TextView` with tab-stops and `TextTag`s for coloring, `TextMark`s for the current selection —
  concept-for-concept port of `hexview.py`, not a line-by-line translation.
- **Widget → `Msg` shim rule**: every GTK signal handler is a thin closure: build a `Msg`, call
  `app::update(&mut state, msg)`, then re-render only the affected widget(s) from the returned `AppState`
  diff. This is intentionally more boilerplate than a "typical" gtk-rs app that mutates state directly in
  each handler — that discipline now is what keeps the future `iced` addition a UI-only rewrite.

---

## 5. Testing strategy

Generic rules: [ARCHITECTURE.md §6](./ARCHITECTURE.md#6-testing-and-coverage).

- **`app/` unit tests**: `update()` for every `Msg` variant against a mocked/attached `Session` (or the
  real `fake_target` helper from `libscanmem`) — no GTK loaded, runs under `--no-default-features`.
- **Cheat-list persistence**: round-trip serialize/deserialize tests in `app/cheatlist.rs`.
- **`ui/` tests**: widget construction smoke tests only (GTK requires a display or `gtk::test::init()` /
  headless Xvfb in CI) — keep these minimal; the bulk of logic coverage lives in `app/`.
- **`HexView`**: unit-testable against a fake in-memory byte buffer independent of any attached process,
  per §0/§4.

---

## 6. Dependencies

Generic policy: [ARCHITECTURE.md §7](./ARCHITECTURE.md#7-dependencies).

| Area | Crate | Notes |
| ---- | ----- | ----- |
| Engine | `libscanmem` | workspace path dependency |
| GTK4 | `gtk4`, `gio`, `glib`, `gdk4` | feature `gtk4`, default on |
| CLI | `clap` (derive) | `--pid`, `--config` |
| Config/persistence | `serde`, `toml` (config), `serde_json` (cheat list, optional) | |
| Logging | `tracing`, `tracing-subscriber` | |
| Errors | `thiserror` | |
| Future GUI | `iced` | feature `iced`, off by default, Phase 7 only |

**Explicitly rejected for v0.1**: GTK3 `gtk` crate, any Glade/`gtk::Builder` XML loading, `CellRendererText`-
based `TreeView` grids.

---

## 7. Phased steps

### Phase 0 — Crate bootstrap

Repo-wide renaming/licensing/CI fixes, including adding `gameconqueror/` as a new workspace member, are
already done. `gameconqueror`-specific bootstrap:

- [ ] `cli/`, `config/`, `settings/`, `logger/`, empty `app/` and `ui/` skeletons.
- [ ] Vertical slice: GTK4 window shell that opens and closes cleanly, no scanning yet.

**Verify**: `cargo run -p gameconqueror` opens an empty window; `cargo build -p gameconqueror --no-default-features`
still compiles (`app/` only, no GTK).

### Phase 1 — Core (`app/`)

- [ ] `AppState`, `Msg` enum covering attach/scan/narrow/write/cheat-list operations.
- [ ] `update(state, msg)` calling `libscanmem::Session`; no GTK types anywhere in `app/`.
- [ ] Unit tests for every `Msg` variant against `fake_target`.

**Verify**: `cargo test -p gameconqueror --no-default-features` green — the core works with zero GUI
toolkit compiled in.

### Phase 2 — Main window shell + process picker

- [ ] `ui/window.rs`: header bar, menu, panel layout.
- [ ] `ui/process_picker.rs`: `ColumnView` + `ListStore` + `FilterListModel` over `/proc`.
- [ ] Wire picker selection → `Msg::Attach` → `app::update`.

**Verify**: manual — pick a real process, confirm attach succeeds/fails visibly.

### Phase 3 — Scan panel + match grid

- [ ] `ui/scan_panel.rs`: data-type/match-type/value inputs → `Msg::StartScan`/`Msg::NarrowScan`.
- [ ] `ui/match_view.rs`: `ColumnView` over scan results, sortable via `SortListModel`, filterable via
  `FilterListModel`.

**Verify**: manual first-scan → narrow loop against `fake_target`, matches shown and narrow correctly.

### Phase 4 — Cheat list

- [ ] `ui/cheat_view.rs`: add-from-match-view, freeze toggle, inline value edit, description field.
- [ ] `app/cheatlist.rs` persistence (TOML/JSON via `serde`), load/save menu actions.

**Verify**: manual — freeze a value, confirm it's rewritten repeatedly; save/reload a cheat list file.

### Phase 5 — HexView

- [ ] `ui/hex_view.rs`: 3-pane offset/hex/ascii view over a byte buffer, selection/editing via
  `TextTag`/`TextMark`.
- [ ] Develop/test against a fake in-memory payload first, then wire to `Session::read`/`write` for a
  selected match's surrounding bytes.

**Verify**: manual — open hex view on a known address, edit a byte, confirm the write round-trips through
`Session`.

### Phase 6 — Polish + first release (GTK4 v0.1)

- [ ] Update/replace `.desktop`, `.metainfo.xml`, icons from `gui/org.scanmem.gameconqueror.*` for the new
  binary.
- [ ] Document the privilege-escalation story (running as root / a documented `pkexec` wrapper) without
  committing to porting the old polkit policy file verbatim.
- [ ] README, CHANGELOG; tag **v0.1.0**.

**Verify**: [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit) gates at
all three feature levels; dogfood full scan → cheat → hex-edit workflow.

### Phase 7 — `iced` migration (post-v0.1, tracked but out of scope for v0.1 definition of done)

- [ ] `ui_iced/` behind feature `iced`, reusing `app/` entirely.
- [ ] Rebuild the match/cheat grids from `iced::widget::canvas` or a virtualized list primitive — no
  `ColumnView` equivalent exists.
- [ ] Rebuild `HexView` from scratch as a custom `iced` canvas widget.
- [ ] Hand-write the layout `GameConqueror.ui`/GTK4 widget tree currently provides for free.

**Verify**: `cargo build -p gameconqueror --all-features` compiles both `ui/` and `ui_iced/`; document
which binary/flag selects which front-end.

---

## 8. Definition of done (v0.1.0, GTK4)

- [ ] `app/` compiles and passes tests under `--no-default-features` — zero GTK types outside `ui/`.
- [ ] No FFI/ctypes-style integration — `gameconqueror` depends on `libscanmem::Session` as an ordinary
  typed Rust dependency.
- [ ] GTK4 only; no `gtk` (GTK3) crate, no ported Glade XML, no `TreeView`/`CellRendererText`.
- [ ] Full attach → scan → narrow → cheat-list → hex-edit workflow works end-to-end.
- [ ] `iced` feature is scaffolding-only for v0.1 (Phase 7 not required for this release).
- [ ] CI green per [ARCHITECTURE.md §8](./ARCHITECTURE.md#8-quality-gates--required-before-every-commit)
  across `--no-default-features`, default, `--all-features`.
