//! Move engines: Random, Greedy (KET-15), CGT heuristic (KET-17).
//!
//! See `docs/specs/phase2-random-greedy-engines.md` and
//! `docs/specs/phase2-cgt-heuristic.md`.

use crate::board::{EdgeId, Position};
use crate::cgt::{
    analyze_endgame, opening_edges, ranked_open_targets, refuse_skip_region, should_refuse_capture,
    Region, RegionKind,
};
use crate::game::Game;
use crate::rng::XorShift64;

/// Selects a legal edge for the side to move.
///
/// Callers must not invoke [`Engine::choose`] on a terminal game.
pub trait Engine {
    fn choose(&mut self, game: &Game) -> EdgeId;
}

/// Uniform random legal move.
#[derive(Clone, Debug)]
pub struct RandomEngine {
    rng: XorShift64,
}

impl RandomEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: XorShift64::new(seed),
        }
    }
}

impl Engine for RandomEngine {
    fn choose(&mut self, game: &Game) -> EdgeId {
        debug_assert!(!game.is_terminal());
        let cap = game.geom().edge_count() as usize;
        let mut legal = Vec::with_capacity(cap);
        legal.extend(game.legal_moves());
        debug_assert!(!legal.is_empty());
        pick(&mut self.rng, &legal)
    }
}

/// Take free boxes; otherwise avoid handing the opponent a 3-sided box.
#[derive(Clone, Debug)]
pub struct GreedyEngine {
    rng: XorShift64,
}

impl GreedyEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: XorShift64::new(seed),
        }
    }
}

impl Engine for GreedyEngine {
    fn choose(&mut self, game: &Game) -> EdgeId {
        debug_assert!(!game.is_terminal());
        let pos = game.position();
        let cap = game.geom().edge_count() as usize;
        let mut legal = Vec::with_capacity(cap);
        legal.extend(game.legal_moves());
        debug_assert!(!legal.is_empty());

        // 1) Maximize completed boxes among capturing moves.
        let mut best_completed = 0usize;
        let mut capturing = Vec::with_capacity(cap);
        for &edge in &legal {
            let n = completed_count(pos, edge);
            if n > best_completed {
                best_completed = n;
                capturing.clear();
                capturing.push(edge);
            } else if n > 0 && n == best_completed {
                capturing.push(edge);
            }
        }
        if best_completed > 0 {
            return pick(&mut self.rng, &capturing);
        }

        // 2) Prefer safe non-capturing moves (do not create a 3-sided box).
        let mut safe = Vec::with_capacity(cap);
        for &edge in &legal {
            if !leaves_three_sided(pos, edge) {
                safe.push(edge);
            }
        }
        if !safe.is_empty() {
            return pick(&mut self.rng, &safe);
        }

        // 3) Forced: every move hands over a box.
        pick(&mut self.rng, &legal)
    }
}

/// Long-chain control: double-cross (all-but-two) and all-but-four on loops.
#[derive(Clone, Debug)]
pub struct CgtEngine {
    rng: XorShift64,
}

impl CgtEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: XorShift64::new(seed),
        }
    }
}

impl Engine for CgtEngine {
    fn choose(&mut self, game: &Game) -> EdgeId {
        debug_assert!(!game.is_terminal());
        let pos = game.position();
        let cap = game.geom().edge_count() as usize;
        let mut legal = Vec::with_capacity(cap);
        legal.extend(game.legal_moves());
        debug_assert!(!legal.is_empty());

        let mut best_completed = 0usize;
        let mut capturing = Vec::with_capacity(cap);
        for &edge in &legal {
            let n = completed_count(pos, edge);
            if n > best_completed {
                best_completed = n;
                capturing.clear();
                capturing.push(edge);
            } else if n > 0 && n == best_completed {
                capturing.push(edge);
            }
        }

        if best_completed > 0 {
            if should_refuse_capture(game) {
                let skip = refuse_skip_region(game);
                if let Some(edge) = pick_opening(game, skip.as_ref(), true, &mut self.rng) {
                    return edge;
                }
            }
            return pick(&mut self.rng, &capturing);
        }

        let analysis = analyze_endgame(game);
        if analysis.decomposed {
            if let Some(edge) = pick_opening(game, None, false, &mut self.rng) {
                return edge;
            }
            return pick(&mut self.rng, &legal);
        }

        let mut safe = Vec::with_capacity(cap);
        for &edge in &legal {
            if !leaves_three_sided(pos, edge) {
                safe.push(edge);
            }
        }
        if !safe.is_empty() {
            return pick(&mut self.rng, &safe);
        }
        if let Some(edge) = pick_opening(game, None, false, &mut self.rng) {
            return edge;
        }
        pick(&mut self.rng, &legal)
    }
}

