# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](keep_a_changelog) and this project adheres to [Semantic
Versioning](semver).

## [Unreleased]

## [0.1.3] - 2026-07-22

### Added

- `NonogramGame` now automatically crosses out remaining cells of a row or column once its filled cells match the clue — applied reactively on every player move, and also on construction/reset for all-empty lines
- `NonogramWidget::interactive` builder flag to render the board read-only without dimming
- `content_size` public helper function to pre-compute the widget's total footprint
- Mobile/narrow-mode layout for the web demo: bottom action bar with Pan/Fill/Cross mode toggle, pannable/zoomable board, and hamburger menu for preset selection, new game, and theme

### Fixed

- Win condition now checks against clues via `row_satisfied`/`col_satisfied` instead of the stored solution grid, so alternate valid arrangements also trigger a win
- Web demo no longer shows the same initial board on every page load: seed is now random instead of hardcoded

## [0.1.1] - 2026-07-21

### Added

- `NonogramWidget` now draws a win banner over the board once the puzzle is solved, sized to the board itself so it never covers the clue gutters or the picture-preview column
- `NonogramWidget::win_message` to customize the banner's text (defaults to `"Solved!"`)

## [0.1.0] - 2026-07-21

### Added

- Initial release of `egui-nonogram`
- `NonogramGame` core game logic API (renderer-agnostic)
- `NonogramGame::from_grid` to build a puzzle from an explicit picture
- `NonogramGame::random` to build a seeded procedural puzzle
- `NonogramWidget` egui widget for interactive board rendering with row/column clues
- Left-click fill / right-click cross-out interaction, with crossed cells never required to match the solution
- Click-and-drag painting: the action decided by the cell a drag starts on is applied to every cell the pointer passes over for the rest of the gesture
- `NonogramGame::fill`/`NonogramGame::cross` direct-set methods (no toggling), for drag-painting and other non-toggle use cases
- Solved-line clue dimming and strikethrough (rows horizontal, columns vertical) as a solving hint
- `NonogramWidget::show_preview` — an optional read-only thumbnail of the currently-filled cells beside the board
- Undo/redo history
- Web example and GitHub Pages deployment workflow

[keep_a_changelog]: https://keepachangelog.com/en/1.1.0
[semver]: https://semver.org/spec/v2.0.0.html
[Unreleased]: https://github.com/cecton/egui-nonogram/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/cecton/egui-nonogram/releases/tag/v0.1.3
[0.1.1]: https://github.com/cecton/egui-nonogram/releases/tag/v0.1.1
[0.1.0]: https://github.com/cecton/egui-nonogram/releases/tag/v0.1.0
