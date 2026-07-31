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
    AddCheat {
        address: usize,
        description: String,
        value: Value,
    },
    /// Remove a cheat-list entry by index.
    RemoveCheat(usize),
    /// Flip a cheat-list entry's freeze flag.
    ToggleFreeze(usize),
    /// Write a new value for an existing cheat-list entry, recording it as the entry's stored
    /// value on success.
    EditCheatValue { index: usize, value: Value },
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
