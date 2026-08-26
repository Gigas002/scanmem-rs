use std::path::PathBuf;

use super::*;

fn write_temp(name: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gameconqueror-config-test-{}-{name}",
        std::process::id()
    ));
    std::fs::write(&path, contents).expect("failed to write temp file");
    path
}

#[test]
fn load_config_reads_and_parses_an_explicit_path() {
    let path = write_temp(
        "config.toml",
        r#"
            pid = 1234
            log-level = "debug"
            default-scan-data-type = "i64"
        "#,
    );

    let config = load_config(Some(&path))
        .expect("expected a valid config")
        .expect("expected Some");

    assert_eq!(config.pid, Some(1234));
    assert_eq!(config.log_level.as_deref(), Some("debug"));
    assert_eq!(config.default_scan_data_type.as_deref(), Some("i64"));
    assert_eq!(config.default_scan_match_type, None);

    std::fs::remove_file(&path).ok();
}

#[test]
fn load_config_with_a_missing_explicit_path_errors() {
    let path = PathBuf::from("/nonexistent/gameconqueror-test/config.toml");

    let err = load_config(Some(&path)).expect_err("expected a read error");

    assert!(matches!(err, ConfigError::Read { .. }));
}

#[test]
fn load_config_with_an_unparsable_file_errors() {
    let path = write_temp("bad.toml", "this is not valid TOML {{{");

    let err = load_config(Some(&path)).expect_err("expected a parse error");

    assert!(matches!(err, ConfigError::Parse { .. }));

    std::fs::remove_file(&path).ok();
}

#[test]
fn load_config_with_no_explicit_path_and_a_missing_default_returns_none() {
    // `load_config(None)` reads from the real conventional default path — this only asserts the
    // "missing conventional default is not an error" contract holds for whatever the test
    // environment's `$XDG_CONFIG_HOME`/`$HOME` happens to resolve to not having a config.toml at.
    // If that assumption ever breaks in CI, this is the test to revisit.
    let result = load_config(None);
    assert!(result.is_ok());
}

#[test]
fn file_config_defaults_to_every_field_none() {
    let config = FileConfig::default();
    assert_eq!(config, FileConfig::default());
    assert_eq!(config.pid, None);
    assert_eq!(config.hex_view_buffer_len, None);
    assert_eq!(config.default_scan_data_type, None);
    assert_eq!(config.default_scan_match_type, None);
}

#[test]
fn config_dir_ends_in_gameconqueror() {
    // Not asserting the exact parent (that's `$XDG_CONFIG_HOME`/`$HOME`-dependent, same reasoning
    // `ui/theme.rs`'s `color_enabled` is left untested directly) — just the fixed suffix every
    // resolution path shares.
    assert_eq!(
        config_dir().file_name(),
        Some(std::ffi::OsStr::new("gameconqueror"))
    );
}
