use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::install_panic_hook;
use super::layout::render;
use super::{input, keymap, process_picker};
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
