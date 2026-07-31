//! Parses one REPL/scripted line into a [`Command`] — the clean-slate grammar from the verb
//! table; no I/O, no `Session` access.

use libscanmem::scanroutines::{Endianness, MatchType, ScanDataType};
use libscanmem::session::{RegionFilter, ScanCriterion, ScanExpr, SessionOption};
use libscanmem::value::{self, ByteOrWildcard, UserValue, Value, ValueParseError};
use thiserror::Error;

use super::Command;

/// Failure parsing a REPL/scripted line into a [`Command`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("unknown command {0:?}")]
    UnknownVerb(String),
    #[error("{0} takes no arguments")]
    UnexpectedArguments(&'static str),
    #[error("{0} requires {1}")]
    MissingArgument(&'static str, &'static str),
    #[error("{0:?} is not a valid pid")]
    InvalidPid(String),
    #[error("{0:?} is not a valid address or index")]
    InvalidNumber(String),
    #[error("{0:?} is not a valid data type")]
    InvalidDataType(String),
    #[error("{0:?} is not a valid match type")]
    InvalidMatchType(String),
    #[error("{0:?} is not a valid write type")]
    InvalidValueType(String),
    #[error("{0:?} does not fit a {1}")]
    ValueOutOfRange(String, &'static str),
    #[error("{0:?} is not a valid option key")]
    InvalidOptionKey(String),
    #[error("{0:?} is not a valid option value")]
    InvalidOptionValue(String),
    #[error(transparent)]
    Value(#[from] ValueParseError),
}

/// Parses one non-empty, already-trimmed `line` into a [`Command`].
pub fn parse(line: &str) -> Result<Command, ParseError> {
    let (verb, rest) = split_first_word(line);
    match verb.to_ascii_lowercase().as_str() {
        "pid" | "attach" => parse_attach(rest),
        "scan" => parse_scan(rest),
        "snapshot" => no_arguments(rest, "snapshot").map(|()| Command::Snapshot),
        "list" => parse_list(rest),
        "dump" => parse_dump(rest),
        "write" => parse_write(rest),
        "delete" => parse_delete(rest),
        "option" => parse_option(rest),
        "reset" => no_arguments(rest, "reset").map(|()| Command::Reset),
        "help" => no_arguments(rest, "help").map(|()| Command::Help),
        "quit" | "exit" => no_arguments(rest, "quit").map(|()| Command::Quit),
        _ => Err(ParseError::UnknownVerb(verb.to_owned())),
    }
}

fn no_arguments(rest: &str, verb: &'static str) -> Result<(), ParseError> {
    if rest.trim().is_empty() {
        Ok(())
    } else {
        Err(ParseError::UnexpectedArguments(verb))
    }
}

fn parse_attach(rest: &str) -> Result<Command, ParseError> {
    let token = rest.trim();
    let pid = token
        .parse::<u32>()
        .map_err(|_| ParseError::InvalidPid(token.to_owned()))?;
    Ok(Command::Attach(pid))
}

fn parse_scan(rest: &str) -> Result<Command, ParseError> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let [data_type_token, match_type_token, value_tokens @ ..] = tokens.as_slice() else {
        return Err(ParseError::MissingArgument(
            "scan",
            "a data type and match type",
        ));
    };
    let data_type = parse_data_type(data_type_token)?;

    if matches!(data_type, ScanDataType::ByteArray | ScanDataType::String) {
        require_equal_to(match_type_token)?;
        let criterion = if data_type == ScanDataType::ByteArray {
            ScanCriterion::Value(UserValue::Bytes(value::parse_bytearray(
                value_tokens.iter().copied(),
            )?))
        } else if value_tokens.is_empty() {
            return Err(ParseError::MissingArgument("string scan", "a string value"));
        } else {
            ScanCriterion::Value(value::parse_string(&value_tokens.join(" ")))
        };
        return Ok(Command::Scan(ScanExpr {
            data_type,
            match_type: MatchType::EqualTo,
            criterion,
        }));
    }

    let match_type = parse_match_type(match_type_token)?;
    let criterion = numeric_criterion(match_type, value_tokens)?;
    Ok(Command::Scan(ScanExpr {
        data_type,
        match_type,
        criterion,
    }))
}

fn require_equal_to(token: &str) -> Result<(), ParseError> {
    if token == "=" || token.eq_ignore_ascii_case("eq") {
        Ok(())
    } else {
        Err(ParseError::InvalidMatchType(token.to_owned()))
    }
}

fn numeric_criterion(
    match_type: MatchType,
    value_tokens: &[&str],
) -> Result<ScanCriterion, ParseError> {
    match match_type {
        MatchType::Any
        | MatchType::Update
        | MatchType::NotChanged
        | MatchType::Changed
        | MatchType::Increased
        | MatchType::Decreased => {
            if value_tokens.is_empty() {
                Ok(ScanCriterion::None)
            } else {
                Err(ParseError::UnexpectedArguments("this match type"))
            }
        }
        MatchType::Range => match value_tokens {
            [low, high] => Ok(ScanCriterion::Range(
                value::parse_number(low)?,
                value::parse_number(high)?,
            )),
            _ => Err(ParseError::MissingArgument(
                "range match",
                "a low and high bound",
            )),
        },
        MatchType::EqualTo
        | MatchType::NotEqualTo
        | MatchType::GreaterThan
        | MatchType::LessThan
        | MatchType::IncreasedBy
        | MatchType::DecreasedBy => match value_tokens {
            [value] => Ok(ScanCriterion::Value(UserValue::Number(
                value::parse_number(value)?,
            ))),
            _ => Err(ParseError::MissingArgument(
                "this match type",
                "exactly one value",
            )),
        },
    }
}

fn parse_data_type(token: &str) -> Result<ScanDataType, ParseError> {
    Ok(match token.to_ascii_lowercase().as_str() {
        "any" => ScanDataType::AnyNumber,
        "anyint" => ScanDataType::AnyInteger,
        "anyfloat" => ScanDataType::AnyFloat,
        "i8" => ScanDataType::Integer8,
        "i16" => ScanDataType::Integer16,
        "i32" => ScanDataType::Integer32,
        "i64" => ScanDataType::Integer64,
        "f32" => ScanDataType::Float32,
        "f64" => ScanDataType::Float64,
        "bytes" => ScanDataType::ByteArray,
        "string" => ScanDataType::String,
        _ => return Err(ParseError::InvalidDataType(token.to_owned())),
    })
}

fn parse_match_type(token: &str) -> Result<MatchType, ParseError> {
    Ok(match token.to_ascii_lowercase().as_str() {
        "any" => MatchType::Any,
        "=" | "eq" => MatchType::EqualTo,
        "!=" | "ne" => MatchType::NotEqualTo,
        ">" | "gt" => MatchType::GreaterThan,
        "<" | "lt" => MatchType::LessThan,
        "range" => MatchType::Range,
        "update" => MatchType::Update,
        "unchanged" => MatchType::NotChanged,
        "changed" => MatchType::Changed,
        "increased" => MatchType::Increased,
        "decreased" => MatchType::Decreased,
        "+" | "increasedby" => MatchType::IncreasedBy,
        "-" | "decreasedby" => MatchType::DecreasedBy,
        _ => return Err(ParseError::InvalidMatchType(token.to_owned())),
    })
}

fn parse_list(rest: &str) -> Result<Command, ParseError> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    match tokens.as_slice() {
        [] => Ok(Command::List(None)),
        [start, end] => {
            let start = parse_usize(start)?;
            let end = parse_usize(end)?;
            Ok(Command::List(Some(start..end)))
        }
        _ => Err(ParseError::MissingArgument("list", "zero or two indices")),
    }
}

