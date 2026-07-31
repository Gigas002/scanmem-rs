use libscanmem::scanroutines::{MatchType, ScanDataType};
use libscanmem::session::{RegionFilter, ScanCriterion, SessionOption};
use libscanmem::value::{ByteOrWildcard, UserValue, Value, parse_number};

use super::Command;
use super::formatter;
use super::parser::{ParseError, parse};

#[test]
fn parses_pid_and_attach_as_the_same_command() {
    assert_eq!(parse("pid 1234").unwrap(), Command::Attach(1234));
    assert_eq!(parse("attach 1234").unwrap(), Command::Attach(1234));
}

#[test]
fn attach_rejects_a_non_numeric_pid() {
    assert!(matches!(parse("pid abc"), Err(ParseError::InvalidPid(_))));
}

#[test]
fn parses_an_equal_to_scan() {
    let command = parse("scan i32 = 100").unwrap();
    let Command::Scan(expr) = command else {
        panic!("expected a Scan command");
    };
    assert_eq!(expr.data_type, ScanDataType::Integer32);
    assert_eq!(expr.match_type, MatchType::EqualTo);
    assert_eq!(
        expr.criterion,
        ScanCriterion::Value(UserValue::Number(parse_number("100").unwrap()))
    );
}

#[test]
fn parses_a_range_scan() {
    let command = parse("scan i32 range 10 20").unwrap();
    let Command::Scan(expr) = command else {
        panic!("expected a Scan command");
    };
    assert_eq!(expr.match_type, MatchType::Range);
    assert_eq!(
        expr.criterion,
        ScanCriterion::Range(parse_number("10").unwrap(), parse_number("20").unwrap())
    );
}

#[test]
fn parses_a_criterion_free_scan() {
    let command = parse("scan i32 increased").unwrap();
    let Command::Scan(expr) = command else {
        panic!("expected a Scan command");
    };
    assert_eq!(expr.match_type, MatchType::Increased);
    assert_eq!(expr.criterion, ScanCriterion::None);
}

#[test]
fn scan_rejects_missing_value_for_equal_to() {
    assert!(parse("scan i32 =").is_err());
}

#[test]
fn parses_a_bytearray_scan() {
    let command = parse("scan bytes = de ad ?? ef").unwrap();
    let Command::Scan(expr) = command else {
        panic!("expected a Scan command");
    };
    assert_eq!(expr.data_type, ScanDataType::ByteArray);
    assert_eq!(
        expr.criterion,
        ScanCriterion::Value(UserValue::Bytes(vec![
            ByteOrWildcard::Fixed(0xde),
            ByteOrWildcard::Fixed(0xad),
            ByteOrWildcard::Wildcard,
            ByteOrWildcard::Fixed(0xef),
        ]))
    );
}

#[test]
fn parses_a_string_scan() {
    let command = parse("scan string = hello world").unwrap();
    let Command::Scan(expr) = command else {
        panic!("expected a Scan command");
    };
    assert_eq!(
        expr.criterion,
        ScanCriterion::Value(UserValue::Str("hello world".to_owned()))
    );
}

#[test]
fn snapshot_takes_no_arguments() {
    assert_eq!(parse("snapshot").unwrap(), Command::Snapshot);
    assert!(matches!(
        parse("snapshot now"),
        Err(ParseError::UnexpectedArguments(_))
    ));
}

#[test]
fn parses_list_with_and_without_a_range() {
    assert_eq!(parse("list").unwrap(), Command::List(None));
    assert_eq!(parse("list 0 10").unwrap(), Command::List(Some(0..10)));
}

#[test]
fn parses_dump() {
    assert_eq!(
        parse("dump 0x1000 16").unwrap(),
        Command::Dump {
            address: 0x1000,
            len: 16
        }
    );
}

#[test]
fn parses_write_with_a_concrete_type() {
    assert_eq!(
        parse("write 0x1000 i32 42").unwrap(),
        Command::Write {
            address: 0x1000,
            value: Value::I32(42),
        }
    );
}

#[test]
fn write_rejects_a_value_that_does_not_fit_the_requested_width() {
    assert!(parse("write 0x1000 u8 1000").is_err());
}

#[test]
fn parses_write_string_and_bytes() {
    assert_eq!(
        parse("write 0x1000 string hi there").unwrap(),
        Command::Write {
            address: 0x1000,
            value: Value::Str("hi there".to_owned()),
        }
    );
    assert_eq!(
        parse("write 0x1000 bytes de ad").unwrap(),
        Command::Write {
            address: 0x1000,
            value: Value::Bytes(vec![0xde, 0xad]),
        }
    );
}

#[test]
fn parses_delete_selector_as_raw_text() {
    assert_eq!(
        parse("delete 1,3-5").unwrap(),
        Command::Delete("1,3-5".to_owned())
    );
}

#[test]
fn parses_option() {
    assert_eq!(
        parse("option endianness swapped").unwrap(),
        Command::SetOption(SessionOption::Endianness(
            libscanmem::scanroutines::Endianness::Swapped
        ))
    );
    assert_eq!(
        parse("option region all").unwrap(),
        Command::SetOption(SessionOption::RegionFilter(RegionFilter::All))
    );
}

#[test]
fn parses_reset_help_and_quit() {
    assert_eq!(parse("reset").unwrap(), Command::Reset);
    assert_eq!(parse("help").unwrap(), Command::Help);
    assert_eq!(parse("quit").unwrap(), Command::Quit);
    assert_eq!(parse("exit").unwrap(), Command::Quit);
}

#[test]
fn unknown_verb_is_rejected() {
    assert!(matches!(
        parse("frobnicate"),
        Err(ParseError::UnknownVerb(_))
    ));
}

#[test]
fn formats_scan_stats() {
    use libscanmem::session::ScanStats;
    assert_eq!(
        formatter::scan_stats(ScanStats { matches: 3 }),
        "3 match(es)"
    );
}

#[test]
fn formats_an_empty_match_table() {
    assert_eq!(formatter::match_table(std::iter::empty()), "no matches");
}

#[test]
fn formats_a_dump() {
    let bytes = b"Hi!\x00";
    let rendered = formatter::dump(0x1000, bytes);
    assert!(rendered.contains("0x00001000"));
    assert!(rendered.contains("48 69 21 00"));
    assert!(rendered.contains("Hi!."));
}

#[test]
fn formats_deleted_count() {
    assert_eq!(formatter::deleted(2), "deleted 2 match(es)");
}

#[test]
fn help_lists_every_verb() {
    let text = formatter::help();
    for verb in [
        "pid", "scan", "snapshot", "list", "dump", "write", "delete", "option", "reset", "help",
        "quit",
    ] {
        assert!(text.contains(verb), "help text missing {verb:?}");
    }
}
