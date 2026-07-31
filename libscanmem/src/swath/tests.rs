use super::*;

fn entry(old_value: u8, flags: MatchFlags) -> SwathEntry {
    SwathEntry { old_value, flags }
}

#[test]
fn add_starts_a_new_swath_for_the_first_byte() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    assert_eq!(store.swaths().len(), 1);
    assert_eq!(store.swaths()[0].first_byte_in_child, 100);
    assert_eq!(store.swaths()[0].entries, vec![entry(1, MatchFlags::U8)]);
}

#[test]
fn add_appends_contiguous_addresses_to_the_same_swath() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    store.add(101, 2, MatchFlags::U8);
    store.add(102, 3, MatchFlags::U8);
    assert_eq!(store.swaths().len(), 1);
    assert_eq!(store.swaths()[0].entries.len(), 3);
    assert_eq!(store.swaths()[0].address_of(2), 102);
}

#[test]
fn add_starts_a_new_swath_when_address_is_not_contiguous() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    store.add(200, 2, MatchFlags::U8);
    assert_eq!(store.swaths().len(), 2);
    assert_eq!(store.swaths()[1].first_byte_in_child, 200);
}

#[test]
fn match_count_ignores_empty_flag_filler_bytes() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U32);
    store.add(101, 0, MatchFlags::empty());
    store.add(102, 0, MatchFlags::empty());
    store.add(103, 0, MatchFlags::empty());
    assert_eq!(store.swaths()[0].entries.len(), 4);
    assert_eq!(store.match_count(), 1);
}

#[test]
fn matches_iterates_addresses_and_entries_in_order() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    store.add(101, 0, MatchFlags::empty());
    store.add(102, 2, MatchFlags::U8);
    let collected: Vec<(usize, u8)> = store
        .matches()
        .map(|(addr, e)| (addr, e.old_value))
        .collect();
    assert_eq!(collected, vec![(100, 1), (102, 2)]);
}

#[test]
fn nth_match_skips_filler_bytes_and_returns_the_right_location() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    store.add(101, 0, MatchFlags::empty());
    store.add(102, 2, MatchFlags::U8);
    store.add(500, 3, MatchFlags::U8);

    let first = store.nth_match(0).unwrap();
    assert_eq!(
        store.entry_at(first),
        Some((100, &entry(1, MatchFlags::U8)))
    );

    let second = store.nth_match(1).unwrap();
    assert_eq!(
        store.entry_at(second),
        Some((102, &entry(2, MatchFlags::U8)))
    );

    let third = store.nth_match(2).unwrap();
    assert_eq!(third.swath_index, 1);
    assert_eq!(
        store.entry_at(third),
        Some((500, &entry(3, MatchFlags::U8)))
    );

    assert_eq!(store.nth_match(3), None);
}

#[test]
fn entry_at_returns_none_for_an_out_of_bounds_location() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    assert_eq!(
        store.entry_at(MatchLocation {
            swath_index: 1,
            entry_index: 0
        }),
        None
    );
    assert_eq!(
        store.entry_at(MatchLocation {
            swath_index: 0,
            entry_index: 5
        }),
        None
    );
}

#[test]
fn delete_in_range_removes_only_addresses_within_the_range() {
    let mut store = SwathStore::new();
    for (addr, val) in [(100, 1), (101, 2), (102, 3), (103, 4), (104, 5)] {
        store.add(addr, val, MatchFlags::U8);
    }
    store.delete_in_range(102, 104);

    let remaining: Vec<(usize, u8)> = store
        .matches()
        .map(|(addr, e)| (addr, e.old_value))
        .collect();
    assert_eq!(remaining, vec![(100, 1), (101, 2), (104, 5)]);
}

#[test]
fn delete_in_range_splits_a_swath_when_the_middle_is_removed() {
    let mut store = SwathStore::new();
    for (addr, val) in [(100, 1), (101, 2), (102, 3), (103, 4), (104, 5)] {
        store.add(addr, val, MatchFlags::U8);
    }
    store.delete_in_range(102, 103);

    assert_eq!(store.swaths().len(), 2);
    assert_eq!(store.swaths()[0].first_byte_in_child, 100);
    assert_eq!(store.swaths()[0].entries.len(), 2);
    assert_eq!(store.swaths()[1].first_byte_in_child, 103);
    assert_eq!(store.swaths()[1].entries.len(), 2);
}

#[test]
fn delete_in_range_with_no_overlap_leaves_the_store_unchanged() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    store.add(101, 2, MatchFlags::U8);
    let before = store.clone();

    store.delete_in_range(200, 300);

    assert_eq!(store, before);
}

#[test]
fn delete_in_range_can_empty_the_store() {
    let mut store = SwathStore::new();
    store.add(100, 1, MatchFlags::U8);
    store.add(101, 2, MatchFlags::U8);

    store.delete_in_range(0, usize::MAX);

    assert_eq!(store.swaths().len(), 0);
    assert_eq!(store.match_count(), 0);
}

#[test]
fn printable_string_replaces_non_printable_bytes_with_a_dot() {
    let mut store = SwathStore::new();
    for (i, b) in (*b"Hi\x01!").into_iter().enumerate() {
        store.add(100 + i, b, MatchFlags::U8);
    }
    let location = MatchLocation {
        swath_index: 0,
        entry_index: 0,
    };
    assert_eq!(store.printable_string(location, 4).as_deref(), Some("Hi.!"));
}

#[test]
fn printable_string_stops_early_at_the_end_of_the_swath() {
    let mut store = SwathStore::new();
    store.add(100, b'a', MatchFlags::U8);
    store.add(101, b'b', MatchFlags::U8);
    let location = MatchLocation {
        swath_index: 0,
        entry_index: 0,
    };
    assert_eq!(store.printable_string(location, 10).as_deref(), Some("ab"));
}

#[test]
fn printable_string_returns_none_for_an_out_of_bounds_swath() {
    let store = SwathStore::new();
    let location = MatchLocation {
        swath_index: 0,
        entry_index: 0,
    };
    assert_eq!(store.printable_string(location, 4), None);
}

#[test]
fn bytearray_text_renders_space_separated_lowercase_hex() {
    let mut store = SwathStore::new();
    store.add(100, 0xde, MatchFlags::U8);
    store.add(101, 0xad, MatchFlags::U8);
    store.add(102, 0xbe, MatchFlags::U8);
    store.add(103, 0xef, MatchFlags::U8);
    let location = MatchLocation {
        swath_index: 0,
        entry_index: 0,
    };
    assert_eq!(
        store.bytearray_text(location, 4).as_deref(),
        Some("de ad be ef")
    );
}

#[test]
fn bytearray_text_reads_starting_from_a_non_zero_entry_index() {
    let mut store = SwathStore::new();
    store.add(100, 0x01, MatchFlags::U8);
    store.add(101, 0x02, MatchFlags::U8);
    store.add(102, 0x03, MatchFlags::U8);
    let location = MatchLocation {
        swath_index: 0,
        entry_index: 1,
    };
    assert_eq!(store.bytearray_text(location, 2).as_deref(), Some("02 03"));
}
