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
    #[error("permission denied")]
    PermissionDenied,
    #[error("the target process has exited")]
    ProcessExited,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid scan expression: {0}")]
    InvalidExpr(String),
    /// Raw `errno` from a failed `ptrace(2)` call in the unsafe core.
    #[error("ptrace failed (errno {0})")]
    Ptrace(i32),
}

#[cfg(test)]
mod tests;