fn parse_dump(rest: &str) -> Result<Command, ParseError> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let [address, len] = tokens.as_slice() else {
        return Err(ParseError::MissingArgument("dump", "an address and length"));
    };
    let address = parse_usize(address)?;
    let len = parse_usize(len)?;
    Ok(Command::Dump { address, len })
}

fn parse_write(rest: &str) -> Result<Command, ParseError> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let [address_token, type_token, value_tokens @ ..] = tokens.as_slice() else {
        return Err(ParseError::MissingArgument(
            "write",
            "an address, type, and value",
        ));
    };
    let address = parse_usize(address_token)?;
    let value = parse_write_value(type_token, value_tokens)?;
    Ok(Command::Write { address, value })
}

fn parse_write_value(type_token: &str, value_tokens: &[&str]) -> Result<Value, ParseError> {
    let type_token_lower = type_token.to_ascii_lowercase();

    if type_token_lower == "bytes" {
        let pattern = value::parse_bytearray(value_tokens.iter().copied())?;
        let bytes = pattern
            .into_iter()
            .map(|entry| match entry {
                ByteOrWildcard::Fixed(byte) => Ok(byte),
                ByteOrWildcard::Wildcard => Err(ParseError::InvalidValueType(
                    "write does not support wildcard (`??`) bytes".to_owned(),
                )),
            })
            .collect::<Result<Vec<u8>, _>>()?;
        return Ok(Value::Bytes(bytes));
    }
    if type_token_lower == "string" {
        if value_tokens.is_empty() {
            return Err(ParseError::MissingArgument("write", "a string value"));
        }
        return Ok(Value::Str(value_tokens.join(" ")));
    }

    let [value_token] = value_tokens else {
        return Err(ParseError::MissingArgument("write", "exactly one value"));
    };

    if type_token_lower == "f32" || type_token_lower == "f64" {
        let number = value::parse_float(value_token)?;
        return Ok(match type_token_lower.as_str() {
            "f32" => Value::F32(number.f32.expect("parse_float always populates f32")),
            "f64" => Value::F64(number.f64.expect("parse_float always populates f64")),
            _ => unreachable!(),
        });
    }

    let number = value::parse_int(value_token)?;
    let out_of_range =
        |width: &'static str| ParseError::ValueOutOfRange(value_token.to_string(), width);
    Ok(match type_token_lower.as_str() {
        "i8" => Value::I8(number.i8.ok_or_else(|| out_of_range("i8"))?),
        "u8" => Value::U8(number.u8.ok_or_else(|| out_of_range("u8"))?),
        "i16" => Value::I16(number.i16.ok_or_else(|| out_of_range("i16"))?),
        "u16" => Value::U16(number.u16.ok_or_else(|| out_of_range("u16"))?),
        "i32" => Value::I32(number.i32.ok_or_else(|| out_of_range("i32"))?),
        "u32" => Value::U32(number.u32.ok_or_else(|| out_of_range("u32"))?),
        "i64" => Value::I64(number.i64.ok_or_else(|| out_of_range("i64"))?),
        "u64" => Value::U64(number.u64.ok_or_else(|| out_of_range("u64"))?),
        _ => return Err(ParseError::InvalidValueType(type_token.to_owned())),
    })
}

