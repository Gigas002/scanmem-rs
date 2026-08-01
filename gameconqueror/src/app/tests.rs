use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::value::Value;

use super::*;

#[test]
fn default_state_starts_on_process_picker_with_no_status() {
    let state = AppState::default();

    assert_eq!(state.focus(), Focus::ProcessPicker);
    assert!(state.session().is_none());
    #[cfg(feature = "cheat-list")]
    assert!(state.cheats().is_empty());
    assert!(!state.help_visible());
    assert!(!state.should_quit());
    assert!(state.status().is_none());
}

#[test]
#[cfg(feature = "cheat-list")]
fn focus_next_and_prev_cycle_through_every_panel() {
    let mut state = AppState::default();

    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::ScanPanel);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::MatchView);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::CheatView);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::HexView);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::ProcessPicker);

    update(&mut state, Msg::FocusPrev);
    assert_eq!(state.focus(), Focus::HexView);
}

#[test]
#[cfg(not(feature = "cheat-list"))]
fn focus_next_and_prev_cycle_through_every_panel() {
    let mut state = AppState::default();

    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::ScanPanel);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::MatchView);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::HexView);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::ProcessPicker);

    update(&mut state, Msg::FocusPrev);
    assert_eq!(state.focus(), Focus::HexView);
}

#[test]
fn show_help_toggles_and_dismiss_always_closes() {
    let mut state = AppState::default();

    update(&mut state, Msg::ShowHelp);
    assert!(state.help_visible());
    update(&mut state, Msg::ShowHelp);
    assert!(!state.help_visible());

    update(&mut state, Msg::ShowHelp);
    assert!(state.help_visible());
    update(&mut state, Msg::Dismiss);
    assert!(!state.help_visible());
    update(&mut state, Msg::Dismiss);
    assert!(!state.help_visible());
}

#[test]
fn quit_sets_the_quit_flag() {
    let mut state = AppState::default();

    update(&mut state, Msg::Quit);

    assert!(state.should_quit());
}

#[test]
fn attach_with_zero_pid_reports_an_error_status_without_touching_focus() {
    let mut state = AppState::default();

    update(&mut state, Msg::Attach(0));

    assert!(state.session().is_none());
    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
fn operations_that_require_a_session_report_not_attached() {
    for msg in [
        Msg::Detach,
        Msg::Scan(libscanmem::session::ScanExpr {
            data_type: libscanmem::scanroutines::ScanDataType::Integer32,
            match_type: libscanmem::scanroutines::MatchType::Any,
            criterion: libscanmem::session::ScanCriterion::None,
        }),
        Msg::Snapshot,
        Msg::ResetScan,
        Msg::Write {
            address: 0x1000,
            value: Value::U32(1),
        },
        Msg::FocusHexView(0x1000),
    ] {
        let mut state = AppState::default();

        update(&mut state, msg);

        let status = state.status().expect("expected a status message");
        assert_eq!(status.level, StatusLevel::Error);
        assert!(status.text.contains("no process is attached"));
    }
}

#[test]
#[cfg(feature = "cheat-list")]
fn add_cheat_appends_an_entry_without_requiring_a_session() {
    let mut state = AppState::default();

    update(
        &mut state,
        Msg::AddCheat {
            address: 0x2000,
            description: "health".to_owned(),
            value: Value::U32(100),
        },
    );

    assert_eq!(state.cheats().len(), 1);
    assert_eq!(state.cheats()[0].address, 0x2000);
    assert_eq!(state.cheats()[0].description, "health");
    assert_eq!(state.cheats()[0].value, Value::U32(100));
    assert!(!state.cheats()[0].frozen);
}

#[test]
#[cfg(feature = "cheat-list")]
fn remove_cheat_drops_the_entry_at_the_given_index() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x2000,
            description: "health".to_owned(),
            value: Value::U32(100),
        },
    );

    update(&mut state, Msg::RemoveCheat(0));

    assert!(state.cheats().is_empty());
}

#[test]
#[cfg(feature = "cheat-list")]
fn remove_cheat_out_of_range_reports_an_error_and_keeps_the_list() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x2000,
            description: "health".to_owned(),
            value: Value::U32(100),
        },
    );

    update(&mut state, Msg::RemoveCheat(5));

    assert_eq!(state.cheats().len(), 1);
    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
#[cfg(feature = "cheat-list")]
fn toggle_freeze_flips_the_flag_each_call() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x2000,
            description: "health".to_owned(),
            value: Value::U32(100),
        },
    );

    update(&mut state, Msg::ToggleFreeze(0));
    assert!(state.cheats()[0].frozen);

    update(&mut state, Msg::ToggleFreeze(0));
    assert!(!state.cheats()[0].frozen);
}

