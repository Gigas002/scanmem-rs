//! Match View panel: a `Table` over the attached session's current match set, sortable and
//! filterable via keys, with all sort/filter/selection state stored in `AppState` — this module
//! only renders it each frame, per the immediate-mode approach `ratatui::widgets::Table` requires.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

use crate::app::{AppState, MatchSortColumn};
use crate::ui::panel_border_style;

/// Renders the match table into `area`: one row per recorded match passing the current filter,
/// with the selected row highlighted and the title showing the active sort column and
/// filter/search prompt. `focused` highlights the panel border when it's the grid's (or expanded
/// view's) current focus.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let matches = state.filtered_matches();

    let rows = matches.iter().map(|entry| {
        Row::new([
            Cell::new(format!("{:#x}", entry.address)),
            Cell::new(entry.old_value.to_string()),
        ])
    });

    let widths = [Constraint::Length(18), Constraint::Min(0)];
    let table = Table::new(rows, widths)
        .header(Row::new(["Address", "Value"]).style(Style::default().add_modifier(Modifier::BOLD)))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style(focused))
                .title(title(state)),
        );

    let mut table_state = TableState::default();
    if !matches.is_empty() {
        table_state.select(Some(state.match_selected().min(matches.len() - 1)));
    }

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn title(state: &AppState) -> String {
    let sort = match state.match_sort() {
        MatchSortColumn::Address => "address",
        MatchSortColumn::Value => "value",
    };

    if state.search_active() {
        format!(
            "Match View — sort: {sort}, search: {}_",
            state.match_filter()
        )
    } else if state.match_filter().is_empty() {
        format!("Match View — sort: {sort} · o: sort, /: filter")
    } else {
        format!(
            "Match View — sort: {sort}, filter: {}",
            state.match_filter()
        )
    }
}
