//! Cheat-list persistence: save/load [`CheatEntry`] lists to/from a TOML file via `serde`. Not
//! the legacy scanmem cheat-list format — a from-scratch, forward-compatible schema.

use std::fs;
use std::path::Path;

use libscanmem::value::Value;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::app::state::CheatEntry;

/// Failure saving or loading a cheat list.
#[derive(Debug, Error)]
pub enum CheatListError {
    #[error("failed to read/write cheat list file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse cheat list file: {0}")]
    Deserialize(#[from] toml::de::Error),
    #[error("failed to serialize cheat list: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// On-disk representation of a saved cheat list.
#[derive(Debug, Serialize, Deserialize)]
struct CheatListFile {
    entries: Vec<CheatEntryFile>,
}

/// On-disk representation of one [`CheatEntry`].
#[derive(Debug, Serialize, Deserialize)]
struct CheatEntryFile {
    address: usize,
    description: String,
    value: ValueFile,
    frozen: bool,
}

/// On-disk representation of a [`Value`] — a `serde`-derivable mirror, since `Value` itself
/// lives in `libscanmem` and has no `serde` dependency.
#[derive(Debug, Serialize, Deserialize)]
enum ValueFile {
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

impl From<&Value> for ValueFile {
    fn from(value: &Value) -> Self {
        match value {
            Value::U8(v) => ValueFile::U8(*v),
            Value::I8(v) => ValueFile::I8(*v),
            Value::U16(v) => ValueFile::U16(*v),
            Value::I16(v) => ValueFile::I16(*v),
            Value::U32(v) => ValueFile::U32(*v),
            Value::I32(v) => ValueFile::I32(*v),
            Value::U64(v) => ValueFile::U64(*v),
            Value::I64(v) => ValueFile::I64(*v),
            Value::F32(v) => ValueFile::F32(*v),
            Value::F64(v) => ValueFile::F64(*v),
            Value::Bytes(v) => ValueFile::Bytes(v.clone()),
            Value::Str(v) => ValueFile::Str(v.clone()),
        }
    }
}

impl From<ValueFile> for Value {
    fn from(value: ValueFile) -> Self {
        match value {
            ValueFile::U8(v) => Value::U8(v),
            ValueFile::I8(v) => Value::I8(v),
            ValueFile::U16(v) => Value::U16(v),
            ValueFile::I16(v) => Value::I16(v),
            ValueFile::U32(v) => Value::U32(v),
            ValueFile::I32(v) => Value::I32(v),
            ValueFile::U64(v) => Value::U64(v),
            ValueFile::I64(v) => Value::I64(v),
            ValueFile::F32(v) => Value::F32(v),
            ValueFile::F64(v) => Value::F64(v),
            ValueFile::Bytes(v) => Value::Bytes(v),
            ValueFile::Str(v) => Value::Str(v),
        }
    }
}

impl From<&CheatEntry> for CheatEntryFile {
    fn from(entry: &CheatEntry) -> Self {
        Self {
            address: entry.address,
            description: entry.description.clone(),
            value: ValueFile::from(&entry.value),
            frozen: entry.frozen,
        }
    }
}

impl From<CheatEntryFile> for CheatEntry {
    fn from(entry: CheatEntryFile) -> Self {
        Self {
            address: entry.address,
            description: entry.description,
            value: entry.value.into(),
            frozen: entry.frozen,
        }
    }
}

/// Serializes `cheats` as TOML and writes it to `path`, overwriting any existing file.
pub fn save(path: &Path, cheats: &[CheatEntry]) -> Result<(), CheatListError> {
    let file = CheatListFile {
        entries: cheats.iter().map(CheatEntryFile::from).collect(),
    };
    let text = toml::to_string_pretty(&file)?;
    fs::write(path, text)?;
    Ok(())
}

/// Reads and parses the cheat list stored at `path`.
pub fn load(path: &Path) -> Result<Vec<CheatEntry>, CheatListError> {
    let text = fs::read_to_string(path)?;
    let file: CheatListFile = toml::from_str(&text)?;
    Ok(file.entries.into_iter().map(CheatEntry::from).collect())
}
