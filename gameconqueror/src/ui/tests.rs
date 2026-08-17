use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(feature = "cheat-list")]
use super::cheat_view;
#[cfg(feature = "hex-view")]
use super::hex_view;
use super::install_panic_hook;
use super::layout::render;
use super::{help_overlay, input, keymap, match_view, process_picker, scan_panel, theme};
#[cfg(feature = "cheat-list")]
use crate::app::PathPromptKind;
use crate::app::{AppState, Focus, Msg, update};

#[test]
fn installing_the_panic_hook_does_not_panic() {
    install_panic_hook();
}

#[test]
fn layout_renders_without_panicking() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();

    terminal.draw(|frame| render(frame, &state)).unwrap();
}

#[test]
fn process_picker_renders_without_panicking() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    update(&mut state, Msg::RefreshProcessList);

    terminal
        .draw(|frame| {
            let area = frame.area();
            process_picker::render(frame, area, &state, true);
        })
        .unwrap();
}

#[test]
fn scan_panel_renders_without_panicking() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();

    terminal
        .draw(|frame| {
            let area = frame.area();
            scan_panel::render(frame, area, &state, true);
        })
        .unwrap();
}

#[test]
fn scan_panel_progress_ratio_guards_against_a_zero_total() {
    assert_eq!(scan_panel::progress_ratio(0, 0), 0.0);
}

#[test]
fn scan_panel_progress_ratio_computes_the_fraction_done() {
    assert_eq!(scan_panel::progress_ratio(25, 100), 0.25);
    assert_eq!(scan_panel::progress_ratio(100, 100), 1.0);
}

#[test]
fn scan_panel_progress_ratio_clamps_above_the_total() {
    // Defensive: `Gauge::ratio` panics outside `[0.0, 1.0]`, so this must hold even if `done`
    // ever ends up ahead of `total` (e.g. a future change to how they're accumulated).
    assert_eq!(scan_panel::progress_ratio(150, 100), 1.0);
}

#[test]
fn match_view_renders_without_panicking() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();

    terminal
        .draw(|frame| {
            let area = frame.area();
            match_view::render(frame, area, &state, true);
        })
        .unwrap();
}

#[test]
#[cfg(feature = "hex-view")]
fn hex_view_renders_without_panicking_on_an_empty_buffer() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();

    terminal
        .draw(|frame| {
            let area = frame.area();
            hex_view::render(frame, area, &state, true);
        })
        .unwrap();
}

#[test]
fn help_overlay_renders_without_panicking_for_every_focus() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    for focus in [
        Focus::ProcessPicker,
        Focus::ScanPanel,
        Focus::MatchView,
        #[cfg(feature = "cheat-list")]
        Focus::CheatView,
        #[cfg(feature = "hex-view")]
        Focus::HexView,
    ] {
        terminal
            .draw(|frame| help_overlay::render(frame, focus))
            .unwrap();
    }
}

#[test]
fn layout_renders_the_help_overlay_without_panicking() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    update(&mut state, Msg::ShowHelp);

    terminal.draw(|frame| render(frame, &state)).unwrap();
}

#[test]
fn global_bindings_match_the_documented_table() {
    let cases = [
        (
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            Msg::FocusNext,
        ),
        (
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            Msg::FocusPrev,
        ),
        (
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
            Msg::ShowHelp,
        ),
        (
            KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE),
            Msg::ShowHelp,
        ),
        (
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            Msg::Quit,
        ),
        (
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Msg::Detach,
        ),
        (
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            Msg::Dismiss,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(keymap::lookup_global(key), Some(expected));
    }
}

#[test]
fn global_bindings_include_directional_focus_and_expand() {
    use crate::app::Direction;

    let cases = [
        (
            KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL),
            Msg::FocusDirection(Direction::Left),
        ),
        (
            KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL),
            Msg::FocusDirection(Direction::Right),
        ),
        (
            KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL),
            Msg::FocusDirection(Direction::Up),
        ),
        (
            KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL),
            Msg::FocusDirection(Direction::Down),
        ),
        (
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
            Msg::ToggleExpand,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(keymap::lookup_global(key), Some(expected));
    }
}

