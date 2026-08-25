use rustix::process::Pid;

use super::*;
use crate::maps::Perms;
use crate::swath::SwathEntry;
use crate::value::parse_int;

fn writable_region() -> Region {
    Region {
        start: 0x1000,
        end: 0x2000,
        perms: Perms {
            read: true,
            write: true,
            exec: false,
            shared: false,
        },
        path: None,
    }
}

fn read_only_region() -> Region {
    Region {
        start: 0x3000,
        end: 0x4000,
        perms: Perms {
            read: true,
            write: false,
            exec: true,
            shared: false,
        },
        path: Some("/usr/bin/x".to_owned()),
    }
}

fn equal_to_expr(literal: &str) -> ScanExpr {
    ScanExpr {
        data_type: ScanDataType::Integer32,
        match_type: MatchType::EqualTo,
        criterion: ScanCriterion::Value(UserValue::Number(parse_int(literal).unwrap())),
    }
}

fn empty_session() -> Session {
    Session {
        process: None,
        matches: SwathStore::new(),
        options: SessionOptions::default(),
        stop_flag: StopFlag::new(),
        progress: ScanProgress::new(),
    }
}

#[test]
fn region_filter_writable_only_excludes_read_only_regions() {
    assert!(RegionFilter::WritableOnly.includes(&writable_region()));
    assert!(!RegionFilter::WritableOnly.includes(&read_only_region()));
}

#[test]
fn region_filter_all_includes_every_readable_region() {
    assert!(RegionFilter::All.includes(&writable_region()));
    assert!(RegionFilter::All.includes(&read_only_region()));
}

#[test]
fn validate_rejects_bytearray_without_a_pattern() {
    let expr = ScanExpr {
        data_type: ScanDataType::ByteArray,
        match_type: MatchType::EqualTo,
        criterion: ScanCriterion::None,
    };
    assert!(matches!(validate(&expr), Err(ScanmemError::InvalidExpr(_))));
}

#[test]
fn validate_rejects_string_without_a_pattern() {
    let expr = ScanExpr {
        data_type: ScanDataType::String,
        match_type: MatchType::EqualTo,
        criterion: ScanCriterion::None,
    };
    assert!(matches!(validate(&expr), Err(ScanmemError::InvalidExpr(_))));
}

#[test]
fn validate_rejects_range_without_bounds() {
    let expr = ScanExpr {
        data_type: ScanDataType::Integer32,
        match_type: MatchType::Range,
        criterion: ScanCriterion::None,
    };
    assert!(matches!(validate(&expr), Err(ScanmemError::InvalidExpr(_))));
}

#[test]
fn validate_accepts_a_well_formed_equal_to_expr() {
    assert!(validate(&equal_to_expr("42")).is_ok());
}

#[test]
fn value_to_bytes_native_matches_to_ne_bytes() {
    let value = Value::U32(0x0102_0304);
    assert_eq!(
        value_to_bytes(&value, Endianness::Native),
        0x0102_0304u32.to_ne_bytes().to_vec()
    );
}

#[test]
fn value_to_bytes_swapped_reverses_the_bytes() {
    let value = Value::U32(0x0102_0304);
    let mut expected = 0x0102_0304u32.to_ne_bytes().to_vec();
    expected.reverse();
    assert_eq!(value_to_bytes(&value, Endianness::Swapped), expected);
}

#[test]
fn value_to_bytes_bytes_and_str_pass_through_unchanged() {
    assert_eq!(
        value_to_bytes(&Value::Bytes(vec![1, 2, 3]), Endianness::Native),
        vec![1, 2, 3]
    );
    assert_eq!(
        value_to_bytes(&Value::Str("hi".to_owned()), Endianness::Native),
        b"hi".to_vec()
    );
}

