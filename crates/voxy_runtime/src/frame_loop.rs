//! Bounded frame scheduling shared by scene shells and headless applications.
use crate::{SimulationClock, TimeDrop, TimeFrame};

#[derive(Clone, Copy, Debug)]
pub struct FrameWork {
    /// Clamped frame-update delta. Zero while paused.
    pub delta_seconds: f64,
    pub fixed: TimeFrame,
    pub dropped: TimeDrop,
}

/// Immediate pause policy over the shared simulation clock. Platform lifecycle
/// boundaries discard partial ticks; resume never catches up suspended time.
#[derive(Debug)]
pub struct FrameLoop {
    clock: SimulationClock,
    step: f64,
    budget: usize,
    paused: bool,
}
impl Default for FrameLoop {
    fn default() -> Self {
        Self::new(1.0 / 60.0, 6).expect("valid default frame budget")
    }
}
impl FrameLoop {
    /// Returns `None` for invalid fixed-step duration or zero work budget.
    #[must_use]
    pub fn new(step_seconds: f64, max_steps: usize) -> Option<Self> {
        (step_seconds.is_finite() && step_seconds > 0.0 && max_steps > 0).then(|| Self {
            clock: SimulationClock::default(),
            step: step_seconds,
            budget: max_steps,
            paused: false,
        })
    }
    #[must_use]
    pub const fn fixed_seconds(&self) -> f64 {
        self.step
    }
    /// Repeated pause events are idempotent. Changing state clears partial ticks.
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.paused = paused;
            self.reset();
        }
    }
    /// Call on suspension, resume or a platform discontinuity. The caller must
    /// also reset its wall-clock timestamp and invalidate presentation history.
    pub fn reset(&mut self) {
        self.clock = SimulationClock::default();
    }
    pub fn advance(&mut self, elapsed_seconds: f64) -> FrameWork {
        let delta = if elapsed_seconds.is_finite() {
            elapsed_seconds.clamp(0.0, 0.1)
        } else {
            0.0
        };
        if self.paused {
            return FrameWork {
                delta_seconds: 0.0,
                fixed: TimeFrame {
                    steps: 0,
                    alpha: 0.0,
                    overloaded: false,
                },
                dropped: TimeDrop::default(),
            };
        }
        FrameWork {
            delta_seconds: delta,
            fixed: self.clock.advance(elapsed_seconds, self.step, self.budget),
            dropped: self.clock.last_drop(),
        }
    }
}

/// Snaps a measured wall-clock frame delta to the nearest standard display refresh interval
/// (`60`, `72`, `90`, `120`, `144`, `165`, `240` Hz) when within `tolerance_ratio` (e.g. `0.03`).
/// Eliminates VSync micro-stutter on fixed-step accumulators while preserving large deviations.
#[must_use]
pub fn snap_frame_delta_to_refresh_rate(elapsed_seconds: f64, tolerance_ratio: f64) -> f64 {
    if !elapsed_seconds.is_finite()
        || elapsed_seconds <= 0.0
        || !tolerance_ratio.is_finite()
        || !(0.0..=0.25).contains(&tolerance_ratio)
    {
        return elapsed_seconds;
    }
    const REFRESH_RATES_HZ: [f64; 7] = [60.0, 72.0, 90.0, 120.0, 144.0, 165.0, 240.0];
    let mut best_target = elapsed_seconds;
    let mut best_rel_err = tolerance_ratio;
    for rate in REFRESH_RATES_HZ {
        let period = 1.0 / rate;
        let rel_err = ((elapsed_seconds - period) / period).abs();
        if rel_err <= best_rel_err {
            best_rel_err = rel_err;
            best_target = period;
        }
    }
    best_target
}

/// Anti-windup PID controller adjusting dynamic resolution / quality scale in `[min_scale, max_scale]`
/// to hold `target_frame_ms` (e.g. `8.33` ms or `16.67` ms) without limit-cycle oscillation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveQualityPidController {
    pub target_frame_ms: f32,
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    integral: f32,
    prev_error: f32,
    current_scale: f32,
}

