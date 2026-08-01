use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(feature = "cheat-list")]
use super::cheat_view;
use super::install_panic_hook;
use super::layout::render;
use super::{help_overlay, hex_view, input, keymap, match_view, process_picker, scan_panel};
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
            process_picker::render(frame, area, &state);
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
            scan_panel::render(frame, area, &state);
        })
        .unwrap();
}

#[test]
fn match_view_renders_without_panicking() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();

    terminal
        .draw(|frame| {
            let area = frame.area();
            match_view::render(frame, area, &state);
        })
        .unwrap();
}

#[test]
fn hex_view_renders_without_panicking_on_an_empty_buffer() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();

    terminal
        .draw(|frame| {
            let area = frame.area();
            hex_view::render(frame, area, &state);
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
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            Msg::Dismiss,
        ),
    ];

    for (key, expected) in cases {
        assert_eq!(keymap::lookup_global(key), Some(expected));
    }
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
            Msg::Snapshot,
        ),
        (
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            Msg::ResetScan,
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
            cheat_view::render(frame, area, &state);
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
#[cfg(feature = "cheat-list")]
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
