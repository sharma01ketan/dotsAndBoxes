//! Strings-and-coins endgame analysis: chains, loops, long-chain parity (KET-16).
//!
//! See `docs/specs/phase2-cgt-endgame-analysis.md`.

use crate::bitboard::BoxBits;
use crate::board::{BoardGeom, BoxId, EdgeId, Position};
use crate::game::{Game, Player};

/// Max regions = max boxes on a supported board.
const MAX_REGIONS: usize = crate::board::MAX_ROWS as usize * crate::board::MAX_COLS as usize;

/// Corridor / loop classification for a connected set of degree-2 coins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKind {
    ShortChain,
    LongChain,
    Loop,
}

/// One chain or loop (membership is the unclaimed corridor boxes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub kind: RegionKind,
    pub length: u8,
    pub boxes: BoxBits,
}

impl Region {
    const EMPTY: Self = Self {
        kind: RegionKind::ShortChain,
        length: 0,
        boxes: BoxBits::EMPTY,
    };
}

/// Structural analysis of unclaimed boxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndgameAnalysis {
    pub long_chain_count: u8,
    pub short_chain_count: u8,
    pub loop_count: u8,
    pub takeable_count: u8,
    /// `long_chain_count % 2` (0 or 1).
    pub long_chain_parity: u8,
    /// What the player to move wants `long_chain_parity` to be.
    pub target_parity: u8,
    /// True when no unclaimed box still has 3 or 4 remaining sides.
    pub decomposed: bool,
    region_count: u8,
    regions: [Region; MAX_REGIONS],
}

impl EndgameAnalysis {
    pub fn regions(&self) -> &[Region] {
        &self.regions[..self.region_count as usize]
    }

    #[inline]
    pub fn parity_ok(self) -> bool {
        self.long_chain_parity == self.target_parity
    }

    pub fn region_containing(&self, box_id: BoxId) -> Option<&Region> {
        self.regions().iter().find(|r| r.boxes.get(box_id))
    }
}

/// Corridor region sharing an undrawn edge with a takeable box, if any.
pub fn attached_region(
    pos: Position,
    analysis: &EndgameAnalysis,
    takeable: BoxId,
) -> Option<&Region> {
    let geom = pos.geom();
    let (row, col) = geom.box_coord(takeable)?;
    let edges = geom.box_edges(row, col)?;
    for e in edges {
        if pos.edge_is_drawn(e) {
            continue;
        }
        let mut touching = [0u16; 2];
        let n = geom.boxes_touching_edge(e, &mut touching);
        for &other in &touching[..n] {
            if other == takeable {
                continue;
            }
            if let Some(r) = analysis.region_containing(other) {
                return Some(r);
            }
        }
    }
    None
}

/// True when taking now would give up control (double-cross / all-but-four).
pub fn should_refuse_capture(game: &Game) -> bool {
    let analysis = analyze_endgame(game);
    refuse_remnant(game, &analysis).is_some()
}

/// Remnant that triggered refuse, if any. Same predicate as [`should_refuse_capture`].
pub fn refuse_skip_region(game: &Game) -> Option<Region> {
    let analysis = analyze_endgame(game);
    refuse_remnant(game, &analysis)
}

fn remnant_triggers_refuse(r: &Region, analysis: &EndgameAnalysis) -> bool {
    (r.kind == RegionKind::ShortChain
        && r.length == 2
        && analysis.long_chain_count + analysis.loop_count > 0)
        || (r.kind == RegionKind::Loop
            && r.length == 4
            && (analysis.long_chain_count > 0 || analysis.loop_count > 1))
}

/// Corridor attached to a takeable that should be double-crossed / all-but-four.
pub fn refuse_remnant(game: &Game, analysis: &EndgameAnalysis) -> Option<Region> {
    if !analysis.decomposed {
        return None;
    }
    let pos = game.position();
    let n = pos.geom().box_count();
    for id in 0..n {
        if pos.box_is_claimed(id) || pos.sides_drawn(id) != 3 {
            continue;
        }
        let Some(r) = attached_region(pos, analysis, id) else {
            continue;
        };
        if remnant_triggers_refuse(r, analysis) {
            return Some(*r);
        }
    }
    None
}

/// Unclaimed boxes with exactly three sides drawn.
pub fn takeable_box_ids(pos: Position) -> Vec<BoxId> {
    let n = pos.geom().box_count();
    let mut out = Vec::new();
    for id in 0..n {
        if !pos.box_is_claimed(id) && pos.sides_drawn(id) == 3 {
            out.push(id);
        }
    }
    out
}

