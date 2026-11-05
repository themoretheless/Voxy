//! Follows the same interpolated world pose used by render extraction.
use crate::*;
use voxy_gameplay::{FollowCamera, FollowSmoother};
#[derive(Clone, Debug)]
pub(super) struct FollowRuntime {
    owner: NodeId,
    target: NodeId,
    descriptor: FollowCamera,
    smoother: FollowSmoother,
}
impl App {
    pub(super) fn update_follow_camera(
        &mut self,
        elapsed: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(simulation) = &self.play.simulation else {
            return Ok(());
        };
        let mut cameras = self.scene.active_components::<FollowCamera>();
        let selected = cameras
            .next()
            .map(|(owner, descriptor)| (owner, descriptor.clone()));
        if cameras.next().is_some() {
            return Err("multiple active follow cameras require explicit camera selection".into());
        }
        drop(cameras);
        let Some((owner, descriptor)) = selected else {
            self.play.follow_camera = None;
            return Ok(());
        };
        if !descriptor.valid() {
            return Err("invalid game.follow-camera.v1 descriptor".into());
        }
        let camera = self
            .scene
            .component::<EditorCamera>(owner)?
            .ok_or("follow camera requires an editor.camera.v1 view")?;
        if !camera.valid() {
            return Err("invalid follow camera view".into());
        }
        let target = self
            .object_ids
            .iter()
            .zip(&self.instances)
            .find_map(|(id, node)| (id == &descriptor.target).then_some(*node))
            .ok_or("follow target is not in the loaded scene")?;
        let world = simulation
            .render_world(&self.scene, target)
            .map_err(|error| {
                format!(
                    "follow camera target {} is unavailable: {error}",
                    descriptor.target.0
                )
            })?;
        let desired =
            world.w_axis.truncate().as_dvec3() + Vec3::from_array(descriptor.offset).as_dvec3();
        let prior = self.play.follow_camera.as_ref().filter(|state| {
            state.owner == owner && state.target == target && state.descriptor == descriptor
        });
        let (mut state, mut view) = if let Some(prior) = prior {
            (prior.clone(), self.camera.clone())
        } else {
            (
                FollowRuntime {
                    owner,
                    target,
                    descriptor: descriptor.clone(),
                    smoother: FollowSmoother::new(desired)?,
                },
                camera.clone(),
            )
        };
        view.target = state
            .smoother
            .advance(desired, descriptor.half_life, elapsed)?
            .as_vec3();
        view.legacy = false;
        if !view.valid() {
            return Err("follow camera target exceeds view bounds".into());
        }
        self.camera = view;
        self.play.follow_camera = Some(state);
        Ok(())
    }
}

/// Presentation participant; publication belongs to the enclosing session transaction.
#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FollowRuntimeCheckpoint {
    version: u32,
    owner: ObjectId,
    target: ObjectId,
    descriptor: FollowCamera,
    smoother: voxy_gameplay::FollowSmootherCheckpoint,
    view: crate::camera::ViewportCamera,
}
#[allow(dead_code)]
impl FollowRuntime {
    pub(crate) fn checkpoint(
        &self,
        view: &crate::camera::ViewportCamera,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<FollowRuntimeCheckpoint, EditorError> {
        if !view.valid() || !self.descriptor.valid() {
            return Err("invalid follow camera checkpoint".into());
        }
        let owner = identify(self.owner).ok_or("unidentified follow camera owner")?;
        let target = identify(self.target).ok_or("unidentified follow camera target")?;
        if owner.0.is_empty() || target != self.descriptor.target {
            return Err("follow camera checkpoint identity mismatch".into());
        }
        Ok(FollowRuntimeCheckpoint {
            version: 1,
            owner,
            target,
            descriptor: self.descriptor.clone(),
            smoother: self.smoother.checkpoint(),
            view: view.clone(),
        })
    }
}
#[allow(dead_code)]
impl FollowRuntimeCheckpoint {
    pub(crate) fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    ) -> Result<(FollowRuntime, crate::camera::ViewportCamera), EditorError> {
        if self.version != 1
            || !self.view.valid()
            || !self.descriptor.valid()
            || self.target != self.descriptor.target
        {
            return Err("invalid follow runtime checkpoint".into());
        }
        let owner = resolve(&self.owner).ok_or("missing follow camera owner")?;
        let target = resolve(&self.target).ok_or("missing follow camera target")?;
        // Validate liveness and typed owner compatibility before returning a candidate.
        scene.world_matrix(target)?;
        let descriptor = scene
            .component::<FollowCamera>(owner)?
            .ok_or("missing follow camera descriptor")?;
        let view = scene
            .component::<EditorCamera>(owner)?
            .ok_or("missing follow camera view")?;
        if descriptor != &self.descriptor || !view.valid() {
            return Err("incompatible follow camera descriptor".into());
        }
        Ok((
            FollowRuntime {
                owner,
                target,
                descriptor: self.descriptor.clone(),
                smoother: self.smoother.prepare()?,
            },
            self.view.clone(),
        ))
    }
}

/// Frame-rate-independent exponential decay step toward `target` governed by `half_life_s`.
#[must_use]
pub fn follow_camera_spring_step(current: f64, target: f64, half_life_s: f64, dt_s: f64) -> f64 {
    if !current.is_finite() || !target.is_finite() || !dt_s.is_finite() || dt_s <= 0.0 {
        return current;
    }
    if !half_life_s.is_finite() || half_life_s <= 0.0 {
        return target;
    }
    let decay = (-std::f64::consts::LN_2 * dt_s / half_life_s).exp();
    target + (current - target) * decay
}

#[cfg(test)]
mod spring_step_tests {
    use super::*;

    #[test]
    fn follow_camera_spring_step_halves_distance_every_half_life() {
        let stepped = follow_camera_spring_step(0.0, 10.0, 0.25, 0.25);
        assert!((stepped - 5.0).abs() < 1e-12);
        let two_half_lives = follow_camera_spring_step(stepped, 10.0, 0.25, 0.25);
        assert!((two_half_lives - 7.5).abs() < 1e-12);
    }
}
