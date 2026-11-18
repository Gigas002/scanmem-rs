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

#[test]
fn new_progress_is_zero() {
    let progress = ScanProgress::new();
    assert_eq!(progress.get(), (0, 0));
}

#[test]
fn reset_sets_the_total_and_zeroes_done() {
    let progress = ScanProgress::new();
    progress.add(10);
    progress.reset(100);
    assert_eq!(progress.get(), (0, 100));
}

#[test]
fn add_accumulates_into_done() {
    let progress = ScanProgress::new();
    progress.reset(100);
    progress.add(30);
    progress.add(20);
    assert_eq!(progress.get(), (50, 100));
}

#[test]
fn clones_share_the_same_underlying_counters() {
    let progress = ScanProgress::new();
    let clone = progress.clone();
    progress.reset(100);
    progress.add(40);
    assert_eq!(clone.get(), (40, 100));
}

#[cfg(feature = "signals")]
#[test]
fn sigint_sets_a_registered_flag() {
    let flag = StopFlag::register_sigint().expect("registering the handler failed");
    assert!(!flag.requested());

    // Handled on this thread before `raise` returns. Only one: a second SIGINT while the flag
    // is set would end the test process, as designed.
    // SAFETY: raise(3) only delivers a signal to the calling thread.
    assert_eq!(unsafe { libc::raise(libc::SIGINT) }, 0);

    assert!(flag.requested());
}
