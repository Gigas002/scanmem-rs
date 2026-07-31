//! `Session` — the public facade (attach/scan/matches/read/write/options).

use rustix::process::Pid;

use crate::error::{Result, ScanmemError};
use crate::interrupt::StopFlag;
use crate::maps::Region;
use crate::process::Process;
use crate::scanroutines::{self, Endianness, MatchType, ScanDataType};
use crate::swath::{Swath, SwathStore};
use crate::value::{ByteOrWildcard, MatchFlags, NumberValue, UserValue, Value};

/// A comparison value for a [`ScanExpr`].
#[derive(Debug, Clone, PartialEq)]
pub enum ScanCriterion {
    /// No comparison value needed, e.g. [`MatchType::Any`]/[`MatchType::Update`].
    None,
    Value(UserValue),
    /// Inclusive `[low, high]` bounds, for [`MatchType::Range`].
    Range(NumberValue, NumberValue),
}

/// One scan request: what to look for, and how — replaces the parsed form of a `scanmem`
/// text command's data-type/match-type/value trio.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanExpr {
    pub data_type: ScanDataType,
    pub match_type: MatchType,
    pub criterion: ScanCriterion,
}

impl ScanExpr {
    fn number(&self) -> Option<&NumberValue> {
        match &self.criterion {
            ScanCriterion::Value(UserValue::Number(number)) => Some(number),
            _ => None,
        }
    }

    fn range(&self) -> Option<(&NumberValue, &NumberValue)> {
        match &self.criterion {
            ScanCriterion::Range(low, high) => Some((low, high)),
            _ => None,
        }
    }

    fn bytes_pattern(&self) -> Option<&[ByteOrWildcard]> {
        match &self.criterion {
            ScanCriterion::Value(UserValue::Bytes(pattern)) => Some(pattern),
            _ => None,
        }
    }

    fn string_pattern(&self) -> Option<&str> {
        match &self.criterion {
            ScanCriterion::Value(UserValue::Str(pattern)) => Some(pattern),
            _ => None,
        }
    }
}

/// Which `/proc/<pid>/maps` regions a scan considers — replaces `region_scan_level_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionFilter {
    /// Only regions the target could itself write to, e.g. heap/stack/writable data segments.
    WritableOnly,
    /// Every readable region, including read-only/executable code and mapped files.
    All,
}

impl RegionFilter {
    fn includes(self, region: &Region) -> bool {
        region.perms.read
            && match self {
                RegionFilter::All => true,
                RegionFilter::WritableOnly => region.is_writable(),
            }
    }
}

/// Resolved `Session` behavior — replaces the relevant fields of `globals_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionOptions {
    pub endianness: Endianness,
    pub region_filter: RegionFilter,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            endianness: Endianness::Native,
            region_filter: RegionFilter::WritableOnly,
        }
    }
}

/// A single option to change on a [`Session`] via [`Session::set_option`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOption {
    Endianness(Endianness),
    RegionFilter(RegionFilter),
}

/// Outcome of a [`Session::scan`]/[`Session::snapshot`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScanStats {
    pub matches: usize,
}

/// A recorded match, as returned by [`Session::matches`]/[`Session::nth_match`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchView {
    pub address: usize,
    pub old_value: u8,
    pub flags: MatchFlags,
}

/// One attached target process plus its current match set and options — the structured
/// replacement for `globals_t` + `sm_backend_exec_cmd`.
#[derive(Debug)]
pub struct Session {
    process: Option<Process>,
    matches: SwathStore,
    options: SessionOptions,
    stop_flag: StopFlag,
}

impl Session {
    /// Attaches to `pid` — replaces `sm_attach`.
    pub fn attach(pid: Pid) -> Result<Self> {
        Ok(Self {
            process: Some(Process::attach(pid)?),
            matches: SwathStore::new(),
            options: SessionOptions::default(),
            stop_flag: StopFlag::new(),
        })
    }

    /// Detaches, resuming the target's execution — replaces `sm_detach`. Further operations on
    /// this `Session` fail with [`ScanmemError::NotAttached`].
    pub fn detach(&mut self) -> Result<()> {
        let process = self.process.take().ok_or(ScanmemError::NotAttached)?;
        process.detach()
    }

    /// Runs a first scan (if no matches are currently recorded) or narrows the current matches
    /// against `expr`.
    pub fn scan(&mut self, expr: &ScanExpr) -> Result<ScanStats> {
        validate(expr)?;
        self.stop_flag.reset();
        if self.matches.match_count() == 0 {
            self.first_scan(expr)
        } else {
            self.narrow_scan(expr)
        }
    }

    /// Records every byte of every considered region as a candidate match, discarding any
    /// current matches — the `MATCHANY` equivalent used to seed later narrowing scans.
    pub fn snapshot(&mut self) -> Result<ScanStats> {
        self.stop_flag.reset();
        self.first_scan(&ScanExpr {
            data_type: ScanDataType::AnyNumber,
            match_type: MatchType::Any,
            criterion: ScanCriterion::None,
        })
    }

