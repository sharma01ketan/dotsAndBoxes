//! Tiny deterministic PRNG shared by engines and tests (no `rand` dep).

/// Xorshift64 — fast, seedable, good enough for move selection and fuzzing.
#[derive(Clone, Debug)]
pub struct XorShift64(u64);

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        // Zero is a fixed point of xorshift; avoid it.
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform index in `0..len` via rejection sampling (no modulo bias).
    pub fn gen_index(&mut self, len: usize) -> usize {
        debug_assert!(len > 0);
        let len = len as u64;
        let zone = u64::MAX - (u64::MAX % len);
        loop {
            let r = self.next_u64();
            if r < zone {
                return (r % len) as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_zero_is_usable_and_deterministic() {
        let mut a = XorShift64::new(0);
        let mut b = XorShift64::new(0);
        assert_eq!(a.next_u64(), b.next_u64());
        assert_eq!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn gen_index_stays_in_range() {
        let mut rng = XorShift64::new(1);
        for len in [1usize, 2, 7, 24] {
            for _ in 0..200 {
                let i = rng.gen_index(len);
                assert!(i < len);
            }
        }
    }
}
