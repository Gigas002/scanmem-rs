//! Scan Panel: cycles the scan's data type/match type and edits its free-text value/range input,
//! then runs or resets a scan against the attached session — `ui/keymap.rs` owns the actual key
//! bindings, this module only renders `AppState`.

use libscanmem::scanroutines::{MatchType, ScanDataType};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

use crate::app::AppState;
use crate::ui::panel_border_style;

/// Renders the Scan Panel into `area`. While a scan/snapshot is running on a background thread
/// (`AppState::is_scanning`), shows a progress gauge instead of the usual controls — the scan
/// itself never blocks rendering, so this bar visibly advances instead of the whole TUI just
/// freezing until it's done. Otherwise shows the current data type/match type, the value/range
/// input (with a trailing cursor while being edited), and a one-line key hint. `focused`
/// highlights the panel border when it's the grid's (or expanded view's) current focus.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    if let Some((done, total)) = state.scan_progress() {
        render_progress(frame, area, done, total, focused);
        return;
    }

    let input = if state.search_active() {
        format!("{}_", state.scan_input())
    } else {
        state.scan_input().to_owned()
    };

    let text = format!(
        "Type: {}    Match: {}    Value: {input}",
        data_type_label(state.scan_data_type()),
        match_type_label(state.scan_match_type()),
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(focused))
        .title("Scan Panel — t: type, m: match, /: value, s: scan, n: snapshot, r: reset");

    frame.render_widget(Paragraph::new(text).block(block), area);
}

/// Renders a `done`/`total`-byte progress gauge.
fn render_progress(frame: &mut Frame, area: Rect, done: usize, total: usize, focused: bool) {
    let ratio = progress_ratio(done, total);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(focused))
        .title("Scan Panel — scanning… (Esc: cancel)");
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(ratio)
        .label(format!("{done}/{total} bytes ({:.0}%)", ratio * 100.0));

    frame.render_widget(gauge, area);
}

/// `done / total`, clamped to `[0.0, 1.0]` and guarded against `total == 0` (briefly true right
/// as a scan starts, before the background thread has read `/proc/<pid>/maps` and called
/// [`libscanmem::interrupt::ScanProgress::reset`]) — `Gauge::ratio` panics outside `[0.0, 1.0]`.
pub(super) fn progress_ratio(done: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (done as f64 / total as f64).min(1.0)
    }
}

fn data_type_label(data_type: ScanDataType) -> &'static str {
    match data_type {
        ScanDataType::AnyNumber => "any",
        ScanDataType::AnyInteger => "anyint",
        ScanDataType::AnyFloat => "anyfloat",
        ScanDataType::Integer8 => "i8",
        ScanDataType::Integer16 => "i16",
        ScanDataType::Integer32 => "i32",
        ScanDataType::Integer64 => "i64",
        ScanDataType::Float32 => "f32",
        ScanDataType::Float64 => "f64",
        ScanDataType::ByteArray => "bytes",
        ScanDataType::String => "string",
    }
}

fn match_type_label(match_type: MatchType) -> &'static str {
    match match_type {
        MatchType::Any => "any",
        MatchType::EqualTo => "=",
        MatchType::NotEqualTo => "!=",
        MatchType::GreaterThan => ">",
        MatchType::LessThan => "<",
        MatchType::Range => "range",
        MatchType::Update => "update",
        MatchType::NotChanged => "unchanged",
        MatchType::Changed => "changed",
        MatchType::Increased => "increased",
        MatchType::Decreased => "decreased",
        MatchType::IncreasedBy => "+",
        MatchType::DecreasedBy => "-",
    }
}
