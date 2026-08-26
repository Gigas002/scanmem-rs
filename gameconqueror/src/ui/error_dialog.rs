//! Error dialog modal: pops up automatically whenever `app::update` resolves a new error status,
//! so a failure is never just a red line at the bottom of a busy status bar — dismissed by any
//! keypress (`ui/input.rs`), unlike the help overlay's `Esc`-only dismissal.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::AppState;

/// Renders the error dialog, centered within `frame`'s full area, if `state.status()` currently
/// holds an error message. A no-op otherwise, so callers can render this unconditionally last.
pub fn render(frame: &mut Frame, state: &AppState) {
    if !state.error_dialog_visible() {
        return;
    }
    let Some(status) = state.status() else {
        return;
    };

    let area = centered_rect(frame.area(), 60, 30);
    let paragraph = Paragraph::new(status.text.clone())
        .style(crate::ui::theme::theme().status_bar_error)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Error — press any key to dismiss"),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

/// A `width_pct`/`height_pct`-sized `Rect` centered within `area` — same shape as
/// `help_overlay`'s helper of the same name (each `ui/` file keeps its own copy rather than
/// sharing one, per existing precedent).
fn centered_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let width = area.width * width_pct / 100;
    let height = area.height * height_pct / 100;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
