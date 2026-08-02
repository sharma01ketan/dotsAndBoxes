//! Full game rules: players, scoring, extra turns, and terminal detection.

use crate::bitboard::BoxBits;
use crate::board::{BoardGeom, BoxId, EdgeId, Position};
use crate::moves::{CompletedBoxes, LegalMoves, MoveError, Undo};

/// Two-player side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Player {
    P1 = 0,
    P2 = 1,
}

impl Player {
    #[inline]
    pub const fn other(self) -> Self {
        match self {
            Player::P1 => Player::P2,
            Player::P2 => Player::P1,
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Outcome of a finished game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Winner {
    Player(Player),
    Draw,
}

/// Result of playing one edge under full rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayResult {
    /// Boxes awarded to the player who just moved.
    pub completed: CompletedBoxes,
    /// Player to move after this play (same as mover if any box was completed).
    pub next_player: Player,
    /// True when the mover earned an extra turn.
    pub extra_turn: bool,
}

/// Undo token for [`Game::play`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameUndo {
    position: Undo,
    previous_player: Player,
    /// Points awarded on this move (0–2), to `previous_player`.
    scored: u8,
}

/// Full game state with turn, scores, and per-player box ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Game {
    position: Position,
    current: Player,
    scores: [u16; 2],
    owner_p1: BoxBits,
    owner_p2: BoxBits,
}

impl Game {
    /// New game on `geom`; Player 1 to move, scores zero.
    pub const fn new(geom: BoardGeom) -> Self {
        Self {
            position: Position::new(geom),
            current: Player::P1,
            scores: [0, 0],
            owner_p1: BoxBits::EMPTY,
            owner_p2: BoxBits::EMPTY,
        }
    }

    #[inline]
    pub const fn position(self) -> Position {
        self.position
    }

    #[inline]
    pub const fn geom(self) -> BoardGeom {
        self.position.geom()
    }

    #[inline]
    pub const fn current_player(self) -> Player {
        self.current
    }

    #[inline]
    pub const fn score(self, player: Player) -> u16 {
        self.scores[player.index()]
    }

    #[inline]
    pub const fn scores(self) -> [u16; 2] {
        self.scores
    }

    /// Owner of a claimed box, if any.
    pub fn box_owner(self, box_id: BoxId) -> Option<Player> {
        if self.owner_p1.get(box_id) {
            Some(Player::P1)
        } else if self.owner_p2.get(box_id) {
            Some(Player::P2)
        } else {
            None
        }
    }

    /// Game is over when every box is claimed.
    #[inline]
    pub fn is_terminal(self) -> bool {
        self.position.boxes().count_ones() as u16 == self.geom().box_count()
    }

    /// Winner when terminal; `None` if the game is still in progress.
    pub fn winner(self) -> Option<Winner> {
        if !self.is_terminal() {
            return None;
        }
        let s1 = self.scores[0];
        let s2 = self.scores[1];
        Some(match s1.cmp(&s2) {
            core::cmp::Ordering::Greater => Winner::Player(Player::P1),
            core::cmp::Ordering::Less => Winner::Player(Player::P2),
            core::cmp::Ordering::Equal => Winner::Draw,
        })
    }

    pub fn legal_moves(self) -> LegalMoves {
        self.position.legal_moves()
    }

    pub fn is_legal(self, edge: EdgeId) -> bool {
        self.position.is_legal(edge)
    }

    /// Play `edge` for the current player.
    ///
    /// Completing one or more boxes awards them to the mover and grants an extra turn.
    pub fn play(&mut self, edge: EdgeId) -> Result<(PlayResult, GameUndo), MoveError> {
        let mover = self.current;
        let pos_undo = self.position.apply_move(edge)?;
        let completed = pos_undo.completed();
        let scored = completed.len() as u8;

        for &box_id in completed.as_slice() {
            match mover {
                Player::P1 => self.owner_p1.set(box_id),
                Player::P2 => self.owner_p2.set(box_id),
            }
        }
        self.scores[mover.index()] += scored as u16;

        let extra_turn = scored > 0;
        let next_player = if extra_turn { mover } else { mover.other() };
        self.current = next_player;

        Ok((
            PlayResult {
                completed,
                next_player,
                extra_turn,
            },
            GameUndo {
                position: pos_undo,
                previous_player: mover,
                scored,
            },
        ))
    }

