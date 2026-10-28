use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedKnot {
    time: u64,
    value: [u64; 3],
    area: [u64; 3],
    coefficients: [[u64; 3]; 4],
    compilation_error: Result<[u64; 3], crate::AnimationError>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootCurveCheckpoint {
    knots: Vec<SavedKnot>,
    duration: u64,
    playback: u8,
    mode: u8,
}
impl RootCurve {
    pub(crate) fn checkpoint(&self) -> RootCurveCheckpoint {
        let Self {
            knots,
            duration,
            playback,
            mode,
        } = self;
        RootCurveCheckpoint {
            knots: knots
                .iter()
                .map(|value| {
                    let Knot {
                        time,
                        value,
                        area,
                        coefficients,
                        compilation_error,
                    } = value;
                    SavedKnot {
                        time: time.to_bits(),
                        value: value.to_array().map(f64::to_bits),
                        area: area.to_array().map(f64::to_bits),
                        coefficients: coefficients.map(|v| v.to_array().map(f64::to_bits)),
                        compilation_error: compilation_error.clone().map(|v| v.map(f64::to_bits)),
                    }
                })
                .collect(),
            duration: duration.to_bits(),
            playback: crate::asset_checkpoint::playback(*playback),
            mode: crate::asset_checkpoint::mode(*mode),
        }
    }
}