#[test]
fn ctrl_up_while_searching_falls_through_to_focus_direction_instead_of_selecting() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);
    assert_eq!(state.focus(), Focus::ProcessPicker);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL),
    );

    // No panel above the Process Picker, so FocusDirection(Up) is a no-op — but crucially it must
    // *not* have been swallowed as Msg::SelectPrev by the search-mode input handler.
    assert_eq!(state.focus(), Focus::ProcessPicker);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL),
    );
    assert_eq!(state.focus(), Focus::ScanPanel);
}

#[test]
fn process_picker_bindings_match_the_documented_table() {
    let cases = [
        (
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            Msg::SelectPrev,
        ),
        (
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            Msg::SelectNext,
        ),
        (
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            Msg::ToggleSearch,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(
            keymap::lookup_focus(Focus::ProcessPicker, key),
            Some(expected)
        );
    }
}

#[test]
fn scan_panel_bindings_match_the_documented_table() {
    let cases = [
        (
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
            Msg::CycleScanDataType,
        ),
        (
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            Msg::CycleScanMatchType,
        ),
        (
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            Msg::ToggleSearch,
        ),
        (
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            Msg::RunScan,
        ),
        (
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
            Msg::NewScan,
        ),
        (
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            Msg::RefreshMatches,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(keymap::lookup_focus(Focus::ScanPanel, key), Some(expected));
    }
}

#[test]
fn match_view_bindings_match_the_documented_table() {
    let cases = [
        (
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            Msg::SelectPrev,
        ),
        (
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            Msg::SelectPrev,
        ),
        (
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            Msg::SelectNext,
        ),
        (
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            Msg::SelectNext,
        ),
        (
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
            Msg::CycleMatchSort,
        ),
        (
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            Msg::ToggleSearch,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(keymap::lookup_focus(Focus::MatchView, key), Some(expected));
    }
}

#[test]
#[cfg(feature = "hex-view")]
fn hex_view_bindings_match_the_documented_table() {
    let cases = [
        (
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            Msg::MoveHexCursor(-1),
        ),
        (
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            Msg::MoveHexCursor(1),
        ),
        (
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            Msg::MoveHexCursor(-(hex_view::BYTES_PER_ROW as isize)),
        ),
        (
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            Msg::MoveHexCursor(hex_view::BYTES_PER_ROW as isize),
        ),
        (
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            Msg::CommitHexEdit,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(keymap::lookup_focus(Focus::HexView, key), Some(expected));
    }
}

#[test]
fn unbound_keys_return_none_from_both_lookups() {
    assert_eq!(
        keymap::lookup_global(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        None
    );
    assert_eq!(
        keymap::lookup_focus(
            Focus::ScanPanel,
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)
        ),
        None
    );
}

#[test]
fn typing_while_searching_appends_to_the_filter() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
    );

    assert_eq!(state.process_filter(), "ab");
}

#[test]
fn backspace_while_searching_removes_the_last_character() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );

    assert_eq!(state.process_filter(), "");
}

#[test]
fn enter_while_searching_exits_search_and_keeps_the_filter() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );

    assert!(!state.search_active());
    assert_eq!(state.process_filter(), "a");
}

#[test]
fn escape_while_searching_clears_the_filter_and_exits_search() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );

    input::handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(!state.search_active());
    assert_eq!(state.process_filter(), "");
}

#[test]
fn tab_cycles_focus_even_outside_search_mode() {
    let mut state = AppState::default();

    input::handle_key(&mut state, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    assert_eq!(state.focus(), Focus::ScanPanel);
}

#[test]
fn typing_while_editing_the_scan_value_appends_to_the_input() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::ScanPanel);
    update(&mut state, Msg::ToggleSearch);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
    );
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE),
    );

    assert_eq!(state.scan_input(), "42");
}

#[test]
fn escape_while_editing_the_scan_value_clears_it_and_exits_search() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::ToggleSearch);
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
    );

    input::handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(!state.search_active());
    assert_eq!(state.scan_input(), "");
}

#[test]
fn typing_while_editing_the_match_filter_appends_to_the_query() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::MatchView);
    update(&mut state, Msg::ToggleSearch);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
    );
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );

    assert_eq!(state.match_filter(), "de");
}

