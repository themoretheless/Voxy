//! Rebuild with trusted native compilers, then require bitwise equality of every stored field.
use super::*;
use crate::AnimatorCheckpointError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct RootRigidPathCheckpointLimits {
    pub max_bytes: usize,
    pub max_nodes: usize,
    pub max_spans: usize,
    /// Aggregate retained spans across all program nodes, not only the output.
    pub max_work_spans: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootRigidTwistStep {
    linear: [u64; 3],
    angular: [u64; 3],
    duration: u64,
}
impl RootRigidTwistStep {
    #[must_use]
    pub fn new(twist: RootRigidTwist, duration: f64) -> Self {
        Self {
            linear: twist.linear.to_array().map(f64::to_bits),
            angular: twist.angular.to_array().map(f64::to_bits),
            duration: duration.to_bits(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootRigidTimedTwistStep {
    twist: RootRigidTwistStep,
    start: u64,
    end: u64,
}
impl RootRigidTimedTwistStep {
    #[must_use]
    pub fn new(twist: RootRigidTwist, start: f64, end: f64) -> Self {
        Self {
            twist: RootRigidTwistStep::new(twist, end - start),
            start: start.to_bits(),
            end: end.to_bits(),
        }
    }
}
/// Flat backward references avoid recursive input graphs. All floating data are bits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RootRigidPathStep {
    Curve {
        revision: String,
        start_bits: u64,
        end_bits: u64,
        axes: [bool; 3],
    },
    Twists {
        segments: Vec<RootRigidTwistStep>,
    },
    TimedTwists {
        segments: Vec<RootRigidTimedTwistStep>,
        duration_bits: u64,
    },
    Retimed {
        source: usize,
        duration_bits: u64,
    },
    Transformed {
        source: usize,
        basis_bits: [u64; 4],
        scale_bits: u64,
        offset_bits: [u64; 3],
    },
    Append {
        first: usize,
        second: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootRigidPathProgram {
    steps: Vec<RootRigidPathStep>,
}
impl RootRigidPathProgram {
    /// Output is the last node. References must address earlier nodes.
    #[must_use]
    pub fn new(steps: Vec<RootRigidPathStep>) -> Self {
        Self { steps }
    }
    fn replay(
        &self,
        resolve: impl Fn(&str) -> Option<RootRigidCurve>,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<RootRigidPath, AnimatorCheckpointError> {
        if self.steps.is_empty()
            || self.steps.len() > limits.max_nodes
            || limits.max_spans > MAX_ROOT_ROTATION_SPANS
        {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let mut values = Vec::<RootRigidPath>::new();
        let mut curves = std::collections::HashMap::new();
        let mut work = 0usize;
        for step in &self.steps {
            let get = |index: usize| values.get(index).ok_or(AnimatorCheckpointError::State);
            let path = match step {
                RootRigidPathStep::Curve {
                    revision,
                    start_bits,
                    end_bits,
                    axes,
                } => {
                    if revision.is_empty() {
                        return Err(AnimatorCheckpointError::State);
                    }
                    if !curves.contains_key(revision) {
                        curves.insert(
                            revision.clone(),
                            resolve(revision).ok_or(AnimatorCheckpointError::State)?,
                        );
                    }
                    curves[revision].path(
                        f64::from_bits(*start_bits),
                        f64::from_bits(*end_bits),
                        *axes,
                        limits.max_spans,
                    )?
                }
                RootRigidPathStep::Twists { segments } => {
                    if segments.len() > limits.max_spans {
                        return Err(AnimatorCheckpointError::Capacity);
                    }
                    let segments = segments
                        .iter()
                        .map(|s| {
                            (
                                RootRigidTwist {
                                    linear: DVec3::from_array(s.linear.map(f64::from_bits)),
                                    angular: DVec3::from_array(s.angular.map(f64::from_bits)),
                                },
                                f64::from_bits(s.duration),
                            )
                        })
                        .collect::<Vec<_>>();
                    RootRigidPath::from_twists(&segments, limits.max_spans)?
                }
                RootRigidPathStep::TimedTwists {
                    segments,
                    duration_bits,
                } => {
                    if segments.len() > limits.max_spans {
                        return Err(AnimatorCheckpointError::Capacity);
                    }
                    let duration = f64::from_bits(*duration_bits);
                    if !duration.is_finite() || duration < 0. {
                        return Err(AnimatorCheckpointError::State);
                    }
                    let mut spans = Vec::with_capacity(segments.len());
                    let mut prefix = RootRigidTransform::IDENTITY;
                    let mut cursor = 0.;
                    for segment in segments {
                        let start = f64::from_bits(segment.start);
                        let end = f64::from_bits(segment.end);
                        if !start.is_finite()
                            || !end.is_finite()
                            || start != cursor
                            || end <= start
                            || end > duration
                            || (end - start).to_bits() != segment.twist.duration
                        {
                            return Err(AnimatorCheckpointError::State);
                        }
                        let twist = RootRigidTwist {
                            linear: DVec3::from_array(segment.twist.linear.map(f64::from_bits)),
                            angular: DVec3::from_array(segment.twist.angular.map(f64::from_bits)),
                        };
                        let span = RootRigidSpan::constant_spatial(twist, prefix, start, end)?;
                        prefix = span.sample(1.)?;
                        spans.push(span);
                        cursor = end;
                    }
                    if cursor != duration {
                        return Err(AnimatorCheckpointError::State);
                    }
                    RootRigidPath {
                        spans,
                        duration,
                        end: prefix,
                        translation_cuts: vec![],
                    }
                }
                RootRigidPathStep::Retimed {
                    source,
                    duration_bits,
                } => get(*source)?.retimed(f64::from_bits(*duration_bits))?,
                RootRigidPathStep::Transformed {
                    source,
                    basis_bits,
                    scale_bits,
                    offset_bits,
                } => get(*source)?.transformed(
                    DQuat::from_array(basis_bits.map(f64::from_bits)),
                    f64::from_bits(*scale_bits),
                    DVec3::from_array(offset_bits.map(f64::from_bits)),
                )?,
                RootRigidPathStep::Append { first, second } => {
                    get(*first)?.append_spatial(get(*second)?, limits.max_spans)?
                }
            };
            work = work
                .checked_add(path.spans.len())
                .ok_or(AnimatorCheckpointError::Capacity)?;
            if work > limits.max_work_spans {
                return Err(AnimatorCheckpointError::Capacity);
            }
            values.push(path);
        }
        values.pop().ok_or(AnimatorCheckpointError::State)
    }
}
/// Nonexecutable equality record for paths rebuilt by another verified owner recipe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RootRigidPathStateRecord(Vec<u64>);
impl RootRigidPathStateRecord {
    #[must_use]
    pub fn matches(&self, path: &RootRigidPath) -> bool {
        self.0 == path.checkpoint_words()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootRigidPathCheckpoint {
    version: u32,
    program: RootRigidPathProgram,
    words: Vec<u64>,
}
impl RootRigidPath {
    #[must_use]
    pub fn state_record(&self) -> RootRigidPathStateRecord {
        RootRigidPathStateRecord(self.checkpoint_words())
    }

    /// Capture a canonical ordered screw path without accumulating rounded durations.
    /// # Errors
    /// Rejects nonscrew/noncanonical paths and checkpoint admission budgets.
    pub fn checkpoint_timed_screws(
        &self,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<RootRigidPathCheckpoint, AnimatorCheckpointError> {
        let segments = self
            .spans
            .iter()
            .map(|span| {
                let (twist, _) = span.screw.ok_or(AnimatorCheckpointError::State)?;
                Ok(RootRigidTimedTwistStep::new(
                    twist,
                    span.start(),
                    span.end(),
                ))
            })
            .collect::<Result<Vec<_>, AnimatorCheckpointError>>()?;
        let program = RootRigidPathProgram::new(vec![RootRigidPathStep::TimedTwists {
            segments,
            duration_bits: self.duration.to_bits(),
        }]);
        self.checkpoint(program, |_| None, limits)
    }
    pub(crate) fn checkpoint_words(&self) -> Vec<u64> {
        let mut words = vec![
            self.duration.to_bits(),
            self.spans.len() as u64,
            self.translation_cuts.len() as u64,
        ];
        words.extend(self.end.translation.to_array().map(f64::to_bits));
        words.extend(self.end.rotation.to_array().map(f64::to_bits));
        for span in &self.spans {
            span.rotation.checkpoint_words(&mut words);
            words.push(u64::from(span.screw.is_some()));
            if let Some((twist, initial)) = span.screw {
                words.extend(twist.linear.to_array().map(f64::to_bits));
                words.extend(twist.angular.to_array().map(f64::to_bits));
                words.extend(initial.translation.to_array().map(f64::to_bits));
                words.extend(initial.rotation.to_array().map(f64::to_bits));
            }
            for vector in span.additive.into_iter().chain(span.pivot) {
                words.extend(vector.to_array().map(f64::to_bits));
            }
        }
        for cut in &self.translation_cuts {
            words.push(cut.evaluated().to_bits());
            words.extend(cut.exact_source_bounds().map(f64::to_bits));
            words.push(cut.absolute_error_bound().to_bits());
        }
        words
    }
    /// Capture requires a reproducible program and verified immutable curve keys.
    /// No replay data or tracking overhead is added to ordinary runtime paths.
    /// # Errors
    /// Rejects a nonmatching program, unresolved resources and admission budgets.
    pub fn checkpoint(
        &self,
        program: RootRigidPathProgram,
        resolve: impl Fn(&str) -> Option<RootRigidCurve>,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<RootRigidPathCheckpoint, AnimatorCheckpointError> {
        if self.spans.len() > limits.max_spans {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let replayed = program.replay(resolve, limits)?;
        let words = self.checkpoint_words();
        if words != replayed.checkpoint_words() {
            return Err(AnimatorCheckpointError::State);
        }
        let saved = RootRigidPathCheckpoint {
            version: 1,
            program,
            words,
        };
        saved.to_json(limits.max_bytes)?;
        Ok(saved)
    }
}
impl RootRigidPathCheckpoint {
    /// # Errors
    /// Rejects unsupported versions and oversized output.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, AnimatorCheckpointError> {
        if self.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// The enclosing owner must bound file IO and verify curve revision identity.
    /// # Errors
    /// Rejects oversized input before parsing, unknown fields and unsupported versions.
    pub fn from_json(text: &str, max_bytes: usize) -> Result<Self, AnimatorCheckpointError> {
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(saved)
    }
    /// Replay existing constructors and require exact equality of coefficients,
    /// ordered spans, STEP events, cached bounds, endpoint and cut error metadata.
    /// Stored words are an equality record, never an executable error certificate.
    /// # Errors
    /// Rejects changed resources/results, invalid programs and node/span/byte budgets.
    pub fn prepare_with_program(
        &self,
        expected: &RootRigidPathProgram,
        resolve: impl Fn(&str) -> Option<RootRigidCurve>,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<RootRigidPath, AnimatorCheckpointError> {
        if &self.program != expected {
            return Err(AnimatorCheckpointError::State);
        }
        self.prepare(resolve, limits)
    }
    /// Replay and validate the stored construction program.
    /// # Errors
    /// Rejects unsupported state, incompatible results and admission limits.
    pub fn prepare(
        &self,
        resolve: impl Fn(&str) -> Option<RootRigidCurve>,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<RootRigidPath, AnimatorCheckpointError> {
        self.to_json(limits.max_bytes)?;
        let path = self.program.replay(resolve, limits)?;
        if path.checkpoint_words() != self.words {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> RootRigidPathCheckpointLimits {
        RootRigidPathCheckpointLimits {
            max_bytes: 1024 * 1024,
            max_nodes: 16,
            max_spans: 64,
            max_work_spans: 256,
        }
    }
    #[test]
    fn ordered_twists_retime_transform_and_append_replay_exactly() {
        let twist = RootRigidTwist {
            linear: DVec3::new(0.1, -0.2, 0.3),
            angular: DVec3::new(0.3, 0.4, -0.1),
        };
        let base = RootRigidPath::from_twists(&[(twist, 0.13), (twist, 0.27)], 64).unwrap();
        let basis = DQuat::from_rotation_y(0.731);
        let offset = DVec3::new(0.1, 0.2, -0.3);
        let path = base
            .retimed(0.8)
            .unwrap()
            .transformed(basis, 1.3, offset)
            .unwrap()
            .append_spatial(&base, 64)
            .unwrap();
        let program = RootRigidPathProgram::new(vec![
            RootRigidPathStep::Twists {
                segments: vec![
                    RootRigidTwistStep::new(twist, 0.13),
                    RootRigidTwistStep::new(twist, 0.27),
                ],
            },
            RootRigidPathStep::Retimed {
                source: 0,
                duration_bits: 0.8f64.to_bits(),
            },
            RootRigidPathStep::Transformed {
                source: 1,
                basis_bits: basis.to_array().map(f64::to_bits),
                scale_bits: 1.3f64.to_bits(),
                offset_bits: offset.to_array().map(f64::to_bits),
            },
            RootRigidPathStep::Append {
                first: 2,
                second: 0,
            },
        ]);
        let saved = path.checkpoint(program, |_| None, limits()).unwrap();
        let text = saved.to_json(limits().max_bytes).unwrap();
        let decoded = RootRigidPathCheckpoint::from_json(&text, limits().max_bytes).unwrap();
        let restored = decoded.prepare(|_| None, limits()).unwrap();
        assert_eq!(path.checkpoint_words(), restored.checkpoint_words());
        for (a, b) in path.spans().iter().zip(restored.spans()) {
            for index in 0..=120 {
                assert_eq!(
                    a.sample(f64::from(index) / 120.).unwrap(),
                    b.sample(f64::from(index) / 120.).unwrap()
                );
            }
        }
        let mut corrupt = decoded.clone();
        corrupt.words[0] ^= 1;
        assert!(corrupt.prepare(|_| None, limits()).is_err());
        let mut corrupt = decoded.clone();
        corrupt.program.steps[1] = RootRigidPathStep::Retimed {
            source: 99,
            duration_bits: 1f64.to_bits(),
        };
        assert!(corrupt.prepare(|_| None, limits()).is_err());
        let mut small = limits();
        small.max_work_spans = 1;
        assert!(decoded.prepare(|_| None, small).is_err());
        assert!(RootRigidPathCheckpoint::from_json(&text, 1).is_err());
    }
}
