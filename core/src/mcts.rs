//! UCT Monte Carlo Tree Search (KET-19).
//!
//! Extra-turn is whatever [`Game::play`] does: the child keeps the same
//! `current_player`, so UCT does not negate at those nodes.
//! See `docs/specs/phase2-mcts.md`.

use crate::board::EdgeId;
use crate::engine::{completed_count, Engine, GreedyEngine};
use crate::game::{Game, Player};
use crate::rng::XorShift64;

const DEFAULT_ITERS: u32 = 256;
const UCT_C: f64 = std::f64::consts::SQRT_2;

/// UCT search with greedy rollouts. Deterministic given seed + iteration budget.
#[derive(Clone, Debug)]
pub struct MctsEngine {
    rng: XorShift64,
    iterations: u32,
}

impl MctsEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: XorShift64::new(seed),
            iterations: DEFAULT_ITERS,
        }
    }

    pub fn with_iterations(mut self, n: u32) -> Self {
        self.iterations = n.max(1);
        self
    }
}

struct Node {
    game: Game,
    parent: Option<usize>,
    children: Vec<(EdgeId, usize)>,
    untried: Vec<EdgeId>,
    visits: u32,
    /// Sum of root-player score margins at terminals backed up through here.
    root_sum: f64,
}

impl Engine for MctsEngine {
    fn choose(&mut self, game: &Game) -> EdgeId {
        debug_assert!(!game.is_terminal());
        let cap = game.geom().edge_count() as usize;
        let mut legal = Vec::with_capacity(cap);
        legal.extend(game.legal_moves());
        debug_assert!(!legal.is_empty());
        if legal.len() == 1 {
            return legal[0];
        }
        // Greedy rollouts cannot represent double-cross, so a free box at the
        // root is taken (max completed). Extra-turn still appears in the tree
        // when search expands a capturing child deeper down.
        if let Some(edge) = best_capture(game, &legal, &mut self.rng) {
            return edge;
        }
        search(game, &legal, self.iterations, &mut self.rng)
    }
}

fn best_capture(game: &Game, legal: &[EdgeId], rng: &mut XorShift64) -> Option<EdgeId> {
    let pos = game.position();
    let mut best_completed = 0usize;
    let mut capturing = Vec::new();
    for &edge in legal {
        let n = completed_count(pos, edge);
        if n > best_completed {
            best_completed = n;
            capturing.clear();
            capturing.push(edge);
        } else if n > 0 && n == best_completed {
            capturing.push(edge);
        }
    }
    if best_completed == 0 {
        None
    } else {
        Some(capturing[rng.gen_index(capturing.len())])
    }
}

fn search(game: &Game, legal: &[EdgeId], iterations: u32, rng: &mut XorShift64) -> EdgeId {
    let root_player = game.current_player();
    let mut nodes = Vec::with_capacity(iterations as usize + 1);
    nodes.push(Node {
        game: *game,
        parent: None,
        children: Vec::new(),
        untried: legal.to_vec(),
        visits: 0,
        root_sum: 0.0,
    });

    for _ in 0..iterations {
        let mut idx = 0usize;
        while nodes[idx].untried.is_empty()
            && !nodes[idx].children.is_empty()
            && !nodes[idx].game.is_terminal()
        {
            idx = uct_child(&nodes, idx, root_player, rng);
        }

        if !nodes[idx].game.is_terminal() && !nodes[idx].untried.is_empty() {
            let pick = rng.gen_index(nodes[idx].untried.len());
            let edge = nodes[idx].untried.swap_remove(pick);
            let mut child_game = nodes[idx].game;
            child_game.play(edge).expect("untried edge must be legal");
            let untried = if child_game.is_terminal() {
                Vec::new()
            } else {
                child_game.legal_moves().collect()
            };
            let child_idx = nodes.len();
            nodes[idx].children.push((edge, child_idx));
            nodes.push(Node {
                game: child_game,
                parent: Some(idx),
                children: Vec::new(),
                untried,
                visits: 0,
                root_sum: 0.0,
            });
            idx = child_idx;
        }

        let mut rolled = nodes[idx].game;
        if !rolled.is_terminal() {
            rollout(&mut rolled, rng);
        }
        let value = root_margin(&rolled, root_player);
        backup(&mut nodes, idx, value);
    }

    most_visited_child(&nodes, rng).unwrap_or_else(|| legal[rng.gen_index(legal.len())])
}

