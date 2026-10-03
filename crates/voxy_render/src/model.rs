//! Explicit-buffer glTF import. No filesystem or network access is performed.
use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use gltf::animation::util::ReadOutputs;
use voxy_animation::{
    AnimationClip, Interpolation, Joint, JointTangents, JointTrack, Playback, QuatKey, Skeleton,
    TrackInterpolation, Transform, Vec3Key,
};

use crate::{SceneMesh, SceneVertex, SkinnedMesh, SkinnedVertex};

#[derive(Clone, Debug)]
pub enum ModelGeometry {
    Static(SceneMesh),
    Skinned(SkinnedMesh),
}

#[derive(Clone, Copy, Debug)]
pub struct ModelTexture {
    pub image: usize,
    pub sampling: crate::TextureSampling,
    pub use_mips: bool,
}

#[derive(Clone, Debug)]
pub struct ModelPrimitive {
    pub geometry: ModelGeometry,
    pub color: [f32; 4],
    pub base_color_texture: Option<ModelTexture>,
}

/// A single mesh instance with its complete node hierarchy and optional skin.
#[derive(Clone, Debug)]
pub struct ModelAsset {
    pub primitives: Vec<ModelPrimitive>,
    pub skeleton: Skeleton,
    pub animations: Vec<Arc<AnimationClip>>,
    joint_names: Vec<Option<Arc<str>>>,
    mesh_joint: usize,
    skinned: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelLimits {
    pub bytes: usize,
    pub vertices: usize,
    pub indices: usize,
    pub keys: usize,
}
impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            vertices: 1_000_000,
            indices: 3_000_000,
            keys: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelError(pub String);
impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ModelError {}
fn fail(message: impl Into<String>) -> ModelError {
    ModelError(message.into())
}

impl ModelAsset {
    /// Original authored node names, in skeleton order. Unnamed nodes have no stable name.
    pub fn joint_names(&self) -> &[Option<Arc<str>>] {
        &self.joint_names
    }

    /// Resolve an exact, unique authored name in this imported revision.
    /// Missing or ambiguous names fail rather than selecting a different joint.
    pub fn resolve_joint_name(&self, name: &str) -> Result<u16, ModelError> {
        let mut matches = self.joint_names.iter().enumerate()
            .filter(|(_, candidate)| candidate.as_deref() == Some(name));
        let (index, _) = matches.next().ok_or_else(|| fail("motion bone name not found"))?;
        if matches.next().is_some() {
            return Err(fail("motion bone name is ambiguous"));
        }
        u16::try_from(index).map_err(|_| fail("motion bone index exceeds supported range"))
    }

