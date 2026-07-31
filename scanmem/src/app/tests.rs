use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::session::{RegionFilter, ScanCriterion, ScanExpr, SessionOption};

use super::AppState;
use super::repl::dispatch;
use crate::commands::Command;

#[test]
fn new_state_has_no_session() {
    let state = AppState::default();
    assert!(state.session().is_none());
}

#[test]
fn attach_to_pid_zero_is_rejected_before_reaching_session() {
    let mut state = AppState::default();
    assert_eq!(
        dispatch(&mut state, Command::Attach(0)),
        "error: pid must not be zero"
    );
}

#[test]
fn scan_without_attach_reports_not_attached() {
    let mut state = AppState::default();
    let command = Command::Scan(ScanExpr {
        data_type: ScanDataType::Integer32,
        match_type: MatchType::Any,
        criterion: ScanCriterion::None,
    });
    assert_eq!(
        dispatch(&mut state, command),
        "error: no process is attached"
    );
}

#[test]
fn list_without_attach_reports_not_attached() {
    let mut state = AppState::default();
    assert_eq!(
        dispatch(&mut state, Command::List(None)),
        "error: no process is attached"
    );
}

#[test]
fn delete_without_attach_reports_not_attached() {
    let mut state = AppState::default();
    assert_eq!(
        dispatch(&mut state, Command::Delete("1".to_owned())),
        "error: no process is attached"
    );
}

#[test]
fn option_without_attach_reports_not_attached() {
    let mut state = AppState::default();
    let command = Command::SetOption(SessionOption::RegionFilter(RegionFilter::All));
    assert_eq!(
        dispatch(&mut state, command),
        "error: no process is attached"
    );
}

#[test]
fn reset_clears_any_session() {
    let mut state = AppState::default();
    assert_eq!(dispatch(&mut state, Command::Reset), "session reset");
    assert!(state.session().is_none());
}

#[test]
fn help_lists_verbs() {
    let mut state = AppState::default();
    assert!(dispatch(&mut state, Command::Help).contains("quit"));
}

mod script {
    use std::process::ExitCode;

    use super::AppState;
    use crate::app::script::run;

    #[test]
    fn empty_script_succeeds() {
        assert_eq!(run(AppState::default(), ""), ExitCode::SUCCESS);
    }

    #[test]
    fn whitespace_only_segments_are_skipped() {
        assert_eq!(run(AppState::default(), "  ; ;  "), ExitCode::SUCCESS);
    }

    #[test]
    fn unknown_verb_fails() {
        assert_eq!(run(AppState::default(), "bogus"), ExitCode::FAILURE);
    }

    #[test]
    fn dispatch_error_fails() {
        assert_eq!(run(AppState::default(), "attach 0"), ExitCode::FAILURE);
    }

    #[test]
    fn not_attached_dispatch_error_fails() {
        assert_eq!(run(AppState::default(), "list"), ExitCode::FAILURE);
    }

    #[test]
    fn quit_stops_processing_remaining_commands() {
        assert_eq!(
            run(AppState::default(), "help; quit; bogus"),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn multiple_valid_commands_succeed() {
        assert_eq!(run(AppState::default(), "help; help"), ExitCode::SUCCESS);
    }
}