#[test]
#[cfg(feature = "cheat-list")]
fn toggle_freeze_out_of_range_reports_an_error() {
    let mut state = AppState::default();

    update(&mut state, Msg::ToggleFreeze(0));

    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
#[cfg(feature = "cheat-list")]
fn edit_cheat_value_without_a_session_reports_not_attached_and_keeps_the_old_value() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x2000,
            description: "health".to_owned(),
            value: Value::U32(100),
        },
    );

    update(
        &mut state,
        Msg::EditCheatValue {
            index: 0,
            value: Value::U32(200),
        },
    );

    assert_eq!(state.cheats()[0].value, Value::U32(100));
    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("no process is attached"));
}

#[test]
#[cfg(feature = "cheat-list")]
fn edit_cheat_value_out_of_range_reports_an_error() {
    let mut state = AppState::default();

    update(
        &mut state,
        Msg::EditCheatValue {
            index: 0,
            value: Value::U32(200),
        },
    );

    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
fn focus_display_names_match_the_status_bar_labels() {
    assert_eq!(Focus::ProcessPicker.to_string(), "Process Picker");
    assert_eq!(Focus::ScanPanel.to_string(), "Scan Panel");
    assert_eq!(Focus::MatchView.to_string(), "Match View");
    #[cfg(feature = "cheat-list")]
    assert_eq!(Focus::CheatView.to_string(), "Cheat View");
    assert_eq!(Focus::HexView.to_string(), "Hex View");
}

#[test]
fn refresh_process_list_populates_processes_and_reports_a_count() {
    let mut state = AppState::default();

    update(&mut state, Msg::RefreshProcessList);

    assert!(!state.processes().is_empty());
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
}

#[test]
fn filter_processes_resets_selection_and_filters_by_substring() {
    let mut state = AppState::default();
    update(&mut state, Msg::RefreshProcessList);

    update(
        &mut state,
        Msg::FilterProcesses("this-should-not-match-anything-zzz".to_owned()),
    );

    assert!(state.filtered_processes().is_empty());
    assert_eq!(state.process_selected(), 0);
    assert!(state.selected_process().is_none());
}

#[test]
fn select_next_and_prev_wrap_around_the_filtered_process_list() {
    let mut state = AppState::default();
    update(&mut state, Msg::RefreshProcessList);
    let len = state.filtered_processes().len();
    if len < 2 {
        // Not enough real processes visible in this environment to exercise wraparound.
        return;
    }

    update(&mut state, Msg::SelectNext);
    assert_eq!(state.process_selected(), 1);

    update(&mut state, Msg::SelectPrev);
    assert_eq!(state.process_selected(), 0);

    update(&mut state, Msg::SelectPrev);
    assert_eq!(state.process_selected(), len - 1);

    update(&mut state, Msg::SelectNext);
    assert_eq!(state.process_selected(), 0);
}

#[test]
fn select_next_and_prev_are_a_no_op_outside_the_process_picker_focus() {
    let mut state = AppState::default();
    update(&mut state, Msg::RefreshProcessList);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::ScanPanel);

    update(&mut state, Msg::SelectNext);

    assert_eq!(state.process_selected(), 0);
}

#[test]
fn toggle_search_flips_the_flag_each_call() {
    let mut state = AppState::default();

    update(&mut state, Msg::ToggleSearch);
    assert!(state.search_active());

    update(&mut state, Msg::ToggleSearch);
    assert!(!state.search_active());
}

#[test]
fn dismiss_cancels_an_active_search_and_clears_the_filter() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);
    update(&mut state, Msg::FilterProcesses("abc".to_owned()));

    update(&mut state, Msg::Dismiss);

    assert!(!state.search_active());
    assert_eq!(state.process_filter(), "");
}

#[test]
fn dismiss_closes_help_before_cancelling_a_pending_search() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);
    update(&mut state, Msg::ShowHelp);

    update(&mut state, Msg::Dismiss);
    assert!(!state.help_visible());
    assert!(state.search_active());

    update(&mut state, Msg::Dismiss);
    assert!(!state.search_active());
}

#[test]
fn default_scan_panel_state_is_i32_equal_to_with_an_empty_value() {
    let state = AppState::default();

    assert_eq!(state.scan_data_type(), ScanDataType::Integer32);
    assert_eq!(state.scan_match_type(), MatchType::EqualTo);
    assert_eq!(state.scan_input(), "");
}

