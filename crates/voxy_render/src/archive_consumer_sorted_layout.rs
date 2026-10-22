//! Exact storage sizes; this module has no device or shader dependencies.
pub(crate) const HEADER_WORDS: u64 = 128;
pub(crate) fn sizes(capacity: u32, limit: u64) -> Option<[u64; 3]> {
    let n = u64::from(capacity);
    let work_words = HEADER_WORDS.checked_add(n.checked_mul(2)?)?;
    let sizes = [
        n.checked_mul(64)?,
        n.checked_mul(32)?,
        work_words.checked_mul(4)?,
    ];
    (n > 0 && u32::try_from(work_words).is_ok() && sizes.iter().all(|&s| s <= limit))
        .then_some(sizes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_width_strips_fit_without_truncating_words() {
        let s = sizes(1_658_880, 128 * 1024 * 1024).unwrap();
        assert_eq!(s, [106_168_320, 53_084_160, 13_271_552]);
        assert_eq!(s.iter().sum::<u64>(), 1_658_880 * 104 + 512);
    }
    #[test]
    fn reject_empty_oversized_or_unaddressable_allocations() {
        assert!(sizes(0, u64::MAX).is_none());
        assert!(sizes(2_097_153, 128 * 1024 * 1024).is_none());
        assert!(sizes(u32::MAX, u64::MAX).is_none());
        assert!(sizes(1, 519).is_none());
        assert_eq!(sizes(1, 520), Some([64, 32, 520]));
    }
}
