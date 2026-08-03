//! Thin WASM bindings over [`dab_core`].
//!
//! API is deliberately data-oriented: indices, typed arrays, and small numeric
//! codes — no rich JS objects — so the web app stays in control of UI state.

use dab_core::{BoardGeom, EdgeCoord, EdgeId, Game, MoveError, Orientation, Player, Winner};
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

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    // Keep panics visible in the browser console once console_error_panic_hook
    // is optional; for now rely on wasm-bindgen's default.
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

    /// `-1` unclaimed, `0` P1, `1` P2.
    #[wasm_bindgen(js_name = boxOwner)]
    pub fn box_owner(&self, box_id: u16) -> i8 {
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
}
