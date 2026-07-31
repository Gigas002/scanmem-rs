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

mod repl {
    use std::cell::Cell;
    use std::rc::Rc;

    use rustyline::Context;
    use rustyline::completion::Completer;
    use rustyline::history::DefaultHistory;

    use crate::app::repl::{ScanmemHelper, current_word, first_word};

    fn helper(match_count: usize) -> ScanmemHelper {
        ScanmemHelper::new(Rc::new(Cell::new(match_count)))
    }

    #[test]
    fn current_word_finds_the_token_under_the_cursor() {
        assert_eq!(current_word("li", 2), (0, "li"));
        assert_eq!(current_word("list 1", 6), (5, "1"));
        assert_eq!(current_word("list ", 5), (5, ""));
    }

    #[test]
    fn first_word_extracts_the_verb() {
        assert_eq!(first_word("list 0 10"), "list");
        assert_eq!(first_word(""), "");
    }

    #[test]
    fn completes_verb_prefixes_at_the_start_of_the_line() {
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (start, candidates) = helper(0).complete("li", 2, &ctx).unwrap();
        assert_eq!(start, 0);
        assert!(candidates.contains(&"list".to_owned()));
        assert!(!candidates.contains(&"dump".to_owned()));
    }

    #[test]
    fn completes_match_indices_after_list_or_delete() {
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (start, candidates) = helper(3).complete("list ", 5, &ctx).unwrap();
        assert_eq!(start, 5);
        assert_eq!(candidates, vec!["0", "1", "2"]);

        let (_, candidates) = helper(3).complete("delete ", 7, &ctx).unwrap();
        assert_eq!(candidates, vec!["0", "1", "2"]);
    }

    #[test]
    fn does_not_offer_match_indices_for_other_verbs() {
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (_, candidates) = helper(3).complete("dump ", 5, &ctx).unwrap();
        assert!(candidates.is_empty());
    }
}
