use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum SavedShape {
    Hold([u64; 4]),
    Arc { from: [u64; 4], axis: [u64; 3] },
    Cubic([[u64; 4]; 4]),
    Step { from: [u64; 4], axis: [u64; 3] },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedKnot {
    time: u64,
    source: [u32; 4],
    value: [u64; 4],
    shape: SavedShape,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootRotationCheckpoint {
    knots: Vec<SavedKnot>,
    fallback: [u64; 4],
    duration: u64,
    playback: u8,
    mode: u8,
    origin: [u64; 4],
    cycle: [u64; 4],
    constant: bool,
    source_constant: bool,
    key_normalization_error: [u64; 4],
    cubic_control_error: Option<[u64; 4]>,
}
impl RootRotationCurve {
    pub(crate) fn checkpoint(&self) -> RootRotationCheckpoint {
        let Curve {
            knots,
            fallback,
            duration,
            playback,
            mode,
            origin,
            cycle,
            constant,
            source_constant,
            key_normalization_error,
            cubic_control_error,
        } = self.0.as_ref();
        RootRotationCheckpoint {
            knots: knots
                .iter()
                .map(|k| {
                    let Knot {
                        time,
                        source,
                        value,
                        shape,
                    } = k;
                    let shape = match shape {
                        Shape::Hold(q) => SavedShape::Hold(q.to_array().map(f64::to_bits)),
                        Shape::Arc { from, axis } => SavedShape::Arc {
                            from: from.to_array().map(f64::to_bits),
                            axis: axis.to_array().map(f64::to_bits),
                        },
                        Shape::Cubic(control) => {
                            SavedShape::Cubic(control.map(|v| v.to_array().map(f64::to_bits)))
                        }
                        Shape::Step { from, axis } => SavedShape::Step {
                            from: from.to_array().map(f64::to_bits),
                            axis: axis.to_array().map(f64::to_bits),
                        },
                    };
                    SavedKnot {
                        time: time.to_bits(),
                        source: source.map(f32::to_bits),
                        value: value.to_array().map(f64::to_bits),
                        shape,
                    }
                })
                .collect(),
            fallback: fallback.to_array().map(f64::to_bits),
            duration: duration.to_bits(),
            playback: crate::asset_checkpoint::playback(*playback),
            mode: crate::asset_checkpoint::mode(*mode),
            origin: origin.to_array().map(f64::to_bits),
            cycle: cycle.to_array().map(f64::to_bits),
            constant: *constant,
            source_constant: *source_constant,
            key_normalization_error: key_normalization_error.map(f64::to_bits),
            cubic_control_error: cubic_control_error.map(|v| v.map(f64::to_bits)),
        }
    }
}
