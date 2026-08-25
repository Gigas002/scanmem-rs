//! Loads `theme.toml` into a raw, unvalidated deserialization type — a separate top-level module
//! from `config/`'s `config.toml` handling, since theme customization (cosmetics) is a distinct
//! concern from general application settings. `ui/theme.rs` is the only place that turns
//! [`FileTheme`]'s plain color strings into real `ratatui::style::Color`s, keeping this module
//! (like `config/`) toolkit-independent.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Failure loading `theme.toml`.
#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("{path}: failed to read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: failed to parse: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

/// Raw `theme.toml` contents — plain color strings in `ratatui::style::Color`'s own `FromStr`
/// syntax (named colors like `"cyan"`/`"light red"`, `"#rrggbb"` hex, or a `0`-`255` palette
/// index), parsed into real `Color`s only in `ui/theme.rs`. Every field is optional; an absent
/// field keeps `ui::theme::Theme`'s built-in default for it (not an intra-doc link: `ui` only
/// exists behind the separate `tui` feature, which `config` doesn't imply).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FileTheme {
    /// Border of whichever panel currently has focus.
    pub focused_border: Option<String>,
    /// Status bar background, normal (non-error) state.
    pub status_bar_bg: Option<String>,
    /// Status bar text, normal (non-error) state.
    pub status_bar_fg: Option<String>,
    /// Status bar background while a process is attached (and no error is showing).
    pub status_bar_attached_bg: Option<String>,
    /// Status bar text while a process is attached (and no error is showing).
    pub status_bar_attached_fg: Option<String>,
    /// Status bar background while showing an error status.
    pub status_bar_error_bg: Option<String>,
    /// Status bar text while showing an error status.
    pub status_bar_error_fg: Option<String>,
    /// Scan Panel's progress gauge fill.
    pub scan_progress: Option<String>,
    /// Selected row highlight, in every table (Process Picker, Match View, Cheat View).
    pub selection: Option<String>,
    /// Match View's value cell for a match whose value changed on the most recent scan/refresh.
    pub match_changed: Option<String>,
    /// Cheat View's Frozen column for a currently-frozen cheat.
    pub frozen: Option<String>,
}

/// Loads `theme.toml`: `cli_path` if given, otherwise the conventional
/// `$XDG_CONFIG_HOME/gameconqueror/theme.toml` (`~/.config/...` if `XDG_CONFIG_HOME` isn't set).
/// A missing *conventional* path is `Ok(None)` (no theme file is the normal case, not an error);
/// a missing *explicitly requested* `cli_path`, or a file that exists but fails to parse, always
/// errors.
pub fn load_theme(cli_path: Option<&Path>) -> Result<Option<FileTheme>, ThemeError> {
    let (path, explicit) = match cli_path {
        Some(path) => (path.to_owned(), true),
        None => (default_theme_path(), false),
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if !explicit && source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(source) => return Err(ThemeError::Read { path, source }),
    };

    toml::from_str(&text)
        .map(Some)
        .map_err(|source| ThemeError::Parse { path, source })
}

fn default_theme_path() -> PathBuf {
    crate::config::config_dir().join("theme.toml")
}

#[cfg(test)]
mod tests;