fn uct_child(nodes: &[Node], parent: usize, root: Player, rng: &mut XorShift64) -> usize {
    let parent_visits = nodes[parent].visits.max(1);
    let ln = (parent_visits as f64).ln();
    let maximize = nodes[parent].game.current_player() == root;
    let children = &nodes[parent].children;
    debug_assert!(!children.is_empty());

    let mut best_i = 0usize;
    let mut best = f64::NEG_INFINITY;
    let mut ties = 0u32;
    for (i, &(_, child)) in children.iter().enumerate() {
        let c = &nodes[child];
        let n = c.visits.max(1) as f64;
        let mean = if c.visits == 0 {
            f64::INFINITY
        } else {
            let m = c.root_sum / n;
            if maximize {
                m
            } else {
                -m
            }
        };
        let score = mean + UCT_C * (ln / n).sqrt();
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

fn backup(nodes: &mut [Node], mut idx: usize, value: f64) {
    loop {
        nodes[idx].visits += 1;
        nodes[idx].root_sum += value;
        match nodes[idx].parent {
            Some(p) => idx = p,
            None => break,
        }
    }
}

fn root_margin(game: &Game, root: Player) -> f64 {
    game.score(root) as f64 - game.score(root.other()) as f64
}

fn rollout(game: &mut Game, rng: &mut XorShift64) {
    let mut greedy = GreedyEngine::new(rng.next_u64());
    while !game.is_terminal() {
        let edge = greedy.choose(game);
        game.play(edge).expect("greedy choose is legal");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoardGeom;
    use crate::engine::GreedyEngine;
    use crate::game::{Player, Winner};

    fn play_all(game: &mut Game, edges: &[EdgeId]) {
        for &e in edges {
            game.play(e).unwrap();
        }
    }

    #[test]
    fn capturing_child_keeps_to_move_and_mcts_takes_it() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        let sides = geom.box_edges(0, 0).unwrap();
        play_all(&mut game, &sides[..3]);
        let last = sides[3];
        let parent = game.current_player();
        let mut child = game;
        let (result, _) = child.play(last).unwrap();
        assert!(result.extra_turn);
        assert_eq!(child.current_player(), parent);

        let mut mcts = MctsEngine::new(1).with_iterations(8);
        assert_eq!(mcts.choose(&game), last);
        assert!(game.is_legal(last));
        assert_eq!(game.current_player(), parent);
    }

    #[test]
    fn same_seed_same_choice() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        let mut a = MctsEngine::new(42).with_iterations(48);
        let mut b = MctsEngine::new(42).with_iterations(48);
        let e1 = a.choose(&game);
        let e2 = b.choose(&game);
        assert_eq!(e1, e2);
        assert!(game.is_legal(e1));
    }

    #[test]
    fn choose_does_not_play() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        let before = game.legal_moves().count();
        let mut mcts = MctsEngine::new(7).with_iterations(16);
        let edge = mcts.choose(&game);
        assert!(game.is_legal(edge));
        assert_eq!(game.legal_moves().count(), before);
        assert_eq!(game.current_player(), Player::P1);
    }

    fn play_mcts_vs_greedy(
        rows: u8,
        cols: u8,
        mcts_is_p1: bool,
        mcts_seed: u64,
        greedy_seed: u64,
        iters: u32,
    ) -> Winner {
        let geom = BoardGeom::new(rows, cols).unwrap();
        let mut game = Game::new(geom);
        let mut mcts = MctsEngine::new(mcts_seed).with_iterations(iters);
        let mut greedy = GreedyEngine::new(greedy_seed);
        while !game.is_terminal() {
            let edge = match (game.current_player(), mcts_is_p1) {
                (Player::P1, true) | (Player::P2, false) => mcts.choose(&game),
                _ => greedy.choose(&game),
            };
            game.play(edge).unwrap();
        }
        game.winner().unwrap()
    }

    fn play_mcts_vs_random(
        rows: u8,
        cols: u8,
        mcts_is_p1: bool,
        mcts_seed: u64,
        random_seed: u64,
        iters: u32,
    ) -> Winner {
        let geom = BoardGeom::new(rows, cols).unwrap();
        let mut game = Game::new(geom);
        let mut mcts = MctsEngine::new(mcts_seed).with_iterations(iters);
        let mut random = crate::engine::RandomEngine::new(random_seed);
        while !game.is_terminal() {
            let edge = match (game.current_player(), mcts_is_p1) {
                (Player::P1, true) | (Player::P2, false) => mcts.choose(&game),
                _ => random.choose(&game),
            };
            game.play(edge).unwrap();
        }
        game.winner().unwrap()
    }

    fn arena_rate(games: u32, play: impl Fn(bool, u64, u64) -> Winner) -> f64 {
        let mut wins = 0u32;
        let mut decisive = 0u32;
        for i in 0..games {
            let first = i % 2 == 0;
            let winner = play(first, 3000 + i as u64, 9000 + i as u64);
            let won = matches!(
                (winner, first),
                (Winner::Player(Player::P1), true) | (Winner::Player(Player::P2), false)
            );
            if matches!(winner, Winner::Draw) {
                continue;
            }
            decisive += 1;
            if won {
                wins += 1;
            }
        }
        assert!(decisive > 0, "no decisive games");
        wins as f64 / decisive as f64
    }

    #[test]
    fn arena_mcts_beats_random_2x2() {
        let rate = arena_rate(40, |mcts_is_p1, a, b| {
            play_mcts_vs_random(2, 2, mcts_is_p1, a, b, 32)
        });
        assert!(
            rate >= 0.65,
            "mcts vs random win rate {rate:.3} below 65% on 2×2"
        );
    }

    #[test]
    fn arena_mcts_beats_greedy_2x2() {
        let rate = arena_rate(40, |mcts_is_p1, a, b| {
            play_mcts_vs_greedy(2, 2, mcts_is_p1, a, b, 128)
        });
        assert!(
            rate >= 0.60,
            "mcts vs greedy win rate {rate:.3} below 60% on 2×2"
        );
    }

    fn play_mcts_vs_cgt(
        rows: u8,
        cols: u8,
        mcts_is_p1: bool,
        mcts_seed: u64,
        cgt_seed: u64,
        iters: u32,
    ) -> Winner {
        let geom = BoardGeom::new(rows, cols).unwrap();
        let mut game = Game::new(geom);
        let mut mcts = MctsEngine::new(mcts_seed).with_iterations(iters);
        let mut cgt = crate::engine::CgtEngine::new(cgt_seed);
        while !game.is_terminal() {
            let edge = match (game.current_player(), mcts_is_p1) {
                (Player::P1, true) | (Player::P2, false) => mcts.choose(&game),
                _ => cgt.choose(&game),
            };
            game.play(edge).unwrap();
        }
        game.winner().unwrap()
    }

    #[test]
    fn arena_mcts_competitive_with_cgt_2x2() {
        let games = 24u32;
        let mut mcts_wins = 0u32;
        let mut decisive = 0u32;
        for i in 0..games {
            let mcts_is_p1 = i % 2 == 0;
            let winner = play_mcts_vs_cgt(2, 2, mcts_is_p1, 4000 + i as u64, 11000 + i as u64, 64);
            let mcts_won = matches!(
                (winner, mcts_is_p1),
                (Winner::Player(Player::P1), true) | (Winner::Player(Player::P2), false)
            );
            if matches!(winner, Winner::Draw) {
                continue;
            }
            decisive += 1;
            if mcts_won {
                mcts_wins += 1;
            }
        }
        if decisive == 0 {
            return;
        }
        let rate = mcts_wins as f64 / decisive as f64;
        assert!(
            rate >= 0.30,
            "mcts vs cgt win rate {rate:.3} below 30% on 2×2 ({mcts_wins}/{decisive} decisive)"
        );
    }
}
