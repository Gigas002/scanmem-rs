//! Library surface for `gameconqueror`'s modules, primarily so `tests/` can exercise
//! `app::update` directly against `libscanmem`'s `fake_target` helper without a terminal
//! attached. `main.rs` is a thin wrapper around this crate.

pub mod app;
pub mod cli;
#[cfg(feature = "config")]
pub mod config;
pub mod logger;
pub mod settings;
#[cfg(feature = "config")]
pub mod theme;

#[cfg(feature = "tui")]
pub mod ui;
