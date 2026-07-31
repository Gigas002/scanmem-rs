use std::io;

use super::*;

#[test]
fn display_messages_are_stable() {
    assert_eq!(
        ScanmemError::NotAttached.to_string(),
        "no process is attached"
    );
    assert_eq!(
        ScanmemError::PermissionDenied.to_string(),
        "permission denied"
    );
    assert_eq!(
        ScanmemError::ProcessExited.to_string(),
        "the target process has exited"
    );
    assert_eq!(
        ScanmemError::InvalidExpr("bad expr".to_owned()).to_string(),
        "invalid scan expression: bad expr"
    );
    assert_eq!(
        ScanmemError::Ptrace(3).to_string(),
        "ptrace failed (errno 3)"
    );
}

#[test]
fn io_error_converts_via_from_and_keeps_its_message() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    let err: ScanmemError = io_err.into();
    assert_eq!(err.to_string(), "denied");
}