pub const REGION_KIND_SHORT: u16 = 0;
pub const REGION_KIND_LONG: u16 = 1;
pub const REGION_KIND_LOOP: u16 = 2;

/// Flat `u16` dump for WASM / overlay. See `docs/specs/phase2-theory-overlay.md`.
pub fn encode_analysis(game: &Game) -> Vec<u16> {
    let analysis = analyze_endgame(game);
    let pos = game.position();
    let takeables = takeable_box_ids(pos);
    let mut out = vec![
        u16::from(analysis.decomposed),
        analysis.long_chain_count as u16,
        analysis.short_chain_count as u16,
        analysis.loop_count as u16,
        analysis.takeable_count as u16,
        analysis.long_chain_parity as u16,
        analysis.target_parity as u16,
        analysis.region_count as u16,
    ];
    for r in analysis.regions() {
        let kind = match r.kind {
            RegionKind::ShortChain => REGION_KIND_SHORT,
            RegionKind::LongChain => REGION_KIND_LONG,
            RegionKind::Loop => REGION_KIND_LOOP,
        };
        let boxes: Vec<BoxId> = (0..pos.geom().box_count())
            .filter(|&id| r.boxes.get(id))
            .collect();
        out.push(kind);
        out.push(r.length as u16);
        out.push(boxes.len() as u16);
        out.extend(boxes);
    }
    debug_assert_eq!(takeables.len(), analysis.takeable_count as usize);
    out.extend(takeables);
    out
}

/// Legal edges that open `region` (ground string on a chain; internal on a loop).
pub fn opening_edges(pos: Position, region: &Region) -> Vec<EdgeId> {
    use crate::bitboard::EdgeBits;
    let geom = pos.geom();
    let mut bits = EdgeBits::EMPTY;
    for id in 0..geom.box_count() {
        if !region.boxes.get(id) {
            continue;
        }
        let Some((row, col)) = geom.box_coord(id) else {
            continue;
        };
        let Some(edges) = geom.box_edges(row, col) else {
            continue;
        };
        for e in edges {
            if pos.edge_is_drawn(e) {
                continue;
            }
            let mut touching = [0u16; 2];
            let n = geom.boxes_touching_edge(e, &mut touching);
            let other_in_region = touching[..n]
                .iter()
                .any(|&b| b != id && region.boxes.get(b));
            let want = match region.kind {
                RegionKind::Loop => other_in_region,
                RegionKind::ShortChain | RegionKind::LongChain => !other_in_region,
            };
            if want {
                bits.set(e);
            }
        }
    }
    (0..geom.edge_count()).filter(|&e| bits.get(e)).collect()
}

/// Regions to open, best first.
///
/// `control`: long chains, then loops, then short chains (double-cross / keep control).
/// Otherwise dump the smallest gift: shortest region first (short chains before long).
pub fn ranked_open_targets<'a>(
    analysis: &'a EndgameAnalysis,
    skip: Option<&Region>,
    control: bool,
) -> Vec<&'a Region> {
    let mut v: Vec<&Region> = analysis
        .regions()
        .iter()
        .filter(|r| skip.map(|s| s.boxes != r.boxes).unwrap_or(true))
        .collect();
    v.sort_by_key(|r| {
        let class = match r.kind {
            RegionKind::LongChain => 0u8,
            RegionKind::Loop => 1,
            RegionKind::ShortChain => 2,
        };
        if control {
            (class, r.length)
        } else {
            (r.length, class)
        }
    });
    v
}

/// Analyze `game` without mutating it.
pub fn analyze_endgame(game: &Game) -> EndgameAnalysis {
    analyze_position(game.position(), game.current_player())
}

