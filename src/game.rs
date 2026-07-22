//! Renderer-agnostic nonogram game logic.

/// The player-visible state of a single cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CellState {
    /// Not yet marked by the player.
    #[default]
    Empty,
    /// Marked as part of the picture.
    Filled,
    /// Marked as definitely *not* part of the picture. Purely a player aid:
    /// never required to match the solution, and freely reversible.
    Crossed,
}

/// The current status of the game.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStatus {
    /// The puzzle is not yet solved.
    Playing,
    /// Every row and column satisfies its clue. Nonograms can have more than
    /// one grid that does this, so this isn't necessarily *the* solution.
    Won,
}

/// A single row or column clue: run-lengths of consecutive filled cells,
/// e.g. `[3, 1]` for a line shaped `###.#`. An all-empty line's clue is
/// `[0]`, never an empty vec, so it can be compared directly against a
/// freshly-computed clue from the player's current grid.
pub type Clue = Vec<u8>;

/// The nonogram game state: board dimensions, the hidden solution, the
/// player's grid, derived clues, and undo/redo history.
///
/// There is no losing state — every action is reversible via
/// [`Self::undo`]/[`Self::redo`], and the game only ever transitions from
/// [`GameStatus::Playing`] to [`GameStatus::Won`] (or back, if an undo
/// reverts a winning move).
pub struct NonogramGame {
    /// Board width in cells.
    pub width: usize,
    /// Board height in cells.
    pub height: usize,
    /// Flat row-major grid of the hidden solution (`solution[y * width + x]`).
    /// Private: consumers query it via [`Self::is_solution_filled`] rather
    /// than reading it directly, keeping the door open to a more compact
    /// representation later without breaking the public API.
    solution: Vec<bool>,
    /// Flat row-major grid of the player's current marks (`cells[y * width + x]`).
    pub cells: Vec<CellState>,
    /// Clue for each row, top to bottom.
    pub row_clues: Vec<Clue>,
    /// Clue for each column, left to right.
    pub col_clues: Vec<Clue>,
    /// Current game status.
    pub status: GameStatus,
    undo_stack: Vec<Vec<CellState>>,
    redo_stack: Vec<Vec<CellState>>,
}

impl NonogramGame {
    /// Build a puzzle from an explicit solution grid (row-major, `grid[y][x]`).
    /// Every row must have the same width. Use this for curated/picture puzzles.
    ///
    /// # Panics
    ///
    /// Panics if `grid` is empty, any row is empty, or rows have differing
    /// lengths.
    pub fn from_grid(grid: Vec<Vec<bool>>) -> Self {
        let height = grid.len();
        assert!(height > 0, "solution grid must have at least one row");
        let width = grid[0].len();
        assert!(width > 0, "solution rows must have at least one column");
        assert!(
            grid.iter().all(|row| row.len() == width),
            "solution rows must all have the same width"
        );
        let solution: Vec<bool> = grid.into_iter().flatten().collect();
        Self::from_solution(width, height, solution)
    }

