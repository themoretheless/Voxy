//! Stateless pose sampling, skin matrix palettes, and CPU/f64 surface deformation.

use glam::{DMat4, DVec3, Mat4, Vec3};
use voxy_render::SceneMesh;

use super::{ModelAsset, ModelError, ModelGeometry, ModelSurface64, fail};

impl ModelAsset {
    /// Samples an explicitly selected clip without retaining playback state in
    /// the shared asset. Each scene instance owns its own clip and clock.
    /// `None` selects the bind pose. Finite negative times retain the clip's
    /// authored loop/clamp behavior.
    /// # Errors
    /// Rejects non-finite time, an absent clip index and invalid interpolated local TRS.
    pub fn sample_pose(
        &self,
        clip: Option<usize>,
        time: f32,
    ) -> Result<voxy_animation::Pose, ModelError> {
        if !time.is_finite() {
            return Err(fail("animation time must be finite"));
        }
        match clip {
            None => Ok(self.skeleton.bind_pose()),
            Some(index) => self
                .animations
                .get(index)
                .ok_or_else(|| fail("animation clip index out of range"))?
                .try_sample(&self.skeleton, time)
                .map_err(|error| fail(error.to_string())),
        }
    }

    /// Stateless authored-phase preview, including the final pose of loop clips.
    /// # Errors
    /// Rejects invalid phase, missing clip and invalid sampled transforms.
    pub fn sample_pose_phase(
        &self,
        clip: Option<usize>,
        phase: f64,
    ) -> Result<voxy_animation::Pose, ModelError> {
        if !phase.is_finite() || !(0. ..=1.).contains(&phase) {
            return Err(fail("animation phase must be between zero and one"));
        }
        match clip {
            None => Ok(self.skeleton.bind_pose()),
            Some(index) => self
                .animations
                .get(index)
                .ok_or_else(|| fail("animation clip index out of range"))?
                .try_sample_phase(&self.skeleton, phase)
                .map_err(|error| fail(error.to_string())),
        }
    }

    /// Returns the mesh node's global transform for a sampled pose.
    /// # Errors
    /// Rejects a pose with the wrong joint count.
    pub fn mesh_transform(&self, pose: &voxy_animation::Pose) -> Result<Mat4, ModelError> {
        if pose.local().len() != self.skeleton.joints().len() {
            return Err(fail("pose count mismatch"));
        }
        let mut matrix = pose.local()[self.mesh_joint].matrix();
        let mut parent = self.skeleton.joints()[self.mesh_joint].parent;
        while let Some(index) = parent {
            matrix = pose.local()[usize::from(index)].matrix() * matrix;
            parent = self.skeleton.joints()[usize::from(index)].parent;
        }
        Ok(matrix)
    }

    /// Sample existing authored keys for physical kinematics without f32 time rounding.
    pub fn sample_pose_phase64(
        &self,
        clip: Option<usize>,
        phase: f64,
    ) -> Result<voxy_animation::Pose64, ModelError> {
        if !phase.is_finite() || !(0. ..=1.).contains(&phase) {
            return Err(fail("animation phase must be between zero and one"));
        }
        match clip {
            None => Ok(self.skeleton.bind_pose64()),
            Some(index) => self
                .animations
                .get(index)
                .ok_or_else(|| fail("animation clip index out of range"))?
                .try_sample_phase64(&self.skeleton, phase)
                .map_err(|e| fail(e.to_string())),
        }
    }

    /// Pose the same validated imported vertices and UNORM16 influences in f64.
    /// No source topology, clock, asset or rendering resource is duplicated.
    pub fn scene_surfaces64(
        &self,
        pose: &voxy_animation::Pose64,
    ) -> Result<Vec<ModelSurface64>, ModelError> {
        if self.has_morph() {
            return Err(fail("morph surfaces require explicit weights"));
        }
        self.scene_surfaces64_internal(pose, None)
    }

