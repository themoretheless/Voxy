//! Real-time driven time scaling and bounded fixed-step scheduling.

/// Fixed-step work for one rendered frame. `alpha` interpolates the last two states.
#[derive(Clone, Copy, Debug)]
pub struct TimeFrame {
    pub steps: usize,
    pub alpha: f64,
    pub overloaded: bool,
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
}

impl Default for SimulationClock {
    fn default() -> Self {
        Self {
            scale: 1.0,
            target: 1.0,
            velocity: 0.0,
            accumulator: 0.0,
            damping: 1.0,
        }
    }
}

impl SimulationClock {
    pub fn scale(&self) -> f64 {
        self.scale
    }
    pub fn target_scale(&self) -> f64 {
        self.target
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
    /// Transition integration uses <=1 ms real-time slices, independently of physics.
    pub fn advance(&mut self, real_dt: f64, step: f64, max_steps: usize) -> TimeFrame {
        assert!(step.is_finite() && step > 0.0 && max_steps > 0);
        let mut remaining = if real_dt.is_finite() {
            real_dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        while remaining > 1e-12 {
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
        while self.accumulator + 1e-12 >= step && steps < max_steps {
            self.accumulator = (self.accumulator - step).max(0.0);
            steps += 1;
        }
        let overloaded = self.accumulator >= step;
        if overloaded {
            self.accumulator %= step;
        }
        TimeFrame {
            steps,
            alpha: self.accumulator / step,
            overloaded,
        }
    }
}

#[cfg(test)]
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
}
