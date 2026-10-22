//! Source prefix plus exact signed radix-16 sparse world coordinates.
#[derive(Debug)]
pub(super) struct Layout {
    pub header: u32,
    pub records: u32,
    pub words: u32,
    pub vertices: u32,
    pub prefix: Vec<u32>,
}
impl Layout {
    pub fn new(source_words: u32, counts: &[u32], byte_limit: u64) -> Option<Self> {
        if source_words == 0 || !source_words.is_multiple_of(12) || counts.is_empty() {
            return None;
        }
        let slots = u32::try_from(counts.len()).ok()?;
        let mut prefix = vec![0u32];
        for &count in counts {
            prefix.push(prefix.last()?.checked_add(count)?);
        }
        let vertices = *prefix.last()?;
        if vertices == 0 {
            return None;
        }
        let header = source_words;
        let unaligned = header.checked_add(17)?.checked_add(slots)?;
        let records = unaligned.checked_add(3)? & !3;
        let words = records.checked_add(vertices.checked_mul(12)?)?;
        if u64::from(words) * 4 > byte_limit {
            return None;
        }
        Some(Self {
            header,
            records,
            words,
            vertices,
            prefix,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_prefix_records_and_zero_instance_ranges() {
        let x = Layout::new(24, &[3, 0, 2], 4096).unwrap();
        assert_eq!(x.prefix, [0, 3, 3, 5]);
        assert_eq!((x.header, x.records, x.words), (24, 44, 104));
    }
    #[test]
    fn reject_overflow_and_incomplete_capacity() {
        assert!(Layout::new(0, &[1], u64::MAX).is_none());
        assert!(Layout::new(13, &[1], u64::MAX).is_none());
        assert!(Layout::new(12, &[u32::MAX, 1], u64::MAX).is_none());
        assert!(Layout::new(12, &[1], 175).is_none());
    }
    #[test]
    fn full_current_street_retains_all_vertices() {
        let mut counts = vec![0; 117337];
        counts[0] = 3503016;
        let x = Layout::new(319824 / 4, &counts, 256 * 1024 * 1024).unwrap();
        assert_eq!(u64::from(x.words) * 4, 168934016);
    }
}
