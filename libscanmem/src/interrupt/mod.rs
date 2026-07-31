//! Cooperative long-scan abort flag — replaces `interrupt.c`'s raw `signal()`/`sigaction` calls.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
    /// `interrupt.c`'s `sig_handler`/signal-registration setup.
    #[cfg(feature = "signals")]
    pub fn register_sigint() -> std::io::Result<Self> {
        let flag = Self::default();
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&flag.0))?;
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

#[cfg(test)]
mod tests;