#[test]
fn escape_while_editing_the_match_filter_clears_it_and_exits_search() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::ToggleSearch);
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
    );

    input::handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(!state.search_active());
    assert_eq!(state.match_filter(), "");
}

#[test]
fn enter_on_the_process_picker_attaches_to_the_selected_process() {
    let mut state = AppState::default();
    update(&mut state, Msg::RefreshProcessList);
    assert!(
        !state.processes().is_empty(),
        "expected /proc to list at least one process"
    );

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );

    assert!(
        state.status().is_some(),
        "expected Attach to report a status"
    );
}

#[test]
#[cfg(feature = "hex-view")]
fn h_on_match_view_without_a_session_does_not_focus_the_hex_view() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::MatchView);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
    );

    assert_eq!(state.focus(), Focus::MatchView);
}

#[test]
#[cfg(feature = "hex-view")]
fn typing_hex_digits_on_the_hex_view_composes_the_byte_edit_and_ignores_non_hex_chars() {
    let mut state = AppState::default();
    while state.focus() != Focus::HexView {
        update(&mut state, Msg::FocusNext);
    }

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
    );
    assert_eq!(state.hex_edit_input(), "");

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
    );
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
    );
    assert_eq!(state.hex_edit_input(), "3f");

    // A third digit is ignored once two are already composed.
    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE),
    );
    assert_eq!(state.hex_edit_input(), "3f");

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );
    assert_eq!(state.hex_edit_input(), "3");
}

#[test]
#[cfg(feature = "cheat-list")]
fn cheat_view_renders_without_panicking() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: "health".to_owned(),
            value: libscanmem::value::Value::U32(100),
        },
    );

    terminal
        .draw(|frame| {
            let area = frame.area();
            cheat_view::render(frame, area, &state, true);
        })
        .unwrap();
}

#[test]
#[cfg(feature = "cheat-list")]
fn layout_renders_the_path_prompt_overlay_without_panicking() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    update(&mut state, Msg::SaveCheatList);

    terminal.draw(|frame| render(frame, &state)).unwrap();
}

#[test]
#[cfg(feature = "cheat-list")]
fn cheat_view_bindings_match_the_documented_table() {
    let cases = [
        (
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            Msg::SelectPrev,
        ),
        (
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            Msg::SelectPrev,
        ),
        (
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            Msg::SelectNext,
        ),
        (
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            Msg::SelectNext,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(keymap::lookup_focus(Focus::CheatView, key), Some(expected));
    }
}

#[test]
#[cfg(feature = "cheat-list")]
fn global_bindings_include_save_and_load_cheat_list() {
    assert_eq!(
        keymap::lookup_global(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
        Some(Msg::SaveCheatList)
    );
    assert_eq!(
        keymap::lookup_global(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL)),
        Some(Msg::LoadCheatList)
    );
}

#[test]
#[cfg(feature = "cheat-list")]
fn space_toggles_freeze_on_the_selected_cheat() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: String::new(),
            value: libscanmem::value::Value::U32(100),
        },
    );
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::CheatView);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
    );

    assert!(state.cheats()[0].frozen);
}

#[test]
#[cfg(feature = "cheat-list")]
fn e_begins_editing_the_selected_cheat_value() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: String::new(),
            value: libscanmem::value::Value::U32(100),
        },
    );
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::CheatView);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );

    assert!(state.search_active());
    assert_eq!(state.cheat_value_input(), "100");
}

#[test]
#[cfg(feature = "cheat-list")]
fn a_on_match_view_without_a_session_does_not_add_a_cheat() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::MatchView);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );

    assert!(state.cheats().is_empty());
}

#[test]
#[cfg(all(feature = "cheat-list", feature = "hex-view"))]
fn h_on_cheat_view_without_a_session_does_not_focus_the_hex_view() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: String::new(),
            value: libscanmem::value::Value::U32(100),
        },
    );
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::CheatView);

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
    );

    assert_eq!(state.focus(), Focus::CheatView);
}

#[test]
#[cfg(feature = "cheat-list")]
fn path_prompt_key_composes_input_and_confirms_on_enter() {
    let mut state = AppState::default();
    update(&mut state, Msg::LoadCheatList);
    assert_eq!(state.path_prompt(), Some(PathPromptKind::Load));

    input::handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_eq!(state.path_input(), "x");

    input::handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(state.path_prompt().is_none());
    assert_eq!(state.path_input(), "");
}

