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
    /// Blend immutable local poses on their original rig using a double-precision
    /// weight and shortest-path quaternion interpolation. Finite weights clamp
    /// to [0,1]; endpoint poses retain their exact authored bits.
    /// # Errors
    /// Different rig/count, nonfinite weight, invalid input/result TRS.
    pub fn blend(a: &Self, b: &Self, weight: f64) -> Result<Self, AnimationError> {
        if a.local.len() != b.local.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&a.rig, &b.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !weight.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        for (i, (a, b)) in a.local.iter().zip(&b.local).enumerate() {
            if !a.valid() || !b.valid() {
                return Err(AnimationError::InvalidPose(i));
            }
        }
        let weight = weight.clamp(0., 1.);
        if weight == 0. {
            return Ok(a.clone());
        }
        if weight == 1. {
            return Ok(b.clone());
        }
        let local = a
            .local
            .iter()
            .zip(&b.local)
            .enumerate()
            .map(|(i, (a, b))| {
                let result = Transform64 {
                    translation: a.translation * (1. - weight) + b.translation * weight,
                    rotation: a.rotation.slerp(b.rotation, weight).normalize(),
                    scale: a.scale * (1. - weight) + b.scale * weight,
                };
                if result.valid() {
                    Ok(result)
                } else {
                    Err(AnimationError::InvalidPose(i))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            rig: a.rig.clone(),
            local,
        })
    }
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
            if !usable_linear(matrix) {
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
                if usable_linear(matrix) {
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
    /// Start on the bind pose and smoothly join the authored clip on the same
    /// phase clock. A cubic smoothstep fades the bind contribution to zero;
    /// after startup the ordinary sample is returned exactly.
    /// # Errors
    /// Invalid startup duration, phase, rig or sampled/blended transforms.
    pub fn try_sample_phase64_with_startup(
        &self,
        skeleton: &Skeleton,
        phase: f64,
        startup_seconds: f64,
    ) -> Result<Pose64, AnimationError> {
        if !startup_seconds.is_finite()
            || startup_seconds < 0.
            || startup_seconds > f64::from(self.duration)
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let target = self.try_sample_phase64(skeleton, phase)?;
        let elapsed = phase * f64::from(self.duration);
        if startup_seconds == 0. || elapsed >= startup_seconds {
            return Ok(target);
        }
        let u = elapsed / startup_seconds;
        Pose64::blend(&skeleton.bind_pose64(), &target, u * u * (3. - 2. * u))
    }
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
    fn wide_blend_retains_clock_endpoints_and_rejects_singular_or_foreign_poses() {
        let rig = rig();
        let a = rig.bind_pose64();
        let mut b = a.clone();
        b.local[0].translation = DVec3::new(3., 0., 0.);
        b.local[0].rotation = DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let weight = 0.5 + 1e-9;
        let blend = Pose64::blend(&a, &b, weight).unwrap();
        assert!((blend.local[0].translation.x - (1. + 2. * weight)).abs() < 1e-15);
        assert_ne!(blend.local[0].translation.x, 2.);
        let point = blend.global_matrices(&rig).unwrap()[1].transform_point3(DVec3::ZERO);
        let angle = std::f64::consts::FRAC_PI_2 * weight;
        assert!((point.x - (1. + 2. * weight + angle.cos())).abs() < 2e-15);
        assert!((point.y - angle.sin()).abs() < 2e-15);
        assert_eq!(Pose64::blend(&a, &b, 0.).unwrap(), a);
        assert_eq!(Pose64::blend(&a, &b, 1.).unwrap(), b);
        assert_eq!(Pose64::blend(&a, &b, -10.).unwrap(), a);
        assert_eq!(Pose64::blend(&a, &b, 10.).unwrap(), b);
        let mut antipodal = b.clone();
        antipodal.local[0].rotation = -b.local[0].rotation;
        let q = Pose64::blend(&b, &antipodal, 0.5).unwrap().local[0].rotation;
        assert!(q.dot(b.local[0].rotation).abs() > 1. - 1e-14);
        let mut foreign = rig.joints().to_vec();
        foreign[0].name = "foreign".into();
        let foreign = Skeleton::new(foreign).unwrap().bind_pose64();
        assert_eq!(
            Pose64::blend(&a, &foreign, 0.).unwrap_err(),
            AnimationError::SkeletonMismatch
        );
        for weight in [f64::NAN, f64::INFINITY] {
            assert_eq!(
                Pose64::blend(&a, &b, weight).unwrap_err(),
                AnimationError::InvalidBlendWeight
            );
        }
        let mut invalid = b.clone();
        invalid.local[0].translation.x = f64::NAN;
        assert_eq!(
            Pose64::blend(&a, &invalid, 0.).unwrap_err(),
            AnimationError::InvalidPose(0)
        );
        let mut reflected = b.clone();
        reflected.local[0].scale = -DVec3::ONE;
        assert_eq!(
            Pose64::blend(&a, &reflected, 0.5).unwrap_err(),
            AnimationError::InvalidPose(0)
        );
    }
    #[test]
    fn startup_joins_the_same_authored_clock_with_exact_endpoints() {
        let rig = rig();
        let clip = AnimationClip::new(
            "startup",
            1.,
            Playback::Clamp,
            vec![
                JointTrack {
                    translations: vec![
                        Vec3Key {
                            time: 0.,
                            value: Vec3::X * 3.,
                        },
                        Vec3Key {
                            time: 1.,
                            value: Vec3::X * 4.,
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
            clip.try_sample_phase64_with_startup(&rig, 0., 0.5).unwrap(),
            rig.bind_pose64()
        );
        for phase in [0., 0.25, 0.5, 1.] {
            assert_eq!(
                clip.try_sample_phase64_with_startup(&rig, phase, 0.)
                    .unwrap(),
                clip.try_sample_phase64(&rig, phase).unwrap()
            );
        }
        for phase in [0.5, 0.75, 1.] {
            assert_eq!(
                clip.try_sample_phase64_with_startup(&rig, phase, 0.5)
                    .unwrap(),
                clip.try_sample_phase64(&rig, phase).unwrap()
            );
        }
        let mid = clip
            .try_sample_phase64_with_startup(&rig, 0.25, 0.5)
            .unwrap();
        assert!((mid.local[0].translation.x - 2.125).abs() < 1e-15);
        let h = 1e-6;
        let first = clip.try_sample_phase64_with_startup(&rig, h, 0.5).unwrap();
        assert!((first.local[0].translation.x - 1.) / h < 1e-4);
        let below = clip
            .try_sample_phase64_with_startup(&rig, 0.5 - h, 0.5)
            .unwrap();
        let authored = clip.try_sample_phase64(&rig, 0.5 - h).unwrap();
        assert!((below.local[0].translation.x - authored.local[0].translation.x).abs() / h < 1e-4);
        for duration in [f64::NAN, -1., 1.1, f64::INFINITY] {
            assert!(
                clip.try_sample_phase64_with_startup(&rig, 0., duration)
                    .is_err()
            );
        }
        assert!(
            clip.try_sample_phase64_with_startup(&rig, f64::NAN, 0.5)
                .is_err()
        );
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
                    bind_local: Transform::IDENTITY,
                    inverse_bind: Mat4::IDENTITY,
                })
                .collect(),
        )
        .unwrap();
        let mut overflowing_pose = huge.bind_pose64();
        for local in &mut overflowing_pose.local {
            local.scale = DVec3::splat(3e38);
        }
        assert!(matches!(
            overflowing_pose.skin_matrices(&huge),
            Err(AnimationError::InvalidPose(_))
        ));
    }
}
