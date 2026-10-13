//! Explicit asset preparation, never an implicit runtime loop correction.
use crate::{AnimationClip, AnimationError, Interpolation, Playback, QuatKey, Skeleton, Vec3Key};

impl AnimationClip {
    /// Appends the first pose without rewriting original keys or channel modes.
    /// The interval is caller-authored in seconds; the resulting cycle is longer.
    /// Missing terminal keys are held to the old duration before closing, so all
    /// samples in the original interval keep their authored interpolation.
    /// Linear channels return continuously; STEP channels retain authored steps.
    /// Endpoint poses match, but STEP channels and seam derivatives can jump.
    /// This does not prove collision safety, cyclic root paths or contacts. Extracted
    /// translation now includes the return to the initial root position.
    /// # Errors
    /// Rejects foreign rigs, non-loop clips, nonpositive/unrepresentable tails
    /// and occupied channels with cubic interpolation. The source and
    /// its compiled motion caches remain unchanged.
    pub fn append_loop_frame(
        &self,
        skeleton: &Skeleton,
        tail: f32,
    ) -> Result<Self, AnimationError> {
        if !crate::rigs_match(&self.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let duration = self.duration + tail;
        if self.playback != Playback::Loop
            || !tail.is_finite()
            || tail <= 0.
            || !duration.is_finite()
            || duration <= self.duration
        {
            return Err(AnimationError::InvalidClipHeader);
        }
        let mut tracks = self.tracks.to_vec();
        for (track, mode) in tracks.iter_mut().zip(self.interpolation.iter()) {
            for (occupied, interpolation) in [
                (!track.translations.is_empty(), mode.translation),
                (!track.rotations.is_empty(), mode.rotation),
                (!track.scales.is_empty(), mode.scale),
            ] {
                if occupied && interpolation == Interpolation::CubicSpline {
                    return Err(AnimationError::UnsupportedLoopClosure);
                }
            }
            for keys in [&mut track.translations, &mut track.scales] {
                if let (Some(first), Some(last)) = (keys.first().copied(), keys.last().copied()) {
                    if last.time < self.duration {
                        keys.push(Vec3Key {
                            time: self.duration,
                            ..last
                        });
                    }
                    keys.push(Vec3Key {
                        time: duration,
                        ..first
                    });
                }
            }
            if let (Some(first), Some(last)) = (
                track.rotations.first().copied(),
                track.rotations.last().copied(),
            ) {
                if last.time < self.duration {
                    track.rotations.push(QuatKey {
                        time: self.duration,
                        ..last
                    });
                }
                track.rotations.push(QuatKey {
                    time: duration,
                    ..first
                });
            }
        }
        Self::new_with_interpolation(
            self.name.clone(),
            duration,
            self.playback,
            tracks,
            self.interpolation.to_vec(),
            skeleton,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Joint, JointTrack, TrackInterpolation, Transform};
    use glam::{Mat4, Quat, Vec3};
    use std::sync::Arc;
    #[test]
    fn closing_linear_loop_preserves_original_samples_and_owned_keys() {
        let rig = Skeleton::new(vec![Joint {
            name: Arc::from("root"),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let track = JointTrack {
            translations: vec![
                Vec3Key {
                    time: 0.1,
                    value: Vec3::Y,
                },
                Vec3Key {
                    time: 0.6,
                    value: Vec3::X,
                },
            ],
            rotations: vec![
                QuatKey {
                    time: 0.,
                    value: Quat::IDENTITY,
                },
                QuatKey {
                    time: 1.,
                    value: Quat::from_rotation_x(0.4),
                },
            ],
            scales: vec![Vec3Key {
                time: 0.,
                value: Vec3::ONE,
            }],
        };
        let original =
            AnimationClip::new("loop", 1., Playback::Loop, vec![track.clone()], &rig).unwrap();
        let closed = original.append_loop_frame(&rig, 0.1).unwrap();
        assert_eq!(original.tracks[0], track);
        assert_eq!(
            &closed.tracks[0].translations[..track.translations.len()],
            track.translations.as_slice()
        );
        assert_eq!(
            closed.try_sample_phase(&rig, 0.).unwrap(),
            closed.try_sample_phase(&rig, 1.).unwrap()
        );
        for tick in 0..=100 {
            let time = f64::from(tick) / 100.;
            let before = original.try_sample_phase64(&rig, time).unwrap();
            let after = closed.try_sample_time64(&rig, time).unwrap();
            assert!(
                before.global_matrices(&rig).unwrap()[0]
                    .abs_diff_eq(after.global_matrices(&rig).unwrap()[0], 1e-12)
            );
        }
        for tail in [0., -0.1, f32::NAN, f32::INFINITY, f32::MIN_POSITIVE] {
            assert!(original.append_loop_frame(&rig, tail).is_err());
        }
        let mut foreign = rig.joints().to_vec();
        foreign[0].bind_local.translation = Vec3::Z;
        assert!(
            original
                .append_loop_frame(&Skeleton::new(foreign).unwrap(), 0.1)
                .is_err()
        );
        let step = AnimationClip::new_with_interpolation(
            "step",
            1.,
            Playback::Loop,
            vec![track.clone()],
            vec![TrackInterpolation {
                translation: Interpolation::Step,
                ..Default::default()
            }],
            &rig,
        )
        .unwrap();
        let closed_step = step.append_loop_frame(&rig, 0.1).unwrap();
        for phase in [0., 0.2, 0.59, 0.6, 0.9, 1.] {
            let before = step.try_sample_phase64(&rig, phase).unwrap();
            let after = closed_step.try_sample_time64(&rig, phase).unwrap();
            assert!(
                before.global_matrices(&rig).unwrap()[0]
                    .abs_diff_eq(after.global_matrices(&rig).unwrap()[0], 1e-12)
            );
        }
        assert_eq!(
            closed_step.try_sample_phase(&rig, 0.).unwrap(),
            closed_step.try_sample_phase(&rig, 1.).unwrap()
        );
        let cubic = AnimationClip::new_with_tangents(
            "cubic",
            1.,
            Playback::Loop,
            vec![track.clone()],
            vec![TrackInterpolation {
                translation: Interpolation::CubicSpline,
                ..Default::default()
            }],
            vec![crate::JointTangents {
                translation: vec![[Vec3::ZERO; 2]; 2],
                ..Default::default()
            }],
            &rig,
        )
        .unwrap();
        assert!(matches!(
            cubic.append_loop_frame(&rig, 0.1),
            Err(AnimationError::UnsupportedLoopClosure)
        ));
        let clamp = AnimationClip::new("clamp", 1., Playback::Clamp, vec![track], &rig).unwrap();
        assert!(clamp.append_loop_frame(&rig, 0.1).is_err());
    }
}
