//! Continuous companion sweeps against terrain and other active bodies.
use super::*;
pub struct AnimalWorld<'a> {
    pub world: &'a World,
    pub bodies: &'a [AnchoredAabb],
}
impl physics::CollisionWorld for AnimalWorld<'_> {
    type Obstacle = ();
    type Error = String;
    fn sweep_aabb(
        &self,
        body: physics::AnchoredAabb,
        displacement: [f64; 3],
        budget: usize,
    ) -> Result<physics::SweepResult<()>, String> {
        let voxel = physics_voxel::VoxelCollisionWorld {
            view: self.world,
            registry: self.world.registry(),
        };
        let hit = voxel
            .sweep_aabb(body, displacement, budget)
            .map_err(|e| e.to_string())?;
        let mut result = physics::SweepResult {
            fraction: hit.fraction,
            normal: hit.normal,
            obstacle: hit.obstacle.map(|_| ()),
        };
        'obstacle: for obstacle in self.bodies {
            let anchor = [obstacle.anchor.x, obstacle.anchor.y, obstacle.anchor.z];
            let origin = [body.anchor.x, body.anchor.y, body.anchor.z];
            let mut min = [0.0; 3];
            let mut max = [0.0; 3];
            for i in 0..3 {
                let offset = anchor[i]
                    .checked_sub(origin[i])
                    .ok_or("animal collision anchor overflow")?;
                if offset.unsigned_abs() > 1_000_000 {
                    continue 'obstacle;
                }
                min[i] = offset as f64 + obstacle.min[i];
                max[i] = offset as f64 + obstacle.max[i];
            }
            if let Some((fraction, normal)) = physics::sweep_box(body, displacement, min, max) {
                if fraction < result.fraction {
                    result = physics::SweepResult {
                        fraction,
                        normal,
                        obstacle: Some(()),
                    };
                }
            }
        }
        Ok(result)
    }
}
