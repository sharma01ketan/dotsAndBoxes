//! AlphaZero PUCT search.
//!
//! [`Evaluate`] is net-agnostic; `wasm-az` supplies tract. No root capture grab
//! (the net may decline a completing move). Endgame handoff is slice C.
//! See `docs/specs/phase4-in-wasm-az.md`.

use crate::board::EdgeId;
use crate::engine::Engine;
use crate::features::{legal_policy_mask, policy_index, to_features, AZ_POLICY};
use crate::game::{Game, Winner};
use crate::rng::XorShift64;

const DEFAULT_SIMS: u32 = 32;
const DEFAULT_C_PUCT: f64 = 1.5;

/// Net-agnostic evaluator. `dab-core` has **no** tract dependency.
pub trait Evaluate {
    /// Policy logits of length [`AZ_POLICY`] and a value in `[-1, 1]` for the
    /// **side to move**, given a [`to_features`] tensor.
    fn evaluate(&self, features: &[f32]) -> (Vec<f32>, f32);
}

/// PUCT over an [`Evaluate`]. Deterministic given seed + sim count.
pub struct AzEngine<'e, E: Evaluate> {
    eval: &'e E,
    rng: XorShift64,
    sims: u32,
    c_puct: f64,
    last_move: Option<EdgeId>,
}

impl<'e, E: Evaluate> AzEngine<'e, E> {
    pub fn new(eval: &'e E, seed: u64) -> Self {
        Self {
            eval,
            rng: XorShift64::new(seed),
            sims: DEFAULT_SIMS,
            c_puct: DEFAULT_C_PUCT,
            last_move: None,
        }
    }

    pub fn with_sims(mut self, sims: u32) -> Self {
        self.sims = sims.max(1);
        self
    }

    /// Last edge played, for the root feature tensor only. Children use the
    /// edge that reached them.
    pub fn with_last_move(mut self, last_move: Option<EdgeId>) -> Self {
        self.last_move = last_move;
        self
    }

    /// Cheap hint path: net policy argmax over legal moves, no tree.
    /// Ties break toward the smaller [`EdgeId`].
    pub fn policy_argmax(&self, game: &Game) -> EdgeId {
        debug_assert!(!game.is_terminal());
        let feat = to_features(game, self.last_move);
        let (logits, _) = self.eval.evaluate(&feat);
        argmax_legal(game, &logits)
    }
}

impl<'e, E: Evaluate> Engine for AzEngine<'e, E> {
    fn choose(&mut self, game: &Game) -> EdgeId {
        debug_assert!(!game.is_terminal());
        let mut legal = Vec::with_capacity(game.geom().edge_count() as usize);
        legal.extend(game.legal_moves());
        debug_assert!(!legal.is_empty());
        if legal.len() == 1 {
            return legal[0];
        }
        search(
            game,
            self.eval,
            self.last_move,
            self.sims,
            self.c_puct,
            &mut self.rng,
        )
    }
}

struct Node {
    game: Game,
    last_move: Option<EdgeId>,
    parent: Option<usize>,
    /// `(edge, child_index)` in legal-move order at expand time.
    children: Vec<(EdgeId, usize)>,
    prior: f32,
    visits: u32,
    /// Sum of backed-up values from **this node's** side-to-move.
    value_sum: f64,
}

fn search<E: Evaluate>(
    game: &Game,
    eval: &E,
    last_move: Option<EdgeId>,
    sims: u32,
    c_puct: f64,
    rng: &mut XorShift64,
) -> EdgeId {
    let mut nodes = Vec::with_capacity(sims as usize * 16 + 1);
    nodes.push(Node {
        game: *game,
        last_move,
        parent: None,
        children: Vec::new(),
        prior: 1.0,
        visits: 0,
        value_sum: 0.0,
    });

    for _ in 0..sims {
        let mut idx = 0usize;
        while !nodes[idx].children.is_empty() && !nodes[idx].game.is_terminal() {
            idx = puct_child(&nodes, idx, c_puct, rng);
        }

        let value = if nodes[idx].game.is_terminal() {
            terminal_value(&nodes[idx].game)
        } else {
            expand(eval, &mut nodes, idx)
        };
        backup(&mut nodes, idx, value);
    }

    most_visited_child(&nodes, rng).expect("root expanded")
}

/// Full expand: one net eval, all legal children with priors, return leaf value.
fn expand<E: Evaluate>(eval: &E, nodes: &mut Vec<Node>, idx: usize) -> f64 {
    let game = nodes[idx].game;
    debug_assert!(!game.is_terminal());
    let feat = to_features(&game, nodes[idx].last_move);
    let (logits, value) = eval.evaluate(&feat);
    let mask = legal_policy_mask(&game);
    let priors = masked_softmax(&logits, &mask);
    let geom = game.geom();

    for edge in game.legal_moves() {
        let mut child_game = game;
        child_game.play(edge).expect("legal");
        let child_idx = nodes.len();
        nodes[idx].children.push((edge, child_idx));
        nodes.push(Node {
            game: child_game,
            last_move: Some(edge),
            parent: Some(idx),
            children: Vec::new(),
            prior: priors[policy_index(geom, edge)],
            visits: 0,
            value_sum: 0.0,
        });
    }
    (value as f64).clamp(-1.0, 1.0)
}

