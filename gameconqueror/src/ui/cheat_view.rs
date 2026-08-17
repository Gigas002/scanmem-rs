//! Cheat View panel: a `Table` over the recorded cheat list — freeze toggling and inline value
//! editing are driven entirely by keys (`ui/keymap.rs`/`ui/input.rs` own the actual bindings,
//! since they need `AppState` to resolve the selected row); this module only renders it.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

use crate::app::AppState;
use crate::ui::panel_border_style;
use crate::ui::theme::theme;

/// Renders the cheat table into `area`: one row per recorded cheat, with the selected row
/// highlighted, a currently-frozen cheat's Frozen cell styled with `theme().frozen`, and the
/// title showing an in-progress value edit, if any. `focused` highlights the panel border when
/// it's the grid's (or expanded view's) current focus.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let cheats = state.cheats();

    let rows = cheats.iter().map(|entry| {
        let frozen_style = if entry.frozen {
            theme().frozen
        } else {
            Style::default()
        };
        Row::new([
            Cell::new(format!("{:#x}", entry.address)),
            Cell::new(entry.description.clone()),
            Cell::new(entry.value.to_string()),
            Cell::new(if entry.frozen { "yes" } else { "" }).style(frozen_style),
        ])
    });

    let widths = [
        Constraint::Length(12),
        Constraint::Min(0),
        Constraint::Length(16),
        Constraint::Length(7),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(["Address", "Description", "Value", "Frozen"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .row_highlight_style(theme().selection)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style(focused))
                .title(title(state)),
        );

    let mut table_state = TableState::default();
    if !cheats.is_empty() {
        table_state.select(Some(state.cheat_selected().min(cheats.len() - 1)));
    }

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn title(state: &AppState) -> String {
    if state.search_active() {
        format!("Cheat View — editing value: {}_", state.cheat_value_input())
    } else {
        "Cheat View — space: freeze, e: edit value, h: hex view".to_owned()
    }
}
