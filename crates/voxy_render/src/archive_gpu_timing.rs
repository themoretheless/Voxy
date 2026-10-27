//! Decode stage-boundary timestamps without assuming different GPU stages
//! execute serially. Metal records pass start at vertex start and pass end at
//! fragment end; intervals from different passes can overlap.
pub(crate) fn decode(ticks: &[u64], period_ns: f32) -> Option<[f64; 3]> {
    if ticks.len() != 6
        || !period_ns.is_finite()
        || period_ns <= 0.
        || ticks.iter().any(|&t| t == 0 || t == u64::MAX)
    {
        return None;
    }
    if ticks[2] < ticks[1] || ticks[4] < ticks[3] {
        return None;
    }
    // On Metal the display pass's end sample can still hold the previous
    // frame's value when resolved; the span then ends at the last shading pass.
    let span = if ticks[5] > ticks[0] {
        ticks
    } else {
        &ticks[..5]
    };
    let first = *span.iter().min()?;
    let last = *span.iter().max()?;
    let scale = f64::from(period_ns) / 1e6;
    Some([
        (last - first) as f64 * scale,
        (ticks[2] - ticks[1]) as f64 * scale,
        (ticks[4] - ticks[3]) as f64 * scale,
    ])
}
#[cfg(test)]
mod tests {
    use super::decode;
    #[test]
    fn sequential() {
        assert_eq!(
            decode(&[100, 120, 200, 220, 260, 300], 1_000_000.),
            Some([200., 80., 40.])
        );
    }
    #[test]
    fn overlapping_passes() {
        assert_eq!(
            decode(&[100, 120, 220, 180, 260, 300], 1_000_000.),
            Some([200., 100., 80.])
        );
    }
    #[test]
    fn overlap_can_exceed_span_when_summed() {
        let v = decode(&[100, 110, 280, 120, 290, 300], 1_000_000.).unwrap();
        assert_eq!(v, [200., 170., 170.]);
        assert!(v[1] + v[2] > v[0]);
    }
    #[test]
    fn reversed_pair_is_invalid() {
        assert!(decode(&[100, 220, 120, 180, 260, 300], 1.).is_none());
        assert!(decode(&[100, 120, 180, 260, 220, 300], 1.).is_none());
    }
    #[test]
    fn stale_display_end_falls_back_to_shading_span() {
        assert_eq!(
            decode(&[100, 120, 200, 220, 260, 50], 1_000_000.),
            Some([160., 80., 40.])
        );
    }
    #[test]
    fn unset_or_error_results_are_invalid() {
        assert!(decode(&[0; 6], 1.).is_none());
        assert!(decode(&[100, 0, 0, 180, 260, 300], 1.).is_none());
        assert!(decode(&[u64::MAX; 6], 1.).is_none());
        assert!(decode(&[1, 2], 1.).is_none());
    }
    #[test]
    fn bad_periods_are_invalid() {
        for period in [0., -1., f32::NAN, f32::INFINITY] {
            assert!(decode(&[100, 120, 200, 220, 260, 300], period).is_none());
        }
    }
}