    /// Every currently recorded match, in ascending address order.
    pub fn matches(&self) -> impl Iterator<Item = MatchView> + '_ {
        self.matches.matches().map(|(address, entry)| MatchView {
            address,
            old_value: entry.old_value,
            flags: entry.flags,
        })
    }

    /// The `n`th recorded match (0-indexed), or `None` if there are fewer than `n + 1`.
    pub fn nth_match(&self, n: usize) -> Option<MatchView> {
        let location = self.matches.nth_match(n)?;
        let (address, entry) = self.matches.entry_at(location)?;
        Some(MatchView {
            address,
            old_value: entry.old_value,
            flags: entry.flags,
        })
    }

    /// Removes every recorded match in `range`, returning how many were removed.
    pub fn delete_in_range(&mut self, range: std::ops::Range<usize>) -> usize {
        let before = self.matches.match_count();
        self.matches.delete_in_range(range.start, range.end);
        before - self.matches.match_count()
    }

    /// Reads `len` bytes starting at `address` in the target's address space.
    pub fn read(&mut self, address: usize, len: usize) -> Result<Vec<u8>> {
        self.process()?.read(address, len)
    }

    /// Writes `value` starting at `address` in the target's address space, honoring the
    /// session's configured [`Endianness`] for numeric values.
    pub fn write(&mut self, address: usize, value: &Value) -> Result<()> {
        let bytes = value_to_bytes(value, self.options.endianness);
        self.process()?.write(address, &bytes)
    }

    pub fn set_option(&mut self, option: SessionOption) {
        match option {
            SessionOption::Endianness(endianness) => self.options.endianness = endianness,
            SessionOption::RegionFilter(region_filter) => {
                self.options.region_filter = region_filter;
            }
        }
    }

    /// Requests that the in-progress (or next) scan abort cooperatively as soon as possible.
    pub fn request_stop(&self) {
        self.stop_flag.request();
    }

    fn process(&self) -> Result<&Process> {
        self.process.as_ref().ok_or(ScanmemError::NotAttached)
    }

    /// Scans every considered region from scratch, replacing any current matches.
    fn first_scan(&mut self, expr: &ScanExpr) -> Result<ScanStats> {
        let endianness = self.options.endianness;
        let region_filter = self.options.region_filter;
        let regions = self.process()?.regions()?;

        let mut store = SwathStore::new();
        for region in regions
            .iter()
            .filter(|region| region_filter.includes(region))
        {
            if self.stop_flag.requested() {
                break;
            }
            let Ok(bytes) = self.process()?.read(region.start, region.size()) else {
                continue;
            };
            for (address, byte, flags) in scan_buffer(region.start, &bytes, expr, endianness) {
                store.add(address, byte, flags);
            }
        }

        self.matches = store;
        Ok(ScanStats {
            matches: self.matches.match_count(),
        })
    }

    /// Re-tests every currently recorded match against fresh memory, replacing the match set
    /// with only the positions that still match.
    fn narrow_scan(&mut self, expr: &ScanExpr) -> Result<ScanStats> {
        let endianness = self.options.endianness;
        let old_swaths = std::mem::take(&mut self.matches);

        let mut store = SwathStore::new();
        for swath in old_swaths.swaths() {
            if self.stop_flag.requested() {
                break;
            }
            let Ok(fresh) = self
                .process()?
                .read(swath.first_byte_in_child, swath.entries.len())
            else {
                continue;
            };
            for (address, byte, flags) in narrow_swath(swath, &fresh, expr, endianness) {
                store.add(address, byte, flags);
            }
        }

        self.matches = store;
        Ok(ScanStats {
            matches: self.matches.match_count(),
        })
    }
}

/// Rejects an `expr` whose `data_type`/`match_type` needs a criterion it doesn't have.
fn validate(expr: &ScanExpr) -> Result<()> {
    if expr.data_type == ScanDataType::ByteArray && expr.bytes_pattern().is_none() {
        return Err(ScanmemError::InvalidExpr(
            "BYTEARRAY scan requires a byte pattern".to_owned(),
        ));
    }
    if expr.data_type == ScanDataType::String && expr.string_pattern().is_none() {
        return Err(ScanmemError::InvalidExpr(
            "STRING scan requires a string pattern".to_owned(),
        ));
    }
    if expr.match_type == MatchType::Range && expr.range().is_none() {
        return Err(ScanmemError::InvalidExpr(
            "range match requires low/high bounds".to_owned(),
        ));
    }
    Ok(())
}

