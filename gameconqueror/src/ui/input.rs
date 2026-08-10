//! Translates a `crossterm::event::KeyEvent` into a [`Msg`] via `ui/keymap.rs` and applies it —
//! the only place in `ui/` that reads raw key events.

#[cfg(feature = "cheat-list")]
use libscanmem::value::Value;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{self, AppState, Focus, Msg};
use crate::ui::keymap;

/// Handles one key event against `state`.
///
/// While the focused panel's text field is active (the Process Picker's filter, the Scan
/// Panel's value/range input, or the Match View's filter), printable keys and backspace edit
/// that field directly (composing free-form text can't be expressed as a fixed key table);
/// everything else goes through `ui/keymap.rs`, falling back to `Enter` attaching to the
/// currently selected process.
pub fn handle_key(state: &mut AppState, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    #[cfg(feature = "cheat-list")]
    if state.path_prompt().is_some() {
        path_prompt_key(state, key);
        return;
    }

    if state.search_active()
        && let Some(msg) = search_input_msg(state, key)
    {
        app::update(state, msg);
        return;
    }

    if let Some(msg) =
        keymap::lookup_global(key).or_else(|| keymap::lookup_focus(state.focus(), key))
    {
        app::update(state, msg);
        return;
    }

    if state.focus() == Focus::ProcessPicker
        && key.code == KeyCode::Enter
        && let Some(pid) = state.selected_process().map(|process| process.pid)
    {
        app::update(state, Msg::Attach(pid));
        return;
    }

    if let Some(msg) = hex_view_edit_msg(state, key) {
        app::update(state, msg);
        return;
    }

    #[cfg_attr(not(feature = "cheat-list"), allow(clippy::needless_return))]
    if let Some(msg) = match_view_focus_hex_msg(state, key) {
        app::update(state, msg);
        return;
    }

    #[cfg(feature = "cheat-list")]
    if let Some(msg) = cheat_view_focus_hex_msg(state, key) {
        app::update(state, msg);
        return;
    }

    #[cfg(feature = "cheat-list")]
    if let Some(msg) = cheat_view_action_msg(state, key) {
        app::update(state, msg);
        return;
    }

    #[cfg(feature = "cheat-list")]
    if let Some(msg) = match_view_add_cheat_msg(state, key) {
        app::update(state, msg);
    }
}

/// Handles one key event while a cheat-list save/load path prompt is open, capturing every key
/// exclusively (unlike the per-focus text fields below, nothing falls through to the global/
/// per-focus keymap while a prompt is open).
#[cfg(feature = "cheat-list")]
fn path_prompt_key(state: &mut AppState, key: KeyEvent) {
    let msg = match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut input = state.path_input().to_owned();
            input.push(c);
            Some(Msg::SetPathInput(input))
        }
        KeyCode::Backspace => {
            let mut input = state.path_input().to_owned();
            input.pop();
            Some(Msg::SetPathInput(input))
        }
        KeyCode::Enter => Some(Msg::ConfirmPathPrompt),
        KeyCode::Esc => Some(Msg::Dismiss),
        _ => None,
    };

    if let Some(msg) = msg {
        app::update(state, msg);
    }
}

/// Builds the `Msg` for `Space`/`e` pressed on the Cheat View outside of value-edit mode,
/// resolving the currently selected row from `AppState` — `ui/keymap.rs` can't do this since its
/// lookups have no `AppState` access.
#[cfg(feature = "cheat-list")]
fn cheat_view_action_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    if state.focus() != Focus::CheatView {
        return None;
    }
    let index = state.cheat_selected();
    match key.code {
        KeyCode::Char(' ') => Some(Msg::ToggleFreeze(index)),
        KeyCode::Char('e') => Some(Msg::BeginEditCheatValue(index)),
        _ => None,
    }
}

/// Builds the `Msg` for `a` pressed on the Match View, adding the currently selected match to the
/// cheat list (same `AppState`-access reasoning as [`cheat_view_action_msg`]).
#[cfg(feature = "cheat-list")]
fn match_view_add_cheat_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    if state.focus() != Focus::MatchView || key.code != KeyCode::Char('a') {
        return None;
    }
    state.selected_match().map(|entry| Msg::AddCheat {
        address: entry.address,
        description: String::new(),
        value: Value::U8(entry.old_value),
    })
}

