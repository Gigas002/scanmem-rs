# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Changed

- First scans are 3 to 7 times faster and use far less memory: on a 256 MiB target, an
  `i32` equal-to scan went from 3.1 s to 0.96 s, an any-number one from 7.9 s to 1.1 s, and a
  snapshot from 9.9 s to 3.0 s; peak memory for a scan with few matches went from 103 MiB to
  14 MiB. Results are unchanged.

### Fixed

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
