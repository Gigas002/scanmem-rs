# gameconqueror — usage guide

`gameconqueror` is a keyboard-only **terminal UI** front-end for `libscanmem`: attach to a running
process, scan its memory for a value, narrow the match set down, and write new values back —
without ever leaving the terminal or touching a mouse.

---

## 1. Building and running

```sh
cargo build -p gameconqueror --release
```

`gameconqueror` needs permission to `ptrace`/read-write `/proc/<pid>/mem` for whatever process you
attach to — the same requirement `scanmem`'s CLI has. Whether you actually need to elevate for a
*given* target depends on your kernel's `kernel.yama.ptrace_scope` setting and on whether the
target already opted itself out of ptrace restriction (some game engines' crash handlers do this;
see the Troubleshooting note below) — plenty of targets attach with zero elevation. When you do
need it, pick one:

```sh
# 1. sudo — simplest, works everywhere, asks every time
sudo ./target/release/gameconqueror
sudo ./target/release/gameconqueror --pid 12345   # attach to a known pid immediately

# 2. setcap — grant the capability to the binary once, run unprivileged after that
sudo setcap cap_sys_ptrace+ep ./target/release/gameconqueror
./target/release/gameconqueror
# Note: this persists on the binary itself — anyone able to run it inherits the capability, and
# it must be re-applied after every rebuild (setcap doesn't survive `cargo build` recompiling the
# file). Reasonable for a personal workstation binary; not something to ship broadly.

# 3. pkexec — desktop polkit auth dialog instead of a terminal sudo prompt
pkexec env TERM="$TERM" ./target/release/gameconqueror
# pkexec strips almost all environment variables for security, including TERM — passing it through
# explicitly (as above) avoids a raw/mis-rendered terminal frame. This uses polkit's default
# "authenticate as an administrator" rule; it does not require porting the legacy GameConqueror
# GTK app's `org.freedesktop.gameconqueror.policy` file.
```

| Flag             | Description                                                                                                     |
| ---------------- | ----------------------------------------------------------------------------------------------------------------- |
| `--pid <PID>`    | Attach to this pid immediately on startup, instead of starting on the Process Picker.                          |
| `--config <PATH>` | Load settings from this `config.toml` instead of the conventional `$XDG_CONFIG_HOME/gameconqueror/config.toml` (`config` feature only). |
| `--theme <PATH>`  | Load colors from this `theme.toml` instead of the conventional `$XDG_CONFIG_HOME/gameconqueror/theme.toml` (`config` feature only).     |

Logs are written to a file only (never stdout/stderr, which would corrupt the terminal frame) —
by default `$TMPDIR/gameconqueror.log` (configurable via `config.toml`, see below).

