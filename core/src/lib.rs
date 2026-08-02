//! Shared Dots and Boxes game engine.
//!
//! This crate is the single source of truth for rules, bitboards, and solvers.
//! It compiles natively (server / training) and later to WASM (browser).

/// Placeholder until the board model lands (KET-6).
pub fn crate_name() -> &'static str {
    "dab-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_stable() {
        assert_eq!(crate_name(), "dab-core");
    }
}
