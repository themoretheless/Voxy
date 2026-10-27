//! Original opaque GI may explicitly match its source raster arithmetic.
//! Compensated DDA and every unrelated compute kernel retain precise math.
pub(super) fn original_gi_precise_math(
    is_compute: bool,
    entry: &str,
    paired_dda: bool,
    match_raster: bool,
) -> bool {
    paired_dda || (is_compute && !(entry == "cs_opaque_gi" && match_raster))
}

#[cfg(test)]
mod tests {
    use super::original_gi_precise_math as precise;
    #[test]
    fn raster_gi_math_policy_is_explicit_and_preserves_other_kernels() {
        assert!(precise(true, "cs_opaque_gi", false, false));
        assert!(!precise(true, "cs_opaque_gi", false, true));
        assert!(precise(true, "cs_opaque_gi", true, true));
        assert!(precise(true, "geometry", false, true));
        assert!(!precise(false, "fs_opaque", false, true));
        assert!(precise(false, "fs_opaque", true, true));
        assert!(!precise(false, "vs_main", false, true));
    }
}