/// Same analysis given a packed position and who is to move.
pub fn analyze_position(pos: Position, to_move: Player) -> EndgameAnalysis {
    let geom = pos.geom();
    let n_boxes = geom.box_count();
    let mut takeable = 0u8;
    let mut open = 0u8;
    let mut corridor = BoxBits::EMPTY;

    for id in 0..n_boxes {
        if pos.box_is_claimed(id) {
            continue;
        }
        match 4u8.saturating_sub(pos.sides_drawn(id)) {
            1 => takeable += 1,
            2 => corridor.set(id),
            3 | 4 => open += 1,
            _ => {}
        }
    }

    let (adj, deg) = corridor_adj(pos, geom, n_boxes, corridor);
    let mut seen = BoxBits::EMPTY;
    let mut regions = [Region::EMPTY; MAX_REGIONS];
    let mut region_count = 0u8;
    let mut long_chain_count = 0u8;
    let mut short_chain_count = 0u8;
    let mut loop_count = 0u8;

    // Paths first: start at G-degree 0 or 1.
    for id in 0..n_boxes {
        if !corridor.get(id) || seen.get(id) {
            continue;
        }
        let d = deg[id as usize];
        if d > 1 {
            continue;
        }
        let region = walk_path(id, &adj, &deg, n_boxes, &mut seen);
        push_region(
            region,
            &mut regions,
            &mut region_count,
            &mut long_chain_count,
            &mut short_chain_count,
            &mut loop_count,
        );
    }

    // Remaining G-degree 2 components are loops.
    for id in 0..n_boxes {
        if !corridor.get(id) || seen.get(id) {
            continue;
        }
        let region = walk_loop(id, &adj, n_boxes, &mut seen);
        push_region(
            region,
            &mut regions,
            &mut region_count,
            &mut long_chain_count,
            &mut short_chain_count,
            &mut loop_count,
        );
    }

    let n = n_boxes;
    let p1_target = ((n + 1) % 2) as u8;
    let target_parity = match to_move {
        Player::P1 => p1_target,
        Player::P2 => 1 - p1_target,
    };
    let long_chain_parity = long_chain_count % 2;

    EndgameAnalysis {
        long_chain_count,
        short_chain_count,
        loop_count,
        takeable_count: takeable,
        long_chain_parity,
        target_parity,
        decomposed: open == 0,
        region_count,
        regions,
    }
}

fn corridor_adj(
    pos: Position,
    geom: BoardGeom,
    n_boxes: u16,
    corridor: BoxBits,
) -> ([[u16; 2]; MAX_REGIONS], [u8; MAX_REGIONS]) {
    let mut adj = [[u16::MAX; 2]; MAX_REGIONS];
    let mut deg = [0u8; MAX_REGIONS];
    for id in 0..n_boxes {
        if !corridor.get(id) {
            continue;
        }
        let Some((row, col)) = geom.box_coord(id) else {
            continue;
        };
        let Some(edges) = geom.box_edges(row, col) else {
            continue;
        };
        for e in edges {
            if pos.edge_is_drawn(e) {
                continue;
            }
            let mut touching = [0u16; 2];
            let n = geom.boxes_touching_edge(e, &mut touching);
            for &other in &touching[..n] {
                if other == id || !corridor.get(other) {
                    continue;
                }
                let d = deg[id as usize] as usize;
                if d < 2 {
                    adj[id as usize][d] = other;
                    deg[id as usize] += 1;
                }
            }
        }
    }
    (adj, deg)
}

fn walk_path(
    start: BoxId,
    adj: &[[u16; 2]; MAX_REGIONS],
    deg: &[u8; MAX_REGIONS],
    n_boxes: u16,
    seen: &mut BoxBits,
) -> Region {
    let mut boxes = BoxBits::EMPTY;
    let mut length = 0u8;
    let mut cur = start;
    let mut prev = u16::MAX;
    loop {
        if cur >= n_boxes || seen.get(cur) {
            break;
        }
        seen.set(cur);
        boxes.set(cur);
        length += 1;
        let nxt = next_unseen(cur, prev, adj, *seen);
        prev = cur;
        match nxt {
            Some(n) => cur = n,
            None => break,
        }
        if deg[cur as usize] > 2 {
            break;
        }
    }
    region_from_boxes(boxes, length, false)
}

fn walk_loop(
    start: BoxId,
    adj: &[[u16; 2]; MAX_REGIONS],
    n_boxes: u16,
    seen: &mut BoxBits,
) -> Region {
    let mut boxes = BoxBits::EMPTY;
    let mut length = 0u8;
    let mut cur = start;
    let mut prev = u16::MAX;
    loop {
        if cur >= n_boxes || seen.get(cur) {
            break;
        }
        seen.set(cur);
        boxes.set(cur);
        length += 1;
        let nxt = next_unseen(cur, prev, adj, *seen);
        prev = cur;
        match nxt {
            Some(n) => cur = n,
            None => break,
        }
    }
    region_from_boxes(boxes, length, true)
}

