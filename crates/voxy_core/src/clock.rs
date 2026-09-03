use std::time::{Duration, Instant};

use crate::TickId;

/// A non-mutating proposal returned by [`FixedClock::plan`].
#[derive(Clone, Copy, Debug)]
pub struct ClockPlan {
    generation: u64,
    observed_at: Instant,
    debt: Duration,
    pub due: u32,
}

impl ClockPlan {
    #[must_use]
    pub fn debt(self) -> Duration {
        self.debt
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockError {
    ZeroRate,
    TimeWentBackwards,
    StalePlan,
    ExecutedMoreThanDue,
    TickOverflow,
}

/// Fixed-step clock with an explicit plan/commit boundary.
#[derive(Debug)]
pub struct FixedClock {
    tick_period: Duration,
    observed_at: Instant,
    debt: Duration,
    generation: u64,
    next_tick: TickId,
}

impl FixedClock {
    /// Creates a clock whose first tick is zero.
    ///
    /// # Errors
    ///
    /// Returns [`ClockError::ZeroRate`] when `ticks_per_second` is zero.
    pub fn new(ticks_per_second: u32, now: Instant) -> Result<Self, ClockError> {
        if ticks_per_second == 0 {
            return Err(ClockError::ZeroRate);
        }
        Ok(Self {
            tick_period: Duration::from_secs(1) / ticks_per_second,
            observed_at: now,
            debt: Duration::ZERO,
            generation: 0,
            next_tick: TickId::new(0),
        })
    }

    /// Observes elapsed time without consuming any due ticks.
    ///
    /// # Errors
    ///
    /// Returns [`ClockError::TimeWentBackwards`] when `now` precedes the last committed plan.
    pub fn plan(&self, now: Instant) -> Result<ClockPlan, ClockError> {
        let elapsed = now
            .checked_duration_since(self.observed_at)
            .ok_or(ClockError::TimeWentBackwards)?;
        let debt = self.debt.saturating_add(elapsed);
        let due_u128 = debt.as_nanos() / self.tick_period.as_nanos();
        let due = u32::try_from(due_u128).unwrap_or(u32::MAX);
        Ok(ClockPlan {
            generation: self.generation,
            observed_at: now,
            debt,
            due,
        })
    }

    /// Commits a plan and consumes exactly `executed` due ticks.
    ///
    /// # Errors
    ///
    /// Rejects stale plans, over-commit, and counter overflow.
    pub fn commit_executed(&mut self, plan: ClockPlan, executed: u32) -> Result<(), ClockError> {
        if plan.generation != self.generation {
            return Err(ClockError::StalePlan);
        }
        if executed > plan.due {
            return Err(ClockError::ExecutedMoreThanDue);
        }
        let next = self
            .next_tick
            .get()
            .checked_add(u64::from(executed))
            .ok_or(ClockError::TickOverflow)?;
        let consumed = self.tick_period.saturating_mul(executed);
        self.debt = plan.debt.saturating_sub(consumed);
        self.observed_at = plan.observed_at;
        self.next_tick = TickId::new(next);
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(ClockError::TickOverflow)?;
        Ok(())
    }

    /// Drops whole overdue ticks above `max_debt_ticks`, returning discarded simulation time.
    pub fn client_discard_excess(&mut self, max_debt_ticks: u32) -> Duration {
        let cap = self.tick_period.saturating_mul(max_debt_ticks);
        if self.debt <= cap {
            return Duration::ZERO;
        }
        let excess = self.debt.checked_sub(cap).unwrap_or(Duration::ZERO);
        let whole_ticks = excess.as_nanos() / self.tick_period.as_nanos();
        let discarded = self
            .tick_period
            .saturating_mul(u32::try_from(whole_ticks).unwrap_or(u32::MAX));
        self.debt = self.debt.saturating_sub(discarded);
        discarded
    }

    #[must_use]
    pub const fn next_tick(&self) -> TickId {
        self.next_tick
    }

    #[must_use]
    pub fn tick_period(&self) -> Duration {
        self.tick_period
    }

    /// Returns the fractional position between the last and next simulation tick.
    ///
    /// # Errors
    ///
    /// Returns [`ClockError::TimeWentBackwards`] when `now` precedes the last committed plan.
    pub fn alpha(&self, now: Instant) -> Result<f64, ClockError> {
        let plan = self.plan(now)?;
        let remainder = plan.debt.as_secs_f64() % self.tick_period.as_secs_f64();
        Ok(remainder / self.tick_period.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planning_does_not_consume_ticks() {
        let start = Instant::now();
        let mut clock = FixedClock::new(10, start).unwrap();
        let now = start + Duration::from_millis(350);
        let first = clock.plan(now).unwrap();
        let second = clock.plan(now).unwrap();
        assert_eq!(first.due, 3);
        assert_eq!(second.due, 3);
        assert_eq!(clock.next_tick(), TickId::new(0));

        clock.commit_executed(first, 2).unwrap();
        assert_eq!(clock.next_tick(), TickId::new(2));
        assert_eq!(clock.plan(now).unwrap().due, 1);
        assert_eq!(clock.commit_executed(second, 0), Err(ClockError::StalePlan));
    }

    #[test]
    fn client_discard_is_explicit_and_tick_aligned() {
        let start = Instant::now();
        let mut clock = FixedClock::new(10, start).unwrap();
        let plan = clock.plan(start + Duration::from_millis(950)).unwrap();
        clock.commit_executed(plan, 1).unwrap();
        assert_eq!(clock.client_discard_excess(2), Duration::from_millis(600));
        assert_eq!(
            clock.plan(start + Duration::from_millis(950)).unwrap().due,
            2
        );
    }

    #[test]
    fn rejects_time_reversal_and_overcommit() {
        let start = Instant::now();
        let mut clock = FixedClock::new(60, start).unwrap();
        assert!(matches!(
            clock.plan(
                start
                    .checked_sub(Duration::from_nanos(1))
                    .expect("test instant supports one-nanosecond subtraction")
            ),
            Err(ClockError::TimeWentBackwards)
        ));
        let plan = clock.plan(start).unwrap();
        assert_eq!(
            clock.commit_executed(plan, 1),
            Err(ClockError::ExecutedMoreThanDue)
        );
    }
}
