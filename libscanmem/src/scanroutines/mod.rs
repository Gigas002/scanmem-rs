//! Generic scan-match routines — replaces `scanroutines.c`/`scanroutines.h`.

use num_traits::Num;

use crate::value::{ByteOrWildcard, MatchFlags, NumberValue};

/// Byte order to interpret raw memory in, replacing the C code's global reverse-endianness flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    /// The host's native byte order.
    Native,
    /// The host's byte order reversed, e.g. to scan a target of the opposite endianness.
    Swapped,
}

/// Replaces `scan_match_type_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Matches unconditionally; used for an initial, unconstrained scan.
    Any,
    EqualTo,
    NotEqualTo,
    GreaterThan,
    LessThan,
    Range,
    /// Matches whenever a value existed for this width in the previous scan.
    Update,
    NotChanged,
    Changed,
    Increased,
    Decreased,
    IncreasedBy,
    DecreasedBy,
}

/// Replaces `scan_data_type_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDataType {
    AnyNumber,
    AnyInteger,
    AnyFloat,
    Integer8,
    Integer16,
    Integer32,
    Integer64,
    Float32,
    Float64,
    ByteArray,
    String,
}

/// A user- or previous-scan-supplied criterion a decoded memory value is compared against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberCriterion<T> {
    None,
    Value(T),
    Range(T, T),
}

impl<T: Copy> NumberCriterion<T> {
    fn value(&self) -> Option<T> {
        match self {
            NumberCriterion::Value(v) => Some(*v),
            _ => None,
        }
    }

    fn range(&self) -> Option<(T, T)> {
        match self {
            NumberCriterion::Range(lo, hi) => Some((*lo, *hi)),
            _ => None,
        }
    }
}

/// Decodes a fixed-width numeric type out of a byte slice, honoring an explicit [`Endianness`].
trait DecodeBytes: Sized {
    const WIDTH: usize;

    fn decode(bytes: &[u8], endianness: Endianness) -> Option<Self>;
}

macro_rules! impl_decode_bytes_int {
    ($($t:ty),+ $(,)?) => {
        $(
            impl DecodeBytes for $t {
                const WIDTH: usize = std::mem::size_of::<$t>();

                fn decode(bytes: &[u8], endianness: Endianness) -> Option<Self> {
                    let bytes: [u8; std::mem::size_of::<$t>()] = bytes.get(..Self::WIDTH)?.try_into().ok()?;
                    let value = <$t>::from_ne_bytes(bytes);
                    Some(match endianness {
                        Endianness::Native => value,
                        Endianness::Swapped => value.swap_bytes(),
                    })
                }
            }
        )+
    };
}

impl_decode_bytes_int!(u8, i8, u16, i16, u32, i32, u64, i64);

macro_rules! impl_decode_bytes_float {
    ($(($t:ty, $bits:ty)),+ $(,)?) => {
        $(
            impl DecodeBytes for $t {
                const WIDTH: usize = std::mem::size_of::<$t>();

                fn decode(bytes: &[u8], endianness: Endianness) -> Option<Self> {
                    let bytes: [u8; std::mem::size_of::<$t>()] = bytes.get(..Self::WIDTH)?.try_into().ok()?;
                    let bits = <$bits>::from_ne_bytes(bytes);
                    let bits = match endianness {
                        Endianness::Native => bits,
                        Endianness::Swapped => bits.swap_bytes(),
                    };
                    Some(<$t>::from_bits(bits))
                }
            }
        )+
    };
}

impl_decode_bytes_float!((f32, u32), (f64, u64));

/// `old ± delta` for [`MatchType::IncreasedBy`]/[`MatchType::DecreasedBy`]: wrapping for
/// integers, so a counter that wraps around still matches (as in C scanmem) instead of
/// overflowing, which panics in debug builds; plain arithmetic for floats.
pub trait Delta: Copy {
    fn plus(self, delta: Self) -> Self;
    fn minus(self, delta: Self) -> Self;
}

macro_rules! impl_delta_int {
    ($($t:ty),+ $(,)?) => {
        $(
            impl Delta for $t {
                fn plus(self, delta: Self) -> Self {
                    self.wrapping_add(delta)
                }

                fn minus(self, delta: Self) -> Self {
                    self.wrapping_sub(delta)
                }
            }
        )+
    };
}

