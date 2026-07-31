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
    T: Num + PartialOrd + Copy,
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
            .is_some_and(|(o, d)| memory == o + d),
        MatchType::DecreasedBy => old
            .zip(criterion.value())
            .is_some_and(|(o, d)| memory == o - d),
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
pub fn scan(
    memory: &[u8],
    data_type: ScanDataType,
    match_type: MatchType,
    endianness: Endianness,
    old: Option<&NumberValue>,
    user: Option<&NumberValue>,
    user_range: Option<(&NumberValue, &NumberValue)>,
) -> Option<(usize, MatchFlags)> {
    let mut flags = MatchFlags::empty();
    let mut best = 0usize;

    macro_rules! try_width {
        ($field:ident, $flag:expr) => {
            if let Some(mem_val) = $field::decode(memory, endianness) {
                let old_val = old.and_then(|o| o.$field);
                let criterion = match match_type {
                    MatchType::Range => user_range
                        .and_then(|(lo, hi)| Some(NumberCriterion::Range(lo.$field?, hi.$field?)))
                        .unwrap_or(NumberCriterion::None),
                    _ => user
                        .and_then(|u| u.$field)
                        .map(NumberCriterion::Value)
                        .unwrap_or(NumberCriterion::None),
                };
                if matches_number(mem_val, old_val, match_type, criterion) {
                    flags.insert($flag);
                    best = best.max($field::WIDTH);
                }
            }
        };
    }

    match data_type {
        ScanDataType::Integer8 => {
            try_width!(u8, MatchFlags::U8);
            try_width!(i8, MatchFlags::S8);
        }
        ScanDataType::Integer16 => {
            try_width!(u16, MatchFlags::U16);
            try_width!(i16, MatchFlags::S16);
        }
        ScanDataType::Integer32 => {
            try_width!(u32, MatchFlags::U32);
            try_width!(i32, MatchFlags::S32);
        }
        ScanDataType::Integer64 => {
            try_width!(u64, MatchFlags::U64);
            try_width!(i64, MatchFlags::S64);
        }
        ScanDataType::Float32 => try_width!(f32, MatchFlags::F32),
        ScanDataType::Float64 => try_width!(f64, MatchFlags::F64),
        ScanDataType::AnyInteger => {
            try_width!(u8, MatchFlags::U8);
            try_width!(i8, MatchFlags::S8);
            try_width!(u16, MatchFlags::U16);
            try_width!(i16, MatchFlags::S16);
            try_width!(u32, MatchFlags::U32);
            try_width!(i32, MatchFlags::S32);
            try_width!(u64, MatchFlags::U64);
            try_width!(i64, MatchFlags::S64);
        }
        ScanDataType::AnyFloat => {
            try_width!(f32, MatchFlags::F32);
            try_width!(f64, MatchFlags::F64);
        }
        ScanDataType::AnyNumber => {
            try_width!(u8, MatchFlags::U8);
            try_width!(i8, MatchFlags::S8);
            try_width!(u16, MatchFlags::U16);
            try_width!(i16, MatchFlags::S16);
            try_width!(u32, MatchFlags::U32);
            try_width!(i32, MatchFlags::S32);
            try_width!(u64, MatchFlags::U64);
            try_width!(i64, MatchFlags::S64);
            try_width!(f32, MatchFlags::F32);
            try_width!(f64, MatchFlags::F64);
        }
        ScanDataType::ByteArray | ScanDataType::String => return None,
    }

    (best > 0).then_some((best, flags))
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

#[cfg(test)]
mod tests;
