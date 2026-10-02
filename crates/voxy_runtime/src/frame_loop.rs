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
}
