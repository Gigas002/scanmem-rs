//! Fixed `(Focus, KeyEvent) -> Msg` lookup tables: global bindings active regardless of focus,
//! and per-focus bindings layered on top. `ui/input.rs` is the only caller for dispatch;
//! `ui/help_overlay.rs` renders straight from the same tables (plus [`dynamic_bindings`], for the
//! handful of bindings that need `AppState` to resolve their `Msg` and so live in `ui/input.rs`
//! instead) so the two can never drift apart.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Focus, Msg};
use crate::ui::hex_view::BYTES_PER_ROW;

/// One documented, statically dispatchable key binding.
pub struct Binding {
    key: (KeyCode, KeyModifiers),
    msg: Msg,
    pub label: &'static str,
    pub description: &'static str,
}

/// One documented binding whose `Msg` needs `AppState` to resolve (e.g. the selected row) and so
/// is actually dispatched from `ui/input.rs`; listed here purely for [`dynamic_bindings`] callers
/// like `ui/help_overlay.rs`.
pub struct DynamicBinding {
    pub label: &'static str,
    pub description: &'static str,
}

/// Bindings active regardless of the current focus.
pub fn global_bindings() -> Vec<Binding> {
    #[cfg_attr(not(feature = "cheat-list"), allow(unused_mut))]
    let mut bindings = vec![
        Binding {
            key: (KeyCode::Tab, KeyModifiers::NONE),
            msg: Msg::FocusNext,
            label: "Tab",
            description: "focus next panel",
        },
        Binding {
            key: (KeyCode::BackTab, KeyModifiers::SHIFT),
            msg: Msg::FocusPrev,
            label: "Shift+Tab",
            description: "focus previous panel",
        },
        Binding {
            key: (KeyCode::Char('?'), KeyModifiers::NONE),
            msg: Msg::ShowHelp,
            label: "?",
            description: "toggle this help overlay",
        },
        Binding {
            key: (KeyCode::F(1), KeyModifiers::NONE),
            msg: Msg::ShowHelp,
            label: "F1",
            description: "toggle this help overlay",
        },
        Binding {
            key: (KeyCode::Char('q'), KeyModifiers::CONTROL),
            msg: Msg::Quit,
            label: "Ctrl+Q",
            description: "quit",
        },
        Binding {
            key: (KeyCode::Esc, KeyModifiers::NONE),
            msg: Msg::Dismiss,
            label: "Esc",
            description: "dismiss a popup, or cancel an active search/edit",
        },
        Binding {
            key: (KeyCode::Char('d'), KeyModifiers::CONTROL),
            msg: Msg::Detach,
            label: "Ctrl+D",
            description: "detach from the current process, resuming its execution",
        },
    ];

    #[cfg(feature = "cheat-list")]
    bindings.extend([
        Binding {
            key: (KeyCode::Char('s'), KeyModifiers::CONTROL),
            msg: Msg::SaveCheatList,
            label: "Ctrl+S",
            description: "save the cheat list",
        },
        Binding {
            key: (KeyCode::Char('l'), KeyModifiers::CONTROL),
            msg: Msg::LoadCheatList,
            label: "Ctrl+L",
            description: "load a cheat list",
        },
    ]);

    bindings
}

/// Looks up a binding active regardless of the current focus.
pub fn lookup_global(key: KeyEvent) -> Option<Msg> {
    global_bindings()
        .into_iter()
        .find(|binding| binding.key == (key.code, key.modifiers))
        .map(|binding| binding.msg)
}

