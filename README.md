# egui-nonogram

[![crates.io](https://img.shields.io/crates/v/egui-nonogram.svg)](https://crates.io/crates/egui-nonogram)
[![docs.rs](https://docs.rs/egui-nonogram/badge.svg)](https://docs.rs/egui-nonogram)
[![deps.rs](https://deps.rs/repo/github/cecton/egui-nonogram/status.svg)](https://deps.rs/repo/github/cecton/egui-nonogram)
[![CI](https://github.com/cecton/egui-nonogram/actions/workflows/ci.yml/badge.svg)](https://github.com/cecton/egui-nonogram/actions/workflows/ci.yml)
[![Rust version](https://img.shields.io/badge/rustc-1.80+-ab6000.svg)](https://blog.rust-lang.org/2024/07/25/Rust-1.80.0.html)
[![License](https://img.shields.io/crates/l/egui-nonogram.svg)](https://github.com/cecton/egui-nonogram#license)
[![Changelog](https://img.shields.io/badge/changelog-Keep%20a%20Changelog%20v1.1.0-%23E05735)](CHANGELOG.md)
[![Live demo](https://img.shields.io/badge/demo-live-brightgreen)](https://cecton.github.io/egui-nonogram)

A self-contained Nonogram (picross) puzzle game library for [egui](https://github.com/emilk/egui).

## Features

- Pure game logic struct (`NonogramGame`) with no egui dependency — usable headlessly or with any renderer
- Ready-to-use egui `Widget` (`NonogramWidget`) that renders an interactive board with row/column clues
- Two ways to build a puzzle: `NonogramGame::from_grid` (supply your own picture) and `NonogramGame::random` (procedural, seeded)
- Monochrome only — cells are filled or empty, clues are plain run-length numbers
- Left-click fills a cell, right-click crosses it out; crossed cells are a player aid only and never need to match the solution
- Click-and-drag paints (or erases) a whole stroke of cells in one gesture
- Solved rows/columns dim and strike through their clue as a solving hint
- Optional read-only progress preview thumbnail beside the board
- Undo/redo history
- No losing state — every action is reversible

## Usage

Add the dependency:

```toml
[dependencies]
egui-nonogram = "0.1"
```

Then use it in your egui app:

```rust,ignore
use egui_nonogram::{NonogramGame, NonogramWidget};

// A curated puzzle from an explicit picture:
let grid = vec![
    vec![false, true, false],
    vec![true, true, true],
    vec![false, true, false],
];
let mut game = NonogramGame::from_grid(grid);

// Or a procedural puzzle, reproducible via a seed:
let mut game = NonogramGame::random(10, 10, 0.45, 42);

// Inside your egui update/UI closure:
ui.add(NonogramWidget::new(&mut game));
```

After each frame you can inspect `game.status` to check for a win:

```rust,ignore
use egui_nonogram::GameStatus;

match game.status {
    GameStatus::Playing => {}
    GameStatus::Won => println!("Solved!"),
}
```

To start over on the same puzzle:

```rust,ignore
game.reset();
```

## egui version compatibility

| egui-nonogram | egui |
|---------------|------|
| 0.1           | 0.35 |

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
