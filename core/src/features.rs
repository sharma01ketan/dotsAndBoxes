//! AlphaZero feature tensor and canonical edge → policy-index map.
//!
//! Frozen in slice A (`docs/specs/phase4-in-wasm-az.md`). Training (`ai/`) and
//! inference (`wasm-az`) must use this encoding; bump [`AZ_FEATURES_VERSION`]
//! on any layout change.

use crate::board::{BoardGeom, EdgeCoord, EdgeId, Orientation};
use crate::game::{Game, Player};

/// HUD / net pad size (max boxes on a side). Smaller boards embed at the origin.
pub const AZ_HUD_ROWS: u8 = 5;
pub const AZ_HUD_COLS: u8 = 5;

pub const AZ_CHANNELS: usize = 7;
/// `2 * AZ_HUD_ROWS + 1` interleaved dots / edges / boxes.
pub const AZ_PLANE: usize = 11;
pub const AZ_FEATURES: usize = AZ_CHANNELS * AZ_PLANE * AZ_PLANE; // 847

/// Horizontal edges on a 5×5 pad: `(5 + 1) * 5`.
const AZ_H_POLICY: usize = 30;
/// Vertical stride on a 5×5 pad: `5 + 1`.
const AZ_V_STRIDE: usize = 6;
/// `edge_count(5, 5)`.
pub const AZ_POLICY: usize = 60;

/// Sidecar / ONNX stamp; bump when channels, plane, or the policy map change.
pub const AZ_FEATURES_VERSION: u32 = 1;

#[inline]
fn chw(channel: usize, y: usize, x: usize) -> usize {
    debug_assert!(channel < AZ_CHANNELS);
    debug_assert!(y < AZ_PLANE && x < AZ_PLANE);
    channel * AZ_PLANE * AZ_PLANE + y * AZ_PLANE + x
}

fn edge_cell(coord: EdgeCoord) -> (usize, usize) {
    match coord.orientation {
        Orientation::Horizontal => (2 * coord.row as usize, 2 * coord.col as usize + 1),
        Orientation::Vertical => (2 * coord.row as usize + 1, 2 * coord.col as usize),
    }
}

/// 7×11×11 CHW tensor from the side-to-move perspective.
///
/// `last_move` is threaded in because [`Game`] has no last-move accessor.
pub fn to_features(game: &Game, last_move: Option<EdgeId>) -> [f32; AZ_FEATURES] {
    let mut out = [0.0f32; AZ_FEATURES];
    let geom = game.geom();
    let pos = game.position();
    let rows = geom.rows();
    let cols = geom.cols();
    let side = game.current_player();

    if side == Player::P1 {
        let y_max = 2 * rows as usize;
        let x_max = 2 * cols as usize;
        for y in 0..=y_max {
            for x in 0..=x_max {
                out[chw(5, y, x)] = 1.0;
            }
        }
    }

    for id in 0..geom.edge_count() {
        let coord = geom.edge_coord(id).expect("id in range");
        let (y, x) = edge_cell(coord);
        if pos.edge_is_drawn(id) {
            out[chw(1, y, x)] = 1.0;
        } else {
            out[chw(0, y, x)] = 1.0;
        }
    }

    if let Some(edge) = last_move {
        if let Some(coord) = geom.edge_coord(edge) {
            let (y, x) = edge_cell(coord);
            out[chw(6, y, x)] = 1.0;
        }
    }

    for id in 0..geom.box_count() {
        let (r, c) = geom.box_coord(id).expect("id in range");
        let y = 2 * r as usize + 1;
        let x = 2 * c as usize + 1;
        match game.box_owner(id) {
            Some(p) if p == side => out[chw(2, y, x)] = 1.0,
            Some(_) => out[chw(3, y, x)] = 1.0,
            None => out[chw(4, y, x)] = 1.0,
        }
    }

    out
}

/// Board-size-invariant edge → policy index (5×5 strides).
///
/// `H(r, c) → r * 5 + c` (0..29); `V(r, c) → 30 + r * 6 + c` (30..59).
/// On a 5×5 board this is the identity on [`EdgeId`].
pub fn policy_index(geom: BoardGeom, edge: EdgeId) -> usize {
    debug_assert!(geom.rows() <= AZ_HUD_ROWS && geom.cols() <= AZ_HUD_COLS);
    let coord = geom.edge_coord(edge).expect("edge on geom");
    match coord.orientation {
        Orientation::Horizontal => {
            debug_assert!(coord.col < AZ_HUD_COLS);
            coord.row as usize * AZ_HUD_COLS as usize + coord.col as usize
        }
        Orientation::Vertical => {
            debug_assert!(coord.col <= AZ_HUD_COLS);
            AZ_H_POLICY + coord.row as usize * AZ_V_STRIDE + coord.col as usize
        }
    }
}

