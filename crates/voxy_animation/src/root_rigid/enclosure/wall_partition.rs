//! Stored wall domains with explicit uncertainty around exact retimed clip keys.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct RootRigidWallInterval {
    pub start: f64,
    pub end: f64,
    pub source_span: Option<usize>,
    pub target_span: Option<usize>,
    /// A retimed key may occur here. No single-span derivative cap is implied.
    pub key_uncertainty: bool,
}
#[derive(Debug)]
pub struct RootRigidWallPartition<'a> {
    source: &'a RootRigidPath,
    target: &'a RootRigidPath,
    intervals: Vec<RootRigidWallInterval>,
}
impl RootRigidWallPartition<'_> {
    pub fn source(&self) -> &RootRigidPath {
        self.source
    }
    pub fn target(&self) -> &RootRigidPath {
        self.target
    }
    pub fn intervals(&self) -> &[RootRigidWallInterval] {
        &self.intervals
    }
}
impl RootRigidPath {
    /// Encloses exact stored key_time / clip_duration * wall_duration.
    /// Outside marked key domains, indices identify proved continuous spans.
    /// Marked domains require whole-field bounds across all intersecting keys.
    /// Instantaneous pose STEP events reject until an explicit policy exists.
    pub fn partition_wall_outward<'a>(
        &'a self,
        target: &'a Self,
        wall_duration: f64,
        max_parts: usize,
    ) -> Result<RootRigidWallPartition<'a>, AnimationError> {
        if !wall_duration.is_finite() || wall_duration <= 0. {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let mut cuts = vec![0., wall_duration];
        let mut guards = Vec::new();
        let mut prepared = [Vec::new(), Vec::new()];
        for (side, path) in [self, target].into_iter().enumerate() {
            for (index, span) in path.spans().iter().enumerate() {
                if span.is_step() {
                    return Err(AnimationError::RootRotationTransitionUnsupported);
                }
                if span.end() <= span.start() {
                    return Err(AnimationError::InvalidSampleTime);
                }
                let clock = |time: f64| -> Result<[f64; 2], AnimationError> {
                    if !time.is_finite() || time < 0. || time > path.duration() {
                        return Err(AnimationError::InvalidSampleTime);
                    }
                    if time == 0. {
                        return Ok([0., 0.]);
                    }
                    if time == path.duration() {
                        return Ok([wall_duration, wall_duration]);
                    }
                    let value = Scalar::exact(time)
                        .div_interval_positive(Scalar::exact(path.duration()))?
                        .mul(Scalar::exact(wall_duration))?;
                    Ok([value.0.max(0.), value.1.min(wall_duration)])
                };
                let start = clock(span.start())?;
                let end = clock(span.end())?;
                for key in [start, end] {
                    cuts.extend(key);
                    if key[0] < key[1] {
                        guards.push(key);
                    }
                }
                prepared[side].push((index, start, end));
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        if cuts.len() - 1 > max_parts.min(MAX_ROOT_ROTATION_SPANS) {
            return Err(AnimationError::RootRigidBudget);
        }
        guards.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let mut merged: Vec<[f64; 2]> = Vec::new();
        for key in guards {
            if let Some(last) = merged.last_mut().filter(|last| last[1] >= key[0]) {
                last[1] = last[1].max(key[1]);
            } else {
                merged.push(key);
            }
        }
        let mut guard_cursor = 0;
        let mut cursors = [0; 2];
        let mut intervals = Vec::with_capacity(cuts.len() - 1);
        for cut in cuts.windows(2) {
            while merged.get(guard_cursor).is_some_and(|key| key[1] <= cut[0]) {
                guard_cursor += 1;
            }
            let uncertain = merged
                .get(guard_cursor)
                .is_some_and(|key| cut[0] < key[1] && cut[1] > key[0]);
            let mut covering = |side: usize| {
                if uncertain {
                    return None;
                }
                while prepared[side]
                    .get(cursors[side])
                    .is_some_and(|(_, _, end)| end[0] < cut[1])
                {
                    cursors[side] += 1;
                }
                prepared[side]
                    .get(cursors[side])
                    .filter(|(_, start, end)| start[1] <= cut[0] && end[0] >= cut[1])
                    .map(|(index, _, _)| *index)
            };
            intervals.push(RootRigidWallInterval {
                start: cut[0],
                end: cut[1],
                source_span: covering(0),
                target_span: covering(1),
                key_uncertainty: uncertain,
            });
        }
        Ok(RootRigidWallPartition {
            source: self,
            target,
            intervals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonrepresentable_key_clock_is_guarded_and_spans_outside_are_proved() {
        let twist = RootRigidTwist {
            linear: DVec3::X,
            angular: DVec3::ZERO,
        };
        let source = RootRigidPath::from_twists(&[(twist, 1.), (twist, 2.)], 2).unwrap();
        let target = RootRigidPath::from_twists(&[(twist, 0.5), (twist, 0.5)], 2).unwrap();
        let partition = source.partition_wall_outward(&target, 1., 32).unwrap();
        assert!(std::ptr::eq(partition.source(), &source));
        assert!(std::ptr::eq(partition.target(), &target));
        let intervals = partition.intervals();
        assert_eq!(intervals[0].start, 0.);
        assert_eq!(intervals.last().unwrap().end, 1.);
        assert!(intervals
            .windows(2)
            .all(|pair| pair[0].end == pair[1].start));
        let guarded = intervals
            .iter()
            .find(|part| part.start <= 1. / 3. && part.end >= 1. / 3.)
            .unwrap();
        assert!(guarded.key_uncertainty);
        assert_eq!((guarded.source_span, guarded.target_span), (None, None));
        println!(
            "WALL_KEY_GUARD {:?}",
            (1., 3., 1., [guarded.start, guarded.end])
        );
        for part in intervals.iter().filter(|part| !part.key_uncertainty) {
            assert_eq!(
                part.source_span,
                Some(if part.end < 1. / 3. { 0 } else { 1 })
            );
            assert_eq!(part.target_span, Some(if part.end < 0.5 { 0 } else { 1 }));
        }
        assert!(source.partition_wall_outward(&target, 1., 1).is_err());
    }
}
