//! Durable authored loads; no runtime velocity or solver state is serialized.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigidPointLoads {
    pub points: Vec<RigidPointForce>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigidPointForce {
    pub root_point_m: [f64; 3],
    pub world_force_n: [f64; 3],
    pub world_force_rate_n_s: [f64; 3],
}
impl RigidPointLoads {
    /// Convert authored root points and rebase each original force recipe to
    /// an explicitly supplied elapsed time. Limits are owned by the caller.
    pub fn prepare(
        &self,
        frame: crate::RigidBodyFrame,
        elapsed_s: f64,
        max_points: usize,
    ) -> Result<Vec<physics::rigid_motion::MaterialPointForce>, String> {
        if self.points.len() > max_points || !elapsed_s.is_finite() || elapsed_s < 0. {
            return Err("invalid point-load time or point budget".into());
        }
        self.points
            .iter()
            .map(|point| {
                frame
                    .prepare_point_force(
                        point.root_point_m,
                        point.world_force_n,
                        point.world_force_rate_n_s,
                    )
                    .map_err(|e| format!("point-load frame: {e:?}"))?
                    .shifted(elapsed_s)
                    .map_err(|e| format!("point-load phase: {e:?}"))
            })
            .collect()
    }
}
