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
    CheatView,
    HexView,
}

/// Fixed cycling order for [`Focus::next`]/[`Focus::prev`].
const ORDER: [Focus; 5] = [
    Focus::ProcessPicker,
    Focus::ScanPanel,
    Focus::MatchView,
    Focus::CheatView,
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
}

impl std::fmt::Display for Focus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Focus::ProcessPicker => "Process Picker",
            Focus::ScanPanel => "Scan Panel",
            Focus::MatchView => "Match View",
            Focus::CheatView => "Cheat View",
            Focus::HexView => "Hex View",
        };
        f.write_str(name)
    }
}
