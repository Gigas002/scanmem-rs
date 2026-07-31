//! Comma/range index-set parser (e.g. `"1,3-5,8"`, `"!2-4"`) — replaces `sets.c`/`sets.h`.

use std::collections::BTreeSet;

use thiserror::Error;

/// Failure parsing an index-set expression.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SetParseError {
    #[error("set must have at least one index")]
    Empty,
    #[error("`!` inversion must appear at the start of the set")]
    InvertNotAtStart,
    #[error("inverting a set spanning the entire range 0..{0} would leave nothing")]
    InvertsEntireRange(usize),
    #[error("index {0} is out of bounds (size is {1})")]
    OutOfBounds(usize, usize),
    #[error("duplicate index {0}")]
    Duplicate(usize),
    #[error("range {0:?} runs backwards (end before start)")]
    InvalidRange(String),
    #[error("invalid token {0:?}")]
    InvalidToken(String),
}

/// Parses a comma/range index-set expression, e.g. `"1,3-5,8"`, bounded by `size` (exclusive).
///
/// A leading `!` inverts the set to every index in `0..size` *not* listed. Individual entries
/// are either a single index or an inclusive `start-end` range; indices may be decimal or
/// `0x`-prefixed hexadecimal. Duplicate indices are rejected rather than silently merged.
pub fn parse_index_set(input: &str, size: usize) -> Result<BTreeSet<usize>, SetParseError> {
    let trimmed = input.trim();
    let (invert, body) = match trimmed.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, trimmed),
    };
    if body.contains('!') {
        return Err(SetParseError::InvertNotAtStart);
    }
    if body.is_empty() {
        return Err(SetParseError::Empty);
    }

    let mut indices = BTreeSet::new();
    for token in body.split(',') {
        let token = token.trim();
        let (start, end) = match token.split_once('-') {
            Some((start, end)) => (parse_index(start, token)?, parse_index(end, token)?),
            None => {
                let index = parse_index(token, token)?;
                (index, index)
            }
        };
        if end < start {
            return Err(SetParseError::InvalidRange(token.to_owned()));
        }
        for index in start..=end {
            if index >= size {
                return Err(SetParseError::OutOfBounds(index, size));
            }
            if !indices.insert(index) {
                return Err(SetParseError::Duplicate(index));
            }
        }
    }

    if !invert {
        return Ok(indices);
    }

    if indices.len() == size {
        return Err(SetParseError::InvertsEntireRange(size));
    }
    Ok((0..size).filter(|index| !indices.contains(index)).collect())
}

/// Parses a single decimal or `0x`-prefixed hexadecimal index. `token` is the whole token used
/// in error messages, which may differ from `text` (e.g. one side of a `start-end` range).
fn parse_index(text: &str, token: &str) -> Result<usize, SetParseError> {
    let text = text.trim();
    let (radix, digits) = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => (16, hex),
        None => (10, text),
    };
    usize::from_str_radix(digits, radix).map_err(|_| SetParseError::InvalidToken(token.to_owned()))
}

#[cfg(test)]
mod tests;
