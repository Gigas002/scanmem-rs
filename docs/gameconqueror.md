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
attach to. In practice that means running it as root, via `sudo`, or with `CAP_SYS_PTRACE` granted
to the binary — the same requirement `scanmem`'s CLI has.

```sh
sudo ./target/release/gameconqueror
# or attach to a known pid immediately on startup:
sudo ./target/release/gameconqueror --pid 12345
```

| Flag          | Description                                                                           |
| ------------- | ------------------------------------------------------------------------------------- |
| `--pid <PID>` | Attach to this pid immediately on startup, instead of starting on the Process Picker. |

Logs are written to a file only (never stdout/stderr, which would corrupt the terminal frame) —
by default `$TMPDIR/gameconqueror.log` (typically `/tmp/gameconqueror.log`).

### Cargo features

| Feature      | Default | Adds                                                                                                                                                                                                                                  |
| ------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tui`        | **on**  | The `ratatui`/`crossterm` terminal UI itself. Building without it produces a binary with no UI (see below).                                                                                                                           |
| `cheat-list` | **off** | The Cheat View panel: pinning matches, freezing/rewriting their value continuously, and saving/loading a cheat list to disk. Disabled by default because it's the newest, least-stable panel — enable it explicitly once you want it. |

```sh
# default build: process picker, scan panel, match view — no cheat list
cargo build -p gameconqueror

# with the cheat list panel enabled
cargo build -p gameconqueror --features cheat-list

# everything
cargo build -p gameconqueror --all-features
```

Building with `--no-default-features` compiles the core state machine only, with no terminal UI at
all — running that binary just prints a message and exits; it exists so the application core can
be tested/verified independently of `ratatui`, not as an end-user configuration.

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
│ Hex View                                     │
└───────────────────────────────────────────────┘
```

> Without the `cheat-list` feature (§1), Match View spans the full width of its row instead of
> sharing it with Cheat View.

Exactly one panel has **focus** at any time — its border is highlighted — and every panel-specific
key binding (§4-§8) is interpreted relative to it. Typing (e.g. into a filter or the Scan Panel's
value input) only affects the focused panel; every other panel keeps showing its own live state
untouched, since they're all rendered every frame regardless of focus.

| Key                 | Action                                                                 |
| ------------------- | ----------------------------------------------------------------------- |
| `Ctrl` + arrow key  | Move focus to whichever panel sits in that direction on the grid above  |
| `Tab` / `Shift+Tab` | Cycle focus forward/backward through every panel, in a fixed order (see below) — a fallback for terminals that don't forward `Ctrl+Arrow` |
| `Ctrl+E`            | Expand the focused panel to fill the whole screen, or collapse back to the grid |

The `Tab`/`Shift+Tab` cycling order is:

```
Process Picker → Scan Panel → Match View → [Cheat View, if built with `cheat-list`] → Hex View → (back to Process Picker)
```

While a panel is expanded (`Ctrl+E`), only it is drawn — the others are hidden until you collapse
back, since there's nothing else on screen to move focus to.

The status bar at the bottom of the screen always shows the currently focused panel's name
(and `(expanded)` when applicable), the result of your last action, and a one-line hint of the
global bindings.

> Cheat View only appears in the grid and the `Tab` cycle at all when `gameconqueror` is built with
> `--features cheat-list` (§1); otherwise the grid drops it and `Tab` skips straight from Match
> View to Hex View.

---

## 3. Global hotkeys (work in every panel)

| Key                | Action                                                                                           |
| ------------------ | ------------------------------------------------------------------------------------------------ |
| `Ctrl` + arrow key | Focus the panel spatially adjacent in that direction on the grid (§2)                            |
| `Ctrl+E`           | Expand the focused panel fullscreen, or collapse back to the grid                                |
| `Tab`              | Focus next panel (fixed cycling order, §2)                                                       |
| `Shift+Tab`        | Focus previous panel                                                                             |
| `?` or `F1`        | Toggle the help overlay for the current panel — lists every binding active for the focused panel, generated straight from `ui/keymap.rs` |
| `Ctrl+Q`           | Quit                                                                                             |
| `Ctrl+D`           | Detach from the current process, resuming its execution — no-op if nothing is attached           |
| `Esc`              | Dismiss — closes the help overlay if open, otherwise cancels an active search/edit and clears it |

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
| `s` | Run the scan (first scan or narrowing scan, picked automatically based on whether matches already exist)                         |
| `n` | Snapshot — reseed the match set with every byte of every considered memory region                                                |
| `r` | Reset — discard the current match set without detaching                                                                          |