    /// Reverse a previous [`Self::play`].
    pub fn undo(&mut self, undo: GameUndo) {
        for &box_id in undo.position.completed().as_slice() {
            match undo.previous_player {
                Player::P1 => self.owner_p1.clear(box_id),
                Player::P2 => self.owner_p2.clear(box_id),
            }
        }
        self.scores[undo.previous_player.index()] -= undo.scored as u16;
        self.current = undo.previous_player;
        self.position.undo(undo.position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{EdgeCoord, Orientation};

    fn edge(geom: BoardGeom, o: Orientation, row: u8, col: u8) -> EdgeId {
        geom.edge_id(EdgeCoord {
            orientation: o,
            row,
            col,
        })
        .unwrap()
    }

    #[test]
    fn no_capture_passes_turn() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        assert_eq!(game.current_player(), Player::P1);
        let (result, _) = game
            .play(edge(geom, Orientation::Horizontal, 0, 0))
            .unwrap();
        assert!(!result.extra_turn);
        assert_eq!(result.next_player, Player::P2);
        assert_eq!(game.current_player(), Player::P2);
        assert_eq!(game.scores(), [0, 0]);
    }

    #[test]
    fn capture_grants_extra_turn_and_score() {
        // 1×1 board: four edges complete the only box.
        let geom = BoardGeom::new(1, 1).unwrap();
        let mut game = Game::new(geom);
        let sides = geom.box_edges(0, 0).unwrap();

        for &e in &sides[..3] {
            let (r, _) = game.play(e).unwrap();
            assert!(!r.extra_turn);
        }
        // After 3 edges with no capture, turn alternates → P2 to move for the 4th
        // if each of the first three passed the turn. P1, P2, P1 played → P2's turn.
        assert_eq!(game.current_player(), Player::P2);
        let (r, _) = game.play(sides[3]).unwrap();
        assert!(r.extra_turn);
        assert_eq!(r.completed.as_slice(), &[0]);
        assert_eq!(game.score(Player::P2), 1);
        assert_eq!(game.score(Player::P1), 0);
        assert_eq!(game.box_owner(0), Some(Player::P2));
        assert_eq!(game.current_player(), Player::P2);
        assert!(game.is_terminal());
        assert_eq!(game.winner(), Some(Winner::Player(Player::P2)));
    }

    #[test]
    fn hand_verified_2x1_sample_game() {
        // 2 boxes in a row. Scripted play with known scores/turns.
        let geom = BoardGeom::new(1, 2).unwrap();
        let mut game = Game::new(geom);

        // Edges for left box (0,0) and right box (0,1).
        let left = geom.box_edges(0, 0).unwrap(); // top,bottom,left,right
        let right = geom.box_edges(0, 1).unwrap();

        // Shared vertical edge between boxes is left[3] == right[2].
        assert_eq!(left[3], right[2]);

        // P1 draws top-left, P2 top-right, P1 bottom-left, P2 bottom-right,
        // P1 left outer, P2 right outer — still no boxes.
        let script = [
            left[0],  // P1
            right[0], // P2
            left[1],  // P1
            right[1], // P2
            left[2],  // P1
            right[3], // P2
        ];
        for &e in &script {
            let (r, _) = game.play(e).unwrap();
            assert!(!r.extra_turn, "unexpected capture on edge {e}");
        }
        assert_eq!(game.scores(), [0, 0]);
        assert_eq!(game.current_player(), Player::P1);

        // P1 takes the shared middle → completes BOTH boxes → scores 2, extra turn, terminal.
        let (r, _) = game.play(left[3]).unwrap();
        assert!(r.extra_turn);
        assert_eq!(r.completed.len(), 2);
        assert_eq!(game.scores(), [2, 0]);
        assert!(game.is_terminal());
        assert_eq!(game.winner(), Some(Winner::Player(Player::P1)));
        assert_eq!(game.box_owner(0), Some(Player::P1));
        assert_eq!(game.box_owner(1), Some(Player::P1));
    }

    #[test]
    fn play_undo_restores_full_game_state() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        let start = game;
        let mut stack = Vec::new();

        for edge in game.legal_moves().collect::<Vec<_>>() {
            let before = game;
            let (_, undo) = game.play(edge).unwrap();
            stack.push((before, undo));
            if game.is_terminal() {
                break;
            }
        }

        while let Some((before, undo)) = stack.pop() {
            game.undo(undo);
            assert_eq!(game, before);
        }
        assert_eq!(game, start);
    }

    #[test]
    fn winner_none_while_in_progress() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        assert!(!game.is_terminal());
        assert_eq!(game.winner(), None);
    }
}
