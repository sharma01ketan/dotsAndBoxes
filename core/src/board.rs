//! Board geometry, canonical edge indexing, and packed position state.
//!
//! # Edge indexing
//!
//! For an `R × C` box grid:
//! - Horizontal edges come first: `(R + 1)` rows × `C` edges (row-major).
//! - Vertical edges follow: `R` rows × `(C + 1)` edges (row-major).
//!
//! Index of horizontal edge at `(row, col)`: `row * C + col`  
//! Index of vertical edge at `(row, col)`: `H + row * (C + 1) + col`  
//! where `H = (R + 1) * C`.

use crate::bitboard::{BoxBits, EdgeBits};

/// Maximum supported box rows (inclusive upper bound is this value).
pub const MAX_ROWS: u8 = 8;
/// Maximum supported box columns.
pub const MAX_COLS: u8 = 8;

/// Edge orientation on the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Grid coordinate of an edge (not yet resolved to a dense index).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EdgeCoord {
    pub orientation: Orientation,
    /// For horizontal: `0..=rows`. For vertical: `0..rows`.
    pub row: u8,
    /// For horizontal: `0..cols`. For vertical: `0..=cols`.
    pub col: u8,
}

/// Dense edge id in `0..geom.edge_count()`.
pub type EdgeId = u16;

/// Dense box id in `0..geom.box_count()` (row-major).
pub type BoxId = u16;

/// Immutable geometry for a Dots and Boxes board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoardGeom {
    rows: u8,
    cols: u8,
}

impl BoardGeom {
    /// Create geometry for an `rows × cols` box grid.
    ///
    /// Returns `None` if dimensions are zero or exceed [`MAX_ROWS`]/[`MAX_COLS`],
    /// or if the edge count would not fit in the edge bitboard.
    pub const fn new(rows: u8, cols: u8) -> Option<Self> {
        if rows == 0 || cols == 0 || rows > MAX_ROWS || cols > MAX_COLS {
            return None;
        }
        let geom = Self { rows, cols };
        if geom.edge_count() as usize > EdgeBits::capacity_bits() {
            return None;
        }
        if geom.box_count() as usize > BoxBits::capacity_bits() {
            return None;
        }
        Some(geom)
    }

    #[inline]
    pub const fn rows(self) -> u8 {
        self.rows
    }

    #[inline]
    pub const fn cols(self) -> u8 {
        self.cols
    }

    /// Number of boxes: `rows * cols`.
    #[inline]
    pub const fn box_count(self) -> u16 {
        self.rows as u16 * self.cols as u16
    }

    /// Number of horizontal edges: `(rows + 1) * cols`.
    #[inline]
    pub const fn horizontal_count(self) -> u16 {
        (self.rows as u16 + 1) * self.cols as u16
    }

    /// Number of vertical edges: `rows * (cols + 1)`.
    #[inline]
    pub const fn vertical_count(self) -> u16 {
        self.rows as u16 * (self.cols as u16 + 1)
    }

    /// Total edges (horizontal then vertical).
    #[inline]
    pub const fn edge_count(self) -> u16 {
        self.horizontal_count() + self.vertical_count()
    }

    /// Convert an edge coordinate to a dense [`EdgeId`].
    pub const fn edge_id(self, coord: EdgeCoord) -> Option<EdgeId> {
        match coord.orientation {
            Orientation::Horizontal => {
                if coord.row > self.rows || coord.col >= self.cols {
                    return None;
                }
                Some(coord.row as u16 * self.cols as u16 + coord.col as u16)
            }
            Orientation::Vertical => {
                if coord.row >= self.rows || coord.col > self.cols {
                    return None;
                }
                let h = self.horizontal_count();
                Some(h + coord.row as u16 * (self.cols as u16 + 1) + coord.col as u16)
            }
        }
    }

    /// Convert a dense [`EdgeId`] back to an [`EdgeCoord`].
    pub const fn edge_coord(self, id: EdgeId) -> Option<EdgeCoord> {
        let h = self.horizontal_count();
        if id < h {
            let cols = self.cols as u16;
            Some(EdgeCoord {
                orientation: Orientation::Horizontal,
                row: (id / cols) as u8,
                col: (id % cols) as u8,
            })
        } else if id < self.edge_count() {
            let local = id - h;
            let stride = self.cols as u16 + 1;
            Some(EdgeCoord {
                orientation: Orientation::Vertical,
                row: (local / stride) as u8,
                col: (local % stride) as u8,
            })
        } else {
            None
        }
    }

    /// Box id for the box whose top-left corner is at grid cell `(row, col)`.
    pub const fn box_id(self, row: u8, col: u8) -> Option<BoxId> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        Some(row as u16 * self.cols as u16 + col as u16)
    }

    /// Row/col of a box id.
    pub const fn box_coord(self, id: BoxId) -> Option<(u8, u8)> {
        if id >= self.box_count() {
            return None;
        }
        let cols = self.cols as u16;
        Some(((id / cols) as u8, (id % cols) as u8))
    }
}

/// Packed board state: drawn edges + claimed boxes.
///
/// Box *ownership* (which player scored the box) lives on [`crate::game::Game`];
/// this bitboard only tracks which boxes are complete/claimed in the position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    geom: BoardGeom,
    edges: EdgeBits,
    boxes: BoxBits,
}

impl Position {
    /// Empty position (no edges drawn, no boxes claimed).
    pub const fn new(geom: BoardGeom) -> Self {
        Self {
            geom,
            edges: EdgeBits::EMPTY,
            boxes: BoxBits::EMPTY,
        }
    }

