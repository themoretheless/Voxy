//! Fixed-rate CFL-safe integration. Unfinished tick credit is retained, never clamped away.
const BASE_HZ: u32 = 240;
const MAX_TICKS: u32 = 4096;
#[derive(Debug)]
pub(super) struct RippleClock {
    pub rate: u32,
    credit: f64,
    pub encoded_ticks: u64,
    pub requested_seconds: f64,
}
impl RippleClock {
    pub fn new(cell: f32, speed: f32) -> Option<Self> {
        if !cell.is_finite() || cell <= 0. || !speed.is_finite() || speed < 0. {
            return None;
        }
        let rate = (2. * f64::from(speed) / f64::from(cell))
            .ceil()
            .max(f64::from(BASE_HZ));
        if rate > f64::from(u32::MAX) {
            return None;
        }
        Some(Self {
            rate: rate as u32,
            credit: 0.,
            encoded_ticks: 0,
            requested_seconds: 0.,
        })
    }
    pub fn plan(&mut self, dt: f64) -> Option<u32> {
        if !dt.is_finite() || dt < 0. {
            return None;
        }
        let credit = self.credit + dt * f64::from(self.rate);
        if !credit.is_finite() || credit > 9_007_199_254_740_990. {
            return None;
        }
        // The GPU copies an odd final result back without taking an extra tick.
        let steps = credit.floor().min(f64::from(MAX_TICKS)) as u32;
        let encoded = self.encoded_ticks.checked_add(u64::from(steps))?;
        self.credit = credit - f64::from(steps);
        self.encoded_ticks = encoded;
        self.requested_seconds += dt;
        Some(steps)
    }
    pub fn pending_seconds(&self) -> f64 {
        self.credit / f64::from(self.rate)
    }
    pub fn encoded_seconds(&self) -> f64 {
        self.encoded_ticks as f64 / f64::from(self.rate)
    }
    pub fn step_seconds(&self) -> f32 {
        1. / self.rate as f32
    }
    pub fn damping(&self, base: f32) -> f32 {
        f64::from(base).powf(f64::from(BASE_HZ) / f64::from(self.rate)) as f32
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ripple_quarter_second_and_full_cycle_do_not_drop_time() {
        let mut c = RippleClock::new(24. / 256., 0.6).unwrap();
        for _ in 0..80 {
            assert_eq!(c.plan(0.25), Some(60));
        }
        assert_eq!(c.encoded_ticks, 4800);
        assert_eq!(c.encoded_seconds(), 20.);
        assert_eq!(c.pending_seconds(), 0.);
    }
    #[test]
    fn ripple_fractional_credit_is_preserved() {
        let mut c = RippleClock::new(0.1, 0.6).unwrap();
        for _ in 0..1000 {
            c.plan(1. / 2400.).unwrap();
        }
        assert!((c.encoded_seconds() + c.pending_seconds() - c.requested_seconds).abs() < 1e-13);
        assert!(c.pending_seconds() >= 0.);
        assert!(c.pending_seconds() < 1. / 240.);
    }
    #[test]
    fn ripple_budget_retains_backlog() {
        let mut c = RippleClock::new(0.1, 0.6).unwrap();
        assert_eq!(c.plan(60.), Some(MAX_TICKS));
        assert!(c.pending_seconds() > 0.);
        while c.pending_seconds() > 0. {
            c.plan(0.).unwrap();
        }
        assert_eq!(c.encoded_ticks, 14400);
        assert_eq!(c.encoded_seconds(), 60.);
    }
    #[test]
    fn ripple_cfl_and_invalid_time() {
        let mut c = RippleClock::new(0.001, 10.).unwrap();
        assert_eq!(c.rate, 20000);
        assert!((10. * f64::from(c.step_seconds()) / 0.001) <= 0.500001);
        assert_eq!(c.plan(0.), Some(0));
        for t in [f64::NAN, f64::INFINITY, -1., f64::MAX] {
            assert!(c.plan(t).is_none());
        }
        assert_eq!(c.encoded_ticks, 0);
        assert_eq!(c.requested_seconds, 0.);
        assert!(RippleClock::new(0., 1.).is_none());
    }
}