/// Builds the `Msg` for a hex-digit or `Backspace` key pressed while the Hex View is focused,
/// composing the in-progress byte edit at the cursor — `ui/keymap.rs` can't do this since it
/// needs the current [`AppState::hex_edit_input`] to append/remove a digit. `None` for any other
/// key so it falls through to the normal global/per-focus lookup (arrows, `Enter`, `Esc`).
fn hex_view_edit_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    if state.focus() != Focus::HexView {
        return None;
    }
    match key.code {
        KeyCode::Char(c) if c.is_ascii_hexdigit() && state.hex_edit_input().len() < 2 => {
            let mut input = state.hex_edit_input().to_owned();
            input.push(c);
            Some(Msg::SetHexEditInput(input))
        }
        KeyCode::Backspace => {
            let mut input = state.hex_edit_input().to_owned();
            input.pop();
            Some(Msg::SetHexEditInput(input))
        }
        _ => None,
    }
}

/// Builds the `Msg` for `h` pressed on the Match View, focusing the Hex View on the selected
/// match's address (same `AppState`-access reasoning as [`match_view_add_cheat_msg`]).
fn match_view_focus_hex_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    if state.focus() != Focus::MatchView || key.code != KeyCode::Char('h') {
        return None;
    }
    state
        .selected_match()
        .map(|entry| Msg::FocusHexView(entry.address))
}

/// Builds the `Msg` for `h` pressed on the Cheat View, focusing the Hex View on the selected
/// cheat's address (same `AppState`-access reasoning as [`match_view_add_cheat_msg`]).
#[cfg(feature = "cheat-list")]
fn cheat_view_focus_hex_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    if state.focus() != Focus::CheatView || key.code != KeyCode::Char('h') {
        return None;
    }
    state
        .selected_cheat()
        .map(|entry| Msg::FocusHexView(entry.address))
}

/// Builds the `Msg` for a key pressed while the focused panel's text field is active, or `None`
/// if `key` should fall through to the normal global/per-focus lookup instead (`Esc`, `Tab`, …).
fn search_input_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    match state.focus() {
        Focus::ProcessPicker => process_search_msg(state, key),
        Focus::ScanPanel => scan_input_msg(state, key),
        Focus::MatchView => match_filter_msg(state, key),
        #[cfg(feature = "cheat-list")]
        Focus::CheatView => cheat_value_msg(state, key),
        Focus::HexView => None,
    }
}

/// Builds the `Msg` for a key pressed while the Process Picker's search box is active.
fn process_search_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut query = state.process_filter().to_owned();
            query.push(c);
            Some(Msg::FilterProcesses(query))
        }
        KeyCode::Backspace => {
            let mut query = state.process_filter().to_owned();
            query.pop();
            Some(Msg::FilterProcesses(query))
        }
        KeyCode::Enter => Some(Msg::ToggleSearch),
        // Plain (not Ctrl+) arrows only — Ctrl+Up/Down must fall through to the global keymap's
        // FocusDirection bindings instead of being swallowed here as a selection move.
        KeyCode::Up if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectPrev),
        KeyCode::Down if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectNext),
        _ => None,
    }
}

/// Builds the `Msg` for a key pressed while the Scan Panel's value/range input is active.
fn scan_input_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut input = state.scan_input().to_owned();
            input.push(c);
            Some(Msg::SetScanInput(input))
        }
        KeyCode::Backspace => {
            let mut input = state.scan_input().to_owned();
            input.pop();
            Some(Msg::SetScanInput(input))
        }
        KeyCode::Enter => Some(Msg::ToggleSearch),
        _ => None,
    }
}

/// Builds the `Msg` for a key pressed while the Match View's filter box is active.
fn match_filter_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut query = state.match_filter().to_owned();
            query.push(c);
            Some(Msg::FilterMatches(query))
        }
        KeyCode::Backspace => {
            let mut query = state.match_filter().to_owned();
            query.pop();
            Some(Msg::FilterMatches(query))
        }
        KeyCode::Enter => Some(Msg::ToggleSearch),
        // See the comment in `process_search_msg` — same Ctrl+Up/Down carve-out.
        KeyCode::Up if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectPrev),
        KeyCode::Down if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectNext),
        _ => None,
    }
}

/// Builds the `Msg` for a key pressed while the Cheat View's value-edit field is active.
#[cfg(feature = "cheat-list")]
fn cheat_value_msg(state: &AppState, key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut input = state.cheat_value_input().to_owned();
            input.push(c);
            Some(Msg::SetCheatValueInput(input))
        }
        KeyCode::Backspace => {
            let mut input = state.cheat_value_input().to_owned();
            input.pop();
            Some(Msg::SetCheatValueInput(input))
        }
        KeyCode::Enter => Some(Msg::ConfirmCheatValueEdit),
        _ => None,
    }
}
