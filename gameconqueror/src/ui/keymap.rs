//! Fixed `(Focus, KeyEvent) -> Msg` lookup tables: global bindings active regardless of focus,
//! and per-focus bindings layered on top. `ui/input.rs` is the only caller; bindings that need
//! data from `AppState` itself (the currently selected process, an in-progress search query) are
//! resolved there instead of here.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Focus, Msg};

/// Looks up a binding active regardless of the current focus.
pub fn lookup_global(key: KeyEvent) -> Option<Msg> {
    match (key.code, key.modifiers) {
        (KeyCode::Tab, KeyModifiers::NONE) => Some(Msg::FocusNext),
        (KeyCode::BackTab, _) => Some(Msg::FocusPrev),
        (KeyCode::Char('?'), KeyModifiers::NONE) => Some(Msg::ShowHelp),
        (KeyCode::F(1), _) => Some(Msg::ShowHelp),
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => Some(Msg::Quit),
        #[cfg(feature = "cheat-list")]
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(Msg::SaveCheatList),
        #[cfg(feature = "cheat-list")]
        (KeyCode::Char('l'), KeyModifiers::CONTROL) => Some(Msg::LoadCheatList),
        (KeyCode::Esc, _) => Some(Msg::Dismiss),
        _ => None,
    }
}

/// Looks up a binding scoped to `focus`, on top of the global bindings.
pub fn lookup_focus(focus: Focus, key: KeyEvent) -> Option<Msg> {
    match focus {
        Focus::ProcessPicker => match (key.code, key.modifiers) {
            (KeyCode::Up, KeyModifiers::NONE) => Some(Msg::SelectPrev),
            (KeyCode::Down, KeyModifiers::NONE) => Some(Msg::SelectNext),
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(Msg::ToggleSearch),
            _ => None,
        },
        Focus::ScanPanel => match (key.code, key.modifiers) {
            (KeyCode::Char('t'), KeyModifiers::NONE) => Some(Msg::CycleScanDataType),
            (KeyCode::Char('m'), KeyModifiers::NONE) => Some(Msg::CycleScanMatchType),
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(Msg::ToggleSearch),
            (KeyCode::Char('s'), KeyModifiers::NONE) => Some(Msg::RunScan),
            (KeyCode::Char('n'), KeyModifiers::NONE) => Some(Msg::Snapshot),
            (KeyCode::Char('r'), KeyModifiers::NONE) => Some(Msg::ResetScan),
            _ => None,
        },
        Focus::MatchView => match (key.code, key.modifiers) {
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                Some(Msg::SelectPrev)
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                Some(Msg::SelectNext)
            }
            (KeyCode::Char('o'), KeyModifiers::NONE) => Some(Msg::CycleMatchSort),
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(Msg::ToggleSearch),
            _ => None,
        },
        #[cfg(feature = "cheat-list")]
        Focus::CheatView => match (key.code, key.modifiers) {
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                Some(Msg::SelectPrev)
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                Some(Msg::SelectNext)
            }
            _ => None,
        },
        Focus::HexView => None,
    }
}