impl_delta_int!(u8, i8, u16, i16, u32, i32, u64, i64);

macro_rules! impl_delta_float {
    ($($t:ty),+ $(,)?) => {
        $(
            impl Delta for $t {
                fn plus(self, delta: Self) -> Self {
                    self + delta
                }

                fn minus(self, delta: Self) -> Self {
                    self - delta
                }
            }
        )+
    };
}

impl_delta_float!(f32, f64);

/// Compares one already-decoded memory value against `old`/`criterion` for `match_type`.
///
/// This single generic routine is what every `IntegerN`/`FloatN`/`Any*` combination in [`scan`]
/// instantiates, replacing the per-width, per-match-type macro expansions in the C source.
pub fn matches_number<T>(
    memory: T,
    old: Option<T>,
    match_type: MatchType,
    criterion: NumberCriterion<T>,
) -> bool
where
    T: Num + PartialOrd + Delta,
{
    match match_type {
        MatchType::Any => true,
        MatchType::EqualTo => criterion.value().is_some_and(|c| memory == c),
        MatchType::NotEqualTo => criterion.value().is_some_and(|c| memory != c),
        MatchType::GreaterThan => criterion.value().is_some_and(|c| memory > c),
        MatchType::LessThan => criterion.value().is_some_and(|c| memory < c),
        MatchType::Range => criterion
            .range()
            .is_some_and(|(lo, hi)| memory >= lo && memory <= hi),
        MatchType::Update => old.is_some(),
        MatchType::NotChanged => old.is_some_and(|o| memory == o),
        MatchType::Changed => old.is_some_and(|o| memory != o),
        MatchType::Increased => old.is_some_and(|o| memory > o),
        MatchType::Decreased => old.is_some_and(|o| memory < o),
        MatchType::IncreasedBy => old
            .zip(criterion.value())
            .is_some_and(|(o, d)| memory == o.plus(d)),
        MatchType::DecreasedBy => old
            .zip(criterion.value())
            .is_some_and(|(o, d)| memory == o.minus(d)),
    }
}

/// Scans one memory position for a numeric [`ScanDataType`], returning the widest matching width
/// in bytes and which specific widths matched, or `None` if nothing matched (or `data_type` is
/// [`ScanDataType::ByteArray`]/[`ScanDataType::String`] — use [`scan_bytearray`]/[`scan_string`]
/// for those).
///
/// `user_range` supplies the `(low, high)` bounds for [`MatchType::Range`]; `user` supplies the
/// single comparison value (or, for `IncreasedBy`/`DecreasedBy`, the delta) for every other
/// value-comparing match type.
///
/// Scanning many positions with the same arguments is faster through [`Matcher`], which this
/// builds for one position.
pub fn scan(
    memory: &[u8],
    data_type: ScanDataType,
    match_type: MatchType,
    endianness: Endianness,
    old: Option<&NumberValue>,
    user: Option<&NumberValue>,
    user_range: Option<(&NumberValue, &NumberValue)>,
) -> Option<(usize, MatchFlags)> {
    Matcher::new(data_type, match_type, endianness, user, user_range).test(memory, old)
}

/// One width a [`Matcher`] tests, with its comparison value already picked out of the user's
/// [`NumberValue`].
#[derive(Debug, Clone, Copy)]
enum WidthTest {
    U8(NumberCriterion<u8>),
    I8(NumberCriterion<i8>),
    U16(NumberCriterion<u16>),
    I16(NumberCriterion<i16>),
    U32(NumberCriterion<u32>),
    I32(NumberCriterion<i32>),
    U64(NumberCriterion<u64>),
    I64(NumberCriterion<i64>),
    F32(NumberCriterion<f32>),
    F64(NumberCriterion<f64>),
}

/// [`scan`] prepared once for a whole scan: the widths `data_type` covers, each with its
/// comparison value resolved, so testing a position only decodes and compares.
#[derive(Debug, Clone)]
pub struct Matcher {
    match_type: MatchType,
    endianness: Endianness,
    widths: Vec<WidthTest>,
    /// For [`MatchType::EqualTo`]: which first bytes some width's value can start with.
    first_bytes: Option<Box<[bool; 256]>>,
    /// For [`MatchType::Any`]: the result for any position with 8 or more bytes left, where
    /// every width decodes and so matches.
    any_full: Option<(usize, MatchFlags)>,
}