#[test]
fn scan_buffer_finds_a_single_equal_to_match() {
    let expr = equal_to_expr("42");
    let bytes = 42i32.to_ne_bytes();
    let results = scan_buffer_chunk(0x1000, &bytes, &expr, Endianness::Native, usize::MAX);

    assert_eq!(results.len(), 4);
    assert_eq!(results[0].0, 0x1000);
    assert!(!results[0].2.is_empty());
    // Filler bytes preserve raw content but carry no match flags of their own.
    assert!(results[1].2.is_empty());
    assert!(results[2].2.is_empty());
    assert!(results[3].2.is_empty());
}

#[test]
fn scan_buffer_finds_no_matches() {
    let expr = equal_to_expr("42");
    let bytes = 7i32.to_ne_bytes();
    assert!(scan_buffer_chunk(0x1000, &bytes, &expr, Endianness::Native, usize::MAX).is_empty());
}

#[test]
fn scan_buffer_records_overlapping_matches_at_adjacent_addresses() {
    // Every byte here independently equals 0x11 under Integer8, so each address is its own
    // match start; scan_buffer_chunk must not skip ahead past an earlier match's width.
    let expr = ScanExpr {
        data_type: ScanDataType::Integer8,
        match_type: MatchType::EqualTo,
        criterion: ScanCriterion::Value(UserValue::Number(parse_int("0x11").unwrap())),
    };
    let bytes = [0x11u8, 0x11, 0x11];
    let results = scan_buffer_chunk(0x1000, &bytes, &expr, Endianness::Native, usize::MAX);

    let addresses: Vec<usize> = results.iter().map(|(address, _, _)| *address).collect();
    assert_eq!(addresses, vec![0x1000, 0x1001, 0x1002]);
    assert!(results.iter().all(|(_, _, flags)| !flags.is_empty()));
}

#[test]
fn scan_buffer_chunk_matches_an_unchunked_scan_across_a_synthetic_boundary() {
    // A single i32 match at offset 2, straddling a synthetic chunk boundary at offset 4 — proves
    // `scan_buffer_chunk`'s overlap+`keep_before` trimming reproduces exactly what one unchunked
    // `scan_buffer` call over the whole buffer finds, with nothing duplicated or dropped.
    let bytes: [u8; 8] = [0xAA, 0xBB, 42, 0, 0, 0, 0xCC, 0xDD];
    let expr = equal_to_expr("42");
    let reference = scan_buffer_chunk(0, &bytes, &expr, Endianness::Native, usize::MAX);

    // Chunk 1 reads its own 4 bytes plus a 3-byte overlap tail (i32's width minus one) so the
    // match starting at offset 2 is fully visible; chunk 2 reads only its own remaining 4 bytes,
    // since offset 4 is the region's actual end here.
    let mut chunked = scan_buffer_chunk(0, &bytes[0..7], &expr, Endianness::Native, 4);
    chunked.extend(scan_buffer_chunk(
        4,
        &bytes[4..8],
        &expr,
        Endianness::Native,
        4,
    ));

    assert_eq!(chunked, reference);
}

#[test]
fn scan_buffer_chunk_excludes_a_group_starting_at_or_after_keep_before() {
    let expr = equal_to_expr("42");
    let bytes = 42i32.to_ne_bytes();

    assert!(scan_buffer_chunk(0x1000, &bytes, &expr, Endianness::Native, 0).is_empty());
    assert_eq!(
        scan_buffer_chunk(0x1000, &bytes, &expr, Endianness::Native, 1).len(),
        4
    );
}

#[test]
fn scan_buffer_chunk_keeps_the_full_filler_tail_of_a_boundary_straddling_match() {
    // The match itself starts at offset 2 (before `keep_before`), but its filler tail runs
    // through offset 5 — past `keep_before` — and must still be kept in full: it belongs to a
    // group that already started inside the owned chunk.
    let bytes: [u8; 7] = [0xAA, 0xBB, 42, 0, 0, 0, 0xCC];
    let expr = equal_to_expr("42");

    let results = scan_buffer_chunk(0, &bytes, &expr, Endianness::Native, 4);

    let addresses: Vec<usize> = results.iter().map(|(address, _, _)| *address).collect();
    assert_eq!(addresses, vec![2, 3, 4, 5]);
}

