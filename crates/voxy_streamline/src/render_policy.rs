//! Translate renderer FG choices using explicit SDK support and live state.
use crate::{FrameGeneration, FrameGenerationState, StreamlineError, StreamlineRuntime};
use voxy_render::{FrameGenerationCapabilities, FrameGenerationError, FrameGenerationMode};

/// Report SDK capabilities without querying or consuming presentation counters.
/// `feature_available` must come from SDK feature support for the selected adapter.
/// Runtime error flags remain in the original state and are not support heuristics.
#[must_use]
pub const fn capabilities(
    state: &FrameGenerationState,
    feature_available: bool,
) -> FrameGenerationCapabilities {
    FrameGenerationCapabilities {
        available: feature_available && state.maximum_generated_frames > 0,
        max_generated_frames: state.maximum_generated_frames,
        dynamic_available: feature_available && state.dynamic_supported == 1,
    }
}

/// Preserve a renderer request exactly after validating SDK limits and Reflex.
/// # Errors
/// Returns the renderer's specific capability, Reflex, count or target error.
pub fn resolve(
    mode: FrameGenerationMode,
    state: &FrameGenerationState,
    feature_available: bool,
    reflex_active: bool,
) -> Result<FrameGeneration, FrameGenerationError> {
    mode.validate(capabilities(state, feature_available), reflex_active)?;
    Ok(match mode {
        FrameGenerationMode::Off => FrameGeneration::Off,
        FrameGenerationMode::Fixed { generated_frames } => {
            FrameGeneration::Fixed { generated_frames }
        }
        FrameGenerationMode::Dynamic { target_fps } => FrameGeneration::Dynamic { target_fps },
    })
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConfigurationError {
    Policy(FrameGenerationError),
    Runtime(StreamlineError),
}

impl StreamlineRuntime {
    /// Apply a renderer policy after explicit snapshot-based preflight.
    /// The native bridge rechecks current SDK capabilities and accepted Reflex
    /// state. This call does not generate frames or install presentation hooks.
    /// Snapshot preflight does not consume counters; native configuration can.
    /// # Errors
    /// Distinguishes renderer policy rejection from native SDK/platform errors.
    pub fn configure_renderer_frame_generation(
        &mut self,
        viewport: u32,
        mode: FrameGenerationMode,
        state: &FrameGenerationState,
        feature_available: bool,
        reflex_active: bool,
    ) -> Result<(), ConfigurationError> {
        let request = resolve(mode, state, feature_available, reflex_active)
            .map_err(ConfigurationError::Policy)?;
        self.configure_frame_generation(viewport, request)
            .map_err(ConfigurationError::Runtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_request_preserves_multiplier_and_dynamic_target() {
        let state = FrameGenerationState {
            maximum_generated_frames: 3,
            dynamic_supported: 1,
            ..FrameGenerationState::default()
        };
        for count in [1, 2, 3] {
            assert_eq!(
                resolve(
                    FrameGenerationMode::Fixed {
                        generated_frames: count
                    },
                    &state,
                    true,
                    true
                ),
                Ok(FrameGeneration::Fixed {
                    generated_frames: count
                })
            );
        }
        for target_fps in [None, Some(144.0)] {
            assert_eq!(
                resolve(
                    FrameGenerationMode::Dynamic { target_fps },
                    &state,
                    true,
                    true
                ),
                Ok(FrameGeneration::Dynamic { target_fps })
            );
        }
        let request = FrameGenerationMode::Fixed {
            generated_frames: 4,
        };
        assert_eq!(
            resolve(request, &state, true, true),
            Err(FrameGenerationError::InvalidFrameCount)
        );
        assert_eq!(
            resolve(request, &state, true, false),
            Err(FrameGenerationError::ReflexRequired)
        );
        assert_eq!(
            resolve(request, &state, false, true),
            Err(FrameGenerationError::Unavailable)
        );
        assert_eq!(
            resolve(FrameGenerationMode::Off, &state, false, false),
            Ok(FrameGeneration::Off)
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn policy_rejection_precedes_native_call_and_valid_policy_preserves_platform_error() {
        let mut runtime = StreamlineRuntime {
            thread_bound: std::marker::PhantomData,
        };
        let state = FrameGenerationState::default();
        assert_eq!(
            runtime.configure_renderer_frame_generation(
                0,
                FrameGenerationMode::Fixed {
                    generated_frames: 1
                },
                &state,
                false,
                false
            ),
            Err(ConfigurationError::Policy(
                FrameGenerationError::Unavailable
            ))
        );
        assert_eq!(
            runtime.configure_renderer_frame_generation(
                0,
                FrameGenerationMode::Off,
                &state,
                false,
                false
            ),
            Err(ConfigurationError::Runtime(StreamlineError::Unsupported))
        );
    }

    #[test]
    fn unknown_dynamic_flag_and_invalid_target_are_rejected() {
        let mut state = FrameGenerationState {
            maximum_generated_frames: 3,
            dynamic_supported: 2,
            ..FrameGenerationState::default()
        };
        let request = FrameGenerationMode::Dynamic { target_fps: None };
        assert_eq!(
            resolve(request, &state, true, true),
            Err(FrameGenerationError::DynamicUnavailable)
        );
        state.dynamic_supported = 1;
        for fps in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                resolve(
                    FrameGenerationMode::Dynamic {
                        target_fps: Some(fps)
                    },
                    &state,
                    true,
                    true
                ),
                Err(FrameGenerationError::InvalidTargetRate)
            );
        }
    }
}
