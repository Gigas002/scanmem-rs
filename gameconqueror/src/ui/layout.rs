//! Top-level frame layout: renders the focused panel's body plus a one-line status bar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use crate::app::{AppState, Focus, StatusLevel};
use crate::ui::{match_view, process_picker, scan_panel};

/// Renders the current frame: the focused panel's body, then a status bar showing the current
/// focus, the last status message (if any), and a one-line hint of the global bindings.
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());

    match state.focus() {
        Focus::ProcessPicker => process_picker::render(frame, chunks[0], state),
        Focus::ScanPanel => scan_panel::render(frame, chunks[0], state),
        Focus::MatchView => match_view::render(frame, chunks[0], state),
        other => frame.render_widget(placeholder(other), chunks[0]),
    }

    frame.render_widget(status_bar(state), chunks[1]);
}

fn placeholder(focus: Focus) -> Paragraph<'static> {
    Paragraph::new(format!("{focus} — not yet implemented"))
}

fn status_bar(state: &AppState) -> Paragraph<'_> {
    let mut text = format!(
        "[{}] Tab: next panel · ?: help · Ctrl+Q: quit",
        state.focus()
    );
    let mut style = Style::default().fg(Color::Black).bg(Color::Gray);

    if let Some(status) = state.status() {
        text = format!("{text} — {}", status.text);
        if status.level == StatusLevel::Error {
            style = Style::default().fg(Color::White).bg(Color::Red);
        }
    }

    Paragraph::new(text).style(style)
}
