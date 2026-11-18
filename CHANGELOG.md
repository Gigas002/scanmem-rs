# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Fixed

- An attached process froze on the first signal it received (SIGCHLD, SIGALRM, a terminal
  resize, Ctrl-C), never got that signal, and stayed stopped after scanmem or gameconqueror
  quit. Attaching no longer makes the target a ptrace tracee: it is only stopped, with
  `PTRACE_SEIZE`/`PTRACE_INTERRUPT` and no SIGSTOP, for the length of a scan, and a signal
  that arrives meanwhile is delivered when it resumes. `reset`, attaching elsewhere and
  quitting mid-scan leave the target running.

## [0.1.0] - 2026-09-04