/// Inverse: canonical index → this board's [`EdgeId`], if that edge exists on-board.
pub fn edge_from_policy_index(geom: BoardGeom, idx: usize) -> Option<EdgeId> {
    if idx >= AZ_POLICY {
        return None;
    }
    let coord = if idx < AZ_H_POLICY {
        EdgeCoord {
            orientation: Orientation::Horizontal,
            row: (idx / AZ_HUD_COLS as usize) as u8,
            col: (idx % AZ_HUD_COLS as usize) as u8,
        }
    } else {
        let local = idx - AZ_H_POLICY;
        EdgeCoord {
            orientation: Orientation::Vertical,
            row: (local / AZ_V_STRIDE) as u8,
            col: (local % AZ_V_STRIDE) as u8,
        }
    };
    geom.edge_id(coord)
}

/// `true` at indices that are legal (on-board and undrawn) for `game`.
pub fn legal_policy_mask(game: &Game) -> [bool; AZ_POLICY] {
    let mut mask = [false; AZ_POLICY];
    let geom = game.geom();
    for edge in game.legal_moves() {
        mask[policy_index(geom, edge)] = true;
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Orientation;
    use crate::game::Game;

    fn geom(rows: u8, cols: u8) -> BoardGeom {
        BoardGeom::new(rows, cols).unwrap()
    }

    fn edge(g: BoardGeom, o: Orientation, row: u8, col: u8) -> EdgeId {
        g.edge_id(EdgeCoord {
            orientation: o,
            row,
            col,
        })
        .unwrap()
    }

    fn at(feat: &[f32; AZ_FEATURES], c: usize, y: usize, x: usize) -> f32 {
        feat[chw(c, y, x)]
    }

    fn channel_sum(feat: &[f32; AZ_FEATURES], c: usize) -> f32 {
        let start = c * AZ_PLANE * AZ_PLANE;
        feat[start..start + AZ_PLANE * AZ_PLANE].iter().sum()
    }

    #[test]
    fn contract_lengths() {
        assert_eq!(AZ_PLANE, 2 * AZ_HUD_ROWS as usize + 1);
        assert_eq!(AZ_FEATURES, 847);
        assert_eq!(AZ_POLICY, geom(5, 5).edge_count() as usize);
        assert_eq!(AZ_FEATURES_VERSION, 1);
    }

    #[test]
    fn empty_3x3_golden() {
        let game = Game::new(geom(3, 3));
        let feat = to_features(&game, None);
        assert_eq!(feat.len(), AZ_FEATURES);

        // 24 undrawn edges; nothing drawn; 9 unclaimed boxes; P1-to-move fills 7×7.
        assert_eq!(channel_sum(&feat, 0), 24.0);
        assert_eq!(channel_sum(&feat, 1), 0.0);
        assert_eq!(channel_sum(&feat, 2), 0.0);
        assert_eq!(channel_sum(&feat, 3), 0.0);
        assert_eq!(channel_sum(&feat, 4), 9.0);
        assert_eq!(channel_sum(&feat, 5), 49.0);
        assert_eq!(channel_sum(&feat, 6), 0.0);

        let h00 = edge(game.geom(), Orientation::Horizontal, 0, 0);
        let (y, x) = edge_cell(game.geom().edge_coord(h00).unwrap());
        assert_eq!((y, x), (0, 1));
        assert_eq!(at(&feat, 0, y, x), 1.0);
        assert_eq!(at(&feat, 1, y, x), 0.0);

        let box00 = game.geom().box_id(0, 0).unwrap();
        let (br, bc) = game.geom().box_coord(box00).unwrap();
        assert_eq!(at(&feat, 4, 2 * br as usize + 1, 2 * bc as usize + 1), 1.0);

        // Padding outside the 7×7 on-board square is zero, including ch5.
        assert_eq!(at(&feat, 5, 7, 0), 0.0);
        assert_eq!(at(&feat, 5, 0, 7), 0.0);
        assert_eq!(at(&feat, 5, 10, 10), 0.0);
    }

    #[test]
    fn one_h_edge_and_last_move_channel() {
        let g = geom(3, 3);
        let mut game = Game::new(g);
        let h00 = edge(g, Orientation::Horizontal, 0, 0);
        game.play(h00).unwrap();
        assert_eq!(game.current_player(), Player::P2);

        let feat = to_features(&game, Some(h00));
        let (y, x) = (0usize, 1usize);
        assert_eq!(at(&feat, 0, y, x), 0.0);
        assert_eq!(at(&feat, 1, y, x), 1.0);
        assert_eq!(at(&feat, 6, y, x), 1.0);
        assert_eq!(channel_sum(&feat, 0), 23.0);
        assert_eq!(channel_sum(&feat, 1), 1.0);
        assert_eq!(channel_sum(&feat, 6), 1.0);
        // P2 to move → ch5 is all zero.
        assert_eq!(channel_sum(&feat, 5), 0.0);

        let without = to_features(&game, None);
        assert_eq!(at(&without, 6, y, x), 0.0);
        assert_eq!(at(&without, 1, y, x), 1.0);
    }

    #[test]
    fn claimed_box_is_side_to_move_relative() {
        // 1×1: four edges; P2 takes the box and keeps the turn.
        let g = geom(1, 1);
        let mut game = Game::new(g);
        let sides = g.box_edges(0, 0).unwrap();
        for &e in &sides {
            game.play(e).unwrap();
        }
        assert_eq!(game.box_owner(0), Some(Player::P2));
        assert_eq!(game.current_player(), Player::P2);

        let feat = to_features(&game, Some(sides[3]));
        assert_eq!(at(&feat, 2, 1, 1), 1.0); // side to move owns it
        assert_eq!(at(&feat, 3, 1, 1), 0.0);
        assert_eq!(at(&feat, 4, 1, 1), 0.0);
        assert_eq!(channel_sum(&feat, 5), 0.0);
    }

    #[test]
    fn policy_index_worked_examples() {
        let g3 = geom(3, 3);
        assert_eq!(
            policy_index(g3, edge(g3, Orientation::Horizontal, 3, 2)),
            17
        );
        assert_eq!(policy_index(g3, edge(g3, Orientation::Vertical, 0, 3)), 33);

        let g5 = geom(5, 5);
        assert_eq!(policy_index(g5, edge(g5, Orientation::Horizontal, 0, 0)), 0);
        assert_eq!(
            policy_index(g5, edge(g5, Orientation::Horizontal, 5, 4)),
            29
        );
        assert_eq!(policy_index(g5, edge(g5, Orientation::Vertical, 0, 0)), 30);
        assert_eq!(policy_index(g5, edge(g5, Orientation::Vertical, 4, 5)), 59);
    }

    #[test]
    fn policy_index_is_identity_on_5x5() {
        let g = geom(5, 5);
        for id in 0..g.edge_count() {
            assert_eq!(policy_index(g, id), id as usize);
            assert_eq!(edge_from_policy_index(g, id as usize), Some(id));
        }
    }

    #[test]
    fn policy_index_round_trip_2x2_through_5x5() {
        for rows in 2u8..=5 {
            for cols in 2u8..=5 {
                let g = geom(rows, cols);
                let mut seen = [false; AZ_POLICY];
                for id in 0..g.edge_count() {
                    let idx = policy_index(g, id);
                    assert!(idx < AZ_POLICY);
                    assert!(!seen[idx], "collision at {idx} on {rows}×{cols}");
                    seen[idx] = true;
                    assert_eq!(edge_from_policy_index(g, idx), Some(id));
                }
            }
        }
    }

    #[test]
    fn edge_from_policy_index_none_off_board() {
        let g = geom(3, 3);
        // H(0, 4) exists on 5×5 only.
        assert_eq!(edge_from_policy_index(g, 4), None);
        // H(4, 0) is past 3×3's last H-row (0..=3).
        assert_eq!(edge_from_policy_index(g, 20), None);
        assert_eq!(edge_from_policy_index(g, AZ_POLICY), None);
        // V(0, 3) is on-board for 3×3.
        assert_eq!(
            edge_from_policy_index(g, 33),
            Some(edge(g, Orientation::Vertical, 0, 3))
        );
    }

    #[test]
    fn legal_mask_empty_3x3() {
        let game = Game::new(geom(3, 3));
        let mask = legal_policy_mask(&game);
        let n = mask.iter().filter(|&&b| b).count();
        assert_eq!(n, game.geom().edge_count() as usize);
        let mut game2 = game;
        let e = edge(game.geom(), Orientation::Horizontal, 0, 0);
        game2.play(e).unwrap();
        let mask2 = legal_policy_mask(&game2);
        assert!(!mask2[policy_index(game.geom(), e)]);
        assert_eq!(mask2.iter().filter(|&&b| b).count(), 23);
    }
}
