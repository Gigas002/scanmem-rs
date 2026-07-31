use super::*;

#[test]
fn parses_single_indices() {
    let set = parse_index_set("1,3,8", 10).unwrap();
    assert_eq!(set, BTreeSet::from([1, 3, 8]));
}

#[test]
fn parses_ranges() {
    let set = parse_index_set("1,3-5,8", 10).unwrap();
    assert_eq!(set, BTreeSet::from([1, 3, 4, 5, 8]));
}

#[test]
fn parses_hex_indices() {
    let set = parse_index_set("0x1,0xA", 20).unwrap();
    assert_eq!(set, BTreeSet::from([1, 10]));
}

#[test]
fn inverts_the_set() {
    let set = parse_index_set("!0-2", 5).unwrap();
    assert_eq!(set, BTreeSet::from([3, 4]));
}

#[test]
fn rejects_empty_set() {
    assert_eq!(parse_index_set("", 10), Err(SetParseError::Empty));
}

#[test]
fn rejects_duplicate_indices() {
    assert_eq!(parse_index_set("1,1", 10), Err(SetParseError::Duplicate(1)));
    assert_eq!(
        parse_index_set("1-3,2", 10),
        Err(SetParseError::Duplicate(2))
    );
}

#[test]
fn rejects_backwards_range() {
    assert_eq!(
        parse_index_set("5-2", 10),
        Err(SetParseError::InvalidRange("5-2".to_owned()))
    );
}

#[test]
fn rejects_out_of_bounds_index() {
    assert_eq!(
        parse_index_set("10", 10),
        Err(SetParseError::OutOfBounds(10, 10))
    );
}

#[test]
fn rejects_malformed_token() {
    assert_eq!(
        parse_index_set("abc", 10),
        Err(SetParseError::InvalidToken("abc".to_owned()))
    );
}

#[test]
fn rejects_inversion_not_at_start() {
    assert_eq!(
        parse_index_set("1,!2", 10),
        Err(SetParseError::InvertNotAtStart)
    );
}

#[test]
fn rejects_inverting_the_entire_range() {
    assert_eq!(
        parse_index_set("!0-4", 5),
        Err(SetParseError::InvertsEntireRange(5))
    );
}