Match types that take no value (`any`, `update`, `unchanged`, `changed`, `increased`, `decreased`)
must be run with an empty input; `range` expects two whitespace-separated bounds (`low high`); every
other match type requires exactly one value.

Attaching does not pause the target — it keeps running normally so you can keep playing while you
search. `s` and `n` run on a background thread and only pause the target for the scan/snapshot
itself, so the rest of the TUI stays responsive while they work: the Scan Panel shows a live
progress gauge for the duration (the status bar shows a percentage too, regardless of which panel
is focused); press `Esc` to cancel early and keep whatever partial matches were found before the
cancellation took effect.

**Example** — find a 32-bit integer currently equal to `100`, then narrow to `95` after it changes
in-game:

1. Focus the Scan Panel (`Tab` from the Process Picker after attaching).
2. Confirm `Type: i32` (press `t` to cycle if it isn't).
3. Confirm `Match: =` (press `m` to cycle if it isn't).
4. Press `/`, type `100`, press `Enter`.
5. Press `s` to run the first scan — the status bar reports the match count.
6. Change the value in the target program, then press `/`, type `95`, `Enter`, `s` again to narrow.

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

| Key | Action                                                       |
| --- | ------------------------------------------------------------ |
| `h` | Open the Hex View on the bytes around the selected match      |
| `a` | Add the selected match to the cheat list _(`cheat-list` build only)_ |

> A cheat added via `a` stores only the single raw byte at the match's address (the same value
> shown in the Value column), not the full width of a multi-byte scan (e.g. an `i32`) — a
> limitation of the current match-tracking data, not of the cheat list itself. Freezing it still
> writes exactly that one byte back, so it never corrupts neighboring bytes; it just won't hold a
> wider value steady on its own. Opening the Hex View with `h` instead lets you edit any byte
> directly, independent of the match's tracked width.

---

## 7. Cheat View _(requires the `cheat-list` feature)_

A table of every recorded cheat: address, description, value, and whether it's frozen. Build with
`cargo build -p gameconqueror --features cheat-list` to enable it — it then also becomes reachable
in the `Tab`/`Shift+Tab` focus cycle (§2).

| Key       | Action                                                             |
| --------- | ------------------------------------------------------------------- |
| `↑`/`↓` or `k`/`j` | Move the selection                                          |
| `Space`   | Toggle freeze on the selected cheat                                |
| `e`       | Edit the selected cheat's value inline (prefilled with its current value; `Enter` confirms, `Esc` cancels) |
| `h`       | Open the Hex View on the bytes around the selected cheat            |

**Freezing**: while a cheat is frozen, `gameconqueror` rewrites its stored value to its address on
every idle tick (a few times a second) for as long as the TUI is running and a session stays
attached — so even if the target process (or another tool) overwrites that address, it snaps back.
Unfreezing (`Space` again) stops the rewriting; the address is left as last written.

**Global bindings** (work from any panel, not just Cheat View):

| Key      | Action                                                                                     |
| -------- | -------------------------------------------------------------------------------------------- |
| `Ctrl+S` | Save the cheat list — writes directly if a path is already known (from a prior save/load), otherwise opens a path prompt |
| `Ctrl+L` | Load a cheat list — always opens a path prompt, replacing the current cheat list on success   |

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

## 8. Hex View

A 3-pane offset/hex/ASCII view over a window of bytes around an address. Its grid tile is always
visible (§2) but empty until you populate it by pressing `h` on a selected row in the Match View
(§6) or Cheat View (§7).

| Key                 | Action                                                              |
| ------------------- | -------------------------------------------------------------------- |
| `←`/`→`              | Move the cursor one byte left/right                                 |
| `↑`/`↓`              | Move the cursor one row (16 bytes) up/down                          |
| `0`-`9`, `a`-`f`     | Type a hex digit into the byte under the cursor (up to two digits)  |
| `Backspace`          | Remove the last typed digit                                         |
| `Enter`              | Write the composed byte to the target's address space               |
| `Esc`                | Cancel an in-progress byte edit without writing it                  |

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

| Symptom                               | Cause                                                                                                                                                                                                |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Attach fails with a permissions error | Run under `sudo`, as root, or grant the binary `CAP_SYS_PTRACE`.                                                                                                                                     |
| Terminal looks broken after a crash   | Shouldn't happen — a panic hook restores the terminal (disables raw mode, leaves the alternate screen) before the default panic message prints. If it does happen anyway, run `reset` in your shell. |
| Nothing happens when I run the binary | Check whether it was built with `--no-default-features` (no `tui` feature) — that build has no terminal UI by design.                                                                                |
