use std::path::PathBuf;

use super::*;

fn write_temp(name: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gameconqueror-theme-test-{}-{name}",
        std::process::id()
    ));
    std::fs::write(&path, contents).expect("failed to write temp file");
    path
}

#[test]
fn load_theme_reads_and_parses_an_explicit_path() {
    let path = write_temp(
        "theme.toml",
        r##"
            focused-border = "cyan"
            match-changed = "#ff8800"
        "##,
    );

    let theme = load_theme(Some(&path))
        .expect("expected a valid theme")
        .expect("expected Some");

    assert_eq!(theme.focused_border.as_deref(), Some("cyan"));
    assert_eq!(theme.match_changed.as_deref(), Some("#ff8800"));
    assert_eq!(theme.frozen, None);

    std::fs::remove_file(&path).ok();
}

#[test]
fn load_theme_with_a_missing_explicit_path_errors() {
    let path = PathBuf::from("/nonexistent/gameconqueror-test/theme.toml");

    let err = load_theme(Some(&path)).expect_err("expected a read error");

    assert!(matches!(err, ThemeError::Read { .. }));
}

#[test]
fn load_theme_with_an_unparsable_file_errors() {
    let path = write_temp("bad.toml", "this is not valid TOML {{{");

    let err = load_theme(Some(&path)).expect_err("expected a parse error");

    assert!(matches!(err, ThemeError::Parse { .. }));

    std::fs::remove_file(&path).ok();
}

#[test]
fn load_theme_with_no_explicit_path_and_a_missing_default_returns_none() {
    // Same reasoning as `config::tests::load_config_with_no_explicit_path_and_a_missing_default_returns_none`
    // — reads the real conventional default path, only asserting the missing-default-is-not-an-
    // error contract.
    let result = load_theme(None);
    assert!(result.is_ok());
}

#[test]
fn file_theme_defaults_to_every_field_none() {
    let theme = FileTheme::default();
    assert_eq!(theme, FileTheme::default());
    assert_eq!(theme.focused_border, None);
    assert_eq!(theme.match_changed, None);
    assert_eq!(theme.frozen, None);
}
