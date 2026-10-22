//! Original instance prefix plus 18 interval endpoint words and nine valid bits.
#[derive(Debug)]
pub(super) struct Layout {
    pub words: u32,
    pub slots: u32,
    pub triangles: u32,
    pub records: u32,
    pub prefix: Vec<u32>,
}
impl Layout {
    pub fn new(counts: &[u32], limit: u64, max_groups: u32) -> Option<Self> {
        let slots = u32::try_from(counts.len()).ok()?;
        if slots == 0 {
            return None;
        }
        let mut prefix = Vec::with_capacity(counts.len().checked_add(1)?);
        prefix.push(0u32);
        for &count in counts {
            prefix.push(prefix.last()?.checked_add(count)?);
        }
        let triangles = *prefix.last()?;
        if triangles == 0 {
            return None;
        }
        let records = 17u32.checked_add(slots)?;
        let words = records.checked_add(triangles.checked_mul(19)?)?;
        if u64::from(words) * 4 > limit || triangles.div_ceil(64) > max_groups {
            return None;
        }
        Some(Self {
            words,
            slots,
            triangles,
            records,
            prefix,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_prefix_and_record_capacity() {
        let l = Layout::new(&[1, 0, 3, 5], 1024, 100).unwrap();
        assert_eq!(l.prefix, [0, 1, 1, 4, 9]);
        assert_eq!((l.records, l.words), (21, 192));
    }
    #[test]
    fn reject_unrepresentable_budgets() {
        for counts in [&[][..], &[0], &[u32::MAX, 1]] {
            assert!(Layout::new(counts, u64::MAX, u32::MAX).is_none());
        }
        assert!(Layout::new(&[1], 147, 1).is_none());
        assert!(Layout::new(&[65], u64::MAX, 1).is_none());
    }
    #[test]
    fn complete_current_street_fits_256_mib() {
        let mut counts = vec![0; 117337];
        counts[0] = 3472196;
        let l = Layout::new(&counts, 256 * 1024 * 1024, 65535).unwrap();
        assert_eq!(u64::from(l.words) * 4, 264356312);
        assert!(Layout::new(&counts, 128 * 1024 * 1024, 65535).is_none());
    }
}
