//! Thin WASM bindings over [`dab_core`].
//!
//! API is deliberately data-oriented: indices, typed arrays, and small numeric
//! codes — no rich JS objects — so the web app stays in control of UI state.

use std::cell::RefCell;

use dab_core::{
    is_perfect_hud_size as core_is_perfect_hud_size, BoardGeom, CgtEngine, EdgeCoord, EdgeId,
    Engine, Game, GreedyEngine, MctsEngine, MoveError, Orientation, PerfectEngine, Player,
    RandomEngine, Winner,
};
use wasm_bindgen::prelude::*;

/// Orientation codes for JS: `0` = horizontal, `1` = vertical.
pub const ORIENT_H: u8 = 0;
pub const ORIENT_V: u8 = 1;

/// Winner codes: `-1` in progress, `0` P1, `1` P2, `2` draw.
pub const WINNER_NONE: i8 = -1;
pub const WINNER_P1: i8 = 0;
pub const WINNER_P2: i8 = 1;
pub const WINNER_DRAW: i8 = 2;

/// Box owner codes: `-1` unclaimed, `0` P1, `1` P2.
pub const OWNER_NONE: i8 = -1;

/// `choose_move` policy: uniform random legal move.
pub const POLICY_RANDOM: u8 = 0;
/// `choose_move` policy: greedy (take boxes / avoid giving).
pub const POLICY_GREEDY: u8 = 1;
/// `choose_move` policy: CGT heuristic (double-cross / all-but-four).
pub const POLICY_CGT: u8 = 2;
/// `choose_move` policy: exact Perfect (2×2 / 3×3 only).
pub const POLICY_PERFECT: u8 = 3;
/// `choose_move` policy: UCT MCTS (greedy rollouts).
pub const POLICY_MCTS: u8 = 4;

#[wasm_bindgen(js_name = POLICY_RANDOM)]
pub fn policy_random() -> u8 {
    POLICY_RANDOM
}

#[wasm_bindgen(js_name = POLICY_GREEDY)]
pub fn policy_greedy() -> u8 {
    POLICY_GREEDY
}

#[wasm_bindgen(js_name = POLICY_CGT)]
pub fn policy_cgt() -> u8 {
    POLICY_CGT
}

#[wasm_bindgen(js_name = POLICY_PERFECT)]
pub fn policy_perfect() -> u8 {
    POLICY_PERFECT
}

#[wasm_bindgen(js_name = POLICY_MCTS)]
pub fn policy_mcts() -> u8 {
    POLICY_MCTS
}

/// Square 2×2 / 3×3 only. HUD and `chooseMove(3)` share this.
#[wasm_bindgen(js_name = isPerfectHudSize)]
pub fn is_perfect_hud_size(rows: u8, cols: u8) -> bool {
    core_is_perfect_hud_size(rows, cols)
}

thread_local! {
    static PERFECT: RefCell<PerfectEngine> = RefCell::new(PerfectEngine::new(1));
}

fn with_perfect<R>(f: impl FnOnce(&mut PerfectEngine) -> R) -> R {
    PERFECT.with(|slot| f(&mut slot.borrow_mut()))
}

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

fn js_err(msg: impl Into<String>) -> JsValue {
    JsValue::from_str(&msg.into())
}

fn move_err(err: MoveError, edge: EdgeId) -> JsValue {
    match err {
        MoveError::AlreadyDrawn => js_err(format!("edge {edge} is already drawn")),
        MoveError::OutOfRange => js_err(format!("edge {edge} is out of range")),
    }
}

/// Browser-facing game handle wrapping [`Game`].
#[wasm_bindgen]
pub struct WasmGame {
    inner: Game,
}

