//! Exact box-margin solver (KET-18).
//!
//! Searches the real [`Game`] graph to a terminal. CGT/Greedy helpers order
//! moves only. No loony closed-form cutoff (not proven equal on all 2×2/3×3).
//!
//! See `docs/specs/phase2-exact-solver.md`.

use std::collections::HashMap;

use crate::board::{BoardGeom, EdgeCoord, EdgeId, Orientation};
use crate::engine::{completed_count, leaves_three_sided, Engine};
use crate::game::{Game, Player};
use crate::rng::XorShift64;

type Score = i16;
const INF: Score = 128;
const TT_EXACT: u8 = 0;
const TT_LOWER: u8 = 1;
const TT_UPPER: u8 = 2;

/// Selects an optimal edge (ties via [`XorShift64`]).
#[derive(Debug)]
pub struct PerfectEngine {
    rng: XorShift64,
    search: Option<Search>,
}

impl PerfectEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: XorShift64::new(seed),
            search: None,
        }
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.rng = XorShift64::new(seed);
    }

    /// Box-difference margin for the player to move (reuses the TT).
    pub fn value(&mut self, game: &Game) -> i8 {
        let current = score_diff(game);
        if game.is_terminal() {
            return current;
        }
        let mut work = *game;
        let geom = work.geom();
        let remaining = self.search_mut(geom).negamax(&mut work, -INF, INF);
        current.saturating_add(remaining as i8)
    }

    fn search_mut(&mut self, geom: BoardGeom) -> &mut Search {
        let reset = match &self.search {
            None => true,
            Some(s) => s.geom != geom,
        };
        if reset {
            self.search = Some(Search::new(geom));
        }
        self.search.as_mut().unwrap()
    }
}

impl Engine for PerfectEngine {
    fn choose(&mut self, game: &Game) -> EdgeId {
        debug_assert!(!game.is_terminal());
        let mut work = *game;
        let geom = work.geom();
        let mut legal: Vec<EdgeId> = work.legal_moves().collect();
        order_moves(work, &mut legal);

        let mut best_val = -INF;
        let mut best: Vec<EdgeId> = Vec::new();
        for &edge in &legal {
            let (result, undo) = work.play(edge).expect("legal");
            let scored = result.completed.len() as Score;
            let child = self.search_mut(geom).negamax(&mut work, -INF, INF);
            work.undo(undo);
            let v = if result.extra_turn {
                scored + child
            } else {
                scored - child
            };
            if v > best_val {
                best_val = v;
                best.clear();
                best.push(edge);
            } else if v == best_val {
                best.push(edge);
            }
        }
        debug_assert!(!best.is_empty());
        best[self.rng.gen_index(best.len())]
    }
}

/// Box-difference margin for the player to move.
///
/// At a terminal: `score(to_move) − score(other)`. Otherwise that quantity
/// plus the exact remaining-play margin.
pub fn perfect_value(game: &Game) -> i8 {
    PerfectEngine::new(0).value(game)
}

pub fn is_perfect_hud_size(rows: u8, cols: u8) -> bool {
    rows == cols && (rows == 2 || rows == 3)
}

fn score_diff(game: &Game) -> i8 {
    let us = game.score(game.current_player()) as i8;
    let them = game.score(game.current_player().other()) as i8;
    us - them
}

#[derive(Debug)]
struct Search {
    geom: BoardGeom,
    zobrist_edge: Vec<u64>,
    zobrist_p2: u64,
    maps: Vec<Vec<EdgeId>>,
    tt: HashMap<u64, (Score, u8)>,
}

impl Search {
    fn new(geom: BoardGeom) -> Self {
        let n = geom.edge_count() as usize;
        let mut rng = XorShift64::new(0xC0FF_EE54_1B18);
        let mut zobrist_edge = Vec::with_capacity(n);
        for _ in 0..n {
            zobrist_edge.push(rng.next_u64());
        }
        let zobrist_p2 = rng.next_u64();
        Self {
            geom,
            maps: symmetry_maps(geom),
            zobrist_edge,
            zobrist_p2,
            tt: HashMap::new(),
        }
    }

