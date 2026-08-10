//! Reads `/proc` for the list of running processes the Process Picker panel shows.

use std::fs;
use std::path::Path;

use crate::app::state::ProcessEntry;

/// Lists every process currently visible under `/proc`, sorted by pid. An entry whose name
/// cannot be read (permission denied, or the process exited between listing and reading) gets
/// the placeholder name `"?"` rather than being dropped, so a stale/inaccessible pid still shows
/// up and can be attempted.
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
            let name = process_name(&entry.path());
            Some(ProcessEntry { pid, name })
        })
        .collect();

    processes.sort_by_key(|process| process.pid);
    processes
}

/// The display/filter name for the process at `proc_dir` (e.g. `/proc/1234`). Prefers the
/// basename of `argv[0]` from `cmdline`, since `comm` is hard-truncated by the kernel to 15
/// characters (`TASK_COMM_LEN - 1`) and cuts off long names (e.g. `Hollow Knight Silksong`
/// becomes `Hollow Knight S`), making them unsearchable by their full title. Falls back to
/// `comm` for kernel threads, whose `cmdline` is empty.
fn process_name(proc_dir: &Path) -> String {
    let argv0 = fs::read_to_string(proc_dir.join("cmdline")).ok().and_then(
        |cmdline| -> Option<String> {
            let arg = cmdline.split('\0').next().filter(|arg| !arg.is_empty())?;
            Some(
                Path::new(arg)
                    .file_name()
                    .map_or_else(|| arg.to_owned(), |name| name.to_string_lossy().into_owned()),
            )
        },
    );
    if let Some(name) = argv0 {
        return name;
    }

    fs::read_to_string(proc_dir.join("comm"))
        .map(|comm| comm.trim().to_owned())
        .unwrap_or_else(|_| "?".to_owned())
}