    pub(super) fn scene_surfaces64_internal(
        &self,
        pose: &voxy_animation::Pose64,
        overrides: Option<&[Option<Vec<[f64; 3]>>]>,
    ) -> Result<Vec<ModelSurface64>, ModelError> {
        let global = pose
            .global_matrices(&self.skeleton)
            .map_err(|e| fail(e.to_string()))?;
        let model = *global
            .get(self.mesh_joint)
            .ok_or_else(|| fail("mesh node outside pose"))?;
        let palette: Vec<_> = global
            .iter()
            .zip(self.skeleton.joints())
            .map(|(global, joint)| {
                *global * DMat4::from_cols_array(&joint.inverse_bind.to_cols_array().map(f64::from))
            })
            .collect();
        if palette.iter().any(|m| !m.is_finite()) {
            return Err(fail("nonfinite wide skin palette"));
        }
        self.primitives
            .iter()
            .enumerate()
            .map(|(primitive_index, primitive)| {
                let morphed = overrides
                    .and_then(|all| all.get(primitive_index))
                    .and_then(Option::as_ref);
                let (positions, indices) = match &primitive.geometry {
                    ModelGeometry::Static(mesh) => (
                        mesh.vertices()
                            .iter()
                            .enumerate()
                            .map(|(index, v)| {
                                model
                                    .transform_point3(DVec3::from_array(
                                        morphed.map_or(v.position.map(f64::from), |positions| {
                                            positions[index]
                                        }),
                                    ))
                                    .to_array()
                            })
                            .collect(),
                        mesh.indices().to_vec(),
                    ),
                    ModelGeometry::Skinned(mesh) => {
                        if palette.len() != usize::from(mesh.joint_count()) {
                            return Err(fail("wide skin palette count mismatch"));
                        }
                        let positions = mesh
                            .vertices()
                            .iter()
                            .enumerate()
                            .map(|(index, v)| {
                                let mut skin = DMat4::ZERO;
                                for (joint, weight) in v.joints.into_iter().zip(v.weights) {
                                    if weight != 0 {
                                        let matrix =
                                            palette.get(usize::from(joint)).ok_or_else(|| {
                                                fail("wide skin joint outside palette")
                                            })?;
                                        skin += *matrix * (f64::from(weight) / 65535.);
                                    }
                                }
                                Ok((skin
                                    * DVec3::from_array(
                                        morphed.map_or(v.position.map(f64::from), |positions| {
                                            positions[index]
                                        }),
                                    )
                                    .extend(1.))
                                .truncate()
                                .to_array())
                            })
                            .collect::<Result<Vec<_>, ModelError>>()?;
                        (positions, mesh.indices().to_vec())
                    }
                };
                if positions.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(fail("nonfinite wide posed vertex"));
                }
                Ok(ModelSurface64 { positions, indices })
            })
            .collect()
    }

    /// Palette for `Renderer::upload_skinned_mesh`. Use the instance transform as
    /// `model`: the palette already includes the glTF node hierarchy.
    /// # Errors
    /// Rejects static assets and invalid poses.
    pub fn skin_matrices(&self, pose: &voxy_animation::Pose) -> Result<Vec<Mat4>, ModelError> {
        if !self.skinned {
            return Err(fail("model has no skin"));
        }
        pose.skin_matrices(&self.skeleton)
            .map_err(|e| fail(e.to_string()))
    }

    /// CPU skinning for the portable `SceneRenderer`, including WebGL/mobile.
    /// Returned positions include node transforms; apply only the instance transform.
    /// GPU skinning remains preferable for large meshes.
    /// # Errors
    /// Rejects an invalid pose or non-finite deformed geometry.
    pub fn scene_meshes(&self, pose: &voxy_animation::Pose) -> Result<Vec<SceneMesh>, ModelError> {
        if self.has_morph() {
            return Err(fail("morph geometry requires a complete animation frame"));
        }
        let palette = pose
            .skin_matrices(&self.skeleton)
            .map_err(|e| fail(e.to_string()))?;
        let model = self.mesh_transform(pose)?;
        self.primitives
            .iter()
            .map(|primitive| match &primitive.geometry {
                ModelGeometry::Static(mesh) => {
                    let positions = mesh
                        .vertices()
                        .iter()
                        .map(|v| {
                            model
                                .transform_point3(Vec3::from_array(v.position))
                                .to_array()
                        })
                        .collect();
                    let normals = mesh
                        .authored_normals()
                        .map(|values| {
                            let transform = model.inverse().transpose();
                            values
                                .iter()
                                .map(|normal| {
                                    let value =
                                        transform.transform_vector3(Vec3::from_array(*normal));
                                    if !value.is_finite() {
                                        return Err(fail("nonfinite transformed model normal"));
                                    }
                                    Ok(value.try_normalize().unwrap_or(Vec3::ZERO).to_array())
                                })
                                .collect::<Result<Vec<_>, ModelError>>()
                        })
                        .transpose()?;
                    mesh.clone()
                        .with_deformed_geometry(positions, normals)
                        .map_err(|e| fail(e.to_string()))
                }
                ModelGeometry::Skinned(mesh) => mesh
                    .posed_scene_mesh(&palette, Mat4::IDENTITY, primitive.color)
                    .map_err(|error| fail(error.to_string())),
            })
            .collect()
    }
}
