use super::*;

#[test]
fn parse_int_decimal_sets_every_width_it_fits() {
    let value = parse_int("123").unwrap();
    assert_eq!(value.u8, Some(123));
    assert_eq!(value.i8, Some(123));
    assert_eq!(value.u64, Some(123));
    assert_eq!(value.i64, Some(123));
    assert!(value.flags().contains(MatchFlags::INTEGER));
    assert!(!value.flags().intersects(MatchFlags::FLOAT));
}

#[test]
fn parse_int_hex_prefix() {
    let value = parse_int("0xFF").unwrap();
    assert_eq!(value.u8, Some(0xFF));
    assert_eq!(value.i8, None); // 255 doesn't fit i8
    assert_eq!(value.u16, Some(0xFF));
}

#[test]
fn parse_int_negative_sets_only_signed_widths() {
    let value = parse_int("-5").unwrap();
    assert_eq!(value.i8, Some(-5));
    assert_eq!(value.i64, Some(-5));
    assert_eq!(value.u8, None);
    assert_eq!(value.u64, None);
}

#[test]
fn parse_int_rejects_malformed_input() {
    assert_eq!(
        parse_int("abc"),
        Err(ValueParseError::InvalidInt("abc".to_owned()))
    );
    assert_eq!(
        parse_int(""),
        Err(ValueParseError::InvalidInt(String::new()))
    );
    assert_eq!(
        parse_int("12.5"),
        Err(ValueParseError::InvalidInt("12.5".to_owned()))
    );
}

#[test]
fn parse_int_rejects_values_out_of_every_width() {
    // 2^64, fits in u128 but not any supported width (max is 64-bit).
    let input = "18446744073709551616";
    assert_eq!(
        parse_int(input),
        Err(ValueParseError::IntOutOfRange(input.to_owned()))
    );
}

#[test]
fn parse_float_basic() {
    let value = parse_float("3.5").unwrap();
    assert_eq!(value.f32, Some(3.5));
    assert_eq!(value.f64, Some(3.5));
    assert_eq!(value.u8, None);
}

#[test]
fn parse_float_rejects_malformed_input() {
    assert_eq!(
        parse_float("abc"),
        Err(ValueParseError::InvalidFloat("abc".to_owned()))
    );
}

#[test]
fn parse_number_widens_integer_literal_with_floats() {
    let value = parse_number("42").unwrap();
    assert_eq!(value.u8, Some(42));
    assert_eq!(value.f64, Some(42.0));
}

#[test]
fn parse_number_widens_whole_float_literal_with_integers() {
    let value = parse_number("42.0").unwrap();
    assert_eq!(value.f64, Some(42.0));
    assert_eq!(value.u8, Some(42));
    assert_eq!(value.i64, Some(42));
}

#[test]
fn parse_number_does_not_widen_fractional_float_with_integers() {
    let value = parse_number("3.5").unwrap();
    assert_eq!(value.f64, Some(3.5));
    assert_eq!(value.u8, None);
    assert_eq!(value.i64, None);
}

#[test]
fn parse_bytearray_fixed_and_wildcard() {
    let pattern = parse_bytearray(["de", "??", "EF"]).unwrap();
    assert_eq!(
        pattern,
        vec![
            ByteOrWildcard::Fixed(0xde),
            ByteOrWildcard::Wildcard,
            ByteOrWildcard::Fixed(0xef),
        ]
    );
}

#[test]
fn parse_bytearray_rejects_bad_tokens() {
    assert_eq!(
        parse_bytearray(["zz"]),
        Err(ValueParseError::InvalidByte("zz".to_owned()))
    );
    assert_eq!(
        parse_bytearray(["a"]),
        Err(ValueParseError::InvalidByte("a".to_owned()))
    );
}

#[test]
fn parse_bytearray_rejects_empty_pattern() {
    assert_eq!(parse_bytearray([]), Err(ValueParseError::EmptyByteArray));
}

#[test]
fn parse_string_never_fails() {
    assert_eq!(parse_string("hello"), UserValue::Str("hello".to_owned()));
}

#[test]
fn value_flag_matches_its_own_width() {
    assert_eq!(Value::U8(1).flag(), MatchFlags::U8);
    assert_eq!(Value::I64(-1).flag(), MatchFlags::S64);
    assert_eq!(Value::Bytes(vec![1, 2]).flag(), MatchFlags::empty());
}

#[test]
fn value_display_round_trips_through_the_underlying_type() {
    let cases: &[(Value, &str)] = &[
        (Value::U8(42), "42"),
        (Value::I8(-1), "-1"),
        (Value::F64(3.5), "3.5"),
        (Value::Str("hi".to_owned()), "hi"),
        (Value::Bytes(vec![0xde, 0xad]), "de ad"),
    ];
    for (value, expected) in cases {
        assert_eq!(&value.to_string(), expected);
    }
}

#[test]
fn match_flags_aggregate_bit_layout() {
    assert_eq!(MatchFlags::I8, MatchFlags::U8 | MatchFlags::S8);
    assert_eq!(
        MatchFlags::INTEGER,
        MatchFlags::I8 | MatchFlags::I16 | MatchFlags::I32 | MatchFlags::I64
    );
    assert_eq!(MatchFlags::ALL, MatchFlags::INTEGER | MatchFlags::FLOAT);
}