/// Tests `expr` against `memory` at one position, optionally against `old` raw bytes recorded
/// at that same position by a previous scan — returns the matched width in bytes and which
/// flags to record, or `None` if nothing matched here.
fn probe(
    memory: &[u8],
    expr: &ScanExpr,
    old: Option<&[u8]>,
    endianness: Endianness,
) -> Option<(usize, MatchFlags)> {
    match expr.data_type {
        ScanDataType::ByteArray => {
            let pattern = expr.bytes_pattern()?;
            scanroutines::scan_bytearray(memory, pattern).map(|width| (width, MatchFlags::all()))
        }
        ScanDataType::String => {
            let pattern = expr.string_pattern()?;
            scanroutines::scan_string(memory, pattern).map(|width| (width, MatchFlags::all()))
        }
        _ => {
            let old_number = old.map(|bytes| scanroutines::decode_number(bytes, endianness));
            scanroutines::scan(
                memory,
                expr.data_type,
                expr.match_type,
                endianness,
                old_number.as_ref(),
                expr.number(),
                expr.range(),
            )
        }
    }
}

/// Scans one contiguous region (already read into `bytes`, starting at `base`) for a first
/// scan, returning `(address, byte, flags)` triples in ascending address order, ready to feed
/// into [`SwathStore::add`].
///
/// A byte is included either because it starts its own match (per [`probe`]), or because it
/// falls within the width of an earlier match in the same region — the latter is filler kept
/// only so a later narrowing scan can reconstruct that match's multi-byte old value.
fn scan_buffer(
    base: usize,
    bytes: &[u8],
    expr: &ScanExpr,
    endianness: Endianness,
) -> Vec<(usize, u8, MatchFlags)> {
    let own: Vec<Option<(usize, MatchFlags)>> = (0..bytes.len())
        .map(|offset| probe(&bytes[offset..], expr, None, endianness))
        .collect();

    let mut results = Vec::new();
    let mut reach = 0usize;
    for (offset, &own_match) in own.iter().enumerate() {
        if own_match.is_none() && offset >= reach {
            continue;
        }
        let flags = own_match.map_or(MatchFlags::empty(), |(_, flags)| flags);
        if let Some((width, _)) = own_match {
            reach = reach.max(offset + width);
        }
        results.push((base + offset, bytes[offset], flags));
    }
    results
}

/// Re-tests every previously matched position in `old_swath` against `fresh` (the same address
/// range and length as `old_swath`, freshly read), returning `(address, byte, flags)` triples
/// for a narrowed [`SwathStore`]. Does not discover new match starts — only positions that
/// already matched are re-tested, matching narrowing-scan semantics.
fn narrow_swath(
    old_swath: &Swath,
    fresh: &[u8],
    expr: &ScanExpr,
    endianness: Endianness,
) -> Vec<(usize, u8, MatchFlags)> {
    let mut results = Vec::new();
    let mut index = 0usize;
    while index < old_swath.entries.len() {
        if old_swath.entries[index].flags.is_empty() {
            index += 1;
            continue;
        }

        let start = index;
        let mut end = index + 1;
        while end < old_swath.entries.len() && old_swath.entries[end].flags.is_empty() {
            end += 1;
        }
        let address = old_swath.address_of(start);
        let old_bytes: Vec<u8> = old_swath.entries[start..end]
            .iter()
            .map(|entry| entry.old_value)
            .collect();

        if let Some(fresh_window) = fresh.get(start..end)
            && let Some((matched_width, flags)) =
                probe(fresh_window, expr, Some(&old_bytes), endianness)
        {
            for (i, &byte) in fresh_window[..matched_width].iter().enumerate() {
                let entry_flags = if i == 0 { flags } else { MatchFlags::empty() };
                results.push((address + i, byte, entry_flags));
            }
        }

        index = end;
    }
    results
}

/// Converts a [`Value`] to its raw byte representation, honoring `endianness` for numeric
/// widths; [`Value::Bytes`]/[`Value::Str`] have no numeric byte order and pass through as-is.
fn value_to_bytes(value: &Value, endianness: Endianness) -> Vec<u8> {
    fn ordered<const N: usize>(bytes: [u8; N], endianness: Endianness) -> Vec<u8> {
        match endianness {
            Endianness::Native => bytes.to_vec(),
            Endianness::Swapped => bytes.iter().rev().copied().collect(),
        }
    }

    match value {
        Value::U8(v) => ordered(v.to_ne_bytes(), endianness),
        Value::I8(v) => ordered(v.to_ne_bytes(), endianness),
        Value::U16(v) => ordered(v.to_ne_bytes(), endianness),
        Value::I16(v) => ordered(v.to_ne_bytes(), endianness),
        Value::U32(v) => ordered(v.to_ne_bytes(), endianness),
        Value::I32(v) => ordered(v.to_ne_bytes(), endianness),
        Value::U64(v) => ordered(v.to_ne_bytes(), endianness),
        Value::I64(v) => ordered(v.to_ne_bytes(), endianness),
        Value::F32(v) => ordered(v.to_ne_bytes(), endianness),
        Value::F64(v) => ordered(v.to_ne_bytes(), endianness),
        Value::Bytes(bytes) => bytes.clone(),
        Value::Str(s) => s.clone().into_bytes(),
    }
}

#[cfg(test)]
mod tests;