fn parse_delete(rest: &str) -> Result<Command, ParseError> {
    let selector = rest.trim();
    if selector.is_empty() {
        return Err(ParseError::MissingArgument("delete", "an index selector"));
    }
    Ok(Command::Delete(selector.to_owned()))
}

fn parse_option(rest: &str) -> Result<Command, ParseError> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let [key, value] = tokens.as_slice() else {
        return Err(ParseError::MissingArgument("option", "a key and value"));
    };
    let option = match key.to_ascii_lowercase().as_str() {
        "endianness" | "endian" => SessionOption::Endianness(parse_endianness(value)?),
        "region" | "regionfilter" => SessionOption::RegionFilter(parse_region_filter(value)?),
        _ => return Err(ParseError::InvalidOptionKey(key.to_string())),
    };
    Ok(Command::SetOption(option))
}

fn parse_endianness(token: &str) -> Result<Endianness, ParseError> {
    match token.to_ascii_lowercase().as_str() {
        "native" => Ok(Endianness::Native),
        "swapped" => Ok(Endianness::Swapped),
        _ => Err(ParseError::InvalidOptionValue(token.to_owned())),
    }
}

fn parse_region_filter(token: &str) -> Result<RegionFilter, ParseError> {
    match token.to_ascii_lowercase().as_str() {
        "writable" => Ok(RegionFilter::WritableOnly),
        "all" => Ok(RegionFilter::All),
        _ => Err(ParseError::InvalidOptionValue(token.to_owned())),
    }
}

/// Parses a decimal or `0x`-prefixed hexadecimal `usize`.
fn parse_usize(token: &str) -> Result<usize, ParseError> {
    let trimmed = token.trim();
    let (radix, digits) = match trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        Some(hex) => (16, hex),
        None => (10, trimmed),
    };
    usize::from_str_radix(digits, radix).map_err(|_| ParseError::InvalidNumber(trimmed.to_owned()))
}

/// Splits the first whitespace-delimited word off `s`, returning `(word, rest)`.
fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(index) => (&s[..index], &s[index..]),
        None => (s, ""),
    }
}