fn puct_child(nodes: &[Node], parent: usize, c_puct: f64, rng: &mut XorShift64) -> usize {
    let children = &nodes[parent].children;
    debug_assert!(!children.is_empty());
    // Standard PUCT uses √N(parent). After the expand visit, that is 1 before
    // any child is visited, so the first selection follows the prior (the spec
    // formula √Σ_b N(b) is 0 at that moment and would ignore P).
    let sqrt_n = (nodes[parent].visits.max(1) as f64).sqrt();
    let parent_player = nodes[parent].game.current_player();

    let mut best_i = 0usize;
    let mut best = f64::NEG_INFINITY;
    let mut ties = 0u32;
    for (i, &(_, ci)) in children.iter().enumerate() {
        let child = &nodes[ci];
        let q = if child.visits == 0 {
            0.0
        } else {
            let mean = child.value_sum / child.visits as f64;
            if child.game.current_player() == parent_player {
                mean
            } else {
                -mean
            }
        };
        let score = q + c_puct * child.prior as f64 * sqrt_n / (1.0 + child.visits as f64);
        if score > best {
            best = score;
            best_i = i;
            ties = 1;
        } else if score == best {
            ties += 1;
            if rng.gen_index(ties as usize) == 0 {
                best_i = i;
            }
        }
    }
    children[best_i].1
}

fn most_visited_child(nodes: &[Node], rng: &mut XorShift64) -> Option<EdgeId> {
    let children = &nodes[0].children;
    if children.is_empty() {
        return None;
    }
    let mut best_edge = children[0].0;
    let mut best_visits = 0u32;
    let mut ties = 0u32;
    for &(edge, child) in children {
        let v = nodes[child].visits;
        if v > best_visits {
            best_visits = v;
            best_edge = edge;
            ties = 1;
        } else if v == best_visits {
            ties += 1;
            if rng.gen_index(ties as usize) == 0 {
                best_edge = edge;
            }
        }
    }
    Some(best_edge)
}

fn backup(nodes: &mut [Node], mut idx: usize, mut value: f64) {
    loop {
        nodes[idx].visits += 1;
        nodes[idx].value_sum += value;
        let Some(parent) = nodes[idx].parent else {
            break;
        };
        let same = nodes[idx].game.current_player() == nodes[parent].game.current_player();
        if !same {
            value = -value;
        }
        idx = parent;
    }
}

fn terminal_value(game: &Game) -> f64 {
    debug_assert!(game.is_terminal());
    let side = game.current_player();
    match game.winner() {
        Some(Winner::Player(p)) if p == side => 1.0,
        Some(Winner::Player(_)) => -1.0,
        Some(Winner::Draw) | None => 0.0,
    }
}

fn masked_softmax(logits: &[f32], mask: &[bool; AZ_POLICY]) -> [f32; AZ_POLICY] {
    let mut max = f64::NEG_INFINITY;
    for (i, &ok) in mask.iter().enumerate() {
        if ok {
            let v = logits.get(i).copied().unwrap_or(0.0) as f64;
            if v > max {
                max = v;
            }
        }
    }
    let mut exps = [0.0f64; AZ_POLICY];
    let mut sum = 0.0f64;
    for (i, &ok) in mask.iter().enumerate() {
        if ok {
            let v = (logits.get(i).copied().unwrap_or(0.0) as f64 - max).exp();
            exps[i] = v;
            sum += v;
        }
    }
    let mut out = [0.0f32; AZ_POLICY];
    if sum > 0.0 {
        for (i, &ok) in mask.iter().enumerate() {
            if ok {
                out[i] = (exps[i] / sum) as f32;
            }
        }
    } else {
        let n = mask.iter().filter(|&&b| b).count().max(1) as f32;
        for (i, &ok) in mask.iter().enumerate() {
            if ok {
                out[i] = 1.0 / n;
            }
        }
    }
    out
}

