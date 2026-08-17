//! Merges CLI + config file (if the `config` feature loads one) + defaults into a single
//! resolved [`Settings`] — the only type passed below this module, per `ARCHITECTURE.md` §3.

use std::path::PathBuf;

use libscanmem::scanroutines::{MatchType, ScanDataType};
use thiserror::Error;
use tracing::level_filters::LevelFilter;

use crate::cli::CliArgs;

/// Fully resolved `gameconqueror` configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Pid to attach to immediately on startup, if any.
    pub pid: Option<u32>,
    /// Path the file-only `tracing` subscriber appends to.
    pub log_file: PathBuf,
    /// Level filter for the `tracing` subscriber.
    pub log_level: LevelFilter,
    /// How often (in milliseconds) the event loop wakes up with no key pressed, to rewrite
    /// frozen cheats and poll a background scan's progress.
    pub poll_interval_ms: u64,
    /// Bytes of session memory loaded into the Hex View on either side of the focused address.
    pub hex_view_buffer_len: usize,
    /// Scan Panel's data type on startup.
    pub default_scan_data_type: ScanDataType,
    /// Scan Panel's match type on startup.
    pub default_scan_match_type: MatchType,
    /// Raw `theme.toml` contents (plain color strings) — `ui/theme.rs` is the only place that
    /// resolves these into real `ratatui::style::Color`s, so they stay unparsed here.
    #[cfg(feature = "config")]
    pub theme: crate::theme::FileTheme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pid: None,
            log_file: std::env::temp_dir().join("gameconqueror.log"),
            log_level: LevelFilter::WARN,
            poll_interval_ms: 100,
            hex_view_buffer_len: 256,
            default_scan_data_type: ScanDataType::Integer32,
            default_scan_match_type: MatchType::EqualTo,
            #[cfg(feature = "config")]
            theme: crate::theme::FileTheme::default(),
        }
    }
}

/// Failure resolving [`Settings`] — only reachable through the `config` feature's file loading
/// and field validation; without it, [`resolve`] cannot fail.
#[derive(Debug, Error)]
pub enum SettingsError {
    #[cfg(feature = "config")]
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[cfg(feature = "config")]
    #[error(transparent)]
    Theme(#[from] crate::theme::ThemeError),
    #[cfg(feature = "config")]
    #[error(
        "invalid log-level {0:?} in config.toml (expected one of: trace, debug, info, warn, \
         error, off)"
    )]
    InvalidLogLevel(String),
    #[cfg(feature = "config")]
    #[error(
        "invalid default-scan-data-type {0:?} in config.toml (expected one of: i8, i16, i32, \
         i64, f32, f64, any, anyint, anyfloat, bytes, string)"
    )]
    InvalidScanDataType(String),
    #[cfg(feature = "config")]
    #[error(
        "invalid default-scan-match-type {0:?} in config.toml (expected one of: =, !=, >, <, \
         range, update, unchanged, changed, increased, decreased, +, -, any)"
    )]
    InvalidScanMatchType(String),
}

/// Resolves `Settings` from `cli`, merging a file config (if the `config` feature loads one) and
/// `cli` itself over the built-in defaults — precedence is fixed: CLI arguments win over the
/// config file, which wins over defaults (`ARCHITECTURE.md` §3.1).
pub fn resolve(cli: &CliArgs) -> Result<Settings, SettingsError> {
    let mut settings = Settings::default();

    #[cfg(feature = "config")]
    apply_file_config(&mut settings, cli)?;

    if let Some(pid) = cli.pid {
        settings.pid = Some(pid);
    }

    Ok(settings)
}

#[cfg(feature = "config")]
fn apply_file_config(settings: &mut Settings, cli: &CliArgs) -> Result<(), SettingsError> {
    if let Some(file) = crate::config::load_config(cli.config.as_deref())? {
        if let Some(pid) = file.pid {
            settings.pid = Some(pid);
        }
        if let Some(level) = file.log_level {
            settings.log_level = level
                .parse()
                .map_err(|_| SettingsError::InvalidLogLevel(level))?;
        }
        if let Some(log_file) = file.log_file {
            settings.log_file = log_file;
        }
        if let Some(ms) = file.poll_interval_ms {
            settings.poll_interval_ms = ms;
        }
        if let Some(len) = file.hex_view_buffer_len {
            settings.hex_view_buffer_len = len;
        }
        if let Some(value) = file.default_scan_data_type {
            settings.default_scan_data_type =
                parse_scan_data_type(&value).ok_or(SettingsError::InvalidScanDataType(value))?;
        }
        if let Some(value) = file.default_scan_match_type {
            settings.default_scan_match_type =
                parse_scan_match_type(&value).ok_or(SettingsError::InvalidScanMatchType(value))?;
        }
    }

    settings.theme = crate::theme::load_theme(cli.theme.as_deref())?.unwrap_or_default();

    Ok(())
}

/// Parses `config.toml`'s `default-scan-data-type` — the same short vocabulary
/// `ui/scan_panel.rs`'s title hint displays (`i8`, `i32`, `any`, `bytes`, ...).
#[cfg(feature = "config")]
fn parse_scan_data_type(s: &str) -> Option<ScanDataType> {
    Some(match s {
        "i8" => ScanDataType::Integer8,
        "i16" => ScanDataType::Integer16,
        "i32" => ScanDataType::Integer32,
        "i64" => ScanDataType::Integer64,
        "f32" => ScanDataType::Float32,
        "f64" => ScanDataType::Float64,
        "any" => ScanDataType::AnyNumber,
        "anyint" => ScanDataType::AnyInteger,
        "anyfloat" => ScanDataType::AnyFloat,
        "bytes" => ScanDataType::ByteArray,
        "string" => ScanDataType::String,
        _ => return None,
    })
}

/// Parses `config.toml`'s `default-scan-match-type` — the same short vocabulary
/// `ui/scan_panel.rs`'s title hint displays (`=`, `range`, `increased`, ...).
#[cfg(feature = "config")]
fn parse_scan_match_type(s: &str) -> Option<MatchType> {
    Some(match s {
        "=" => MatchType::EqualTo,
        "!=" => MatchType::NotEqualTo,
        ">" => MatchType::GreaterThan,
        "<" => MatchType::LessThan,
        "range" => MatchType::Range,
        "update" => MatchType::Update,
        "unchanged" => MatchType::NotChanged,
        "changed" => MatchType::Changed,
        "increased" => MatchType::Increased,
        "decreased" => MatchType::Decreased,
        "+" => MatchType::IncreasedBy,
        "-" => MatchType::DecreasedBy,
        "any" => MatchType::Any,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
