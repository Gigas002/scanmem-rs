//! `?`/`F1` help overlay: a centered popup listing every binding active for the focused panel,
//! read straight from `ui/keymap.rs`'s global/per-focus/dynamic binding tables so it can never
//! drift from what actually works.

use std::collections::HashSet;

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Clear, Row, Table};

use crate::app::Focus;
use crate::ui::keymap;

/// Renders the help overlay for `focus`, centered within `frame`'s full area.
pub fn render(frame: &mut Frame, focus: Focus) {
    let area = centered_rect(frame.area(), 70, 80);

    let rows: Vec<Row> = entries(focus)
        .into_iter()
        .map(|(label, description)| Row::new([Cell::new(label), Cell::new(description)]))
        .collect();

    let widths = [Constraint::Length(12), Constraint::Min(0)];
    let table = Table::new(rows, widths)
        .header(Row::new(["Key", "Action"]).style(Style::default().add_modifier(Modifier::BOLD)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Help — {focus} (Esc to close)")),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(table, area);
}

/// Every `(label, description)` pair active for `focus`: global bindings, per-focus bindings, and
/// the dynamic ones resolved in `ui/input.rs`, deduplicated — `Up` and `k` both move the
/// selection in some panels, for example, and share one row here.
fn entries(focus: Focus) -> Vec<(&'static str, &'static str)> {
    let mut seen = HashSet::new();
    keymap::global_bindings()
        .iter()
        .map(|binding| (binding.label, binding.description))
        .chain(
            keymap::focus_bindings(focus)
                .iter()
                .map(|binding| (binding.label, binding.description)),
        )
        .chain(
            keymap::dynamic_bindings(focus)
                .iter()
                .map(|binding| (binding.label, binding.description)),
        )
        .filter(|entry| seen.insert(*entry))
        .collect()
}

/// A `width_pct`/`height_pct`-sized `Rect` centered within `area`.
fn centered_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let width = area.width * width_pct / 100;
    let height = area.height * height_pct / 100;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
