//! Pose-specific quality bounds over one immutable skeletal vertex domain.
use std::{fmt, sync::Arc};

use glam::{Mat4, Vec3};

use crate::{
    CertifiedLodError, CertifiedLodIndexSet, CertifiedLodSubdivisionVariant, CertifiedLodVariant,
    LodError, LodLevel, LodPolicy, LodSurface, SceneCamera, SceneError, SceneMesh, SceneVertex,
    SkinnedMesh, SkinnedUploadError, certify_lod_error,
};

#[derive(Debug)]
struct Source {
    mesh: Arc<SkinnedMesh>,
    variants: Box<[Variant]>,
}
#[derive(Debug)]
struct Variant {
    indices: Vec<u32>,
    witnesses: Witnesses,
}
#[derive(Debug)]
enum Witnesses {
    Whole(
        Vec<crate::LodTriangleWitness>,
        Vec<crate::LodTriangleWitness>,
    ),
    Subdivided(
        Vec<crate::LodSubdivisionWitness>,
        Vec<crate::LodSubdivisionWitness>,
    ),
}

/// Shared skeletal attributes and index variants. Witnesses are retained so
/// deformation can be verified against the exact candidate pose, not rest pose.
#[derive(Clone, Debug)]
pub struct SkinnedLodMesh(Arc<Source>);

/// Immutable pose and its verified world-space geometric error envelope.
/// Quality describes these exact CPU-skinned f32 positions. It is not an
/// all-animation bound, an attribute bound, or proof of GPU arithmetic identity.
#[derive(Debug)]
pub struct PreparedSkinnedLod {
    source: SkinnedLodMesh,
    joints: Box<[Mat4]>,
    model: Mat4,
    positions: Box<[[f32; 3]]>,
    levels: Box<[LodLevel]>,
    min: Vec3,
    max: Vec3,
}

#[derive(Debug)]
pub enum SkinnedLodError {
    Certificate(CertifiedLodError),
    Skin(SkinnedUploadError),
}
impl fmt::Display for SkinnedLodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "skinned LOD error: {self:?}")
    }
}
impl std::error::Error for SkinnedLodError {}

impl SkinnedLodMesh {
    /// Validates complete bidirectional witnesses and monotonically decreasing
    /// index counts against the rest surface. One vertex/skin domain serves all
    /// levels. Preparation later re-verifies errors for each supplied pose.
    /// # Errors
    /// Rejects malformed variants or witnesses before publishing the source.
    pub fn new(
        mesh: Arc<SkinnedMesh>,
        variants: Vec<CertifiedLodVariant>,
    ) -> Result<Self, SkinnedLodError> {
        CertifiedLodIndexSet::new(
            mesh.vertices()
                .iter()
                .map(|vertex| vertex.position)
                .collect(),
            mesh.indices().to_vec(),
            variants
                .iter()
                .map(|variant| CertifiedLodVariant {
                    indices: variant.indices.clone(),
                    source_to_variant: variant.source_to_variant.clone(),
                    variant_to_source: variant.variant_to_source.clone(),
                })
                .collect(),
        )
        .map_err(SkinnedLodError::Certificate)?;
        Ok(Self(Arc::new(Source {
            mesh,
            variants: variants
                .into_iter()
                .map(|variant| Variant {
                    indices: variant.indices,
                    witnesses: Witnesses::Whole(
                        variant.source_to_variant,
                        variant.variant_to_source,
                    ),
                })
                .collect(),
        })))
    }

    /// Retains complete subdivision coverage for pose-specific re-verification.
    /// # Errors
    /// Rejects invalid surfaces, counts, cells, depth or incomplete witnesses.
    pub fn new_subdivided(
        mesh: Arc<SkinnedMesh>,
        variants: Vec<CertifiedLodSubdivisionVariant>,
    ) -> Result<Self, SkinnedLodError> {
        CertifiedLodIndexSet::new_subdivided(
            mesh.vertices()
                .iter()
                .map(|vertex| vertex.position)
                .collect(),
            mesh.indices().to_vec(),
            variants
                .iter()
                .map(|variant| CertifiedLodSubdivisionVariant {
                    indices: variant.indices.clone(),
                    source_to_variant: variant.source_to_variant.clone(),
                    variant_to_source: variant.variant_to_source.clone(),
                })
                .collect(),
        )
        .map_err(SkinnedLodError::Certificate)?;
        Ok(Self(Arc::new(Source {
            mesh,
            variants: variants
                .into_iter()
                .map(|variant| Variant {
                    indices: variant.indices,
                    witnesses: Witnesses::Subdivided(
                        variant.source_to_variant,
                        variant.variant_to_source,
                    ),
                })
                .collect(),
        })))
    }