impl Matcher {
    /// Same arguments as [`scan`], minus the memory and the previous value.
    pub fn new(
        data_type: ScanDataType,
        match_type: MatchType,
        endianness: Endianness,
        user: Option<&NumberValue>,
        user_range: Option<(&NumberValue, &NumberValue)>,
    ) -> Self {
        // A match type that compares against a value never matches a width the value doesn't
        // fit (`matches_number` needs the criterion); leaving such widths out changes nothing.
        let needs_criterion = !matches!(
            match_type,
            MatchType::Any
                | MatchType::Update
                | MatchType::NotChanged
                | MatchType::Changed
                | MatchType::Increased
                | MatchType::Decreased
        );
        let mut widths = Vec::new();

        macro_rules! width {
            ($field:ident, $variant:ident) => {{
                let criterion = match match_type {
                    MatchType::Range => user_range
                        .and_then(|(lo, hi)| Some(NumberCriterion::Range(lo.$field?, hi.$field?)))
                        .unwrap_or(NumberCriterion::None),
                    _ => user
                        .and_then(|u| u.$field)
                        .map(NumberCriterion::Value)
                        .unwrap_or(NumberCriterion::None),
                };
                if !(needs_criterion && matches!(criterion, NumberCriterion::None)) {
                    widths.push(WidthTest::$variant(criterion));
                }
            }};
        }

        match data_type {
            ScanDataType::Integer8 => {
                width!(u8, U8);
                width!(i8, I8);
            }
            ScanDataType::Integer16 => {
                width!(u16, U16);
                width!(i16, I16);
            }
            ScanDataType::Integer32 => {
                width!(u32, U32);
                width!(i32, I32);
            }
            ScanDataType::Integer64 => {
                width!(u64, U64);
                width!(i64, I64);
            }
            ScanDataType::Float32 => width!(f32, F32),
            ScanDataType::Float64 => width!(f64, F64),
            ScanDataType::AnyInteger => {
                width!(u8, U8);
                width!(i8, I8);
                width!(u16, U16);
                width!(i16, I16);
                width!(u32, U32);
                width!(i32, I32);
                width!(u64, U64);
                width!(i64, I64);
            }
            ScanDataType::AnyFloat => {
                width!(f32, F32);
                width!(f64, F64);
            }
            ScanDataType::AnyNumber => {
                width!(u8, U8);
                width!(i8, I8);
                width!(u16, U16);
                width!(i16, I16);
                width!(u32, U32);
                width!(i32, I32);
                width!(u64, U64);
                width!(i64, I64);
                width!(f32, F32);
                width!(f64, F64);
            }
            ScanDataType::ByteArray | ScanDataType::String => {}
        }

        let mut matcher = Self {
            match_type,
            endianness,
            widths,
            first_bytes: None,
            any_full: None,
        };
        if match_type == MatchType::EqualTo {
            matcher.first_bytes = Some(matcher.equal_to_first_bytes());
        }
        if match_type == MatchType::Any {
            matcher.any_full = matcher.test_each_width(&[0; 8], None);
        }
        matcher
    }