    /// Imports GLB's embedded buffer, or glTF buffers provided in document order.
    ///
    /// # Errors
    /// Rejects budgets, malformed data, multiple mesh instances/skins, morph targets,
    /// textured materials and non-linear animation. External URIs are never opened.
    // Node indices are bounded to MAX_JOINTS above.
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    pub fn parse(
        source: &[u8],
        buffers: &[&[u8]],
        limits: ModelLimits,
    ) -> Result<Self, ModelError> {
        let total = buffers
            .iter()
            .try_fold(source.len(), |n, b| n.checked_add(b.len()));
        if total.is_none_or(|n| n > limits.bytes) {
            return Err(fail("model byte budget exceeded"));
        }
        let gltf = gltf::Gltf::from_slice(source).map_err(|e| fail(e.to_string()))?;
        if gltf.document.extensions_required().next().is_some() {
            return Err(fail("required glTF extensions unsupported"));
        }
        let nodes: Vec<_> = gltf.nodes().collect();
        if nodes.is_empty() || nodes.len() > voxy_animation::MAX_JOINTS {
            return Err(fail("node count outside skeleton budget"));
        }
        let mut data = Vec::new();
        for buffer in gltf.buffers() {
            let bytes = match buffer.source() {
                gltf::buffer::Source::Bin => gltf.blob.as_deref(),
                gltf::buffer::Source::Uri(_) => buffers.get(buffer.index()).copied(),
            }
            .ok_or_else(|| fail("missing explicit buffer"))?;
            if bytes.len() < buffer.length() {
                return Err(fail("truncated buffer"));
            }
            data.push(&bytes[..buffer.length()]);
        }
        // Validate all accessor ranges before readers form slices.
        for accessor in gltf.accessors() {
            if accessor.sparse().is_some() {
                return Err(fail("sparse accessors unsupported"));
            }
            let view = accessor
                .view()
                .ok_or_else(|| fail("accessor without buffer view"))?;
            let stride = view.stride().unwrap_or(accessor.size());
            let end = accessor
                .count()
                .saturating_sub(1)
                .checked_mul(stride)
                .and_then(|n| n.checked_add(accessor.offset()))
                .and_then(|n| {
                    n.checked_add(if accessor.count() == 0 {
                        0
                    } else {
                        accessor.size()
                    })
                });
            if stride < accessor.size()
                || end.is_none_or(|n| n > view.length())
                || view
                    .offset()
                    .checked_add(view.length())
                    .is_none_or(|n| n > data[view.buffer().index()].len())
            {
                return Err(fail("accessor outside buffer bounds"));
            }
        }
        let meshes: Vec<_> = nodes.iter().filter(|n| n.mesh().is_some()).collect();
        if meshes.len() != 1 {
            return Err(fail("expected one mesh instance"));
        }
        let mesh_node = meshes[0];
        let skin = mesh_node.skin();
        if gltf.skins().count() > 1 {
            return Err(fail("multiple skins unsupported"));
        }
        let mut parents = vec![None; nodes.len()];
        for node in &nodes {
            for child in node.children() {
                if parents[child.index()].replace(node.index()).is_some() {
                    return Err(fail("node has multiple parents"));
                }
            }
        }
        let mut order = Vec::new();
        while order.len() < nodes.len() {
            let before = order.len();
            for (index, parent) in parents.iter().enumerate() {
                if !order.contains(&index) && parent.is_none_or(|p| order.contains(&p)) {
                    order.push(index);
                }
            }
            if before == order.len() {
                return Err(fail("cyclic node hierarchy"));
            }
        }
        let mut mapping = vec![0; nodes.len()];
        for (joint, &node) in order.iter().enumerate() {
            mapping[node] = joint;
        }
        let mut inverse = vec![Mat4::IDENTITY; nodes.len()];
        let skin_nodes: Vec<_> = skin
            .as_ref()
            .map(|s| s.joints().collect())
            .unwrap_or_default();
        let unique: std::collections::HashSet<_> =
            skin_nodes.iter().map(gltf::Node::index).collect();
        if unique.len() != skin_nodes.len() {
            return Err(fail("duplicate skin joint"));
        }
        if let Some(skin) = &skin {
            let matrices: Vec<_> = skin
                .reader(|b| data.get(b.index()).copied())
                .read_inverse_bind_matrices()
                .map_or_else(
                    || vec![Mat4::IDENTITY; skin_nodes.len()],
                    |r| r.map(|m| Mat4::from_cols_array_2d(&m)).collect(),
                );
            if matrices.len() != skin_nodes.len() {
                return Err(fail("inverse bind count mismatch"));
            }
            for (node, matrix) in skin_nodes.iter().zip(matrices) {
                inverse[node.index()] = matrix;
            }
        }
        let mut joints = Vec::new();
        for &index in &order {
            let (translation, rotation, scale) = nodes[index].transform().decomposed();
            let transform = Transform {
                translation: Vec3::from_array(translation),
                rotation: Quat::from_array(rotation),
                scale: Vec3::from_array(scale),
            };
            if !transform.matrix().abs_diff_eq(
                Mat4::from_cols_array_2d(&nodes[index].transform().matrix()),
                1e-4,
            ) {
                return Err(fail("node matrix cannot be represented by TRS"));
            }
            joints.push(Joint {
                name: Arc::from(format!("{}#{index}", nodes[index].name().unwrap_or("node"))),
                parent: parents[index].map(|p| mapping[p] as u16),
                bind_local: transform,
                inverse_bind: inverse[index],
            });
        }
        let joint_names = order.iter().map(|&index| nodes[index].name().map(Arc::from)).collect();
        let skeleton = Skeleton::new(joints).map_err(|e| fail(e.to_string()))?;
        let mut primitives = Vec::new();
        let (mut vertex_count, mut index_count) = (0usize, 0usize);
        for primitive in mesh_node
            .mesh()
            .ok_or_else(|| fail("missing mesh"))?
            .primitives()
        {
            if primitive.mode() != gltf::mesh::Mode::Triangles
                || primitive.morph_targets().next().is_some()
            {
                return Err(fail(
                    "only triangle primitives without morph targets are supported",
                ));
            }
            let material = primitive.material();
            let pbr = material.pbr_metallic_roughness();
            if pbr.metallic_roughness_texture().is_some()
                || material.normal_texture().is_some()
                || material.occlusion_texture().is_some()
                || material.emissive_texture().is_some()
            {
                return Err(fail("non-base glTF material maps unsupported"));
            }
            if pbr.base_color_texture().is_some()
                && material.alpha_mode() != gltf::material::AlphaMode::Opaque
            {
                return Err(fail("textured alpha modes unsupported"));
            }
            let base_color_texture = pbr
                .base_color_texture()
                .map(|info| {
                    if info.tex_coord() != 0
                        || primitive.get(&gltf::Semantic::TexCoords(0)).is_none()
                    {
                        return Err(fail("base color texture requires TEXCOORD_0"));
                    }
                    let sampler = info.texture().sampler();
                    let wrap = |mode| match mode {
                        gltf::texture::WrappingMode::ClampToEdge => crate::TextureWrap::Clamp,
                        gltf::texture::WrappingMode::Repeat => crate::TextureWrap::Repeat,
                        gltf::texture::WrappingMode::MirroredRepeat => crate::TextureWrap::Mirror,
                    };
                    Ok(ModelTexture {
                        image: info.texture().source().index(),
                        use_mips: !matches!(
                            sampler.min_filter(),
                            Some(
                                gltf::texture::MinFilter::Nearest
                                    | gltf::texture::MinFilter::Linear
                            )
                        ),
                        sampling: crate::TextureSampling {
                            wrap_u: wrap(sampler.wrap_s()),
                            wrap_v: wrap(sampler.wrap_t()),
                            min_filter: if matches!(
                                sampler.min_filter(),
                                Some(
                                    gltf::texture::MinFilter::Nearest
                                        | gltf::texture::MinFilter::NearestMipmapNearest
                                        | gltf::texture::MinFilter::NearestMipmapLinear
                                )
                            ) {
                                crate::TextureFilter::Nearest
                            } else {
                                crate::TextureFilter::Linear
                            },
                            mag_filter: if sampler.mag_filter()
                                == Some(gltf::texture::MagFilter::Nearest)
                            {
                                crate::TextureFilter::Nearest
                            } else {
                                crate::TextureFilter::Linear
                            },
                            mipmap_filter: Some(
                                if matches!(
                                    sampler.min_filter(),
                                    Some(
                                        gltf::texture::MinFilter::NearestMipmapNearest
                                            | gltf::texture::MinFilter::LinearMipmapNearest
                                    )
                                ) {
                                    crate::TextureFilter::Nearest
                                } else {
                                    crate::TextureFilter::Linear
                                },
                            ),
                            anisotropy: 1,
                        },
                    })
                })
                .transpose()?;
            let positions_accessor = primitive
                .get(&gltf::Semantic::Positions)
                .ok_or_else(|| fail("missing positions"))?;
            for (semantic, accessor) in primitive.attributes() {
                if accessor.count() != positions_accessor.count() {
                    return Err(fail(format!("attribute count mismatch: {semantic:?}")));
                }
            }
            vertex_count = vertex_count
                .checked_add(positions_accessor.count())
                .ok_or_else(|| fail("vertex overflow"))?;
            let count = primitive
                .indices()
                .map_or(positions_accessor.count(), |a| a.count());
            index_count = index_count
                .checked_add(count)
                .ok_or_else(|| fail("index overflow"))?;
            if vertex_count > limits.vertices || index_count > limits.indices {
                return Err(fail("geometry budget exceeded"));
            }
            let reader = primitive.reader(|b| data.get(b.index()).copied());
            let positions: Vec<_> = reader
                .read_positions()
                .ok_or_else(|| fail("invalid positions"))?
                .collect();
            let position_count =
                u32::try_from(positions.len()).map_err(|_| fail("too many positions"))?;
            let indices: Vec<u32> = reader
                .read_indices()
                .map_or_else(|| (0..position_count).collect(), |r| r.into_u32().collect());
            let uvs: Vec<_> = reader.read_tex_coords(0).map_or_else(
                || vec![[0.; 2]; positions.len()],
                |r| r.into_f32().collect(),
            );
            if uvs.len() != positions.len() {
                return Err(fail("UV count mismatch"));
            }
            let color = pbr.base_color_factor();
            let geometry = if skin.is_some() {
                let normals: Option<Vec<_>> = reader.read_normals().map(Iterator::collect);
                let ids: Vec<_> = reader
                    .read_joints(0)
                    .ok_or_else(|| fail("missing JOINTS_0"))?
                    .into_u16()
                    .collect();
                let weights: Vec<_> = reader
                    .read_weights(0)
                    .ok_or_else(|| fail("missing WEIGHTS_0"))?
                    .into_f32()
                    .collect();
                if reader.read_joints(1).is_some() || reader.read_weights(1).is_some() {
                    return Err(fail("more than four influences unsupported"));
                }
                if normals
                    .as_ref()
                    .is_some_and(|normals| normals.len() != positions.len())
                    || ids.len() != positions.len()
                    || weights.len() != positions.len()
                {
                    return Err(fail("skin attribute count mismatch"));
                }
                let mut vertices = Vec::new();
                for i in 0..positions.len() {
                    let mut joints = [0; 4];
                    let weights = quantize_weights(weights[i])?;
                    for k in 0..4 {
                        if weights[k] > 0 {
                            let node = skin_nodes
                                .get(usize::from(ids[i][k]))
                                .ok_or_else(|| fail("joint index outside skin"))?;
                            joints[k] = mapping[node.index()] as u16;
                        }
                    }
                    vertices.push(SkinnedVertex {
                        position: positions[i],
                        normal: normals.as_ref().map_or([0.; 3], |normals| normals[i]),
                        uv: uvs[i],
                        joints,
                        weights,
                    });
                }
                // A zero normal stream marks absent NORMAL. The scene shader
                // derives one flat face normal from the current deformed geometry.
                ModelGeometry::Skinned(
                    SkinnedMesh::new(vertices, indices, nodes.len() as u16)
                        .map_err(|e| fail(e.to_string()))?,
                )
            } else {
                ModelGeometry::Static(
                    SceneMesh::new(
                        positions
                            .iter()
                            .zip(&uvs)
                            .map(|(&position, &uv)| SceneVertex {
                                position,
                                uv,
                                color,
                            })
                            .collect(),
                        indices,
                    )
                    .map_err(|e| fail(e.to_string()))?,
                )
            };
            primitives.push(ModelPrimitive {
                geometry,
                color,
                base_color_texture,
            });
        }
        if primitives.is_empty() {
            return Err(fail("empty model"));
        }
        let mut animations = Vec::new();
        let mut key_count = 0usize;
        for animation in gltf.animations() {
            let mut tracks = vec![JointTrack::default(); nodes.len()];
            let mut modes = vec![TrackInterpolation::default(); nodes.len()];
            let mut tangents = vec![JointTangents::default(); nodes.len()];
            let mut seen = std::collections::HashSet::new();
            let mut duration = 0f32;
            for channel in animation.channels() {
                let mode = match channel.sampler().interpolation() {
                    gltf::animation::Interpolation::Linear => Interpolation::Linear,
                    gltf::animation::Interpolation::Step => Interpolation::Step,
                    gltf::animation::Interpolation::CubicSpline => Interpolation::CubicSpline,
                };
                let target = channel.target();
                if !seen.insert((target.node().index(), target.property() as u8)) {
                    return Err(fail("duplicate animation channel"));
                }
                let output_count = channel
                    .sampler()
                    .input()
                    .count()
                    .checked_mul(if mode == Interpolation::CubicSpline {
                        3
                    } else {
                        1
                    })
                    .ok_or_else(|| fail("animation key overflow"))?;
                if channel.sampler().output().count() != output_count {
                    return Err(fail("animation output count mismatch"));
                }
                key_count = key_count
                    .checked_add(output_count)
                    .ok_or_else(|| fail("key overflow"))?;
                if key_count > limits.keys {
                    return Err(fail("animation key budget exceeded"));
                }
                let reader = channel.reader(|b| data.get(b.index()).copied());
                let times: Vec<_> = reader
                    .read_inputs()
                    .ok_or_else(|| fail("missing animation times"))?
                    .collect();
                duration = duration.max(*times.last().ok_or_else(|| fail("empty animation"))?);
                let track = &mut tracks[mapping[target.node().index()]];
                let interpolation = &mut modes[mapping[target.node().index()]];
                let derivatives = &mut tangents[mapping[target.node().index()]];
                match reader
                    .read_outputs()
                    .ok_or_else(|| fail("missing animation outputs"))?
                {
                    ReadOutputs::Translations(values) => {
                        interpolation.translation = mode;
                        let (values, curve_tangents) =
                            animation_values(values.map(Vec3::from_array), times.len(), mode)?;
                        derivatives.translation = curve_tangents;
                        track.translations = times
                            .iter()
                            .zip(values)
                            .map(|(&time, value)| Vec3Key { time, value: value })
                            .collect();
                    }
                    ReadOutputs::Scales(values) => {
                        interpolation.scale = mode;
                        let (values, curve_tangents) =
                            animation_values(values.map(Vec3::from_array), times.len(), mode)?;
                        derivatives.scale = curve_tangents;
                        track.scales = times
                            .iter()
                            .zip(values)
                            .map(|(&time, value)| Vec3Key { time, value: value })
                            .collect();
                    }
                    ReadOutputs::Rotations(values) => {
                        interpolation.rotation = mode;
                        let (values, curve_tangents) = animation_values(
                            values.into_f32().map(glam::Vec4::from_array),
                            times.len(),
                            mode,
                        )?;
                        derivatives.rotation = curve_tangents;
                        track.rotations = times
                            .iter()
                            .zip(values)
                            .map(|(&time, value)| QuatKey {
                                time,
                                value: Quat::from_array(value.to_array()),
                            })
                            .collect();
                    }
                    ReadOutputs::MorphTargetWeights(_) => {
                        return Err(fail("morph animation unsupported"));
                    }
                }
            }
            animations.push(Arc::new(
                AnimationClip::new_with_tangents(
                    animation.name().unwrap_or("animation"),
                    duration.max(f32::EPSILON),
                    Playback::Loop,
                    tracks,
                    modes,
                    tangents,
                    &skeleton,
                )
                .map_err(|e| fail(e.to_string()))?,
            ));
        }
        Ok(Self {
            joint_names,
            primitives,
            skeleton,
            animations,
            mesh_joint: mapping[mesh_node.index()],
            skinned: skin.is_some(),
        })
    }

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
        let palette = pose
            .skin_matrices(&self.skeleton)
            .map_err(|e| fail(e.to_string()))?;
        let model = self.mesh_transform(pose)?;
        self.primitives
            .iter()
            .map(|primitive| {
                let (vertices, indices) = match &primitive.geometry {
                    ModelGeometry::Static(mesh) => (
                        mesh.vertices()
                            .iter()
                            .map(|v| SceneVertex {
                                position: model
                                    .transform_point3(Vec3::from_array(v.position))
                                    .to_array(),
                                ..*v
                            })
                            .collect(),
                        mesh.indices().to_vec(),
                    ),
                    ModelGeometry::Skinned(mesh) => {
                        return mesh
                            .posed_scene_mesh(&palette, Mat4::IDENTITY, primitive.color)
                            .map_err(|error| fail(error.to_string()));
                    }
                };
                SceneMesh::new(vertices, indices).map_err(|e| fail(e.to_string()))
            })
            .collect()
    }
}

