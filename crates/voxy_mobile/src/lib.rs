//! Native mobile ABI boundary; the engine and shared application forbid unsafe.

#[cfg(any(target_os = "android", target_os = "ios"))]
fn run(event_loop: winit::event_loop::EventLoop<()>) {
    let result = voxy_shell::SceneApp::new(false).and_then(|app| app.run(event_loop));
    if let Err(error) = result {
        eprintln!("Voxy mobile error: {error}");
    }
}

/// `NativeActivity` glue entrypoint. Owns the Android event-loop thread.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "Rust" fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    match winit::event_loop::EventLoop::builder()
        .with_android_app(app)
        .build()
    {
        Ok(event_loop) => run(event_loop),
        Err(error) => eprintln!("Voxy Android event-loop error: {error}"),
    }
}

/// Called exactly once by the iOS executable's main function, on the main thread.
#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn voxy_ios_main() {
    match winit::event_loop::EventLoop::new() {
        Ok(event_loop) => run(event_loop),
        Err(error) => eprintln!("Voxy iOS event-loop error: {error}"),
    }
}

/// Platform thermal state reported by iOS (`NSProcessInfoThermalState`) or Android (`ADPF`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MobileThermalState {
    Nominal,
    Fair,
    Serious,
    Critical,
}

impl MobileThermalState {
    /// Returns `(render_resolution_scale, target_fps)` for adaptive mobile power management.
    #[must_use]
    pub const fn quality_budget(self) -> (f32, u32) {
        match self {
            Self::Nominal => (1.0, 60),
            Self::Fair => (0.85, 60),
            Self::Serious => (0.70, 45),
            Self::Critical => (0.50, 30),
        }
    }

    /// Transitions mobile thermal state from `skin_temp_c` (°C) using deadband `hysteresis_c`
    /// around thresholds (`38 °C`, `42 °C`, `46 °C`) to prevent quality-budget oscillation.
    #[must_use]
    pub fn step_with_hysteresis(self, skin_temp_c: f32, hysteresis_c: f32) -> Self {
        if !skin_temp_c.is_finite() {
            return self;
        }
        let h = if hysteresis_c.is_finite() && hysteresis_c >= 0.0 {
            hysteresis_c
        } else {
            1.5
        };
        match self {
            Self::Nominal => {
                if skin_temp_c >= 46.0 {
                    Self::Critical
                } else if skin_temp_c >= 42.0 {
                    Self::Serious
                } else if skin_temp_c >= 38.0 {
                    Self::Fair
                } else {
                    Self::Nominal
                }
            }
            Self::Fair => {
                if skin_temp_c >= 46.0 {
                    Self::Critical
                } else if skin_temp_c >= 42.0 {
                    Self::Serious
                } else if skin_temp_c <= 38.0 - h {
                    Self::Nominal
                } else {
                    Self::Fair
                }
            }
            Self::Serious => {
                if skin_temp_c >= 46.0 {
                    Self::Critical
                } else if skin_temp_c <= 38.0 - h {
                    Self::Nominal
                } else if skin_temp_c <= 42.0 - h {
                    Self::Fair
                } else {
                    Self::Serious
                }
            }
            Self::Critical => {
                if skin_temp_c <= 38.0 - h {
                    Self::Nominal
                } else if skin_temp_c <= 42.0 - h {
                    Self::Fair
                } else if skin_temp_c <= 46.0 - h {
                    Self::Serious
                } else {
                    Self::Critical
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_thermal_budget_scales_resolution_and_framerate_monotonically() {
        let (s0, fps0) = MobileThermalState::Nominal.quality_budget();
        let (s1, fps1) = MobileThermalState::Fair.quality_budget();
        let (s2, fps2) = MobileThermalState::Serious.quality_budget();
        let (s3, fps3) = MobileThermalState::Critical.quality_budget();
        assert!(s0 > s1 && s1 > s2 && s2 > s3);
        assert!(fps0 >= fps1 && fps1 > fps2 && fps2 > fps3);

        let state = MobileThermalState::Nominal.step_with_hysteresis(38.5, 1.5);
        assert_eq!(state, MobileThermalState::Fair);
        assert_eq!(state.step_with_hysteresis(37.2, 1.5), MobileThermalState::Fair);
        assert_eq!(
            state.step_with_hysteresis(36.0, 1.5),
            MobileThermalState::Nominal
        );
    }
}