fn pick_opening(
    game: &Game,
    skip: Option<&Region>,
    control: bool,
    rng: &mut XorShift64,
) -> Option<EdgeId> {
    let analysis = analyze_endgame(game);
    let pos = game.position();
    let targets = ranked_open_targets(&analysis, skip, control);
    if targets.is_empty() {
        return None;
    }
    let best = targets[0];
    let best_class = match best.kind {
        RegionKind::LongChain => 0u8,
        RegionKind::Loop => 1,
        RegionKind::ShortChain => 2,
    };
    let tied: Vec<&Region> = targets
        .iter()
        .copied()
        .filter(|r| {
            let class = match r.kind {
                RegionKind::LongChain => 0u8,
                RegionKind::Loop => 1,
                RegionKind::ShortChain => 2,
            };
            r.length == best.length && class == best_class
        })
        .collect();
    let region = tied[rng.gen_index(tied.len())];
    let edges = opening_edges(pos, region);
    if edges.is_empty() {
        for region in targets {
            let edges = opening_edges(pos, region);
            if !edges.is_empty() {
                return Some(pick(rng, &edges));
            }
        }
        return None;
    }
    Some(pick(rng, &edges))
}

fn pick(rng: &mut XorShift64, items: &[EdgeId]) -> EdgeId {
    items[rng.gen_index(items.len())]
}

fn completed_count(pos: Position, edge: EdgeId) -> usize {
    let mut probe = pos;
    let undo = probe.apply_move(edge).expect("legal edge must apply");
    undo.completed().len()
}

