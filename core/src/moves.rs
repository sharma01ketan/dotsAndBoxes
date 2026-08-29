//! Legal move generation and make/unmake over [`Position`].

use crate::board::{BoardGeom, BoxId, EdgeCoord, EdgeId, Orientation, Position};

/// Maximum boxes an edge can complete in one move (shared by two boxes).
pub const MAX_COMPLETED_PER_MOVE: usize = 2;

/// Error applying a move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    /// Edge id is outside `0..edge_count`.
    OutOfRange,
    /// Edge is already drawn.
    AlreadyDrawn,
}

/// Boxes completed by a successful move (0–2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CompletedBoxes {
    boxes: [BoxId; MAX_COMPLETED_PER_MOVE],
    len: u8,
}

impl CompletedBoxes {
    #[inline]
    pub const fn len(self) -> usize {
        self.len as usize
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn as_slice(&self) -> &[BoxId] {
        &self.boxes[..self.len as usize]
    }

    #[inline]
    fn push(&mut self, id: BoxId) {
        debug_assert!((self.len as usize) < MAX_COMPLETED_PER_MOVE);
        self.boxes[self.len as usize] = id;
        self.len += 1;
    }
}

/// Token that reverses a previous [`Position::apply_move`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Undo {
    edge: EdgeId,
    completed: CompletedBoxes,
}

impl Undo {
    #[inline]
    pub const fn edge(self) -> EdgeId {
        self.edge
    }

    #[inline]
    pub const fn completed(self) -> CompletedBoxes {
        self.completed
    }
}

/// Iterator over undrawn edge ids.
#[derive(Clone, Debug)]
pub struct LegalMoves {
    pos: Position,
    next: EdgeId,
    end: EdgeId,
}

impl Iterator for LegalMoves {
    type Item = EdgeId;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.end {
            let id = self.next;
            self.next += 1;
            if !self.pos.edge_is_drawn(id) {
                return Some(id);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.end - self.next) as usize;
        (0, Some(remaining))
    }
}

impl BoardGeom {
    /// The four edges bounding box `(row, col)`, in order: top, bottom, left, right.
    pub const fn box_edges(self, row: u8, col: u8) -> Option<[EdgeId; 4]> {
        if row >= self.rows() || col >= self.cols() {
            return None;
        }
        let top = match self.edge_id(EdgeCoord {
            orientation: Orientation::Horizontal,
            row,
            col,
        }) {
            Some(e) => e,
            None => return None,
        };
        let bottom = match self.edge_id(EdgeCoord {
            orientation: Orientation::Horizontal,
            row: row + 1,
            col,
        }) {
            Some(e) => e,
            None => return None,
        };
        let left = match self.edge_id(EdgeCoord {
            orientation: Orientation::Vertical,
            row,
            col,
        }) {
            Some(e) => e,
            None => return None,
        };
        let right = match self.edge_id(EdgeCoord {
            orientation: Orientation::Vertical,
            row,
            col: col + 1,
        }) {
            Some(e) => e,
            None => return None,
        };
        Some([top, bottom, left, right])
    }

    /// Boxes adjacent to an edge (0–2), filled into `out`; returns count.
    pub fn boxes_touching_edge(self, edge: EdgeId, out: &mut [BoxId; 2]) -> usize {
        let Some(coord) = self.edge_coord(edge) else {
            return 0;
        };
        let mut n = 0;
        match coord.orientation {
            Orientation::Horizontal => {
                if coord.row > 0 {
                    if let Some(id) = self.box_id(coord.row - 1, coord.col) {
                        out[n] = id;
                        n += 1;
                    }
                }
                if coord.row < self.rows() {
                    if let Some(id) = self.box_id(coord.row, coord.col) {
                        out[n] = id;
                        n += 1;
                    }
                }
            }
            Orientation::Vertical => {
                if coord.col > 0 {
                    if let Some(id) = self.box_id(coord.row, coord.col - 1) {
                        out[n] = id;
                        n += 1;
                    }
                }
                if coord.col < self.cols() {
                    if let Some(id) = self.box_id(coord.row, coord.col) {
                        out[n] = id;
                        n += 1;
                    }
                }
            }
        }
        n
    }
}

impl Position {
    /// Whether `edge` is a legal move (in range and not yet drawn).
    #[inline]
    pub fn is_legal(self, edge: EdgeId) -> bool {
        edge < self.geom().edge_count() && !self.edge_is_drawn(edge)
    }

    /// Iterate undrawn edges.
    pub fn legal_moves(self) -> LegalMoves {
        LegalMoves {
            pos: self,
            next: 0,
            end: self.geom().edge_count(),
        }
    }

    /// Count of undrawn edges.
    pub fn legal_move_count(self) -> u16 {
        self.legal_moves().count() as u16
    }

    /// Draw `edge`, claim any newly completed boxes, and return an undo token.
    pub fn apply_move(&mut self, edge: EdgeId) -> Result<Undo, MoveError> {
        if edge >= self.geom().edge_count() {
            return Err(MoveError::OutOfRange);
        }
        if self.edge_is_drawn(edge) {
            return Err(MoveError::AlreadyDrawn);
        }

        self.set_edge_drawn(edge);

        let mut touching = [0u16; 2];
        let n = self.geom().boxes_touching_edge(edge, &mut touching);
        let mut completed = CompletedBoxes::default();

        for &box_id in touching[..n].iter() {
            if self.box_is_claimed(box_id) {
                continue;
            }
            if self.box_is_complete(box_id) {
                self.set_box_claimed(box_id);
                completed.push(box_id);
            }
        }

        Ok(Undo { edge, completed })
    }

