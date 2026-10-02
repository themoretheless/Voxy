use super::{Renderer, RendererError};
use crate::skinned_lod_gpu::GpuSkinnedLod;
use crate::{LodPolicy, PreparedSkinnedLod, SceneCamera, SkinnedLodGpuError, SkinnedLodResidency};
use glam::Mat4;
#[cfg(test)]
mod tests;

impl Renderer {
    /// Replaces the animated mesh with base-only LOD residency. Budget includes
    /// the old-plus-new skeletal allocation peak, not other renderer resources.
    /// A rejected replacement leaves the previous mesh and history intact.
    /// # Errors
    /// Rejects device loss, insufficient budget or invalid skeletal upload.
    pub fn upload_skinned_lod(
        &mut self,
        pose: PreparedSkinnedLod,
        material_layer: u32,
        budget: u64,
    ) -> Result<(), RendererError> {
        self.check_device()?;
        let live = self.skinned_lod.as_ref().map_or_else(
            || {
                self.skinned
                    .as_ref()
                    .map_or(0, super::GpuSkinnedMesh::allocation_bytes)
            },
            |lod| lod.bytes,
        );
        let (gpu, lod) = GpuSkinnedLod::upload(
            &self.device,
            &self.skin_layout,
            pose,
            material_layer,
            live,
            budget,
        )
        .map_err(RendererError::SkinnedLod)?;
        let motion = crate::skinned_motion::ResidentSkinnedMotion {
            history: crate::SkinnedMotionHistory::new(lod.pose.source().mesh().clone()),
            joints: lod.pose.joints().to_vec(),
            model: lod.pose.model(),
        };
        self.skinned = Some(gpu);
        self.skinned_lod = Some(lod);
        self.skinned_motion = Some(motion);
        self.reset_skinned_motion();
        Ok(())
    }

    /// Allocates only one missing index buffer; all skeletal streams stay shared.
    /// # Errors
    /// Rejects invalid levels, device loss or budget overflow before allocation.
    pub fn ensure_skinned_lod_level(
        &mut self,
        level: usize,
        budget: u64,
    ) -> Result<bool, RendererError> {
        self.check_device()?;
        self.skinned_lod
            .as_mut()
            .ok_or(RendererError::NoSkinnedLod)?
            .ensure(&self.device, level, budget)
            .map_err(RendererError::SkinnedLod)
    }

    /// Selects from the current pose using this renderer's actual camera/viewport.
    /// Missing levels fall back to the closest resident finer level. Rendering
    /// re-evaluates the stored policy after camera/viewport changes.
    /// Bounds certify CPU-skinned geometry, not GPU arithmetic/image equality.
    /// # Errors
    /// Rejects invalid camera/policy/history without changing the active draw.
    pub fn select_skinned_lod(
        &mut self,
        policy: LodPolicy,
        previous: Option<usize>,
    ) -> Result<usize, RendererError> {
        self.check_device()?;
        let camera = self.skinned_lod_camera();
        let viewport = [self.config.width, self.config.height];
        let lod = self
            .skinned_lod
            .as_ref()
            .ok_or(RendererError::NoSkinnedLod)?;
        let desired = lod
            .pose
            .select_for_camera(camera, viewport, policy, previous)
            .map_err(|error| RendererError::SkinnedLod(SkinnedLodGpuError::Policy(error)))?;
        let selected = lod.fallback(desired).map_err(RendererError::SkinnedLod)?;
        self.bind_skinned_lod_level(selected)?;
        self.skinned_lod
            .as_mut()
            .ok_or(RendererError::NoSkinnedLod)?
            .policy = Some(policy);
        Ok(selected)
    }

