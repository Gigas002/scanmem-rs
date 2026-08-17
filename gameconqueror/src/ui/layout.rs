//! Top-level frame layout: renders the focused panel's body plus a one-line status bar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
#[cfg(feature = "cheat-list")]
use ratatui::widgets::{Block, Borders, Clear};

#[cfg(feature = "cheat-list")]
use crate::app::PathPromptKind;
use crate::app::{AppState, Focus, StatusLevel};
#[cfg(feature = "cheat-list")]
use crate::ui::cheat_view;
#[cfg(feature = "hex-view")]
use crate::ui::hex_view;
use crate::ui::{help_overlay, match_view, process_picker, scan_panel};

/// Renders the current frame: either the multi-panel grid or (while `AppState::expanded`) just
/// the focused panel fullscreen, then a status bar showing the current focus, the last status
/// message (if any), and a one-line hint of the global bindings.
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());

    if state.expanded() {
        render_expanded(frame, chunks[0], state);
    } else {
        render_grid(frame, chunks[0], state);
    }

    frame.render_widget(status_bar(state), chunks[1]);

    #[cfg(feature = "cheat-list")]
    if let Some(prompt) = state.path_prompt() {
        render_path_prompt(frame, prompt, state.path_input());
    }

    if state.help_visible() {
        help_overlay::render(frame, state.focus());
    }
}

/// Renders only the focused panel into `area`, filling it entirely — `Msg::ToggleExpand`
/// (`Ctrl+E`)'s fullscreen mode.
fn render_expanded(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.focus() {
        Focus::ProcessPicker => process_picker::render(frame, area, state, true),
        Focus::ScanPanel => scan_panel::render(frame, area, state, true),
        Focus::MatchView => match_view::render(frame, area, state, true),
        #[cfg(feature = "cheat-list")]
        Focus::CheatView => cheat_view::render(frame, area, state, true),
        #[cfg(feature = "hex-view")]
        Focus::HexView => hex_view::render(frame, area, state, true),
    }
}

/// Renders every panel at once into `area`, arranged in a fixed grid — top row: Process Picker |
/// Scan Panel; middle row: Match View | Cheat View (or just Match View without the `cheat-list`
/// feature); bottom row: Hex View, spanning the full width, if built with the `hex-view` feature
/// (dropped entirely otherwise, leaving a 2-row grid). `Focus::towards` (in `app/focus.rs`)
/// encodes `Ctrl+<Arrow>` navigation over exactly this arrangement, so changing it here means
/// updating that too.
fn render_grid(frame: &mut Frame, area: Rect, state: &AppState) {
    #[cfg(feature = "hex-view")]
    let row_constraints = vec![
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Min(0),
    ];
    #[cfg(not(feature = "hex-view"))]
    let row_constraints = vec![Constraint::Percentage(30), Constraint::Min(0)];

    let rows = Layout::vertical(row_constraints).split(area);

    let top =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).split(rows[0]);
    process_picker::render(frame, top[0], state, state.focus() == Focus::ProcessPicker);
    scan_panel::render(frame, top[1], state, state.focus() == Focus::ScanPanel);

    #[cfg(feature = "cheat-list")]
    {
        let middle = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        match_view::render(frame, middle[0], state, state.focus() == Focus::MatchView);
        cheat_view::render(frame, middle[1], state, state.focus() == Focus::CheatView);
    }
    #[cfg(not(feature = "cheat-list"))]
    match_view::render(frame, rows[1], state, state.focus() == Focus::MatchView);

    #[cfg(feature = "hex-view")]
    hex_view::render(frame, rows[2], state, state.focus() == Focus::HexView);
}

fn status_bar(state: &AppState) -> Paragraph<'_> {
    let attach_label = match state.attached() {
        Some(process) => format!("attached: {} ({})", process.pid, process.name),
        None => "not attached".to_owned(),
    };
    // Read live off `scan_progress` (not the one-shot "scanning…" status message) so this
    // updates every frame regardless of which panel is focused, not just the Scan Panel.
    let scan_label = state.scan_progress().map(|(done, total)| {
        let percent = if total == 0 {
            0.0
        } else {
            (done as f64 / total as f64 * 100.0).min(100.0)
        };
        format!(" · scanning {percent:.0}% (Esc: cancel)")
    });
    let expanded_label = if state.expanded() { " (expanded)" } else { "" };
    let mut text = format!(
        "[{}{expanded_label}] {attach_label}{} · Ctrl+arrows: switch panel · Ctrl+E: expand \
         · Tab: cycle · ?: help · Ctrl+Q: quit",
        state.focus(),
        scan_label.unwrap_or_default(),
    );
    let mut style = crate::ui::theme::theme().status_bar;

    if let Some(status) = state.status() {
        text = format!("{text} — {}", status.text);
        if status.level == StatusLevel::Error {
            style = crate::ui::theme::theme().status_bar_error;
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
