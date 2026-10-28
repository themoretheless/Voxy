//! Native event-loop cadence is independent of the fixed simulation step.
use std::{
    str::FromStr,
    time::{Duration, Instant},
};
use winit::event_loop::ControlFlow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Presentation {
    #[default]
    Vsync,
    Unthrottled,
}
impl FromStr for Presentation {
    type Err = crate::EditorError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "vsync" => Ok(Self::Vsync),
            "unthrottled" => Ok(Self::Unthrottled),
            _ => Err("VOXY_EDITOR_PRESENTATION must be vsync or unthrottled".into()),
        }
    }
}
impl Presentation {
    pub(crate) fn from_environment() -> Result<Self, crate::EditorError> {
        match std::env::var("VOXY_EDITOR_PRESENTATION") {
            Ok(value) => value.parse(),
            Err(std::env::VarError::NotPresent) => Ok(Self::default()),
            Err(error) => Err(error.to_string().into()),
        }
    }
    pub(crate) fn apply(
        self,
        host: &mut voxy_render::SceneSurface,
    ) -> Result<(), crate::EditorError> {
        if self == Self::Unthrottled {
            let mode = host
                .enable_unthrottled_presentation()
                .map_err(|error| error.to_string())?;
            println!("EDITOR PRESENTATION requested=unthrottled selected={mode:?}");
        }
        Ok(())
    }
}

/// Keep the existing background polling interval for hidden/minimized windows.
/// Active redraws use GPU presentation for pacing, without a 16 ms CPU timer.
pub(crate) fn control_flow(drawable: bool, now: Instant) -> ControlFlow {
    if drawable {
        ControlFlow::Poll
    } else {
        ControlFlow::WaitUntil(now + Duration::from_millis(16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_configuration_is_explicit_and_rejects_unknown_values() {
        assert_eq!(Presentation::default(), Presentation::Vsync);
        assert_eq!(
            "vsync".parse::<Presentation>().unwrap(),
            Presentation::Vsync
        );
        assert_eq!(
            "unthrottled".parse::<Presentation>().unwrap(),
            Presentation::Unthrottled
        );
        for invalid in ["", "auto", "120", "unlimited", "VSYNC"] {
            assert!(invalid.parse::<Presentation>().is_err());
        }
    }
}