/// True if drawing `edge` leaves an adjacent unclaimed box with exactly 3 sides.
fn leaves_three_sided(pos: Position, edge: EdgeId) -> bool {
    let mut probe = pos;
    probe.apply_move(edge).expect("legal edge must apply");

    let mut touching = [0u16; 2];
    let n = pos.geom().boxes_touching_edge(edge, &mut touching);
    for &box_id in &touching[..n] {
        if probe.box_is_claimed(box_id) {
            continue;
        }
        if probe.sides_drawn(box_id) == 3 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardGeom, EdgeCoord, Orientation};
    use crate::cgt::should_refuse_capture;
    use crate::game::{Player, Winner};

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

    #[test]
    fn random_returns_legal_and_is_deterministic() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        let mut a = RandomEngine::new(42);
        let mut b = RandomEngine::new(42);
        let e1 = a.choose(&game);
        let e2 = b.choose(&game);
        assert_eq!(e1, e2);
        assert!(game.is_legal(e1));
    }

    #[test]
    fn greedy_takes_completing_move_when_available() {
        let geom = BoardGeom::new(1, 1).unwrap();
        let mut game = Game::new(geom);
        let sides = geom.box_edges(0, 0).unwrap();
        play_all(&mut game, &sides[..3]);
        let last = sides[3];
        let mut greedy = GreedyEngine::new(1);
        assert_eq!(greedy.choose(&game), last);
    }

    #[test]
    fn greedy_prefers_double_claim_over_single() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        let a = geom.box_edges(0, 0).unwrap();
        let b = geom.box_edges(0, 1).unwrap();
        let shared = a[3];
        assert_eq!(shared, b[2]);
        play_all(&mut game, &[a[0], a[1], a[2], b[0], b[1], b[3]]);
        let c = geom.box_edges(1, 0).unwrap();
        play_all(&mut game, &[c[2], c[3]]);
        let single = c[1];
        assert_eq!(completed_count(game.position(), shared), 2);
        assert_eq!(completed_count(game.position(), single), 1);

        let mut greedy = GreedyEngine::new(7);
        assert_eq!(greedy.choose(&game), shared);
    }

    #[test]
    fn greedy_avoids_unsafe_when_safe_exists() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        let box00 = geom.box_edges(0, 0).unwrap();
        play_all(&mut game, &[box00[0], box00[1]]);
        let unsafe_edge = box00[2];
        assert!(leaves_three_sided(game.position(), unsafe_edge));

        let far = edge(geom, Orientation::Horizontal, 2, 1);
        assert!(!leaves_three_sided(game.position(), far));
        assert!(game.is_legal(far));
        assert!(game.is_legal(unsafe_edge));

        let mut greedy = GreedyEngine::new(99);
        for _ in 0..20 {
            let choice = greedy.choose(&game);
            assert!(
                !leaves_three_sided(game.position(), choice),
                "chose unsafe edge {choice}"
            );
            assert_ne!(choice, unsafe_edge);
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
    fn cgt_is_deterministic() {
        let geom = BoardGeom::new(3, 3).unwrap();
        let game = Game::new(geom);
        let mut a = CgtEngine::new(42);
        let mut b = CgtEngine::new(42);
        let e1 = a.choose(&game);
        let e2 = b.choose(&game);
        assert_eq!(e1, e2);
        assert!(game.is_legal(e1));
    }

    #[test]
    fn cgt_takes_free_box_in_midgame() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        let sides = geom.box_edges(0, 0).unwrap();
        play_all(&mut game, &sides[..3]);
        let last = sides[3];
        let mut cgt = CgtEngine::new(1);
        assert_eq!(cgt.choose(&game), last);
    }

    #[test]
    fn cgt_double_cross_leaves_last_two_when_another_chain_exists() {
        let geom = BoardGeom::new(2, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        game.play(edge(geom, Orientation::Vertical, 0, 0)).unwrap();
        let take = edge(geom, Orientation::Vertical, 0, 1);
        assert_eq!(completed_count(game.position(), take), 1);
        assert!(should_refuse_capture(&game));

        let mut cgt = CgtEngine::new(7);
        let choice = cgt.choose(&game);
        assert_ne!(choice, take);
        let open_a = edge(geom, Orientation::Vertical, 1, 0);
        let open_b = edge(geom, Orientation::Vertical, 1, 3);
        assert!(
            choice == open_a || choice == open_b,
            "expected ground open of the other chain, got {choice}"
        );
    }

    #[test]
    fn cgt_takes_last_two_when_no_other_control_region() {
        let geom = BoardGeom::new(1, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        game.play(edge(geom, Orientation::Vertical, 0, 0)).unwrap();
        let take = edge(geom, Orientation::Vertical, 0, 1);
        assert!(!should_refuse_capture(&game));
        let mut cgt = CgtEngine::new(3);
        assert_eq!(cgt.choose(&game), take);
    }

    #[test]
    fn cgt_all_but_four_refuses_loop_takeables() {
        let geom = BoardGeom::new(3, 3).unwrap();
        let mut game = Game::new(geom);
        play_all(
            &mut game,
            &[
                // 2×2 loop outers (top-left)
                edge(geom, Orientation::Horizontal, 0, 0),
                edge(geom, Orientation::Horizontal, 0, 1),
                edge(geom, Orientation::Horizontal, 2, 0),
                edge(geom, Orientation::Horizontal, 2, 1),
                edge(geom, Orientation::Vertical, 0, 0),
                edge(geom, Orientation::Vertical, 1, 0),
                edge(geom, Orientation::Vertical, 0, 2),
                edge(geom, Orientation::Vertical, 1, 2),
                // Close leftover col-2 so the position is decomposed (boxes 2 and 5
                // become a vertical short chain).
                edge(geom, Orientation::Horizontal, 0, 2),
                // bottom row chain of 3: remaining top H(2,2) + all bottoms
                edge(geom, Orientation::Horizontal, 2, 2),
                edge(geom, Orientation::Horizontal, 3, 0),
                edge(geom, Orientation::Horizontal, 3, 1),
                edge(geom, Orientation::Horizontal, 3, 2),
            ],
        );
        // Open the loop (one internal).
        game.play(edge(geom, Orientation::Vertical, 0, 1)).unwrap();
        assert!(should_refuse_capture(&game));
        let take_a = edge(geom, Orientation::Horizontal, 1, 0);
        let take_b = edge(geom, Orientation::Horizontal, 1, 1);
        let mut cgt = CgtEngine::new(11);
        let choice = cgt.choose(&game);
        assert_ne!(choice, take_a);
        assert_ne!(choice, take_b);
        let open_a = edge(geom, Orientation::Vertical, 2, 0);
        let open_b = edge(geom, Orientation::Vertical, 2, 3);
        assert!(
            choice == open_a || choice == open_b,
            "expected ground open of the bottom chain, got {choice}"
        );
    }

    fn play_match(
        rows: u8,
        cols: u8,
        greedy_is_p1: bool,
        greedy_seed: u64,
        random_seed: u64,
    ) -> Winner {
        let geom = BoardGeom::new(rows, cols).unwrap();
        let mut game = Game::new(geom);
        let mut greedy = GreedyEngine::new(greedy_seed);
        let mut random = RandomEngine::new(random_seed);
        while !game.is_terminal() {
            let edge = match (game.current_player(), greedy_is_p1) {
                (Player::P1, true) | (Player::P2, false) => greedy.choose(&game),
                _ => random.choose(&game),
            };
            game.play(edge).unwrap();
        }
        game.winner().unwrap()
    }

    fn arena_greedy_win_rate(rows: u8, cols: u8, games: u32) -> f64 {
        let mut greedy_wins = 0u32;
        let mut decisive = 0u32;
        for i in 0..games {
            let greedy_is_p1 = i % 2 == 0;
            let winner = play_match(rows, cols, greedy_is_p1, 1000 + i as u64, 5000 + i as u64);
            let greedy_won = matches!(
                (winner, greedy_is_p1),
                (Winner::Player(Player::P1), true) | (Winner::Player(Player::P2), false)
            );
            if matches!(winner, Winner::Draw) {
                continue;
            }
            decisive += 1;
            if greedy_won {
                greedy_wins += 1;
            }
        }
        assert!(decisive > 0, "no decisive games");
        greedy_wins as f64 / decisive as f64
    }

    #[test]
    fn arena_greedy_beats_random_2x2() {
        let rate = arena_greedy_win_rate(2, 2, 200);
        assert!(rate >= 0.65, "greedy win rate {rate:.3} below 65% on 2×2");
    }

    #[test]
    fn arena_greedy_beats_random_3x3() {
        let rate = arena_greedy_win_rate(3, 3, 200);
        assert!(rate >= 0.65, "greedy win rate {rate:.3} below 65% on 3×3");
    }

    fn play_cgt_vs_greedy(
        rows: u8,
        cols: u8,
        cgt_is_p1: bool,
        cgt_seed: u64,
        greedy_seed: u64,
    ) -> Winner {
        let geom = BoardGeom::new(rows, cols).unwrap();
        let mut game = Game::new(geom);
        let mut cgt = CgtEngine::new(cgt_seed);
        let mut greedy = GreedyEngine::new(greedy_seed);
        while !game.is_terminal() {
            let edge = match (game.current_player(), cgt_is_p1) {
                (Player::P1, true) | (Player::P2, false) => cgt.choose(&game),
                _ => greedy.choose(&game),
            };
            game.play(edge).unwrap();
        }
        game.winner().unwrap()
    }

    fn arena_cgt_win_rate(rows: u8, cols: u8, games: u32) -> f64 {
        let mut cgt_wins = 0u32;
        let mut decisive = 0u32;
        for i in 0..games {
            let cgt_is_p1 = i % 2 == 0;
            let winner =
                play_cgt_vs_greedy(rows, cols, cgt_is_p1, 2000 + i as u64, 8000 + i as u64);
            let cgt_won = matches!(
                (winner, cgt_is_p1),
                (Winner::Player(Player::P1), true) | (Winner::Player(Player::P2), false)
            );
            if matches!(winner, Winner::Draw) {
                continue;
            }
            decisive += 1;
            if cgt_won {
                cgt_wins += 1;
            }
        }
        assert!(decisive > 0, "no decisive games");
        cgt_wins as f64 / decisive as f64
    }

    #[test]
    fn arena_cgt_beats_greedy_3x3() {
        let rate = arena_cgt_win_rate(3, 3, 400);
        assert!(rate >= 0.60, "cgt win rate {rate:.3} below 60% on 3×3");
    }
}