#[wasm_bindgen]
impl WasmGame {
    /// Create a new game with `rows × cols` boxes.
    #[wasm_bindgen(constructor)]
    pub fn new(rows: u8, cols: u8) -> Result<WasmGame, JsValue> {
        let geom = BoardGeom::new(rows, cols).ok_or_else(|| {
            js_err(format!(
                "invalid board size {rows}x{cols} (supported 1..={} by 1..={})",
                dab_core::MAX_ROWS,
                dab_core::MAX_COLS
            ))
        })?;
        Ok(Self {
            inner: Game::new(geom),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn rows(&self) -> u8 {
        self.inner.geom().rows()
    }

    #[wasm_bindgen(getter)]
    pub fn cols(&self) -> u8 {
        self.inner.geom().cols()
    }

    #[wasm_bindgen(js_name = edgeCount)]
    pub fn edge_count(&self) -> u16 {
        self.inner.geom().edge_count()
    }

    #[wasm_bindgen(js_name = boxCount)]
    pub fn box_count(&self) -> u16 {
        self.inner.geom().box_count()
    }

    /// Current player: `0` = P1, `1` = P2.
    #[wasm_bindgen(js_name = currentPlayer)]
    pub fn current_player(&self) -> u8 {
        self.inner.current_player().index() as u8
    }

    #[wasm_bindgen(js_name = scoreP1)]
    pub fn score_p1(&self) -> u16 {
        self.inner.score(Player::P1)
    }

    #[wasm_bindgen(js_name = scoreP2)]
    pub fn score_p2(&self) -> u16 {
        self.inner.score(Player::P2)
    }

    #[wasm_bindgen(js_name = isTerminal)]
    pub fn is_terminal(&self) -> bool {
        self.inner.is_terminal()
    }

    /// `-1` in progress, `0` P1 wins, `1` P2 wins, `2` draw.
    pub fn winner(&self) -> i8 {
        match self.inner.winner() {
            None => WINNER_NONE,
            Some(Winner::Player(Player::P1)) => WINNER_P1,
            Some(Winner::Player(Player::P2)) => WINNER_P2,
            Some(Winner::Draw) => WINNER_DRAW,
        }
    }

    #[wasm_bindgen(js_name = isLegal)]
    pub fn is_legal(&self, edge: u16) -> bool {
        self.inner.is_legal(edge)
    }

    #[wasm_bindgen(js_name = edgeIsDrawn)]
    pub fn edge_is_drawn(&self, edge: u16) -> bool {
        if edge >= self.edge_count() {
            return false;
        }
        self.inner.position().edge_is_drawn(edge)
    }

    /// `-1` unclaimed or out of range, `0` P1, `1` P2.
    #[wasm_bindgen(js_name = boxOwner)]
    pub fn box_owner(&self, box_id: u16) -> i8 {
        if box_id >= self.box_count() {
            return OWNER_NONE;
        }
        match self.inner.box_owner(box_id) {
            None => OWNER_NONE,
            Some(Player::P1) => 0,
            Some(Player::P2) => 1,
        }
    }

    /// Undrawn edge ids (JS: `Uint16Array`).
    #[wasm_bindgen(js_name = legalMoves)]
    pub fn legal_moves(&self) -> Vec<u16> {
        self.inner.legal_moves().collect()
    }

    /// Resolve edge id → `[orientation, row, col]` (`orientation`: 0=H, 1=V).
    #[wasm_bindgen(js_name = edgeCoord)]
    pub fn edge_coord(&self, edge: u16) -> Result<Vec<u16>, JsValue> {
        let coord = self
            .inner
            .geom()
            .edge_coord(edge)
            .ok_or_else(|| js_err(format!("edge {edge} out of range")))?;
        let orient = match coord.orientation {
            Orientation::Horizontal => ORIENT_H as u16,
            Orientation::Vertical => ORIENT_V as u16,
        };
        Ok(vec![orient, coord.row as u16, coord.col as u16])
    }

    /// Resolve `(orientation, row, col)` → edge id (`orientation`: 0=H, 1=V).
    #[wasm_bindgen(js_name = edgeId)]
    pub fn edge_id(&self, orientation: u8, row: u8, col: u8) -> Result<u16, JsValue> {
        let orientation = match orientation {
            ORIENT_H => Orientation::Horizontal,
            ORIENT_V => Orientation::Vertical,
            _ => return Err(js_err("orientation must be 0 (H) or 1 (V)")),
        };
        self.inner
            .geom()
            .edge_id(EdgeCoord {
                orientation,
                row,
                col,
            })
            .ok_or_else(|| {
                js_err(format!(
                    "coord out of range: orientation={orient_code} row={row} col={col}",
                    orient_code = match orientation {
                        Orientation::Horizontal => ORIENT_H,
                        Orientation::Vertical => ORIENT_V,
                    }
                ))
            })
    }

    /// Play an edge. Returns `[extraTurn (0/1), completedCount, ...completedBoxIds]`.
    pub fn play(&mut self, edge: u16) -> Result<Vec<u16>, JsValue> {
        let (result, _) = self.inner.play(edge).map_err(|e| move_err(e, edge))?;
        let mut out = Vec::with_capacity(2 + result.completed.len());
        out.push(if result.extra_turn { 1 } else { 0 });
        out.push(result.completed.len() as u16);
        out.extend_from_slice(result.completed.as_slice());
        Ok(out)
    }

    /// Compact CGT analysis dump (KET-21). Does not mutate the game.
    pub fn analyze(&self) -> Vec<u16> {
        dab_core::encode_analysis(&self.inner)
    }

    /// Choose a legal edge without applying it.
    ///
    /// `policy`: `0` = random, `1` = greedy, `2` = CGT, `3` = Perfect, `4` = MCTS.
    /// `seed` seeds the engine RNG.
    #[wasm_bindgen(js_name = chooseMove)]
    pub fn choose_move(&mut self, policy: u8, seed: u64) -> Result<u16, JsValue> {
        if self.inner.is_terminal() {
            return Err(js_err("cannot choose a move on a terminal game"));
        }
        let edge = match policy {
            POLICY_RANDOM => RandomEngine::new(seed).choose(&self.inner),
            POLICY_GREEDY => GreedyEngine::new(seed).choose(&self.inner),
            POLICY_CGT => CgtEngine::new(seed).choose(&self.inner),
            POLICY_MCTS => MctsEngine::new(seed).choose(&self.inner),
            POLICY_PERFECT => {
                let geom = self.inner.geom();
                if !core_is_perfect_hud_size(geom.rows(), geom.cols()) {
                    return Err(js_err("Perfect is only available on 2×2 and 3×3 boards"));
                }
                with_perfect(|engine| {
                    engine.set_seed(seed);
                    engine.choose(&self.inner)
                })
            }
            _ => {
                return Err(js_err(format!(
                    "unknown policy {policy} (0=random, 1=greedy, 2=cgt, 3=perfect, 4=mcts)"
                )));
            }
        };
        Ok(edge)
    }

    /// Box-difference margin for the side to move (2×2 / 3×3 only).
    #[wasm_bindgen(js_name = perfectValue)]
    pub fn perfect_value(&self) -> Result<i8, JsValue> {
        let geom = self.inner.geom();
        if !core_is_perfect_hud_size(geom.rows(), geom.cols()) {
            return Err(js_err("Perfect is only available on 2×2 and 3×3 boards"));
        }
        Ok(with_perfect(|engine| engine.value(&self.inner)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_game_play_smoke() {
        let mut game = WasmGame::new(1, 1).unwrap();
        assert_eq!(game.edge_count(), 4);
        assert_eq!(game.legal_moves().len(), 4);
        let result = game.play(0).unwrap();
        assert_eq!(result[0], 0); // no extra turn
        assert_eq!(game.current_player(), 1);
        assert_eq!(game.score_p1(), 0);
        assert!(game.edge_is_drawn(0));
    }

    #[test]
    fn choose_move_returns_legal_and_does_not_play() {
        let mut game = WasmGame::new(2, 2).unwrap();
        let edge = game.choose_move(POLICY_GREEDY, 42).unwrap();
        assert!(game.is_legal(edge));
        assert!(!game.edge_is_drawn(edge));
        assert_eq!(game.current_player(), 0);
        assert_eq!(game.legal_moves().len(), game.edge_count() as usize);

        let again = game.choose_move(POLICY_RANDOM, 7).unwrap();
        assert!(game.is_legal(again));

        let cgt = game.choose_move(POLICY_CGT, 11).unwrap();
        assert!(game.is_legal(cgt));
        assert!(!game.edge_is_drawn(cgt));

        let perfect = game.choose_move(POLICY_PERFECT, 13).unwrap();
        assert!(game.is_legal(perfect));
        assert!(!game.edge_is_drawn(perfect));

        let mcts = game.choose_move(POLICY_MCTS, 17).unwrap();
        assert!(game.is_legal(mcts));
        assert!(!game.edge_is_drawn(mcts));
        assert_eq!(game.current_player(), 0);
        assert_eq!(game.legal_moves().len(), game.edge_count() as usize);

        let v = game.perfect_value().unwrap();
        assert_eq!(
            v,
            dab_core::perfect_value(&dab_core::Game::new(
                dab_core::BoardGeom::new(2, 2).unwrap()
            ))
        );
    }

    #[test]
    fn box_owner_oob_is_none_and_does_not_panic() {
        let game = WasmGame::new(2, 2).unwrap();
        assert_eq!(game.box_count(), 4);
        assert_eq!(game.box_owner(0), OWNER_NONE);
        assert_eq!(game.box_owner(game.box_count()), OWNER_NONE);
        assert_eq!(game.box_owner(200), OWNER_NONE);
        assert!(!game.edge_is_drawn(game.edge_count()));
    }

    #[test]
    fn box_owner_claimed_is_player_code() {
        let mut game = WasmGame::new(1, 1).unwrap();
        assert_eq!(game.box_owner(0), OWNER_NONE);
        for edge in 0..4 {
            game.play(edge).unwrap();
        }
        assert!(game.is_terminal());
        assert_eq!(game.box_owner(0), 1);
    }

    #[test]
    fn perfect_hud_size_rejects_4x4() {
        let game = WasmGame::new(4, 4).unwrap();
        assert!(!is_perfect_hud_size(game.rows(), game.cols()));
        assert!(!is_perfect_hud_size(5, 5));
        assert!(is_perfect_hud_size(2, 2));
        assert!(is_perfect_hud_size(3, 3));
    }

    #[test]
    fn analyze_dump_matches_core_and_does_not_play() {
        let game = WasmGame::new(1, 3).unwrap();
        let dump = game.analyze();
        assert_eq!(
            dump,
            dab_core::encode_analysis(&dab_core::Game::new(
                dab_core::BoardGeom::new(1, 3).unwrap()
            ))
        );
        assert_eq!(game.legal_moves().len(), game.edge_count() as usize);
        assert_eq!(game.current_player(), 0);
    }
}
