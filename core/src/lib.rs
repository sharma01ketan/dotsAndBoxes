//! Shared Dots and Boxes game engine.
//!
//! This crate is the single source of truth for rules, bitboards, and solvers.
//! It compiles natively (server / training) and later to WASM (browser).

pub mod az;
pub mod bitboard;
pub mod board;
pub mod cgt;
pub mod engine;
pub mod features;
pub mod game;
pub mod mcts;
pub mod moves;
pub mod rng;
pub mod solver;

pub use az::{AzEngine, Evaluate};
pub use bitboard::{BoxBits, EdgeBits, BOX_WORDS, EDGE_WORDS};
pub use board::{BoardGeom, BoxId, EdgeCoord, EdgeId, Orientation, Position, MAX_COLS, MAX_ROWS};
pub use cgt::{
    analyze_endgame, analyze_position, encode_analysis, EndgameAnalysis, Region, RegionKind,
};
pub use engine::{CgtEngine, Engine, GreedyEngine, RandomEngine};
pub use features::{
    edge_from_policy_index, legal_policy_mask, policy_index, to_features, AZ_CHANNELS, AZ_FEATURES,
    AZ_FEATURES_VERSION, AZ_HUD_COLS, AZ_HUD_ROWS, AZ_PLANE, AZ_POLICY,
};
pub use game::{Game, GameUndo, PlayResult, Player, Winner};
pub use mcts::MctsEngine;
pub use moves::{CompletedBoxes, LegalMoves, MoveError, Undo, MAX_COMPLETED_PER_MOVE};
pub use rng::XorShift64;
pub use solver::{is_perfect_hud_size, perfect_value, PerfectEngine};

/// Crate identity helper (used by the server health payload).
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
