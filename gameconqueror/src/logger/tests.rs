use std::fs;

use super::init;
use crate::settings::Settings;

#[test]
fn init_does_not_panic_and_creates_the_log_file() {
    let log_file = std::env::temp_dir().join("gameconqueror-logger-test.log");
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