    fn key(&self, game: &Game) -> u64 {
        let geom = game.geom();
        let edges = game.position().edges();
        let mut best = u64::MAX;
        for map in &self.maps {
            let mut h = 0u64;
            for id in 0..geom.edge_count() {
                if edges.get(id) {
                    let img = map[id as usize];
                    h ^= self.zobrist_edge[img as usize];
                }
            }
            if h < best {
                best = h;
            }
        }
        if game.current_player() == Player::P2 {
            best ^= self.zobrist_p2;
        }
        best
    }

    /// Remaining-play margin for the side to move (ignores current scores).
    fn negamax(&mut self, game: &mut Game, mut alpha: Score, mut beta: Score) -> Score {
        if game.is_terminal() {
            return 0;
        }

        let alpha_orig = alpha;
        let key = self.key(game);
        if let Some(&(val, flag)) = self.tt.get(&key) {
            match flag {
                TT_EXACT => return val,
                TT_LOWER if val > alpha => {
                    alpha = val;
                }
                TT_UPPER if val < beta => {
                    beta = val;
                }
                _ => {}
            }
            if alpha >= beta {
                return val;
            }
        }

        let mut legal: Vec<EdgeId> = game.legal_moves().collect();
        order_moves(*game, &mut legal);

        let mut best = -INF;
        for &edge in &legal {
            let (result, undo) = game.play(edge).expect("legal");
            let scored = result.completed.len() as Score;
            let child = if result.extra_turn {
                self.negamax(game, alpha - scored, beta - scored)
            } else {
                -self.negamax(game, -beta + scored, -alpha + scored)
            };
            game.undo(undo);
            let v = scored + child;
            if v > best {
                best = v;
            }
            if v > alpha {
                alpha = v;
            }
            if alpha >= beta {
                break;
            }
        }
        let flag = if best <= alpha_orig {
            TT_UPPER
        } else if best >= beta {
            TT_LOWER
        } else {
            TT_EXACT
        };
        self.tt.insert(key, (best, flag));
        best
    }
}

fn order_moves(game: Game, legal: &mut [EdgeId]) {
    let pos = game.position();
    legal.sort_by_key(|&edge| {
        let n = completed_count(pos, edge);
        if n > 0 {
            (0u8, -(n as i8))
        } else if !leaves_three_sided(pos, edge) {
            (1, 0)
        } else {
            (2, 0)
        }
    });
}

type CoordMap = fn(u8, u8, u8, u8) -> (u8, u8);

fn symmetry_maps(geom: BoardGeom) -> Vec<Vec<EdgeId>> {
    let n = geom.edge_count() as usize;
    let kinds: &[CoordMap] = if geom.rows() == geom.cols() {
        &[
            d4_id,
            d4_rot90,
            d4_rot180,
            d4_rot270,
            d4_flip_h,
            d4_flip_v,
            d4_transpose,
            d4_anti,
        ]
    } else {
        &[d2_id, d2_flip_h, d2_flip_v, d2_rot180]
    };
    kinds
        .iter()
        .map(|f| {
            let mut map = vec![0u16; n];
            for id in 0..geom.edge_count() {
                map[id as usize] = map_edge(geom, id, *f);
            }
            map
        })
        .collect()
}

fn map_edge(geom: BoardGeom, id: EdgeId, f: fn(u8, u8, u8, u8) -> (u8, u8)) -> EdgeId {
    let coord = geom.edge_coord(id).expect("id");
    let rows = geom.rows();
    let cols = geom.cols();
    let (r1, c1, r2, c2) = match coord.orientation {
        Orientation::Horizontal => (coord.row, coord.col, coord.row, coord.col + 1),
        Orientation::Vertical => (coord.row, coord.col, coord.row + 1, coord.col),
    };
    let (a_r, a_c) = f(rows, cols, r1, c1);
    let (b_r, b_c) = f(rows, cols, r2, c2);
    edge_from_dots(geom, a_r, a_c, b_r, b_c)
}

fn edge_from_dots(geom: BoardGeom, r1: u8, c1: u8, r2: u8, c2: u8) -> EdgeId {
    if r1 == r2 {
        let col = c1.min(c2);
        geom.edge_id(EdgeCoord {
            orientation: Orientation::Horizontal,
            row: r1,
            col,
        })
        .expect("H")
    } else {
        debug_assert_eq!(c1, c2);
        let row = r1.min(r2);
        geom.edge_id(EdgeCoord {
            orientation: Orientation::Vertical,
            row,
            col: c1,
        })
        .expect("V")
    }
}

