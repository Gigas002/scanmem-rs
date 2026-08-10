//! Process Picker panel: a `Table` over `/proc`, filtered incrementally via `/`.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

use crate::app::AppState;
use crate::ui::panel_border_style;

/// Renders the process table into `area`: one row per pid/name pair passing the current filter,
/// with the selected row highlighted and the title showing the active filter or search prompt.
/// `focused` highlights the panel border when it's the grid's (or expanded view's) current focus.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let processes = state.filtered_processes();

    let rows = processes.iter().map(|process| {
        Row::new([
            Cell::new(process.pid.to_string()),
            Cell::new(process.name.clone()),
        ])
    });

    let widths = [Constraint::Length(8), Constraint::Min(0)];
    let table = Table::new(rows, widths)
        .header(Row::new(["PID", "Name"]).style(Style::default().add_modifier(Modifier::BOLD)))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style(focused))
                .title(title(state)),
        );

    let mut table_state = TableState::default();
    if !processes.is_empty() {
        table_state.select(Some(state.process_selected().min(processes.len() - 1)));
    }

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn title(state: &AppState) -> String {
    if state.search_active() {
        format!("Process Picker — search: {}_", state.process_filter())
    } else if state.process_filter().is_empty() {
        "Process Picker — Enter: attach, /: search".to_owned()
    } else {
        format!("Process Picker — filter: {}", state.process_filter())
    }
}