#[test]
fn cycle_scan_data_type_visits_every_variant_once_and_wraps() {
    let mut state = AppState::default();
    let start = state.scan_data_type();

    let mut seen = vec![start];
    for _ in 0..10 {
        update(&mut state, Msg::CycleScanDataType);
        seen.push(state.scan_data_type());
    }
    update(&mut state, Msg::CycleScanDataType);

    assert_eq!(seen.len(), 11);
    assert_eq!(state.scan_data_type(), start);
}

#[test]
fn cycle_scan_match_type_visits_every_variant_once_and_wraps() {
    let mut state = AppState::default();
    let start = state.scan_match_type();

    let mut seen = vec![start];
    for _ in 0..12 {
        update(&mut state, Msg::CycleScanMatchType);
        seen.push(state.scan_match_type());
    }
    update(&mut state, Msg::CycleScanMatchType);

    assert_eq!(seen.len(), 13);
    assert_eq!(state.scan_match_type(), start);
}

#[test]
fn set_scan_input_replaces_the_value() {
    let mut state = AppState::default();

    update(&mut state, Msg::SetScanInput("42".to_owned()));

    assert_eq!(state.scan_input(), "42");
}

#[test]
fn run_scan_without_a_value_reports_an_error_even_without_a_session() {
    let mut state = AppState::default();

    update(&mut state, Msg::RunScan);

    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("requires a value"));
}

#[test]
fn run_scan_range_without_two_values_reports_an_error() {
    let mut state = AppState::default();
    for _ in 0..4 {
        update(&mut state, Msg::CycleScanMatchType);
    }
    assert_eq!(state.scan_match_type(), MatchType::Range);

    update(&mut state, Msg::SetScanInput("10".to_owned()));
    update(&mut state, Msg::RunScan);

    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("low and high"));
}

#[test]
fn run_scan_with_a_valid_value_but_no_session_reports_not_attached() {
    let mut state = AppState::default();
    update(&mut state, Msg::SetScanInput("42".to_owned()));

    update(&mut state, Msg::RunScan);

    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("no process is attached"));
}

#[test]
fn cycle_match_sort_toggles_between_address_and_value_and_resets_selection() {
    let mut state = AppState::default();
    assert_eq!(state.match_sort(), MatchSortColumn::Address);

    update(&mut state, Msg::CycleMatchSort);
    assert_eq!(state.match_sort(), MatchSortColumn::Value);

    update(&mut state, Msg::CycleMatchSort);
    assert_eq!(state.match_sort(), MatchSortColumn::Address);
    assert_eq!(state.match_selected(), 0);
}

#[test]
fn filter_matches_sets_the_filter_and_resets_selection() {
    let mut state = AppState::default();

    update(&mut state, Msg::FilterMatches("dead".to_owned()));

    assert_eq!(state.match_filter(), "dead");
    assert_eq!(state.match_selected(), 0);
    assert!(state.filtered_matches().is_empty());
}

#[test]
fn select_next_and_prev_are_a_no_op_in_match_view_without_a_session() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::MatchView);

    update(&mut state, Msg::SelectNext);
    update(&mut state, Msg::SelectPrev);

    assert_eq!(state.match_selected(), 0);
}

#[test]
fn dismiss_clears_the_scan_input_while_in_scan_panel_focus() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::ScanPanel);
    update(&mut state, Msg::ToggleSearch);
    update(&mut state, Msg::SetScanInput("42".to_owned()));

    update(&mut state, Msg::Dismiss);

    assert!(!state.search_active());
    assert_eq!(state.scan_input(), "");
}

#[test]
fn dismiss_clears_the_match_filter_while_in_match_view_focus() {
    let mut state = AppState::default();
    update(&mut state, Msg::FocusNext);
    update(&mut state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::MatchView);
    update(&mut state, Msg::ToggleSearch);
    update(&mut state, Msg::FilterMatches("dead".to_owned()));

    update(&mut state, Msg::Dismiss);

    assert!(!state.search_active());
    assert_eq!(state.match_filter(), "");
}

#[cfg(feature = "cheat-list")]
fn focus_cheat_view(state: &mut AppState) {
    update(state, Msg::FocusNext);
    update(state, Msg::FocusNext);
    update(state, Msg::FocusNext);
    assert_eq!(state.focus(), Focus::CheatView);
}

