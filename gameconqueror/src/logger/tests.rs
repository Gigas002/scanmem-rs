use std::fs;

use std::os::unix::fs::PermissionsExt;

use super::{init, open_log_file};
use crate::settings::Settings;

#[test]
fn init_does_not_panic_and_creates_the_log_file() {
    // Per-process name: parallel runs would otherwise delete each other's file.
    let log_file = std::env::temp_dir().join(format!(
        "gameconqueror-logger-test-{}.log",
        std::process::id()
    ));
    let _ = fs::remove_file(&log_file);

    let settings = Settings {
        log_file: log_file.clone(),
        ..Settings::default()
    };

    init(&settings);
    init(&settings);

    assert!(log_file.exists());
    let _ = fs::remove_file(&log_file);
}

/// A fresh directory under the temp dir, per process and test.
fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gameconqueror-logger-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_log_and_its_directories_are_private() {
    let dir = scratch_dir("private");
    let log_file = dir.join("state/gameconqueror/gameconqueror.log");

    open_log_file(&log_file).unwrap();

    let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&log_file), 0o600);
    assert_eq!(mode(log_file.parent().unwrap()), 0o700);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_symlink_in_place_of_the_log_is_not_followed() {
    let dir = scratch_dir("symlink");
    let victim = dir.join("victim");
    fs::write(&victim, "untouched").unwrap();
    let log_file = dir.join("gameconqueror.log");
    std::os::unix::fs::symlink(&victim, &log_file).unwrap();

    assert!(open_log_file(&log_file).is_err());

    assert_eq!(fs::read_to_string(&victim).unwrap(), "untouched");
    let _ = fs::remove_dir_all(&dir);
}