// Additional GI passes are optional; absent timestamps are not zero-duration work.
pub(crate) fn decode_with_gi(
    ticks: &[u64],
    period_ns: f32,
) -> Option<([f64; 3], Option<[f64; 2]>)> {
    if ticks.len() == 6 {
        return decode(ticks, period_ns).map(|base| (base, None));
    }
    if ticks.len() != 10 {
        return None;
    }
    let mut base = decode(&ticks[..6], period_ns)?;
    if ticks[6..].iter().any(|&t| t == 0 || t == u64::MAX)
        || ticks[7] < ticks[6]
        || ticks[9] < ticks[8]
    {
        return None;
    }
    let scale = f64::from(period_ns) / 1e6;
    let valid: Vec<_> = ticks
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != 5 || ticks[5] > ticks[0])
        .map(|(_, v)| *v)
        .collect();
    base[0] = (*valid.iter().max()? - *valid.iter().min()?) as f64 * scale;
    Some((
        base,
        Some([
            (ticks[7] - ticks[6]) as f64 * scale,
            (ticks[9] - ticks[8]) as f64 * scale,
        ]),
    ))
}
#[cfg(test)]
mod gi_tests {
    use super::decode_with_gi;
    #[test]
    fn optional_stages_are_absent() {
        assert_eq!(
            decode_with_gi(&[100, 120, 200, 220, 260, 300], 1_000_000.),
            Some(([200., 80., 40.], None))
        );
    }
    #[test]
    fn compute_and_fragment_intervals_can_overlap() {
        assert_eq!(
            decode_with_gi(
                &[100, 170, 220, 180, 260, 300, 110, 140, 130, 190],
                1_000_000.
            ),
            Some(([200., 50., 80.], Some([30., 60.])))
        );
    }
    #[test]
    fn includes_gi_in_total_and_ignores_stale_display() {
        assert_eq!(
            decode_with_gi(
                &[100, 170, 220, 180, 260, 50, 90, 140, 130, 280],
                1_000_000.
            ),
            Some(([190., 50., 80.], Some([50., 150.])))
        );
    }
    #[test]
    fn invalid_gi_samples_rejected() {
        for extra in [
            [0, 140, 130, 190],
            [140, 110, 130, 190],
            [110, 140, 190, 130],
            [110, 140, 130, u64::MAX],
        ] {
            let mut ticks = vec![100, 170, 220, 180, 260, 300];
            ticks.extend(extra);
            assert!(decode_with_gi(&ticks, 1.).is_none());
        }
        assert!(decode_with_gi(&[1; 8], 1.).is_none());
    }
}

pub(crate) fn decode_with_rays(
    ticks: &[u64],
    period_ns: f32,
) -> Option<([f64; 3], Option<[f64; 2]>, Option<f64>)> {
    if ticks.len() != 12 {
        return decode_with_gi(ticks, period_ns).map(|(base, gi)| (base, gi, None));
    }
    let (mut base, gi) = decode_with_gi(&ticks[..10], period_ns)?;
    if ticks[10..].iter().any(|&t| t == 0 || t == u64::MAX) || ticks[11] < ticks[10] {
        return None;
    }
    let valid: Vec<_> = ticks
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != 5 || ticks[5] > ticks[0])
        .map(|(_, v)| *v)
        .collect();
    let scale = f64::from(period_ns) / 1e6;
    base[0] = (*valid.iter().max()? - *valid.iter().min()?) as f64 * scale;
    Some((base, gi, Some((ticks[11] - ticks[10]) as f64 * scale)))
}
#[cfg(test)]
mod ray_tests {
    use super::decode_with_rays;
    #[test]
    fn absent_reflections_are_not_zero_samples() {
        assert_eq!(
            decode_with_rays(&[100, 120, 200, 220, 260, 300], 1_000_000.),
            Some(([200., 80., 40.], None, None))
        );
    }
    #[test]
    fn includes_reflections_in_frame_span() {
        assert_eq!(
            decode_with_rays(
                &[100, 170, 220, 180, 260, 50, 110, 140, 130, 190, 90, 280],
                1_000_000.
            ),
            Some(([190., 50., 80.], Some([30., 60.]), Some(190.)))
        );
    }
    #[test]
    fn invalid_reflections_rejected() {
        for extra in [[0, 280], [280, 90], [90, u64::MAX]] {
            let mut ticks = vec![100, 170, 220, 180, 260, 300, 110, 140, 130, 190];
            ticks.extend(extra);
            assert!(decode_with_rays(&ticks, 1.).is_none());
        }
    }
}