    /// Prepares quality and selection before publishing palette/model changes.
    /// Use this atomic path for animated LOD instead of independent low-level
    /// updates, which conservatively return to base. Same-level updates preserve
    /// temporal correspondence; a topology change resets it before the next draw.
    /// # Errors
    /// Rejects invalid pose/camera/policy/history without modifying GPU buffers.
    pub fn update_skinned_lod_pose(
        &mut self,
        joints: &[Mat4],
        model: Mat4,
        policy: LodPolicy,
        previous: Option<usize>,
    ) -> Result<usize, RendererError> {
        self.check_device()?;
        let camera = self.skinned_lod_camera();
        let viewport = [self.config.width, self.config.height];
        let lod = self
            .skinned_lod
            .as_ref()
            .ok_or(RendererError::NoSkinnedLod)?;
        let pose = lod
            .pose
            .source()
            .prepare(joints, model)
            .map_err(|error| RendererError::SkinnedLod(SkinnedLodGpuError::Pose(error)))?;
        let desired = pose
            .select_for_camera(camera, viewport, policy, previous)
            .map_err(|error| RendererError::SkinnedLod(SkinnedLodGpuError::Policy(error)))?;
        let selected = lod.fallback(desired).map_err(RendererError::SkinnedLod)?;
        let gpu = self.skinned.as_ref().ok_or(RendererError::NoSkinnedMesh)?;
        self.queue
            .write_buffer(&gpu.joint_buffer, 0, bytemuck::cast_slice(pose.joints()));
        self.queue.write_buffer(
            &gpu.object_buffer,
            0,
            bytemuck::bytes_of(&crate::skinned::object_uniform(
                pose.model(),
                gpu.material_layer,
            )),
        );
        if let Some(motion) = &mut self.skinned_motion {
            motion.joints.clone_from_slice(pose.joints());
            motion.model = pose.model();
        }
        self.skinned_lod
            .as_mut()
            .ok_or(RendererError::NoSkinnedLod)?
            .pose = pose;
        self.skinned_lod
            .as_mut()
            .ok_or(RendererError::NoSkinnedLod)?
            .policy = Some(policy);
        self.bind_skinned_lod_level(selected)?;
        Ok(selected)
    }

    /// Evicts optional indices. An active level first returns to resident base
    /// and resets temporal history; base/vertex/palette buffers stay pinned.
    /// # Errors
    /// Rejects base/out-of-range levels and device loss without mutation.
    pub fn evict_skinned_lod_level(&mut self, level: usize) -> Result<bool, RendererError> {
        self.check_device()?;
        let lod = self
            .skinned_lod
            .as_ref()
            .ok_or(RendererError::NoSkinnedLod)?;
        if level == 0 || level >= lod.levels.len() {
            return Err(RendererError::SkinnedLod(SkinnedLodGpuError::InvalidLevel));
        }
        if lod.selected == level {
            self.bind_skinned_lod_level(0)?;
        }
        self.skinned_lod
            .as_mut()
            .ok_or(RendererError::NoSkinnedLod)?
            .evict(level)
            .map_err(RendererError::SkinnedLod)
    }

    #[must_use]
    pub fn skinned_lod_residency(&self) -> Option<SkinnedLodResidency> {
        self.skinned_lod.as_ref().map(GpuSkinnedLod::residency)
    }

    fn skinned_lod_camera(&self) -> SceneCamera {
        let [x, y] = super::camera_half_extents(self.config.width, self.config.height, self.camera);
        SceneCamera {
            eye: self.camera.eye,
            target: self.camera.target,
            up: self.camera.up,
            projection: crate::SceneProjection::Orthographic {
                left: -x,
                right: x,
                bottom: -y,
                top: y,
                near: self.camera.near_plane,
                far: 512.,
            },
        }
    }

    pub(super) fn refresh_skinned_lod(&mut self) -> Result<(), RendererError> {
        if let Some(lod) = &self.skinned_lod
            && let Some(policy) = lod.policy
        {
            let previous = Some(lod.selected);
            self.select_skinned_lod(policy, previous)?;
        }
        Ok(())
    }

    pub(super) fn bind_skinned_lod_level(&mut self, level: usize) -> Result<(), RendererError> {
        let lod = self
            .skinned_lod
            .as_mut()
            .ok_or(RendererError::NoSkinnedLod)?;
        let gpu = self.skinned.as_mut().ok_or(RendererError::NoSkinnedMesh)?;
        if lod
            .bind(level, gpu, &mut self.skinned_motion)
            .map_err(RendererError::SkinnedLod)?
        {
            self.reset_skinned_motion();
        }
        Ok(())
    }
}
