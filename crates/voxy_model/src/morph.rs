//! Local morph deformation, before node transforms and skinning. No clocks or GPU ownership.
mod checkpoint;
use super::{ModelError, ModelGeometry, ModelPrimitive, fail};
pub use checkpoint::{ModelPrimitiveCheckpoint, MorphArchiveLimits, MorphGeometryCheckpoint};
use voxy_render::SkinnedMesh;

/// glTF-style source-local additive delta streams shared with GPU skinning.
pub use voxy_render::SkinnedMorphTarget as MorphTarget;

/// Immutable source primitive and validated target streams. Evaluation never
/// accumulates onto a previously deformed frame or changes the source primitive.
#[derive(Clone, Debug)]
pub struct MorphGeometry {
    pub(super) source: ModelPrimitive,
    targets: Vec<MorphTarget>,
}

impl MorphGeometry {
    /// Validate target streams against the base primitive and a total delta-vector budget.
    ///
    /// # Errors
    /// Rejects empty targets, invalid counts/values, normal deltas without base
    /// normals, and exceeded budgets. Tangent morphing is not represented here.
    pub fn new(
        source: ModelPrimitive,
        targets: Vec<MorphTarget>,
        max_delta_vectors: usize,
    ) -> Result<Self, ModelError> {
        if targets.is_empty() || targets.len() > voxy_render::MAX_MORPH_TARGETS {
            return Err(fail("empty morph targets"));
        }
        let (count, has_normals) = match &source.geometry {
            ModelGeometry::Static(mesh) => {
                (mesh.vertices().len(), mesh.authored_normals().is_some())
            }
            ModelGeometry::Skinned(mesh) => (
                mesh.vertices().len(),
                mesh.vertices().iter().all(|v| v.normal != [0.; 3]),
            ),
        };
        let mut total = 0usize;
        for target in &targets {
            if target.positions.is_none() && target.normals.is_none() {
                return Err(fail("empty morph target streams"));
            }
            if target.normals.is_some() && !has_normals {
                return Err(fail("morph normal deltas require base normals"));
            }
            for values in [target.positions.as_ref(), target.normals.as_ref()]
                .into_iter()
                .flatten()
            {
                total = total
                    .checked_add(values.len())
                    .ok_or_else(|| fail("morph delta budget overflow"))?;
                if total > max_delta_vectors {
                    return Err(fail("morph delta budget exceeded"));
                }
                if values.len() != count || values.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(fail("invalid morph delta stream"));
                }
            }
        }
        Ok(Self { source, targets })
    }

    #[must_use]
    pub fn targets(&self) -> &[MorphTarget] {
        &self.targets
    }

    fn validate_weights(&self, weights: &[f32]) -> Result<(), ModelError> {
        if weights.len() != self.targets.len() || weights.iter().any(|w| !w.is_finite()) {
            return Err(fail("invalid morph weights"));
        }
        Ok(())
    }

    fn component_terms<'a>(
        &'a self,
        vertex: usize,
        weights: &'a [f32],
        normal: bool,
        axis: usize,
    ) -> impl Iterator<Item = (f32, f32)> + 'a {
        self.targets
            .iter()
            .zip(weights)
            .map(move |(target, weight)| {
                let stream = if normal {
                    &target.normals
                } else {
                    &target.positions
                };
                (
                    stream.as_ref().map_or(0., |values| values[vertex][axis]),
                    *weight,
                )
            })
    }

    fn deform64(&self, base: [f32; 3], vertex: usize, weights: &[f32], normal: bool) -> [f64; 3] {
        std::array::from_fn(|axis| {
            voxy_render::exact_morph_component64(
                base[axis],
                self.component_terms(vertex, weights, normal, axis),
            )
            .expect("validated finite bounded source morph terms")
        })
    }

    fn deform32(
        &self,
        base: [f32; 3],
        vertex: usize,
        weights: &[f32],
        normal: bool,
    ) -> Result<[f32; 3], ModelError> {
        let mut result = [0.; 3];
        for axis in 0..3 {
            result[axis] = voxy_render::exact_morph_component(
                base[axis],
                self.component_terms(vertex, weights, normal, axis),
            )
            .ok_or_else(|| fail("morph deformation overflows f32"))?;
        }
        Ok(result)
    }

    /// The validated GPU source uses the same immutable delta representation.
    /// # Errors
    /// Rejects static geometry (which requires a rigid skin binding first).
    pub fn skinned_morph_mesh(&self) -> Result<voxy_render::SkinnedMorphMesh, ModelError> {
        let ModelGeometry::Skinned(mesh) = &self.source.geometry else {
            return Err(fail("GPU morph source requires skin binding"));
        };
        voxy_render::SkinnedMorphMesh::new(
            std::sync::Arc::new(mesh.clone()),
            self.targets.clone(),
            usize::MAX,
        )
        .map_err(|e| fail(e.to_string()))
    }

    /// Exact source sums rounded to f64 for physical/reference consumers.
    /// Weights are neither normalized nor clamped, including negative values.
    /// # Errors
    /// Rejects nonfinite weights or a mismatched target count.
    pub fn positions64(&self, weights: &[f32]) -> Result<Vec<[f64; 3]>, ModelError> {
        self.validate_weights(weights)?;
        let positions: Vec<_> = match &self.source.geometry {
            ModelGeometry::Static(mesh) => mesh.vertices().iter().map(|v| v.position).collect(),
            ModelGeometry::Skinned(mesh) => mesh.vertices().iter().map(|v| v.position).collect(),
        };
        Ok(positions
            .into_iter()
            .enumerate()
            .map(|(i, p)| self.deform64(p, i, weights, false))
            .collect())
    }

    /// Produce local geometry for existing skinning/upload paths. Normalization
    /// remains with the subsequent normal transform, after all deltas are summed.
    /// # Errors
    /// Rejects invalid weights and deformation outside the finite f32 range.
    pub fn evaluate(&self, weights: &[f32]) -> Result<ModelPrimitive, ModelError> {
        self.validate_weights(weights)?;
        let geometry = match &self.source.geometry {
            ModelGeometry::Static(mesh) => {
                let positions = mesh
                    .vertices()
                    .iter()
                    .enumerate()
                    .map(|(i, v)| self.deform32(v.position, i, weights, false))
                    .collect::<Result<Vec<_>, _>>()?;
                let normals = mesh
                    .authored_normals()
                    .map(|values| {
                        values
                            .iter()
                            .enumerate()
                            .map(|(i, n)| self.deform32(*n, i, weights, true))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?;
                ModelGeometry::Static(
                    mesh.clone()
                        .with_deformed_geometry(positions, normals)
                        .map_err(|e| fail(e.to_string()))?,
                )
            }
            ModelGeometry::Skinned(mesh) => {
                let mut vertices = mesh.vertices().to_vec();
                for (i, v) in vertices.iter_mut().enumerate() {
                    v.position = self.deform32(v.position, i, weights, false)?;
                    v.normal = self.deform32(v.normal, i, weights, true)?;
                }
                ModelGeometry::Skinned(
                    SkinnedMesh::new(vertices, mesh.indices().to_vec(), mesh.joint_count())
                        .map_err(|e| fail(e.to_string()))?,
                )
            }
        };
        Ok(ModelPrimitive {
            geometry,
            ..self.source.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Vec3};
    use voxy_render::{SceneMesh, SceneVertex, SkinnedVertex};

    pub(super) fn primitive(skinned: bool) -> ModelPrimitive {
        let positions = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let geometry = if skinned {
            ModelGeometry::Skinned(
                SkinnedMesh::new(
                    positions
                        .map(|position| SkinnedVertex {
                            position,
                            normal: [0., 0., 1.],
                            uv: [0.25, 0.75],
                            joints: [0; 4],
                            weights: [65535, 0, 0, 0],
                        })
                        .to_vec(),
                    vec![0, 1, 2],
                    1,
                )
                .unwrap(),
            )
        } else {
            ModelGeometry::Static(
                SceneMesh::new(
                    positions
                        .map(|position| SceneVertex {
                            position,
                            uv: [0.25, 0.75],
                            color: [0.2; 4],
                        })
                        .to_vec(),
                    vec![0, 1, 2],
                )
                .unwrap()
                .with_normals(vec![[0., 0., 1.]; 3])
                .unwrap()
                .with_material_coordinates(positions.to_vec())
                .unwrap()
                .with_material_parameters([1., 2., 3., 4.])
                .unwrap(),
            )
        };
        ModelPrimitive {
            geometry,
            color: [0.2; 4],
            base_color_texture: None,
        }
    }

    pub(super) fn targets() -> Vec<MorphTarget> {
        vec![
            MorphTarget {
                positions: Some(vec![[2., 0., 0.]; 3]),
                normals: Some(vec![[1., 0., 0.]; 3]),
            },
            MorphTarget {
                positions: Some(vec![[0., 4., 0.]; 3]),
                normals: None,
            },
        ]
    }

    #[test]
    fn morph_is_local_before_skinning_and_preserves_source_and_influences() {
        let morph = MorphGeometry::new(primitive(true), targets(), 9).unwrap();
        let result = morph.evaluate(&[1.5, -0.5]).unwrap();
        let ModelGeometry::Skinned(mesh) = result.geometry else {
            panic!("expected skin")
        };
        assert_eq!(mesh.vertices()[0].position, [4., -2., 0.]);
        assert_eq!(mesh.vertices()[0].normal, [1.5, 0., 1.]);
        assert_eq!(mesh.vertices()[0].uv, [0.25, 0.75]);
        assert_eq!(mesh.vertices()[0].weights, [65535, 0, 0, 0]);
        let matrix = Mat4::from_scale_rotation_translation(
            Vec3::new(2., 3., 1.),
            glam::Quat::IDENTITY,
            Vec3::new(10., 0., 0.),
        );
        let posed = mesh
            .posed_scene_mesh(&[matrix], Mat4::IDENTITY, result.color)
            .unwrap();
        assert_eq!(posed.vertices()[0].position, [18., -6., 0.]);
        // Re-evaluation always starts from the original, never the previous output.
        let ModelGeometry::Skinned(reset) = morph.evaluate(&[0., 0.]).unwrap().geometry else {
            panic!("expected skin")
        };
        assert_eq!(reset.vertices()[0].position, [1., 0., 0.]);
        assert_eq!(morph.positions64(&[1.5, -0.5]).unwrap()[0], [4., -2., 0.]);
    }

    #[test]
    fn static_morph_retains_material_streams_and_normal_deltas() {
        let source = primitive(false);
        let ModelGeometry::Static(base) = &source.geometry else {
            panic!("expected static")
        };
        let morph = MorphGeometry::new(source.clone(), targets(), 9).unwrap();
        let ModelGeometry::Static(mesh) = morph.evaluate(&[1.5, -0.5]).unwrap().geometry else {
            panic!("expected static")
        };
        assert_eq!(mesh.vertices()[0].position, [4., -2., 0.]);
        assert_eq!(mesh.authored_normals().unwrap()[0], [1.5, 0., 1.]);
        assert_eq!(
            mesh.explicit_material_coordinates(),
            base.explicit_material_coordinates()
        );
        assert_eq!(mesh.indices(), base.indices());
        assert_eq!(mesh.vertices()[0].color, base.vertices()[0].color);
    }

    #[test]
    fn morph_rejects_bad_streams_weights_budgets_and_overflow() {
        assert!(MorphGeometry::new(primitive(true), targets(), 8).is_err());
        let mut bad = targets();
        bad[0].positions.as_mut().unwrap().pop();
        assert!(MorphGeometry::new(primitive(true), bad, 9).is_err());
        let mut bad = targets();
        bad[0].positions.as_mut().unwrap()[0][0] = f32::NAN;
        assert!(MorphGeometry::new(primitive(true), bad, 9).is_err());
        let morph = MorphGeometry::new(primitive(true), targets(), 9).unwrap();
        assert!(morph.evaluate(&[0.]).is_err());
        assert!(morph.evaluate(&[f32::NAN, 0.]).is_err());
        assert!(morph.evaluate(&[f32::MAX, 0.]).is_err());
        assert!(morph.positions64(&[f32::MAX, 0.]).unwrap()[0][0].is_finite());
    }
}

/// One imported morph owner, before primitive expansion. Unnamed nodes can be
/// inspected by source index but cannot be used as stable named scene overrides.
#[derive(Clone, Debug, PartialEq)]
pub struct MorphNode {
    pub source_node: usize,
    pub name: Option<std::sync::Arc<str>>,
    pub weights: Vec<f32>,
}

#[derive(Clone, Debug)]
pub(super) struct MorphBinding {
    pub node: usize,
    pub name: Option<std::sync::Arc<str>>,
    pub geometry: MorphGeometry,
    pub defaults: Vec<f32>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct MorphImportData {
    pub defaults: voxy_animation::PropertySet,
    pub sources: Vec<std::sync::Arc<voxy_animation::AnimationClip>>,
    pub skeleton: Option<voxy_animation::Skeleton>,
    pub primitives: Vec<Option<MorphBinding>>,
    pub clips: Vec<voxy_animation::PropertyClip>,
}

pub(super) fn import_defaults(
    nodes: &[gltf::Node<'_>],
) -> Result<voxy_animation::PropertySet, ModelError> {
    let mut entries = Vec::new();
    for node in nodes.iter().filter(|node| node.mesh().is_some()) {
        let weights = node_weights(node)?;
        if entries
            .len()
            .checked_add(weights.len())
            .is_none_or(|n| n > voxy_animation::MAX_PROPERTY_TRACKS)
        {
            return Err(fail("morph property schema budget exceeded"));
        }
        entries.extend(weights.into_iter().enumerate().map(|(index, weight)| {
            (
                std::sync::Arc::from(weight_name(node.index(), index)),
                voxy_animation::PropertyValue::Float(weight),
            )
        }));
    }
    voxy_animation::PropertySet::new(entries).map_err(|e| fail(e.to_string()))
}

pub(super) fn node_weights(node: &gltf::Node<'_>) -> Result<Vec<f32>, ModelError> {
    let mesh = node
        .mesh()
        .ok_or_else(|| fail("morph weights require a mesh"))?;
    let mut counts = mesh.primitives().map(|p| p.morph_targets().count());
    let count = counts.next().unwrap_or(0);
    if counts.any(|n| n != count) {
        return Err(fail(
            "mesh primitives have inconsistent morph target counts",
        ));
    }
    if mesh
        .weights()
        .is_some_and(|w| w.len() != count || w.iter().any(|v| !v.is_finite()))
    {
        return Err(fail("invalid mesh morph defaults"));
    }
    let weights = node
        .weights()
        .or_else(|| mesh.weights())
        .map_or_else(|| vec![0.; count], <[f32]>::to_vec);
    if weights.len() != count || weights.iter().any(|w| !w.is_finite()) {
        return Err(fail("invalid node morph defaults"));
    }
    Ok(weights)
}

fn weight_name(node: usize, weight: usize) -> String {
    format!("node{node}.weight{weight}")
}

pub(super) fn import_weight_tracks(
    tracks: &mut Vec<voxy_animation::PropertyTrack>,
    node: usize,
    count: usize,
    times: &[f32],
    values: Vec<f32>,
    mode: voxy_animation::Interpolation,
) -> Result<(), ModelError> {
    use voxy_animation::{Interpolation, PropertyKey, PropertyTrack, PropertyValue};
    let duration = times
        .last()
        .copied()
        .ok_or_else(|| fail("empty morph animation"))?
        .max(f32::EPSILON);
    if tracks
        .len()
        .checked_add(count)
        .is_none_or(|n| n > voxy_animation::MAX_PROPERTY_TRACKS)
    {
        return Err(fail("morph property track budget exceeded"));
    }
    let stride = if mode == Interpolation::CubicSpline {
        3
    } else {
        1
    };
    if times
        .len()
        .checked_mul(stride)
        .and_then(|n| n.checked_mul(count))
        != Some(values.len())
    {
        return Err(fail("morph animation output count mismatch"));
    }
    if values.iter().any(|v| !v.is_finite()) {
        return Err(fail("nonfinite morph animation value"));
    }
    for weight in 0..count {
        let keys = times
            .iter()
            .enumerate()
            .map(|(key, &time)| PropertyKey {
                time,
                value: PropertyValue::Float(
                    values[(key * stride + usize::from(stride == 3)) * count + weight],
                ),
            })
            .collect();
        let tangents = if stride == 3 {
            (0..times.len())
                .map(|key| {
                    (
                        PropertyValue::Float(values[key * 3 * count + weight]),
                        PropertyValue::Float(values[(key * 3 + 2) * count + weight]),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        tracks.push(
            PropertyTrack::new_with_interpolation(
                weight_name(node, weight),
                keys,
                duration,
                mode,
                tangents,
            )
            .map_err(|e| fail(e.to_string()))?,
        );
    }
    Ok(())
}

/// Explicit convenience wrapper over the same frame-aware ModelAsset owner.
#[derive(Clone, Debug)]
pub struct MorphModelAsset {
    model: super::ModelAsset,
}
impl MorphModelAsset {
    /// Import all supported geometry/weights through the shared importer.
    /// # Errors
    /// Rejects malformed, unsupported and over-budget source features.
    pub fn parse(
        source: &[u8],
        buffers: &[&[u8]],
        limits: super::ModelLimits,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            model: super::ModelAsset::parse_animated(source, buffers, limits)?,
        })
    }
    /// CPU reference deformation using the complete immutable frame.
    /// # Errors
    /// Rejects incompatible frame data and overflowing deformation.
    pub fn scene_meshes(
        &self,
        frame: &voxy_animation::AnimatorFrame,
    ) -> Result<Vec<voxy_render::SceneMesh>, ModelError> {
        self.model.scene_meshes_frame(frame)
    }
}
impl std::ops::Deref for MorphModelAsset {
    type Target = super::ModelAsset;
    fn deref(&self) -> &Self::Target {
        &self.model
    }
}

impl super::ModelAsset {
    #[must_use]
    pub fn has_morph(&self) -> bool {
        !self.morph.defaults.is_empty()
    }

    fn validate_morph_layout(&self) -> Result<(), ModelError> {
        if self
            .morph
            .skeleton
            .as_ref()
            .is_none_or(|rig| rig.joints() != self.skeleton.joints())
        {
            return Err(fail("morph source rig changed without reimport"));
        }
        if self.morph.primitives.len() != self.primitives.len() {
            return Err(fail("morph primitive layout changed"));
        }
        for (source, binding) in self.primitives.iter().zip(&self.morph.primitives) {
            if let Some(binding) = binding {
                let same = match (&source.geometry, &binding.geometry.source.geometry) {
                    (ModelGeometry::Static(a), ModelGeometry::Static(b)) => {
                        a.vertices() == b.vertices()
                            && a.indices() == b.indices()
                            && a.authored_normals() == b.authored_normals()
                            && a.explicit_material_coordinates()
                                == b.explicit_material_coordinates()
                    }
                    (ModelGeometry::Skinned(a), ModelGeometry::Skinned(b)) => {
                        (std::ptr::eq(a.vertices(), b.vertices()) || a.vertices() == b.vertices())
                            && (std::ptr::eq(a.indices(), b.indices())
                                || a.indices() == b.indices())
                            && a.joint_count() == b.joint_count()
                    }
                    _ => false,
                };
                if !same {
                    return Err(fail("morph source geometry changed without reimport"));
                }
            }
        }
        Ok(())
    }

    /// Bind imported property curves to an existing skeletal playback owner.
    /// # Errors
    /// Rejects foreign/replaced source clips or invalid property schemas.
    pub fn bind_animator_properties(
        &self,
        animator: &mut voxy_animation::Animator,
    ) -> Result<(), ModelError> {
        if !self.has_morph() {
            return Ok(());
        }
        self.validate_morph_layout()?;
        let pairs = self.property_bindings()?;
        animator
            .bind_property_clips(self.morph.defaults.clone(), pairs)
            .map_err(|e| fail(e.to_string()))
    }

    fn property_bindings(
        &self,
    ) -> Result<
        Vec<(
            std::sync::Arc<voxy_animation::AnimationClip>,
            std::sync::Arc<voxy_animation::PropertyClip>,
        )>,
        ModelError,
    > {
        let pairs = self
            .animations
            .iter()
            .map(|clip| {
                let index = self
                    .morph
                    .sources
                    .iter()
                    .position(|source| std::sync::Arc::ptr_eq(source, clip))
                    .ok_or_else(|| fail("morph source clip identity changed without reimport"))?;
                Ok((
                    clip.clone(),
                    std::sync::Arc::new(self.morph.clips[index].clone()),
                ))
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Ok(pairs)
    }

    /// Admit new source revisions without replacing the active fade's old data.
    /// # Errors
    /// Rejects source/layout/schema conflicts; the animator remains unchanged.
    pub fn register_animator_properties(
        &self,
        animator: &mut voxy_animation::Animator,
    ) -> Result<(), ModelError> {
        if !self.has_morph() {
            return Ok(());
        }
        self.validate_morph_layout()?;
        animator
            .register_property_clips(self.morph.defaults.clone(), self.property_bindings()?)
            .map_err(|e| fail(e.to_string()))
    }

    #[must_use]
    pub fn morph_schema_compatible(&self, other: &Self) -> bool {
        self.has_morph() == other.has_morph()
            && self.morph.defaults.len() == other.morph.defaults.len()
            && self.morph.defaults.entries().iter().all(|(name, value)| {
                other
                    .morph
                    .defaults
                    .get(name)
                    .is_some_and(|old| old.same_kind(*value))
            })
    }

    /// Old weight sources can fade into a revision only while their target
    /// indices retain the same source delta basis. Changed bases need a new owner.
    #[must_use]
    pub fn morph_basis_compatible(&self, other: &Self) -> bool {
        if !self.has_morph() && !other.has_morph() {
            return true;
        }
        self.morph_schema_compatible(other)
            && self.morph.primitives.len() == other.morph.primitives.len()
            && self
                .morph
                .primitives
                .iter()
                .zip(&other.morph.primitives)
                .all(|(a, b)| match (a, b) {
                    (None, None) => true,
                    (Some(a), Some(b)) => {
                        a.node == b.node && a.geometry.targets == b.geometry.targets
                    }
                    _ => false,
                })
    }

    /// Compare property curves/defaults as well as skeletal clip metadata when
    /// deciding whether a resource-only rebind can retain its current sources.
    #[must_use]
    pub fn has_same_morph_animation(&self, other: &Self, allow_reorder: bool) -> bool {
        if !self.has_morph() && !other.has_morph() {
            return true;
        }
        if !self.morph_basis_compatible(other)
            || self.morph.defaults != other.morph.defaults
            || self.animations.len() != other.animations.len()
        {
            return false;
        }
        self.animations.iter().enumerate().all(|(index, clip)| {
            let other_index = if allow_reorder {
                let mut matches = other
                    .animations
                    .iter()
                    .enumerate()
                    .filter(|(_, other)| other.name() == clip.name());
                let first = matches.next();
                if matches.next().is_some() {
                    return false;
                }
                let Some((index, _)) = first else {
                    return false;
                };
                index
            } else {
                index
            };
            self.property_clip(index)
                .ok()
                .zip(other.property_clip(other_index).ok())
                .is_some_and(|(a, b)| a == b)
        })
    }

    fn property_clip(&self, index: usize) -> Result<&voxy_animation::PropertyClip, ModelError> {
        let clip = self
            .animations
            .get(index)
            .ok_or_else(|| fail("animation clip index out of range"))?;
        let source = self
            .morph
            .sources
            .iter()
            .position(|source| std::sync::Arc::ptr_eq(source, clip))
            .ok_or_else(|| fail("morph source clip identity changed without reimport"))?;
        self.morph
            .clips
            .get(source)
            .ok_or_else(|| fail("missing morph property clip"))
    }

    /// Stateless authored-phase frame; phase one includes the final authored key
    /// even for looping clips, matching the existing skeletal preview contract.
    /// # Errors
    /// Rejects invalid phases, source identity/schema changes and invalid samples.
    pub fn sample_frame_phase(
        &self,
        clip: Option<usize>,
        phase: f64,
    ) -> Result<voxy_animation::AnimatorFrame, ModelError> {
        let pose = self.sample_pose_phase(clip, phase)?;
        let mut frame = self.bind_frame()?;
        frame.skin_matrices = pose
            .skin_matrices(&self.skeleton)
            .map_err(|e| fail(e.to_string()))?;
        frame.pose = pose;
        if self.has_morph() {
            if let Some(index) = clip {
                let sampled = self
                    .property_clip(index)?
                    .sample_authored_phase(phase)
                    .map_err(|e| fail(e.to_string()))?;
                frame.properties = voxy_animation::PropertySet::new(
                    self.morph
                        .defaults
                        .entries()
                        .iter()
                        .map(|(name, default)| {
                            (name.clone(), sampled.get(name).unwrap_or(*default))
                        })
                        .collect(),
                )
                .map_err(|e| fail(e.to_string()))?;
            }
        }
        Ok(frame)
    }

    /// Complete bind-pose frame, retaining authored initial morph weights.
    /// # Errors
    /// Rejects invalid pose or mutated morph layout.
    pub fn bind_frame(&self) -> Result<voxy_animation::AnimatorFrame, ModelError> {
        if self.has_morph() {
            self.validate_morph_layout()?;
        }
        let pose = self.skeleton.bind_pose();
        let skin_matrices = pose
            .skin_matrices(&self.skeleton)
            .map_err(|e| fail(e.to_string()))?;
        Ok(voxy_animation::AnimatorFrame {
            pose,
            skin_matrices,
            properties: self.morph.defaults.clone(),
            root_motion: glam::Vec3::ZERO,
            root_motion_joint: 0,
            transition_weight: 1.,
        })
    }

    #[must_use]
    pub fn animations(&self) -> &[std::sync::Arc<voxy_animation::AnimationClip>] {
        &self.animations
    }
    #[must_use]
    pub fn skeleton(&self) -> &voxy_animation::Skeleton {
        &self.skeleton
    }
    #[must_use]
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }
    #[must_use]
    pub fn morph_primitive_count(&self) -> usize {
        self.morph.primitives.iter().flatten().count()
    }
    #[must_use]
    pub fn weight_clips(&self) -> &[voxy_animation::PropertyClip] {
        &self.morph.clips
    }

    /// Create the existing skeletal Animator with this asset's immutable weight
    /// curves/defaults bound to exact source clip identities. No second clock.
    /// # Errors
    /// Rejects an absent clip, invalid initial pose or property binding.
    pub fn animator(&self, clip: usize) -> Result<voxy_animation::Animator, ModelError> {
        let initial = self
            .animations
            .get(clip)
            .ok_or_else(|| fail("animation clip index out of range"))?;
        let mut animator =
            voxy_animation::Animator::try_new(initial.clone()).map_err(|e| fail(e.to_string()))?;
        self.bind_animator_properties(&mut animator)?;
        Ok(animator)
    }

    /// Read immutable morph weights carried with a skeletal frame.
    /// # Errors
    /// Rejects a missing/incompatible property schema instead of using defaults
    /// to conceal an unbound or foreign playback frame.
    pub fn frame_weights(
        &self,
        frame: &voxy_animation::AnimatorFrame,
    ) -> Result<Vec<Vec<f32>>, ModelError> {
        if !self.has_morph() {
            return Ok(vec![Vec::new(); self.primitives.len()]);
        }
        self.validate_morph_layout()?;
        if frame.properties.len() != self.morph.defaults.len()
            || self.morph.defaults.entries().iter().any(|(name, default)| {
                frame
                    .properties
                    .get(name)
                    .is_none_or(|value| !value.same_kind(*default))
            })
        {
            return Err(fail("morph frame property schema mismatch"));
        }
        Ok(self
            .morph
            .primitives
            .iter()
            .map(|binding| {
                binding.as_ref().map_or_else(Vec::new, |binding| {
                    (0..binding.defaults.len())
                        .map(|i| {
                            frame
                                .properties
                                .get_float(&weight_name(binding.node, i))
                                .expect("validated complete scalar morph schema")
                        })
                        .collect()
                })
            })
            .collect())
    }

    /// Enumerate distinct imported morph source nodes, their authored names and
    /// initial weights. Primitive expansion does not duplicate node entries.
    #[must_use]
    pub fn morph_nodes(&self) -> Vec<MorphNode> {
        let mut nodes = std::collections::BTreeMap::new();
        for binding in self.morph.primitives.iter().flatten() {
            nodes.entry(binding.node).or_insert_with(|| MorphNode {
                source_node: binding.node,
                name: binding.name.clone(),
                weights: binding.defaults.clone(),
            });
        }
        nodes.into_values().collect()
    }

    /// Resolve a unique authored morph node name to its original glTF index.
    /// # Errors
    /// Rejects missing, ambiguous or non-morph nodes. Palette order is irrelevant.
    pub fn resolve_morph_node_name(&self, name: &str) -> Result<usize, ModelError> {
        self.resolve_joint_name(name)?;
        self.morph
            .primitives
            .iter()
            .flatten()
            .find(|binding| binding.name.as_deref() == Some(name))
            .map(|binding| binding.node)
            .ok_or_else(|| fail("named morph node is missing"))
    }

    /// Replace one authored node's morph weights in an accepted frame.
    /// All primitives belonging to that node share the replacement. This changes
    /// neither the immutable asset nor the animation clock. No clamping is applied.
    /// The frame is published only after the complete deformation is admitted.
    /// # Errors
    /// Rejects missing nodes, incompatible schemas/counts, nonfinite weights,
    /// overflowing deformation or a palette inconsistent with the pose.
    pub fn set_frame_morph_weights(
        &self,
        frame: &mut voxy_animation::AnimatorFrame,
        node: usize,
        weights: &[f32],
    ) -> Result<(), ModelError> {
        self.frame_weights(frame)?;
        let bindings: Vec<_> = self
            .morph
            .primitives
            .iter()
            .flatten()
            .filter(|binding| binding.node == node)
            .collect();
        if bindings.is_empty() {
            return Err(fail("morph node is missing"));
        }
        for binding in &bindings {
            binding.geometry.validate_weights(weights)?;
        }
        let mut entries = frame.properties.entries().to_vec();
        for (index, weight) in weights.iter().enumerate() {
            let name = weight_name(node, index);
            let entry = entries
                .iter_mut()
                .find(|(key, _)| key.as_ref() == name)
                .ok_or_else(|| fail("morph frame property schema mismatch"))?;
            entry.1 = voxy_animation::PropertyValue::Float(*weight);
        }
        let mut candidate = frame.clone();
        candidate.properties =
            voxy_animation::PropertySet::new(entries).map_err(|e| fail(e.to_string()))?;
        self.validate_morph_frame(&candidate)?;
        *frame = candidate;
        Ok(())
    }

    /// Admit morph geometry and its posed output before a simulation transaction
    /// publishes clocks or body movement. No GPU resources are changed.
    /// # Errors
    /// Rejects incompatible schemas, overflowing deformation or invalid skin maps.
    pub fn validate_morph_frame(
        &self,
        frame: &voxy_animation::AnimatorFrame,
    ) -> Result<(), ModelError> {
        if !self.has_morph() {
            return Ok(());
        }
        if frame.skin_matrices
            != frame
                .pose
                .skin_matrices(&self.skeleton)
                .map_err(|e| fail(e.to_string()))?
        {
            return Err(fail("morph frame palette differs from pose"));
        }
        let weights = self.frame_weights(frame)?;
        for (binding, weights) in self.morph.primitives.iter().zip(weights) {
            if let Some(binding) = binding {
                let deformed = binding.geometry.evaluate(&weights)?;
                match &deformed.geometry {
                    ModelGeometry::Skinned(mesh) => mesh
                        .validate_scene_pose(&frame.skin_matrices)
                        .map_err(|e| fail(e.to_string()))?,
                    ModelGeometry::Static(_) => {
                        let mut model = self.clone();
                        model.primitives = vec![deformed];
                        model.morph = Default::default();
                        model.scene_meshes(&frame.pose)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Existing CPU skinning applied to an immutable pose AND property snapshot.
    /// This does not re-sample either channel or advance a clock.
    /// # Errors
    /// Rejects incompatible frame/schema and nonfinite deformation.
    pub fn scene_meshes_frame(
        &self,
        frame: &voxy_animation::AnimatorFrame,
    ) -> Result<Vec<voxy_render::SceneMesh>, ModelError> {
        if !self.has_morph() {
            return self.scene_meshes(&frame.pose);
        }
        let weights = self.frame_weights(frame)?;
        let mut evaluated = self.clone();
        for ((primitive, binding), weights) in evaluated
            .primitives
            .iter_mut()
            .zip(&self.morph.primitives)
            .zip(weights)
        {
            if let Some(binding) = binding {
                let mut deformed = binding.geometry.evaluate(&weights)?;
                deformed.color = primitive.color;
                deformed.base_color_texture = primitive.base_color_texture;
                *primitive = deformed;
            }
        }
        evaluated.morph = Default::default();
        evaluated.scene_meshes(&frame.pose)
    }

    /// Immutable morph source for the existing SceneSkinner upload path.
    /// Non-morph primitives return None; invalid indices are errors.
    /// # Errors
    /// Rejects absent primitive indices or invalid source layout/bindings.
    /// Imported rigid morphs use their authored node, including one unskinned mesh.
    pub fn morph_source(
        &self,
        primitive: usize,
    ) -> Result<Option<voxy_render::SkinnedMorphMesh>, ModelError> {
        if primitive >= self.primitives.len() {
            return Err(fail("model primitive index out of range"));
        }
        if !self.has_morph() {
            return Ok(None);
        }
        self.validate_morph_layout()?;
        self.morph
            .primitives
            .get(primitive)
            .ok_or_else(|| fail("model primitive index out of range"))?
            .as_ref()
            .map(|binding| binding.geometry.skinned_morph_mesh())
            .transpose()
    }

    /// One weight vector per primitive, in original import order. Non-morph
    /// primitives have an empty vector. Missing channels retain node/mesh defaults.
    /// # Errors
    /// Rejects invalid clip/time and nonfinite interpolated values.
    pub fn sample_weights(
        &self,
        clip: Option<usize>,
        time: f32,
    ) -> Result<Vec<Vec<f32>>, ModelError> {
        if !time.is_finite() {
            return Err(fail("animation time must be finite"));
        }
        if !self.has_morph() {
            if clip.is_some_and(|index| index >= self.animations.len()) {
                return Err(fail("animation clip index out of range"));
            }
            return Ok(vec![Vec::new(); self.primitives.len()]);
        }
        self.validate_morph_layout()?;
        let sampled = clip
            .map(|i| {
                self.property_clip(i)?
                    .sample(time)
                    .map_err(|e| fail(e.to_string()))
            })
            .transpose()?;
        Ok(self
            .morph
            .primitives
            .iter()
            .map(|binding| {
                binding.as_ref().map_or_else(Vec::new, |binding| {
                    binding
                        .defaults
                        .iter()
                        .enumerate()
                        .map(|(i, default)| {
                            sampled
                                .as_ref()
                                .and_then(|set| set.get_float(&weight_name(binding.node, i)))
                                .unwrap_or(*default)
                        })
                        .collect()
                })
            })
            .collect())
    }

    /// Wide physical/reference surfaces with explicit accepted weights. Local
    /// source sums round directly to f64 before the existing wide skinning path.
    /// # Errors
    /// Rejects invalid source layout, weights, pose or nonfinite wide geometry.
    pub fn scene_surfaces64_with_weights(
        &self,
        pose: &voxy_animation::Pose64,
        weights: &[Vec<f32>],
    ) -> Result<Vec<super::ModelSurface64>, ModelError> {
        if weights.len() != self.primitives.len() {
            return Err(fail("morph primitive weight count mismatch"));
        }
        if !self.has_morph() {
            if weights.iter().any(|w| !w.is_empty()) {
                return Err(fail("weights supplied for a non-morph primitive"));
            }
            return self.scene_surfaces64(pose);
        }
        self.validate_morph_layout()?;
        let overrides = self
            .morph
            .primitives
            .iter()
            .zip(weights)
            .map(|(binding, weights)| {
                if let Some(binding) = binding {
                    binding.geometry.positions64(weights).map(Some)
                } else if weights.is_empty() {
                    Ok(None)
                } else {
                    Err(fail("weights supplied for a non-morph primitive"))
                }
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        self.scene_surfaces64_internal(pose, Some(&overrides))
    }

    /// CPU reference path: sample TRS and weights at the same supplied clip time,
    /// morph in local space, then use the existing hierarchy and skinning owner.
    /// No playback clock or accepted runtime state is created here.
    /// # Errors
    /// Rejects invalid samples and deformation outside the renderer's finite range.
    pub fn sample_meshes(
        &self,
        clip: Option<usize>,
        time: f32,
    ) -> Result<Vec<voxy_render::SceneMesh>, ModelError> {
        let weights = self.sample_weights(clip, time)?;
        self.sample_meshes_with_weights(clip, time, &weights)
    }

    /// Explicit reference evaluation with caller-supplied weights, one vector per
    /// source primitive. Useful for authoring shapes absent from a source clip.
    /// This never alters source defaults, curves or runtime playback state.
    /// # Errors
    /// Rejects wrong primitive/target counts, nonfinite weights or invalid pose.
    pub fn sample_meshes_with_weights(
        &self,
        clip: Option<usize>,
        time: f32,
        weights: &[Vec<f32>],
    ) -> Result<Vec<voxy_render::SceneMesh>, ModelError> {
        if !self.has_morph() {
            if weights.len() != self.primitives.len() || weights.iter().any(|w| !w.is_empty()) {
                return Err(fail("invalid non-morph weights"));
            }
            return self.scene_meshes(&self.sample_pose(clip, time)?);
        }
        self.validate_morph_layout()?;
        if weights.len() != self.morph.primitives.len() {
            return Err(fail("morph primitive weight count mismatch"));
        }
        for (binding, weights) in self.morph.primitives.iter().zip(weights) {
            if let Some(binding) = binding {
                binding.geometry.validate_weights(weights)?;
            } else if !weights.is_empty() {
                return Err(fail("weights supplied for a non-morph primitive"));
            }
        }
        let pose = self.sample_pose(clip, time)?;
        let mut evaluated = self.clone();
        for ((primitive, binding), weights) in evaluated
            .primitives
            .iter_mut()
            .zip(&self.morph.primitives)
            .zip(weights)
        {
            if let Some(binding) = binding {
                let mut deformed = binding.geometry.evaluate(weights)?;
                deformed.color = primitive.color;
                deformed.base_color_texture = primitive.base_color_texture;
                *primitive = deformed;
            }
        }
        evaluated.morph = Default::default();
        evaluated.scene_meshes(&pose)
    }
}
