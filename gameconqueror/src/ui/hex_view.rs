//! Hex View panel: a 3-pane offset/hex/ASCII view over the bytes loaded into `AppState`'s hex
//! buffer, with the cursor and any in-progress byte edit rendered as styled `Span`s per cell —
//! `ui/keymap.rs`/`ui/input.rs` own cursor movement and edit input, this module only renders it.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::app::AppState;

/// Bytes shown per row; also the byte distance `ui/keymap.rs` moves the cursor on `Up`/`Down`.
pub const BYTES_PER_ROW: usize = 16;

/// Renders the hex table into `area`: one row per [`BYTES_PER_ROW`] bytes of the loaded buffer,
/// with the cursor byte highlighted in both the hex and ASCII columns.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let buffer = state.hex_buffer();
    let base = state.hex_base_address();
    let cursor = state.hex_cursor();
    let edit_input = state.hex_edit_input();

    let rows = buffer
        .chunks(BYTES_PER_ROW)
        .enumerate()
        .map(|(row, chunk)| {
            let row_start = row * BYTES_PER_ROW;
            let offset = Cell::new(format!("{:#010x}", base + row_start));

            let mut hex_spans = Vec::with_capacity(chunk.len() * 2);
            let mut ascii_spans = Vec::with_capacity(chunk.len());
            for (column, &byte) in chunk.iter().enumerate() {
                let index = row_start + column;
                let is_cursor = index == cursor;
                let style = if is_cursor {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };

                let hex_text = if is_cursor && !edit_input.is_empty() {
                    format!("{edit_input:<2}")
                } else {
                    format!("{byte:02x}")
                };
                hex_spans.push(Span::styled(hex_text, style));
                hex_spans.push(Span::raw(" "));

                let ascii_char = if byte.is_ascii_graphic() || byte == b' ' {
                    byte as char
                } else {
                    '.'
                };
                ascii_spans.push(Span::styled(ascii_char.to_string(), style));
            }

            Row::new([
                offset,
                Cell::from(Line::from(hex_spans)),
                Cell::from(Line::from(ascii_spans)),
            ])
        });

    let widths = [
        Constraint::Length(12),
        Constraint::Length((BYTES_PER_ROW * 3) as u16),
        Constraint::Length(BYTES_PER_ROW as u16),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(["Offset", "Hex", "ASCII"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(title(state)));

    frame.render_widget(table, area);
}

fn title(state: &AppState) -> String {
    match state.hex_cursor_address() {
        Some(address) if !state.hex_edit_input().is_empty() => {
            format!(
                "Hex View @ {address:#x} — editing byte: {}_",
                state.hex_edit_input()
            )
        }
        Some(address) => format!("Hex View @ {address:#x} — type hex digits, Enter to write"),
        None => "Hex View — h: open at the selected match/cheat".to_owned(),
    }
}
