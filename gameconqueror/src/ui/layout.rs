//! Top-level frame layout: renders the focused panel's body plus a one-line status bar.

use ratatui::Frame;
#[cfg(feature = "cheat-list")]
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
#[cfg(feature = "cheat-list")]
use ratatui::widgets::{Block, Borders, Clear};

#[cfg(feature = "cheat-list")]
use crate::app::PathPromptKind;
use crate::app::{AppState, Focus, StatusLevel};
#[cfg(feature = "cheat-list")]
use crate::ui::cheat_view;
use crate::ui::{match_view, process_picker, scan_panel};

/// Renders the current frame: the focused panel's body, then a status bar showing the current
/// focus, the last status message (if any), and a one-line hint of the global bindings.
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());

    match state.focus() {
        Focus::ProcessPicker => process_picker::render(frame, chunks[0], state),
        Focus::ScanPanel => scan_panel::render(frame, chunks[0], state),
        Focus::MatchView => match_view::render(frame, chunks[0], state),
        #[cfg(feature = "cheat-list")]
        Focus::CheatView => cheat_view::render(frame, chunks[0], state),
        other => frame.render_widget(placeholder(other), chunks[0]),
    }

    frame.render_widget(status_bar(state), chunks[1]);

    #[cfg(feature = "cheat-list")]
    if let Some(prompt) = state.path_prompt() {
        render_path_prompt(frame, prompt, state.path_input());
    }
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

/// Renders a small centered modal prompting for the path a cheat-list save/load should use.
#[cfg(feature = "cheat-list")]
fn render_path_prompt(frame: &mut Frame, prompt: PathPromptKind, input: &str) {
    let title = match prompt {
        PathPromptKind::Save => "Save cheat list — path (Enter: confirm, Esc: cancel)",
        PathPromptKind::Load => "Load cheat list — path (Enter: confirm, Esc: cancel)",
    };
    let area = centered_rect(frame.area(), 60, 3);

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(format!("{input}_"))
            .block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

/// A `width_pct`-wide, `height`-tall `Rect` centered within `area`.
#[cfg(feature = "cheat-list")]
fn centered_rect(area: Rect, width_pct: u16, height: u16) -> Rect {
    let width = area.width * width_pct / 100;
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
