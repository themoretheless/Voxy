//! glTF animation channel, cubic tangent, and morph-weight track import.

use std::sync::Arc;

use glam::{Quat, Vec3};
use gltf::animation::util::ReadOutputs;
use voxy_animation::{
    AnimationClip, Interpolation, JointTangents, JointTrack, Playback, QuatKey, Skeleton,
    TrackInterpolation, Vec3Key,
};

use super::{ModelAsset, ModelError, ModelLimits, fail, morph};

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

impl ModelAsset {
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub(super) fn import_animations(
        gltf: &gltf::Gltf,
        data: &[&[u8]],
        skeleton: &Skeleton,
        mapping: &[usize],
        limits: ModelLimits,
        allow_morph: bool,
        morph_data: &mut morph::MorphImportData,
    ) -> Result<Vec<Arc<AnimationClip>>, ModelError> {
        let mut animations = Vec::new();
        let mut key_count = 0usize;
        for animation in gltf.animations() {
            let mut tracks = vec![JointTrack::default(); skeleton.joints().len()];
            let mut modes = vec![TrackInterpolation::default(); skeleton.joints().len()];
            let mut tangents = vec![JointTangents::default(); skeleton.joints().len()];
            let mut seen = std::collections::HashSet::new();
            let mut duration = 0f32;
            let mut property_tracks = Vec::new();
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
                let morph_count =
                    if target.property() == gltf::animation::Property::MorphTargetWeights {
                        if !allow_morph {
                            return Err(fail("morph animation unsupported"));
                        }
                        let count = morph::node_weights(&target.node())?.len();
                        if count == 0 {
                            return Err(fail("morph channel has no targets"));
                        }
                        count
                    } else {
                        1
                    };
                let output_count = channel
                    .sampler()
                    .input()
                    .count()
                    .checked_mul(if mode == Interpolation::CubicSpline {
                        3
                    } else {
                        1
                    })
                    .and_then(|n| n.checked_mul(morph_count))
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
                            .map(|(&time, value)| Vec3Key { time, value })
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
                            .map(|(&time, value)| Vec3Key { time, value })
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
                    ReadOutputs::MorphTargetWeights(values) => {
                        morph::import_weight_tracks(
                            &mut property_tracks,
                            target.node().index(),
                            morph_count,
                            &times,
                            values.into_f32().collect(),
                            mode,
                        )?;
                    }
                }
            }
            if allow_morph {
                morph_data.clips.push(
                    voxy_animation::PropertyClip::new(
                        animation.name().unwrap_or("animation"),
                        duration.max(f32::EPSILON),
                        Playback::Loop,
                        property_tracks,
                    )
                    .map_err(|e| fail(e.to_string()))?,
                );
            }
            animations.push(Arc::new(
                AnimationClip::new_with_tangents(
                    animation.name().unwrap_or("animation"),
                    duration.max(f32::EPSILON),
                    Playback::Loop,
                    tracks,
                    modes,
                    tangents,
                    skeleton,
                )
                .map_err(|e| fail(e.to_string()))?,
            ));
        }
        Ok(animations)
    }
}
