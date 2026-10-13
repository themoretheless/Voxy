//! Explicit Quaternius-to-Voxy humanoid pose profile. This prepares authored
//! gait poses; contact events, actor motion and physical admission stay outside.
use glam::{Quat, Vec3};
use std::sync::Arc;
use voxy_animation::{ModelRotationRetarget, Pose, RetargetBinding, RetargetJoint, Skeleton};
use voxy_render::ModelAsset;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocomotionClip {
    Idle,
    Walk,
    Jog,
    Sprint,
    JumpStart,
    JumpLoop,
    JumpLand,
}
impl LocomotionClip {
    fn slot(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Walk => 1,
            Self::Jog => 2,
            Self::Sprint => 3,
            Self::JumpStart => 4,
            Self::JumpLoop => 5,
            Self::JumpLand => 6,
        }
    }
}
#[derive(Debug)]
pub struct VoxyHumanoidLocomotion {
    source: Arc<ModelAsset>,
    binding: ModelRotationRetarget,
    clips: [usize; 7],
    leg_ratio: f32,
    pelvis: u16,
    target_scales: Vec<Vec3>,
    neutral: Pose,
}
impl VoxyHumanoidLocomotion {
    /// Compiles the named Voxy humanoid preset against an explicitly owned
    /// source asset. No asset registry, filesystem access or mutable rig copy.
    /// # Errors
    /// Requires all authored source/target bones, identity target bind rotations,
    /// positive leg lengths and the preset's unit-scale source hip ancestors.
    pub fn new(
        source: Arc<ModelAsset>,
        target: &Skeleton,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let target_index = |name: &str| {
            target
                .joints()
                .iter()
                .position(|joint| joint.name.as_ref() == name)
                .ok_or_else(|| format!("missing Voxy humanoid joint {name}"))
        };
        if target
            .joints()
            .iter()
            .any(|joint| joint.bind_local.rotation != Quat::IDENTITY)
        {
            return Err("Voxy humanoid preset requires its canonical bind rotations".into());
        }
        let pelvis = target_index("pelvis")?;
        if target.joints()[pelvis].parent.is_some() {
            return Err("Voxy pelvis must be a skeleton root".into());
        }
        let mut topology = vec![
            ("spine".to_owned(), "pelvis".to_owned()),
            ("chest".into(), "spine".into()),
            ("head".into(), "chest".into()),
        ];
        for side in ["left", "right"] {
            for (child, parent) in [
                ("upper_arm", "chest"),
                ("forearm", "upper_arm"),
                ("hand", "forearm"),
                ("thigh", "pelvis"),
                ("shin", "thigh"),
                ("foot", "shin"),
            ] {
                topology.push((
                    format!("{side}_{child}"),
                    if parent == "chest" || parent == "pelvis" {
                        parent.to_owned()
                    } else {
                        format!("{side}_{parent}")
                    },
                ));
            }
            for finger in 0..5 {
                for segment in 0..3 {
                    topology.push((
                        format!("{side}_finger_{finger}_{segment}"),
                        if segment == 0 {
                            format!("{side}_hand")
                        } else {
                            format!("{side}_finger_{finger}_{}", segment - 1)
                        },
                    ));
                }
            }
        }
        for (child, parent) in topology {
            if target.joints()[target_index(&child)?]
                .parent
                .map(usize::from)
                != Some(target_index(&parent)?)
            {
                return Err(format!("Voxy preset parent mismatch for {child}").into());
            }
        }
        let source_bind = source
            .skeleton
            .bind_pose64()
            .global_matrices(&source.skeleton)?;
        let target_bind = target.bind_pose64().global_matrices(target)?;
        let s: [usize; 3] = ["DEF-thigh.L", "DEF-shin.L", "DEF-foot.L"]
            .map(|name| source.resolve_joint_name(name).map(usize::from))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .unwrap();
        let t: [usize; 3] = ["left_thigh", "left_shin", "left_foot"]
            .map(target_index)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .unwrap();
        let length = |matrices: &[glam::DMat4], indices: [usize; 3]| {
            let [a, b, c] = indices.map(|index| matrices[index].w_axis.truncate());
            a.distance(b) + b.distance(c)
        };
        let leg_ratio = (length(&target_bind, t) / length(&source_bind, s)) as f32;
        if !leg_ratio.is_finite() || leg_ratio <= 0. {
            return Err("invalid humanoid leg ratio".into());
        }
        let hip = usize::from(source.resolve_joint_name("DEF-hips")?);
        let mut parent = source.skeleton.joints()[hip].parent;
        let mut translation_basis = Quat::IDENTITY;
        while let Some(index) = parent {
            let joint = &source.skeleton.joints()[usize::from(index)];
            if joint.bind_local.scale != Vec3::ONE {
                return Err("source hip ancestor scale differs from the authored preset".into());
            }
            translation_basis = (joint.bind_local.rotation * translation_basis).normalize();
            parent = joint.parent;
        }
        let mut pairs: Vec<(String, String)> = [
            ("DEF-hips", "pelvis"),
            ("DEF-spine.001", "spine"),
            ("DEF-spine.003", "chest"),
            ("DEF-head", "head"),
        ]
        .map(|(a, b)| (a.into(), b.into()))
        .to_vec();
        for (side, suffix) in [("left", "L"), ("right", "R")] {
            for part in ["upper_arm", "forearm", "hand", "thigh", "shin", "foot"] {
                pairs.push((format!("DEF-{part}.{suffix}"), format!("{side}_{part}")));
            }
            for (finger, part) in ["thumb", "f_index", "f_middle", "f_ring", "f_pinky"]
                .into_iter()
                .enumerate()
            {
                for segment in 0..3 {
                    pairs.push((
                        format!("DEF-{part}.{:02}.{suffix}", segment + 1),
                        format!("{side}_finger_{finger}_{segment}"),
                    ));
                }
            }
        }
        let mapping = pairs
            .iter()
            .map(|(from, to)| {
                let index = usize::from(source.resolve_joint_name(from)?);
                target_index(to)?;
                Ok(RetargetJoint {
                    source: source.skeleton.joints()[index].name.clone(),
                    target: to.as_str().into(),
                    rotation_basis: Quat::IDENTITY,
                    translation_basis,
                    translation_scale: if to == "pelvis" { leg_ratio } else { 0. },
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let mut reference = target.bind_pose();
        for (side, sign) in [("left", 1.), ("right", -1.)] {
            let upper = target_index(&format!("{side}_upper_arm"))?;
            let forearm = target_index(&format!("{side}_forearm"))?;
            let hand = target_index(&format!("{side}_hand"))?;
            let a = target.joints()[forearm].bind_local.translation;
            let b = target.joints()[hand].bind_local.translation;
            let theta_upper = (-a.y).atan2(sign * a.x);
            let theta_forearm = (-b.y).atan2(sign * b.x);
            // Authored palm axis of the existing Voxy mannequin (.02,-.12,0).
            let theta_hand = 0.12f32.atan2(0.02);
            for (joint, angle) in [
                (upper, theta_upper),
                (forearm, theta_forearm - theta_upper),
                (hand, theta_hand - theta_forearm),
            ] {
                reference.set_joint_rotation(joint, Quat::from_rotation_z(sign * angle))?;
            }
        }
        let binding = RetargetBinding::new(&source.skeleton, target, &mapping)?
            .with_target_reference_pose(&reference)?
            .compile_model_rotations(Quat::IDENTITY)?;
        let clips: [usize; 7] = [
            "Idle_Loop",
            "Walk_Loop",
            "Jog_Fwd_Loop",
            "Sprint_Loop",
            "Jump_Start",
            "Jump_Loop",
            "Jump_Land",
        ]
        .map(|name| {
            source
                .animations
                .iter()
                .position(|clip| clip.name() == name)
                .ok_or_else(|| format!("missing locomotion clip {name}"))
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .unwrap();
        Ok(Self {
            source,
            binding,
            clips,
            leg_ratio,
            pelvis: pelvis.try_into()?,
            neutral: target.bind_pose(),
            target_scales: target
                .joints()
                .iter()
                .map(|joint| joint.bind_local.scale)
                .collect(),
        })
    }
    /// Samples a pose for a physically controlled actor. The pelvis translation
    /// returns to canonical bind and its authored offset is returned separately.
    /// The offset is absolute relative to bind, NOT an interval motion delta;
    /// callers must not accumulate it or add it to a ballistic jump each frame.
    /// Rotation and all other joints retain the authored retargeted animation.
    /// # Errors
    /// Rejects invalid phases, a foreign target rig and invalid palettes.
    pub fn pose_phase_in_place(
        &self,
        clip: LocomotionClip,
        phase: f64,
        target: &Skeleton,
    ) -> Result<(Pose, Vec3), Box<dyn std::error::Error>> {
        self.in_place_pose(self.pose_phase(clip, phase)?,target)
    }
    fn in_place_pose(&self,pose:Pose,target:&Skeleton)->Result<(Pose,Vec3),Box<dyn std::error::Error>> {
        let index = usize::from(self.pelvis);
        let bind = target
            .joints()
            .get(index)
            .ok_or("missing target pelvis")?
            .bind_local
            .translation;
        let offset = pose.local()[index].translation - bind;
        // The shared extraction API verifies rig identity and rebuilds the
        // palette before publication; no stale pre-extraction palette is used.
        let frame = voxy_animation::AnimatorFrame {
            pose,
            skin_matrices: Vec::new(),
            root_motion: offset,
            root_motion_joint: self.pelvis,
            transition_weight: 1.,
        };
        let (frame, offset) = frame.into_in_place_translation(target, [true; 3])?;
        Ok((frame.pose, offset))
    }
    /// Authored jump pose synchronized to the demo's physical jump clock.
    /// Actor translation remains external. Contact IK must run on the returned
    /// pose before publication; this method only owns clip phases and blending.
    /// # Errors
    /// Rejects non-finite time, a foreign rig and invalid sampled poses.
    pub fn jump_pose(
        &self,
        time: f64,
        target: &Skeleton,
    ) -> Result<Pose, Box<dyn std::error::Error>> {
        use crate::jump_motion::{FLIGHT, LANDING, TAKEOFF};
        if !time.is_finite() {
            return Err("jump time must be finite".into());
        }
        let cycle = time.rem_euclid(6.);
        let t = cycle.rem_euclid(2.);
        let sample = |clip, phase| self.pose_phase_in_place(clip, phase, target).map(|v| v.0);
        let phase_ease = |t: f64| {
            let t = t.clamp(0., 1.);
            t * t * t * (10. + t * (-15. + 6. * t))
        };
        let ease = |t| phase_ease(t) as f32;
        // The physical demo initializes in the canonical target rest pose.
        // Keep that same pose at rest, checking its rig identity through blend.
        let idle = || -> Result<Pose, Box<dyn std::error::Error>> {
            Ok(Pose::blend(&self.neutral, &target.bind_pose(), 0.)?)
        };
        if cycle >= 4. || t >= 1.65 {
            return idle();
        }
        if t < TAKEOFF {
            let start = sample(
                LocomotionClip::JumpStart,
                1. - (1. - t / TAKEOFF) * phase_ease((TAKEOFF - t) / 0.10),
            )?;
            return Ok(Pose::blend(&idle()?, &start, ease(t / 0.18))?);
        }
        if t < LANDING {
            let flight = sample(
                LocomotionClip::JumpLoop,
                (t - TAKEOFF) / FLIGHT * phase_ease((t - TAKEOFF) / 0.10),
            )?;
            let landing = sample(LocomotionClip::JumpLand, 0.)?;
            return Ok(Pose::blend(
                &flight,
                &landing,
                ease((t - (LANDING - 0.10)) / 0.10),
            )?);
        }
        let land = sample(
            LocomotionClip::JumpLand,
            (t - LANDING) / (1.65 - LANDING) * phase_ease((t - LANDING) / 0.10),
        )?;
        Ok(Pose::blend(&land, &idle()?, ease((t - 1.50) / 0.15))?)
    }
    pub(crate) fn source_asset(&self) -> Arc<ModelAsset> {
        self.source.clone()
    }
    pub fn leg_ratio(&self) -> f32 {
        self.leg_ratio
    }
    pub fn duration(&self, clip: LocomotionClip) -> f32 {
        self.source.animations[self.clips[clip.slot()]].duration()
    }
    /// Samples an authored phase, retaining the original target rig identity.
    /// # Errors
    /// Rejects invalid phase, source pose and target palette failures.
    pub fn pose_phase(
        &self,
        clip: LocomotionClip,
        phase: f64,
    ) -> Result<Pose, Box<dyn std::error::Error>> {
        let source = self
            .source
            .sample_pose_phase(Some(self.clips[clip.slot()]), phase)?;
        self.retarget_source_pose(&source)
    }
    fn retarget_source_pose(&self,source:&Pose)->Result<Pose,Box<dyn std::error::Error>> {
        let mut pose = self.binding.apply_pose(source)?;
        // This humanoid preset transfers motion, not source character dimensions.
        // Keep the target physical bone metric, including any authored target
        // scales, rather than importing exporter scale channels into contact IK.
        for (joint, &scale) in self.target_scales.iter().enumerate() {
            pose.set_joint_scale(joint, scale)?;
        }
        Ok(pose)
    }
}

/// One source clock and one blended pose for every consumer of a grounded actor.
/// Target bindings are read-only consumers; this owner does not move the actor
/// or infer foot contacts from a clip name/normalized phase.
#[derive(Clone,Debug)]
pub struct GroundedPlayback {
    source:Arc<ModelAsset>,
    clips:[Arc<voxy_animation::AnimationClip>;4],
    animator:voxy_animation::Animator,
    frame:Pose,
    selected:LocomotionClip,
}
impl GroundedPlayback {
    pub fn new(library:&VoxyHumanoidLocomotion,clip:LocomotionClip)->Result<Self,Box<dyn std::error::Error>> {
        Self::grounded(clip)?;
        let clips=std::array::from_fn(|i|library.source.animations[library.clips[i]].clone());
        let mut animator=voxy_animation::Animator::try_new(clips[clip.slot()].clone())?;
        let frame=animator.advance_wall(&library.source.skeleton,0.)?.pose;
        Ok(Self {source:library.source.clone(),clips,animator,frame,selected:clip})
    }
    fn grounded(clip:LocomotionClip)->Result<(),Box<dyn std::error::Error>> {
        if clip.slot()>=4 {return Err("grounded playback requires idle, walk, jog or sprint".into());}
        Ok(())
    }
    pub fn selected(&self)->LocomotionClip {self.selected}
    pub fn normalized_phase(&self)->f64 {self.animator.normalized_phase()}
    /// Contact consumers must use these source/target/frozen-pose identities,
    /// rather than assigning an interrupted source a guessed gait phase.
    pub fn pose_blend_phases(&self)->voxy_animation::PoseBlendPhases<'_> {
        self.animator.pose_blend_phases()
    }
    /// A repeated selection preserves the existing transition and both clocks.
    /// Interrupted fades reuse Animator's frozen displayed source pose.
    pub fn transition_to(&mut self,clip:LocomotionClip,duration:f32)->Result<(),Box<dyn std::error::Error>> {
        Self::grounded(clip)?;
        let mut candidate=self.animator.clone();
        let phase=if matches!(clip,LocomotionClip::Idle) {0.} else {candidate.normalized_phase()};
        candidate.transition_to_at_phase(self.clips[clip.slot()].clone(),duration,phase)?;
        if clip==self.selected {return Ok(());}
        let frame=candidate.advance_wall(&self.source.skeleton,0.)?.pose;
        self.animator=candidate;self.frame=frame;self.selected=clip;
        Ok(())
    }
    /// Failed sampling preserves the previous clock, transition and source pose.
    pub fn advance(&mut self,dt:f64)->Result<(),Box<dyn std::error::Error>> {
        let frame=self.animator.advance_wall(&self.source.skeleton,dt)?.pose;
        self.frame=frame;
        Ok(())
    }
    /// Both render and physical-shell bindings read this exact same source pose.
    /// Returns the authored pelvis offset separately; it is not a motion delta.
    pub fn pose_in_place(&self,library:&VoxyHumanoidLocomotion,target:&Skeleton)->Result<(Pose,Vec3),Box<dyn std::error::Error>> {
        if !Arc::ptr_eq(&self.source,&library.source) {return Err("grounded playback belongs to another source revision".into());}
        library.in_place_pose(library.retarget_source_pose(&self.frame)?,target)
    }
}

/// Explicit preparation recipe for the pinned Quaternius source pack. Original
/// assets stay untouched; only Jog gains a one-frame, 24 Hz closing interval.
/// Other immutable clip allocations are shared unchanged.
pub fn prepare_quaternius_gait_source(mut source: ModelAsset) -> Result<ModelAsset, Box<dyn std::error::Error>> {
    let indices: Vec<_> = source.animations.iter().enumerate().filter(|(_,clip)|clip.name()=="Jog_Fwd_Loop").map(|(i,_)|i).collect();
    if indices.len()!=1 {return Err("Quaternius preparation requires one uniquely named Jog_Fwd_Loop".into());}
    let index=indices[0];
    let clip=&source.animations[index];
    if clip.try_sample_phase(&source.skeleton,0.)? != clip.try_sample_phase(&source.skeleton,1.)? {
        source.animations[index]=Arc::new(clip.append_loop_frame(&source.skeleton,1. / 24.)?);
    }
    Ok(source)
}