Colors follow the [`NO_COLOR`](https://no-color.org/) convention (same check as `scanmem`'s CLI):
set `NO_COLOR=1` to fall back to bold/reverse-video styling instead of ANSI colors, regardless of
what `theme.toml` says. Otherwise, every color gameconqueror uses is customizable — see
[Configuration and theming](#configuration-and-theming) below.

### Cargo features

| Feature      | Default | Adds                                                                                                                                                                                                        |
| ------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tui`        | **on**  | The `ratatui`/`crossterm` terminal UI itself. Building without it produces a binary with no UI (see below).                                                                                               |
| `cheat-list` | **on**  | The Cheat View panel: pinning matches, freezing/rewriting their value continuously, and saving/loading a cheat list to disk.                                                                              |
| `hex-view`   | **off** | The Hex View panel: raw byte-level inspection/editing of memory around an address, independent of the match-tracking system. Opt-in since most editing goes through the Cheat View once it's enabled.    |
| `config`     | **on**  | Loading `config.toml`/`theme.toml` (`--config`/`--theme`, or their conventional paths). Without it, gameconqueror runs on built-in defaults only — `cheat-list` pulls this in regardless, since its cheat-list file persistence also needs `serde`/`toml`. |

```sh
# default build: process picker, scan panel, match view, cheat list — no hex view
cargo build -p gameconqueror

# with the hex view panel enabled too
cargo build -p gameconqueror --features hex-view

# a minimal build: process picker, scan panel, match view only
cargo build -p gameconqueror --no-default-features --features tui

# everything
cargo build -p gameconqueror --all-features
```

Building with `--no-default-features` compiles the core state machine only, with no terminal UI at
all — running that binary just prints a message and exits; it exists so the application core can
be tested/verified independently of `ratatui`, not as an end-user configuration.

### Configuration and theming

Two independent, optional TOML files (`config` feature, on by default) — general settings and
colors are deliberately separate files, loaded independently:

| File          | Flag        | Conventional path                          | Contents                                                        |
| ------------- | ----------- | ------------------------------------------- | ----------------------------------------------------------------|
| `config.toml` | `--config`  | `$XDG_CONFIG_HOME/gameconqueror/config.toml` | Startup pid, log level/path, poll interval, Hex View buffer size, default scan type/match type. |
| `theme.toml`  | `--theme`   | `$XDG_CONFIG_HOME/gameconqueror/theme.toml`  | Every customizable color (see below).                           |

Both fall back to `~/.config/gameconqueror/...` if `$XDG_CONFIG_HOME` isn't set. Neither file is
required — a missing conventional path is normal and silently uses built-in defaults; a missing
*explicitly requested* path (via `--config`/`--theme`), or a file that exists but fails to parse
or has an invalid value, is a startup error. Every key in both files is optional — set only what
you want to change from the default. Fully commented, ready-to-copy examples with every key live
at [`examples/gameconqueror/config.toml`](../examples/gameconqueror/config.toml) and
[`examples/gameconqueror/theme.toml`](../examples/gameconqueror/theme.toml).

**`theme.toml`** colors are `ratatui`'s own color syntax: a name (`"cyan"`, `"light red"`, `"dark
gray"`, ...), `"#rrggbb"` hex, or a `0`-`255` terminal palette index. Every element gameconqueror
colors is a separate key:

| Key                    | What it colors                                                                    |
| ----------------------- | ---------------------------------------------------------------------------------- |
| `focused-border`        | Border of whichever panel currently has focus.                                    |
| `status-bar-fg`/`-bg`   | Status bar, normal state.                                                         |
| `status-bar-error-fg`/`-bg` | Status bar while showing an error.                                            |
| `scan-progress`         | Scan Panel's progress gauge fill.                                                 |
| `selection`             | Selected row (every table) and the Hex View cursor cell — a background color, replacing the default reverse-video highlight. |
| `match-changed`         | Match View's value cell for a match whose value changed on the most recent scan/refresh (§6). |
| `frozen`                | Cheat View's Frozen column for a currently-frozen cheat (§7).                     |

`NO_COLOR=1` overrides every key above with a colorless (bold/reverse-video-only) fallback,
regardless of what `theme.toml` says.

---

## 2. The focus model

There is no mouse. Every panel is visible at once, arranged in a fixed grid (like `bottom`'s
widget layout):

```
┌─────────────────┬───────────────────────────┐
│ Process Picker   │ Scan Panel                │
├─────────────────┴───────────────────────────┤
│ Match View       │ Cheat View (cheat-list)   │
├───────────────────────────────────────────────┤
│ Hex View (hex-view)                          │
└───────────────────────────────────────────────┘
```

> Without the `cheat-list` feature (§1), Match View spans the full width of its row instead of
> sharing it with Cheat View. Without the `hex-view` feature, the Hex View row is dropped
> entirely, leaving a 2-row grid.

Exactly one panel has **focus** at any time — its border is highlighted — and every panel-specific
key binding (§4-§8) is interpreted relative to it. Typing (e.g. into a filter or the Scan Panel's
value input) only affects the focused panel; every other panel keeps showing its own live state
untouched, since they're all rendered every frame regardless of focus.

| Key                 | Action                                                                                                                                    |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Ctrl` + arrow key  | Move focus to whichever panel sits in that direction on the grid above                                                                    |
| `Tab` / `Shift+Tab` | Cycle focus forward/backward through every panel, in a fixed order (see below) — a fallback for terminals that don't forward `Ctrl+Arrow` |
| `Ctrl+E`            | Expand the focused panel to fill the whole screen, or collapse back to the grid                                                           |

The `Tab`/`Shift+Tab` cycling order is:

```
Process Picker → Scan Panel → Match View → [Cheat View, if built with `cheat-list`]
  → [Hex View, if built with `hex-view`] → (back to Process Picker)
```

While a panel is expanded (`Ctrl+E`), only it is drawn — the others are hidden until you collapse
back, since there's nothing else on screen to move focus to.

The status bar at the bottom of the screen always shows the currently focused panel's name
(and `(expanded)` when applicable), the result of your last action, and a one-line hint of the
global bindings.

> Cheat View only appears in the grid and the `Tab` cycle at all when `gameconqueror` is built with
> the `cheat-list` feature (§1, on by default); Hex View likewise only appears when built with the
> (opt-in) `hex-view` feature. `Tab` skips whichever of the two isn't built.

---

## 3. Global hotkeys (work in every panel)

| Key                | Action                                                                                                                                   |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `Ctrl` + arrow key | Focus the panel spatially adjacent in that direction on the grid (§2)                                                                    |
| `Ctrl+E`           | Expand the focused panel fullscreen, or collapse back to the grid                                                                        |
| `Tab`              | Focus next panel (fixed cycling order, §2)                                                                                               |
| `Shift+Tab`        | Focus previous panel                                                                                                                     |
| `?` or `F1`        | Toggle the help overlay for the current panel — lists every binding active for the focused panel, generated straight from `ui/keymap.rs` |
| `Ctrl+Q`           | Quit                                                                                                                                     |
| `Ctrl+D`           | Detach from the current process, resuming its execution — no-op if nothing is attached                                                   |
| `Esc`              | Dismiss — closes the help overlay if open, otherwise cancels an active search/edit and clears it                                         |

The status bar also always shows whether a process is currently attached (and its pid/name), regardless of which panel is focused.

---

## 4. Process Picker

The starting panel: a table of every process visible under `/proc`, refreshed automatically on
startup.

| Key       | Action                                                                                  |
| --------- | --------------------------------------------------------------------------------------- |
| `↑` / `↓` | Move the selection                                                                      |
| `/`       | Enter incremental search — type to filter by pid or process name                        |
| `Enter`   | While searching: confirm and keep the filter. Otherwise: attach to the selected process |

While searching, printable characters append to the filter and `Backspace` removes the last
character; `Esc` clears the filter and exits search instead of keeping it.

**Example** — find and attach to a process named `fake_target`:

1. Press `/`, type `fake_target`.
2. Press `Enter` to stop editing the filter (or just press `Enter` again on the highlighted row —
   `Enter` also attaches once the filter has narrowed the list to the one you want).
3. The status bar reports `attached to pid <pid>: N region(s)` on success, or an error otherwise.

---

## 5. Scan Panel

Configure and run a scan against the attached process, mirroring `scanmem`'s CLI scan grammar but
driven by keys instead of typed commands.

| Key | Action                                                                                                                           |
| --- | -------------------------------------------------------------------------------------------------------------------------------- |
| `t` | Cycle the data type (`i8`/`i16`/`i32`/`i64`/`f32`/`f64`/`any`/`anyint`/`anyfloat`/`bytes`/`string`)                              |
| `m` | Cycle the match type (`=`, `!=`, `>`, `<`, `range`, `update`, `unchanged`, `changed`, `increased`, `decreased`, `+`, `-`, `any`) |
| `/` | Edit the value/range input                                                                                                       |
| `s` | Scan — narrows the current matches against the value/type above if any are already recorded, otherwise scans from scratch        |
| `n` | New scan — always discards the current matches and scans every considered byte from scratch, even if matches already exist       |
| `r` | Refresh — re-reads every current match's value in place, without narrowing; never drops a match just because its value changed   |

Match types that take no value (`any`, `update`, `unchanged`, `changed`, `increased`, `decreased`)
must be run with an empty input; `range` expects two whitespace-separated bounds (`low high`); every
other match type requires exactly one value.

`s` only ever touches addresses already in the match set once some exist — it narrows, it never
discovers new addresses. `n` is the way to force a fresh full-region scan (e.g. after changing the
data type, or to start over with a different value) without first clearing the match set by hand;
setting the data type to `any` and the match type to `any` before pressing `n` records every byte
as a candidate, the starting point for an "I don't know the exact value, but I'll watch it change"
search — then narrow with `increased`/`decreased`/`changed` as usual.

Attaching does not pause the target — it keeps running normally so you can keep playing while you
search. `s`, `n`, and `r` all run on a background thread and only pause the target for the
scan/refresh itself, so the rest of the TUI stays responsive while they work: the Scan Panel shows
a live progress gauge for the duration (the status bar shows a percentage too, regardless of which
panel is focused); press `Esc` to cancel early and keep whatever partial matches were found before
the cancellation took effect.

**Example** — find a 32-bit integer currently equal to `100`, then narrow to `95` after it changes
in-game:

1. Focus the Scan Panel (`Tab` from the Process Picker after attaching).
2. Confirm `Type: i32` (press `t` to cycle if it isn't).
3. Confirm `Match: =` (press `m` to cycle if it isn't).
4. Press `/`, type `100`, press `Enter`.
5. Press `s` to run the first scan — the status bar reports the match count.
6. Change the value in the target program, then press `/`, type `95`, `Enter`, `s` again to narrow.

**Example** — check whether a match you found earlier is still holding its value, without
narrowing anything: focus the Scan Panel and press `r`. The Match View's values update in place;
the match count never changes from this alone.

---

## 6. Match View

A table of the current match set (address + value), sorted and filtered live from `AppState` —
there's no separate "apply filter" step.

| Key                | Action                                                                             |
| ------------------ | ---------------------------------------------------------------------------------- |
| `↑`/`↓` or `k`/`j` | Move the selection                                                                 |
| `o`                | Cycle the sort column (address ↔ value)                                            |
| `/`                | Edit the incremental filter (matches against the hex address or the decimal value) |

As with the Process Picker, `Backspace` edits the filter while active and `Esc` clears it instead
of keeping it.

| Key | Action                                                               |
| --- | -------------------------------------------------------------------- |
| `h` | Open the Hex View on the bytes around the selected match _(`hex-view` build only)_ |
| `a` | Add the selected match to the cheat list _(`cheat-list` build only)_ |

> A cheat added via `a` stores the same full-width value shown in the Value column (e.g. an `i32`
> match records all 4 bytes), not just its first byte — freezing it rewrites that whole width on
> every tick, so it holds steady without corrupting neighboring bytes.

**Highlighting what just changed**: a match's Value cell is colored (`theme.toml`'s
`match-changed`, yellow by default) when its value differs from what it was on the *previous*
scan/refresh — press `r` (Scan Panel, §5) to re-read every current match's value and watch which
rows light up between presses. A match that's held steady since the last look stays plain; a
newly discovered match (first scan, or one that only just started matching) is never flagged
"changed" on the scan that finds it — there's nothing yet to compare it against.

---

## 7. Cheat View _(requires the `cheat-list` feature, on by default)_

A table of every recorded cheat: address, description, value, and whether it's frozen. Built by
default; build with `--no-default-features --features tui` to leave it out instead.

| Key                | Action                                                                                                     |
| ------------------ | ---------------------------------------------------------------------------------------------------------- |
| `↑`/`↓` or `k`/`j` | Move the selection                                                                                         |
| `Space`            | Toggle freeze on the selected cheat                                                                        |
| `e`                | Edit the selected cheat's value inline (prefilled with its current value; `Enter` confirms, `Esc` cancels) |
| `h`                | Open the Hex View on the bytes around the selected cheat _(`hex-view` build only)_                         |

**Freezing**: while a cheat is frozen, `gameconqueror` rewrites its stored value to its address on
every idle tick (a few times a second) for as long as the TUI is running and a session stays
attached — so even if the target process (or another tool) overwrites that address, it snaps back.
Unfreezing (`Space` again) stops the rewriting; the address is left as last written. The Frozen
column is colored (`theme.toml`'s `frozen`, magenta by default) for every currently-frozen row, so
a glance down the column tells you what's actively held steady versus just recorded.

**Global bindings** (work from any panel, not just Cheat View):

| Key      | Action                                                                                                                   |
| -------- | ------------------------------------------------------------------------------------------------------------------------ |
| `Ctrl+S` | Save the cheat list — writes directly if a path is already known (from a prior save/load), otherwise opens a path prompt |
| `Ctrl+L` | Load a cheat list — always opens a path prompt, replacing the current cheat list on success                              |

While a save/load path prompt is open, it captures every key exclusively: type the path, `Enter`
confirms, `Esc` cancels without saving/loading. The cheat list file is TOML (not the legacy
`gameconqueror`/GTK format) — see [gameconqueror-plan.md §3](./gameconqueror-plan.md) for the
schema rationale.

**Example** — freeze a match's value, then save the cheat list:

1. From the Match View, select a match and press `a` to add it to the cheat list.
2. `Tab` to the Cheat View — the new entry appears with an empty description.
3. Press `Space` to freeze it; the status bar confirms.
4. Press `Ctrl+S`; since no path is known yet, a prompt opens. Type a path (e.g.
   `cheats.toml`) and press `Enter`. Subsequent `Ctrl+S` presses save to that same path directly.
5. Press `Ctrl+L` any time afterward, type the same path, `Enter`, to reload it (e.g. after
   restarting `gameconqueror` and reattaching).

---

## 8. Hex View _(requires the `hex-view` feature, off by default)_

A 3-pane offset/hex/ASCII view over a window of bytes around an address — raw byte-level
inspection/editing independent of the match-tracking system, useful for viewing/editing memory
context (neighboring fields, struct layout) that the Match/Cheat Views don't show. Build with
`cargo build -p gameconqueror --features hex-view` to enable it — it then also becomes reachable
in the `Tab`/`Shift+Tab` focus cycle (§2). Its grid tile is always visible once built, but empty
until you populate it by pressing `h` on a selected row in the Match View (§6) or Cheat View (§7).

| Key              | Action                                                             |
| ---------------- | ------------------------------------------------------------------ |
| `←`/`→`          | Move the cursor one byte left/right                                |
| `↑`/`↓`          | Move the cursor one row (16 bytes) up/down                         |
| `0`-`9`, `a`-`f` | Type a hex digit into the byte under the cursor (up to two digits) |
| `Backspace`      | Remove the last typed digit                                        |
| `Enter`          | Write the composed byte to the target's address space              |
| `Esc`            | Cancel an in-progress byte edit without writing it                 |

The cursor doesn't scroll past the loaded window — re-open the Hex View (`h`) from a different row
to look elsewhere. Composing a byte edit doesn't touch memory until `Enter` commits it; the status
bar reports the write's result (or why it failed, e.g. no process attached).

**Example** — edit a byte directly, without adding it to the cheat list first:

1. From the Match View, select a match and press `h` — the Hex View opens with the cursor on that
   match's address.
2. Type two hex digits, e.g. `4` then `2`.
3. Press `Enter` — the status bar confirms the write, or reports why it failed.

---

## 9. Troubleshooting

| Symptom                                                     | Cause                                                                                                                                                                                                |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Attach fails with a permissions error                        | Run under `sudo`, as root, or grant the binary `CAP_SYS_PTRACE` (§1). Check `cat /proc/sys/kernel/yama/ptrace_scope` — `1` (the common default) restricts attaching to non-descendant processes unless you have `CAP_SYS_PTRACE`. |
| Attach *succeeds* without `sudo`, and that's surprising       | Some targets opt themselves out of ptrace restriction entirely, independent of `ptrace_scope` — e.g. Unity's crash handler process registers `prctl(PR_SET_PTRACER, PR_SET_PTRACER_ANY)` so it can attach to its own parent game process, which as a side effect lets *any* same-user process (`gameconqueror` included) attach too. Not a bug in either program — the target voluntarily disabled its own protection. |
| Terminal looks broken after a crash                          | Shouldn't happen — a panic hook restores the terminal (disables raw mode, leaves the alternate screen) before the default panic message prints. If it does happen anyway, run `reset` in your shell. |
| Nothing happens when I run the binary                        | Check whether it was built with `--no-default-features` (no `tui` feature) — that build has no terminal UI by design.                                                                                |
| Exits immediately with a `config.toml`/`theme.toml` error      | The message names the exact key and file — a missing *explicitly requested* `--config`/`--theme` path, a malformed TOML file, or a value gameconqueror doesn't recognize (e.g. a typo'd color name or scan type) all fail fast at startup rather than falling back silently. A missing *conventional* path (no `--config`/`--theme` given) is never the cause — that's normal and just uses defaults. |
