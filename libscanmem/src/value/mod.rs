//! `Value`/`UserValue`/`MatchFlags` — replaces `value.c`/`value.h`.

use std::fmt;

use bitflags::bitflags;
use thiserror::Error;

bitflags! {
    /// Which numeric width(s) a value could be interpreted as.
    ///
    /// Bit layout mirrors the C `match_flags` enum so the aggregates below stay meaningful:
    /// each `Ixx` is the union of its unsigned/signed bit, and `INTEGER`/`FLOAT`/`ALL` are unions of those.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MatchFlags: u16 {
        const U8  = 1 << 0;
        const S8  = 1 << 1;
        const U16 = 1 << 2;
        const S16 = 1 << 3;
        const U32 = 1 << 4;
        const S32 = 1 << 5;
        const U64 = 1 << 6;
        const S64 = 1 << 7;
        const F32 = 1 << 8;
        const F64 = 1 << 9;

        const I8  = Self::U8.bits()  | Self::S8.bits();
        const I16 = Self::U16.bits() | Self::S16.bits();
        const I32 = Self::U32.bits() | Self::S32.bits();
        const I64 = Self::U64.bits() | Self::S64.bits();

        const INTEGER = Self::I8.bits() | Self::I16.bits() | Self::I32.bits() | Self::I64.bits();
        const FLOAT = Self::F32.bits() | Self::F64.bits();
        const ALL = Self::INTEGER.bits() | Self::FLOAT.bits();
    }
}

/// A single concrete value, e.g. a byte read from target memory or a resolved match.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    Bytes(Vec<u8>),
    Str(String),
}

impl Value {
    /// The single [`MatchFlags`] bit this value's own width/signedness corresponds to.
    ///
    /// [`Value::Bytes`] and [`Value::Str`] have no numeric width, so this is [`MatchFlags::empty`].
    pub fn flag(&self) -> MatchFlags {
        match self {
            Value::U8(_) => MatchFlags::U8,
            Value::I8(_) => MatchFlags::S8,
            Value::U16(_) => MatchFlags::U16,
            Value::I16(_) => MatchFlags::S16,
            Value::U32(_) => MatchFlags::U32,
            Value::I32(_) => MatchFlags::S32,
            Value::U64(_) => MatchFlags::U64,
            Value::I64(_) => MatchFlags::S64,
            Value::F32(_) => MatchFlags::F32,
            Value::F64(_) => MatchFlags::F64,
            Value::Bytes(_) | Value::Str(_) => MatchFlags::empty(),
        }
    }

    /// A numeric ordering key for sorting mixed-width matches by value (e.g. `gameconqueror`'s
    /// Match View "sort by value" column) — `Value` can't derive `Ord` itself since its float
    /// variants only implement `PartialOrd`. Every numeric variant widens to `f64`; non-numeric
    /// values (`Bytes`/`Str`) have no natural numeric order, so they sort after every numeric one.
    pub fn numeric_key(&self) -> f64 {
        match self {
            Value::U8(v) => f64::from(*v),
            Value::I8(v) => f64::from(*v),
            Value::U16(v) => f64::from(*v),
            Value::I16(v) => f64::from(*v),
            Value::U32(v) => f64::from(*v),
            Value::I32(v) => f64::from(*v),
            Value::U64(v) => *v as f64,
            Value::I64(v) => *v as f64,
            Value::F32(v) => f64::from(*v),
            Value::F64(v) => *v,
            Value::Bytes(_) | Value::Str(_) => f64::INFINITY,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::U8(v) => write!(f, "{v}"),
            Value::I8(v) => write!(f, "{v}"),
            Value::U16(v) => write!(f, "{v}"),
            Value::I16(v) => write!(f, "{v}"),
            Value::U32(v) => write!(f, "{v}"),
            Value::I32(v) => write!(f, "{v}"),
            Value::U64(v) => write!(f, "{v}"),
            Value::I64(v) => write!(f, "{v}"),
            Value::F32(v) => write!(f, "{v}"),
            Value::F64(v) => write!(f, "{v}"),
            Value::Bytes(bytes) => {
                let mut iter = bytes.iter();
                if let Some(first) = iter.next() {
                    write!(f, "{first:02x}")?;
                    for byte in iter {
                        write!(f, " {byte:02x}")?;
                    }
                }
                Ok(())
            }
            Value::Str(s) => write!(f, "{s}"),
        }
    }
}

/// Every numeric width a user-typed number could simultaneously be interpreted as.
///
/// A literal like `"42"` fits `u8`/`i8`/.../`u64`/`i64` all at once — [`NumberValue::flags`]
/// reports which of those interpretations are actually available.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NumberValue {
    pub u8: Option<u8>,
    pub i8: Option<i8>,
    pub u16: Option<u16>,
    pub i16: Option<i16>,
    pub u32: Option<u32>,
    pub i32: Option<i32>,
    pub u64: Option<u64>,
    pub i64: Option<i64>,
    pub f32: Option<f32>,
    pub f64: Option<f64>,
}

impl NumberValue {
    /// The set of widths that were successfully populated during parsing.
    pub fn flags(&self) -> MatchFlags {
        let mut flags = MatchFlags::empty();
        flags.set(MatchFlags::U8, self.u8.is_some());
        flags.set(MatchFlags::S8, self.i8.is_some());
        flags.set(MatchFlags::U16, self.u16.is_some());
        flags.set(MatchFlags::S16, self.i16.is_some());
        flags.set(MatchFlags::U32, self.u32.is_some());
        flags.set(MatchFlags::S32, self.i32.is_some());
        flags.set(MatchFlags::U64, self.u64.is_some());
        flags.set(MatchFlags::S64, self.i64.is_some());
        flags.set(MatchFlags::F32, self.f32.is_some());
        flags.set(MatchFlags::F64, self.f64.is_some());
        flags
    }
}

