//! Raw `ptrace(2)` and `waitpid(2)` calls — the only unsafe code in this crate, limited to
//! seizing, interrupting and detaching a tracee and waiting for its stop. Reading and writing
//! memory is safe Rust layered on top in [`super`].
//!
//! `libc`'s `ptrace` request constants and call signature are Linux-specific (this crate's only
//! supported platform, see workspace docs), so the real implementation is gated on
//! `target_os = "linux"`; other platforms get a stub that reports `ENOSYS` instead of failing to
//! compile, so the rest of the crate stays buildable for contributors on other host platforms.

use rustix::process::Pid;

use crate::error::ScanmemError;

/// Seizes `pid` (`PTRACE_SEIZE`): becomes its tracer without stopping it or sending it a
/// signal. Pair with [`interrupt`] to stop it.
#[cfg(target_os = "linux")]
pub(super) fn seize(pid: Pid) -> Result<(), ScanmemError> {
    match request(libc::PTRACE_SEIZE, pid, 0) {
        // A zombie can't be seized: EPERM, though access was checked when attaching.
        Err(ScanmemError::Ptrace(libc::EPERM)) if is_zombie(pid) => {
            Err(ScanmemError::ProcessExited)
        }
        Err(ScanmemError::Ptrace(libc::EPERM)) => Err(ScanmemError::PermissionDenied),
        result => result,
    }
}

/// Whether `pid` has exited and not been reaped yet (or is gone altogether).
#[cfg(target_os = "linux")]
fn is_zombie(pid: Pid) -> bool {
    match std::fs::read_to_string(format!("/proc/{}/stat", pid.as_raw_pid())) {
        // The state follows the parenthesised command name, which may contain anything.
        Ok(stat) => stat
            .rsplit_once(')')
            .is_some_and(|(_, rest)| rest.trim_start().starts_with(['Z', 'X'])),
        Err(_) => true,
    }
}

/// Stops a seized tracee (`PTRACE_INTERRUPT`); the stop is reported through [`wait_stop`].
#[cfg(target_os = "linux")]
pub(super) fn interrupt(pid: Pid) -> Result<(), ScanmemError> {
    request(libc::PTRACE_INTERRUPT, pid, 0)
}

/// Detaches from a stopped tracee, resuming it and delivering `signal` (0 for none): the
/// signal that stopped it, if one arrived while it was being stopped.
#[cfg(target_os = "linux")]
pub(super) fn detach(pid: Pid, signal: i32) -> Result<(), ScanmemError> {
    request(libc::PTRACE_DETACH, pid, signal)
}

/// How a tracee reported its stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stop {
    /// The `PTRACE_INTERRUPT` stop, or a group-stop: no signal to hand back.
    Interrupted,
    /// A signal arrived first, and the tracee stopped to report it; it must be delivered on
    /// detach, or the target never sees it.
    Signal(i32),
    /// The tracee exited or was killed.
    Exited,
}

/// Waits for the next stop of a seized tracee.
#[cfg(target_os = "linux")]
pub(super) fn wait_stop(pid: Pid) -> Result<Stop, ScanmemError> {
    let mut status = 0;
    loop {
        // SAFETY: `status` is a valid, writable c_int for the duration of the call.
        let ret = unsafe { libc::waitpid(pid.as_raw_pid(), &mut status, libc::__WALL) };
        if ret != -1 {
            break;
        }
        let err = std::io::Error::last_os_error();
        if err.kind() != std::io::ErrorKind::Interrupted {
            return Err(err.into());
        }
    }
    if !libc::WIFSTOPPED(status) {
        return Ok(Stop::Exited);
    }
    if status >> 16 == libc::PTRACE_EVENT_STOP {
        Ok(Stop::Interrupted)
    } else {
        Ok(Stop::Signal(libc::WSTOPSIG(status)))
    }
}

/// Issues a `ptrace(2)` request against `pid` with a null `addr` and the given `data`.
#[cfg(target_os = "linux")]
fn request(request: libc::c_uint, pid: Pid, data: i32) -> Result<(), ScanmemError> {
    // SAFETY: SEIZE, INTERRUPT and DETACH take no `addr`, and `data` as a plain integer
    // (options, or the signal to deliver), never as a pointer (ptrace(2)).
    let ret = unsafe {
        libc::ptrace(
            request,
            pid.as_raw_pid(),
            std::ptr::null_mut::<libc::c_void>(),
            data as usize as *mut libc::c_void,
        )
    };
    if ret == -1 {
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
        if errno == libc::ESRCH {
            return Err(ScanmemError::ProcessExited);
        }
        return Err(ScanmemError::Ptrace(errno));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn seize(_pid: Pid) -> Result<(), ScanmemError> {
    Err(ScanmemError::Ptrace(libc::ENOSYS))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn interrupt(_pid: Pid) -> Result<(), ScanmemError> {
    Err(ScanmemError::Ptrace(libc::ENOSYS))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn detach(_pid: Pid, _signal: i32) -> Result<(), ScanmemError> {
    Err(ScanmemError::Ptrace(libc::ENOSYS))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn wait_stop(_pid: Pid) -> Result<Stop, ScanmemError> {
    Err(ScanmemError::Ptrace(libc::ENOSYS))
}
