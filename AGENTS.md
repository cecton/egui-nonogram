# AGENTS.md

Instructions for AI coding agents working in this repository.

## What this is

`egui-nonogram` is a self-contained Rust library that implements a nonogram
(picross) puzzle game for [egui](https://github.com/emilk/egui): renderer-agnostic
game logic plus a ready-to-use `egui::Widget`. It has no application of its
own beyond the demo in `examples/webapp.rs` — it's meant to be pulled into
other egui apps as a dependency.

## Module layout

- `src/game.rs` — `NonogramGame`, `CellState`, `GameStatus`, `Clue`. Pure
  logic, no `egui::Widget`/`Ui` usage. Keep it that way: it should stay
  usable headlessly (e.g. for tests or a non-egui renderer) without pulling
  in any painting code.
- `src/widget.rs` — `NonogramWidget`, `TapMode`, and all painting/input
  handling. This is the only file allowed to depend on `egui::Ui`/`Painter`.
- `src/lib.rs` — thin re-export surface. `#![doc = include_str!("../README.md")]`
  means the crate-level docs are the README; keep the two in sync (usage
  snippets especially).
- `examples/webapp.rs` — a wasm demo app (via `xtask-wasm`), deployed to
  GitHub Pages by `.github/workflows/pages.yml` on every push to `main`.
  Not part of the published crate (`Cargo.toml` excludes `/examples`).

## Building and testing

```sh
cargo check
cargo test --lib
cargo clippy -- -D warnings
cargo fmt --check
```

These four are exactly what `.github/workflows/ci.yml` runs on every push
and PR. Run them locally before committing.

The wasm demo isn't covered by `ci.yml` (only `pages.yml` builds it, on
push to `main`). If you touch `examples/webapp.rs`, check it manually:

```sh
cargo check --target wasm32-unknown-unknown --example webapp
cargo clippy --target wasm32-unknown-unknown --example webapp -- -D warnings
```

## Conventions

- Monochrome only (no per-cell color/palette). `CellState` is `Empty` /
  `Filled` / `Crossed`. If color support is ever added, do it as a new,
  additive API — don't break the existing monochrome one.
- No losing state. Every player action must be reversible; the game only
  transitions `Playing` -> `Won` (or back, if an undo reverts a winning move).
- `Crossed` cells are a player aid and must never be required to match the
  solution for a win — only `Filled` cells are checked.
- Add unit tests in `src/game.rs` for any new game-logic behavior (clue
  computation, win detection, generation). The widget/painting code isn't
  unit-testable in the same way; verify it by eye via the wasm demo.
- Keep the crate's public API renderer-agnostic where possible: prefer
  exposing queries (`row_satisfied`, `is_solution_filled`, etc.) over raw
  field access, so the internal representation can change without breaking
  callers.

## Release process

Every published version gets a git tag and a changelog entry. To cut a release:

1. Update `CHANGELOG.md`: move the `[Unreleased]` section's contents under
   a new `## [X.Y.Z] - YYYY-MM-DD` heading (Keep a Changelog format), and
   add the corresponding link reference at the bottom of the file.
2. Bump the `version` in `Cargo.toml` to match.
3. Run the full check suite above, plus `cargo package --list` as a final
   sanity check of what will actually be published.
4. `cargo publish`. This is irreversible per-version (a bad release can
   only be `cargo yank`-ed, not deleted) — don't skip step 3.
5. `git tag vX.Y.Z && git push && git push --tags`.

Follow SemVer: breaking changes (renamed/removed public items, changed
method signatures) require a major version bump (or a minor bump pre-1.0,
per SemVer's pre-1.0 rules).
