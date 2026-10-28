//! Property snapshots from Animator's existing target/source clocks and frozen fades.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct BoundPropertyClip {
    pub(super) source: Arc<AnimationClip>,
    pub(super) properties: Arc<PropertyClip>,
    pub(super) defaults: PropertySet,
}
#[derive(Clone, Debug)]
pub(crate) struct PropertyBindings {
    pub(super) defaults: PropertySet,
    pub(super) clips: Vec<BoundPropertyClip>,
}

impl PropertyBindings {
    pub(super) fn validate(
        defaults: &PropertySet,
        clips: &[(Arc<AnimationClip>, Arc<PropertyClip>)],
        rig: &Arc<[Joint]>,
    ) -> Result<(), AnimationError> {
        if clips.is_empty() || clips.len() > MAX_PROPERTY_TRACKS {
            return Err(AnimationError::InvalidGraphParam);
        }
        for (index, (clip, properties)) in clips.iter().enumerate() {
            if !rigs_match(rig, &clip.rig)
                || clip.duration != properties.duration()
                || clip.playback != properties.playback()
                || clips[..index]
                    .iter()
                    .any(|(previous, _)| Arc::ptr_eq(previous, clip))
                || properties.tracks().iter().any(|track| {
                    defaults
                        .get(track.name())
                        .is_none_or(|value| !value.same_kind(track.keys()[0].value))
                })
            {
                return Err(AnimationError::InvalidGraphParam);
            }
        }
        Ok(())
    }

    fn sample(&self, clip: &Arc<AnimationClip>, time: f64) -> Result<PropertySet, AnimationError> {
        let binding = self
            .clips
            .iter()
            .find(|binding| Arc::ptr_eq(&binding.source, clip))
            .ok_or(AnimationError::InvalidGraphParam)?;
        let sampled = binding.properties.sample_clock(time)?;
        PropertySet::new(
            binding
                .defaults
                .entries()
                .iter()
                .map(|(name, default)| (name.clone(), sampled.get(name).unwrap_or(*default)))
                .collect(),
        )
    }
}

impl Animator {
    /// Bind immutable property clips by exact skeletal-clip identity. Missing
    /// channels retain the supplied schema/defaults. Every future transition
    /// must reference a bound clip; names alone never select another revision.
    /// Attach before starting a fade; replacing a binding during a fade fails.
    ///
    /// # Errors
    /// Rejects foreign rigs, mismatched duration/playback, duplicate bindings,
    /// unknown/mismatched property schemas, excessive clip counts and invalid
    /// initial property samples. Failure preserves all previous animator state.
    pub fn bind_property_clips(
        &mut self,
        defaults: PropertySet,
        clips: Vec<(Arc<AnimationClip>, Arc<PropertyClip>)>,
    ) -> Result<(), AnimationError> {
        if self.transition.is_some() || clips.is_empty() || clips.len() > MAX_PROPERTY_TRACKS {
            return Err(AnimationError::InvalidGraphParam);
        }
        PropertyBindings::validate(&defaults, &clips, &self.current.rig)?;
        let clips = clips
            .into_iter()
            .map(|(source, properties)| BoundPropertyClip {
                source,
                properties,
                defaults: defaults.clone(),
            })
            .collect();
        let bindings = Arc::new(PropertyBindings { defaults, clips });
        let sampled = bindings.sample(&self.current, self.time)?;
        self.property_bindings = Some(bindings);
        self.accepted_properties = sampled;
        Ok(())
    }

