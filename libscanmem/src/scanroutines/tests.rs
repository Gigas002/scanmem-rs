use super::*;

fn nv_u8(v: u8) -> NumberValue {
    NumberValue {
        u8: Some(v),
        ..Default::default()
    }
}

fn nv_i32(v: i32) -> NumberValue {
    NumberValue {
        i32: Some(v),
        ..Default::default()
    }
}

fn nv_u32(v: u32) -> NumberValue {
    NumberValue {
        u32: Some(v),
        ..Default::default()
    }
}

/// Populates both the `u32` and `i32` widths, mirroring how `parse_int` treats a literal that fits both.
fn nv_both32(v: u32) -> NumberValue {
    NumberValue {
        u32: Some(v),
        i32: i32::try_from(v).ok(),
        ..Default::default()
    }
}

fn nv_f32(v: f32) -> NumberValue {
    NumberValue {
        f32: Some(v),
        ..Default::default()
    }
}

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

// --- Integer32: one match type per test, exercising every MatchType against Integer32. ---

#[test]
fn integer32_any_matches_regardless_of_criteria() {
    let mem = le32(42);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Any,
        Endianness::Native,
        None,
        None,
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));
}

#[test]
fn integer32_equal_to_matches_only_the_width_that_equals() {
    let mem = le32(u32::MAX); // = -1 as i32, = u32::MAX as u32
    let user = nv_i32(-1);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::S32)));

    let user = nv_u32(u32::MAX);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32)));
}

#[test]
fn integer32_equal_to_reports_no_match() {
    let mem = le32(42);
    let user = nv_u32(7);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, None);
}

#[test]
fn integer32_not_equal_to() {
    let mem = le32(42);
    let user = nv_both32(7);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::NotEqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));
}

#[test]
fn integer32_greater_than() {
    let mem = le32(42);
    let user = nv_both32(7);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::GreaterThan,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));

    let user = nv_both32(100);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::GreaterThan,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, None);
}

#[test]
fn integer32_less_than() {
    let mem = le32(42);
    let user = nv_both32(100);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::LessThan,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));
}

#[test]
fn integer32_range_inside_and_outside_bounds() {
    let mem = le32(42);
    let lo = nv_both32(40);
    let hi = nv_both32(50);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Range,
        Endianness::Native,
        None,
        None,
        Some((&lo, &hi)),
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));

    let lo = nv_both32(100);
    let hi = nv_both32(200);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Range,
        Endianness::Native,
        None,
        None,
        Some((&lo, &hi)),
    );
    assert_eq!(result, None);
}

#[test]
fn integer32_update_requires_a_previous_value_for_that_width() {
    let mem = le32(42);
    let old = nv_u32(0);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Update,
        Endianness::Native,
        Some(&old),
        None,
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32)));

    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Update,
        Endianness::Native,
        None,
        None,
        None,
    );
    assert_eq!(result, None);
}

#[test]
fn integer32_not_changed_and_changed() {
    let mem = le32(42);
    let old = nv_u32(42);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::NotChanged,
        Endianness::Native,
        Some(&old),
        None,
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32)));

    let old = nv_u32(7);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Changed,
        Endianness::Native,
        Some(&old),
        None,
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32)));
}

#[test]
fn integer32_increased_and_decreased() {
    let mem = le32(42);
    let old = nv_both32(7);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Increased,
        Endianness::Native,
        Some(&old),
        None,
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));

    let old = nv_both32(100);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Decreased,
        Endianness::Native,
        Some(&old),
        None,
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));
}

#[test]
fn integer32_increased_by_and_decreased_by() {
    let mem = le32(42);
    let old = nv_both32(40);
    let delta = nv_both32(2);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::IncreasedBy,
        Endianness::Native,
        Some(&old),
        Some(&delta),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));

    let old = nv_both32(44);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::DecreasedBy,
        Endianness::Native,
        Some(&old),
        Some(&delta),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));
}

// --- Sanity per remaining IntegerN/FloatN widths. ---

#[test]
fn integer8_equal_to() {
    let mem = [200u8];
    let user = nv_u8(200);
    let result = scan(
        &mem,
        ScanDataType::Integer8,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((1, MatchFlags::U8)));
}

#[test]
fn integer16_equal_to() {
    let mem = 1000u16.to_le_bytes();
    let user = NumberValue {
        u16: Some(1000),
        ..Default::default()
    };
    let result = scan(
        &mem,
        ScanDataType::Integer16,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((2, MatchFlags::U16)));
}

#[test]
fn integer64_equal_to() {
    let mem = 123_456_789_012u64.to_le_bytes();
    let user = NumberValue {
        u64: Some(123_456_789_012),
        ..Default::default()
    };
    let result = scan(
        &mem,
        ScanDataType::Integer64,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((8, MatchFlags::U64)));
}

