use libscanmem::value::Value;

use super::*;

#[test]
fn default_state_starts_on_process_picker_with_no_status() {
    let state = AppState::default();

    assert_eq!(state.focus(), Focus::ProcessPicker);
    assert!(state.session().is_none());
    assert!(state.cheats().is_empty());
    assert!(!state.help_visible());
    assert!(!state.should_quit());
    assert!(state.status().is_none());
}

#[test]
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
    ] {
        let mut state = AppState::default();

        update(&mut state, msg);

        let status = state.status().expect("expected a status message");
        assert_eq!(status.level, StatusLevel::Error);
        assert!(status.text.contains("no process is attached"));
    }
}

#[test]
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
fn toggle_freeze_out_of_range_reports_an_error() {
    let mut state = AppState::default();

    update(&mut state, Msg::ToggleFreeze(0));

    assert_eq!(state.status().unwrap().level, StatusLevel::Error);
}

#[test]
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