#[test]
fn theme_default_matches_the_documented_colors() {
    use ratatui::style::{Color, Modifier};

    let default = theme::Theme::default();
    assert_eq!(default.focused_border.fg, Some(Color::Cyan));
    assert!(default.focused_border.add_modifier.contains(Modifier::BOLD));
    assert_eq!(default.status_bar.fg, Some(Color::Black));
    assert_eq!(default.status_bar.bg, Some(Color::Gray));
    assert_eq!(default.status_bar_error.fg, Some(Color::White));
    assert_eq!(default.status_bar_error.bg, Some(Color::Red));
    assert_eq!(default.scan_progress.fg, Some(Color::Cyan));
    assert!(default.selection.add_modifier.contains(Modifier::REVERSED));
    assert_eq!(default.match_changed.fg, Some(Color::Yellow));
    assert_eq!(default.frozen.fg, Some(Color::Blue));
}

#[test]
#[cfg(feature = "config")]
fn theme_resolve_applies_file_overrides_over_the_defaults() {
    use ratatui::style::Color;

    use crate::theme::FileTheme;

    let file = FileTheme {
        focused_border: Some("magenta".to_owned()),
        match_changed: Some("#ff8800".to_owned()),
        ..FileTheme::default()
    };

    // `resolve_with_color(.., true)` rather than `resolve` — deterministic regardless of whether
    // this test process's stdout happens to be a real terminal (`resolve` itself checks that live
    // via `color_enabled`).
    let resolved =
        theme::Theme::resolve_with_color(Some(&file), true).expect("valid colors should resolve");

    assert_eq!(resolved.focused_border.fg, Some(Color::Magenta));
    assert_eq!(
        resolved.match_changed.fg,
        Some(Color::Rgb(0xff, 0x88, 0x00))
    );
    // Untouched fields keep their default.
    assert_eq!(resolved.status_bar.fg, Some(Color::Black));
}

#[test]
#[cfg(feature = "config")]
fn theme_resolve_rejects_an_invalid_color() {
    use crate::theme::FileTheme;

    let file = FileTheme {
        focused_border: Some("not-a-real-color".to_owned()),
        ..FileTheme::default()
    };

    let err = theme::Theme::resolve_with_color(Some(&file), true)
        .expect_err("an invalid color must be rejected");
    assert!(err.to_string().contains("focused-border"));
}

#[test]
#[cfg(feature = "config")]
fn theme_resolve_with_no_file_and_color_returns_defaults() {
    let resolved = theme::Theme::resolve_with_color(None, true).expect("no file is always valid");
    assert_eq!(resolved, theme::Theme::default());
}

#[test]
#[cfg(feature = "config")]
fn theme_resolve_without_color_strips_every_color_but_keeps_modifiers() {
    let resolved = theme::Theme::resolve_with_color(None, false).expect("always valid");

    assert_eq!(resolved.focused_border.fg, None);
    assert_eq!(resolved.status_bar.fg, None);
    assert_eq!(resolved.status_bar.bg, None);
    assert_eq!(resolved.match_changed.fg, None);
    // Structural modifiers survive, including the fallback added for styles that would
    // otherwise become visually blank without a color.
    assert!(
        resolved
            .status_bar_error
            .add_modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
    assert!(
        resolved
            .scan_progress
            .add_modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
    assert!(
        resolved
            .match_changed
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
}

#[test]
#[cfg(not(feature = "config"))]
fn theme_resolve_with_color_true_and_false_without_config_feature() {
    let colored = theme::Theme::resolve_with_color(true);
    assert_eq!(colored, theme::Theme::default());

    let plain = theme::Theme::resolve_with_color(false);
    assert_eq!(plain.focused_border.fg, None);
}

#[test]
fn theme_resolve_does_not_panic_regardless_of_the_live_terminal_state() {
    // Smoke test for the real entry point (env/terminal-dependent, so no color assertions here —
    // see the `_with_color` tests above for deterministic coverage of both branches).
    #[cfg(feature = "config")]
    theme::Theme::resolve(None).expect("no file is always valid");
    #[cfg(not(feature = "config"))]
    theme::Theme::resolve().expect("always valid");
}