    /// Reverse a previous [`Self::apply_move`].
    pub fn undo(&mut self, undo: Undo) {
        for &box_id in undo.completed.as_slice() {
            self.clear_box_claimed(box_id);
        }
        self.clear_edge_drawn(undo.edge);
    }

    /// True when all four sides of `box_id` are drawn.
    pub fn box_is_complete(self, box_id: BoxId) -> bool {
        let Some((row, col)) = self.geom().box_coord(box_id) else {
            return false;
        };
        let Some(edges) = self.geom().box_edges(row, col) else {
            return false;
        };
        edges.into_iter().all(|e| self.edge_is_drawn(e))
    }

    /// How many of the four sides of `box_id` are currently drawn (0–4).
    pub fn sides_drawn(self, box_id: BoxId) -> u8 {
        let Some((row, col)) = self.geom().box_coord(box_id) else {
            return 0;
        };
        let Some(edges) = self.geom().box_edges(row, col) else {
            return 0;
        };
        edges.into_iter().filter(|&e| self.edge_is_drawn(e)).count() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoardGeom;
    use crate::rng::XorShift64;

    #[test]
    fn legal_moves_starts_with_all_edges() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let pos = Position::new(geom);
        let moves: Vec<_> = pos.legal_moves().collect();
        assert_eq!(moves.len(), geom.edge_count() as usize);
        assert_eq!(moves[0], 0);
        assert_eq!(*moves.last().unwrap(), geom.edge_count() - 1);
    }

    #[test]
    fn apply_then_undo_restores_state() {
        let geom = BoardGeom::new(3, 3).unwrap();
        let mut pos = Position::new(geom);
        let before = pos;
        let edge = geom
            .edge_id(EdgeCoord {
                orientation: Orientation::Horizontal,
                row: 1,
                col: 1,
            })
            .unwrap();
        let undo = pos.apply_move(edge).unwrap();
        assert!(pos.edge_is_drawn(edge));
        assert_ne!(pos, before);
        pos.undo(undo);
        assert_eq!(pos, before);
    }

    #[test]
    fn completing_a_box_claims_it() {
        let geom = BoardGeom::new(1, 1).unwrap();
        let mut pos = Position::new(geom);
        let edges = geom.box_edges(0, 0).unwrap();
        for &e in &edges[..3] {
            let undo = pos.apply_move(e).unwrap();
            assert!(undo.completed().is_empty());
        }
        let undo = pos.apply_move(edges[3]).unwrap();
        assert_eq!(undo.completed().as_slice(), &[0]);
        assert!(pos.box_is_claimed(0));
        pos.undo(undo);
        assert!(!pos.box_is_claimed(0));
        assert!(!pos.edge_is_drawn(edges[3]));
    }

    #[test]
    fn reject_already_drawn_and_oob() {
        let geom = BoardGeom::new(2, 2).unwrap();
        let mut pos = Position::new(geom);
        pos.apply_move(0).unwrap();
        assert_eq!(pos.apply_move(0), Err(MoveError::AlreadyDrawn));
        assert_eq!(
            pos.apply_move(geom.edge_count()),
            Err(MoveError::OutOfRange)
        );
    }

    #[test]
    fn random_playouts_stay_legal_and_undo_clean() {
        let mut rng = XorShift64::new(0xDEAD_BEEF_CAFE_BABE);
        for rows in 1..=5u8 {
            for cols in 1..=5u8 {
                let geom = BoardGeom::new(rows, cols).unwrap();
                for _game in 0..20 {
                    let mut pos = Position::new(geom);
                    let mut stack: Vec<Undo> = Vec::new();
                    let start = pos;

                    while let Some(edge) = {
                        let legal: Vec<_> = pos.legal_moves().collect();
                        if legal.is_empty() {
                            None
                        } else {
                            Some(legal[rng.gen_index(legal.len())])
                        }
                    } {
                        assert!(pos.is_legal(edge));
                        let before = pos;
                        let undo = pos.apply_move(edge).unwrap();
                        // Drawn edge count increases by 1; completed boxes newly claimed.
                        assert!(pos.edge_is_drawn(edge));
                        assert_eq!(pos.edges().count_ones(), before.edges().count_ones() + 1);
                        for &b in undo.completed().as_slice() {
                            assert!(pos.box_is_claimed(b));
                            assert!(pos.box_is_complete(b));
                        }
                        // Claimed boxes ⊆ complete boxes; never claim twice.
                        assert!(pos.boxes().count_ones() <= geom.box_count() as u32);
                        stack.push(undo);
                    }

                    // Terminal: all edges drawn, all boxes claimed.
                    assert_eq!(pos.legal_move_count(), 0);
                    assert_eq!(pos.edges().count_ones(), geom.edge_count() as u32);
                    assert_eq!(pos.boxes().count_ones(), geom.box_count() as u32);

                    while let Some(undo) = stack.pop() {
                        pos.undo(undo);
                    }
                    assert_eq!(pos, start);
                }
            }
        }
    }
}
