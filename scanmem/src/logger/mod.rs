//! `tracing` subscriber initialization from [`Settings`] — the only place this crate configures
//! a subscriber.

use crate::settings::Settings;

/// Initializes the global `tracing` subscriber at the level resolved in `settings`.
///
/// Uses `try_init` so calling this more than once (e.g. across tests in the same process) does
/// not panic; only the first call takes effect.
pub fn init(settings: &Settings) {
    let _ = tracing_subscriber::fmt()
        .with_max_level(settings.log_level)
        .with_target(false)
        .try_init();
}

#[cfg(test)]
mod tests;
