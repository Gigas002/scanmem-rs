use std::io;

use super::*;

#[test]
fn display_messages_are_stable() {
    assert_eq!(
        ScanmemError::NotAttached.to_string(),
        "no process is attached"
    );
    assert!(
        ScanmemError::PermissionDenied
            .to_string()
            .contains("ptrace_scope")
    );
    assert_eq!(ScanmemError::NoSuchProcess.to_string(), "no such process");
    assert_eq!(
        ScanmemError::ProcessExited.to_string(),
        "the target process has exited"
    );
    assert_eq!(
        ScanmemError::InvalidExpr("bad expr".to_owned()).to_string(),
        "invalid scan expression: bad expr"
    );
    assert_eq!(
        ScanmemError::Ptrace(libc::ESRCH).to_string(),
        "ptrace failed: No such process (os error 3)"
    );
}

#[test]
fn esrch_from_io_means_the_process_exited() {
    let err: ScanmemError = io::Error::from_raw_os_error(libc::ESRCH).into();
    assert!(matches!(err, ScanmemError::ProcessExited));
}

#[test]
fn proc_open_errors_name_their_cause() {
    let denied = ScanmemError::from_proc_open(io::Error::from(io::ErrorKind::PermissionDenied));
    assert!(matches!(denied, ScanmemError::PermissionDenied));
    let missing = ScanmemError::from_proc_open(io::Error::from(io::ErrorKind::NotFound));
    assert!(matches!(missing, ScanmemError::NoSuchProcess));
    let other = ScanmemError::from_proc_open(io::Error::from(io::ErrorKind::InvalidData));
    assert!(matches!(other, ScanmemError::Io(_)));
}

#[test]
fn io_error_converts_via_from_and_keeps_its_message() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    let err: ScanmemError = io_err.into();
    assert_eq!(err.to_string(), "denied");
}