#[test]
#[cfg(feature = "cheat-list")]
fn select_next_and_prev_wrap_around_the_cheat_list() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: String::new(),
            value: Value::U32(1),
        },
    );
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x2000,
            description: String::new(),
            value: Value::U32(2),
        },
    );
    focus_cheat_view(&mut state);

    update(&mut state, Msg::SelectNext);
    assert_eq!(state.cheat_selected(), 1);

    update(&mut state, Msg::SelectNext);
    assert_eq!(state.cheat_selected(), 0);

    update(&mut state, Msg::SelectPrev);
    assert_eq!(state.cheat_selected(), 1);
}

#[test]
#[cfg(feature = "cheat-list")]
fn begin_edit_cheat_value_prefills_the_input_and_activates_search() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: String::new(),
            value: Value::U32(100),
        },
    );

    update(&mut state, Msg::BeginEditCheatValue(0));

    assert!(state.search_active());
    assert_eq!(state.cheat_value_input(), "100");
}

#[test]
#[cfg(feature = "cheat-list")]
fn begin_edit_cheat_value_out_of_range_reports_an_error_and_does_not_activate_search() {
    let mut state = AppState::default();

    update(&mut state, Msg::BeginEditCheatValue(0));

    assert!(!state.search_active());
    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
#[cfg(feature = "cheat-list")]
fn confirm_cheat_value_edit_without_a_pending_edit_reports_an_error() {
    let mut state = AppState::default();

    update(&mut state, Msg::ConfirmCheatValueEdit);

    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
#[cfg(feature = "cheat-list")]
fn confirm_cheat_value_edit_out_of_width_input_reports_an_error_without_a_session() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: String::new(),
            value: Value::U8(1),
        },
    );
    update(&mut state, Msg::BeginEditCheatValue(0));

    update(&mut state, Msg::SetCheatValueInput("9999".to_owned()));
    update(&mut state, Msg::ConfirmCheatValueEdit);

    assert!(!state.search_active());
    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("does not fit"));
    assert_eq!(state.cheats()[0].value, Value::U8(1));
}

#[test]
#[cfg(feature = "cheat-list")]
fn confirm_cheat_value_edit_without_a_session_reports_not_attached() {
    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: String::new(),
            value: Value::U32(100),
        },
    );
    update(&mut state, Msg::BeginEditCheatValue(0));
    update(&mut state, Msg::SetCheatValueInput("200".to_owned()));

    update(&mut state, Msg::ConfirmCheatValueEdit);

    assert!(!state.search_active());
    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("no process is attached"));
    assert_eq!(state.cheats()[0].value, Value::U32(100));
}

#[test]
#[cfg(feature = "cheat-list")]
fn save_cheat_list_without_a_known_path_opens_a_save_prompt() {
    let mut state = AppState::default();

    update(&mut state, Msg::SaveCheatList);

    assert_eq!(state.path_prompt(), Some(PathPromptKind::Save));
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
}

#[test]
#[cfg(feature = "cheat-list")]
fn load_cheat_list_always_opens_a_load_prompt() {
    let mut state = AppState::default();

    update(&mut state, Msg::LoadCheatList);

    assert_eq!(state.path_prompt(), Some(PathPromptKind::Load));
}

#[test]
#[cfg(feature = "cheat-list")]
fn confirm_path_prompt_save_then_load_round_trips_the_cheat_list() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "gameconqueror-confirm-path-prompt-test-{}.toml",
        std::process::id()
    ));

    let mut state = AppState::default();
    update(
        &mut state,
        Msg::AddCheat {
            address: 0x1000,
            description: "health".to_owned(),
            value: Value::U32(100),
        },
    );
    update(&mut state, Msg::SaveCheatList);
    update(&mut state, Msg::SetPathInput(path.display().to_string()));
    update(&mut state, Msg::ConfirmPathPrompt);

    assert!(state.path_prompt().is_none());
    assert_eq!(state.status().unwrap().level, StatusLevel::Info);
    assert_eq!(state.cheat_list_path(), Some(path.as_path()));

    update(&mut state, Msg::RemoveCheat(0));
    assert!(state.cheats().is_empty());

    update(&mut state, Msg::LoadCheatList);
    update(&mut state, Msg::SetPathInput(path.display().to_string()));
    update(&mut state, Msg::ConfirmPathPrompt);

    std::fs::remove_file(&path).expect("cleanup should succeed");
    assert_eq!(state.cheats().len(), 1);
    assert_eq!(state.cheats()[0].address, 0x1000);
    assert_eq!(state.cheats()[0].description, "health");
}

