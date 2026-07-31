//! Top-level frame layout: renders the empty body area and a one-line status bar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};

use crate::app::AppState;

/// Renders the current frame: an empty body area plus a status bar with the quit hint.
pub fn render(frame: &mut Frame, state: &AppState) {
    let _ = state;

    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());

    frame.render_widget(Block::default(), chunks[0]);
    frame.render_widget(
        Paragraph::new("gameconqueror — Ctrl+Q to quit")
            .style(Style::default().fg(Color::Black).bg(Color::Gray)),
        chunks[1],
    );
}