    /// Register immutable clip revisions without replacing live/frozen source
    /// curves. Defaults belong to each revision; schemas must match. Old entries
    /// not needed by current/fade/new clips are pruned to bound repeated reloads.
    /// # Errors
    /// Rejects schema changes, duplicate/conflicting identities and clip budgets.
    /// Failed registration preserves clocks, bindings and accepted snapshots.
    pub fn register_property_clips(
        &mut self,
        defaults: PropertySet,
        clips: Vec<(Arc<AnimationClip>, Arc<PropertyClip>)>,
    ) -> Result<(), AnimationError> {
        let Some(previous) = &self.property_bindings else {
            return self.bind_property_clips(defaults, clips);
        };
        if defaults.len() != previous.defaults.len()
            || defaults.entries().iter().any(|(name, value)| {
                previous
                    .defaults
                    .get(name)
                    .is_none_or(|old| !old.same_kind(*value))
            })
        {
            return Err(AnimationError::InvalidGraphParam);
        }
        PropertyBindings::validate(&defaults, &clips, &self.current.rig)?;
        let mut retained = previous
            .clips
            .iter()
            .filter(|entry| {
                Arc::ptr_eq(&entry.source, &self.current)
                    || self
                        .transition
                        .as_ref()
                        .is_some_and(|fade| Arc::ptr_eq(&fade.source, &entry.source))
                    || clips
                        .iter()
                        .any(|(source, _)| Arc::ptr_eq(source, &entry.source))
            })
            .cloned()
            .collect::<Vec<_>>();
        for (source, properties) in clips {
            if let Some(old) = retained
                .iter()
                .find(|entry| Arc::ptr_eq(&entry.source, &source))
            {
                if old.properties.as_ref() != properties.as_ref() || old.defaults != defaults {
                    return Err(AnimationError::InvalidGraphParam);
                }
            } else {
                retained.push(BoundPropertyClip {
                    source,
                    properties,
                    defaults: defaults.clone(),
                });
            }
        }
        if retained.len() > MAX_PROPERTY_TRACKS {
            return Err(AnimationError::InvalidGraphParam);
        }
        let mut candidate = self.clone();
        candidate.property_bindings = Some(Arc::new(PropertyBindings {
            defaults,
            clips: retained,
        }));
        let displayed = candidate.sample_displayed_properties()?;
        if displayed != self.accepted_properties {
            return Err(AnimationError::InvalidGraphParam);
        }
        *self = candidate;
        Ok(())
    }

    /// Immutable property snapshot accepted with the current skeletal state.
    #[must_use]
    pub fn properties(&self) -> &PropertySet {
        &self.accepted_properties
    }

    pub(crate) fn sample_bound_properties(
        &self,
        clip: &Arc<AnimationClip>,
        time: f64,
    ) -> Result<PropertySet, AnimationError> {
        self.property_bindings.as_ref().map_or_else(
            || Ok(PropertySet::default()),
            |bindings| bindings.sample(clip, time),
        )
    }

    pub(crate) fn sample_displayed_properties(&self) -> Result<PropertySet, AnimationError> {
        if self.property_bindings.is_none() {
            return Ok(PropertySet::default());
        }
        let target = self.sample_bound_properties(&self.current, self.time)?;
        let Some(transition) = &self.transition else {
            return Ok(target);
        };
        let sampled;
        let source = if let Some(frozen) = &transition.source_properties {
            frozen.as_ref()
        } else {
            sampled = self.sample_bound_properties(&transition.source, transition.source_time)?;
            &sampled
        };
        PropertySet::blend(
            source,
            &target,
            (transition.elapsed / transition.duration) as f32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig() -> Skeleton {
        Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap()
    }
    fn pair(rig: &Skeleton, name: &str, a: f32, b: f32) -> (Arc<AnimationClip>, Arc<PropertyClip>) {
        let track = JointTrack {
            translations: vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::new(a, 0., 0.),
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::new(b, 0., 0.),
                },
            ],
            ..Default::default()
        };
        let clip =
            Arc::new(AnimationClip::new(name, 1., Playback::Loop, vec![track], rig).unwrap());
        let property = PropertyTrack::new(
            "shape",
            vec![
                PropertyKey {
                    time: 0.,
                    value: PropertyValue::Float(a),
                },
                PropertyKey {
                    time: 1.,
                    value: PropertyValue::Float(b),
                },
            ],
            1.,
        )
        .unwrap();
        let props = Arc::new(PropertyClip::new(name, 1., Playback::Loop, vec![property]).unwrap());
        (clip, props)
    }
    fn defaults() -> PropertySet {
        PropertySet::new(vec![
            ("shape".into(), PropertyValue::Float(0.)),
            ("unanimated".into(), PropertyValue::Float(3.)),
        ])
        .unwrap()
    }

