use clap::Parser;

#[cfg(feature = "config")]
use super::SettingsError;
use super::{Settings, resolve};
use crate::cli::CliArgs;

fn cli(args: &[&str]) -> CliArgs {
    let mut full = vec!["gameconqueror"];
    full.extend_from_slice(args);
    CliArgs::parse_from(full)
}

#[test]
fn defaults_when_nothing_is_provided() {
    let settings = resolve(&cli(&[])).expect("resolve should succeed with no config file");
    assert_eq!(settings, Settings::default());
}

#[test]
fn cli_pid_is_resolved() {
    let settings = resolve(&cli(&["--pid", "1234"])).expect("resolve should succeed");
    assert_eq!(settings.pid, Some(1234));
}

#[test]
#[cfg(feature = "config")]
fn cli_config_pid_is_resolved_and_cli_flag_overrides_it() {
    let path = std::env::temp_dir().join(format!(
        "gameconqueror-settings-test-{}-config.toml",
        std::process::id()
    ));
    std::fs::write(&path, "pid = 1111\nlog-level = \"debug\"\n").unwrap();

    let settings = resolve(&cli(&["--config", path.to_str().unwrap()]))
        .expect("resolve should succeed with a valid config file");
    assert_eq!(settings.pid, Some(1111));
    assert_eq!(
        settings.log_level,
        tracing::level_filters::LevelFilter::DEBUG
    );

    // CLI still wins over the config file.
    let settings = resolve(&cli(&["--config", path.to_str().unwrap(), "--pid", "2222"]))
        .expect("resolve should succeed");
    assert_eq!(settings.pid, Some(2222));

    std::fs::remove_file(&path).ok();
}

#[test]
#[cfg(feature = "config")]
fn cli_config_with_an_invalid_scan_data_type_errors() {
    let path = std::env::temp_dir().join(format!(
        "gameconqueror-settings-test-{}-bad-scan-type.toml",
        std::process::id()
    ));
    std::fs::write(&path, "default-scan-data-type = \"not-a-type\"\n").unwrap();

    let err = resolve(&cli(&["--config", path.to_str().unwrap()]))
        .expect_err("an unrecognized scan data type must be rejected");
    assert!(matches!(err, SettingsError::InvalidScanDataType(_)));

    std::fs::remove_file(&path).ok();
}

#[test]
#[cfg(feature = "config")]
fn cli_theme_is_carried_through_unparsed() {
    let path = std::env::temp_dir().join(format!(
        "gameconqueror-settings-test-{}-theme.toml",
        std::process::id()
    ));
    std::fs::write(&path, "focused-border = \"magenta\"\n").unwrap();

    let settings = resolve(&cli(&["--theme", path.to_str().unwrap()]))
        .expect("resolve should succeed with a valid theme file");
    assert_eq!(settings.theme.focused_border.as_deref(), Some("magenta"));

    std::fs::remove_file(&path).ok();
}
