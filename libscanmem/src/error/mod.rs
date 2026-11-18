//! Structured `ScanmemError` — replaces `common.c`/`show_message.c` error handling.

use thiserror::Error;

/// `Result` alias using [`ScanmemError`], for every fallible `libscanmem` operation.
pub type Result<T> = std::result::Result<T, ScanmemError>;

/// Every way a `Session` operation can fail — replaces the mixed bool-return/`show_error`
/// reporting in `common.c`/`show_message.c` with a typed, matchable error.
#[derive(Debug, Error)]
pub enum ScanmemError {
    #[error("no process is attached")]
    NotAttached,
    /// The kernel refused access to the process: another user's process, or Yama's
    /// `ptrace_scope`, which by default only lets a process trace its own descendants.
    #[error(
        "permission denied: tracing this process isn't allowed (see \
         /proc/sys/kernel/yama/ptrace_scope; run as root or with CAP_SYS_PTRACE)"
    )]
    PermissionDenied,
    #[error("no such process")]
    NoSuchProcess,
    #[error("the target process has exited")]
    ProcessExited,
    #[error(transparent)]
    Io(std::io::Error),
    #[error("invalid scan expression: {0}")]
    InvalidExpr(String),
    /// Raw `errno` from a failed `ptrace(2)` call in the unsafe core.
    #[error("ptrace failed: {}", std::io::Error::from_raw_os_error(*.0))]
    Ptrace(i32),
}

impl From<std::io::Error> for ScanmemError {
    /// `ESRCH` from a `/proc` file or `waitpid` means the process is gone (or a zombie).
    fn from(err: std::io::Error) -> Self {
        if err.raw_os_error() == Some(libc::ESRCH) {
            Self::ProcessExited
        } else {
            Self::Io(err)
        }
    }
}

impl ScanmemError {
    /// Classifies a failure to open a process's `/proc` files: access denied and a missing
    /// process get their own variants, so callers can tell them apart and the message says why.
    pub(crate) fn from_proc_open(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            std::io::ErrorKind::NotFound => Self::NoSuchProcess,
            _ => err.into(),
        }
    }
}

#[cfg(test)]
mod tests;
