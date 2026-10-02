//! Typed hand actions and profile bindings; attach/sync belongs to the session.
use crate::XrRuntimeError;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hand {
    Left,
    Right,
}
impl Hand {
    fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}
#[derive(Debug)]
pub struct HandInput {
    pub select: openxr::ActionState<bool>,
    pub trigger: openxr::ActionState<f32>,
    pub squeeze: openxr::ActionState<f32>,
    pub stick: openxr::ActionState<openxr::Vector2f>,
    pub grip_active: bool,
    pub aim_active: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct HapticPulse {
    pub amplitude: f32,
    /// Zero requests the runtime's default frequency; positive values are Hz.
    pub frequency: f32,
    pub duration: std::time::Duration,
}
impl HapticPulse {
    /// # Errors
    /// Rejects non-finite/out-of-range values and zero/overflowing durations.
    pub fn validate(self) -> Result<i64, XrRuntimeError> {
        let nanos =
            i64::try_from(self.duration.as_nanos()).map_err(|_| XrRuntimeError::InvalidHaptic)?;
        if !self.amplitude.is_finite()
            || !(0.0..=1.0).contains(&self.amplitude)
            || !self.frequency.is_finite()
            || self.frequency < 0.0
            || nanos == 0
        {
            return Err(XrRuntimeError::InvalidHaptic);
        }
        Ok(nanos)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerProfile {
    Simple,
    OculusTouch,
}

pub struct XrActions {
    pub set: openxr::ActionSet,
    pub hands: [openxr::Path; 2],
    pub grip: openxr::Action<openxr::Posef>,
    pub aim: openxr::Action<openxr::Posef>,
    pub select: openxr::Action<bool>,
    pub trigger: openxr::Action<f32>,
    pub squeeze: openxr::Action<f32>,
    pub stick: openxr::Action<openxr::Vector2f>,
    pub haptic: openxr::Action<openxr::Haptic>,
}
impl std::fmt::Debug for XrActions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XrActions")
            .field("hands", &self.hands)
            .finish_non_exhaustive()
    }
}
impl XrActions {
    /// Creates identity-offset grip and aim spaces for one hand, in that order.
    /// Retain these spaces for the session lifetime and locate them against the
    /// same tracking space used for the eyes, at the frame's predicted display time.
    /// Sync actions first and respect both action activity and location validity.
    /// # Errors
    /// Reports runtime errors for incompatible actions/session or space creation.
    pub fn create_hand_spaces<G: openxr::Graphics>(
        &self,
        session: &openxr::Session<G>,
        hand: Hand,
    ) -> Result<(openxr::Space, openxr::Space), XrRuntimeError> {
        let path = self.hands[hand.index()];
        let grip = self
            .grip
            .create_space(session, path, openxr::Posef::IDENTITY)
            .map_err(XrRuntimeError::Runtime)?;
        let aim = self
            .aim
            .create_space(session, path, openxr::Posef::IDENTITY)
            .map_err(XrRuntimeError::Runtime)?;
        Ok((grip, aim))
    }

    /// Reads one hand after `sync`; inactive values must not drive gameplay.
    /// # Errors
    /// Returns runtime errors if the session/action lifecycle is invalid.
    pub fn read_hand<G: openxr::Graphics>(
        &self,
        session: &openxr::Session<G>,
        hand: Hand,
    ) -> Result<HandInput, XrRuntimeError> {
        let path = self.hands[hand.index()];
        Ok(HandInput {
            select: self
                .select
                .state(session, path)
                .map_err(XrRuntimeError::Runtime)?,
            trigger: self
                .trigger
                .state(session, path)
                .map_err(XrRuntimeError::Runtime)?,
            squeeze: self
                .squeeze
                .state(session, path)
                .map_err(XrRuntimeError::Runtime)?,
            stick: self
                .stick
                .state(session, path)
                .map_err(XrRuntimeError::Runtime)?,
            grip_active: self
                .grip
                .is_active(session, path)
                .map_err(XrRuntimeError::Runtime)?,
            aim_active: self
                .aim
                .is_active(session, path)
                .map_err(XrRuntimeError::Runtime)?,
        })
    }

    /// # Errors
    /// Rejects invalid pulses before calling the runtime, then reports feedback errors.
    pub fn vibrate<G: openxr::Graphics>(
        &self,
        session: &openxr::Session<G>,
        hand: Hand,
        pulse: HapticPulse,
    ) -> Result<(), XrRuntimeError> {
        let nanos = pulse.validate()?;
        let event = openxr::HapticVibration::new()
            .amplitude(pulse.amplitude)
            .frequency(pulse.frequency)
            .duration(openxr::Duration::from_nanos(nanos));
        self.haptic
            .apply_feedback(session, self.hands[hand.index()], &event)
            .map_err(XrRuntimeError::Runtime)
    }

    /// # Errors
    /// Returns runtime errors if the haptic action/session is unavailable.
    pub fn stop_vibration<G: openxr::Graphics>(
        &self,
        session: &openxr::Session<G>,
        hand: Hand,
    ) -> Result<(), XrRuntimeError> {
        self.haptic
            .stop_feedback(session, self.hands[hand.index()])
            .map_err(XrRuntimeError::Runtime)
    }
    /// # Errors
    /// Returns runtime errors, including attachment to an incompatible/already attached session.
    pub fn attach<G: openxr::Graphics>(
        &self,
        session: &openxr::Session<G>,
    ) -> Result<(), XrRuntimeError> {
        session
            .attach_action_sets(&[&self.set])
            .map_err(XrRuntimeError::Runtime)
    }