/// Bindings scoped to `focus`, on top of the global bindings.
pub fn focus_bindings(focus: Focus) -> Vec<Binding> {
    match focus {
        Focus::ProcessPicker => vec![
            Binding {
                key: (KeyCode::Up, KeyModifiers::NONE),
                msg: Msg::SelectPrev,
                label: "Up",
                description: "move selection up",
            },
            Binding {
                key: (KeyCode::Down, KeyModifiers::NONE),
                msg: Msg::SelectNext,
                label: "Down",
                description: "move selection down",
            },
            Binding {
                key: (KeyCode::Char('/'), KeyModifiers::NONE),
                msg: Msg::ToggleSearch,
                label: "/",
                description: "search by pid or name",
            },
        ],
        Focus::ScanPanel => vec![
            Binding {
                key: (KeyCode::Char('t'), KeyModifiers::NONE),
                msg: Msg::CycleScanDataType,
                label: "t",
                description: "cycle the data type",
            },
            Binding {
                key: (KeyCode::Char('m'), KeyModifiers::NONE),
                msg: Msg::CycleScanMatchType,
                label: "m",
                description: "cycle the match type",
            },
            Binding {
                key: (KeyCode::Char('/'), KeyModifiers::NONE),
                msg: Msg::ToggleSearch,
                label: "/",
                description: "edit the value/range input",
            },
            Binding {
                key: (KeyCode::Char('s'), KeyModifiers::NONE),
                msg: Msg::RunScan,
                label: "s",
                description: "run the scan",
            },
            Binding {
                key: (KeyCode::Char('n'), KeyModifiers::NONE),
                msg: Msg::Snapshot,
                label: "n",
                description: "snapshot every considered byte",
            },
            Binding {
                key: (KeyCode::Char('r'), KeyModifiers::NONE),
                msg: Msg::ResetScan,
                label: "r",
                description: "reset the match set",
            },
        ],
        Focus::MatchView => vec![
            Binding {
                key: (KeyCode::Up, KeyModifiers::NONE),
                msg: Msg::SelectPrev,
                label: "Up/k",
                description: "move selection up",
            },
            Binding {
                key: (KeyCode::Char('k'), KeyModifiers::NONE),
                msg: Msg::SelectPrev,
                label: "Up/k",
                description: "move selection up",
            },
            Binding {
                key: (KeyCode::Down, KeyModifiers::NONE),
                msg: Msg::SelectNext,
                label: "Down/j",
                description: "move selection down",
            },
            Binding {
                key: (KeyCode::Char('j'), KeyModifiers::NONE),
                msg: Msg::SelectNext,
                label: "Down/j",
                description: "move selection down",
            },
            Binding {
                key: (KeyCode::Char('o'), KeyModifiers::NONE),
                msg: Msg::CycleMatchSort,
                label: "o",
                description: "cycle the sort column",
            },
            Binding {
                key: (KeyCode::Char('/'), KeyModifiers::NONE),
                msg: Msg::ToggleSearch,
                label: "/",
                description: "edit the filter",
            },
        ],
        #[cfg(feature = "cheat-list")]
        Focus::CheatView => vec![
            Binding {
                key: (KeyCode::Up, KeyModifiers::NONE),
                msg: Msg::SelectPrev,
                label: "Up/k",
                description: "move selection up",
            },
            Binding {
                key: (KeyCode::Char('k'), KeyModifiers::NONE),
                msg: Msg::SelectPrev,
                label: "Up/k",
                description: "move selection up",
            },
            Binding {
                key: (KeyCode::Down, KeyModifiers::NONE),
                msg: Msg::SelectNext,
                label: "Down/j",
                description: "move selection down",
            },
            Binding {
                key: (KeyCode::Char('j'), KeyModifiers::NONE),
                msg: Msg::SelectNext,
                label: "Down/j",
                description: "move selection down",
            },
        ],
        Focus::HexView => vec![
            Binding {
                key: (KeyCode::Left, KeyModifiers::NONE),
                msg: Msg::MoveHexCursor(-1),
                label: "Left",
                description: "move cursor back one byte",
            },
            Binding {
                key: (KeyCode::Right, KeyModifiers::NONE),
                msg: Msg::MoveHexCursor(1),
                label: "Right",
                description: "move cursor forward one byte",
            },
            Binding {
                key: (KeyCode::Up, KeyModifiers::NONE),
                msg: Msg::MoveHexCursor(-(BYTES_PER_ROW as isize)),
                label: "Up",
                description: "move cursor up one row",
            },
            Binding {
                key: (KeyCode::Down, KeyModifiers::NONE),
                msg: Msg::MoveHexCursor(BYTES_PER_ROW as isize),
                label: "Down",
                description: "move cursor down one row",
            },
            Binding {
                key: (KeyCode::Enter, KeyModifiers::NONE),
                msg: Msg::CommitHexEdit,
                label: "Enter",
                description: "commit the in-progress byte edit",
            },
        ],
    }
}

/// Looks up a binding scoped to `focus`, on top of the global bindings.
pub fn lookup_focus(focus: Focus, key: KeyEvent) -> Option<Msg> {
    focus_bindings(focus)
        .into_iter()
        .find(|binding| binding.key == (key.code, key.modifiers))
        .map(|binding| binding.msg)
}

/// Bindings scoped to `focus` that need `AppState` to resolve their `Msg` (the selected row) and
/// so are dispatched directly from `ui/input.rs`; listed here purely so `ui/help_overlay.rs` has
/// one place to read every active binding from.
pub fn dynamic_bindings(focus: Focus) -> Vec<DynamicBinding> {
    match focus {
        Focus::ProcessPicker => vec![DynamicBinding {
            label: "Enter",
            description: "attach to the selected process",
        }],
        Focus::MatchView => {
            #[cfg_attr(not(feature = "cheat-list"), allow(unused_mut))]
            let mut bindings = vec![DynamicBinding {
                label: "h",
                description: "open hex view at the selected match",
            }];
            #[cfg(feature = "cheat-list")]
            bindings.push(DynamicBinding {
                label: "a",
                description: "add the selected match to the cheat list",
            });
            bindings
        }
        #[cfg(feature = "cheat-list")]
        Focus::CheatView => vec![
            DynamicBinding {
                label: "Space",
                description: "toggle freeze",
            },
            DynamicBinding {
                label: "e",
                description: "edit the value inline",
            },
            DynamicBinding {
                label: "h",
                description: "open hex view at the selected cheat",
            },
        ],
        Focus::ScanPanel | Focus::HexView => Vec::new(),
    }
}
