//! Qualification of rendered 120-Hz cycles, separate from the 30-step stress replay.
pub const PHYSICS_DT: f64 = 1. / 120.;
pub const MOTION_CYCLE_STEPS: u64 = 2400;

#[derive(Debug, PartialEq)]
pub struct RenderedCycle {
    pub every_physics_step_rendered: bool,
    pub complete_cycles: u64,
    pub minimum_120_completed_fps: bool,
}

pub fn audit(
    offscreen: bool,
    every_step_requested: bool,
    diagnostic_capture: bool,
    physics_steps: u64,
    completed_frame_seconds: &[f64],
) -> RenderedCycle {
    let frames = completed_frame_seconds.len() as u64;
    let every_physics_step_rendered = offscreen && every_step_requested && physics_steps == frames;
    let complete_cycles = if every_physics_step_rendered {
        frames / MOTION_CYCLE_STEPS
    } else {
        0
    };
    let minimum_120_completed_fps = every_physics_step_rendered
        && !diagnostic_capture
        && complete_cycles > 0
        && completed_frame_seconds
            .iter()
            .all(|&seconds| seconds.is_finite() && seconds > 0. && seconds <= PHYSICS_DT);
    RenderedCycle {
        every_physics_step_rendered,
        complete_cycles,
        minimum_120_completed_fps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_physics_cycle_with_only_eighty_rendered_frames_cannot_pass() {
        let samples = vec![PHYSICS_DT; 80];
        assert_eq!(audit(true, false, false, 2400, &samples).complete_cycles, 0);
        assert!(!audit(true, true, false, 2400, &samples).minimum_120_completed_fps);
    }
    #[test]
    fn requires_a_complete_rendered_cycle_and_every_completed_frame_within_budget() {
        let mut samples = vec![PHYSICS_DT; 2400];
        let passed = audit(true, true, false, 2400, &samples);
        assert_eq!(passed.complete_cycles, 1);
        assert!(passed.minimum_120_completed_fps);
        assert!(!audit(true, true, false, 2399, &samples[..2399]).minimum_120_completed_fps);
        assert!(!audit(true, true, false, 2399, &samples).minimum_120_completed_fps);
        assert!(!audit(true, true, true, 2400, &samples).minimum_120_completed_fps);
        assert!(!audit(false, true, false, 2400, &samples).minimum_120_completed_fps);
        for invalid in [PHYSICS_DT + 1e-9, f64::NAN, f64::INFINITY, -1., 0.] {
            samples[123] = invalid;
            assert!(!audit(true, true, false, 2400, &samples).minimum_120_completed_fps);
        }
    }
}
