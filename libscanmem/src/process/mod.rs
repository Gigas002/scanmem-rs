//! Process attach/detach and safe `/proc/<pid>/mem` read/write — replaces the attach/detach and
//! bulk-transfer paths of `ptrace.c` (its text-command orchestration stays out of this crate).

mod ptrace;

use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;

use rustix::process::{Pid, Signal, WaitOptions, kill_process, waitpid};

use crate::error::ScanmemError;
use crate::maps::{Region, parse_maps};

/// An attached target process: its pid and the `/proc/<pid>/mem` file opened for read/write.
#[derive(Debug)]
pub struct Process {
    pid: Pid,
    mem: File,
}

impl Process {
    /// Attaches to `pid` via `PTRACE_ATTACH`, waits for the resulting stop, then immediately
    /// resumes it — `PTRACE_ATTACH` implicitly stops the tracee, but staying attached must not
    /// otherwise pause the target for as long as it's attached (that would freeze the game just
    /// for having a session open). Opens `/proc/<pid>/mem` for read/write before returning —
    /// replaces `sm_attach`. Callers that need a consistent snapshot (a scan) should bracket it
    /// with [`Self::stop`]/[`Self::resume`].
    pub fn attach(pid: Pid) -> Result<Self, ScanmemError> {
        ptrace::attach(pid)?;
        wait_for_stop(pid)?;
        ptrace::cont(pid)?;

        let mem = OpenOptions::new()
            .read(true)
            .write(true)
            .open(mem_path(pid))?;

        Ok(Self { pid, mem })
    }

    /// The pid of the attached process.
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Stops the tracee (`SIGSTOP`, observed via the ptrace relationship [`Self::attach`]
    /// established) so a scan can read a consistent memory snapshot. Pair with [`Self::resume`]
    /// once the scan is done — the target must not stay paused any longer than that.
    pub fn stop(&self) -> Result<(), ScanmemError> {
        kill_process(self.pid, Signal::STOP).map_err(std::io::Error::from)?;
        wait_for_stop(self.pid)
    }

    /// Resumes the tracee after [`Self::stop`].
    pub fn resume(&self) -> Result<(), ScanmemError> {
        ptrace::cont(self.pid)
    }

    /// Detaches, resuming the target's execution — replaces `sm_detach`. `PTRACE_DETACH` is only
    /// valid while the tracee is in a ptrace-stop, so this stops it first (a harmless no-op if a
    /// scan already left it stopped — see [`Self::stop`]); the target keeps running the rest of
    /// the time it's attached, so this is normally the one that briefly pauses it. `/proc/<pid>/mem`
    /// is closed (by dropping `self.mem`) before the underlying `PTRACE_DETACH` call, matching the
    /// C code's close-before-detach ordering.
    pub fn detach(self) -> Result<(), ScanmemError> {
        self.stop()?;
        drop(self.mem);
        ptrace::detach(self.pid)
    }

    /// Reads `len` bytes starting at `address` in the target's address space.
    pub fn read(&self, address: usize, len: usize) -> Result<Vec<u8>, ScanmemError> {
        let mut buf = vec![0u8; len];
        self.mem.read_exact_at(&mut buf, address as u64)?;
        Ok(buf)
    }

    /// Writes `data` starting at `address` in the target's address space.
    pub fn write(&self, address: usize, data: &[u8]) -> Result<(), ScanmemError> {
        self.mem.write_all_at(data, address as u64)?;
        Ok(())
    }

    /// Current memory regions (`/proc/<pid>/maps`), for scan/read orchestration.
    pub fn regions(&self) -> Result<Vec<Region>, ScanmemError> {
        let text = std::fs::read_to_string(maps_path(self.pid))?;
        Ok(parse_maps(&text))
    }
}

/// Blocks until `pid` reports a ptrace-stop (per `ptrace(2)`, both the implicit stop from
/// `PTRACE_ATTACH` and a `SIGSTOP` delivered to an already-attached tracee count), used by both
/// [`Process::attach`] and [`Process::stop`].
fn wait_for_stop(pid: Pid) -> Result<(), ScanmemError> {
    let (_, status) = waitpid(Some(pid), WaitOptions::empty())
        .map_err(std::io::Error::from)?
        .ok_or(ScanmemError::ProcessExited)?;
    if !status.stopped() {
        return Err(ScanmemError::ProcessExited);
    }
    Ok(())
}

fn mem_path(pid: Pid) -> String {
    format!("/proc/{}/mem", pid.as_raw_pid())
}

fn maps_path(pid: Pid) -> String {
    format!("/proc/{}/maps", pid.as_raw_pid())
}

#[cfg(test)]
mod tests;
