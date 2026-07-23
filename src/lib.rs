#![doc = include_str!("../README.md")]

mod game;
mod widget;

pub use game::{is_logically_solvable, CellState, Clue, GameStatus, NonogramGame};
pub use widget::{content_size, NonogramWidget, TapMode};
