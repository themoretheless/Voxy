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

/// Outcome of comparing baseline and candidate p99 frame times.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenchmarkRegressionVerdict {
    pub baseline_p99_ms: f64,
    pub candidate_p99_ms: f64,
    pub relative_delta: f64,
    pub regressed: bool,
}

fn percentile_99_ms(samples_seconds: &[f64]) -> Option<f64> {
    let mut valid: Vec<f64> = samples_seconds
        .iter()
        .copied()
        .filter(|s| s.is_finite() && *s > 0.0)
        .map(|s| s * 1000.0)
        .collect();
    if valid.is_empty() {
        return None;
    }
    valid.sort_by(|a, b| a.total_cmp(b));
    let idx = ((valid.len() - 1) * 99) / 100;
    Some(valid[idx])
}

/// Compares the 99th-percentile frame durations of `baseline_seconds` and `candidate_seconds`,
/// flagging a regression when the relative increase exceeds `max_regression_fraction`.
#[allow(dead_code)]
pub fn compare_cycle_p99_ms(
    baseline_seconds: &[f64],
    candidate_seconds: &[f64],
    max_regression_fraction: f64,
) -> Option<BenchmarkRegressionVerdict> {
    let base_p99 = percentile_99_ms(baseline_seconds)?;
    let cand_p99 = percentile_99_ms(candidate_seconds)?;
    let tol = if max_regression_fraction.is_finite() {
        max_regression_fraction.max(0.0)
    } else {
        0.05
    };
    let relative_delta = (cand_p99 - base_p99) / base_p99;
    Some(BenchmarkRegressionVerdict {
        baseline_p99_ms: base_p99,
        candidate_p99_ms: cand_p99,
        relative_delta,
        regressed: relative_delta > tol,
    })
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

    #[test]
    fn compare_cycle_p99_detects_frame_time_regression() {
        let base = vec![0.006; 100];
        let ok_cand = vec![0.0062; 100];
        let bad_cand = vec![0.0075; 100];
        let ok = compare_cycle_p99_ms(&base, &ok_cand, 0.05).unwrap();
        assert!(!ok.regressed);
        let bad = compare_cycle_p99_ms(&base, &bad_cand, 0.05).unwrap();
        assert!(bad.regressed);

        assert!((frame_pacing_consistency_index(&base) - 1.0).abs() < 1e-9);
        let jittery = [0.004, 0.016, 0.004, 0.016, 0.004, 0.016];
        assert!(frame_pacing_consistency_index(&jittery) < 0.7);
    }
}

/// Computes a normalized frame-pacing consistency index in `[0.0, 1.0]` (`1.0` = zero frame-to-frame
/// judder) from consecutive frame-time deltas `1 / (1 + rms_consecutive_delta / mean_frame_time)`.
#[allow(dead_code)]
pub fn frame_pacing_consistency_index(frame_times_s: &[f64]) -> f64 {
    let valid: Vec<f64> = frame_times_s
        .iter()
        .copied()
        .filter(|&t| t.is_finite() && t > 0.0)
        .collect();
    if valid.len() < 2 {
        return 0.0;
    }
    let mean = valid.iter().sum::<f64>() / (valid.len() as f64);
    if mean <= 1e-12 {
        return 0.0;
    }
    let sum_sq_diff: f64 = valid
        .windows(2)
        .map(|w| (w[1] - w[0]).powi(2))
        .sum();
    let rms_delta = (sum_sq_diff / ((valid.len() - 1) as f64)).sqrt();
    (1.0 / (1.0 + rms_delta / mean)).clamp(0.0, 1.0)
}