    /// The first byte of each width's value as it sits in memory. A float also equals the
    /// other zero (`-0.0 == 0.0`), and NaN equals nothing.
    fn equal_to_first_bytes(&self) -> Box<[bool; 256]> {
        let mut table = Box::new([false; 256]);
        let swapped = self.endianness == Endianness::Swapped;
        let mut mark = |ne_bytes: &[u8]| {
            let first = if swapped {
                ne_bytes[ne_bytes.len() - 1]
            } else {
                ne_bytes[0]
            };
            table[usize::from(first)] = true;
        };
        for width in &self.widths {
            match *width {
                WidthTest::U8(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::I8(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::U16(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::I16(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::U32(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::I32(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::U64(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::I64(NumberCriterion::Value(v)) => mark(&v.to_ne_bytes()),
                WidthTest::F32(NumberCriterion::Value(v)) => {
                    mark(&v.to_ne_bytes());
                    mark(&(-v).to_ne_bytes());
                }
                WidthTest::F64(NumberCriterion::Value(v)) => {
                    mark(&v.to_ne_bytes());
                    mark(&(-v).to_ne_bytes());
                }
                // Without a value, an equal-to width never matches.
                _ => {}
            }
        }
        table
    }

    /// Tests the value at the start of `memory` (and, for match types that compare with the
    /// previous scan, `old`): the widest matching width and every width that matched, or `None`.
    #[inline]
    pub fn test(&self, memory: &[u8], old: Option<&NumberValue>) -> Option<(usize, MatchFlags)> {
        if let Some(first_bytes) = &self.first_bytes {
            // A float's negation only differs in the sign, which is in another byte unless
            // byte-swapped; marking both covers `-0.0 == 0.0` either way, and a value that
            // isn't zero just adds a harmless candidate.
            if !first_bytes[usize::from(*memory.first()?)] {
                return None;
            }
        }
        if memory.len() >= 8
            && let Some(result) = self.any_full
        {
            return Some(result);
        }
        self.test_each_width(memory, old)
    }

    #[inline]
    fn test_each_width(
        &self,
        memory: &[u8],
        old: Option<&NumberValue>,
    ) -> Option<(usize, MatchFlags)> {
        let mut flags = MatchFlags::empty();
        let mut best = 0usize;

        macro_rules! check {
            ($t:ty, $field:ident, $flag:expr, $criterion:expr) => {
                if let Some(mem_val) = <$t>::decode(memory, self.endianness) {
                    let old_val = old.and_then(|o| o.$field);
                    if matches_number(mem_val, old_val, self.match_type, *$criterion) {
                        flags.insert($flag);
                        best = best.max(<$t>::WIDTH);
                    }
                }
            };
        }

        for width in &self.widths {
            match width {
                WidthTest::U8(c) => check!(u8, u8, MatchFlags::U8, c),
                WidthTest::I8(c) => check!(i8, i8, MatchFlags::S8, c),
                WidthTest::U16(c) => check!(u16, u16, MatchFlags::U16, c),
                WidthTest::I16(c) => check!(i16, i16, MatchFlags::S16, c),
                WidthTest::U32(c) => check!(u32, u32, MatchFlags::U32, c),
                WidthTest::I32(c) => check!(i32, i32, MatchFlags::S32, c),
                WidthTest::U64(c) => check!(u64, u64, MatchFlags::U64, c),
                WidthTest::I64(c) => check!(i64, i64, MatchFlags::S64, c),
                WidthTest::F32(c) => check!(f32, f32, MatchFlags::F32, c),
                WidthTest::F64(c) => check!(f64, f64, MatchFlags::F64, c),
            }
        }

        (best > 0).then_some((best, flags))
    }
}

/// Matches a fixed byte pattern (with `??` wildcards) at the start of `memory` — replaces
/// `scan_routine_BYTEARRAY*_EQUALTO`. Only equal-to matching is meaningful for a byte pattern, so
/// unlike [`scan`] this takes no `match_type` parameter.
pub fn scan_bytearray(memory: &[u8], pattern: &[ByteOrWildcard]) -> Option<usize> {
    if memory.len() < pattern.len() {
        return None;
    }
    let matched = pattern.iter().zip(memory).all(|(p, &m)| match p {
        ByteOrWildcard::Fixed(b) => *b == m,
        ByteOrWildcard::Wildcard => true,
    });
    matched.then_some(pattern.len())
}

/// Matches a fixed string at the start of `memory` — replaces `scan_routine_STRING*_EQUALTO`.
pub fn scan_string(memory: &[u8], pattern: &str) -> Option<usize> {
    let pattern = pattern.as_bytes();
    (memory.len() >= pattern.len() && &memory[..pattern.len()] == pattern).then_some(pattern.len())
}

/// Decodes every numeric width that fits in `bytes`, for reconstructing a previous scan's raw
/// bytes into the same [`NumberValue`] shape [`scan`] expects for its `old` parameter.
pub fn decode_number(bytes: &[u8], endianness: Endianness) -> NumberValue {
    NumberValue {
        u8: u8::decode(bytes, endianness),
        i8: i8::decode(bytes, endianness),
        u16: u16::decode(bytes, endianness),
        i16: i16::decode(bytes, endianness),
        u32: u32::decode(bytes, endianness),
        i32: i32::decode(bytes, endianness),
        u64: u64::decode(bytes, endianness),
        i64: i64::decode(bytes, endianness),
        f32: f32::decode(bytes, endianness),
        f64: f64::decode(bytes, endianness),
    }
}

#[cfg(test)]
mod tests;
