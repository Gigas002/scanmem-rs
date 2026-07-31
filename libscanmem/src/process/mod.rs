//! Process attach/detach and safe `/proc/<pid>/mem` read/write — replaces the attach/detach and
//! bulk-transfer paths of `ptrace.c` (its text-command orchestration stays out of this crate).

mod ptrace;

use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;

use rustix::process::{Pid, WaitOptions, waitpid};

use crate::error::ScanmemError;
use crate::maps::{Region, parse_maps};

/// An attached target process: its pid and the `/proc/<pid>/mem` file opened for read/write.
#[derive(Debug)]
pub struct Process {
    pid: Pid,
    mem: File,
}

impl Process {
    /// Attaches to `pid` via `PTRACE_ATTACH`, waits for the resulting stop, then opens
    /// `/proc/<pid>/mem` for read/write — replaces `sm_attach`.
    pub fn attach(pid: Pid) -> Result<Self, ScanmemError> {
        ptrace::attach(pid)?;

        let (_, status) = waitpid(Some(pid), WaitOptions::empty())
            .map_err(std::io::Error::from)?
            .ok_or(ScanmemError::ProcessExited)?;
        if !status.stopped() {
            return Err(ScanmemError::ProcessExited);
        }

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

    /// Detaches, resuming the target's execution — replaces `sm_detach`. `/proc/<pid>/mem` is
    /// closed (by dropping `self.mem`) before the underlying `PTRACE_DETACH` call, matching the
    /// C code's close-before-detach ordering.
    pub fn detach(self) -> Result<(), ScanmemError> {
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

fn mem_path(pid: Pid) -> String {
    format!("/proc/{}/mem", pid.as_raw_pid())
}

fn maps_path(pid: Pid) -> String {
    format!("/proc/{}/maps", pid.as_raw_pid())
}

#[cfg(test)]
mod tests;