fn animation_values<T: Copy>(
    values: impl Iterator<Item = T>,
    count: usize,
    mode: Interpolation,
) -> Result<(Vec<T>, Vec<[T; 2]>), ModelError> {
    let values: Vec<_> = values.collect();
    if mode == Interpolation::CubicSpline {
        if count < 2
            || values.len()
                != count
                    .checked_mul(3)
                    .ok_or_else(|| fail("animation key overflow"))?
        {
            return Err(fail("invalid cubic animation stream"));
        }
        Ok((
            values.chunks_exact(3).map(|key| key[1]).collect(),
            values.chunks_exact(3).map(|key| [key[0], key[2]]).collect(),
        ))
    } else if values.len() == count {
        Ok((values, Vec::new()))
    } else {
        Err(fail("animation output count mismatch"))
    }
}

// Values are checked nonnegative and normalized into the u16 range.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn quantize_weights(weights: [f32; 4]) -> Result<[u16; 4], ModelError> {
    if weights.iter().any(|w| !w.is_finite() || *w < 0.) {
        return Err(fail("invalid skin weights"));
    }
    let sum: f64 = weights.iter().map(|&w| f64::from(w)).sum();
    if sum <= 0. {
        return Err(fail("zero skin weights"));
    }
    let mut result = weights.map(|w| (f64::from(w) / sum * f64::from(u16::MAX)).floor() as u16);
    let remainder = u32::from(u16::MAX) - result.iter().map(|&w| u32::from(w)).sum::<u32>();
    let largest = (0..4)
        .max_by(|&a, &b| weights[a].total_cmp(&weights[b]))
        .expect("four weights");
    result[largest] += remainder as u16;
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn imported_fox_selects_animated_hip_instead_of_static_container() {
        let bytes = include_bytes!("../examples/assets/fox/Fox.glb");
        let gltf = gltf::Gltf::from_slice(bytes).unwrap();
        let asset = ModelAsset::parse(
            bytes,
            &[gltf.blob.as_deref().unwrap()],
            ModelLimits::default(),
        )
        .unwrap();
        let joint = asset
            .skeleton
            .joints()
            .iter()
            .position(|j| j.name.starts_with("b_Hip_01#"))
            .unwrap();
        let clip = asset.animations[2].clone();
        let mut animator = voxy_animation::Animator::new(clip);
        let first = animator.advance(&asset.skeleton, 0.1).unwrap();
        assert!(first.root_motion.length() < 1e-6);
        animator.set_root_motion_joint(joint as u16).unwrap();
        let frame = animator.advance(&asset.skeleton, 0.1).unwrap();
        let before = asset.sample_pose(Some(2), 0.1).unwrap();
        let after = asset.sample_pose(Some(2), 0.2).unwrap();
        let expected = after.local()[joint].translation - before.local()[joint].translation;
        assert!(expected.length() > 0.01);
        assert!(frame.root_motion.abs_diff_eq(expected, 1e-5));
        assert_eq!(frame.root_motion_joint, joint as u16);
        assert_eq!(frame.pose, after);
    }

    use super::*;

    #[test]
    fn gltf_step_translation_holds_until_key_and_linear_keeps_interpolating() {
        let (json, bytes) = fixture();
        let step = ModelAsset::parse(
            json.replace("LINEAR", "STEP").as_bytes(),
            &[&bytes],
            ModelLimits::default(),
        )
        .unwrap();
        let linear = ModelAsset::parse(json.as_bytes(), &[&bytes], ModelLimits::default()).unwrap();
        let position = |model: &ModelAsset, time| {
            let pose = model.sample_pose(Some(0), time).unwrap();
            model.scene_meshes(&pose).unwrap()[0].vertices()[0].position
        };
        assert_eq!(position(&step, 0.75), [0.; 3]);
        assert_eq!(position(&linear, 0.75), [1.5, 0., 0.]);
        // Loop endpoint wraps, while the last value is selected by the sampler
        // at an internal exact key (covered independently in voxy_animation).
        assert_eq!(position(&step, 1.0), [0.; 3]);
        assert!(
            ModelAsset::parse(
                json.replace("LINEAR", "CUBICSPLINE").as_bytes(),
                &[&bytes],
                ModelLimits::default()
            )
            .is_err()
        );
    }

    #[test]
    fn cubic_gltf_triplets_preserve_nonzero_derivatives_and_bound_decoded_keys() {
        let (json, mut bytes) = fixture();
        let mut document: serde_json::Value = serde_json::from_str(&json).unwrap();
        bytes.truncate(144);
        for value in [1.0_f32, 3.0] {
            bytes.extend(value.to_le_bytes());
        }
        for vector in [
            [0.0_f32; 3],
            [0.; 3],
            [4., 0., 0.],
            [-2., 0., 0.],
            [2., 0., 0.],
            [0.; 3],
        ] {
            for value in vector {
                bytes.extend(value.to_le_bytes());
            }
        }
        document["buffers"][0]["byteLength"] = bytes.len().into();
        document["bufferViews"][5]["byteLength"] = 72.into();
        document["accessors"][4]["min"] = serde_json::json!([1]);
        document["accessors"][4]["max"] = serde_json::json!([3]);
        document["accessors"][5]["count"] = 6.into();
        document["animations"][0]["samplers"][0]["interpolation"] = "CUBICSPLINE".into();
        let json = serde_json::to_vec(&document).unwrap();
        let asset = ModelAsset::parse(&json, &[&bytes], ModelLimits::default()).unwrap();
        let pose = asset.sample_pose(Some(0), 1.5).unwrap();
        assert!(
            Vec3::from_array(asset.scene_meshes(&pose).unwrap()[0].vertices()[0].position)
                .abs_diff_eq(Vec3::X * 1.625, 1e-6)
        );
        assert!(
            ModelAsset::parse(
                &json,
                &[&bytes],
                ModelLimits {
                    keys: 5,
                    ..ModelLimits::default()
                }
            )
            .is_err()
        );
        assert!(
            ModelAsset::parse(
                &json,
                &[&bytes],
                ModelLimits {
                    keys: 6,
                    ..ModelLimits::default()
                }
            )
            .is_ok()
        );
        document["accessors"][5]["count"] = 5.into();
        assert!(
            ModelAsset::parse(
                &serde_json::to_vec(&document).unwrap(),
                &[&bytes],
                ModelLimits::default()
            )
            .is_err()
        );
    }

    #[test]
    fn authored_bone_names_survive_inserted_nodes_and_reject_ambiguity() {
        let (json, bytes) = fixture();
        let mut doc: serde_json::Value = serde_json::from_str(&json).unwrap();
        doc["nodes"][1]["name"] = "motion#1".into();
        let parse = |doc: &serde_json::Value| ModelAsset::parse(
            &serde_json::to_vec(doc).unwrap(), &[&bytes], ModelLimits::default()).unwrap();
        let original = parse(&doc);
        assert_eq!(original.resolve_joint_name("motion#1").unwrap(), 1);
        doc["nodes"].as_array_mut().unwrap().push(serde_json::json!({"name":"new sibling"}));
        doc["nodes"].as_array_mut().unwrap().swap(1, 2);
        doc["nodes"][0]["children"] = serde_json::json!([1, 2]);
        let reordered = parse(&doc);
        assert_eq!(reordered.resolve_joint_name("motion#1").unwrap(), 2);
        assert_eq!(reordered.joint_names()[2].as_deref(), Some("motion#1"));
        assert!(reordered.resolve_joint_name("missing").is_err());
        doc["nodes"][1]["name"] = "motion#1".into();
        assert!(parse(&doc).resolve_joint_name("motion#1").is_err());
    }

    #[test]
    fn gltf_mirrored_root_preserves_positions_and_inverse_transpose_normals() {
        let (json, bytes) = fixture();
        let mut document: serde_json::Value = serde_json::from_str(&json).unwrap();
        document["nodes"][0]["scale"] = serde_json::json!([-2, 3, 4]);
        let model = ModelAsset::parse(
            &serde_json::to_vec(&document).unwrap(),
            &[&bytes],
            ModelLimits::default(),
        )
        .unwrap();
        let pose = model.sample_pose(None, 0.0).unwrap();
        let mesh = model.scene_meshes(&pose).unwrap().remove(0);
        assert_eq!(mesh.vertices()[1].position, [-2., 0., 0.]);
        assert_eq!(mesh.vertices()[2].position, [0., 3., 0.]);
        assert!(
            mesh.authored_normals()
                .unwrap()
                .iter()
                .all(|n| *n == [0., 0., 1.])
        );
        document["nodes"][0]["scale"] = serde_json::json!([-2, 3, 0]);
        assert!(
            ModelAsset::parse(
                &serde_json::to_vec(&document).unwrap(),
                &[&bytes],
                ModelLimits::default()
            )
            .is_err()
        );
    }

    fn fixture() -> (String, Vec<u8>) {
        let json = r#"{
          "asset":{"version":"2.0"},
          "buffers":[{"uri":"model.bin","byteLength":176}],
          "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":36},
            {"buffer":0,"byteOffset":72,"byteLength":24},
            {"buffer":0,"byteOffset":96,"byteLength":48},
            {"buffer":0,"byteOffset":144,"byteLength":8},
            {"buffer":0,"byteOffset":152,"byteLength":24}],
          "accessors":[
            {"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},
            {"bufferView":1,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":2,"componentType":5123,"count":3,"type":"VEC4"},
            {"bufferView":3,"componentType":5126,"count":3,"type":"VEC4"},
            {"bufferView":4,"componentType":5126,"count":2,"type":"SCALAR","min":[0],"max":[1]},
            {"bufferView":5,"componentType":5126,"count":2,"type":"VEC3"}],
          "nodes":[{"name":"root","children":[1]},{"mesh":0,"skin":0}],
          "skins":[{"joints":[0]}],
          "meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1,"JOINTS_0":2,"WEIGHTS_0":3}}]}],
          "animations":[{"name":"move","samplers":[{"input":4,"output":5,"interpolation":"LINEAR"}],"channels":[{"sampler":0,"target":{"node":0,"path":"translation"}}]}]
        }"#.to_owned();
        let mut bytes = Vec::new();
        for value in [
            0f32, 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 1.,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0u8; 24]);
        for value in [
            1f32, 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 2., 0., 0.,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        assert_eq!(bytes.len(), 176);
        (json, bytes)
    }

    #[test]
    fn imports_skin_and_animation_and_deforms_vertex() {
        let (json, bytes) = fixture();
        let asset = ModelAsset::parse(json.as_bytes(), &[&bytes], ModelLimits::default()).unwrap();
        let mut animator = voxy_animation::Animator::new(Arc::clone(&asset.animations[0]));
        let frame = animator.advance(&asset.skeleton, 0.5).unwrap();
        let ModelGeometry::Skinned(mesh) = &asset.primitives[0].geometry else {
            panic!("expected skin")
        };
        let matrix =
            asset.skin_matrices(&frame.pose).unwrap()[usize::from(mesh.vertices()[0].joints[0])];
        assert!(
            matrix
                .transform_point3(Vec3::ZERO)
                .abs_diff_eq(Vec3::X, 1e-6)
        );
        let meshes = asset.scene_meshes(&frame.pose).unwrap();
        assert!(Vec3::from_array(meshes[0].vertices()[0].position).abs_diff_eq(Vec3::X, 1e-6));
        assert!(
            asset
                .mesh_transform(&frame.pose)
                .unwrap()
                .transform_point3(Vec3::ZERO)
                .abs_diff_eq(Vec3::X, 1e-6)
        );
    }

    #[test]
    fn sampled_instances_have_independent_clocks_and_reject_invalid_requests() {
        let (json, bytes) = fixture();
        let asset = ModelAsset::parse(json.as_bytes(), &[&bytes], ModelLimits::default()).unwrap();
        let first = asset.sample_pose(Some(0), 0.25).unwrap();
        let second = asset.sample_pose(Some(0), 0.75).unwrap();
        let position = |pose: &voxy_animation::Pose| {
            Vec3::from_array(asset.scene_meshes(pose).unwrap()[0].vertices()[0].position)
        };
        assert!(position(&first).abs_diff_eq(Vec3::X * 0.5, 1e-6));
        assert!(position(&second).abs_diff_eq(Vec3::X * 1.5, 1e-6));
        assert_eq!(
            position(&first),
            position(&asset.sample_pose(Some(0), 0.25).unwrap())
        );
        assert!(position(&asset.sample_pose(None, 10.0).unwrap()).abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(asset.sample_pose(Some(1), 0.0).is_err());
        for time in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(asset.sample_pose(Some(0), time).is_err());
            assert!(asset.sample_pose(None, time).is_err());
        }
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn embedded_glb_matches_explicit_buffers() {
        let (json, mut bytes) = fixture();
        let mut json = json.replace("\"uri\":\"model.bin\",", "").into_bytes();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        let mut glb = Vec::new();
        for word in [
            0x4654_6c67u32,
            2,
            (12 + 8 + json.len() + 8 + bytes.len()) as u32,
            json.len() as u32,
            0x4e4f_534a,
        ] {
            glb.extend(word.to_le_bytes());
        }
        glb.extend(json);
        glb.extend((bytes.len() as u32).to_le_bytes());
        glb.extend(0x004e_4942u32.to_le_bytes());
        glb.extend(bytes);
        let asset = ModelAsset::parse(&glb, &[], ModelLimits::default()).unwrap();
        assert_eq!(asset.animations[0].name(), "move");
        assert_eq!(asset.skeleton.joints().len(), 2);
    }

    #[test]
    fn imports_static_and_rejects_unsupported_and_broken_data() {
        let (json, bytes) = fixture();
        let static_json = json.replace(",\"skin\":0", "");
        let asset =
            ModelAsset::parse(static_json.as_bytes(), &[&bytes], ModelLimits::default()).unwrap();
        assert!(matches!(
            asset.primitives[0].geometry,
            ModelGeometry::Static(_)
        ));
        for bad in [
            json.replace("LINEAR", "CUBICSPLINE"),
            json.replace("\"byteOffset\":152", "\"byteOffset\":172"),
        ] {
            assert!(ModelAsset::parse(bad.as_bytes(), &[&bytes], ModelLimits::default()).is_err());
        }
        assert!(ModelAsset::parse(json.as_bytes(), &[], ModelLimits::default()).is_err());
        assert!(
            ModelAsset::parse(json.as_bytes(), &[&bytes[..100]], ModelLimits::default()).is_err()
        );
        assert!(
            ModelAsset::parse(
                json.as_bytes(),
                &[&bytes],
                ModelLimits {
                    vertices: 2,
                    ..ModelLimits::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn weight_quantization_preserves_exact_sum() {
        for weights in [[0.25; 4], [0.1, 0.2, 0.3, 0.4], [0., 0., 0., 5.]] {
            assert_eq!(
                quantize_weights(weights)
                    .unwrap()
                    .iter()
                    .map(|&w| u32::from(w))
                    .sum::<u32>(),
                65535
            );
        }
        assert!(quantize_weights([0.; 4]).is_err());
        assert!(quantize_weights([f32::NAN; 4]).is_err());
    }
}

#[cfg(test)]
mod import_tests {
    use super::*;

    const TRIANGLE: &[u8] = br#"{
      "asset":{"version":"2.0"},
      "buffers":[{"uri":"must-not-be-opened.bin","byteLength":36}],
      "bufferViews":[{"buffer":0,"byteLength":36}],
      "accessors":[{"bufferView":0,"componentType":5126,"count":3,
        "type":"VEC3","min":[0,0,0],"max":[1,1,0]}],
      "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
      "nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0
    }"#;

    fn triangle_buffer() -> Vec<u8> {
        [0_f32, 0., 0., 1., 0., 0., 0., 1., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect()
    }

    #[test]
    fn imports_explicit_triangle_without_opening_uri() {
        let buffer = triangle_buffer();
        let asset = ModelAsset::parse(TRIANGLE, &[&buffer], ModelLimits::default()).unwrap();
        assert_eq!(asset.primitives.len(), 1);
        let ModelGeometry::Static(mesh) = &asset.primitives[0].geometry else {
            panic!("static triangle unexpectedly skinned");
        };
        assert_eq!(mesh.vertices().len(), 3);
        assert_eq!(mesh.indices(), &[0, 1, 2]);
        assert_eq!(
            mesh.vertices()[1].position.map(f32::to_bits),
            [1.0_f32, 0.0, 0.0].map(f32::to_bits)
        );
        assert!(asset.animations.is_empty());
        assert!(ModelAsset::parse(TRIANGLE, &[], ModelLimits::default()).is_err());
        assert!(ModelAsset::parse(TRIANGLE, &[&buffer[..35]], ModelLimits::default()).is_err());
    }

    #[test]
    fn rejects_geometry_and_combined_source_buffer_budgets() {
        let buffer = triangle_buffer();
        for limits in [
            ModelLimits {
                vertices: 2,
                ..ModelLimits::default()
            },
            ModelLimits {
                indices: 2,
                ..ModelLimits::default()
            },
            ModelLimits {
                bytes: TRIANGLE.len() + buffer.len() - 1,
                ..ModelLimits::default()
            },
        ] {
            assert!(ModelAsset::parse(TRIANGLE, &[&buffer], limits).is_err());
        }
    }
}