#[test]
fn batch_entries_end_stops_at_the_first_group_boundary_at_or_after_max_len() {
    let entries = vec![
        SwathEntry {
            old_value: 1,
            flags: MatchFlags::U8,
        },
        SwathEntry {
            old_value: 2,
            flags: MatchFlags::U8,
        },
        SwathEntry {
            old_value: 3,
            flags: MatchFlags::empty(), // filler, continues the group started at index 2
        },
        SwathEntry {
            old_value: 4,
            flags: MatchFlags::U8,
        },
    ];

    // A max_len of 2 lands mid-group (index 2 is filler for the group starting at index 1), so
    // the batch must extend to index 3 — the next real group boundary — not stop short of it.
    assert_eq!(batch_entries_end(&entries, 0, 2), 3);
    // A max_len that already lands exactly on a group boundary needs no extension.
    assert_eq!(batch_entries_end(&entries, 0, 1), 1);
}

#[test]
fn batch_entries_end_always_includes_at_least_the_first_whole_group() {
    let entries = vec![
        SwathEntry {
            old_value: 1,
            flags: MatchFlags::U8,
        },
        SwathEntry {
            old_value: 2,
            flags: MatchFlags::empty(),
        },
        SwathEntry {
            old_value: 3,
            flags: MatchFlags::empty(),
        },
    ];

    // max_len of 0 would ask for an empty batch, but the whole 3-entry group starting at index 0
    // must still come back in full rather than getting split.
    assert_eq!(batch_entries_end(&entries, 0, 0), 3);
}

#[test]
fn narrow_swath_keeps_a_match_that_still_equals_the_new_criterion() {
    let old_swath = Swath {
        first_byte_in_child: 0x1000,
        entries: vec![
            SwathEntry {
                old_value: 42,
                flags: MatchFlags::U32 | MatchFlags::S32,
            },
            SwathEntry {
                old_value: 0,
                flags: MatchFlags::empty(),
            },
            SwathEntry {
                old_value: 0,
                flags: MatchFlags::empty(),
            },
            SwathEntry {
                old_value: 0,
                flags: MatchFlags::empty(),
            },
        ],
    };
    let fresh = 42i32.to_ne_bytes();

    let results = narrow_swath(&old_swath, &fresh, &equal_to_expr("42"), Endianness::Native);
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].0, 0x1000);
    assert!(!results[0].2.is_empty());
}

#[test]
fn narrow_swath_drops_a_match_that_no_longer_equals_the_new_criterion() {
    let old_swath = Swath {
        first_byte_in_child: 0x1000,
        entries: vec![
            SwathEntry {
                old_value: 42,
                flags: MatchFlags::U32 | MatchFlags::S32,
            },
            SwathEntry {
                old_value: 0,
                flags: MatchFlags::empty(),
            },
            SwathEntry {
                old_value: 0,
                flags: MatchFlags::empty(),
            },
            SwathEntry {
                old_value: 0,
                flags: MatchFlags::empty(),
            },
        ],
    };
    let fresh = 7i32.to_ne_bytes();

    let results = narrow_swath(&old_swath, &fresh, &equal_to_expr("42"), Endianness::Native);
    assert!(results.is_empty());
}

#[test]
fn session_matches_and_nth_match_read_the_swath_store() {
    let mut session = empty_session();
    session.matches.add(0x1000, 42, MatchFlags::U8);
    // Far enough away to start its own swath, so it can't be mistaken for a filler byte of the
    // match above.
    session.matches.add(0x2000, 0, MatchFlags::empty());

    let matches: Vec<MatchView> = session.matches().collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].address, 0x1000);
    assert_eq!(matches[0].old_value, Value::U8(42));

    let first = session.nth_match(0).expect("one match recorded");
    assert_eq!(first.address, 0x1000);
    assert_eq!(first.old_value, Value::U8(42));
    assert!(session.nth_match(1).is_none());
}