    #[must_use]
    pub fn mesh(&self) -> &SkinnedMesh {
        &self.0.mesh
    }

    #[must_use]
    pub fn indices(&self, level: usize) -> Option<&[u32]> {
        if level == 0 {
            Some(self.mesh().indices())
        } else {
            self.0
                .variants
                .get(level - 1)
                .map(|variant| variant.indices.as_slice())
        }
    }

    /// Re-evaluates the same coverage witnesses on exact deformed positions.
    /// Work is linear in vertices and retained witnesses, without a new nearest
    /// surface search or copying skeletal attributes/index variants. A failed
    /// candidate does not mutate any prior prepared pose or source.
    /// # Errors
    /// Rejects palette mismatch, nonaffine/nonfinite transforms, overflowing
    /// positions, or a failed geometric certificate.
    pub fn prepare(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<PreparedSkinnedLod, SkinnedLodError> {
        let positions = self
            .mesh()
            .posed_positions(joints, model)
            .map_err(SkinnedLodError::Skin)?;
        let base = LodSurface {
            positions: &positions,
            indices: self.mesh().indices(),
        };
        let count = |indices: &[u32]| {
            u32::try_from(indices.len()).map_err(|_| {
                SkinnedLodError::Certificate(CertifiedLodError::Levels(LodError::InvalidLevels))
            })
        };
        let mut levels = vec![LodLevel {
            object_error: 0.0,
            index_count: count(self.mesh().indices())?,
        }];
        let mut error = 0.0_f64;
        for variant in &self.0.variants {
            let approximation = LodSurface {
                positions: &positions,
                indices: &variant.indices,
            };
            let measured = match &variant.witnesses {
                Witnesses::Whole(forward, reverse) => {
                    certify_lod_error(base, approximation, forward, reverse)
                }
                Witnesses::Subdivided(forward, reverse) => {
                    crate::certify_subdivided_lod_error(base, approximation, forward, reverse)
                }
            }
            .map_err(|error| SkinnedLodError::Certificate(CertifiedLodError::Certificate(error)))?;
            error = error.max(measured);
            levels.push(LodLevel {
                object_error: error,
                index_count: count(&variant.indices)?,
            });
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for position in &positions {
            let point = Vec3::from_array(*position);
            min = min.min(point);
            max = max.max(point);
        }
        Ok(PreparedSkinnedLod {
            source: self.clone(),
            joints: joints.into(),
            model,
            positions: positions.into_boxed_slice(),
            levels: levels.into_boxed_slice(),
            min,
            max,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        LOD_BARYCENTRIC_DENOMINATOR as D, LodTriangleWitness, SceneProjection, SkinnedVertex,
    };

    pub(crate) fn source() -> SkinnedLodMesh {
        let mesh = SkinnedMesh::new(
            [[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.], [0., 0., 0.]]
                .into_iter()
                .enumerate()
                .map(|(index, position)| SkinnedVertex {
                    position,
                    normal: [0., 0., 1.],
                    uv: [0.; 2],
                    joints: [u16::from(index == 3), 0, 0, 0],
                    weights: [u16::MAX, 0, 0, 0],
                })
                .collect(),
            vec![0, 1, 2, 0, 1, 3],
            2,
        )
        .unwrap();
        let identity = LodTriangleWitness {
            target_triangle: 0,
            weights: [[D, 0, 0], [0, D, 0], [0, 0, D]],
        };
        SkinnedLodMesh::new(
            Arc::new(mesh),
            vec![CertifiedLodVariant {
                indices: vec![0, 1, 2],
                source_to_variant: vec![
                    identity,
                    LodTriangleWitness {
                        target_triangle: 0,
                        weights: [[D, 0, 0], [0, D, 0], [D / 4, D / 4, D / 2]],
                    },
                ],
                variant_to_source: vec![identity],
            }],
        )
        .unwrap()
    }

    #[test]
    fn subdivision_archive_binds_exact_source_and_recertifies_deformation() {
        let original = source();
        let positions: Vec<_> = original
            .mesh()
            .vertices()
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        let base = original.indices(0).unwrap();
        let indices = original.indices(1).unwrap();
        let proof = crate::generate_subdivided_lod_witnesses(
            LodSurface {
                positions: &positions,
                indices: base,
            },
            LodSurface {
                positions: &positions,
                indices,
            },
            0,
            100,
        )
        .unwrap();
        let limits = crate::LodArchiveLimits {
            bytes: 4096,
            positions: 4,
            levels: 2,
            indices: 9,
            cells: 3,
        };
        let bytes = crate::encode_lod_archive(
            &positions,
            base,
            &[CertifiedLodSubdivisionVariant {
                indices: indices.to_vec(),
                source_to_variant: proof.source_to_approximation,
                variant_to_source: proof.approximation_to_source,
            }],
            limits,
        )
        .unwrap();
        let source =
            crate::decode_skinned_lod_archive(original.0.mesh.clone(), &bytes, limits).unwrap();
        let rest = source
            .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
            .unwrap();
        let deformed = source
            .prepare(
                &[Mat4::IDENTITY, Mat4::from_translation(Vec3::Z)],
                Mat4::IDENTITY,
            )
            .unwrap();
        assert!(rest.levels()[1].object_error < 1e-10);
        assert!(deformed.levels()[1].object_error >= 1.);
        assert_eq!(
            deformed.select_for_camera(camera(2.), [100, 100], POLICY, Some(1)),
            Ok(0)
        );
        let mut changed = original.mesh().vertices().to_vec();
        changed[0].position[0] = 0.;
        let changed = SkinnedMesh::new(changed, base.to_vec(), 2).unwrap();
        assert!(matches!(
            crate::decode_skinned_lod_archive(Arc::new(changed), &bytes, limits),
            Err(crate::SkinnedLodArchiveError::SourceMismatch)
        ));
        let changed_indices = SkinnedMesh::new(
            original.mesh().vertices().to_vec(),
            vec![0, 2, 1, 0, 1, 3],
            2,
        )
        .unwrap();
        assert!(matches!(
            crate::decode_skinned_lod_archive(Arc::new(changed_indices), &bytes, limits),
            Err(crate::SkinnedLodArchiveError::SourceMismatch)
        ));
        let mut truncated = bytes.clone();
        truncated.pop();
        assert!(
            crate::decode_skinned_lod_archive(original.0.mesh.clone(), &truncated, limits).is_err()
        );
        let mut tight = limits;
        tight.cells = 2;
        assert!(matches!(
            crate::decode_skinned_lod_archive(original.0.mesh.clone(), &bytes, tight),
            Err(crate::SkinnedLodArchiveError::Archive(
                crate::LodArchiveError::BudgetExceeded
            ))
        ));
        assert_eq!(source.indices(1).unwrap(), indices);
    }

    fn camera(span: f32) -> SceneCamera {
        SceneCamera {
            eye: Vec3::new(0., 0., 10.),
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection: SceneProjection::Orthographic {
                left: -span / 2.,
                right: span / 2.,
                bottom: -span / 2.,
                top: span / 2.,
                near: 0.1,
                far: 100.,
            },
        }
    }
    const POLICY: LodPolicy = LodPolicy {
        target_pixels: 2.,
        hysteresis: 0.1,
    };

    #[test]
    fn deformation_recertifies_error_and_two_views_select_independently() {
        let source = source();
        let rest = source
            .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
            .unwrap();
        assert!(rest.levels()[1].object_error < 1e-10);
        assert_eq!(
            rest.select_for_camera(camera(2.), [100, 100], POLICY, None),
            Ok(1)
        );
        let joints = [Mat4::IDENTITY, Mat4::from_translation(Vec3::Z)];
        let deformed = source.prepare(&joints, Mat4::IDENTITY).unwrap();
        assert!(deformed.levels()[1].object_error >= 1.);
        assert!(deformed.levels()[1].object_error < 1.000001);
        assert_eq!(
            deformed.select_for_camera(camera(2.), [100, 100], POLICY, Some(1)),
            Ok(0)
        );
        assert_eq!(
            deformed.select_for_camera(camera(200.), [100, 100], POLICY, None),
            Ok(1)
        );
        assert!(std::ptr::eq(rest.source().mesh(), deformed.source().mesh()));
        assert!(std::ptr::eq(
            rest.source().indices(1).unwrap(),
            deformed.source().indices(1).unwrap()
        ));
        let scaled = source
            .prepare(&joints, Mat4::from_scale(Vec3::splat(3.)))
            .unwrap();
        assert!(scaled.levels()[1].object_error >= 3.);
        let baked = deformed.posed_scene_mesh(1, [1.; 4]).unwrap();
        assert_eq!(baked.indices(), &[0, 1, 2]);
        assert_eq!(baked.vertices()[3].position, [0., 0., 1.]);
        assert_eq!(deformed.joints(), &joints);
        assert_eq!(deformed.model(), Mat4::IDENTITY);
        let mut crossing = camera(2.);
        crossing.eye = Vec3::new(0., 0., 0.5);
        crossing.projection = SceneProjection::Perspective {
            vertical_fov: 1.,
            aspect: 1.,
            near: 0.1,
            far: 100.,
        };
        assert_eq!(
            deformed.select_for_camera(crossing, [100, 100], POLICY, None),
            Ok(0)
        );
        assert_eq!(
            deformed.select_for_camera(crossing, [100, 100], POLICY, Some(2)),
            Err(LodError::InvalidPreviousLevel)
        );
    }

    #[test]
    fn invalid_pose_or_witness_never_changes_last_good_snapshot() {
        let source = source();
        let mut joints = [Mat4::IDENTITY; 2];
        let valid = source.prepare(&joints, Mat4::IDENTITY).unwrap();
        joints[1] = Mat4::from_translation(Vec3::splat(100.));
        assert_eq!(valid.joints(), &[Mat4::IDENTITY; 2]);
        assert_eq!(valid.positions()[3], [0.; 3]);
        assert!(matches!(
            source.prepare(&[], Mat4::IDENTITY),
            Err(SkinnedLodError::Skin(
                SkinnedUploadError::JointCountMismatch
            ))
        ));
        let mut projective = Mat4::IDENTITY;
        projective.x_axis.w = 1.;
        assert!(matches!(
            source.prepare(&joints, projective),
            Err(SkinnedLodError::Skin(SkinnedUploadError::NonAffineMatrix))
        ));
        let overflow = [Mat4::from_scale(Vec3::splat(f32::MAX)); 2];
        assert!(
            source
                .prepare(&overflow, Mat4::from_scale(Vec3::splat(2.)))
                .is_err()
        );
        assert!(
            SkinnedLodMesh::new(
                source.0.mesh.clone(),
                vec![CertifiedLodVariant {
                    indices: vec![0, 1, 2],
                    source_to_variant: Vec::new(),
                    variant_to_source: Vec::new(),
                }]
            )
            .is_err()
        );
        assert_eq!(
            valid.select_for_camera(camera(2.), [100, 100], POLICY, None),
            Ok(1)
        );
        assert!(valid.posed_scene_mesh(2, [1.; 4]).is_err());
        assert!(valid.posed_scene_mesh(0, [f32::NAN; 4]).is_err());
    }

    #[test]
    fn skeletal_gpu_lod_budget_shared_streams_and_topology_history() {
        use crate::SkinnedLodGpuError;
        use crate::skinned_lod_gpu::GpuSkinnedLod;
        let source = source();
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let (foreign, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let layout = crate::skinned::create_skin_layout(&device);
        let pose = || {
            source
                .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
                .unwrap()
        };
        assert!(matches!(
            GpuSkinnedLod::upload(&device, &layout, pose(), 0, 0, 0),
            Err(SkinnedLodGpuError::BudgetExceeded { .. })
        ));
        let (mut gpu, mut lod) =
            GpuSkinnedLod::upload(&device, &layout, pose(), 0, 0, 4096).unwrap();
        let pinned = gpu.allocation_bytes();
        assert_eq!(lod.bytes, pinned);
        let vertex = gpu.vertex.clone();
        let palette = gpu.joint_buffer.clone();
        let object = gpu.object_buffer.clone();
        let base = gpu.index.clone();
        assert_eq!(lod.fallback(1).unwrap(), 0);
        assert!(matches!(
            lod.ensure(&foreign, 1, 4096),
            Err(SkinnedLodGpuError::ForeignDevice)
        ));
        assert!(matches!(
            lod.ensure(&device, 1, pinned + 11),
            Err(SkinnedLodGpuError::BudgetExceeded { .. })
        ));
        assert_eq!(lod.bytes, pinned);
        assert_eq!(lod.residency().resident_levels, 1);
        assert!(lod.ensure(&device, 1, pinned + 12).unwrap());
        assert!(!lod.ensure(&device, 1, 0).unwrap());
        assert_eq!(lod.bytes, pinned + 12);
        assert!(std::ptr::eq(
            lod.levels[0].as_ref().unwrap().mesh.vertices(),
            lod.levels[1].as_ref().unwrap().mesh.vertices()
        ));
        let mut motion = Some(crate::skinned_motion::ResidentSkinnedMotion {
            history: crate::SkinnedMotionHistory::new(source.mesh().clone()),
            joints: vec![Mat4::IDENTITY; 2],
            model: Mat4::IDENTITY,
        });
        assert!(motion.as_mut().unwrap().commit_presented());
        assert!(lod.bind(1, &mut gpu, &mut motion).unwrap());
        assert_eq!(gpu.index_count, 3);
        let prepared = motion
            .as_ref()
            .unwrap()
            .history
            .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
            .unwrap();
        assert!(!prepared.history_valid);
        assert_eq!(prepared.vertices.len(), 3);
        assert!(motion.as_mut().unwrap().commit_presented());
        assert!(!lod.bind(1, &mut gpu, &mut motion).unwrap());
        assert!(
            motion
                .as_ref()
                .unwrap()
                .history
                .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
                .unwrap()
                .history_valid
        );
        assert!(lod.evict(1).is_err()); // An active draw must first move to a finer level.
        lod.pose = source
            .prepare(
                &[Mat4::IDENTITY, Mat4::from_translation(Vec3::Z)],
                Mat4::IDENTITY,
            )
            .unwrap();
        let desired = lod
            .pose
            .select_for_camera(camera(2.), [100, 100], POLICY, Some(1))
            .unwrap();
        assert_eq!(desired, 0);
        assert!(lod.bind(desired, &mut gpu, &mut motion).unwrap());
        let prepared = motion
            .as_ref()
            .unwrap()
            .history
            .prepare(lod.pose.joints(), lod.pose.model())
            .unwrap();
        assert!(!prepared.history_valid);
        assert_eq!(prepared.vertices.len(), 6);
        assert!(lod.evict(1).unwrap());
        assert_eq!(lod.bytes, pinned);
        assert_eq!(gpu.index, base);
        assert_eq!(gpu.vertex, vertex);
        assert_eq!(gpu.joint_buffer, palette);
        assert_eq!(gpu.object_buffer, object);
        assert!(lod.evict(0).is_err());
        assert!(lod.ensure(&device, 2, 4096).is_err());
        assert!(
            GpuSkinnedLod::upload(&device, &layout, pose(), 0, pinned, pinned * 2 - 1).is_err()
        );
        assert_eq!(gpu.index, base);
    }
}

impl PreparedSkinnedLod {
    #[must_use]
    pub fn source(&self) -> &SkinnedLodMesh {
        &self.source
    }
    #[must_use]
    pub fn joints(&self) -> &[Mat4] {
        &self.joints
    }
    #[must_use]
    pub fn model(&self) -> Mat4 {
        self.model
    }
    #[must_use]
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }
    #[must_use]
    pub fn levels(&self) -> &[LodLevel] {
        &self.levels
    }

    /// Uses the shared camera/projection and hysteresis policy over the current
    /// deformed world domain. History belongs to the caller's instance and view;
    /// reset it when changing source. Near-plane crossing requires base geometry.
    /// # Errors
    /// Rejects invalid camera, viewport, policy or prior level.
    pub fn select_for_camera(
        &self,
        camera: SceneCamera,
        viewport: [u32; 2],
        policy: LodPolicy,
        previous: Option<usize>,
    ) -> Result<usize, LodError> {
        let bounds = camera.lod_bounds(Mat4::IDENTITY, self.min, self.max)?;
        policy.select_for_camera(&self.levels, camera, bounds, viewport, previous)
    }

    /// Explicit CPU-baked selected geometry with shared vertex indexing. Use an
    /// identity scene transform: positions already include the model transform.
    /// # Errors
    /// Rejects invalid level or color. Does not publish GPU resources or history.
    pub fn posed_scene_mesh(&self, level: usize, color: [f32; 4]) -> Result<SceneMesh, SceneError> {
        let indices = self
            .source
            .indices(level)
            .ok_or(SceneError::InvalidGeometry)?;
        let vertices = self
            .positions
            .iter()
            .zip(self.source.mesh().vertices())
            .map(|(position, vertex)| SceneVertex {
                position: *position,
                uv: vertex.uv,
                color,
            })
            .collect();
        SceneMesh::new(vertices, indices.to_vec())
    }
}
