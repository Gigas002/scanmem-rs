# scanmem-rs

A from-scratch Rust rewrite of [scanmem](https://github.com/scanmem/scanmem)/GameConqueror — a process
memory scanner and editor: attach to a running process, scan its memory for a value, narrow the match
set down as the value changes, and write new values back.

## Documentation

- [docs/gameconqueror.md](./docs/gameconqueror.md) — `gameconqueror` usage guide: hotkeys, panels, Cargo
  feature flags, examples.
- [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) — workspace conventions: module layout, Cargo feature
  policy, testing/quality gates.

## License

`libscanmem` is LGPL-3.0-or-later; `scanmem` and `gameconqueror` are GPL-3.0-or-later — see
[LICENSE-LGPL-3.0](./LICENSE-LGPL-3.0) and [LICENSE-GPL-3.0](./LICENSE-GPL-3.0).
