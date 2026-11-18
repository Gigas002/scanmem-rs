# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Changed

- First scans are 3 to 7 times faster and use far less memory: on a 256 MiB target, an
  `i32` equal-to scan went from 3.1 s to 0.96 s, an any-number one from 7.9 s to 1.1 s, and a
  snapshot from 9.9 s to 3.0 s; peak memory for a scan with few matches went from 103 MiB to
  14 MiB. Results are unchanged.
- scanmem: `delete` walked every match twice per index, so deleting 1000 of 1.5 million
  matches took 48 s and `delete !0` didn't finish; both now take a single pass (milliseconds).
  Counting matches after each command no longer decodes every value.
- gameconqueror: with millions of matches the interface froze, because every frame (ten a
  second) decoded, filtered and sorted all of them and formatted a table row for each. The
  filtered, sorted list is now rebuilt only when the matches, filter or sort change, and only
  visible rows are drawn: with 3.7 million matches, idle CPU went from 94 % to 0.4 %, an arrow
  key from 1.5 s to 1 ms, and memory from 4.3 GiB to 560 MiB.

### Fixed

- gameconqueror: the default log moved from the shared `/tmp/gameconqueror.log` to
  `$XDG_STATE_HOME/gameconqueror/gameconqueror.log` (usually
  `~/.local/state/gameconqueror/`). Run as root (as the docs suggest), it opened a file in a
  world-writable directory that another user could have created first. The log is now created
  `0600` in a `0700` directory, and a symlink in its place is refused.
- Attach and ptrace errors say what happened: a refused attach explains `ptrace_scope` (it
  was "Permission denied (os error 13)"), a missing process is "no such process", and a target
  that exited is "the target process has exited" (it was "ptrace failed (errno 1)" or "No such
  process (os error 3)"). An `--exec` script stops when `--pid` can't be attached, instead of
  failing each command. `ScanmemError` has a new `NoSuchProcess` variant, and `Ptrace` shows the
  errno's text.
- Narrowing lost most matches whose bytes overlapped the next match: each match was
  re-tested only on its bytes up to the next match's start, so `scan i32 = 0` repeated on
  unchanged memory went from 3.77 million matches to 121 thousand. Matches are now tested on
  their full width (old bytes included), narrowing never adds a width that didn't match
  before, and an overlapped match shows its full value (an f64 was listed as an integer).
- An attached process froze on the first signal it received (SIGCHLD, SIGALRM, a terminal
  resize, Ctrl-C), never got that signal, and stayed stopped after scanmem or gameconqueror
  quit. Attaching no longer makes the target a ptrace tracee: it is only stopped, with
  `PTRACE_SEIZE`/`PTRACE_INTERRUPT` and no SIGSTOP, for the length of a scan, and a signal
  that arrives meanwhile is delivered when it resumes. `reset`, attaching elsewhere and
  quitting mid-scan leave the target running.
- scanmem: Ctrl-C during a scan ended the REPL instead of cancelling the scan. It now stops
  the scan and keeps the matches found so far ("stopped early: partial results"); a second
  Ctrl-C still exits.
- "Increased by" and "decreased by" scans panicked in debug builds when `old ± delta`
  overflowed the width being tested (an `i8` of 120 increased by 10). Integers now wrap, as
  release builds and C scanmem already did.

## [0.1.0] - 2026-09-04
