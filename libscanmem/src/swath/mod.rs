//! `Vec<Swath>`-backed match storage — safe replacement for `targetmem.c`/`targetmem.h`.

use crate::value::MatchFlags;

/// One recorded byte within a [`Swath`]: its last observed value and which numeric width(s)/
/// signs it currently satisfies. Empty [`MatchFlags`] marks a byte kept only to preserve
/// contiguity with a neighboring multi-byte match (e.g. the trailing bytes of a matched `u32`),
/// not a match in its own right — mirrors `flags_empty` entries written by the C scan loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwathEntry {
    pub old_value: u8,
    pub flags: MatchFlags,
}

/// A contiguous run of recorded addresses in the target process — the safe replacement for
/// `matches_and_old_values_swath`. Unlike the C struct, this owns a plain `Vec` instead of a
/// packed flexible array member, so there is no manual `realloc`/offset arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swath {
    pub first_byte_in_child: usize,
    pub entries: Vec<SwathEntry>,
}

impl Swath {
    /// The address of `entries[index]` in the target process.
    pub fn address_of(&self, index: usize) -> usize {
        self.first_byte_in_child + index
    }
}

/// Where a specific entry lives: which swath, and its offset within that swath's `entries` —
/// replaces `match_location`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchLocation {
    pub swath_index: usize,
    pub entry_index: usize,
}

/// `Vec<Swath>`-backed match storage — the safe replacement for `matches_and_old_values_array`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SwathStore {
    swaths: Vec<Swath>,
}

impl SwathStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The swaths currently stored, in ascending address order.
    pub fn swaths(&self) -> &[Swath] {
        &self.swaths
    }

    /// Number of actual matches (entries with non-empty [`MatchFlags`]) currently stored; does
    /// not count contiguity-filler bytes.
    pub fn match_count(&self) -> usize {
        self.matches().count()
    }

    /// Records a byte at `address`, appending it to the last swath when `address` immediately
    /// follows it, or starting a new swath otherwise — replaces `add_element`.
    pub fn add(&mut self, address: usize, old_value: u8, flags: MatchFlags) {
        Self::push_entry(&mut self.swaths, address, SwathEntry { old_value, flags });
    }

    /// Iterates every actual match (entries with non-empty [`MatchFlags`]) as `(address, entry)`,
    /// skipping contiguity-filler bytes — replaces the `match_info != flags_empty` filter in
    /// C's `nth_match`.
    pub fn matches(&self) -> impl Iterator<Item = (usize, &SwathEntry)> + '_ {
        self.raw_iter().filter(|(_, entry)| !entry.flags.is_empty())
    }

    /// The location of the `n`th actual match (0-indexed), or `None` if there are fewer than
    /// `n + 1` — replaces `nth_match`.
    pub fn nth_match(&self, n: usize) -> Option<MatchLocation> {
        let mut seen = 0;
        for (swath_index, swath) in self.swaths.iter().enumerate() {
            for (entry_index, entry) in swath.entries.iter().enumerate() {
                if entry.flags.is_empty() {
                    continue;
                }
                if seen == n {
                    return Some(MatchLocation {
                        swath_index,
                        entry_index,
                    });
                }
                seen += 1;
            }
        }
        None
    }

    /// The `(address, entry)` a [`MatchLocation`] refers to, or `None` if it is out of bounds.
    pub fn entry_at(&self, location: MatchLocation) -> Option<(usize, &SwathEntry)> {
        let swath = self.swaths.get(location.swath_index)?;
        let entry = swath.entries.get(location.entry_index)?;
        Some((swath.address_of(location.entry_index), entry))
    }

    /// Removes every recorded byte whose address falls in `[start, end)`, compacting the
    /// remaining bytes into (possibly fewer, possibly merged) swaths — replaces
    /// `delete_in_address_range`.
    pub fn delete_in_range(&mut self, start: usize, end: usize) {
        let mut compacted: Vec<Swath> = Vec::with_capacity(self.swaths.len());
        for swath in self.swaths.drain(..) {
            let first = swath.first_byte_in_child;
            for (index, entry) in swath.entries.into_iter().enumerate() {
                let address = first + index;
                if address >= start && address < end {
                    continue;
                }
                Self::push_entry(&mut compacted, address, entry);
            }
        }
        self.swaths = compacted;
    }

    /// Printable text for up to `length` recorded bytes starting at `location`, stopping early
    /// at the end of that entry's swath; non-printable bytes render as `.` — replaces
    /// `data_to_printable_string`.
    pub fn printable_string(&self, location: MatchLocation, length: usize) -> Option<String> {
        self.bytes_from(location, length).map(|bytes| {
            bytes
                .iter()
                .map(|&b| {
                    if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect()
        })
    }

    /// Hex text (`"aa bb cc"`) for up to `length` recorded bytes starting at `location`, stopping
    /// early at the end of that entry's swath — replaces `data_to_bytearray_text`.
    pub fn bytearray_text(&self, location: MatchLocation, length: usize) -> Option<String> {
        self.bytes_from(location, length).map(|bytes| {
            bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
    }

    /// Iterates every recorded `(address, entry)` pair, including contiguity-filler bytes.
    fn raw_iter(&self) -> impl Iterator<Item = (usize, &SwathEntry)> + '_ {
        self.swaths.iter().flat_map(|swath| {
            swath
                .entries
                .iter()
                .enumerate()
                .map(move |(i, entry)| (swath.address_of(i), entry))
        })
    }

    fn bytes_from(&self, location: MatchLocation, length: usize) -> Option<Vec<u8>> {
        let swath = self.swaths.get(location.swath_index)?;
        let start = location.entry_index;
        if start > swath.entries.len() {
            return None;
        }
        let end = (start + length).min(swath.entries.len());
        Some(
            swath.entries[start..end]
                .iter()
                .map(|e| e.old_value)
                .collect(),
        )
    }

    /// Appends `entry` at `address` to `swaths`, merging into the last swath when contiguous.
    fn push_entry(swaths: &mut Vec<Swath>, address: usize, entry: SwathEntry) {
        if let Some(last) = swaths.last_mut() {
            let next_address = last.first_byte_in_child + last.entries.len();
            if next_address == address {
                last.entries.push(entry);
                return;
            }
        }
        swaths.push(Swath {
            first_byte_in_child: address,
            entries: vec![entry],
        });
    }
}

#[cfg(test)]
mod tests;