#[test]
fn float32_equal_to() {
    let mem = 3.5f32.to_le_bytes();
    let user = nv_f32(3.5);
    let result = scan(
        &mem,
        ScanDataType::Float32,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::F32)));
}

#[test]
fn float64_equal_to() {
    let mem = 3.5f64.to_le_bytes();
    let user = NumberValue {
        f64: Some(3.5),
        ..Default::default()
    };
    let result = scan(
        &mem,
        ScanDataType::Float64,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((8, MatchFlags::F64)));
}

// --- Any* aggregation across widths. ---

#[test]
fn any_integer_matches_the_widest_width_that_fits_and_equals() {
    let mem = le32(42);
    let user = nv_both32(42);
    let result = scan(
        &mem,
        ScanDataType::AnyInteger,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32 | MatchFlags::S32)));
}

#[test]
fn any_float_matches_across_both_float_widths() {
    let mem = 1.0f32.to_le_bytes();
    let user = NumberValue {
        f32: Some(1.0),
        f64: Some(1.0),
        ..Default::default()
    };
    let result = scan(
        &mem,
        ScanDataType::AnyFloat,
        MatchType::EqualTo,
        Endianness::Native,
        None,
        Some(&user),
        None,
    );
    // Only the 4-byte f32 interpretation fits in a 4-byte buffer.
    assert_eq!(result, Some((4, MatchFlags::F32)));
}

#[test]
fn any_number_prefers_the_any_match_over_no_match() {
    let mem = le32(42);
    let result = scan(
        &mem,
        ScanDataType::AnyNumber,
        MatchType::Any,
        Endianness::Native,
        None,
        None,
        None,
    );
    let (width, flags) = result.expect("Any always matches every width that fits");
    assert_eq!(width, 4);
    assert!(flags.contains(MatchFlags::U32 | MatchFlags::S32));
}

// --- Endianness. ---

#[test]
fn swapped_endianness_decodes_the_byte_reversed_value() {
    let mem = 0x0102_0304u32.to_be_bytes(); // big-endian bytes, host is assumed little-endian here
    let user = nv_u32(0x0102_0304);
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::EqualTo,
        Endianness::Swapped,
        None,
        Some(&user),
        None,
    );
    assert_eq!(result, Some((4, MatchFlags::U32)));
}

// --- Insufficient bytes / unsupported data types. ---

#[test]
fn returns_none_when_memory_is_shorter_than_the_width() {
    let mem = [0u8; 2];
    let result = scan(
        &mem,
        ScanDataType::Integer32,
        MatchType::Any,
        Endianness::Native,
        None,
        None,
        None,
    );
    assert_eq!(result, None);
}

#[test]
fn returns_none_for_bytearray_and_string_data_types() {
    let mem = le32(42);
    assert_eq!(
        scan(
            &mem,
            ScanDataType::ByteArray,
            MatchType::EqualTo,
            Endianness::Native,
            None,
            None,
            None
        ),
        None
    );
    assert_eq!(
        scan(
            &mem,
            ScanDataType::String,
            MatchType::EqualTo,
            Endianness::Native,
            None,
            None,
            None
        ),
        None
    );
}

// --- Byte array / string equal-to matching. ---

#[test]
fn scan_bytearray_matches_fixed_bytes() {
    let pattern = [ByteOrWildcard::Fixed(0xDE), ByteOrWildcard::Fixed(0xAD)];
    assert_eq!(scan_bytearray(&[0xDE, 0xAD, 0xBE, 0xEF], &pattern), Some(2));
}

#[test]
fn scan_bytearray_wildcard_matches_any_byte() {
    let pattern = [ByteOrWildcard::Fixed(0xDE), ByteOrWildcard::Wildcard];
    assert_eq!(scan_bytearray(&[0xDE, 0x00], &pattern), Some(2));
    assert_eq!(scan_bytearray(&[0xDE, 0xFF], &pattern), Some(2));
}

#[test]
fn scan_bytearray_rejects_mismatch_and_short_memory() {
    let pattern = [ByteOrWildcard::Fixed(0xDE), ByteOrWildcard::Fixed(0xAD)];
    assert_eq!(scan_bytearray(&[0xDE, 0xFF], &pattern), None);
    assert_eq!(scan_bytearray(&[0xDE], &pattern), None);
}

#[test]
fn scan_string_matches_prefix() {
    assert_eq!(scan_string(b"hello world", "hello"), Some(5));
}

#[test]
fn scan_string_rejects_mismatch_and_short_memory() {
    assert_eq!(scan_string(b"help", "hello"), None);
    assert_eq!(scan_string(b"he", "hello"), None);
}
