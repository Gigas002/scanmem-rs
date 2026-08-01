//! `Msg` — one user-triggered action, the single input to [`crate::app::update`]. No
//! `ratatui`/`crossterm` types appear here; `ui/input.rs` is responsible for translating raw key
//! events into these.

use libscanmem::session::ScanExpr;
use libscanmem::value::Value;

/// One user-triggered action.
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    /// Attach to a target process by pid, replacing any currently attached session.
    Attach(u32),
    /// Detach from the current target, if any, resuming its execution.
    Detach,
    /// Run a first or narrowing scan; `Session::scan` picks based on whether matches are
    /// already recorded.
    Scan(ScanExpr),
    /// Reseed the match set with every byte of every considered region.
    Snapshot,
    /// Discard the current match set without detaching.
    ResetScan,
    /// Write a value into the target's address space.
    Write { address: usize, value: Value },
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
    /// Build a `ScanExpr` from the Scan Panel's current data type/match type/input and run it;
    /// `Session::scan` picks first vs. narrowing based on whether matches are already recorded.
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
    /// Toggle the help overlay for the current focus.
    ShowHelp,
    /// Close a popup/overlay/prompt without acting.
    Dismiss,
    /// Request application exit.
    Quit,
}
