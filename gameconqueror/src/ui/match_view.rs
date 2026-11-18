//! Match View panel: a `Table` over the attached session's current match set, sortable and
//! filterable via keys, with all sort/filter/selection state stored in `AppState` — this module
//! only renders it each frame, per the immediate-mode approach `ratatui::widgets::Table` requires.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

use crate::app::{AppState, MatchSortColumn};
use crate::ui::panel_border_style;
use crate::ui::theme::theme;

/// Renders the match table into `area`: one row per recorded match passing the current filter,
/// with the selected row highlighted and the title showing the active sort column and
/// filter/search prompt. A match whose value changed on the most recent scan/refresh
/// (`AppState::match_recently_changed`) gets its value cell styled with `theme().match_changed`;
/// every other value cell is plain, so a value that just moved is the one that visually stands
/// out. `focused` highlights the panel border when it's the grid's (or expanded view's) current
/// focus.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let matches = state.filtered_matches();
    // Only the rows that fit are formatted: borders and the header take three lines.
    let height = usize::from(area.height.saturating_sub(3)).max(1);
    let selected = state.match_selected().min(matches.len().saturating_sub(1));
    let offset = scroll_offset(state.match_offset.get(), selected, height, matches.len());
    state.match_offset.set(offset);
    let visible = &matches[offset..(offset + height).min(matches.len())];

    let rows = visible.iter().map(|entry| {
        let value_style = if state.match_recently_changed(entry.address) {
            theme().match_changed
        } else {
            Style::default()
        };
        Row::new([
            Cell::new(format!("{:#x}", entry.address)),
            Cell::new(entry.old_value.to_string()).style(value_style),
        ])
    });

    let widths = [Constraint::Length(18), Constraint::Min(0)];
    let table = Table::new(rows, widths)
        .header(Row::new(["Address", "Value"]).style(Style::default().add_modifier(Modifier::BOLD)))
        .row_highlight_style(theme().selection)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style(focused))
                .title(title(state)),
        );

    let mut table_state = TableState::default();
    if !matches.is_empty() {
        table_state.select(Some(selected - offset));
    }

    frame.render_stateful_widget(table, area, &mut table_state);
}

/// The first visible row: `offset` moved just enough to keep `selected` within `height` rows,
/// as ratatui's `TableState` scrolls, and clamped so the last page stays full.
pub(crate) fn scroll_offset(offset: usize, selected: usize, height: usize, len: usize) -> usize {
    let offset = if selected < offset {
        selected
    } else if selected >= offset + height {
        selected + 1 - height
    } else {
        offset
    };
    offset.min(len.saturating_sub(height))
}

/// Hint for the keys `ui/input.rs` resolves dynamically (need `AppState` to know the selected
/// row, so they're not in `ui/keymap.rs`'s static tables) — kept alongside the static `o`/`/`
/// hint below so the title never drifts from what's actually bound.
#[cfg(feature = "cheat-list")]
const DYNAMIC_HINT: &str = "h: hex view, a: add cheat";
#[cfg(not(feature = "cheat-list"))]
const DYNAMIC_HINT: &str = "h: hex view";

fn title(state: &AppState) -> String {
    let sort = match state.match_sort() {
        MatchSortColumn::Address => "address",
        MatchSortColumn::Value => "value",
    };

    if state.search_active() {
        return format!(
            "Match View — sort: {sort}, search: {}_",
            state.match_filter()
        );
    }

    if state.match_filter().is_empty() {
        format!("Match View — sort: {sort} · o: sort, /: filter, {DYNAMIC_HINT}")
    } else {
        format!(
            "Match View — sort: {sort}, filter: {} · {DYNAMIC_HINT}",
            state.match_filter()
        )
    }
}
