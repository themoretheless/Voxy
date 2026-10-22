//! Real-time driven time scaling and bounded fixed-step scheduling.

/// Fixed-step work for one rendered frame. `alpha` interpolates the last two states.
#[derive(Clone, Copy, Debug)]
pub struct TimeFrame {
    pub steps: usize,
    pub alpha: f64,
    pub overloaded: bool,
}

/// Explicit discarded time for the most recent scheduling call.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeDrop {
    pub real_seconds: f64,
    pub simulation_seconds: f64,
}
/// Smooth braking and an underdamped spring when increasing simulation speed.
/// Call `advance` using real seconds, including while the simulation is paused.
#[derive(Clone, Debug)]
pub struct SimulationClock {
    scale: f64,
    target: f64,
    velocity: f64,
    accumulator: f64,
    damping: f64,
    last_drop: TimeDrop,
}

impl Default for SimulationClock {
    fn default() -> Self {
        Self {
            scale: 1.0,
            target: 1.0,
            velocity: 0.0,
            accumulator: 0.0,
            damping: 1.0,
            last_drop: TimeDrop::default(),
        }
    }
}

impl SimulationClock {
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.scale
    }
    #[must_use]
    pub fn target_scale(&self) -> f64 {
        self.target
    }

    #[must_use]
    pub fn last_drop(&self) -> TimeDrop {
        self.last_drop
    }

    /// Supported targets are 0..=4. Invalid values are rejected without mutation.
    pub fn set_target(&mut self, scale: f64) -> bool {
        if !scale.is_finite() || !(0.0..=4.0).contains(&scale) {
            return false;
        }
        self.damping = if scale > self.scale { 0.55 } else { 1.0 };
        self.target = scale;
        true
    }

    /// Immediate freeze for focus loss; normal user pauses use `set_target(0.0)`.
    pub fn freeze(&mut self) {
        self.scale = 0.0;
        self.target = 0.0;
        self.velocity = 0.0;
    }

    /// Caps real elapsed time at 100 ms and drops excess whole steps on overload.
    /// `last_drop` reports both discarded real time and scaled simulation backlog.
    /// # Panics
    /// Panics for invalid steps or a zero work budget.
    /// Transition integration uses <=1 ms real-time slices, independently of physics.
    pub fn advance(&mut self, real_dt: f64, step: f64, max_steps: usize) -> TimeFrame {
        assert!(step.is_finite() && step > 0.0 && max_steps > 0);
        self.last_drop = TimeDrop::default();
        if real_dt.is_finite() {
            self.last_drop.real_seconds = (real_dt - 0.1).max(0.0);
        }
        let mut remaining = if real_dt.is_finite() {
            real_dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        while remaining > 0.0 {
            let dt = remaining.min(0.001);
            let old = self.scale;
            let omega = 12.0;
            self.velocity += (omega * omega * (self.target - self.scale)
                - 2.0 * self.damping * omega * self.velocity)
                * dt;
            self.scale = (self.scale + self.velocity * dt).max(0.0);
            if self.scale == 0.0 && self.velocity < 0.0 {
                self.velocity = 0.0;
            }
            if (self.scale - self.target).abs() < 1e-5 && self.velocity.abs() < 1e-4 {
                self.scale = self.target;
                self.velocity = 0.0;
            }
            self.accumulator += (old + self.scale) * 0.5 * dt;
            remaining -= dt;
        }
        let mut steps = 0;
        while self.accumulator + step * 1e-10 >= step && steps < max_steps {
            self.accumulator = (self.accumulator - step).max(0.0);
            steps += 1;
        }
        let overloaded = self.accumulator >= step;
        if overloaded {
            let remainder = self.accumulator % step;
            self.last_drop.simulation_seconds = self.accumulator - remainder;
            self.accumulator = remainder;
        }
        TimeFrame {
            steps,
            alpha: self.accumulator / step,
            overloaded,
        }
    }
}

/// Zero-allocation sliding window of frame durations (in microseconds) providing
/// exact median/percentile latency telemetry and micro-stutter detection.
#[derive(Clone, Debug)]
pub struct FrameLatencyWindow<const N: usize = 128> {
    samples_us: [u32; N],
    cursor: usize,
    filled: usize,
    stutters: u64,
}

impl<const N: usize> Default for FrameLatencyWindow<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> FrameLatencyWindow<N> {
    #[must_use]
    pub const fn new() -> Self {
        const { assert!(N > 0, "FrameLatencyWindow capacity must be non-zero") };
        Self {
            samples_us: [0_u32; N],
            cursor: 0,
            filled: 0,
            stutters: 0,
        }
    }

