//! Plain-text formatting for command results — no ANSI color, no `Session` access.

use libscanmem::session::{MatchView, ScanStats};

/// Formats the outcome of a `scan`/`snapshot` command.
pub fn scan_stats(stats: ScanStats) -> String {
    format!("{} match(es)", stats.matches)
}

/// Formats a table of `(index, match)` pairs, one per line.
pub fn match_table(matches: impl Iterator<Item = (usize, MatchView)>) -> String {
    let mut rows: Vec<String> = matches
        .map(|(index, view)| format!("[{index}] {:#x} = {}", view.address, view.old_value))
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
                "{:#010x}  {:<47}  {ascii}",
                address + row * 16,
                hex.join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Formats how many matches a `delete` command removed.
pub fn deleted(count: usize) -> String {
    format!("deleted {count} match(es)")
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