fn next_unseen(
    cur: BoxId,
    prev: BoxId,
    adj: &[[u16; 2]; MAX_REGIONS],
    seen: BoxBits,
) -> Option<BoxId> {
    adj[cur as usize]
        .into_iter()
        .find(|&n| n != u16::MAX && n != prev && !seen.get(n))
}

fn region_from_boxes(boxes: BoxBits, length: u8, is_loop: bool) -> Region {
    let kind = if is_loop {
        RegionKind::Loop
    } else if length >= 3 {
        RegionKind::LongChain
    } else {
        RegionKind::ShortChain
    };
    Region {
        kind,
        length,
        boxes,
    }
}

fn push_region(
    region: Region,
    regions: &mut [Region; MAX_REGIONS],
    region_count: &mut u8,
    long_chain_count: &mut u8,
    short_chain_count: &mut u8,
    loop_count: &mut u8,
) {
    if region.length == 0 || *region_count as usize >= MAX_REGIONS {
        return;
    }
    match region.kind {
        RegionKind::LongChain => *long_chain_count += 1,
        RegionKind::ShortChain => *short_chain_count += 1,
        RegionKind::Loop => *loop_count += 1,
    }
    regions[*region_count as usize] = region;
    *region_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardGeom, EdgeCoord, EdgeId, Orientation};
    use crate::game::Game;

    fn edge(geom: BoardGeom, o: Orientation, row: u8, col: u8) -> EdgeId {
        geom.edge_id(EdgeCoord {
            orientation: o,
            row,
            col,
        })
        .unwrap()
    }

    fn draw(game: &mut Game, edges: &[EdgeId]) {
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

    fn region_boxes(region: &Region, n: u16) -> Vec<BoxId> {
        (0..n).filter(|&id| region.boxes.get(id)).collect()
    }

    fn only_region(a: &EndgameAnalysis, kind: RegionKind, n: u16) -> Vec<BoxId> {
        assert_eq!(a.regions().len(), 1);
        let r = &a.regions()[0];
        assert_eq!(r.kind, kind);
        region_boxes(r, n)
    }

    #[test]
    fn analyze_does_not_mutate_game() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        game.play(0).unwrap();
        let before = game;
        let _ = analyze_endgame(&game);
        assert_eq!(game, before);
    }

    #[test]
    fn terminal_is_empty_and_decomposed() {
        let geom = BoardGeom::new(1, 1).unwrap();
        let mut game = Game::new(geom);
        for e in geom.box_edges(0, 0).unwrap() {
            game.play(e).unwrap();
        }
        assert!(game.is_terminal());
        let a = analyze_endgame(&game);
        assert!(a.decomposed);
        assert_eq!(a.long_chain_count, 0);
        assert_eq!(a.short_chain_count, 0);
        assert_eq!(a.loop_count, 0);
        assert_eq!(a.takeable_count, 0);
        assert!(a.regions().is_empty());
    }

    #[test]
    fn empty_2x2_is_not_decomposed() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        let a = analyze_endgame(&game);
        assert!(!a.decomposed);
        assert_eq!(a.long_chain_count, 0);
        assert_eq!(a.short_chain_count, 0);
        assert_eq!(a.loop_count, 0);
        assert_eq!(a.takeable_count, 0);
    }

    #[test]
    fn one_by_three_is_a_long_chain() {
        let geom = BoardGeom::new(1, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        let a = analyze_endgame(&game);
        assert!(a.decomposed);
        assert_eq!(a.long_chain_count, 1);
        assert_eq!(a.short_chain_count, 0);
        assert_eq!(a.loop_count, 0);
        assert_eq!(a.takeable_count, 0);
        let boxes = only_region(&a, RegionKind::LongChain, 3);
        assert_eq!(boxes, vec![0, 1, 2]);
        assert_eq!(a.regions()[0].length, 3);
    }

    #[test]
    fn encode_analysis_1x3_long_chain() {
        let geom = BoardGeom::new(1, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        let dump = encode_analysis(&game);
        assert_eq!(dump[0], 1); // decomposed
        assert_eq!(dump[1], 1); // L
        assert_eq!(dump[2], 0);
        assert_eq!(dump[3], 0);
        assert_eq!(dump[4], 0); // takeables
        assert_eq!(dump[7], 1); // one region
        assert_eq!(dump[8], REGION_KIND_LONG);
        assert_eq!(dump[9], 3);
        assert_eq!(dump[10], 3);
        assert_eq!(&dump[11..14], &[0, 1, 2]);
    }

    #[test]
    fn one_by_two_is_a_short_chain() {
        let geom = BoardGeom::new(1, 2).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        let a = analyze_endgame(&game);
        assert!(a.decomposed);
        assert_eq!(a.long_chain_count, 0);
        assert_eq!(a.short_chain_count, 1);
        let boxes = only_region(&a, RegionKind::ShortChain, 2);
        assert_eq!(boxes, vec![0, 1]);
        assert_eq!(a.regions()[0].length, 2);
    }

    #[test]
    fn one_by_one_degree_two_is_short_chain_len_1() {
        let geom = BoardGeom::new(1, 1).unwrap();
        let mut game = Game::new(geom);
        let sides = geom.box_edges(0, 0).unwrap();
        draw(&mut game, &[sides[0], sides[1]]); // top, bottom
        let a = analyze_endgame(&game);
        assert!(a.decomposed);
        assert_eq!(a.short_chain_count, 1);
        assert_eq!(a.long_chain_count, 0);
        let boxes = only_region(&a, RegionKind::ShortChain, 1);
        assert_eq!(boxes, vec![0]);
        assert_eq!(a.regions()[0].length, 1);
    }

    #[test]
    fn two_by_two_outer_drawn_is_a_loop() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        draw(
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
            ],
        );
        let a = analyze_endgame(&game);
        assert!(a.decomposed);
        assert_eq!(a.loop_count, 1);
        assert_eq!(a.long_chain_count, 0);
        assert_eq!(a.short_chain_count, 0);
        let boxes = only_region(&a, RegionKind::Loop, 4);
        assert_eq!(boxes, vec![0, 1, 2, 3]);
        assert_eq!(a.regions()[0].length, 4);
    }

    #[test]
    fn encode_analysis_2x2_loop() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut game = Game::new(geom);
        draw(
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
            ],
        );
        let dump = encode_analysis(&game);
        assert_eq!(dump[0], 1);
        assert_eq!(dump[3], 1); // loop_count
        assert_eq!(dump[7], 1);
        assert_eq!(dump[8], REGION_KIND_LOOP);
        assert_eq!(dump[9], 4);
        assert_eq!(dump[10], 4);
    }

    #[test]
    fn two_by_three_double_corridor_two_long_chains() {
        let geom = BoardGeom::new(2, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        let a = analyze_endgame(&game);
        assert!(a.decomposed);
        assert_eq!(a.long_chain_count, 2);
        assert_eq!(a.short_chain_count, 0);
        assert_eq!(a.loop_count, 0);
        assert_eq!(a.regions().len(), 2);
        for r in a.regions() {
            assert_eq!(r.kind, RegionKind::LongChain);
            assert_eq!(r.length, 3);
        }
        let mut all = Vec::new();
        for r in a.regions() {
            all.extend(region_boxes(r, 6));
        }
        all.sort();
        assert_eq!(all, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn long_chain_parity_p1_wants_even_on_odd_n() {
        let geom = BoardGeom::new(1, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        assert_eq!(game.current_player(), Player::P1);
        let a = analyze_endgame(&game);
        // N=3 odd → P1 target 0; L=1 → parity 1; not ok.
        assert_eq!(a.target_parity, 0);
        assert_eq!(a.long_chain_parity, 1);
        assert!(!a.parity_ok());

        let p2 = analyze_position(game.position(), Player::P2);
        assert_eq!(p2.target_parity, 1);
        assert!(p2.parity_ok());
    }

    #[test]
    fn p1_wants_odd_long_chains_when_n_even() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let game = Game::new(geom);
        let a = analyze_endgame(&game);
        assert_eq!(a.target_parity, 1);
        assert_eq!(a.long_chain_parity, 0);
        assert!(!a.parity_ok());
    }

    #[test]
    fn takeable_is_not_a_chain() {
        let geom = BoardGeom::new(1, 3).unwrap();
        let mut game = Game::new(geom);
        draw_all_horizontals(&mut game);
        game.play(edge(geom, Orientation::Vertical, 0, 0)).unwrap();
        let a = analyze_endgame(&game);
        assert_eq!(a.takeable_count, 1);
        assert_eq!(a.short_chain_count, 1);
        assert_eq!(a.long_chain_count, 0);
        let boxes = only_region(&a, RegionKind::ShortChain, 3);
        assert_eq!(boxes, vec![1, 2]);
    }
}