// Reuse aggregate endpoints; every intermediate boundary has one unique slot.
pub(crate) fn ray_stage_index(
    base: u32,
    chunks: u32,
    kind: u32,
    tile: u32,
    stage: u32,
    end: bool,
) -> u32 {
    let ordinal = tile * 6 + stage * 2 + u32::from(end);
    if ordinal == 0 {
        8 + kind * 2
    } else if ordinal == chunks * 6 - 1 {
        9 + kind * 2
    } else {
        base + kind * (chunks * 6 - 2) + ordinal - 1
    }
}
pub(crate) fn ray_stage_count(base: u32, chunks: u32) -> Option<u32> {
    if chunks == 0 || (base != 10 && base != 12) {
        return None;
    }
    base.checked_add(
        chunks
            .checked_mul(6)?
            .checked_sub(2)?
            .checked_mul(if base == 12 { 2 } else { 1 })?,
    )
}
pub(crate) fn ray_stage_durations(
    ticks: &[u64],
    base: u32,
    chunks: u32,
    period: f32,
) -> Option<Vec<[f64; 3]>> {
    if ticks.len() != ray_stage_count(base, chunks)? as usize || !period.is_finite() || period <= 0.
    {
        return None;
    }
    let mut result = vec![[0.; 3]; if base == 12 { 2 } else { 1 }];
    for (kind, values) in result.iter_mut().enumerate() {
        for tile in 0..chunks {
            for stage in 0..3 {
                let a =
                    ticks[ray_stage_index(base, chunks, kind as u32, tile, stage, false) as usize];
                let b =
                    ticks[ray_stage_index(base, chunks, kind as u32, tile, stage, true) as usize];
                if a == 0 || b == u64::MAX || b < a {
                    return None;
                }
                values[stage as usize] += (b - a) as f64 * f64::from(period) / 1e6;
            }
        }
    }
    Some(result)
}
#[cfg(test)]
mod phase_tests {
    use super::*;
    #[test]
    fn unique_complete_slots() {
        for base in [10, 12] {
            for chunks in [1, 16, 20, 512] {
                let count = ray_stage_count(base, chunks).unwrap();
                let mut slots = std::collections::BTreeSet::new();
                for kind in 0..if base == 12 { 2 } else { 1 } {
                    for tile in 0..chunks {
                        for stage in 0..3 {
                            for end in [false, true] {
                                assert!(
                                    slots.insert(ray_stage_index(
                                        base, chunks, kind, tile, stage, end
                                    ))
                                );
                            }
                        }
                    }
                }
                assert_eq!(slots, (8..count).collect());
            }
        }
    }
    #[test]
    fn sums_each_stage_without_assuming_serial_execution() {
        let mut ticks = vec![100; 20];
        for kind in 0..2 {
            for stage in 0..3 {
                ticks[ray_stage_index(12, 1, kind, 0, stage, false) as usize] = 100;
                ticks[ray_stage_index(12, 1, kind, 0, stage, true) as usize] = 200;
            }
        }
        assert_eq!(
            ray_stage_durations(&ticks, 12, 1, 1_000_000.),
            Some(vec![[100.; 3]; 2])
        );
    }
    #[test]
    fn invalid_shapes_rejected() {
        assert_eq!(ray_stage_count(12, 0), None);
        assert_eq!(ray_stage_count(6, 1), None);
        assert_eq!(ray_stage_count(12, u32::MAX), None);
        assert!(ray_stage_durations(&[0; 20], 12, 1, 1.).is_none());
    }
}

/// Never resolve unwritten optional-stage queries on an inline crossover frame.
pub(crate) fn active_query_count(base:u32,total:u32,chunks:u32,inactive_gi:bool)->u32 {
    if inactive_gi {6} else if chunks>0 {total} else {base}
}
#[cfg(test)]
mod optional_stage_tests {
    #[test]
    fn inactive_gi_never_resolves_stale_or_unwritten_optional_counters() {
        assert_eq!(super::active_query_count(10,10,0,true),6);
        assert_eq!(super::active_query_count(12,24,2,true),6);
        assert_eq!(super::active_query_count(10,10,0,false),10);
        assert_eq!(super::active_query_count(12,24,2,false),24);
        assert_eq!(super::active_query_count(6,6,0,false),6);
    }
}
