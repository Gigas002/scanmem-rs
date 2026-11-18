//! `Session` — the public facade (attach/scan/matches/read/write/options).

use rustix::process::Pid;

use crate::error::{Result, ScanmemError};
use crate::interrupt::{ScanProgress, StopFlag};
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

    /// The widest a single match against this expression can be — 8 for every numeric type
    /// (`i64`/`f64`), or the pattern's own length for `ByteArray`/`String`. Used to size the
    /// overlap between chunked region reads in [`scan_region_chunked`] so a match starting near a
    /// chunk boundary is never split across two reads.
    fn max_match_width(&self) -> usize {
        match self.data_type {
            ScanDataType::ByteArray => self.bytes_pattern().map_or(8, <[_]>::len),
            ScanDataType::String => self.string_pattern().map_or(8, str::len),
            _ => 8,
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
///
/// `old_value` is the *full* value last observed at `address` — [`SwathStore`] itself only keeps
/// one raw byte per address plus [`MatchFlags`] recording which numeric width(s)/sign(s) matched
/// there (see the `swath` module), so a multi-byte match (e.g. an `i32`) has its bytes gathered
/// and decoded back into a single value on the way out; see `reconstruct_value`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchView {
    pub address: usize,
    pub old_value: Value,
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
    progress: ScanProgress,
}

impl Session {
    /// Attaches to `pid` — replaces `sm_attach`. The target keeps running; it is only stopped
    /// during a scan (see [`Process`]).
    pub fn attach(pid: Pid) -> Result<Self> {
        Ok(Self {
            process: Some(Process::attach(pid)?),
            matches: SwathStore::new(),
            options: SessionOptions::default(),
            stop_flag: StopFlag::new(),
            progress: ScanProgress::new(),
        })
    }

    /// Detaches — replaces `sm_detach`. Further operations on this `Session` fail with
    /// [`ScanmemError::NotAttached`]. Dropping the `Session` detaches too.
    pub fn detach(&mut self) -> Result<()> {
        let process = self.process.take().ok_or(ScanmemError::NotAttached)?;
        process.detach()
    }

    /// A cloneable handle to this session's scan progress, readable from another thread while
    /// `scan`/`snapshot` runs — obtain it *before* moving the `Session` to a background thread,
    /// since the getters need `self`.
    pub fn progress_handle(&self) -> ScanProgress {
        self.progress.clone()
    }

    /// A cloneable handle to request the in-progress (or next) scan to stop early, usable from
    /// another thread — same before-the-move caveat as [`Self::progress_handle`].
    pub fn stop_handle(&self) -> StopFlag {
        self.stop_flag.clone()
    }

    /// Uses `flag` as this session's stop flag from now on, e.g. one that a `SIGINT` handler
    /// sets (see [`StopFlag::register_sigint`]).
    pub fn set_stop_handle(&mut self, flag: StopFlag) {
        self.stop_flag = flag;
    }

    /// Runs a first scan (if no matches are currently recorded) or narrows the current matches
    /// against `expr`, synchronously: [`Self::prepare_scan`], [`Self::run_scan`], then
    /// [`Self::resume_after_scan`] — the target is only paused for the duration of the scan
    /// itself, not for the rest of the attached session. A convenience wrapper for callers on a
    /// single thread (e.g. the `scanmem` CLI's REPL); a caller that wants the actual scan work
    /// off its own thread (e.g. a UI that can't block on it) should call the three steps
    /// separately instead — see their docs for why they're split out.
    pub fn scan(&mut self, expr: &ScanExpr) -> Result<ScanStats> {
        validate(expr)?;
        self.prepare_scan()?;
        let result = self.run_scan(expr);
        self.resume_after_scan();
        result
    }

    /// Records every byte of every considered region as a candidate match, discarding any
    /// current matches — the `MATCHANY` equivalent used to seed later narrowing scans.
    /// Synchronous convenience wrapper, same reasoning as [`Self::scan`].
    pub fn snapshot(&mut self) -> Result<ScanStats> {
        self.prepare_scan()?;
        let result = self.run_snapshot();
        self.resume_after_scan();
        result
    }

    /// Stops the target (see [`Process::stop`]) ahead of a scan/snapshot and resets the abort
    /// flag. Pair with [`Self::run_scan`]/[`Self::run_snapshot`] (safe to run on any thread) and
    /// [`Self::resume_after_scan`], which must run on the same thread as this method: ptrace ties
    /// the tracee to the thread that stopped it, and only that thread can resume it.
    pub fn prepare_scan(&mut self) -> Result<()> {
        self.stop_flag.reset();
        self.process()?.stop()
    }

    /// Resumes the target after [`Self::prepare_scan`] (and a [`Self::run_scan`]/
    /// [`Self::run_snapshot`] in between), on the thread that called [`Self::prepare_scan`]. Errors (e.g. the target having exited mid-scan) are swallowed:
    /// there is nothing a caller already past the scan can usefully do about a resume failure.
    pub fn resume_after_scan(&self) {
        if let Ok(process) = self.process() {
            let _ = process.resume();
        }
    }

    /// Runs a first scan (if no matches are currently recorded) or narrows the current matches
    /// against `expr`. The target must already be stopped via [`Self::prepare_scan`], but unlike
    /// that method (and [`Self::resume_after_scan`]) this one never touches ptrace itself — it
    /// only reads `/proc/<pid>/mem`, a regular file read against an fd opened back at
    /// [`Self::attach`] — so, also unlike those two, it's safe to call from any thread.
    pub fn run_scan(&mut self, expr: &ScanExpr) -> Result<ScanStats> {
        validate(expr)?;
        if self.matches.match_count() == 0 {
            self.first_scan(expr)
        } else {
            self.narrow_scan(expr)
        }
    }

    /// Runs a scan against every considered byte from scratch, discarding any current matches
    /// even if some exist — unlike [`Self::run_scan`], which narrows the current matches instead
    /// of rescanning from scratch whenever any are already recorded. Same
    /// prepare-first/any-thread-safe contract as [`Self::run_scan`].
    pub fn run_new_scan(&mut self, expr: &ScanExpr) -> Result<ScanStats> {
        validate(expr)?;
        self.first_scan(expr)
    }

    /// Records every byte of every considered region as a candidate match, discarding any
    /// current matches. Same prepare-first/any-thread-safe contract as [`Self::run_scan`].
    pub fn run_snapshot(&mut self) -> Result<ScanStats> {
        self.first_scan(&ScanExpr {
            data_type: ScanDataType::AnyNumber,
            match_type: MatchType::Any,
            criterion: ScanCriterion::None,
        })
    }

    /// Re-reads every currently recorded match's bytes fresh from the target and updates their
    /// stored values in place. Unlike [`Self::run_scan`]'s narrowing, this never filters by
    /// value or needs a [`ScanExpr`] — every match is kept regardless of what its new value turns
    /// out to be; a match is only dropped if its swath's region has become unreadable entirely
    /// (e.g. unmapped since the last scan), same as a narrowing scan's read-failure handling.
    /// Same prepare-first/any-thread-safe contract as [`Self::run_scan`].
    pub fn refresh_matches(&mut self) -> Result<ScanStats> {
        let old_swaths = std::mem::take(&mut self.matches);
        self.progress.reset(
            old_swaths
                .swaths()
                .iter()
                .map(|swath| swath.entries.len())
                .sum(),
        );

        let mut store = SwathStore::new();
        let process = self.process()?;
        'swaths: for swath in old_swaths.swaths() {
            // Chunked (see `scan_region_chunked`'s docs for why) — a swath built from a broad
            // snapshot can span an entire region, several GB in the worst case. No group
            // alignment is needed here, unlike `narrow_scan`: every byte's flags are already
            // fixed by the previous scan, so a chunk boundary can fall anywhere without losing
            // anything.
            let mut offset = 0usize;
            while offset < swath.entries.len() {
                if self.stop_flag.requested() {
                    break 'swaths;
                }
                let chunk_len = SCAN_CHUNK_LEN.min(swath.entries.len() - offset);
                let chunk_start = swath.address_of(offset);
                if let Ok(fresh) = process.read(chunk_start, chunk_len) {
                    for (index, &byte) in fresh.iter().enumerate() {
                        store.add(
                            chunk_start + index,
                            byte,
                            swath.entries[offset + index].flags,
                        );
                    }
                }
                self.progress.add(chunk_len);
                offset += chunk_len;
            }
        }

        self.matches = store;
        Ok(ScanStats {
            matches: self.matches.match_count(),
        })
    }

    /// Every currently recorded match, in ascending address order.
    pub fn matches(&self) -> impl Iterator<Item = MatchView> + '_ {
        let endianness = self.options.endianness;
        self.matches
            .matches_with_location()
            .map(move |(location, address, entry)| {
                let bytes = self
                    .matches
                    .match_bytes(location)
                    .expect("location came from this same store's matches_with_location");
                MatchView {
                    address,
                    old_value: reconstruct_value(&bytes, entry.flags, endianness),
                    flags: entry.flags,
                }
            })
    }

    /// The `n`th recorded match (0-indexed), or `None` if there are fewer than `n + 1`.
    pub fn nth_match(&self, n: usize) -> Option<MatchView> {
        let location = self.matches.nth_match(n)?;
        let (address, entry) = self.matches.entry_at(location)?;
        let bytes = self
            .matches
            .match_bytes(location)
            .expect("location came from this same store's nth_match");
        Some(MatchView {
            address,
            old_value: reconstruct_value(&bytes, entry.flags, self.options.endianness),
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

    /// Number of `/proc/<pid>/maps` regions currently mapped for the attached process.
    pub fn region_count(&self) -> Result<usize> {
        Ok(self.process()?.regions()?.len())
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
        let regions: Vec<Region> = self
            .process()?
            .regions()?
            .into_iter()
            .filter(|region| region_filter.includes(region))
            .collect();
        self.progress.reset(regions.iter().map(Region::size).sum());

        let mut store = SwathStore::new();
        let process = self.process()?;
        for region in &regions {
            let finished = scan_region_chunked(
                process,
                region,
                expr,
                endianness,
                &self.stop_flag,
                &self.progress,
                &mut store,
            );
            if !finished {
                break;
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
        self.progress.reset(
            old_swaths
                .swaths()
                .iter()
                .map(|swath| swath.entries.len())
                .sum(),
        );

        let mut store = SwathStore::new();
        let process = self.process()?;
        let probe = Probe::new(expr, endianness);
        'swaths: for swath in old_swaths.swaths() {
            // Chunked — after a broad snapshot, a single swath can be as large as an entire
            // region, several GB in the worst case. Each read reaches `window - 1` bytes past
            // its chunk (within the swath), so a match near the end still sees its full width.
            let len = swath.entries.len();
            let mut reach = 0usize;
            let mut start = 0usize;
            while start < len {
                if self.stop_flag.requested() {
                    break 'swaths;
                }
                let end = (start + SCAN_CHUNK_LEN).min(len);
                let read_end = (end + probe.window() - 1).min(len);
                match process.read(swath.address_of(start), read_end - start) {
                    Ok(fresh) => narrow_entries(
                        swath,
                        start..end,
                        &fresh,
                        &probe,
                        endianness,
                        &mut reach,
                        |address, byte, flags| store.add(address, byte, flags),
                    ),
                    // Unreadable now: its matches are dropped, and so are the filler bytes of
                    // any match before it that reached into it.
                    Err(_) => reach = 0,
                }
                self.progress.add(end - start);
                start = end;
            }
        }

        self.matches = store;
        Ok(ScanStats {
            matches: self.matches.match_count(),
        })
    }
}

/// Bytes read per chunk while scanning a region/swath, rather than in one allocation covering the
/// whole thing — a single region (or, after a broad snapshot, a single swath) can span several GB
/// as one contiguous run (e.g. a large game's heap), and without chunking neither the peak memory
/// used nor how promptly [`StopFlag::request`] takes effect is bounded by anything but that size.
/// Deliberately not configurable: it only trades off syscall count against peak memory/stop
/// latency, and 4 MiB is a reasonable point on that curve for either.
const SCAN_CHUNK_LEN: usize = 4 * 1024 * 1024;

/// Scans one region in bounded-size, overlapping chunks (see [`SCAN_CHUNK_LEN`]) instead of
/// reading/scanning it in a single allocation. Each chunk's read extends [`ScanExpr::max_match_width`]
/// `- 1` bytes past its own share of the region so a match starting right at the boundary is still
/// fully visible; [`scan_buffer_chunk`]'s `keep_before` then excludes that overlap tail from the
/// results (it belongs to, and gets rescanned as part of, the next chunk) so nothing is reported
/// twice. Returns `false` if `stop_flag` was set partway through (matches found so far are still
/// kept), `true` if the whole region was scanned.
fn scan_region_chunked(
    process: &Process,
    region: &Region,
    expr: &ScanExpr,
    endianness: Endianness,
    stop_flag: &StopFlag,
    progress: &ScanProgress,
    store: &mut SwathStore,
) -> bool {
    let overlap = expr.max_match_width().saturating_sub(1);
    let probe = Probe::new(expr, endianness);
    let mut offset = 0usize;
    while offset < region.size() {
        if stop_flag.requested() {
            return false;
        }
        let chunk_len = SCAN_CHUNK_LEN.min(region.size() - offset);
        let read_len = (chunk_len + overlap).min(region.size() - offset);
        let chunk_start = region.start + offset;

        if let Ok(bytes) = process.read(chunk_start, read_len) {
            for (address, byte, flags) in
                scan_buffer_chunk(chunk_start, &bytes, &probe, endianness, chunk_len)
            {
                store.add(address, byte, flags);
            }
        }
        progress.add(chunk_len);
        offset += chunk_len;
    }
    true
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

/// `expr` prepared once per scan, so testing a position doesn't redo per-scan work.
enum Probe<'a> {
    Bytes(&'a [ByteOrWildcard]),
    Str(&'a str),
    Number(scanroutines::Matcher),
    /// A byte/string scan without its pattern; `validate` rejects these first.
    Nothing,
}

impl<'a> Probe<'a> {
    fn new(expr: &'a ScanExpr, endianness: Endianness) -> Self {
        match expr.data_type {
            ScanDataType::ByteArray => expr.bytes_pattern().map_or(Probe::Nothing, Probe::Bytes),
            ScanDataType::String => expr.string_pattern().map_or(Probe::Nothing, Probe::Str),
            _ => Probe::Number(scanroutines::Matcher::new(
                expr.data_type,
                expr.match_type,
                endianness,
                expr.number(),
                expr.range(),
            )),
        }
    }

    /// How many bytes from a position the expression can look at: 8 for numbers (the widest
    /// width), the pattern's length otherwise.
    fn window(&self) -> usize {
        match self {
            Probe::Bytes(pattern) => pattern.len().max(1),
            Probe::Str(pattern) => pattern.len().max(1),
            Probe::Number(_) | Probe::Nothing => 8,
        }
    }

    /// Tests the expression against `memory` at one position, optionally against `old` raw
    /// bytes recorded at that same position by a previous scan — returns the matched width in
    /// bytes and which flags to record, or `None` if nothing matched here.
    #[inline]
    fn test(
        &self,
        memory: &[u8],
        old: Option<&[u8]>,
        endianness: Endianness,
    ) -> Option<(usize, MatchFlags)> {
        match self {
            Probe::Bytes(pattern) => scanroutines::scan_bytearray(memory, pattern)
                .map(|width| (width, MatchFlags::all())),
            Probe::Str(pattern) => {
                scanroutines::scan_string(memory, pattern).map(|width| (width, MatchFlags::all()))
            }
            Probe::Number(matcher) => {
                let old_number = old.map(|bytes| scanroutines::decode_number(bytes, endianness));
                matcher.test(memory, old_number.as_ref())
            }
            Probe::Nothing => None,
        }
    }
}

/// Scans one contiguous region (already read into `bytes`, starting at `base`) for a first scan,
/// returning `(address, byte, flags)` triples in ascending address order, ready to feed into
/// [`SwathStore::add`]. Only includes entries belonging to a match/filler group whose own match
/// started before `keep_before` (a byte offset into `bytes`) — pass `usize::MAX` for an unchunked
/// scan of the whole buffer (nothing to trim).
///
/// A byte is included either because it starts its own match (per [`probe`]), or because it
/// falls within the width of an earlier match in the same region — the latter is filler kept
/// only so a later narrowing scan can reconstruct that match's multi-byte old value. `keep_before`
/// exists for [`scan_region_chunked`]: `bytes` there is one chunk's own share of a region plus a
/// short overlap tail borrowed from the next chunk (wide enough that a match starting right at
/// the boundary is still fully visible here), and `keep_before` (the chunk's own length, excluding
/// that tail) excludes any group that only starts in the borrowed tail — it belongs to, and gets
/// rescanned as part of, the next chunk's own (overlap-extended) read, so excluding it here is
/// what keeps it from being reported twice.
fn scan_buffer_chunk(
    base: usize,
    bytes: &[u8],
    probe: &Probe<'_>,
    endianness: Endianness,
    keep_before: usize,
) -> Vec<(usize, u8, MatchFlags)> {
    let mut results = Vec::new();
    let mut reach = 0usize;
    let mut group_start = 0usize;
    for offset in 0..bytes.len() {
        let own_match = probe.test(&bytes[offset..], None, endianness);
        if own_match.is_none() && offset >= reach {
            continue;
        }
        if own_match.is_some() {
            group_start = offset;
        }
        if group_start >= keep_before {
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

/// Re-tests the previously matched positions among `swath.entries[range]` against `fresh`
/// (target memory from `range.start` on, reaching past `range.end` where the swath does) and
/// emits `(address, byte, flags)` for a narrowed [`SwathStore`], in address order. Does not
/// discover new match starts, and never adds a width: a numeric match keeps only the widths that
/// matched before and still match.
///
/// Each match is tested on a full-width window of fresh bytes, with its old value read from the
/// following recorded bytes whatever matches those belong to: overlapping matches share bytes.
/// Bytes within the width of a surviving match are kept as filler, the same rule a first scan
/// uses; `reach` (an entry index) carries that across the chunks of one swath.
fn narrow_entries(
    swath: &Swath,
    range: std::ops::Range<usize>,
    fresh: &[u8],
    probe: &Probe<'_>,
    endianness: Endianness,
    reach: &mut usize,
    mut emit: impl FnMut(usize, u8, MatchFlags),
) {
    let entries = &swath.entries;
    for index in range.clone() {
        let Some(fresh_window) = fresh.get(index - range.start..) else {
            break;
        };
        let Some(&fresh_byte) = fresh_window.first() else {
            break;
        };
        let old_flags = entries[index].flags;
        let own = if old_flags.is_empty() {
            None
        } else {
            let mut old = [0u8; 8];
            let old_len = (entries.len() - index).min(old.len());
            for (slot, entry) in old.iter_mut().zip(&entries[index..index + old_len]) {
                *slot = entry.old_value;
            }
            probe
                .test(fresh_window, Some(&old[..old_len]), endianness)
                .and_then(|(width, flags)| {
                    if flags == MatchFlags::all() {
                        // A byte-array or string match: no widths to narrow.
                        return Some((width, flags));
                    }
                    let kept = flags & old_flags;
                    (!kept.is_empty()).then(|| (kept.max_width(), kept))
                })
        };
        if own.is_none() && index >= *reach {
            continue;
        }
        if let Some((width, _)) = own {
            *reach = (*reach).max(index + width);
        }
        emit(
            swath.address_of(index),
            fresh_byte,
            own.map_or(MatchFlags::empty(), |(_, flags)| flags),
        );
    }
}

/// Reconstructs the full-width [`Value`] a match's raw `bytes` (as returned by
/// [`SwathStore::match_bytes`]) represent, honoring `endianness` and the [`MatchFlags`] recorded
/// for the match's first byte.
///
/// A byte-array/string pattern match is the only case [`probe`] sets every [`MatchFlags`] bit at
/// once — recognized here by `bytes.len() > 1` (a plain numeric scan never sets every bit *and*
/// spans more than one byte, since [`scan`](scanroutines::scan) only sets the bits for the widths
/// it actually tried) and rendered as raw [`Value::Bytes`] instead of a spurious numeric guess.
/// Otherwise picks the highest-priority matching numeric interpretation that fits `bytes` — float
/// over integer, wider over narrower, signed over unsigned at the same width. Ties (a value that
/// matched as both `u32` and `i32`, say) have no single "correct" answer, so this is a
/// deterministic, documented pick rather than a recovery of the user's exact original scan-time
/// intent, which the recorded flags alone can't distinguish.
fn reconstruct_value(bytes: &[u8], flags: MatchFlags, endianness: Endianness) -> Value {
    if bytes.len() > 1 && flags == MatchFlags::all() {
        return Value::Bytes(bytes.to_vec());
    }

    let numbers = scanroutines::decode_number(bytes, endianness);
    let candidates = [
        (MatchFlags::F64, numbers.f64.map(Value::F64)),
        (MatchFlags::F32, numbers.f32.map(Value::F32)),
        (MatchFlags::S64, numbers.i64.map(Value::I64)),
        (MatchFlags::U64, numbers.u64.map(Value::U64)),
        (MatchFlags::S32, numbers.i32.map(Value::I32)),
        (MatchFlags::U32, numbers.u32.map(Value::U32)),
        (MatchFlags::S16, numbers.i16.map(Value::I16)),
        (MatchFlags::U16, numbers.u16.map(Value::U16)),
        (MatchFlags::S8, numbers.i8.map(Value::I8)),
        (MatchFlags::U8, numbers.u8.map(Value::U8)),
    ];
    for (bit, value) in candidates {
        if flags.contains(bit)
            && let Some(value) = value
        {
            return value;
        }
    }

    // Only reachable with hand-built `SwathEntry`s (e.g. in tests) whose flags claim a width
    // wider than the bytes actually recorded — every real scan's flags always fit its own width.
    numbers
        .u64
        .map(Value::U64)
        .or(numbers.u32.map(Value::U32))
        .or(numbers.u16.map(Value::U16))
        .or(numbers.u8.map(Value::U8))
        .unwrap_or_else(|| Value::Bytes(bytes.to_vec()))
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
