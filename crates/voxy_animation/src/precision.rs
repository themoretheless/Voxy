//! Double-precision kinematics over the same immutable rig and authored keys.
use super::*;
use glam::{DMat4, DQuat, DVec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform64 {
    pub translation: DVec3,
    pub rotation: DQuat,
    pub scale: DVec3,
}
impl Transform64 {
    pub fn matrix(self) -> DMat4 {
        DMat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
    fn valid(self) -> bool {
        self.translation.is_finite()
            && self.rotation.is_finite()
            && self.rotation.is_normalized()
            && self.scale.is_finite()
            && self.scale.abs().min_element() > 0.
    }
    fn from_authored(t: Transform) -> Self {
        Self {
            translation: t.translation.as_dvec3(),
            rotation: quaternion(t.rotation),
            scale: t.scale.as_dvec3(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Pose64 {
    rig: Arc<[Joint]>,
    local: Vec<Transform64>,
}
impl Pose64 {
    pub fn local(&self) -> &[Transform64] {
        &self.local
    }
    /// Parent-before-child global transforms, retaining the original rig binding.
    pub fn global_matrices(&self, skeleton: &Skeleton) -> Result<Vec<DMat4>, AnimationError> {
        if self.local.len() != skeleton.joints.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&self.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let mut global: Vec<DMat4> = Vec::with_capacity(self.local.len());
        for (i, (local, joint)) in self.local.iter().zip(skeleton.joints.iter()).enumerate() {
            if !local.valid() {
                return Err(AnimationError::InvalidPose(i));
            }
            let matrix = match joint.parent {
                Some(parent) => global[usize::from(parent)] * local.matrix(),
                None => local.matrix(),
            };
            if !matrix.is_finite() {
                return Err(AnimationError::InvalidPose(i));
            }
            global.push(matrix);
        }
        Ok(global)
    }
    pub fn skin_matrices(&self, skeleton: &Skeleton) -> Result<Vec<DMat4>, AnimationError> {
        self.global_matrices(skeleton)?
            .into_iter()
            .zip(skeleton.joints.iter())
            .enumerate()
            .map(|(i, (global, joint))| {
                let matrix = global
                    * DMat4::from_cols_array(&joint.inverse_bind.to_cols_array().map(f64::from));
                if matrix.is_finite() {
                    Ok(matrix)
                } else {
                    Err(AnimationError::InvalidPose(i))
                }
            })
            .collect()
    }
}
impl Skeleton {
    pub fn bind_pose64(&self) -> Pose64 {
        Pose64 {
            rig: self.joints.clone(),
            local: self
                .joints
                .iter()
                .map(|j| Transform64::from_authored(j.bind_local))
                .collect(),
        }
    }
}
impl AnimationClip {
    /// Stateless authored phase in f64, including the final pose of loop clips.
    /// This samples existing keys; it does not retarget or emit events.
    pub fn try_sample_phase64(
        &self,
        skeleton: &Skeleton,
        phase: f64,
    ) -> Result<Pose64, AnimationError> {
        if !phase.is_finite() || !(0. ..=1.).contains(&phase) {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.sample_local64(skeleton, phase * f64::from(self.duration))
    }
    pub fn try_sample_time64(
        &self,
        skeleton: &Skeleton,
        time: f64,
    ) -> Result<Pose64, AnimationError> {
        if !time.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.sample_local64(skeleton, self.phase(time))
    }
    fn sample_local64(&self, skeleton: &Skeleton, time: f64) -> Result<Pose64, AnimationError> {
        if self.tracks.len() != skeleton.joints.len() {
            return Err(AnimationError::TrackCountMismatch {
                expected: skeleton.joints.len(),
                actual: self.tracks.len(),
            });
        }
        if !self.is_compatible_with(skeleton) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let local: Vec<_> = self
            .rig
            .iter()
            .zip(self.tracks.iter())
            .zip(self.interpolation.iter())
            .zip(self.tangents.iter())
            .map(|(((joint, track), mode), tangent)| Transform64 {
                translation: sample_vector(
                    &track.translations,
                    time,
                    joint.bind_local.translation,
                    mode.translation,
                    &tangent.translation,
                ),
                rotation: sample_rotation(
                    &track.rotations,
                    time,
                    joint.bind_local.rotation,
                    mode.rotation,
                    &tangent.rotation,
                ),
                scale: sample_vector(
                    &track.scales,
                    time,
                    joint.bind_local.scale,
                    mode.scale,
                    &tangent.scale,
                ),
            })
            .collect();
        for (i, value) in local.iter().enumerate() {
            if !value.valid() {
                return Err(AnimationError::InvalidPose(i));
            }
        }
        Ok(Pose64 {
            rig: self.rig.clone(),
            local,
        })
    }
}
fn quaternion(value: Quat) -> DQuat {
    DQuat::from_array(value.to_array().map(f64::from)).normalize()
}
fn segment<T: Timed>(keys: &[T], time: f64) -> Option<(usize, &T, &T, f64)> {
    let upper = keys.partition_point(|k| f64::from(k.time()) <= time);
    if upper == 0 || upper == keys.len() {
        return None;
    }
    let a = &keys[upper - 1];
    let b = &keys[upper];
    Some((
        upper - 1,
        a,
        b,
        (time - f64::from(a.time())) / (f64::from(b.time()) - f64::from(a.time())),
    ))
}
fn cubic<const N: usize>(
    a: [f32; N],
    b: [f32; N],
    outgoing: [f32; N],
    incoming: [f32; N],
    t: f64,
    duration: f64,
) -> [f64; N] {
    let t2 = t * t;
    let t3 = t2 * t;
    std::array::from_fn(|i| {
        f64::from(a[i]) * (2. * t3 - 3. * t2 + 1.)
            + f64::from(outgoing[i]) * duration * (t3 - 2. * t2 + t)
            + f64::from(b[i]) * (-2. * t3 + 3. * t2)
            + f64::from(incoming[i]) * duration * (t3 - t2)
    })
}
pub(super) fn linear_vector(from: Vec3, to: Vec3, weight: f64) -> DVec3 {
    if weight == 0. {
        return from.as_dvec3();
    }
    if weight == 1. {
        return to.as_dvec3();
    }
    let a = from.as_dvec3();
    let b = to.as_dvec3();
    a + (b - a) * weight
}
pub(super) fn sample_vector(
    keys: &[Vec3Key],
    time: f64,
    fallback: Vec3,
    mode: Interpolation,
    tangents: &[[Vec3; 2]],
) -> DVec3 {
    let Some(first) = keys.first() else {
        return fallback.as_dvec3();
    };
    if time <= f64::from(first.time) {
        return first.value.as_dvec3();
    }
    let last = keys.last().unwrap();
    if time >= f64::from(last.time) {
        return last.value.as_dvec3();
    }
    let Some((i, a, b, t)) = segment(keys, time) else {
        return first.value.as_dvec3();
    };
    match mode {
        Interpolation::Step => a.value.as_dvec3(),
        Interpolation::Linear => linear_vector(a.value, b.value, t),
        Interpolation::CubicSpline => DVec3::from_array(cubic(
            a.value.to_array(),
            b.value.to_array(),
            tangents[i][1].to_array(),
            tangents[i + 1][0].to_array(),
            t,
            f64::from(b.time) - f64::from(a.time),
        )),
    }
}
pub(super) fn sample_rotation(
    keys: &[QuatKey],
    time: f64,
    fallback: Quat,
    mode: Interpolation,
    tangents: &[[Vec4; 2]],
) -> DQuat {
    let Some(first) = keys.first() else {
        return quaternion(fallback);
    };
    if time <= f64::from(first.time) {
        return quaternion(first.value);
    }
    let last = keys.last().unwrap();
    if time >= f64::from(last.time) {
        return quaternion(last.value);
    }
    let Some((i, a, b, t)) = segment(keys, time) else {
        return quaternion(first.value);
    };
    match mode {
        Interpolation::Step => quaternion(a.value),
        Interpolation::Linear => quaternion(a.value)
            .slerp(quaternion(b.value), t)
            .normalize(),
        Interpolation::CubicSpline => {
            let raw = cubic(
                a.value.to_array(),
                b.value.to_array(),
                tangents[i][1].to_array(),
                tangents[i + 1][0].to_array(),
                t,
                f64::from(b.time) - f64::from(a.time),
            );
            let scale = raw.iter().map(|v| v.abs()).fold(0., f64::max);
            if !scale.is_finite() || scale == 0. || raw.iter().any(|v| !v.is_finite()) {
                DQuat::from_array([f64::NAN; 4])
            } else {
                DQuat::from_array(raw.map(|v| v / scale)).normalize()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rig() -> Skeleton {
        Skeleton::new(vec![
            Joint {
                name: "root".into(),
                parent: None,
                bind_local: Transform {
                    translation: Vec3::X,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                name: "child".into(),
                parent: Some(0),
                bind_local: Transform {
                    translation: Vec3::X,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            },
        ])
        .unwrap()
    }
    #[test]
    fn sub_f32_clock_steps_survive_translation_rotation_and_hierarchy() {
        let rig = rig();
        let clip = AnimationClip::new(
            "motion",
            1.,
            Playback::Loop,
            vec![
                JointTrack {
                    translations: vec![
                        Vec3Key {
                            time: 0.,
                            value: Vec3::X,
                        },
                        Vec3Key {
                            time: 1.,
                            value: Vec3::X * 2.,
                        },
                    ],
                    rotations: vec![
                        QuatKey {
                            time: 0.,
                            value: Quat::IDENTITY,
                        },
                        QuatKey {
                            time: 1.,
                            value: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                        },
                    ],
                    scales: vec![],
                },
                JointTrack::default(),
            ],
            &rig,
        )
        .unwrap();
        let t = 0.275;
        let dt = 1e-9;
        assert_eq!(
            clip.try_sample_phase(&rig, t).unwrap(),
            clip.try_sample_phase(&rig, t + dt).unwrap()
        );
        let a = clip.try_sample_phase64(&rig, t).unwrap();
        let b = clip.try_sample_phase64(&rig, t + dt).unwrap();
        assert!((b.local()[0].translation.x - a.local()[0].translation.x - dt).abs() < 1e-15);
        assert_ne!(a.local()[0].rotation, b.local()[0].rotation);
        for phase in [t, t + dt, 1.] {
            let pose = clip.try_sample_phase64(&rig, phase).unwrap();
            let global = pose.global_matrices(&rig).unwrap();
            let angle = phase * std::f64::consts::FRAC_PI_2;
            let expected = DVec3::new(1. + phase + angle.cos(), angle.sin(), 0.);
            assert!((global[1].transform_point3(DVec3::ZERO) - expected).length() < 1e-14);
            assert!(
                pose.skin_matrices(&rig)
                    .unwrap()
                    .iter()
                    .all(|m| m.is_finite())
            );
        }
    }
    #[test]
    fn wide_time_uses_seconds_and_preserves_loop_seam() {
        let rig = rig();
        for playback in [Playback::Clamp, Playback::Loop] {
            let clip = AnimationClip::new(
                "clock",
                2.,
                playback,
                vec![
                    JointTrack {
                        translations: vec![
                            Vec3Key {
                                time: 0.,
                                value: Vec3::ZERO,
                            },
                            Vec3Key {
                                time: 2.,
                                value: Vec3::X,
                            },
                        ],
                        ..JointTrack::default()
                    },
                    JointTrack::default(),
                ],
                &rig,
            )
            .unwrap();
            assert_eq!(
                clip.try_sample_time64(&rig, 0.5).unwrap(),
                clip.try_sample_phase64(&rig, 0.25).unwrap()
            );
            let expected = if playback == Playback::Loop { 0.25 } else { 1. };
            assert_eq!(
                clip.try_sample_time64(&rig, 2.5).unwrap().local()[0]
                    .translation
                    .x,
                expected
            );
            assert!(clip.try_sample_time64(&rig, f64::INFINITY).is_err());
            if playback == Playback::Loop {
                assert!(
                    clip.try_sample_time64(&rig, 2_f64.next_down())
                        .unwrap()
                        .local()[0]
                        .translation
                        .x
                        > 0.999999999999
                );
                assert_eq!(
                    clip.try_sample_time64(&rig, 2.).unwrap().local()[0].translation,
                    DVec3::ZERO
                );
            }
        }
    }
    #[test]
    fn cubic_vector_keeps_analytic_fraction_and_step_remains_authored() {
        let keys = [
            Vec3Key {
                time: 0.,
                value: Vec3::ZERO,
            },
            Vec3Key {
                time: 1.,
                value: Vec3::X,
            },
        ];
        let tangents = [[Vec3::ZERO; 2]; 2];
        let t = 0.275 + 1e-9;
        let p = sample_vector(&keys, t, Vec3::ZERO, Interpolation::CubicSpline, &tangents);
        assert!((p.x - (3. * t * t - 2. * t * t * t)).abs() < 1e-15);
        assert_eq!(
            sample_vector(&keys, t, Vec3::ZERO, Interpolation::Step, &[]),
            DVec3::ZERO
        );
    }
    #[test]
    fn wide_pose_rejects_foreign_rig_invalid_phase_and_global_overflow() {
        let rig = rig();
        let mut foreign = rig.joints().to_vec();
        foreign[0].name = "other".into();
        let foreign = Skeleton::new(foreign).unwrap();
        assert_eq!(
            rig.bind_pose64().skin_matrices(&foreign).unwrap_err(),
            AnimationError::SkeletonMismatch
        );
        let clip = AnimationClip::new(
            "motion",
            1.,
            Playback::Clamp,
            vec![JointTrack::default(); 2],
            &rig,
        )
        .unwrap();
        for p in [f64::NAN, -0.1, 1.1] {
            assert_eq!(
                clip.try_sample_phase64(&rig, p).unwrap_err(),
                AnimationError::InvalidSampleTime
            );
        }
        let huge = Skeleton::new(
            (0..12)
                .map(|i| Joint {
                    name: format!("j{i}").into(),
                    parent: if i == 0 { None } else { Some(i - 1) },
                    bind_local: Transform {
                        scale: Vec3::splat(3e38),
                        ..Transform::IDENTITY
                    },
                    inverse_bind: Mat4::IDENTITY,
                })
                .collect(),
        )
        .unwrap();
        assert!(matches!(
            huge.bind_pose64().skin_matrices(&huge),
            Err(AnimationError::InvalidPose(_))
        ));
    }
}
