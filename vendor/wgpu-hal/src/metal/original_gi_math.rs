//! Explicit pipeline override policy for Metal fast-math vs IEEE/uncontracted math.
//! Compute pipelines default to precise math when `metal-safe-compute` is enabled;
//! pipelines may opt in (`> 0.0`) or opt out (`< 0.0`) via `WGPU_METAL_PRECISE_MATH`.
pub(super) fn metal_precise_math(is_compute: bool, override_mode: Option<f64>) -> bool {
    match override_mode {
        Some(v) if v > 0.0 => true,
        Some(v) if v < 0.0 => false,
        _ => is_compute,
    }
}

#[cfg(test)]
mod tests {
    use super::metal_precise_math as precise;
    #[test]
    fn metal_precise_math_policy_respects_stage_default_and_explicit_override() {
        assert!(precise(true, None));
        assert!(precise(true, Some(0.0)));
        assert!(!precise(true, Some(-1.0)));
        assert!(precise(true, Some(1.0)));
        assert!(!precise(false, None));
        assert!(!precise(false, Some(0.0)));
        assert!(precise(false, Some(1.0)));
    }
}
