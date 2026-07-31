//! Reads `/proc` for the list of running processes the Process Picker panel shows.

use std::fs;

use crate::app::state::ProcessEntry;

/// Lists every process currently visible under `/proc`, sorted by pid. An entry whose `comm`
/// file cannot be read (permission denied, or the process exited between listing and reading)
/// gets the placeholder name `"?"` rather than being dropped, so a stale/inaccessible pid still
/// shows up and can be attempted.
pub fn list_processes() -> Vec<ProcessEntry> {
    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(err) => {
            tracing::warn!("failed to read /proc: {err}");
            return Vec::new();
        }
    };

    let mut processes: Vec<ProcessEntry> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let name = fs::read_to_string(entry.path().join("comm"))
                .map(|comm| comm.trim().to_owned())
                .unwrap_or_else(|_| "?".to_owned());
            Some(ProcessEntry { pid, name })
        })
        .collect();

    processes.sort_by_key(|process| process.pid);
    processes
}