    #[inline]
    pub const fn geom(self) -> BoardGeom {
        self.geom
    }

    #[inline]
    pub const fn edges(self) -> EdgeBits {
        self.edges
    }

    #[inline]
    pub const fn boxes(self) -> BoxBits {
        self.boxes
    }

    #[inline]
    pub fn edge_is_drawn(self, id: EdgeId) -> bool {
        debug_assert!(id < self.geom.edge_count());
        self.edges.get(id)
    }

    #[inline]
    pub fn box_is_claimed(self, id: BoxId) -> bool {
        debug_assert!(id < self.geom.box_count());
        self.boxes.get(id)
    }

    /// Mark an edge as drawn. Does not check legality or update boxes.
    #[inline]
    pub fn set_edge_drawn(&mut self, id: EdgeId) {
        debug_assert!(id < self.geom.edge_count());
        self.edges.set(id);
    }

    /// Clear a drawn edge (for undo / testing).
    #[inline]
    pub fn clear_edge_drawn(&mut self, id: EdgeId) {
        debug_assert!(id < self.geom.edge_count());
        self.edges.clear(id);
    }

    /// Mark a box as claimed.
    #[inline]
    pub fn set_box_claimed(&mut self, id: BoxId) {
        debug_assert!(id < self.geom.box_count());
        self.boxes.set(id);
    }

    /// Clear a claimed box (for undo / testing).
    #[inline]
    pub fn clear_box_claimed(&mut self, id: BoxId) {
        debug_assert!(id < self.geom.box_count());
        self.boxes.clear(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_round_trip(geom: BoardGeom) {
        for id in 0..geom.edge_count() {
            let coord = geom.edge_coord(id).expect("coord");
            let back = geom.edge_id(coord).expect("id");
            assert_eq!(back, id, "round-trip failed at edge {id} on {geom:?}");
        }
    }

    #[test]
    fn geom_rejects_invalid_sizes() {
        assert!(BoardGeom::new(0, 3).is_none());
        assert!(BoardGeom::new(3, 0).is_none());
        assert!(BoardGeom::new(MAX_ROWS + 1, 3).is_none());
        assert!(BoardGeom::new(3, MAX_COLS + 1).is_none());
    }

    #[test]
    fn edge_counts_match_formula() {
        let g = BoardGeom::new(3, 4).unwrap();
        assert_eq!(g.box_count(), 12);
        assert_eq!(g.horizontal_count(), 4 * 4); // (3+1)*4
        assert_eq!(g.vertical_count(), 3 * 5);
        assert_eq!(g.edge_count(), 16 + 15);
    }

    #[test]
    fn edge_index_round_trip_multiple_sizes() {
        for rows in 1..=MAX_ROWS {
            for cols in 1..=MAX_COLS {
                let geom = BoardGeom::new(rows, cols).unwrap();
                assert_round_trip(geom);
            }
        }
    }

    #[test]
    fn known_corner_edges_on_2x2() {
        let g = BoardGeom::new(2, 2).unwrap();
        // Top-left horizontal
        assert_eq!(
            g.edge_id(EdgeCoord {
                orientation: Orientation::Horizontal,
                row: 0,
                col: 0
            }),
            Some(0)
        );
        // Bottom-right horizontal (row=2, col=1) → 2*2+1 = 5
        assert_eq!(
            g.edge_id(EdgeCoord {
                orientation: Orientation::Horizontal,
                row: 2,
                col: 1
            }),
            Some(5)
        );
        // First vertical starts after 6 horizontals
        assert_eq!(
            g.edge_id(EdgeCoord {
                orientation: Orientation::Vertical,
                row: 0,
                col: 0
            }),
            Some(6)
        );
        assert_eq!(g.edge_count(), 6 + 6);
    }

    #[test]
    fn box_coord_round_trip() {
        let g = BoardGeom::new(3, 5).unwrap();
        for id in 0..g.box_count() {
            let (r, c) = g.box_coord(id).unwrap();
            assert_eq!(g.box_id(r, c), Some(id));
        }
    }

    #[test]
    fn position_is_copy_and_tracks_bits() {
        fn assert_copy<T: Copy>(_: T) {}
        let geom = BoardGeom::new(5, 5).unwrap();
        let mut pos = Position::new(geom);
        assert_copy(pos);
        // Cheap for search: fixed-size, stack-friendly, no heap.
        assert!(core::mem::size_of::<Position>() <= 64);

        pos.set_edge_drawn(0);
        pos.set_edge_drawn(17);
        pos.set_box_claimed(3);
        assert!(pos.edge_is_drawn(0));
        assert!(pos.edge_is_drawn(17));
        assert!(!pos.edge_is_drawn(1));
        assert!(pos.box_is_claimed(3));
        assert_eq!(pos.edges().count_ones(), 2);
        assert_eq!(pos.boxes().count_ones(), 1);
    }

    #[test]
    fn out_of_range_coords_rejected() {
        let g = BoardGeom::new(2, 3).unwrap();
        assert!(g
            .edge_id(EdgeCoord {
                orientation: Orientation::Horizontal,
                row: 3,
                col: 0
            })
            .is_none());
        assert!(g
            .edge_id(EdgeCoord {
                orientation: Orientation::Horizontal,
                row: 0,
                col: 3
            })
            .is_none());
        assert!(g
            .edge_id(EdgeCoord {
                orientation: Orientation::Vertical,
                row: 2,
                col: 0
            })
            .is_none());
        assert!(g.edge_coord(g.edge_count()).is_none());
    }
}
