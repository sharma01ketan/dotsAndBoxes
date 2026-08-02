//! GPU compute kernels via wgpu / WGSL (Phase 4).
//!
//! Stub crate so the workspace layout matches PLAN.md from day one.

/// Placeholder until WebGPU kernels land (KET-38).
pub fn crate_name() -> &'static str {
    "dab-gpu"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_stable() {
        assert_eq!(crate_name(), "dab-gpu");
    }
}
