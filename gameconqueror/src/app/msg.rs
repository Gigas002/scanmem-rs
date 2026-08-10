//! `Msg` — one user-triggered action, the single input to [`crate::app::update`]. No
//! `ratatui`/`crossterm` types appear here; `ui/input.rs` is responsible for translating raw key
//! events into these.

use libscanmem::session::ScanExpr;
use libscanmem::value::Value;

use crate::app::Direction;

/// One user-triggered action.
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    /// Attach to a target process by pid, replacing any currently attached session.
    Attach(u32),
    /// Detach from the current target, if any, resuming its execution.
    Detach,
    /// Run a first or narrowing scan synchronously; `Session::scan` picks based on whether
    /// matches are already recorded. Unlike `RunScan`, this blocks the caller until the scan
    /// completes — used for direct/scripted invocation (and in tests) where that's fine; the
    /// interactive TUI always goes through `RunScan` instead so a slow scan can't freeze it.
    Scan(ScanExpr),
    /// Reseed the match set with every byte of every considered region, on a background thread —
    /// same non-blocking reasoning as `RunScan`.
    Snapshot,
    /// Checks whether a background scan/snapshot started by `RunScan`/`Snapshot` has finished;
    /// if so, restores the session and reports its outcome. A no-op if none is running or it
    /// hasn't finished yet — the event loop dispatches this every iteration, not just after
    /// starting one.
    PollScan,
    /// Discard the current match set without detaching.
    ResetScan,
    /// Write a value into the target's address space.
    Write { address: usize, value: Value },
    /// Load a window of session memory centered on `address` into the Hex View buffer and switch
    /// focus to it.
    FocusHexView(usize),
    /// Move the Hex View cursor by this many bytes, clamped to the loaded buffer.
    MoveHexCursor(isize),
    /// Set the Hex View's in-progress byte-edit input at the cursor, replacing any previous one.
    SetHexEditInput(String),
    /// Parse the in-progress byte edit and write it to the cursor's address, updating the loaded
    /// buffer on success.
    CommitHexEdit,
    /// Record a new cheat-list entry.
    #[cfg(feature = "cheat-list")]
    AddCheat {
        address: usize,
        description: String,
        value: Value,
    },
    /// Remove a cheat-list entry by index.
    #[cfg(feature = "cheat-list")]
    RemoveCheat(usize),
    /// Flip a cheat-list entry's freeze flag.
    #[cfg(feature = "cheat-list")]
    ToggleFreeze(usize),
    /// Write a new value for an existing cheat-list entry, recording it as the entry's stored
    /// value on success.
    #[cfg(feature = "cheat-list")]
    EditCheatValue { index: usize, value: Value },
    /// Enter inline-edit mode for a cheat-list entry's value, prefilling the edit input with its
    /// current value.
    #[cfg(feature = "cheat-list")]
    BeginEditCheatValue(usize),
    /// Set the in-progress cheat-value edit input, replacing any previous one.
    #[cfg(feature = "cheat-list")]
    SetCheatValueInput(String),
    /// Parse the in-progress cheat-value edit input against the entry's current value's width
    /// and apply it via [`Msg::EditCheatValue`], then exit edit mode.
    #[cfg(feature = "cheat-list")]
    ConfirmCheatValueEdit,
    /// Save the cheat list, using the last load/save path if known, otherwise opening a path
    /// prompt.
    #[cfg(feature = "cheat-list")]
    SaveCheatList,
    /// Open a path prompt to load a cheat list, replacing the current one on success.
    #[cfg(feature = "cheat-list")]
    LoadCheatList,
    /// Set the in-progress save/load path prompt input, replacing any previous one.
    #[cfg(feature = "cheat-list")]
    SetPathInput(String),
    /// Perform the save or load the path prompt was opened for, using its current input.
    #[cfg(feature = "cheat-list")]
    ConfirmPathPrompt,
    /// Periodic tick from the event loop (not user-triggered): rewrites every frozen cheat-list
    /// entry's stored value back to its address.
    #[cfg(feature = "cheat-list")]
    Tick,
    /// Rescan `/proc` for the current list of running processes.
    RefreshProcessList,
    /// Set the Process Picker's incremental filter query, replacing any previous one.
    FilterProcesses(String),
    /// Cycle the Scan Panel's selected data type forward.
    CycleScanDataType,
    /// Cycle the Scan Panel's selected match type forward.
    CycleScanMatchType,
    /// Set the Scan Panel's free-text value/range input, replacing any previous one.
    SetScanInput(String),
    /// Build a `ScanExpr` from the Scan Panel's current data type/match type/input and run it on
    /// a background thread so the UI keeps rendering — a scan can easily take longer than a
    /// frame; `Session::scan` picks first vs. narrowing based on whether matches are already
    /// recorded. Poll for the result with `PollScan`.
    RunScan,
    /// Cycle the Match View's sort column.
    CycleMatchSort,
    /// Set the Match View's incremental filter query, replacing any previous one.
    FilterMatches(String),
    /// Enter or exit the focused panel's text-editing mode: the Process Picker's filter, the
    /// Scan Panel's value/range input, or the Match View's filter (`/` to enter, `Enter` to
    /// confirm-and-exit while active, `Dismiss` to cancel-and-exit clearing the field).
    ToggleSearch,
    /// Move the focused panel's selection forward.
    SelectNext,
    /// Move the focused panel's selection backward.
    SelectPrev,
    /// Cycle the focused panel forward.
    FocusNext,
    /// Cycle the focused panel backward.
    FocusPrev,
    /// Move focus to whichever panel sits in `Direction` on the grid `ui/layout.rs` renders
    /// (`Ctrl+<Arrow>`), a no-op if there is none. Unlike `FocusNext`/`FocusPrev`, this only
    /// applies while the grid is showing every panel — it's meaningless while one is expanded,
    /// since none of the others are on screen to move to.
    FocusDirection(Direction),
    /// Toggle between the multi-panel grid and showing only the focused panel fullscreen —
    /// `Ctrl+E`, mirroring `bottom`'s widget-expand binding.
    ToggleExpand,
    /// Toggle the help overlay for the current focus.
    ShowHelp,
    /// Close a popup/overlay/prompt without acting.
    Dismiss,
    /// Request application exit.
    Quit,
}
