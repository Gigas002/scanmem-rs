//! `Command` — one parsed REPL/scripted user action; grammar lives in [`parser`], plain-text
//! rendering lives in [`formatter`]. Dispatch against a `Session` lives in `app::repl`.

pub mod formatter;
pub mod parser;

use std::ops::Range;

use libscanmem::session::{ScanExpr, SessionOption};
use libscanmem::value::Value;

/// One parsed REPL/scripted user action.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `pid <n>` / `attach <n>`.
    Attach(u32),
    /// `scan <type> <match> [value...]`.
    Scan(ScanExpr),
    /// `snapshot`.
    Snapshot,
    /// `list [start end]` — `None` prints every match, `Some` prints indices `[start, end)`.
    List(Option<Range<usize>>),
    /// `dump <addr> <len>`.
    Dump { address: usize, len: usize },
    /// `write <addr> <type> <value>`.
    Write { address: usize, value: Value },
    /// `delete <selector>` — a raw `libscanmem::sets` index-set expression; resolved against the
    /// live match count at dispatch time, since that bound isn't known while parsing.
    Delete(String),
    /// `option <key> <value>`.
    SetOption(SessionOption),
    /// `reset`.
    Reset,
    /// `help`.
    Help,
    /// `quit` / `exit`.
    Quit,
}

#[cfg(test)]
mod tests;
