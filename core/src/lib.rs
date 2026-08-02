//! Shared Dots and Boxes game engine.
//!
//! This crate is the single source of truth for rules, bitboards, and solvers.
//! It compiles natively (server / training) and later to WASM (browser).

pub mod bitboard;
pub mod board;
pub mod game;
pub mod moves;

pub use bitboard::{BoxBits, EdgeBits, BOX_WORDS, EDGE_WORDS};
pub use board::{BoardGeom, BoxId, EdgeCoord, EdgeId, Orientation, Position, MAX_COLS, MAX_ROWS};
pub use game::{Game, GameUndo, PlayResult, Player, Winner};
pub use moves::{CompletedBoxes, LegalMoves, MoveError, Undo, MAX_COMPLETED_PER_MOVE};

/// Crate identity helper (used by the server stub smoke check).
pub fn crate_name() -> &'static str {
    "dab-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_stable() {
        assert_eq!(crate_name(), "dab-core");
    }
}