    #[test]
    fn property_snapshots_follow_live_fades_repeated_interruptions_and_completion() {
        let rig = rig();
        let a = pair(&rig, "a", 0., 2.);
        let b = pair(&rig, "b", 4., 4.);
        let c = pair(&rig, "c", 8., 8.);
        let mut animator = Animator::try_new(a.0.clone()).unwrap();
        animator
            .bind_property_clips(defaults(), vec![a.clone(), b.clone(), c.clone()])
            .unwrap();
        let check = |frame: &AnimatorFrame, expected| {
            assert_eq!(frame.properties.get_float("shape"), Some(expected));
            assert_eq!(frame.pose.local()[0].translation.x, expected);
            assert_eq!(frame.properties.get_float("unanimated"), Some(3.));
        };
        check(&animator.advance_wall(&rig, 0.25).unwrap(), 0.5);
        animator.transition_to(b.0.clone(), 1.).unwrap();
        let live = animator.advance_wall(&rig, 0.25).unwrap();
        check(&live, 1.75);
        animator.transition_to(c.0.clone(), 1.).unwrap();
        check(&animator.advance_wall(&rig, 0.).unwrap(), 1.75);
        check(&animator.advance_wall(&rig, 0.25).unwrap(), 3.3125);
        animator.transition_to(b.0, 0.5).unwrap();
        check(&animator.advance_wall(&rig, 0.).unwrap(), 3.3125);
        check(&animator.advance_wall(&rig, 0.75).unwrap(), 4.);
        assert_eq!(animator.properties().get_float("shape"), Some(4.));
        // An earlier immutable frame remains unchanged after all later transitions.
        check(&live, 1.75);
    }

    #[test]
    fn property_step_boundaries_use_the_same_unnarrowed_f64_clock() {
        let rig = rig();
        let a = pair(&rig, "step", 0., 2.);
        let step = PropertyTrack::new_with_interpolation(
            "shape",
            vec![
                PropertyKey {
                    time: 0.,
                    value: PropertyValue::Float(0.),
                },
                PropertyKey {
                    time: 0.5,
                    value: PropertyValue::Float(1.),
                },
            ],
            1.,
            Interpolation::Step,
            vec![],
        )
        .unwrap();
        let props = Arc::new(PropertyClip::new("step", 1., Playback::Loop, vec![step]).unwrap());
        let mut animator = Animator::try_new(a.0.clone()).unwrap();
        animator
            .bind_property_clips(defaults(), vec![(a.0, props)])
            .unwrap();
        let before = 0.5_f64.next_down();
        let frame = animator.advance_wall(&rig, before).unwrap();
        assert_eq!(frame.properties.get_float("shape"), Some(0.));
        assert_eq!(
            animator
                .advance_wall(&rig, 0.5 - before)
                .unwrap()
                .properties
                .get_float("shape"),
            Some(1.)
        );
        assert_eq!(
            animator
                .advance_wall(&rig, 0.5)
                .unwrap()
                .properties
                .get_float("shape"),
            Some(0.)
        );
    }

    #[test]
    fn failed_property_sample_or_transition_preserves_clocks_and_snapshot() {
        let rig = rig();
        let clip = Arc::new(
            AnimationClip::new(
                "overflow",
                16.,
                Playback::Clamp,
                vec![JointTrack::default()],
                &rig,
            )
            .unwrap(),
        );
        let track = PropertyTrack::new_with_interpolation(
            "shape",
            vec![
                PropertyKey {
                    time: 0.,
                    value: PropertyValue::Float(0.),
                },
                PropertyKey {
                    time: 16.,
                    value: PropertyValue::Float(0.),
                },
            ],
            16.,
            Interpolation::CubicSpline,
            vec![
                (PropertyValue::Float(0.), PropertyValue::Float(f32::MAX)),
                (PropertyValue::Float(-f32::MAX), PropertyValue::Float(0.)),
            ],
        )
        .unwrap();
        let props =
            Arc::new(PropertyClip::new("overflow", 16., Playback::Clamp, vec![track]).unwrap());
        let mut animator = Animator::try_new(clip.clone()).unwrap();
        animator
            .bind_property_clips(defaults(), vec![(clip, props)])
            .unwrap();
        animator.set_speed(8.).unwrap();
        let before = animator.properties().clone();
        assert!(animator.advance_wall(&rig, 1.).is_err());
        assert_eq!(animator.normalized_phase(), 0.);
        assert_eq!(animator.properties(), &before);
        let foreign_revision = pair(&rig, "overflow", 1., 1.);
        assert!(animator.transition_to(foreign_revision.0, 0.).is_err());
        assert_eq!(animator.normalized_phase(), 0.);
        assert_eq!(animator.properties(), &before);
    }