impl AdaptiveQualityPidController {
    #[must_use]
    pub fn new(target_frame_ms: f32, min_scale: f32, max_scale: f32) -> Option<Self> {
        if !target_frame_ms.is_finite()
            || target_frame_ms <= 0.5
            || !min_scale.is_finite()
            || !max_scale.is_finite()
            || min_scale <= 0.1
            || max_scale < min_scale
        {
            return None;
        }
        Some(Self {
            target_frame_ms,
            kp: 0.015,
            ki: 0.004,
            kd: 0.005,
            min_scale,
            max_scale,
            integral: 0.0,
            prev_error: 0.0,
            current_scale: 1.0_f32.clamp(min_scale, max_scale),
        })
    }

    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.current_scale
    }

    /// Updates the quality scale from a measured frame duration `measured_frame_ms`.
    pub fn step(&mut self, measured_frame_ms: f32) -> f32 {
        if !measured_frame_ms.is_finite() || measured_frame_ms <= 0.0 {
            return self.current_scale;
        }
        // Positive error when we have headroom (measured < target -> increase quality).
        let error = self.target_frame_ms - measured_frame_ms;
        if error.abs() < 0.15 {
            self.prev_error = error;
            return self.current_scale;
        }
        self.integral = (self.integral + error).clamp(-20.0, 20.0);
        let derivative = error - self.prev_error;
        self.prev_error = error;
        let delta = self.kp * error + self.ki * self.integral + self.kd * derivative;
        self.current_scale =
            (self.current_scale + delta.clamp(-0.1, 0.1)).clamp(self.min_scale, self.max_scale);
        self.current_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stall_is_bounded_and_backlog_is_reported() {
        let mut scheduler = FrameLoop::new(0.01, 3).unwrap();
        let frame = scheduler.advance(10.0);
        assert_eq!(frame.fixed.steps, 3);
        assert!(frame.fixed.overloaded);
        assert!(frame.dropped.real_seconds > 9.8);
        assert!(frame.dropped.simulation_seconds > 0.06);
        assert_eq!(scheduler.advance(0.0).fixed.steps, 0);
    }
    #[test]
    fn pause_and_platform_reset_discard_partial_ticks() {
        let mut scheduler = FrameLoop::new(0.02, 8).unwrap();
        assert_eq!(scheduler.advance(0.015).fixed.steps, 0);
        scheduler.set_paused(true);
        assert_eq!(scheduler.advance(60.0).fixed.steps, 0);
        scheduler.set_paused(true);
        scheduler.set_paused(false);
        assert_eq!(scheduler.advance(0.01).fixed.steps, 0);
        scheduler.reset();
        assert_eq!(scheduler.advance(0.01).fixed.steps, 0);
        assert_eq!(scheduler.advance(0.01).fixed.steps, 1);
    }
    #[test]
    fn repeated_running_state_preserves_partial_tick() {
        let mut scheduler = FrameLoop::new(0.02, 8).unwrap();
        assert_eq!(scheduler.advance(0.01).fixed.steps, 0);
        scheduler.set_paused(false);
        assert_eq!(scheduler.advance(0.01).fixed.steps, 1);
    }
    #[test]
    fn invalid_elapsed_does_not_poison_next_frame() {
        let mut scheduler = FrameLoop::new(0.02, 8).unwrap();
        for elapsed in [f64::NAN, f64::INFINITY, -1.0] {
            assert_eq!(scheduler.advance(elapsed).fixed.steps, 0);
        }
        assert_eq!(scheduler.advance(0.02).fixed.steps, 1);
        assert!(FrameLoop::new(0.0, 1).is_none());
        assert!(FrameLoop::new(0.02, 0).is_none());
    }
    #[test]
    fn refresh_rate_snapping_locks_near_standard_periods() {
        let snapped_60 = snap_frame_delta_to_refresh_rate(0.016_75, 0.02);
        assert!((snapped_60 - 1.0 / 60.0).abs() < 1e-12);
        let snapped_144 = snap_frame_delta_to_refresh_rate(0.006_98, 0.02);
        assert!((snapped_144 - 1.0 / 144.0).abs() < 1e-12);
        let unsnapped = snap_frame_delta_to_refresh_rate(0.025, 0.02);
        assert!((unsnapped - 0.025).abs() < 1e-12);
        let mut pid = AdaptiveQualityPidController::new(16.67, 0.5, 1.0).unwrap();
        let reduced = pid.step(24.0);
        assert!(reduced < 1.0 && reduced >= 0.5);
        let recovered = pid.step(10.0);
        assert!(recovered > reduced);
    }
}
