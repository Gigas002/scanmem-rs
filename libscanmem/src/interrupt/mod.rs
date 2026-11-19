//! Cooperative long-scan abort flag and progress counter — the abort flag replaces
//! `interrupt.c`'s raw `signal()`/`sigaction` calls; the progress counter has no C precedent
//! (the original CLI/GTK frontends had no notion of a cancellable, progress-reporting scan
//! running off the main thread).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A cooperative stop flag: [`StopFlag::requested`] becomes `true` once a long-running scan
/// should abort, whether set directly via [`StopFlag::request`] or — with the `signals` feature
/// — by an incoming `SIGINT` via [`StopFlag::register_sigint`].
#[derive(Debug, Clone, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl StopFlag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a `SIGINT` handler that sets a freshly created flag — replaces
    /// `interrupt.c`'s `sig_handler`/signal-registration setup. A second `SIGINT` while the flag
    /// is still set terminates the process as usual, so a command that never checks the flag
    /// can still be interrupted; [`Self::reset`] it once the command is done.
    #[cfg(feature = "signals")]
    pub fn register_sigint() -> std::io::Result<Self> {
        use signal_hook::consts::SIGINT;

        let flag = Self::default();
        // Handlers run in registration order: check for an earlier press, then record this one.
        signal_hook::flag::register_conditional_default(SIGINT, Arc::clone(&flag.0))?;
        signal_hook::flag::register(SIGINT, Arc::clone(&flag.0))?;
        Ok(flag)
    }

    /// `true` once a stop has been requested.
    pub fn requested(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Requests a stop; embedders driving their own event loop call this directly instead of
    /// relying on [`Self::register_sigint`].
    pub fn request(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Clears the flag, e.g. before starting a new scan.
    pub fn reset(&self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
struct Counters {
    done: AtomicUsize,
    total: AtomicUsize,
}

/// A cloneable, cross-thread-safe `(done, total)` byte counter for an in-progress
/// [`crate::session::Session::scan`]/`snapshot` call — lets a UI thread poll a running scan's
/// progress (and render it) while the scan itself runs elsewhere, e.g. on a background thread so
/// it doesn't block rendering.
#[derive(Debug, Clone, Default)]
pub struct ScanProgress(Arc<Counters>);

impl ScanProgress {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a fresh scan of `total` bytes, zeroing `done`.
    pub fn reset(&self, total: usize) {
        self.0.done.store(0, Ordering::Relaxed);
        self.0.total.store(total, Ordering::Relaxed);
    }

    /// Records `bytes` more as scanned.
    pub fn add(&self, bytes: usize) {
        self.0.done.fetch_add(bytes, Ordering::Relaxed);
    }

    /// `(bytes scanned so far, total bytes considered by the current/last scan)`.
    pub fn get(&self) -> (usize, usize) {
        (
            self.0.done.load(Ordering::Relaxed),
            self.0.total.load(Ordering::Relaxed),
        )
    }
}

#[cfg(test)]
mod tests;