    /// Records a frame duration in seconds and returns `true` if it exceeded $2\times$ the current rolling mean.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn record_seconds(&mut self, dt_seconds: f64) -> bool {
        if !dt_seconds.is_finite() || dt_seconds < 0.0 {
            return false;
        }
        let us = (dt_seconds * 1_000_000.0).round().min(f64::from(u32::MAX)) as u32;
        self.record_micros(us)
    }

    /// Records a frame duration in microseconds and returns `true` if it was a stutter ($> 2\times$ mean).
    pub fn record_micros(&mut self, duration_us: u32) -> bool {
        let mean = self.mean_micros();
        let is_stutter =
            self.filled >= 8 && mean > 0 && u64::from(duration_us) > 2 * u64::from(mean);
        if is_stutter {
            self.stutters = self.stutters.saturating_add(1);
        }
        self.samples_us[self.cursor] = duration_us;
        self.cursor = (self.cursor + 1) % N;
        if self.filled < N {
            self.filled += 1;
        }
        is_stutter
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.filled
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.filled == 0
    }

    #[must_use]
    pub const fn stutter_count(&self) -> u64 {
        self.stutters
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn mean_micros(&self) -> u32 {
        if self.filled == 0 {
            return 0;
        }
        let sum: u64 = self.samples_us[..self.filled]
            .iter()
            .map(|&v| u64::from(v))
            .sum();
        (sum / (self.filled as u64)) as u32
    }

    /// Returns `(p50_us, p95_us, p99_us)` over the active window without heap allocations.
    #[must_use]
    pub fn percentiles_micros(&self) -> (u32, u32, u32) {
        if self.filled == 0 {
            return (0, 0, 0);
        }
        let mut sorted = self.samples_us;
        let active = &mut sorted[..self.filled];
        active.sort_unstable();
        let last = self.filled - 1;
        let p50 = active[last / 2];
        let p95 = active[(last * 95) / 100];
        let p99 = active[(last * 99) / 100];
        (p50, p95, p99)
    }
}

/// Token-bucket budget limiter for background chunk meshing and streaming per frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameWorkBudget {
    budget_us: u64,
    consumed_us: u64,
}

impl FrameWorkBudget {
    #[must_use]
    pub const fn from_micros(budget_us: u64) -> Self {
        Self {
            budget_us,
            consumed_us: 0,
        }
    }

    pub const fn reset(&mut self) {
        self.consumed_us = 0;
    }

    #[must_use]
    pub const fn remaining_micros(self) -> u64 {
        self.budget_us.saturating_sub(self.consumed_us)
    }

    /// Attempts to spend `cost_us`, returning `true` if within budget.
    pub const fn try_consume(&mut self, cost_us: u64) -> bool {
        let next = self.consumed_us.saturating_add(cost_us);
        if next <= self.budget_us {
            self.consumed_us = next;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    #[test]
    fn braking_pause_and_real_time_resume() {
        let mut clock = SimulationClock::default();
        clock.set_target(0.0);
        let mut previous = 1.0;
        for _ in 0..300 {
            clock.advance(0.01, 1.0 / 240.0, 128);
            assert!(clock.scale() <= previous);
            previous = clock.scale();
        }
        assert_eq!(clock.scale(), 0.0);
        assert_eq!(clock.advance(0.1, 1.0 / 240.0, 128).steps, 0);
        clock.set_target(1.0);
        let mut peak: f64 = 0.0;
        for _ in 0..300 {
            clock.advance(0.01, 1.0 / 240.0, 128);
            peak = peak.max(clock.scale());
        }
        assert!(peak > 1.05 && peak < 1.2, "peak {peak}");
        assert_eq!(clock.scale(), 1.0);
    }
    #[test]
    fn retarget_is_continuous_and_invalid_inputs_are_ignored() {
        let mut clock = SimulationClock::default();
        clock.set_target(4.0);
        clock.advance(0.1, 0.01, 128);
        let before = clock.scale();
        clock.set_target(0.1);
        assert_eq!(clock.scale(), before);
        assert!(!clock.set_target(f64::NAN));
        assert!(!clock.set_target(-1.0));
        clock.advance(0.001, 0.01, 128);
        assert!((clock.scale() - before).abs() < 0.05);
        assert_eq!(clock.advance(f64::NAN, 0.01, 128).steps, 0);
    }
    #[test]
    fn zero_elapsed_cannot_emit_subpicosecond_ticks() {
        let mut clock = SimulationClock::default();
        assert_eq!(clock.advance(0.0, 1e-15, 8).steps, 0);
        assert_eq!(clock.advance(1e-16, 1e-15, 8).steps, 0);
        assert_eq!(clock.advance(9e-16, 1e-15, 8).steps, 1);
    }
    #[test]
    fn fixed_steps_and_budget() {
        let mut clock = SimulationClock::default();
        let mut ticks = 0;
        for _ in 0..100 {
            ticks += clock.advance(0.01, 1.0 / 240.0, 128).steps;
        }
        assert_eq!(ticks, 240);
        let frame = clock.advance(20.0, 0.001, 8);
        assert_eq!(frame.steps, 8);
        assert!(frame.overloaded);
        assert!((0.0..1.0).contains(&frame.alpha));
    }

    #[test]
    fn latency_window_tracks_percentiles_and_stutters() {
        let mut window = FrameLatencyWindow::<16>::new();
        for _ in 0..12 {
            assert!(!window.record_micros(8_000));
        }
        assert_eq!(window.mean_micros(), 8_000);
        assert!(window.record_micros(25_000));
        assert_eq!(window.stutter_count(), 1);
        let (p50, p95, p99) = window.percentiles_micros();
        assert_eq!(p50, 8_000);
        assert!(p95 >= 8_000 && p99 == 25_000);

        let mut budget = FrameWorkBudget::from_micros(1_500);
        assert!(budget.try_consume(900));
        assert_eq!(budget.remaining_micros(), 600);
        assert!(!budget.try_consume(700));
        assert!(budget.try_consume(600));
        assert_eq!(budget.remaining_micros(), 0);
    }
}