/// One entry of a `BYTEARRAY` scan pattern: a fixed byte to match exactly, or `??` to match anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrWildcard {
    Fixed(u8),
    Wildcard,
}

/// A value as typed by the user, before it is narrowed to a single [`Value`] width.
#[derive(Debug, Clone, PartialEq)]
pub enum UserValue {
    Number(NumberValue),
    Bytes(Vec<ByteOrWildcard>),
    Str(String),
}

/// Failure parsing user-provided scan/write input into a [`UserValue`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValueParseError {
    #[error("{0:?} is not a valid integer")]
    InvalidInt(String),
    #[error("{0:?} does not fit any supported integer width")]
    IntOutOfRange(String),
    #[error("{0:?} is not a valid floating-point number")]
    InvalidFloat(String),
    #[error("byte pattern entry {0:?} is not two hex digits or `??`")]
    InvalidByte(String),
    #[error("byte pattern must have at least one entry")]
    EmptyByteArray,
}

/// Parses a decimal or `0x`-prefixed hexadecimal integer, populating every width it fits in.
pub fn parse_int(input: &str) -> Result<NumberValue, ValueParseError> {
    let (negative, magnitude) = parse_signed_magnitude(input)
        .ok_or_else(|| ValueParseError::InvalidInt(input.to_owned()))?;

    let mut value = NumberValue::default();

    if !negative {
        value.u8 = u8::try_from(magnitude).ok();
        value.u16 = u16::try_from(magnitude).ok();
        value.u32 = u32::try_from(magnitude).ok();
        value.u64 = u64::try_from(magnitude).ok();
    }

    let signed = if negative {
        i128::try_from(magnitude).ok().map(|m| -m)
    } else {
        i128::try_from(magnitude).ok()
    };
    if let Some(signed) = signed {
        value.i8 = i8::try_from(signed).ok();
        value.i16 = i16::try_from(signed).ok();
        value.i32 = i32::try_from(signed).ok();
        value.i64 = i64::try_from(signed).ok();
    }

    if value.flags().is_empty() {
        return Err(ValueParseError::IntOutOfRange(input.to_owned()));
    }

    Ok(value)
}

/// Parses a floating-point literal (whatever Rust's `f64` `FromStr` accepts).
pub fn parse_float(input: &str) -> Result<NumberValue, ValueParseError> {
    let trimmed = input.trim();
    let parsed: f64 = trimmed
        .parse()
        .map_err(|_| ValueParseError::InvalidFloat(input.to_owned()))?;

    Ok(NumberValue {
        f32: Some(parsed as f32),
        f64: Some(parsed),
        ..NumberValue::default()
    })
}

/// Parses either an integer or a float, widening the result to every representation it fits:
/// an integer literal also gets its float value filled in, and a whole-number float literal
/// also gets every integer width it fits in filled in.
pub fn parse_number(input: &str) -> Result<NumberValue, ValueParseError> {
    if let Ok(mut value) = parse_int(input) {
        let whole = value
            .i64
            .map(|v| v as f64)
            .or(value.u64.map(|v| v as f64))
            .expect("parse_int always populates at least one 64-bit width");
        value.f32 = Some(whole as f32);
        value.f64 = Some(whole);
        return Ok(value);
    }

    let mut value = parse_float(input)?;
    let as_f64 = value.f64.expect("parse_float always populates f64");
    if as_f64.fract() == 0.0
        && let Ok(int_value) = parse_int(&format!("{as_f64:.0}"))
    {
        value.u8 = int_value.u8;
        value.i8 = int_value.i8;
        value.u16 = int_value.u16;
        value.i16 = int_value.i16;
        value.u32 = int_value.u32;
        value.i32 = int_value.i32;
        value.u64 = int_value.u64;
        value.i64 = int_value.i64;
    }
    Ok(value)
}

/// Parses a `BYTEARRAY` pattern from already-split tokens, each either two hex digits or `??`.
pub fn parse_bytearray<'a>(
    tokens: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<ByteOrWildcard>, ValueParseError> {
    let pattern = tokens
        .into_iter()
        .map(|token| match token {
            "??" => Ok(ByteOrWildcard::Wildcard),
            _ if token.len() == 2 => u8::from_str_radix(token, 16)
                .map(ByteOrWildcard::Fixed)
                .map_err(|_| ValueParseError::InvalidByte(token.to_owned())),
            _ => Err(ValueParseError::InvalidByte(token.to_owned())),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if pattern.is_empty() {
        return Err(ValueParseError::EmptyByteArray);
    }

    Ok(pattern)
}

/// Wraps arbitrary user input as a `STRING` scan/write value; strings never fail to parse.
pub fn parse_string(input: &str) -> UserValue {
    UserValue::Str(input.to_owned())
}

/// Splits an optional sign and `0x`/`0X` hex prefix off `input`, returning `(negative, magnitude)`.
fn parse_signed_magnitude(input: &str) -> Option<(bool, u128)> {
    let trimmed = input.trim();
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let (radix, digits) = match rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        Some(hex) => (16, hex),
        None => (10, rest),
    };
    if digits.is_empty() {
        return None;
    }
    u128::from_str_radix(digits, radix)
        .ok()
        .map(|magnitude| (negative, magnitude))
}

#[cfg(test)]
mod tests;