fn argmax_legal(game: &Game, logits: &[f32]) -> EdgeId {
    let geom = game.geom();
    let mut best_edge: Option<EdgeId> = None;
    let mut best_logit = f32::NEG_INFINITY;
    for edge in game.legal_moves() {
        let logit = logits
            .get(policy_index(geom, edge))
            .copied()
            .unwrap_or(f32::NEG_INFINITY);
        let take = match best_edge {
            None => true,
            Some(prev) => logit > best_logit || (logit == best_logit && edge < prev),
        };
        if take {
            best_logit = logit;
            best_edge = Some(edge);
        }
    }
    best_edge.expect("non-terminal games have a legal move")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardGeom, EdgeCoord, Orientation};
    use crate::engine::completed_count;
    use crate::features::{AZ_FEATURES, AZ_POLICY};
    use crate::game::Player;

    struct ConstEval {
        logits: Vec<f32>,
        value: f32,
    }

    impl Evaluate for ConstEval {
        fn evaluate(&self, features: &[f32]) -> (Vec<f32>, f32) {
            assert_eq!(features.len(), AZ_FEATURES);
            (self.logits.clone(), self.value)
        }
    }

    /// Extreme prior on `favor`; everything else very negative.
    struct FavorEval {
        favor: EdgeId,
        geom: BoardGeom,
        value: f32,
    }

    impl Evaluate for FavorEval {
        fn evaluate(&self, features: &[f32]) -> (Vec<f32>, f32) {
            assert_eq!(features.len(), AZ_FEATURES);
            let mut logits = vec![-20.0; AZ_POLICY];
            logits[policy_index(self.geom, self.favor)] = 20.0;
            (logits, self.value)
        }
    }

    fn geom(rows: u8, cols: u8) -> BoardGeom {
        BoardGeom::new(rows, cols).unwrap()
    }

    fn play_all(game: &mut Game, edges: &[EdgeId]) {
        for &e in edges {
            game.play(e).unwrap();
        }
    }

    fn uniform() -> ConstEval {
        ConstEval {
            logits: vec![0.0; AZ_POLICY],
            value: 0.0,
        }
    }

    #[test]
    fn dummy_eval_round_trips_features() {
        let game = Game::new(geom(3, 3));
        let feat = to_features(&game, None);
        let eval = ConstEval {
            logits: vec![0.0; AZ_POLICY],
            value: 0.25,
        };
        let (policy, value) = eval.evaluate(&feat);
        assert_eq!(policy.len(), AZ_POLICY);
        assert_eq!(value, 0.25);
    }

    #[test]
    fn choose_does_not_play_and_is_legal() {
        let game = Game::new(geom(2, 2));
        let before = game.legal_moves().count();
        let eval = uniform();
        let mut az = AzEngine::new(&eval, 7).with_sims(16);
        let edge = az.choose(&game);
        assert!(game.is_legal(edge));
        assert!(!game.position().edge_is_drawn(edge));
        assert_eq!(game.legal_moves().count(), before);
        assert_eq!(game.current_player(), Player::P1);
    }

    #[test]
    fn same_seed_same_choice() {
        let game = Game::new(geom(2, 2));
        let eval = uniform();
        let mut a = AzEngine::new(&eval, 42).with_sims(24);
        let mut b = AzEngine::new(&eval, 42).with_sims(24);
        assert_eq!(a.choose(&game), b.choose(&game));
    }

    #[test]
    fn policy_argmax_picks_highest_legal_logit() {
        let g = geom(2, 2);
        let game = Game::new(g);
        let want = g
            .edge_id(EdgeCoord {
                orientation: Orientation::Vertical,
                row: 0,
                col: 0,
            })
            .unwrap();
        let eval = FavorEval {
            favor: want,
            geom: g,
            value: 0.0,
        };
        let az = AzEngine::new(&eval, 1);
        assert_eq!(az.policy_argmax(&game), want);
        assert!(game.is_legal(want));
    }

    #[test]
    fn capturing_child_keeps_to_move() {
        let g = geom(2, 2);
        let mut game = Game::new(g);
        let sides = g.box_edges(0, 0).unwrap();
        play_all(&mut game, &sides[..3]);
        let last = sides[3];
        let parent = game.current_player();
        let mut child = game;
        let (result, _) = child.play(last).unwrap();
        assert!(result.extra_turn);
        assert_eq!(child.current_player(), parent);
    }

    #[test]
    fn az_can_decline_a_completing_move_at_root() {
        let g = geom(2, 2);
        let mut game = Game::new(g);
        let sides = g.box_edges(0, 0).unwrap();
        play_all(&mut game, &sides[..3]);
        let capture = sides[3];
        assert!(completed_count(game.position(), capture) > 0);

        let decline = game
            .legal_moves()
            .find(|&e| e != capture && completed_count(game.position(), e) == 0)
            .expect("a non-capturing legal edge");

        let eval = FavorEval {
            favor: decline,
            geom: g,
            value: 0.0,
        };
        let mut az = AzEngine::new(&eval, 3).with_sims(32);
        let chosen = az.choose(&game);
        assert_eq!(chosen, decline);
        assert_ne!(chosen, capture);
        assert!(game.is_legal(chosen));
        assert!(!game.position().edge_is_drawn(capture));
    }

    #[test]
    fn one_legal_move_returns_it() {
        let g = geom(1, 1);
        let mut game = Game::new(g);
        let sides = g.box_edges(0, 0).unwrap();
        play_all(&mut game, &sides[..3]);
        assert_eq!(game.legal_moves().count(), 1);
        let eval = uniform();
        let mut az = AzEngine::new(&eval, 9);
        assert_eq!(az.choose(&game), sides[3]);
    }
}
