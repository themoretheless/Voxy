//! Scene-box collision backend for the native Rush character demonstration.
use physics::{AnchoredAabb, CollisionWorld, SweepResult};
#[derive(Debug)]
pub(crate) struct ScriptCollisionWorld {
    pub boxes: Vec<(String, [f64; 3], [f64; 3])>,
}
impl CollisionWorld for ScriptCollisionWorld {
    type Obstacle = String;
    type Error = std::convert::Infallible;
    fn sweep_aabb(
        &self,
        body: AnchoredAabb,
        displacement: [f64; 3],
        _: usize,
    ) -> Result<SweepResult<String>, Self::Error> {
        let mut result = SweepResult {
            fraction: 1.0,
            normal: [0; 3],
            obstacle: None,
        };
        let anchor = [
            body.anchor.x as f64,
            body.anchor.y as f64,
            body.anchor.z as f64,
        ];
        for (name, min, max) in &self.boxes {
            // A touching face is not an initial overlap when travelling away or
            // parallel to it. Filter it before the shared swept-box primitive.
            if (0..3).any(|i| {
                (body.max[i] <= min[i] - anchor[i] && displacement[i] <= 0.0)
                    || (body.min[i] >= max[i] - anchor[i] && displacement[i] >= 0.0)
            }) {
                continue;
            }
            if let Some((fraction, normal)) = physics::sweep_box(
                body,
                displacement,
                std::array::from_fn(|i| min[i] - anchor[i]),
                std::array::from_fn(|i| max[i] - anchor[i]),
            ) {
                if fraction < result.fraction
                    || (fraction == result.fraction && result.obstacle.is_none())
                {
                    result = SweepResult {
                        fraction,
                        normal,
                        obstacle: Some(name.clone()),
                    };
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn touching_box_allows_moving_away_and_parallel_but_blocks_approach() {
        let world = ScriptCollisionWorld {
            boxes: vec![("wall".into(), [1.5, -0.5, -0.5], [2.5, 0.5, 0.5])],
        };
        let body = AnchoredAabb {
            anchor: physics::Origin { x: 0, y: 0, z: 0 },
            min: [0.5, -0.5, -0.5],
            max: [1.5, 0.5, 0.5],
        };
        assert!(
            world
                .sweep_aabb(body, [-0.1, 0.0, 0.0], 10)
                .unwrap()
                .obstacle
                .is_none()
        );
        assert!(
            world
                .sweep_aabb(body, [0.0, 0.0, 0.1], 10)
                .unwrap()
                .obstacle
                .is_none()
        );
        let hit = world.sweep_aabb(body, [0.1, 0.0, 0.0], 10).unwrap();
        assert_eq!(hit.fraction, 0.0);
        assert_eq!(hit.normal, [-1, 0, 0]);
    }
}
