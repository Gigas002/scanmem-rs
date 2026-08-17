//! Loads `config.toml` into a raw, unvalidated deserialization type — per `ARCHITECTURE.md` §3.2,
//! this module only reads files; `settings/` applies CLI-over-config-over-defaults precedence and
//! validates field values. `theme.toml` is handled by the separate top-level `theme` module —
//! cosmetics are a distinct concern from general application settings, even though both are TOML
//! files loaded the same way.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Failure loading `config.toml`.
#[derive(Debug, Error)]
pub enum ConfigError {
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

/// Raw `config.toml` contents — every field optional, since any of them may be left at the
/// built-in default or overridden by a CLI flag instead; see `settings::resolve` for how these
/// merge. `hex-view`/`cheat-list`-specific fields (`hex_view_buffer_len`) are always present here
/// regardless of which of those features a build enables, so the *same* `config.toml` works
/// unchanged across every build — an unused field is simply ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FileConfig {
    /// Pid to attach to immediately on startup.
    pub pid: Option<u32>,
    /// `tracing` level filter: `"trace"`/`"debug"`/`"info"`/`"warn"`/`"error"`.
    pub log_level: Option<String>,
    /// Path the file-only `tracing` subscriber appends to.
    pub log_file: Option<PathBuf>,
    /// How often (in milliseconds) the event loop wakes up with no key pressed, to rewrite
    /// frozen cheats and poll a background scan's progress.
    pub poll_interval_ms: Option<u64>,
    /// Bytes of session memory loaded into the Hex View on either side of the focused address.
    pub hex_view_buffer_len: Option<usize>,
    /// Scan Panel's data type on startup — one of `"i8"`/`"i16"`/`"i32"`/`"i64"`/`"f32"`/`"f64"`/
    /// `"any"`/`"anyint"`/`"anyfloat"`/`"bytes"`/`"string"`.
    pub default_scan_data_type: Option<String>,
    /// Scan Panel's match type on startup — one of `"="`/`"!="`/`">"`/`"<"`/`"range"`/
    /// `"update"`/`"unchanged"`/`"changed"`/`"increased"`/`"decreased"`/`"+"`/`"-"`/`"any"`.
    pub default_scan_match_type: Option<String>,
}

/// Loads `config.toml`: `cli_path` if given, otherwise the conventional
/// `$XDG_CONFIG_HOME/gameconqueror/config.toml` (`~/.config/...` if `XDG_CONFIG_HOME` isn't set).
/// A missing *conventional* path is `Ok(None)` (no config file is the normal case, not an error);
/// a missing *explicitly requested* `cli_path`, or a file that exists but fails to parse, always
/// errors.
pub fn load_config(cli_path: Option<&Path>) -> Result<Option<FileConfig>, ConfigError> {
    let (path, explicit) = match cli_path {
        Some(path) => (path.to_owned(), true),
        None => (default_config_path(), false),
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if !explicit && source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(source) => return Err(ConfigError::Read { path, source }),
    };

    toml::from_str(&text)
        .map(Some)
        .map_err(|source| ConfigError::Parse { path, source })
}

fn default_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// `$XDG_CONFIG_HOME/gameconqueror` (`~/.config/gameconqueror` if `XDG_CONFIG_HOME` isn't set) —
/// the conventional directory `config.toml` and (separately, in the `theme` module) `theme.toml`
/// both live in.
pub(crate) fn config_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("gameconqueror");
    }
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    home.join(".config").join("gameconqueror")
}

#[cfg(test)]
mod tests;
