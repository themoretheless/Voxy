use super::*;

/// Shared normalized progress interval. Indices refer to the original paths.
/// None means a stationary gap; original clip durations remain explicit.
#[derive(Clone, Debug, PartialEq)]
pub struct RootMotionInterval {
    pub start: f64,
    pub end: f64,
    pub source_span: Option<usize>,
    pub target_span: Option<usize>,
}
/// Simultaneous events retain each path's original event order.
/// No interpolation or source/target event composition is inferred here.
#[derive(Clone, Debug, PartialEq)]
pub struct RootMotionStep {
    pub progress: f64,
    pub source_spans: Vec<usize>,
    pub target_spans: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RootMotionPartition {
    pub intervals: Vec<RootMotionInterval>,
    pub steps: Vec<RootMotionStep>,
}
impl RootRigidPath {
    /// Partitions two paths on shared progress without crossing any span boundary.
    /// Durations can differ: velocities/rate bounds must be explicitly retimed by
    /// the caller to a shared wall interval before integration.
    /// # Errors
    /// Invalid normalized cuts or combined interval/event capacity exhaustion.
    pub fn partition_for_blend(
        &self,
        target: &Self,
        max_parts: usize,
    ) -> Result<RootMotionPartition, AnimationError> {
        let mut cuts = vec![0., 1.];
        let mut events = Vec::new();
        for (source, path) in [(true, self), (false, target)] {
            for (index, span) in path.spans().iter().enumerate() {
                let start = span.start() / path.duration();
                let end = span.end() / path.duration();
                if !start.is_finite()
                    || !end.is_finite()
                    || start < 0.
                    || end > 1.
                    || end < start
                    || (end == start && !span.is_step())
                {
                    return Err(AnimationError::NumericalOverflow);
                }
                cuts.extend([start, end]);
                if span.is_step() {
                    events.push((start, source, index));
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        let capacity = max_parts.min(MAX_ROOT_ROTATION_SPANS);
        if cuts.len() - 1 + events.len() > capacity {
            return Err(AnimationError::RootRigidBudget);
        }
        let covering = |path: &Self, start: f64, end: f64, cursor: &mut usize| {
            while let Some(span) = path.spans().get(*cursor) {
                if !span.is_step() && span.end() / path.duration() > start {
                    break;
                }
                *cursor += 1;
            }
            path.spans()
                .get(*cursor)
                .filter(|span| {
                    span.start() / path.duration() <= start && span.end() / path.duration() >= end
                })
                .map(|_| *cursor)
        };
        let mut source_cursor = 0;
        let mut target_cursor = 0;
        let intervals = cuts
            .windows(2)
            .map(|pair| RootMotionInterval {
                start: pair[0],
                end: pair[1],
                source_span: covering(self, pair[0], pair[1], &mut source_cursor),
                target_span: covering(target, pair[0], pair[1], &mut target_cursor),
            })
            .collect();
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut steps: Vec<RootMotionStep> = Vec::new();
        for (progress, source, index) in events {
            if steps.last().is_none_or(|step| step.progress != progress) {
                steps.push(RootMotionStep {
                    progress,
                    source_spans: vec![],
                    target_spans: vec![],
                });
            }
            let step = steps.last_mut().unwrap();
            if source {
                step.source_spans.push(index);
            } else {
                step.target_spans.push(index);
            }
        }
        Ok(RootMotionPartition { intervals, steps })
    }
}
