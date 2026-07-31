//! Raw `ptrace(2)` calls — the only unsafe code in this crate, limited to `PTRACE_ATTACH`/
//! `PTRACE_DETACH`. Everything else (waiting for the stop, reading/writing memory) is safe Rust
//! layered on top in [`super`].
//!
//! `libc`'s `ptrace` request constants and call signature are Linux-specific (this crate's only
//! supported platform, see workspace docs), so the real implementation is gated on
//! `target_os = "linux"`; other platforms get a stub that reports `ENOSYS` instead of failing to
//! compile, so the rest of the crate stays buildable for contributors on other host platforms.

use rustix::process::Pid;

use crate::error::ScanmemError;

/// Attaches to `pid`, per ptrace(2): this sends `SIGSTOP` to the tracee but does not itself wait
/// for it to stop — the caller must still `waitpid` for the resulting stop.
#[cfg(target_os = "linux")]
pub(super) fn attach(pid: Pid) -> Result<(), ScanmemError> {
    request(libc::PTRACE_ATTACH, pid)
}

/// Detaches from `pid`, resuming its execution. Per ptrace(2), `PTRACE_DETACH`'s `addr`/`data`
/// arguments are ignored on Linux.
#[cfg(target_os = "linux")]
pub(super) fn detach(pid: Pid) -> Result<(), ScanmemError> {
    request(libc::PTRACE_DETACH, pid)
}

/// Issues a payload-less `ptrace(2)` request (`addr`/`data` both null) against `pid`.
#[cfg(target_os = "linux")]
fn request(request: libc::c_uint, pid: Pid) -> Result<(), ScanmemError> {
    // SAFETY: `PTRACE_ATTACH`/`PTRACE_DETACH` take no `addr`/`data` payload on Linux (ptrace(2)),
    // so passing null pointers for both is the documented no-op form of this call.
    let ret = unsafe {
        libc::ptrace(
            request,
            pid.as_raw_pid(),
            std::ptr::null_mut::<libc::c_void>(),
            std::ptr::null_mut::<libc::c_void>(),
        )
    };
    if ret == -1 {
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
        return Err(ScanmemError::Ptrace(errno));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn attach(_pid: Pid) -> Result<(), ScanmemError> {
    Err(ScanmemError::Ptrace(libc::ENOSYS))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn detach(_pid: Pid) -> Result<(), ScanmemError> {
    Err(ScanmemError::Ptrace(libc::ENOSYS))
}
