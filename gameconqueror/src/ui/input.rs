//! Translates a `crossterm::event::KeyEvent` into a [`Msg`] via `ui/keymap.rs` and applies it —
//! the only place in `ui/` that reads raw key events.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{self, AppState, Focus, Msg};
use crate::ui::keymap;

/// Handles one key event against `state`.
///
/// While the Process Picker's search box is active, printable keys and backspace edit the
/// filter query directly (composing free-form text can't be expressed as a fixed key table);
/// everything else goes through `ui/keymap.rs`, falling back to `Enter` attaching to the
/// currently selected process.
pub fn handle_key(state: &mut AppState, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if state.focus() == Focus::ProcessPicker
        && state.search_active()
        && let Some(msg) = process_search_msg(state, key)
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
    }
}

/// Builds the `Msg` for a key pressed while the Process Picker's search box is active, or `None`
/// if `key` should fall through to the normal global/per-focus lookup instead (`Esc`, `Tab`, …).
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
        KeyCode::Up => Some(Msg::SelectPrev),
        KeyCode::Down => Some(Msg::SelectNext),
        _ => None,
    }
}
