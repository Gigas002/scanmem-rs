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
    #[cfg(feature = "hex-view")]
    HexView,
}

/// Cycling order for [`Focus::next`]/[`Focus::prev`], built at call time (not a fixed-size const
/// array) so it scales cleanly across every combination of the `cheat-list`/`hex-view` features
/// instead of needing one array per combination.
fn order() -> Vec<Focus> {
    #[allow(unused_mut)]
    let mut order = vec![Focus::ProcessPicker, Focus::ScanPanel, Focus::MatchView];
    #[cfg(feature = "cheat-list")]
    order.push(Focus::CheatView);
    #[cfg(feature = "hex-view")]
    order.push(Focus::HexView);
    order
}

impl Focus {
    /// The next panel in cycling order, wrapping back to `ProcessPicker`.
    #[must_use]
    pub fn next(self) -> Self {
        let order = order();
        let index = order.iter().position(|&focus| focus == self).unwrap_or(0);
        order[(index + 1) % order.len()]
    }

    /// The previous panel in cycling order, wrapping back to the last panel.
    #[must_use]
    pub fn prev(self) -> Self {
        let order = order();
        let index = order.iter().position(|&focus| focus == self).unwrap_or(0);
        order[(index + order.len() - 1) % order.len()]
    }

    /// The panel spatially adjacent to `self` in `dir`, per the fixed grid `ui/layout.rs` renders
    /// (top row: Process Picker | Scan Panel; middle row: Match View | Cheat View — or just Match
    /// View without the `cheat-list` feature; bottom row: Hex View, spanning the full width, if
    /// built with the `hex-view` feature). Returns `self` unchanged if there is no panel in that
    /// direction, so `Msg::FocusDirection` can assign the result unconditionally without an extra
    /// `Option` dance.
    ///
    /// Each rule is gated individually rather than duplicating the whole match per feature
    /// combination — a rule mentioning `CheatView`/`HexView` simply doesn't exist when that
    /// variant doesn't, and falls through to whatever rule (if any) is left for that `(self, dir)`
    /// pair.
    #[must_use]
    pub fn towards(self, dir: Direction) -> Self {
        use Direction::{Down, Left, Right, Up};

        let target = match (self, dir) {
            (Focus::ProcessPicker, Right) => Some(Focus::ScanPanel),
            (Focus::ProcessPicker, Down) => Some(Focus::MatchView),
            (Focus::ScanPanel, Left) => Some(Focus::ProcessPicker),
            #[cfg(feature = "cheat-list")]
            (Focus::ScanPanel, Down) => Some(Focus::CheatView),
            #[cfg(not(feature = "cheat-list"))]
            (Focus::ScanPanel, Down) => Some(Focus::MatchView),
            (Focus::MatchView, Up) => Some(Focus::ProcessPicker),
            #[cfg(feature = "cheat-list")]
            (Focus::MatchView, Right) => Some(Focus::CheatView),
            #[cfg(feature = "hex-view")]
            (Focus::MatchView, Down) => Some(Focus::HexView),
            #[cfg(feature = "cheat-list")]
            (Focus::CheatView, Up) => Some(Focus::ScanPanel),
            #[cfg(feature = "cheat-list")]
            (Focus::CheatView, Left) => Some(Focus::MatchView),
            #[cfg(all(feature = "cheat-list", feature = "hex-view"))]
            (Focus::CheatView, Down) => Some(Focus::HexView),
            #[cfg(feature = "hex-view")]
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
            #[cfg(feature = "hex-view")]
            Focus::HexView => "Hex View",
        };
        f.write_str(name)
    }
}