    /// Build a procedurally-generated puzzle: each cell is independently
    /// filled with probability `density`, reproducible via `seed`.
    /// Regenerates until the result isn't degenerate (not all-empty, not
    /// all-filled), so the clues are always meaningful. `density` is
    /// clamped to `0.05..=0.95` — the exact extremes would make a
    /// non-degenerate result impossible (and the regeneration loop would
    /// never terminate).
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` is zero.
    pub fn random(width: usize, height: usize, density: f32, seed: u64) -> Self {
        assert!(width > 0 && height > 0);
        let density = density.clamp(0.05, 0.95);
        let mut rng = fastrand::Rng::with_seed(seed);
        loop {
            let solution: Vec<bool> = (0..width * height).map(|_| rng.f32() < density).collect();
            let filled = solution.iter().filter(|&&v| v).count();
            if filled > 0 && filled < solution.len() {
                return Self::from_solution(width, height, solution);
            }
        }
    }

    fn from_solution(width: usize, height: usize, solution: Vec<bool>) -> Self {
        let row_clues = (0..height)
            .map(|y| Self::line_clue(&solution[y * width..(y + 1) * width]))
            .collect();
        let col_clues = (0..width)
            .map(|x| {
                let col: Vec<bool> = (0..height).map(|y| solution[y * width + x]).collect();
                Self::line_clue(&col)
            })
            .collect();
        let mut game = Self {
            width,
            height,
            solution,
            cells: vec![CellState::Empty; width * height],
            row_clues,
            col_clues,
            status: GameStatus::Playing,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        game.cross_out_empty_lines();
        game
    }

    /// Clear the player's grid and undo/redo history, keeping the same
    /// puzzle (solution and clues unchanged). To get a *different* puzzle,
    /// construct a new [`NonogramGame`].
    pub fn reset(&mut self) {
        self.cells = vec![CellState::Empty; self.width * self.height];
        self.status = GameStatus::Playing;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.cross_out_empty_lines();
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Whether `(x, y)` is part of the hidden solution.
    pub fn is_solution_filled(&self, x: usize, y: usize) -> bool {
        self.solution[self.idx(x, y)]
    }

    /// Toggle a cell between `Filled` and `Empty` (clearing any `Crossed`
    /// mark in the process). This is the left-click action.
    pub fn toggle_fill(&mut self, x: usize, y: usize) {
        let new_state = if self.cells[self.idx(x, y)] == CellState::Filled {
            CellState::Empty
        } else {
            CellState::Filled
        };
        self.set_cell(x, y, new_state);
    }

    /// Toggle a cell between `Crossed` and `Empty` (clearing any `Filled`
    /// mark in the process). This is the right-click action.
    pub fn toggle_cross(&mut self, x: usize, y: usize) {
        let new_state = if self.cells[self.idx(x, y)] == CellState::Crossed {
            CellState::Empty
        } else {
            CellState::Crossed
        };
        self.set_cell(x, y, new_state);
    }

    /// Directly clear a cell back to `Empty`.
    pub fn clear_cell(&mut self, x: usize, y: usize) {
        self.set_cell(x, y, CellState::Empty);
    }

    /// Directly set a cell to `Filled`, no toggling. Idempotent — a no-op
    /// (no undo entry pushed) if the cell is already `Filled`. Intended for
    /// click-and-drag painting, where the same target state is applied to
    /// every cell the pointer passes over.
    pub fn fill(&mut self, x: usize, y: usize) {
        self.set_cell(x, y, CellState::Filled);
    }

    /// Directly set a cell to `Crossed`, no toggling. Idempotent, for the
    /// same click-and-drag painting use case as [`Self::fill`].
    pub fn cross(&mut self, x: usize, y: usize) {
        self.set_cell(x, y, CellState::Crossed);
    }

    fn set_cell(&mut self, x: usize, y: usize, new_state: CellState) {
        let idx = self.idx(x, y);
        if self.cells[idx] == new_state {
            return;
        }
        self.undo_stack.push(self.cells.clone());
        self.redo_stack.clear();
        self.cells[idx] = new_state;
        self.auto_cross_completed_lines(x, y);
        self.check_win();
    }

    /// Cross out the remaining `Empty` cells of row `y` and column `x` if
    /// that line's filled cells already match its clue: once satisfied, no
    /// other cell in the line can ever be filled without breaking the
    /// match, so the rest are surely empty. Only the line containing the
    /// just-changed cell can have changed satisfaction.
    fn auto_cross_completed_lines(&mut self, x: usize, y: usize) {
        self.cross_row_if_satisfied(y);
        self.cross_col_if_satisfied(x);
    }

    /// Cross out every row/column whose clue is `[0]`: on an all-`Empty`
    /// grid, those lines are already satisfied, but [`Self::set_cell`]'s
    /// reactive auto-crossing never runs on them since nothing ever
    /// mutates a cell in an already-solved empty line. Called once at
    /// construction and on [`Self::reset`] to keep the board consistent
    /// with the clue dimming, which uses the same satisfaction check.
    fn cross_out_empty_lines(&mut self) {
        for y in 0..self.height {
            self.cross_row_if_satisfied(y);
        }
        for x in 0..self.width {
            self.cross_col_if_satisfied(x);
        }
    }

    fn cross_row_if_satisfied(&mut self, y: usize) {
        if self.row_satisfied(y) {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                if self.cells[idx] == CellState::Empty {
                    self.cells[idx] = CellState::Crossed;
                }
            }
        }
    }

    fn cross_col_if_satisfied(&mut self, x: usize) {
        if self.col_satisfied(x) {
            for y in 0..self.height {
                let idx = self.idx(x, y);
                if self.cells[idx] == CellState::Empty {
                    self.cells[idx] = CellState::Crossed;
                }
            }
        }
    }

    /// Whether row `y`'s currently-filled cells already match its clue. Used
    /// both as a hint for the player (dim/strikethrough the clue) and,
    /// together with [`Self::col_satisfied`], as the win condition.
    pub fn row_satisfied(&self, y: usize) -> bool {
        let line: Vec<bool> = (0..self.width)
            .map(|x| self.cells[self.idx(x, y)] == CellState::Filled)
            .collect();
        Self::line_clue(&line) == self.row_clues[y]
    }

    /// Whether column `x`'s currently-filled cells already match its clue.
    /// See [`Self::row_satisfied`].
    pub fn col_satisfied(&self, x: usize) -> bool {
        let line: Vec<bool> = (0..self.height)
            .map(|y| self.cells[self.idx(x, y)] == CellState::Filled)
            .collect();
        Self::line_clue(&line) == self.col_clues[x]
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack
                .push(std::mem::replace(&mut self.cells, prev));
            self.check_win();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack
                .push(std::mem::replace(&mut self.cells, next));
            self.check_win();
        }
    }

    fn check_win(&mut self) {
        let won = (0..self.height).all(|y| self.row_satisfied(y))
            && (0..self.width).all(|x| self.col_satisfied(x));
        self.status = if won {
            GameStatus::Won
        } else {
            GameStatus::Playing
        };
    }

    /// Run-length-encode a line of booleans into a clue. An all-`false`
    /// line yields `[0]`, matching standard nonogram convention and keeping
    /// this directly comparable to a stored clue with no special-casing.
    fn line_clue(line: &[bool]) -> Clue {
        let mut clue = Vec::new();
        let mut run = 0u8;
        for &filled in line {
            if filled {
                run += 1;
            } else if run > 0 {
                clue.push(run);
                run = 0;
            }
        }
        if run > 0 {
            clue.push(run);
        }
        if clue.is_empty() {
            clue.push(0);
        }
        clue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_clue_examples() {
        assert_eq!(NonogramGame::line_clue(&[false, false, false]), vec![0]);
        assert_eq!(
            NonogramGame::line_clue(&[false, true, true, false, true]),
            vec![2, 1]
        );
        assert_eq!(NonogramGame::line_clue(&[true, true, true]), vec![3]);
        assert_eq!(
            NonogramGame::line_clue(&[true, false, true, false, true]),
            vec![1, 1, 1]
        );
    }

    #[test]
    fn from_grid_computes_clues() {
        let grid = vec![
            vec![false, true, false],
            vec![true, true, true],
            vec![false, true, false],
        ];
        let game = NonogramGame::from_grid(grid);
        assert_eq!(game.row_clues, vec![vec![1], vec![3], vec![1]]);
        assert_eq!(game.col_clues, vec![vec![1], vec![3], vec![1]]);
    }

    #[test]
    #[should_panic(expected = "same width")]
    fn from_grid_rejects_ragged_rows() {
        NonogramGame::from_grid(vec![vec![true, false], vec![true]]);
    }

    #[test]
    fn row_and_col_satisfied() {
        let grid = vec![vec![true, false], vec![false, true]];
        let mut game = NonogramGame::from_grid(grid);
        assert!(!game.row_satisfied(0));
        game.toggle_fill(0, 0);
        assert!(game.row_satisfied(0));
        assert!(game.col_satisfied(0));
        assert!(!game.row_satisfied(1));
    }

    #[test]
    fn crossed_cells_do_not_count_against_satisfaction_or_win() {
        let grid = vec![vec![true, false]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        game.toggle_cross(1, 0); // correctly-empty cell, crossed out
        assert_eq!(game.status, GameStatus::Won);
    }

    #[test]
    fn completing_a_row_auto_crosses_its_remaining_cells() {
        let grid = vec![vec![true, true, false], vec![false, false, true]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        assert_eq!(game.cells[game.idx(2, 0)], CellState::Empty);
        game.toggle_fill(1, 0);
        assert!(game.row_satisfied(0));
        assert_eq!(game.cells[game.idx(2, 0)], CellState::Crossed);
    }

    #[test]
    fn completing_a_column_auto_crosses_its_remaining_cells() {
        let grid = vec![vec![true, false], vec![true, false], vec![false, true]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        assert_eq!(game.cells[game.idx(0, 2)], CellState::Empty);
        game.toggle_fill(0, 1);
        assert!(game.col_satisfied(0));
        assert_eq!(game.cells[game.idx(0, 2)], CellState::Crossed);
    }

    #[test]
    fn auto_crossed_cells_do_not_count_against_satisfaction_or_win() {
        let grid = vec![vec![true, false]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        assert_eq!(game.cells[game.idx(1, 0)], CellState::Crossed);
        assert_eq!(game.status, GameStatus::Won);
    }

    #[test]
    fn undo_reverts_auto_crossing_along_with_the_completing_move() {
        let grid = vec![vec![true, false], vec![false, true]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        assert_eq!(game.cells[game.idx(1, 0)], CellState::Crossed);
        game.undo();
        assert_eq!(game.cells[game.idx(1, 0)], CellState::Empty);
    }

    #[test]
    fn un_satisfying_a_line_later_leaves_its_auto_crosses_in_place() {
        let grid = vec![vec![true, false], vec![false, true]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        assert_eq!(game.cells[game.idx(1, 0)], CellState::Crossed);
        game.toggle_fill(0, 0); // un-fill the completing cell directly (not undo)
        assert!(!game.row_satisfied(0));
        assert_eq!(game.cells[game.idx(1, 0)], CellState::Crossed);
    }

    #[test]
    fn empty_line_is_crossed_out_from_the_start() {
        let grid = vec![vec![true, false], vec![false, false]];
        let game = NonogramGame::from_grid(grid);
        assert_eq!(game.row_clues[1], vec![0]);
        assert_eq!(game.cells[game.idx(0, 1)], CellState::Crossed);
        assert_eq!(game.cells[game.idx(1, 1)], CellState::Crossed);
        // Row 0 isn't satisfied yet, so its cells are untouched.
        assert_eq!(game.cells[game.idx(0, 0)], CellState::Empty);
    }

    #[test]
    fn reset_reapplies_the_empty_line_crossing() {
        let grid = vec![vec![true, false], vec![false, false]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        game.reset();
        assert_eq!(game.cells[game.idx(0, 1)], CellState::Crossed);
        assert_eq!(game.cells[game.idx(0, 0)], CellState::Empty);
    }

    #[test]
    fn wrongly_crossed_required_cell_blocks_win() {
        let grid = vec![vec![true, false]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_cross(0, 0); // required cell, wrongly crossed instead of filled
        assert_eq!(game.status, GameStatus::Playing);
        game.toggle_fill(1, 0); // fill the wrong (non-solution) cell too
        assert_eq!(game.status, GameStatus::Playing);
    }

    #[test]
    fn undo_redo_round_trip() {
        let grid = vec![vec![true, true]];
        let mut game = NonogramGame::from_grid(grid);
        assert!(!game.can_undo());
        game.toggle_fill(0, 0);
        game.toggle_fill(1, 0);
        assert_eq!(game.status, GameStatus::Won);
        assert!(game.can_undo());
        game.undo();
        assert_eq!(game.status, GameStatus::Playing);
        assert_eq!(game.cells[1], CellState::Empty);
        assert!(game.can_redo());
        game.redo();
        assert_eq!(game.status, GameStatus::Won);
    }

    #[test]
    fn reset_keeps_puzzle_clears_progress() {
        let grid = vec![vec![true]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(0, 0);
        assert_eq!(game.status, GameStatus::Won);
        let row_clues_before = game.row_clues.clone();
        game.reset();
        assert_eq!(game.status, GameStatus::Playing);
        assert!(game.cells.iter().all(|&c| c == CellState::Empty));
        assert!(!game.can_undo());
        assert_eq!(game.row_clues, row_clues_before);
    }

    #[test]
    fn random_is_deterministic_per_seed() {
        let a = NonogramGame::random(10, 10, 0.4, 42);
        let b = NonogramGame::random(10, 10, 0.4, 42);
        assert_eq!(a.row_clues, b.row_clues);
        assert_eq!(a.col_clues, b.col_clues);
    }

    #[test]
    fn col_satisfied_after_filling_whole_column() {
        // "Stairs" puzzle bit pattern from the doneward catalog: width 10,
        // rows &[15, 15, 63, 63, 255, 255, 1023, 1023, 1023, 1023].
        let rows: [u16; 10] = [15, 15, 63, 63, 255, 255, 1023, 1023, 1023, 1023];
        let grid: Vec<Vec<bool>> = rows
            .iter()
            .map(|&row| (0..10).map(|x| (row >> x) & 1 != 0).collect())
            .collect();
        let mut game = NonogramGame::from_grid(grid);
        assert_eq!(game.col_clues[0], vec![10]);
        for y in 0..10 {
            game.toggle_fill(0, y);
        }
        for y in 0..10 {
            assert_eq!(game.cells[game.idx(0, y)], CellState::Filled);
        }
        assert!(
            game.col_satisfied(0),
            "column 0 should be satisfied after filling all 10 of its cells"
        );
    }

    #[test]
    fn alternate_valid_solution_also_wins() {
        // Diagonal solution: row clues [1],[1], col clues [1],[1] — but the
        // anti-diagonal satisfies the exact same clues, so it's an equally
        // valid solution and must also count as a win.
        let grid = vec![vec![true, false], vec![false, true]];
        let mut game = NonogramGame::from_grid(grid);
        game.toggle_fill(1, 0);
        game.toggle_fill(0, 1);
        assert_eq!(game.status, GameStatus::Won);
    }

    #[test]
    fn random_avoids_degenerate_puzzles() {
        // Density 0.0/1.0 would be degenerate if not corrected for; the
        // generator must still terminate and produce a non-degenerate grid.
        let game = NonogramGame::random(4, 4, 0.0, 7);
        let filled = (0..4)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .filter(|&(x, y)| game.is_solution_filled(x, y))
            .count();
        assert!(filled > 0 && filled < 16);
    }
}