    #[test]
    fn prepared_rigid_prefix_includes_property_receipt_and_frozen_snapshot() {
        let rig = rig();
        let a = pair(&rig, "a", 0., 2.);
        let b = pair(&rig, "b", 4., 4.);
        let mut animator = Animator::try_new(a.0.clone()).unwrap();
        animator
            .bind_property_clips(defaults(), vec![a.clone(), b.clone()])
            .unwrap();
        animator.advance_wall(&rig, 0.25).unwrap();
        animator.transition_to(b.0, 1.).unwrap();
        let plan = animator
            .prepare_root_rigid_fade_wall(&rig, 0.5, [true, false, false], 64)
            .unwrap()
            .unwrap();
        assert!(plan.matches_animator(&animator));
        let (_, prefix) = plan.prepare_accepted_frame(&rig, 0.25).unwrap();
        assert_eq!(prefix.properties.get_float("shape"), Some(1.75));
        let mut changed = animator.clone();
        changed.accepted_properties = PropertySet::default();
        assert!(!plan.matches_animator(&changed));
        assert!(animator.bind_property_clips(defaults(), vec![a]).is_err());
        assert!(plan.matches_animator(&animator));
    }
    #[test]
    fn reload_registration_keeps_source_defaults_and_bounds_unused_revisions() {
        let rig = rig();
        let a = pair(&rig, "a", 0., 2.);
        let b = pair(&rig, "b", 4., 4.);
        let c = pair(&rig, "c", 8., 8.);
        let with_default = |value| {
            PropertySet::new(vec![
                ("shape".into(), PropertyValue::Float(0.)),
                ("unanimated".into(), PropertyValue::Float(value)),
            ])
            .unwrap()
        };
        let mut animator = Animator::try_new(a.0.clone()).unwrap();
        animator.bind_property_clips(defaults(), vec![a]).unwrap();
        animator.advance_wall(&rig, 0.25).unwrap();
        animator
            .register_property_clips(with_default(9.), vec![b.clone()])
            .unwrap();
        animator.transition_to(b.0, 1.).unwrap();
        let frame = animator.advance_wall(&rig, 0.25).unwrap();
        assert_eq!(frame.properties.get_float("shape"), Some(1.75));
        assert_eq!(frame.properties.get_float("unanimated"), Some(4.5));
        let plan = animator
            .prepare_root_rigid_fade_wall(&rig, 0.25, [true, false, false], 64)
            .unwrap()
            .unwrap();
        animator
            .register_property_clips(with_default(15.), vec![c.clone()])
            .unwrap();
        assert!(!plan.matches_animator(&animator));
        assert_eq!(animator.properties(), &frame.properties);
        animator.transition_to(c.0.clone(), 1.).unwrap();
        let frozen = animator.advance_wall(&rig, 0.25).unwrap();
        assert_eq!(frozen.properties.get_float("shape"), Some(3.3125));
        assert_eq!(frozen.properties.get_float("unanimated"), Some(7.125));
        assert!(
            animator
                .register_property_clips(with_default(42.), vec![c])
                .is_err()
        );
        assert_eq!(animator.properties(), &frozen.properties);
        let phase = animator.normalized_phase();
        for _ in 0..300 {
            animator
                .register_property_clips(
                    with_default(3.),
                    vec![pair(&rig, "unused-revision", 1., 1.)],
                )
                .unwrap();
            assert!(animator.property_bindings.as_ref().unwrap().clips.len() <= 3);
        }
        assert_eq!(animator.normalized_phase(), phase);
        assert_eq!(animator.properties(), &frozen.properties);
    }
}