#[test]
#[cfg(feature = "cheat-list")]
fn confirm_path_prompt_load_missing_file_reports_an_error() {
    let mut state = AppState::default();
    update(&mut state, Msg::LoadCheatList);
    update(
        &mut state,
        Msg::SetPathInput("/nonexistent/gameconqueror-cheatlist.toml".to_owned()),
    );

    update(&mut state, Msg::ConfirmPathPrompt);

    assert!(state.path_prompt().is_none());
    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
#[cfg(feature = "cheat-list")]
fn dismiss_closes_an_open_path_prompt_before_touching_search_state() {
    let mut state = AppState::default();
    update(&mut state, Msg::ToggleSearch);
    update(&mut state, Msg::SaveCheatList);

    update(&mut state, Msg::Dismiss);

    assert!(state.path_prompt().is_none());
    assert!(state.search_active());
}

#[test]
fn move_hex_cursor_is_a_no_op_on_an_empty_buffer() {
    let mut state = AppState::default();

    update(&mut state, Msg::MoveHexCursor(1));
    update(&mut state, Msg::MoveHexCursor(-1));

    assert_eq!(state.hex_cursor(), 0);
    assert!(state.hex_buffer().is_empty());
}

#[test]
fn set_hex_edit_input_replaces_the_value() {
    let mut state = AppState::default();

    update(&mut state, Msg::SetHexEditInput("3f".to_owned()));

    assert_eq!(state.hex_edit_input(), "3f");
}

#[test]
fn commit_hex_edit_without_input_reports_an_error() {
    let mut state = AppState::default();

    update(&mut state, Msg::CommitHexEdit);

    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("no byte value entered"));
    assert_eq!(state.hex_edit_input(), "");
}

#[test]
fn commit_hex_edit_with_invalid_hex_reports_an_error() {
    let mut state = AppState::default();
    update(&mut state, Msg::SetHexEditInput("zz".to_owned()));

    update(&mut state, Msg::CommitHexEdit);

    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("not a valid hex byte"));
}

#[test]
fn commit_hex_edit_with_valid_hex_but_no_session_reports_not_attached() {
    let mut state = AppState::default();
    update(&mut state, Msg::SetHexEditInput("3f".to_owned()));

    update(&mut state, Msg::CommitHexEdit);

    let status = state.status().expect("expected a status message");
    assert_eq!(status.level, StatusLevel::Error);
    assert!(status.text.contains("no process is attached"));
}

#[test]
fn dismiss_clears_an_in_progress_hex_edit() {
    let mut state = AppState::default();
    while state.focus() != Focus::HexView {
        update(&mut state, Msg::FocusNext);
    }
    update(&mut state, Msg::SetHexEditInput("3".to_owned()));

    update(&mut state, Msg::Dismiss);

    assert_eq!(state.hex_edit_input(), "");
}

#[cfg(feature = "cheat-list")]
fn sample_cheats() -> Vec<CheatEntry> {
    vec![
        CheatEntry {
            address: 0x1000,
            description: "health".to_owned(),
            value: Value::U32(100),
            frozen: true,
        },
        CheatEntry {
            address: 0x2000,
            description: String::new(),
            value: Value::F64(3.5),
            frozen: false,
        },
        CheatEntry {
            address: 0x3000,
            description: "pattern".to_owned(),
            value: Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
            frozen: false,
        },
    ]
}

#[test]
#[cfg(feature = "cheat-list")]
fn cheatlist_save_then_load_round_trips_every_entry() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "gameconqueror-cheatlist-test-{}.toml",
        std::process::id()
    ));
    let cheats = sample_cheats();

    cheatlist::save(&path, &cheats).expect("save should succeed");
    let loaded = cheatlist::load(&path).expect("load should succeed");

    std::fs::remove_file(&path).expect("cleanup should succeed");
    assert_eq!(loaded, cheats);
}

#[test]
#[cfg(feature = "cheat-list")]
fn cheatlist_load_missing_file_reports_an_io_error() {
    let path = std::env::temp_dir().join("gameconqueror-cheatlist-test-does-not-exist.toml");

    let err = cheatlist::load(&path).expect_err("missing file should fail to load");

    assert!(matches!(err, cheatlist::CheatListError::Io(_)));
}

#[test]
#[cfg(feature = "cheat-list")]
fn cheatlist_load_invalid_toml_reports_a_deserialize_error() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "gameconqueror-cheatlist-test-invalid-{}.toml",
        std::process::id()
    ));
    std::fs::write(&path, "not valid cheat list toml").expect("write should succeed");

    let err = cheatlist::load(&path).expect_err("invalid toml should fail to parse");

    std::fs::remove_file(&path).expect("cleanup should succeed");
    assert!(matches!(err, cheatlist::CheatListError::Deserialize(_)));
}