    /// Synchronizes both hand subactions. Call once each active frame before reading states.
    /// # Errors
    /// Returns session focus/lifecycle or action synchronization errors.
    pub fn sync<G: openxr::Graphics>(
        &self,
        session: &openxr::Session<G>,
    ) -> Result<(), XrRuntimeError> {
        session
            .sync_actions(&[openxr::ActiveActionSet::new(&self.set)])
            .map_err(XrRuntimeError::Runtime)
    }

    /// Creates one action set with left/right subaction paths and suggests a profile.
    /// Attach the set once to a matching session before syncing/reading actions.
    /// Simple controllers leave analog inputs unbound; their state is inactive.
    /// # Errors
    /// Returns path/action creation or unsupported profile binding errors.
    pub fn new(
        instance: &openxr::Instance,
        profile: ControllerProfile,
    ) -> Result<Self, XrRuntimeError> {
        let path = |name: &str| {
            instance
                .string_to_path(name)
                .map_err(XrRuntimeError::Runtime)
        };
        let hands = [path("/user/hand/left")?, path("/user/hand/right")?];
        let set = instance
            .create_action_set("voxy_input", "Voxy input", 0)
            .map_err(XrRuntimeError::Runtime)?;
        let actions = Self {
            grip: set
                .create_action("grip_pose", "Grip pose", &hands)
                .map_err(XrRuntimeError::Runtime)?,
            aim: set
                .create_action("aim_pose", "Aim pose", &hands)
                .map_err(XrRuntimeError::Runtime)?,
            select: set
                .create_action("select", "Select", &hands)
                .map_err(XrRuntimeError::Runtime)?,
            trigger: set
                .create_action("trigger", "Trigger", &hands)
                .map_err(XrRuntimeError::Runtime)?,
            squeeze: set
                .create_action("squeeze", "Squeeze", &hands)
                .map_err(XrRuntimeError::Runtime)?,
            stick: set
                .create_action("stick", "Stick", &hands)
                .map_err(XrRuntimeError::Runtime)?,
            haptic: set
                .create_action("haptic", "Haptic", &hands)
                .map_err(XrRuntimeError::Runtime)?,
            set,
            hands,
        };
        let mut bindings = Vec::new();
        for hand in ["/user/hand/left", "/user/hand/right"] {
            bindings.push(openxr::Binding::new(
                &actions.grip,
                path(&format!("{hand}/input/grip/pose"))?,
            ));
            bindings.push(openxr::Binding::new(
                &actions.aim,
                path(&format!("{hand}/input/aim/pose"))?,
            ));
            bindings.push(openxr::Binding::new(
                &actions.haptic,
                path(&format!("{hand}/output/haptic"))?,
            ));
            match profile {
                ControllerProfile::Simple => bindings.push(openxr::Binding::new(
                    &actions.select,
                    path(&format!("{hand}/input/select/click"))?,
                )),
                ControllerProfile::OculusTouch => {
                    bindings.push(openxr::Binding::new(
                        &actions.select,
                        path(&format!("{hand}/input/thumbstick/click"))?,
                    ));
                    bindings.push(openxr::Binding::new(
                        &actions.trigger,
                        path(&format!("{hand}/input/trigger/value"))?,
                    ));
                    bindings.push(openxr::Binding::new(
                        &actions.squeeze,
                        path(&format!("{hand}/input/squeeze/value"))?,
                    ));
                    bindings.push(openxr::Binding::new(
                        &actions.stick,
                        path(&format!("{hand}/input/thumbstick"))?,
                    ));
                }
            }
        }
        let profile_path = match profile {
            ControllerProfile::Simple => "/interaction_profiles/khr/simple_controller",
            ControllerProfile::OculusTouch => "/interaction_profiles/oculus/touch_controller",
        };
        instance
            .suggest_interaction_profile_bindings(path(profile_path)?, &bindings)
            .map_err(XrRuntimeError::Runtime)?;
        Ok(actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn haptic_limits_checked_without_runtime() {
        let valid = HapticPulse {
            amplitude: 0.5,
            frequency: 0.0,
            duration: std::time::Duration::from_millis(10),
        };
        assert_eq!(valid.validate().unwrap(), 10_000_000);
        for pulse in [
            HapticPulse {
                amplitude: f32::NAN,
                ..valid
            },
            HapticPulse {
                amplitude: -0.1,
                ..valid
            },
            HapticPulse {
                amplitude: 1.1,
                ..valid
            },
            HapticPulse {
                frequency: f32::INFINITY,
                ..valid
            },
            HapticPulse {
                frequency: -1.0,
                ..valid
            },
            HapticPulse {
                duration: std::time::Duration::ZERO,
                ..valid
            },
            HapticPulse {
                duration: std::time::Duration::MAX,
                ..valid
            },
        ] {
            assert!(matches!(
                pulse.validate(),
                Err(XrRuntimeError::InvalidHaptic)
            ));
        }
        assert_eq!(Hand::Left.index(), 0);
        assert_eq!(Hand::Right.index(), 1);
    }
}
