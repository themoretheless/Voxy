//! Frame-generation requests validated against live SDK capability reports.
//! This module does not load Streamline or generate/present interpolated frames.

/// User-selected frame-generation policy. Counts exclude the rendered frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FrameGenerationMode {
    #[default]
    Off,
    Fixed {
        generated_frames: u32,
    },
    /// `None` lets the SDK select the display refresh rate.
    Dynamic {
        target_fps: Option<f32>,
    },
}

/// Report from an initialized integration, not an adapter-name heuristic.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameGenerationCapabilities {
    pub available: bool,
    pub max_generated_frames: u32,
    pub dynamic_available: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameGenerationError {
    Unavailable,
    ReflexRequired,
    InvalidFrameCount,
    DynamicUnavailable,
    InvalidTargetRate,
}

impl FrameGenerationMode {
    /// Validate a request before passing it to the native integration.
    ///
    /// # Errors
    /// Rejects unavailable features, absent Reflex and invalid SDK limits/rates.
    pub fn validate(
        self,
        capabilities: FrameGenerationCapabilities,
        reflex_active: bool,
    ) -> Result<(), FrameGenerationError> {
        if self == Self::Off {
            return Ok(());
        }
        if !capabilities.available || capabilities.max_generated_frames == 0 {
            return Err(FrameGenerationError::Unavailable);
        }
        if !reflex_active {
            return Err(FrameGenerationError::ReflexRequired);
        }
        match self {
            Self::Off => Ok(()),
            Self::Fixed { generated_frames } => {
                if generated_frames == 0 || generated_frames > capabilities.max_generated_frames {
                    return Err(FrameGenerationError::InvalidFrameCount);
                }
                Ok(())
            }
            Self::Dynamic { target_fps } => {
                if !capabilities.dynamic_available {
                    return Err(FrameGenerationError::DynamicUnavailable);
                }
                if target_fps.is_some_and(|fps| !fps.is_finite() || fps <= 0.0) {
                    return Err(FrameGenerationError::InvalidTargetRate);
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_limits_and_reflex_gate_fixed_generation() {
        let caps = FrameGenerationCapabilities {
            available: true,
            max_generated_frames: 5,
            dynamic_available: false,
        };
        assert_eq!(
            FrameGenerationMode::Off.validate(FrameGenerationCapabilities::default(), false),
            Ok(())
        );
        let request = FrameGenerationMode::Fixed {
            generated_frames: 5,
        };
        assert_eq!(request.validate(caps, true), Ok(()));
        assert_eq!(
            request.validate(caps, false),
            Err(FrameGenerationError::ReflexRequired)
        );
        assert_eq!(
            request.validate(FrameGenerationCapabilities::default(), true),
            Err(FrameGenerationError::Unavailable)
        );
        for generated_frames in [0, 6, u32::MAX] {
            assert_eq!(
                FrameGenerationMode::Fixed { generated_frames }.validate(caps, true),
                Err(FrameGenerationError::InvalidFrameCount)
            );
        }
    }

    #[test]
    fn dynamic_requires_reported_support_and_finite_positive_target() {
        let mut caps = FrameGenerationCapabilities {
            available: true,
            max_generated_frames: 5,
            dynamic_available: false,
        };
        let automatic = FrameGenerationMode::Dynamic { target_fps: None };
        assert_eq!(
            automatic.validate(caps, true),
            Err(FrameGenerationError::DynamicUnavailable)
        );
        caps.dynamic_available = true;
        assert_eq!(automatic.validate(caps, true), Ok(()));
        assert_eq!(
            FrameGenerationMode::Dynamic {
                target_fps: Some(144.0)
            }
            .validate(caps, true),
            Ok(())
        );
        for fps in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                FrameGenerationMode::Dynamic {
                    target_fps: Some(fps)
                }
                .validate(caps, true),
                Err(FrameGenerationError::InvalidTargetRate)
            );
        }
    }
}
