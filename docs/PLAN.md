# scanmem-rs — pre-implementation bootstrap plan

This document tracks the **one-time repository jobs** that must happen before Phase 0 of any of the three
per-crate implementation plans starts. The repo is currently still the generic `rust-template` scaffold
(placeholder crate names, empty `LICENSE`, default `MIT` license, empty package metadata, `TODO` markers in
CI workflows per the template's own [README.md](../README.md) "Requires manual changes" section) — none of
that is scanmem-rs-specific yet.

Sibling docs: [ARCHITECTURE.md](./ARCHITECTURE.md) (generic rules), [libscanmem-plan.md](./libscanmem-plan.md),
[scanmem-plan.md](./scanmem-plan.md), [gameconqueror-plan.md](./gameconqueror-plan.md) (the three
crate-specific implementation plans — their own "Phase 0" sections assume the jobs here are already done).

**This plan is infra/hygiene only — no engine, CLI, or GUI code.**

---

## 1. Licensing decision & fix

Upstream `scanmem`/`gameconqueror` is **GPL-3.0**, `libscanmem` is **LGPL-3.0**. The current template
defaults to a single workspace-wide `license = "MIT"` (in `[workspace.package]` and `deny.toml`'s
`[licenses].allow`), with an **empty** root `LICENSE` file.

| Task | Detail |
| ---- | ------ |
| [ ] Decide license per crate | **Recommended**: match upstream split — `libscanmem` = `LGPL-3.0-or-later`, `scanmem` and `gameconqueror` = `GPL-3.0-or-later`. This is a from-scratch rewrite (no C/Python source copied), so a uniform `MIT` license is legally defensible too — but a split rewrite of a GPL/LGPL project shipping as permissive `MIT` invites "license laundering" perception. Pick one and record the rationale in the root README's License section |
| [ ] Populate `LICENSE` | If split: add `LICENSE-LGPL-3.0` and `LICENSE-GPL-3.0` at repo root (or per-crate `LICENSE` files), remove/replace the single empty `LICENSE`. If MIT: fill the existing empty `LICENSE` with standard MIT text |
| [ ] Fix `Cargo.toml` license fields | Per-crate license differs under the split option, so `license.workspace = true` cannot be used uniformly — each crate's `[package]` sets its own `license = "..."` instead; `[workspace.package]` drops the `license` key (or keeps it only if MIT-everywhere is chosen) |
| [ ] Fix `deny.toml` | `[licenses].allow` must list whatever is actually used: `["LGPL-3.0-or-later", "GPL-3.0-or-later"]` for the split option, or stay `["MIT"]` if MIT is chosen |
| [ ] Cross-check `cargo deny check` | Re-run after the change — a GPL-3.0 binary crate depending on an LGPL-3.0 library crate is fine; make sure no dependency's license is incompatible with whichever choice is made |

---

## 2. Crate renaming & workspace metadata

| Task | Detail |
| ---- | ------ |
| [ ] Rename `crate/` → `libscanmem/` | `git mv crate libscanmem`; update `[package] name = "libscanmem"` in its `Cargo.toml` |
| [ ] Rename `binary-crate/` → `scanmem/` | `git mv binary-crate scanmem`; update `[package] name = "scanmem"` |
| [ ] Add `gameconqueror/` | New binary crate directory with a minimal `Cargo.toml` + `src/main.rs` stub (real work starts at [gameconqueror-plan.md Phase 0](./gameconqueror-plan.md#phase-0--crate-bootstrap)) |
| [ ] Fix `scanmem`'s dependency block | Currently `library-crate = { package = "crate", path = "../crate", version = "0.1.0" }` — replace with `libscanmem = { path = "../libscanmem", version = "0.1.0" }` |
| [ ] Drop the placeholder `extra` feature | Both `crate/Cargo.toml` and `binary-crate/Cargo.toml` have a template-only `extra = []` / `extra = ["library-crate/extra"]` feature with no real meaning — remove it; real feature sets are defined per crate in each plan's §2.1 (`signals`/`serde` for `libscanmem`, `tui`/`serde` for `scanmem`, `gtk4`/`iced` for `gameconqueror`) |
| [ ] Update root `Cargo.toml` | `members = ["libscanmem", "scanmem", "gameconqueror"]`; fill in `[workspace.package]` `description`, `homepage`, `repository` (`https://github.com/Gigas002/scanmem-rs`), `keywords` (e.g. `["memory", "scanner", "ptrace", "reverse-engineering"]`) — currently all empty strings/arrays |
| [ ] `Cargo.lock` | Regenerate and commit after the rename (`cargo generate-lockfile` or just `cargo build --workspace`) |

Do **not** pre-declare every dependency the three plans will eventually need in `[workspace.dependencies]`
here — add each one in the phase that first needs it, per
[ARCHITECTURE.md §7.1](./ARCHITECTURE.md#71-latest-versions) ("justify every new dependency in the change
that introduces it").

---

## 3. CI workflow fixes

The template's own README calls these out as required manual changes (`TODO` markers in the workflow
files) — resolve them now that final crate names exist:

| Task | File | Detail |
| ---- | ---- | ------ |
| [ ] Codecov matrix | `.github/workflows/test.yml` | `package: [binary-crate, crate]` → `package: [libscanmem, scanmem, gameconqueror]` |
| [ ] Deploy binaries/publish order | `.github/workflows/deploy.yml` | `BINARY_CRATES`/`PUBLISH_CRATES` → real names; `gameconqueror` also produces a binary artifact; publish order must be `libscanmem` before `scanmem`/`gameconqueror` (dependency order) |
| [ ] Confirm secrets exist | org/repo settings (outside this repo) | `CARGO_REGISTRY_TOKEN`, `CODECOV_TOKEN` — flag only, cannot be verified from the repo itself |

---

## 4. Root docs

| Task | Detail |
| ---- | ------ |
| [ ] Replace root `README.md` | Currently the generic `rust-template` doc ("Template for rust projects...") — replace with a real scanmem-rs overview: what it is, links to [ARCHITECTURE.md](./ARCHITECTURE.md) and the three crate plans, License section (result of §1), build/test instructions |
| [ ] `CHANGELOG.md` | Leave as-is (empty `## Unreleased`) until the first real change lands in any crate |
| [ ] `examples/` | Leave empty until a crate plan phase actually needs fixtures there |

---

## 5. Verify

- [ ] `cargo build --workspace`, `cargo fmt --all -- --check`, `cargo deny check` all green with the three
  real crate names and final license configuration.
- [ ] `rg -i "binary-crate|\"crate\"|library-crate|rust-template"` across the repo returns nothing stale
  (README, Cargo.toml, workflow files).
- [ ] `docs/libscanmem-plan.md`, `docs/scanmem-plan.md`, `docs/gameconqueror-plan.md` Phase 0 sections can
  now proceed without re-doing any of the above.

## 6. Non-goals

- No engine/CLI/GUI logic — tracked entirely in the three sibling plans.
- No upfront dependency pinning beyond what §2 lists — each sibling plan adds its own dependencies
  incrementally, per phase, as [ARCHITECTURE.md §7](./ARCHITECTURE.md#7-dependencies) requires.
