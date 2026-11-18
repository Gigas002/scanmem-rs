use rustix::process::Pid;

use super::*;

#[test]
fn attach_to_a_nonexistent_pid_fails() {
    // `/proc/<pid>/mem` doesn't exist for a pid that doesn't exist, so this needs no
    // privilege and touches no real process.
    let pid = Pid::from_raw(i32::MAX - 1).expect("pid literal is non-zero");
    assert!(Process::attach(pid).is_err());
}

#[test]
fn mem_and_maps_paths_are_built_from_the_pid() {
    let pid = Pid::from_raw(1234).expect("pid literal is non-zero");
    assert_eq!(mem_path(pid), "/proc/1234/mem");
    assert_eq!(maps_path(pid), "/proc/1234/maps");
}