fn d4_id(_rows: u8, _cols: u8, r: u8, c: u8) -> (u8, u8) {
    (r, c)
}

fn d4_rot90(rows: u8, _cols: u8, r: u8, c: u8) -> (u8, u8) {
    let n = rows;
    (c, n - r)
}

fn d4_rot180(rows: u8, cols: u8, r: u8, c: u8) -> (u8, u8) {
    (rows - r, cols - c)
}

fn d4_rot270(rows: u8, _cols: u8, r: u8, c: u8) -> (u8, u8) {
    let n = rows;
    (n - c, r)
}

fn d4_flip_h(_rows: u8, cols: u8, r: u8, c: u8) -> (u8, u8) {
    (r, cols - c)
}

fn d4_flip_v(rows: u8, _cols: u8, r: u8, c: u8) -> (u8, u8) {
    (rows - r, c)
}

fn d4_transpose(_rows: u8, _cols: u8, r: u8, c: u8) -> (u8, u8) {
    (c, r)
}

fn d4_anti(rows: u8, cols: u8, r: u8, c: u8) -> (u8, u8) {
    (cols - c, rows - r)
}

fn d2_id(rows: u8, cols: u8, r: u8, c: u8) -> (u8, u8) {
    d4_id(rows, cols, r, c)
}

fn d2_flip_h(rows: u8, cols: u8, r: u8, c: u8) -> (u8, u8) {
    d4_flip_h(rows, cols, r, c)
}

fn d2_flip_v(rows: u8, cols: u8, r: u8, c: u8) -> (u8, u8) {
    d4_flip_v(rows, cols, r, c)
}

