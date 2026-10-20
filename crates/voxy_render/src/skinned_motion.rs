//! Last-presented skeletal poses for deformation-aware temporal rendering.
use crate::{PreviousPositionVertex, SkinnedMesh, SkinnedUploadError};
use glam::Mat4;

#[derive(Clone, Debug)]
struct Pose {
    joints: Vec<Mat4>,
    model: Mat4,
}
/// One history per mesh instance and independently presented VR eye.
#[derive(Clone, Debug)]
pub struct SkinnedMotionHistory {
    mesh: SkinnedMesh,
    presented: Option<Pose>,
    generation: std::sync::Arc<()>,
}
#[derive(Debug)]
pub struct SkinnedMotionFrame {
    pub vertices: Vec<PreviousPositionVertex>,
    /// False on the first frame or after reset; also reset the temporal consumer.
    pub history_valid: bool,
}
/// Immutable scene geometry, correspondence and the exact candidate pose.
/// Submit/build/render consumers from this snapshot, then commit only on present.
#[derive(Debug)]
pub struct PreparedSkinnedFrame {
    scene: crate::SceneMesh,
    motion: SkinnedMotionFrame,
    pose: Pose,
    generation: std::sync::Arc<()>,
}
impl PreparedSkinnedFrame {
    /// Previous world triangles from the same last-presented pose as raster
    /// motion. Keys use current TLAS instance/custom data/geometry and the first
    /// primitive of this mesh in its BLAS. Keep topology/vertex order unchanged;
    /// remap stable scene identity when current instance indices change.
    /// First frame/reset returns no correspondence. Skipped candidates never
    /// advance the source pose; commit this snapshot only on successful present.
    /// # Errors
    /// Rejects malformed triangle correspondence and primitive-index overflow.
    pub fn previous_reflection_triangles(
        &self,
        instance_geometry: [u32; 3],
        first_primitive: u32,
    ) -> Result<Vec<crate::PreviousReflectionTriangle>, crate::RaySceneError> {
        if !self.motion.history_valid {
            return Ok(Vec::new());
        }
        let vertices = &self.motion.vertices;
        if !vertices.len().is_multiple_of(3)
            || vertices
                .iter()
                .any(|v| v.previous.iter().any(|p| !p.is_finite()))
        {
            return Err(crate::RaySceneError::InvalidGeometry);
        }
        vertices
            .chunks_exact(3)
            .enumerate()
            .map(|(index, triangle)| {
                let primitive = u32::try_from(index)
                    .ok()
                    .and_then(|i| first_primitive.checked_add(i))
                    .ok_or(crate::RaySceneError::Capacity)?;
                Ok(crate::PreviousReflectionTriangle {
                    identity: [
                        instance_geometry[0],
                        instance_geometry[1],
                        instance_geometry[2],
                        primitive,
                    ],
                    vertices: std::array::from_fn(|i| {
                        let p = triangle[i].previous;
                        [p[0], p[1], p[2], 1.0]
                    }),
                })
            })
            .collect()
    }

    #[must_use]
    pub const fn scene(&self) -> &crate::SceneMesh {
        &self.scene
    }
    #[must_use]
    pub const fn motion(&self) -> &SkinnedMotionFrame {
        &self.motion
    }
}

impl SkinnedMotionHistory {
    #[doc(hidden)]
    pub fn mesh(&self) -> &SkinnedMesh {
        &self.mesh
    }

