//! Process attach/detach and safe `/proc/<pid>/mem` read/write — replaces the attach/detach and
//! bulk-transfer paths of `ptrace.c` (its text-command orchestration stays out of this crate).

mod ptrace;

use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;

use std::cell::Cell;

use rustix::process::Pid;

use crate::error::ScanmemError;
use crate::maps::{Region, parse_maps};

/// An attached target process: its pid and the `/proc/<pid>/mem` file opened for read/write.
///
/// "Attached" doesn't mean traced. The process is a ptrace tracee only between [`Self::stop`]
/// and [`Self::resume`] (a scan). A tracee stops on every signal it receives until its tracer
/// waits for it and lets the signal through, so staying traced for a whole session would freeze
/// the target on its first SIGCHLD, SIGALRM or Ctrl-C. Reading and writing memory needs no
/// tracing: opening `/proc/<pid>/mem` already passed the ptrace permission check.
#[derive(Debug)]
pub struct Process {
    pid: Pid,
    mem: File,
    /// Set between [`Self::stop`] and [`Self::resume`]: the signal to hand back on resume.
    stopped: Cell<Option<i32>>,
}

impl Process {
    /// Opens `/proc/<pid>/mem` for read/write — replaces `sm_attach`. The kernel allows that only
    /// to a process allowed to ptrace `pid` (including Yama's `ptrace_scope`), so this is the
    /// permission check; the target isn't stopped or traced. Callers that need a consistent
    /// snapshot (a scan) bracket it with [`Self::stop`]/[`Self::resume`].
    pub fn attach(pid: Pid) -> Result<Self, ScanmemError> {
        let mem = OpenOptions::new()
            .read(true)
            .write(true)
            .open(mem_path(pid))
            .map_err(ScanmemError::from_proc_open)?;

        Ok(Self {
            pid,
            mem,
            stopped: Cell::new(None),
        })
    }

    /// The pid of the attached process.
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Stops the target so a scan reads a consistent snapshot: `PTRACE_SEIZE` and
    /// `PTRACE_INTERRUPT`, which stop it without sending it a signal. Pair with [`Self::resume`]
    /// once the scan is done, on the same thread: ptrace ties a tracee to the tracer thread.
    pub fn stop(&self) -> Result<(), ScanmemError> {
        if self.stopped.get().is_some() {
            return Ok(());
        }
        ptrace::seize(self.pid)?;
        // A failure here means the target exited in between; there is nothing to detach.
        ptrace::interrupt(self.pid)?;
        match ptrace::wait_stop(self.pid)? {
            ptrace::Stop::Exited => Err(ScanmemError::ProcessExited),
            ptrace::Stop::Interrupted => {
                self.stopped.set(Some(0));
                Ok(())
            }
            ptrace::Stop::Signal(signal) => {
                self.stopped.set(Some(signal));
                Ok(())
            }
        }
    }

    /// Resumes the target after [`Self::stop`] and stops tracing it, delivering any signal that
    /// arrived while it was stopping. A no-op when it isn't stopped.
    pub fn resume(&self) -> Result<(), ScanmemError> {
        match self.stopped.take() {
            Some(signal) => ptrace::detach(self.pid, signal),
            None => Ok(()),
        }
    }

    /// Detaches — replaces `sm_detach`. Resumes the target if a scan left it stopped, and
    /// closes `/proc/<pid>/mem`. Dropping the `Process` does the same.
    pub fn detach(self) -> Result<(), ScanmemError> {
        self.resume()
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

impl Drop for Process {
    /// Never leave the target stopped or traced, whichever way the session ends.
    fn drop(&mut self) {
        let _ = self.resume();
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
