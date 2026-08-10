//! `Focus` — which panel currently receives keyboard input, and the fixed order
//! `Msg::FocusNext`/`Msg::FocusPrev` cycle through.

/// The panel that currently receives keyboard input; every other key binding is interpreted
/// relative to this.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Focus {
    #[default]
    ProcessPicker,
    ScanPanel,
    MatchView,
    #[cfg(feature = "cheat-list")]
    CheatView,
    HexView,
}

/// Fixed cycling order for [`Focus::next`]/[`Focus::prev`].
#[cfg(feature = "cheat-list")]
const ORDER: [Focus; 5] = [
    Focus::ProcessPicker,
    Focus::ScanPanel,
    Focus::MatchView,
    Focus::CheatView,
    Focus::HexView,
];

/// Fixed cycling order for [`Focus::next`]/[`Focus::prev`].
#[cfg(not(feature = "cheat-list"))]
const ORDER: [Focus; 4] = [
    Focus::ProcessPicker,
    Focus::ScanPanel,
    Focus::MatchView,
    Focus::HexView,
];

impl Focus {
    /// The next panel in cycling order, wrapping from `HexView` back to `ProcessPicker`.
    #[must_use]
    pub fn next(self) -> Self {
        let index = ORDER.iter().position(|&focus| focus == self).unwrap_or(0);
        ORDER[(index + 1) % ORDER.len()]
    }

    /// The previous panel in cycling order, wrapping from `ProcessPicker` back to `HexView`.
    #[must_use]
    pub fn prev(self) -> Self {
        let index = ORDER.iter().position(|&focus| focus == self).unwrap_or(0);
        ORDER[(index + ORDER.len() - 1) % ORDER.len()]
    }

    /// The panel spatially adjacent to `self` in `dir`, per the fixed grid `ui/layout.rs` renders
    /// (top row: Process Picker | Scan Panel; middle row: Match View | Cheat View — or just Match
    /// View without the `cheat-list` feature; bottom row: Hex View, spanning the full width).
    /// Returns `self` unchanged if there is no panel in that direction, so `Msg::FocusDirection`
    /// can assign the result unconditionally without an extra `Option` dance.
    #[must_use]
    pub fn towards(self, dir: Direction) -> Self {
        use Direction::{Down, Left, Right, Up};

        #[cfg(feature = "cheat-list")]
        let target = match (self, dir) {
            (Focus::ProcessPicker, Right) => Some(Focus::ScanPanel),
            (Focus::ProcessPicker, Down) => Some(Focus::MatchView),
            (Focus::ScanPanel, Left) => Some(Focus::ProcessPicker),
            (Focus::ScanPanel, Down) => Some(Focus::CheatView),
            (Focus::MatchView, Up) => Some(Focus::ProcessPicker),
            (Focus::MatchView, Right) => Some(Focus::CheatView),
            (Focus::MatchView, Down) => Some(Focus::HexView),
            (Focus::CheatView, Up) => Some(Focus::ScanPanel),
            (Focus::CheatView, Left) => Some(Focus::MatchView),
            (Focus::CheatView, Down) => Some(Focus::HexView),
            (Focus::HexView, Up) => Some(Focus::MatchView),
            _ => None,
        };

        #[cfg(not(feature = "cheat-list"))]
        let target = match (self, dir) {
            (Focus::ProcessPicker, Right) => Some(Focus::ScanPanel),
            (Focus::ProcessPicker, Down) => Some(Focus::MatchView),
            (Focus::ScanPanel, Left) => Some(Focus::ProcessPicker),
            (Focus::ScanPanel, Down) => Some(Focus::MatchView),
            (Focus::MatchView, Up) => Some(Focus::ProcessPicker),
            (Focus::MatchView, Down) => Some(Focus::HexView),
            (Focus::HexView, Up) => Some(Focus::MatchView),
            _ => None,
        };

        target.unwrap_or(self)
    }
}

/// A navigation direction for `Msg::FocusDirection`/[`Focus::towards`], driven by `Ctrl+<Arrow>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for Focus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Focus::ProcessPicker => "Process Picker",
            Focus::ScanPanel => "Scan Panel",
            Focus::MatchView => "Match View",
            #[cfg(feature = "cheat-list")]
            Focus::CheatView => "Cheat View",
            Focus::HexView => "Hex View",
        };
        f.write_str(name)
    }
}