#[test]
fn session_matches_reconstructs_the_full_width_value_of_a_multi_byte_match() {
    // Reproduces the reported bug: an Integer32 EqualTo scan for 54276 (0x0000_d404) must report
    // the full value, not just its low byte (0x04 = 4).
    let mut session = empty_session();
    let expr = ScanExpr {
        data_type: ScanDataType::Integer32,
        match_type: MatchType::EqualTo,
        criterion: ScanCriterion::Value(UserValue::Number(parse_int("54276").unwrap())),
    };
    let bytes = 54276i32.to_ne_bytes();
    for (address, byte, flags) in
        scan_buffer_chunk(0x1000, &bytes, &expr, Endianness::Native, usize::MAX)
    {
        session.matches.add(address, byte, flags);
    }

    let matches: Vec<MatchView> = session.matches().collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].address, 0x1000);
    assert_eq!(matches[0].old_value, Value::I32(54276));

    let first = session.nth_match(0).expect("one match recorded");
    assert_eq!(first.old_value, Value::I32(54276));
}

#[test]
fn reconstruct_value_prefers_signed_wide_and_float_at_matching_widths() {
    let bytes = 54276i32.to_ne_bytes();
    assert_eq!(
        reconstruct_value(
            &bytes,
            MatchFlags::U32 | MatchFlags::S32,
            Endianness::Native
        ),
        Value::I32(54276)
    );

    let float_bytes = 1.5f64.to_ne_bytes();
    assert_eq!(
        reconstruct_value(&float_bytes, MatchFlags::F64, Endianness::Native),
        Value::F64(1.5)
    );
}

#[test]
fn reconstruct_value_treats_a_multi_byte_all_flags_match_as_raw_bytes() {
    let bytes = b"hi!!".to_vec();
    assert_eq!(
        reconstruct_value(&bytes, MatchFlags::all(), Endianness::Native),
        Value::Bytes(bytes)
    );
}

#[test]
fn reconstruct_value_treats_a_single_byte_all_flags_candidate_as_signed() {
    // What a lone `AnyNumber`+`Any` snapshot byte looks like before any narrowing scan — signed
    // is preferred over unsigned at the same width, same as every other tie.
    assert_eq!(
        reconstruct_value(&[7], MatchFlags::all(), Endianness::Native),
        Value::I8(7)
    );
}

#[test]
fn reconstruct_value_falls_back_to_the_widest_decodable_width_when_flags_overclaim() {
    // Only reachable via hand-built `SwathEntry`s: flags claim a 4-byte width but only 2 bytes
    // were actually recorded.
    assert_eq!(
        reconstruct_value(&[42, 0], MatchFlags::U32, Endianness::Native),
        Value::U16(42)
    );
}

#[test]
fn session_delete_in_range_reports_how_many_matches_were_removed() {
    let mut session = empty_session();
    session.matches.add(0x1000, 42, MatchFlags::U8);
    session.matches.add(0x2000, 7, MatchFlags::U8);

    let removed = session.delete_in_range(0x1000..0x1001);
    assert_eq!(removed, 1);
    assert_eq!(session.matches().count(), 1);
}

#[test]
fn session_operations_without_attach_fail_with_not_attached() {
    let mut session = empty_session();
    assert!(matches!(
        session.read(0x1000, 4),
        Err(ScanmemError::NotAttached)
    ));
    assert!(matches!(
        session.write(0x1000, &Value::U8(1)),
        Err(ScanmemError::NotAttached)
    ));
    assert!(matches!(session.detach(), Err(ScanmemError::NotAttached)));
    assert!(matches!(
        session.region_count(),
        Err(ScanmemError::NotAttached)
    ));
}

#[test]
fn scan_on_a_detached_session_fails_with_not_attached() {
    let mut session = empty_session();
    let result = session.scan(&equal_to_expr("42"));
    assert!(matches!(result, Err(ScanmemError::NotAttached)));
}

#[test]
fn attach_to_a_nonexistent_pid_fails() {
    let pid = Pid::from_raw(i32::MAX - 1).expect("pid literal is non-zero");
    assert!(Session::attach(pid).is_err());
}
