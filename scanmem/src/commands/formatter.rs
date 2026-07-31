//! Human-readable formatting for command results — ANSI-colored when stdout is a tty and
//! `NO_COLOR` is unset (plain text otherwise); no `Session` access.

use std::fmt::Display;
use std::io::IsTerminal;

use libscanmem::session::{MatchView, ScanStats};

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

/// Whether ANSI color codes should be emitted for the current process's stdout.
fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

/// Wraps `text` in `code`/[`RESET`] when [`color_enabled`], otherwise returns it unchanged.
fn colorize(code: &str, text: &str) -> String {
    if color_enabled() {
        format!("{code}{text}{RESET}")
    } else {
        text.to_owned()
    }
}

/// Formats a failed command or parse error uniformly, e.g. `error: no process is attached`.
pub fn error(err: impl Display) -> String {
    colorize(RED, &format!("error: {err}"))
}

/// Whether `text` (as produced by [`error`] or elsewhere) represents an error result.
pub fn is_error(text: &str) -> bool {
    text.contains("error: ")
}

/// Formats a plain confirmation message, e.g. `session reset`.
pub fn info(text: &str) -> String {
    colorize(GREEN, text)
}

/// Formats the outcome of a `scan`/`snapshot` command.
pub fn scan_stats(stats: ScanStats) -> String {
    colorize(GREEN, &format!("{} match(es)", stats.matches))
}

/// Formats a table of `(index, match)` pairs, one per line.
pub fn match_table(matches: impl Iterator<Item = (usize, MatchView)>) -> String {
    let mut rows: Vec<String> = matches
        .map(|(index, view)| {
            format!(
                "[{}] {} = {}",
                colorize(YELLOW, &index.to_string()),
                colorize(CYAN, &format!("{:#x}", view.address)),
                view.old_value
            )
        })
        .collect();
    if rows.is_empty() {
        rows.push("no matches".to_owned());
    }
    rows.join("\n")
}

/// Formats `len` bytes read from `address` as hex + ASCII, 16 bytes per line.
pub fn dump(address: usize, bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "no bytes read".to_owned();
    }

    bytes
        .chunks(16)
        .enumerate()
        .map(|(row, chunk)| {
            let hex: Vec<String> = chunk.iter().map(|byte| format!("{byte:02x}")).collect();
            let ascii: String = chunk
                .iter()
                .map(|&byte| {
                    if byte.is_ascii_graphic() {
                        byte as char
                    } else {
                        '.'
                    }
                })
                .collect();
            format!(
                "{}  {:<47}  {ascii}",
                colorize(CYAN, &format!("{:#010x}", address + row * 16)),
                hex.join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Formats how many matches a `delete` command removed.
pub fn deleted(count: usize) -> String {
    colorize(GREEN, &format!("deleted {count} match(es)"))
}

/// Static help text listing every REPL verb.
pub fn help() -> String {
    [
        "pid <n> | attach <n>          attach to a running process",
        "scan <type> <match> [value]   run or narrow a scan",
        "snapshot                      record every byte as a candidate match",
        "list [start end]              print recorded matches",
        "dump <addr> <len>             read and print target memory",
        "write <addr> <type> <value>   write target memory",
        "delete <selector>             remove matches by index set, e.g. 1,3-5",
        "option <key> <value>          set endianness/region option",
        "reset                         drop the current session",
        "help                          show this text",
        "quit | exit                   leave the REPL",
    ]
    .join("\n")
}
