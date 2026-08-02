//! Fixed-width bitsets packed into `u64` words.
//!
//! Sized for boards up to [`crate::board::MAX_ROWS`] × [`crate::board::MAX_COLS`]
//! boxes (enough for classic play and small-board perfect solvers).

use core::fmt;

/// Number of `u64` words for the edge bitboard (256 bits).
pub const EDGE_WORDS: usize = 4;
/// Number of `u64` words for the box bitboard (128 bits).
pub const BOX_WORDS: usize = 2;

/// Compact bitset used for drawn edges or claimed boxes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitboard<const N: usize> {
    words: [u64; N],
}

pub type EdgeBits = Bitboard<EDGE_WORDS>;
pub type BoxBits = Bitboard<BOX_WORDS>;

impl<const N: usize> Default for Bitboard<N> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl<const N: usize> Bitboard<N> {
    pub const EMPTY: Self = Self { words: [0; N] };

    #[inline]
    pub const fn new() -> Self {
        Self::EMPTY
    }

    #[inline]
    pub const fn capacity_bits() -> usize {
        N * 64
    }

    #[inline]
    pub fn get(self, index: u16) -> bool {
        let i = index as usize;
        debug_assert!(i < Self::capacity_bits());
        let word = i / 64;
        let bit = i % 64;
        (self.words[word] >> bit) & 1 == 1
    }

    #[inline]
    pub fn set(&mut self, index: u16) {
        let i = index as usize;
        debug_assert!(i < Self::capacity_bits());
        let word = i / 64;
        let bit = i % 64;
        self.words[word] |= 1u64 << bit;
    }

    #[inline]
    pub fn clear(&mut self, index: u16) {
        let i = index as usize;
        debug_assert!(i < Self::capacity_bits());
        let word = i / 64;
        let bit = i % 64;
        self.words[word] &= !(1u64 << bit);
    }

    #[inline]
    pub fn toggle(&mut self, index: u16) {
        let i = index as usize;
        debug_assert!(i < Self::capacity_bits());
        let word = i / 64;
        let bit = i % 64;
        self.words[word] ^= 1u64 << bit;
    }

    #[inline]
    pub fn count_ones(self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    #[inline]
    pub fn words(&self) -> &[u64; N] {
        &self.words
    }

    #[inline]
    pub fn from_words(words: [u64; N]) -> Self {
        Self { words }
    }
}

impl<const N: usize> fmt::Debug for Bitboard<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Bitboard").field(&self.words).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_clear_round_trip() {
        let mut bb = EdgeBits::new();
        assert!(!bb.get(0));
        bb.set(0);
        bb.set(63);
        bb.set(64);
        bb.set(200);
        assert!(bb.get(0));
        assert!(bb.get(63));
        assert!(bb.get(64));
        assert!(bb.get(200));
        assert_eq!(bb.count_ones(), 4);
        bb.clear(64);
        assert!(!bb.get(64));
        assert_eq!(bb.count_ones(), 3);
    }

    #[test]
    fn bitboard_is_copy() {
        fn assert_copy<T: Copy>(_: T) {}
        assert_copy(EdgeBits::new());
        assert_copy(BoxBits::new());
    }
}
