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

There is no mouse. Exactly one panel has **focus** at any time, and every other key binding is
interpreted relative to it. `Tab` / `Shift+Tab` cycle focus forward/backward through, in order:

```
Process Picker → Scan Panel → Match View → [Cheat View, if built with `cheat-list`] → Hex View → (back to Process Picker)
```

The status bar at the bottom of the screen always shows the currently focused panel's name, the
result of your last action, and a one-line hint of the global bindings.

> Hex View is a placeholder in the current build (planned for a later phase) — focusing it shows a
> "not yet implemented" message.

---

## 3. Global hotkeys (work in every panel)

| Key         | Action                                                                                           |
| ----------- | ------------------------------------------------------------------------------------------------ |
| `Tab`       | Focus next panel                                                                                 |
| `Shift+Tab` | Focus previous panel                                                                             |
| `?` or `F1` | Toggle the help overlay for the current panel _(planned — not yet wired up)_                     |
| `Ctrl+Q`    | Quit                                                                                             |
| `Esc`       | Dismiss — closes the help overlay if open, otherwise cancels an active search/edit and clears it |

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

> Writing a new value to a selected match, and adding it to the cheat list, are implemented at the
> `app`/state-machine level (`Msg::Write`, `Msg::AddCheat`) but not yet bound to a key in this
> panel — that wiring lands with the Cheat View (§7) and Hex View phases.

---

## 7. Cheat View _(requires the `cheat-list` feature)_

Not yet built — this section will document freezing values, editing them inline, and
saving/loading a cheat list once the Cheat View panel lands. Until then, focusing this panel (only
reachable when `gameconqueror` is built with `--features cheat-list`) shows a placeholder.

Planned global bindings, once implemented:

| Key      | Action              |
| -------- | ------------------- |
| `Ctrl+S` | Save the cheat list |
| `Ctrl+L` | Load a cheat list   |

---

## 8. Hex View _(planned)_

Not yet built. Will provide a 3-pane offset/hex/ASCII view over the bytes around a selected match,
navigable and editable with the arrow keys and `Enter`.

---

## 9. Troubleshooting

| Symptom                               | Cause                                                                                                                                                                                                |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Attach fails with a permissions error | Run under `sudo`, as root, or grant the binary `CAP_SYS_PTRACE`.                                                                                                                                     |
| Terminal looks broken after a crash   | Shouldn't happen — a panic hook restores the terminal (disables raw mode, leaves the alternate screen) before the default panic message prints. If it does happen anyway, run `reset` in your shell. |
| Nothing happens when I run the binary | Check whether it was built with `--no-default-features` (no `tui` feature) — that build has no terminal UI by design.                                                                                |
