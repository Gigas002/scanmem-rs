//! Scan Panel: cycles the scan's data type/match type and edits its free-text value/range input,
//! then runs or resets a scan against the attached session — `ui/keymap.rs` owns the actual key
//! bindings, this module only renders `AppState`.

use libscanmem::scanroutines::{MatchType, ScanDataType};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;

/// Renders the Scan Panel into `area`: the current data type/match type, the value/range input
/// (with a trailing cursor while being edited), and a one-line key hint.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
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
        .title("Scan Panel — t: type, m: match, /: value, s: scan, n: snapshot, r: reset");

    frame.render_widget(Paragraph::new(text).block(block), area);
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