fn d2_rot180(rows: u8, cols: u8, r: u8, c: u8) -> (u8, u8) {
    d4_rot180(rows, cols, r, c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardGeom, EdgeCoord, Orientation};
    use crate::engine::CgtEngine;
    #[cfg(not(debug_assertions))]
    use crate::engine::GreedyEngine;
    use crate::game::Player;
    #[cfg(not(debug_assertions))]
    use crate::game::Winner;

    fn edge(geom: BoardGeom, o: Orientation, row: u8, col: u8) -> EdgeId {
        geom.edge_id(EdgeCoord {
            orientation: o,
            row,
            col,
        })
        .unwrap()
    }

    fn play_all(game: &mut Game, edges: &[EdgeId]) {
        for &e in edges {
            game.play(e).unwrap();
        }
    }

    fn draw_all_horizontals(game: &mut Game) {
        let geom = game.geom();
        for row in 0..=geom.rows() {
            for col in 0..geom.cols() {
                game.play(edge(geom, Orientation::Horizontal, row, col))
                    .unwrap();
            }
        }
    }

    #[test]
    fn d4_rot90_four_times_is_identity() {
        let geom = BoardGeom::new(3, 3).unwrap();
        for id in 0..geom.edge_count() {
            let mut x = id;
            for _ in 0..4 {
                x = map_edge(geom, x, d4_rot90);
            }
            assert_eq!(x, id, "rot90^4 failed at {id}");
        }
    }

    #[test]
    fn d2_flips_are_involutions() {
        let geom = BoardGeom::new(2, 3).unwrap();
        for id in 0..geom.edge_count() {
            let h = map_edge(geom, id, d2_flip_h);
            assert_eq!(map_edge(geom, h, d2_flip_h), id);
            let v = map_edge(geom, id, d2_flip_v);
            assert_eq!(map_edge(geom, v, d2_flip_v), id);
        }
    }

    #[test]
    fn hud_size_is_square_two_or_three() {
        assert!(is_perfect_hud_size(2, 2));
        assert!(is_perfect_hud_size(3, 3));
        assert!(!is_perfect_hud_size(1, 1));
        assert!(!is_perfect_hud_size(4, 4));
        assert!(!is_perfect_hud_size(2, 3));
    }

    #[test]
    fn empty_1x1_is_minus_one() {
        let geom = BoardGeom::new(1, 1).unwrap();
        let game = Game::new(geom);
        assert_eq!(perfect_value(&game), -1);
    }

    #[test]
    fn terminal_margin_is_score_diff() {
        let geom = BoardGeom::new(1, 1).unwrap();
        let mut game = Game::new(geom);
        let sides = geom.box_edges(0, 0).unwrap();
        play_all(&mut game, &sides);
        assert!(game.is_terminal());
        assert_eq!(game.current_player(), Player::P2);
        assert_eq!(game.score(Player::P2), 1);
        assert_eq!(perfect_value(&game), 1);
    }

    #[test]
    fn empty_2x2_is_first_player_plus_two() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        assert_eq!(perfect_value(&game), 2);
    }

    #[test]
    fn remaining_matches_naive_on_near_terminal_2x2() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        let mut drawn = 0u8;
        for id in 0..geom.edge_count() {
            if drawn >= 8 {
                break;
            }
            if game.is_legal(id) {
                game.play(id).unwrap();
                drawn += 1;
            }
        }
        let mut naive_game = game;
        let naive = naive_remaining(&mut naive_game);
        let fast = perfect_value(&game) - super::score_diff(&game);
        assert_eq!(fast, naive, "search {fast} vs naive {naive}");
    }

    fn naive_remaining(game: &mut Game) -> i8 {
        if game.is_terminal() {
            return 0;
        }
        let legal: Vec<EdgeId> = game.legal_moves().collect();
        let mut best = i8::MIN;
        for &edge in &legal {
            let (result, undo) = game.play(edge).unwrap();
            let scored = result.completed.len() as i8;
            let child = if result.extra_turn {
                naive_remaining(game)
            } else {
                -naive_remaining(game)
            };
            game.undo(undo);
            let v = scored + child;
            if v > best {
                best = v;
            }
        }
        best
    }

    #[test]
    fn same_seed_same_choice() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        let mut a = PerfectEngine::new(42);
        let mut b = PerfectEngine::new(42);
        assert_eq!(a.choose(&game), b.choose(&game));
        assert_eq!(a.value(&game), perfect_value(&game));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn empty_3x3_is_second_player_win_by_three() {
        // Barker & Korf: 3×3 boxes is a second-player win by three.
        // Full search is too slow in debug; freeze in release.
        let geom = BoardGeom::new(3, 3).unwrap();
        let game = Game::new(geom);
        assert_eq!(perfect_value(&game), -3);
    }

    #[test]
    fn symmetry_preserves_value_after_one_edge() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let e0 = edge(geom, Orientation::Horizontal, 0, 0);
        let mut a = Game::new(geom);
        a.play(e0).unwrap();
        let v = perfect_value(&a);
        for f in [
            d4_rot90,
            d4_rot180,
            d4_rot270,
            d4_flip_h,
            d4_flip_v,
            d4_transpose,
            d4_anti,
        ] {
            let mut b = Game::new(geom);
            b.play(map_edge(geom, e0, f)).unwrap();
            assert_eq!(b.current_player(), a.current_player());
            assert_eq!(perfect_value(&b), v);
        }
    }

    #[test]
    fn perfect_on_cgt_double_cross_fixture_is_legal() {
        let geom = BoardGeom::new(2, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        game.play(edge(geom, Orientation::Vertical, 0, 0)).unwrap();
        let take = edge(geom, Orientation::Vertical, 0, 1);
        let mut cgt = CgtEngine::new(7);
        let cgt_choice = cgt.choose(&game);
        let mut perfect = PerfectEngine::new(7);
        let choice = perfect.choose(&game);
        assert_eq!(choice, take, "Perfect takes; CGT refuses");
        assert_ne!(cgt_choice, take);
        assert_eq!(perfect_value(&game), 2);
    }

    #[test]
    fn perfect_on_cgt_all_but_four_fixture_is_legal() {
        let geom = BoardGeom::new(3, 3).unwrap();
        let mut game = Game::new(geom);
        play_all(
            &mut game,
            &[
                edge(geom, Orientation::Horizontal, 0, 0),
                edge(geom, Orientation::Horizontal, 0, 1),
                edge(geom, Orientation::Horizontal, 2, 0),
                edge(geom, Orientation::Horizontal, 2, 1),
                edge(geom, Orientation::Vertical, 0, 0),
                edge(geom, Orientation::Vertical, 1, 0),
                edge(geom, Orientation::Vertical, 0, 2),
                edge(geom, Orientation::Vertical, 1, 2),
                edge(geom, Orientation::Horizontal, 0, 2),
                edge(geom, Orientation::Horizontal, 2, 2),
                edge(geom, Orientation::Horizontal, 3, 0),
                edge(geom, Orientation::Horizontal, 3, 1),
                edge(geom, Orientation::Horizontal, 3, 2),
            ],
        );
        game.play(edge(geom, Orientation::Vertical, 0, 1)).unwrap();
        let take_a = edge(geom, Orientation::Horizontal, 1, 0);
        let take_b = edge(geom, Orientation::Horizontal, 1, 1);
        let mut perfect = PerfectEngine::new(11);
        let choice = perfect.choose(&game);
        assert!(
            choice == take_a || choice == take_b,
            "Perfect takes the loop; CGT refuses, got {choice}"
        );
        assert_eq!(perfect_value(&game), 5);
    }

    #[cfg(not(debug_assertions))]
    fn play_perfect_vs(
        perfect: &mut PerfectEngine,
        rows: u8,
        cols: u8,
        perfect_is_p1: bool,
        perfect_seed: u64,
        other_seed: u64,
        greedy: bool,
    ) -> Winner {
        let geom = BoardGeom::new(rows, cols).unwrap();
        let mut game = Game::new(geom);
        perfect.set_seed(perfect_seed);
        let mut greedy_eng = GreedyEngine::new(other_seed);
        let mut cgt_eng = CgtEngine::new(other_seed);
        while !game.is_terminal() {
            let is_perfect = matches!(
                (game.current_player(), perfect_is_p1),
                (Player::P1, true) | (Player::P2, false)
            );
            let edge = if is_perfect {
                perfect.choose(&game)
            } else if greedy {
                greedy_eng.choose(&game)
            } else {
                cgt_eng.choose(&game)
            };
            game.play(edge).unwrap();
        }
        game.winner().unwrap()
    }

    #[cfg(not(debug_assertions))]
    fn arena_perfect_win_rate(greedy: bool, games: u32) -> (f64, u32, u32) {
        let mut perfect = PerfectEngine::new(1);
        let mut perfect_wins = 0u32;
        let mut p2_perfect_losses = 0u32;
        let mut decisive = 0u32;
        for i in 0..games {
            let perfect_is_p1 = i % 2 == 0;
            let winner = play_perfect_vs(
                &mut perfect,
                3,
                3,
                perfect_is_p1,
                3000 + i as u64,
                9000 + i as u64,
                greedy,
            );
            if !perfect_is_p1 && matches!(winner, Winner::Player(Player::P1)) {
                p2_perfect_losses += 1;
            }
            let perfect_won = matches!(
                (winner, perfect_is_p1),
                (Winner::Player(Player::P1), true) | (Winner::Player(Player::P2), false)
            );
            if matches!(winner, Winner::Draw) {
                continue;
            }
            decisive += 1;
            if perfect_won {
                perfect_wins += 1;
            }
        }
        assert!(decisive > 0, "no decisive games");
        (
            perfect_wins as f64 / decisive as f64,
            p2_perfect_losses,
            decisive,
        )
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn arena_perfect_vs_greedy_3x3() {
        let (rate, p2_losses, decisive) = arena_perfect_win_rate(true, 20);
        eprintln!(
            "perfect vs greedy: win rate {rate:.3} ({decisive} decisive), P2 losses {p2_losses}"
        );
        assert_eq!(p2_losses, 0, "Perfect as P2 lost a 3×3 (forced P2 win)");
        assert!(rate >= 0.50, "perfect vs greedy win rate {rate:.3}");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn arena_perfect_vs_cgt_3x3() {
        let (rate, p2_losses, decisive) = arena_perfect_win_rate(false, 20);
        eprintln!(
            "perfect vs cgt: win rate {rate:.3} ({decisive} decisive), P2 losses {p2_losses}"
        );
        assert_eq!(p2_losses, 0, "Perfect as P2 lost a 3×3 (forced P2 win)");
        assert!(rate >= 0.50, "perfect vs cgt win rate {rate:.3}");
    }
}
