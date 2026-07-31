use super::*;

#[test]
fn new_flag_is_not_requested() {
    let flag = StopFlag::new();
    assert!(!flag.requested());
}

#[test]
fn request_sets_the_flag() {
    let flag = StopFlag::new();
    flag.request();
    assert!(flag.requested());
}

#[test]
fn reset_clears_the_flag() {
    let flag = StopFlag::new();
    flag.request();
    flag.reset();
    assert!(!flag.requested());
}

#[test]
fn clones_share_the_same_underlying_flag() {
    let flag = StopFlag::new();
    let clone = flag.clone();
    flag.request();
    assert!(clone.requested());
}

#[test]
#[cfg(feature = "signals")]
fn register_sigint_returns_a_flag_that_is_not_requested_yet() {
    let flag = StopFlag::register_sigint().expect("failed to register SIGINT handler");
    assert!(!flag.requested());
}