    #[must_use]
    pub fn new(mesh: SkinnedMesh) -> Self {
        Self {
            mesh,
            presented: None,
            generation: std::sync::Arc::new(()),
        }
    }
    /// Prepare correspondence without advancing the last presented pose.
    /// # Errors
    /// Rejects invalid palettes/models or overflowing skinned positions.
    pub fn prepare(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<SkinnedMotionFrame, SkinnedUploadError> {
        let previous = self.presented.as_ref();
        let vertices = self.mesh.previous_position_vertices(
            [
                joints,
                previous.map_or(joints, |pose| pose.joints.as_slice()),
            ],
            [model, previous.map_or(model, |pose| pose.model)],
        )?;
        Ok(SkinnedMotionFrame {
            vertices,
            history_valid: previous.is_some(),
        })
    }
    /// Prepare matching raster/ray geometry and motion without advancing history.
    /// Use identity model with the world-space scene mesh. Rebuild ray geometry
    /// before querying, and reset the temporal consumer when history is invalid.
    /// # Errors
    /// Rejects invalid poses, overflowing positions or invalid vertex color.
    pub fn prepare_frame(
        &self,
        joints: &[Mat4],
        model: Mat4,
        color: [f32; 4],
    ) -> Result<PreparedSkinnedFrame, Box<dyn std::error::Error>> {
        let scene = self.mesh.posed_scene_mesh(joints, model, color)?;
        let motion = self.prepare(joints, model)?;
        Ok(PreparedSkinnedFrame {
            scene,
            motion,
            pose: Pose {
                joints: joints.to_vec(),
                model,
            },
            generation: std::sync::Arc::clone(&self.generation),
        })
    }
    /// Commit the captured candidate pose only after successful presentation.
    /// Failed/skipped submissions leave this history unchanged.
    /// # Errors
    /// Rejects foreign snapshots and candidates made before reset/another commit.
    pub fn presented_frame(
        &mut self,
        frame: &PreparedSkinnedFrame,
    ) -> Result<(), SkinnedUploadError> {
        if !std::sync::Arc::ptr_eq(&self.generation, &frame.generation) {
            return Err(SkinnedUploadError::StaleHistory);
        }
        self.presented(&frame.pose.joints, frame.pose.model)
    }
    /// Commit exactly the pose used by a successfully presented frame.
    /// Call only after presentation succeeds, never after preparation/submission alone.
    /// Validation failure preserves the previous pose. Skipped frames need no call.
    /// # Errors
    /// Rejects invalid palettes/models or overflowing skinned positions.
    pub fn presented(&mut self, joints: &[Mat4], model: Mat4) -> Result<(), SkinnedUploadError> {
        // Only immutable geometry and the exact previously admitted pose can reuse
        // admission. Keep small meshes on the original path, including changed poses.
        let unchanged = self.mesh.vertices().len() >= 1024
            && self.presented.as_ref().is_some_and(|pose| {
                bytemuck::bytes_of(&pose.model) == bytemuck::bytes_of(&model)
                    && bytemuck::cast_slice::<Mat4, u8>(&pose.joints)
                        == bytemuck::cast_slice::<Mat4, u8>(joints)
            });
        if unchanged {
            self.generation = std::sync::Arc::new(());
            return Ok(());
        }
        self.mesh.validate_temporal_pose(joints, model)?;
        if let Some(pose) = &mut self.presented {
            pose.joints.clone_from_slice(joints);
            pose.model = model;
        } else {
            self.presented = Some(Pose {
                joints: joints.to_vec(),
                model,
            });
        }
        self.generation = std::sync::Arc::new(());
        Ok(())
    }
    #[cfg(test)]
    fn presented_reference(
        &mut self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<(), SkinnedUploadError> {
        self.mesh.validate_temporal_pose(joints, model)?;
        if let Some(pose) = &mut self.presented {
            pose.joints.clone_from_slice(joints);
            pose.model = model;
        } else {
            self.presented = Some(Pose {
                joints: joints.to_vec(),
                model,
            });
        }
        self.generation = std::sync::Arc::new(());
        Ok(())
    }
    /// Reset on camera cuts, resource recreation, topology changes or teleports.
    pub fn reset(&mut self) {
        self.presented = None;
        self.generation = std::sync::Arc::new(());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkinnedVertex;
    #[test]
    fn prepared_frames_commit_captured_pose_and_reject_stale_or_foreign_candidates() {
        let mesh = SkinnedMesh::new(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
                .map(|position| SkinnedVertex {
                    position,
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0; 2],
                    joints: [0; 4],
                    weights: [65535, 0, 0, 0],
                })
                .to_vec(),
            vec![0, 1, 2],
            1,
        )
        .unwrap();
        let mut history = SkinnedMotionHistory::new(mesh.clone());
        let mut joints = [Mat4::IDENTITY];
        let first = history
            .prepare_frame(&joints, Mat4::IDENTITY, [1.0; 4])
            .unwrap();
        assert!(!first.motion().history_valid);
        assert!(
            first
                .previous_reflection_triangles([4, 7, 2], 9)
                .unwrap()
                .is_empty()
        );
        joints[0] = Mat4::from_translation(glam::Vec3::X);
        history.presented_frame(&first).unwrap();
        assert_eq!(
            history.presented_frame(&first),
            Err(SkinnedUploadError::StaleHistory)
        );
        let skipped = history
            .prepare_frame(&joints, Mat4::IDENTITY, [1.0; 4])
            .unwrap();
        joints[0] = Mat4::from_translation(glam::Vec3::Y);
        let mut next = history
            .prepare_frame(&joints, Mat4::IDENTITY, [1.0; 4])
            .unwrap();
        assert!(next.motion().history_valid);
        for (&index, pair) in next.scene().indices().iter().zip(&next.motion().vertices) {
            assert_eq!(
                next.scene().vertices()[index as usize].position,
                pair.current
            );
        }
        assert!(
            glam::Vec3::from_array(next.motion().vertices[0].previous)
                .abs_diff_eq(glam::Vec3::ZERO, 1e-6)
        );
        let reflected = next.previous_reflection_triangles([4, 7, 2], 9).unwrap();
        assert_eq!(reflected.len(), 1);
        assert_eq!(reflected[0].identity, [4, 7, 2, 9]);
        assert_eq!(
            reflected[0].vertices,
            [
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0, 1.0]
            ]
        );
        let extra = next.motion.vertices.clone();
        next.motion.vertices.extend(extra);
        assert!(matches!(
            next.previous_reflection_triangles([0; 3], u32::MAX),
            Err(crate::RaySceneError::Capacity)
        ));
        next.motion.vertices.truncate(3);
        let saved = next.motion.vertices[0].previous;
        next.motion.vertices[0].previous[0] = f32::NAN;
        assert!(matches!(
            next.previous_reflection_triangles([0; 3], 0),
            Err(crate::RaySceneError::InvalidGeometry)
        ));
        next.motion.vertices[0].previous = saved;
        // The discarded X-translation candidate must not replace the previous pose.
        history.presented_frame(&next).unwrap();
        assert_eq!(
            history.presented_frame(&skipped),
            Err(SkinnedUploadError::StaleHistory)
        );
        let candidate = history
            .prepare_frame(&joints, Mat4::IDENTITY, [1.0; 4])
            .unwrap();
        let mut foreign = SkinnedMotionHistory::new(mesh);
        assert_eq!(
            foreign.presented_frame(&candidate),
            Err(SkinnedUploadError::StaleHistory)
        );
        history.reset();
        assert_eq!(
            history.presented_frame(&candidate),
            Err(SkinnedUploadError::StaleHistory)
        );
        assert!(
            !history
                .prepare_frame(&joints, Mat4::IDENTITY, [1.0; 4])
                .unwrap()
                .motion()
                .history_valid
        );
    }
    #[test]
    fn skipped_and_invalid_frames_preserve_pose_and_reset_clears_it() {
        let mesh = SkinnedMesh::new(
            vec![SkinnedVertex {
                position: [0.0; 3],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0; 2],
                joints: [0; 4],
                weights: [65535, 0, 0, 0],
            }],
            vec![0; 3],
            1,
        )
        .unwrap();
        let mut history = SkinnedMotionHistory::new(mesh);
        let first = [Mat4::IDENTITY];
        let skipped = [Mat4::from_translation(glam::Vec3::X)];
        let next = [Mat4::from_translation(glam::Vec3::Y)];
        assert!(
            !history
                .prepare(&first, Mat4::IDENTITY)
                .unwrap()
                .history_valid
        );
        history.presented(&first, Mat4::IDENTITY).unwrap();
        history.prepare(&skipped, Mat4::IDENTITY).unwrap();
        assert!(history.presented(&[], Mat4::IDENTITY).is_err());
        let frame = history.prepare(&next, Mat4::IDENTITY).unwrap();
        assert!(frame.history_valid);
        for vertex in frame.vertices {
            assert!(glam::Vec3::from_array(vertex.previous).abs_diff_eq(glam::Vec3::ZERO, 1e-6));
            assert!(glam::Vec3::from_array(vertex.current).abs_diff_eq(glam::Vec3::Y, 1e-6));
        }
        history.presented(&next, Mat4::IDENTITY).unwrap();
        history.reset();
        let frame = history.prepare(&skipped, Mat4::IDENTITY).unwrap();
        assert!(!frame.history_valid);
        for vertex in frame.vertices {
            assert!(
                glam::Vec3::from_array(vertex.current)
                    .abs_diff_eq(glam::Vec3::from_array(vertex.previous), 1e-6)
            );
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResidentSkinnedMotion {
    pub history: SkinnedMotionHistory,
    pub joints: Vec<Mat4>,
    pub model: Mat4,
}
impl ResidentSkinnedMotion {
    pub fn commit_presented(&mut self) -> bool {
        if self.history.presented(&self.joints, self.model).is_err() {
            self.history.reset();
            return false;
        }
        true
    }
}

#[cfg(test)]
mod resident_tests {
    use super::*;
    #[test]
    fn commit_and_invalid_pose_reset_resident_history() {
        let mesh = SkinnedMesh::new(
            vec![crate::SkinnedVertex {
                position: [0.0; 3],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0; 2],
                joints: [0; 4],
                weights: [65535, 0, 0, 0],
            }],
            vec![0; 3],
            1,
        )
        .unwrap();
        let mut resident = ResidentSkinnedMotion {
            history: SkinnedMotionHistory::new(mesh),
            joints: vec![Mat4::IDENTITY],
            model: Mat4::IDENTITY,
        };
        resident.commit_presented();
        resident.joints[0] = Mat4::from_translation(glam::Vec3::X);
        let prepared = resident
            .history
            .prepare(&resident.joints, resident.model)
            .unwrap();
        assert!(
            glam::Vec3::from_array(prepared.vertices[0].previous)
                .abs_diff_eq(glam::Vec3::ZERO, 1e-6)
        );
        resident.commit_presented();
        let prepared = resident
            .history
            .prepare(&[Mat4::IDENTITY], resident.model)
            .unwrap();
        assert!(
            glam::Vec3::from_array(prepared.vertices[0].previous).abs_diff_eq(glam::Vec3::X, 1e-6)
        );
        resident.model = Mat4::from_cols_array(&[f32::NAN; 16]);
        resident.commit_presented();
        assert!(
            !resident
                .history
                .prepare(&[Mat4::IDENTITY], Mat4::IDENTITY)
                .unwrap()
                .history_valid
        );
    }
}

#[cfg(test)]
mod admission_cache_tests {
    use super::*;
    use crate::SkinnedVertex;
    fn mesh(count: usize) -> SkinnedMesh {
        SkinnedMesh::new(
            (0..count)
                .map(|i| SkinnedVertex {
                    position: [(i % 71) as f32 * 0.03, 0.2, 0.3],
                    normal: [0., 1., 0.],
                    uv: [0.; 2],
                    joints: [0; 4],
                    weights: [65535, 0, 0, 0],
                })
                .collect(),
            vec![0, 1, 2],
            1,
        )
        .unwrap()
    }
    #[test]
    fn reused_pose_keeps_generation_and_rejection_semantics() {
        let mut cached = SkinnedMotionHistory::new(mesh(2048));
        let mut reference = cached.clone();
        let joints = [Mat4::IDENTITY];
        for model in [
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            Mat4::from_translation(glam::Vec3::X),
            Mat4::from_translation(glam::Vec3::X),
        ] {
            assert_eq!(
                cached.presented(&joints, model),
                reference.presented_reference(&joints, model)
            );
            assert_eq!(
                cached.presented.as_ref().unwrap().joints,
                reference.presented.as_ref().unwrap().joints
            );
            assert_eq!(
                cached.presented.as_ref().unwrap().model,
                reference.presented.as_ref().unwrap().model
            );
        }
        let snapshot = cached
            .prepare_frame(&joints, Mat4::IDENTITY, [1.; 4])
            .unwrap();
        let generation = cached.generation.clone();
        cached.presented(&joints, Mat4::IDENTITY).unwrap();
        cached.presented(&joints, Mat4::IDENTITY).unwrap();
        assert!(!Arc::ptr_eq(&generation, &cached.generation));
        assert_eq!(
            cached.presented_frame(&snapshot),
            Err(SkinnedUploadError::StaleHistory)
        );
        let admitted = cached.presented.clone().unwrap();
        for (palette, model) in [
            (
                &[Mat4::from_cols_array(&[f32::NAN; 16])][..],
                Mat4::IDENTITY,
            ),
            (&[][..], Mat4::IDENTITY),
            (&joints[..], Mat4::from_scale(glam::Vec3::splat(f32::MAX))),
        ] {
            let mut reference = cached.clone();
            let result = cached.presented(palette, model);
            assert!(result.is_err());
            assert_eq!(result, reference.presented_reference(palette, model));
            assert_eq!(cached.presented.as_ref().unwrap().joints, admitted.joints);
            assert_eq!(cached.presented.as_ref().unwrap().model, admitted.model);
        }
        cached.reset();
        assert!(cached.presented.is_none());
        cached.presented(&joints, Mat4::IDENTITY).unwrap();
    }
    #[test]
    #[ignore = "manual CPU paired history benchmark"]
    fn paired_history_cpu_bench() {
        use std::{hint::black_box, time::Instant};
        for (name, count, repeated) in [
            ("repeat_large", 100_000, true),
            ("change_large", 100_000, false),
            ("change_small", 3, false),
        ] {
            let base = SkinnedMotionHistory::new(mesh(count));
            let joints = [Mat4::IDENTITY];
            let batch = if count < 1024 { 256 } else { 1 };
            for round in 1..=5 {
                for (variant, reference) in if round % 2 == 1 {
                    [("before", true), ("after", false)]
                } else {
                    [("after", false), ("before", true)]
                } {
                    let mut history = base.clone();
                    let mut samples = Vec::new();
                    for sample in 0..34 {
                        let start = Instant::now();
                        for i in 0..batch {
                            let model = if repeated {
                                Mat4::IDENTITY
                            } else {
                                Mat4::from_translation(
                                    glam::Vec3::X * ((sample * batch + i) % 2) as f32,
                                )
                            };
                            if reference {
                                black_box(
                                    history
                                        .presented_reference(black_box(&joints), black_box(model)),
                                )
                                .unwrap();
                            } else {
                                black_box(history.presented(black_box(&joints), black_box(model)))
                                    .unwrap();
                            }
                        }
                        if sample >= 3 {
                            samples.push(start.elapsed().as_secs_f64() * 1e6 / batch as f64);
                        }
                    }
                    samples.sort_by(f64::total_cmp);
                    println!("HISTORY_PAIR,{round},{variant},{name},{:.6}", samples[15]);
                }
            }
        }
    }
    use std::sync::Arc;
}
