//! `/proc/<pid>/maps` parsing into [`Region`]s — replaces `maps.c`/`maps.h`.

/// One entry from `/proc/<pid>/maps`: an address range, its permissions, and the backing path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub start: usize,
    pub end: usize,
    pub perms: Perms,
    /// The backing file, or a pseudo-path like `[heap]`/`[stack]`; `None` for anonymous mappings.
    pub path: Option<String>,
}

impl Region {
    /// Number of bytes covered by this region.
    pub fn size(&self) -> usize {
        self.end - self.start
    }

    pub fn is_writable(&self) -> bool {
        self.perms.write
    }

    /// Classifies the region the way scan-level filtering needs: heap/stack/anonymous/file-backed.
    pub fn kind(&self) -> RegionKind {
        match self.path.as_deref() {
            Some("[heap]") => RegionKind::Heap,
            Some("[stack]") => RegionKind::Stack,
            Some(_) => RegionKind::File,
            None => RegionKind::Anonymous,
        }
    }
}

/// The `rwsp` permission bits of a [`Region`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Perms {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    /// `true` for a shared (`s`) mapping, `false` for private (`p`).
    pub shared: bool,
}

/// Coarse classification of a [`Region`], replacing the classification half of C's
/// `region_scan_level_t`/`region_type_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// `[heap]`.
    Heap,
    /// `[stack]`.
    Stack,
    /// No backing file (e.g. an anonymous `mmap`), and not heap or stack.
    Anonymous,
    /// Backed by a file on disk (executable, shared library, or other mapped file).
    File,
}

/// Parses the full contents of a `/proc/<pid>/maps` file into its [`Region`]s.
///
/// Lines that don't match the expected format are skipped rather than failing the whole parse,
/// mirroring how permissive the format itself is across kernel versions.
pub fn parse_maps(text: &str) -> Vec<Region> {
    text.lines().filter_map(parse_line).collect()
}

/// Parses a single `/proc/<pid>/maps` line, e.g. `"00400000-00452000 r-xp 00000000 08:02 173521 /usr/bin/x"`.
fn parse_line(line: &str) -> Option<Region> {
    let (range, rest) = split_first_word(line);
    let (perms, rest) = split_first_word(rest);
    let (_offset, rest) = split_first_word(rest);
    let (_dev, rest) = split_first_word(rest);
    let (_inode, rest) = split_first_word(rest);

    let path = rest.trim();
    let path = (!path.is_empty()).then(|| path.to_owned());

    let (start, end) = range.split_once('-')?;
    let start = usize::from_str_radix(start, 16).ok()?;
    let end = usize::from_str_radix(end, 16).ok()?;

    let mut perm_chars = perms.chars();
    let perms = Perms {
        read: perm_chars.next()? == 'r',
        write: perm_chars.next()? == 'w',
        exec: perm_chars.next()? == 'x',
        shared: perm_chars.next()? == 's',
    };

    Some(Region {
        start,
        end,
        perms,
        path,
    })
}

/// Splits the first whitespace-delimited word off `s`, returning `(word, rest)`.
fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(idx) => (&s[..idx], &s[idx..]),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests;
