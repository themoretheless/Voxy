//! Transactional dyadic stopping before accepted-component node/face clearance.
use super::QuadraticDynamics;
use crate::plasticity::mesh::{QuadraticSweepLimits, Vec3};
#[derive(Clone, Debug)]
pub struct QuadraticClearanceStep {
    pub accepted_dt_s: f64,
    pub energy_defect_j: f64,
    pub attempts: usize,
    /// True if a longer step was rejected by dynamics or clearance guards.
    pub shortened: bool,
}
impl QuadraticDynamics {
    /// Try a Verlet step, halving its duration until every accepted-component
    /// exposed-node/face drift is separated. The drift is linear after the first
    /// kick, exactly matching the searched trajectory. The total energy budget
    /// is apportioned by accepted_dt/requested_dt. Only one accepted prefix is
    /// committed; the caller must handle remaining time and collision response.
    /// No state changes on failure or minimum-step exhaustion. New crack surfaces
    /// born during this step and edge-edge crossings are not covered.
    pub fn step_loaded_before_fragment_clearance(
        &mut self,
        requested_dt_s: f64,
        minimum_dt_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        energy_tolerance_j: f64,
        clearance_m: f64,
        limits: QuadraticSweepLimits,
        max_queries: usize,
        max_attempts: usize,
    ) -> Result<QuadraticClearanceStep, &'static str> {
        if !requested_dt_s.is_finite()
            || !minimum_dt_s.is_finite()
            || minimum_dt_s <= 0.
            || requested_dt_s < minimum_dt_s
            || max_attempts == 0
        {
            return Err("invalid fragment clearance step limits");
        }
        let mut dt = requested_dt_s;
        for attempt in 1..=max_attempts {
            let mut candidate = self.clone();
            let defect = match candidate.step_loaded(
                dt,
                loads,
                acceleration,
                energy_tolerance_j * (dt / requested_dt_s),
            ) {
                Ok(defect) => Some(defect),
                Err(
                    "quadratic dynamic energy defect"
                    | "inverted quadratic integration point"
                    | "quadratic friction kick energy increase"
                    | "quadratic Coulomb iteration limit reached"
                    | "quadratic surface motion limit reached"
                    | "quadratic surface sweep clearance reached"
                    | "quadratic surface sweep unresolved",
                ) => None,
                Err(error) => return Err(error),
            };
            if defect.is_none() {
                let reduced = 0.5 * dt;
                if reduced < minimum_dt_s || reduced >= dt {
                    return Err("fragment clearance minimum step reached");
                }
                dt = reduced;
                continue;
            }
            let defect = defect.unwrap();
            let possible = self.body.fragment_sweeps_at(
                &candidate.body.positions,
                clearance_m,
                limits,
                max_queries,
            )?;
            if possible.is_empty() {
                *self = candidate;
                return Ok(QuadraticClearanceStep {
                    accepted_dt_s: dt,
                    energy_defect_j: defect,
                    attempts: attempt,
                    shortened: attempt > 1,
                });
            }
            let reduced = 0.5 * dt;
            if reduced < minimum_dt_s || reduced >= dt {
                return Err("fragment clearance minimum step reached");
            }
            dt = reduced;
        }
        Err("fragment clearance attempt limit")
    }
}
impl super::FiniteQuadraticDynamics {
    pub fn step_loaded_before_fragment_clearance(
        &mut self,
        requested_dt_s: f64,
        minimum_dt_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        energy_tolerance_j: f64,
        clearance_m: f64,
        limits: QuadraticSweepLimits,
        max_queries: usize,
        max_attempts: usize,
    ) -> Result<QuadraticClearanceStep, &'static str> {
        self.inner.step_loaded_before_fragment_clearance(
            requested_dt_s,
            minimum_dt_s,
            loads,
            acceleration,
            energy_tolerance_j,
            clearance_m,
            limits,
            max_queries,
            max_attempts,
        )
    }
}

#[derive(Clone, Debug)]
pub struct QuadraticClearanceAdvance {
    pub advanced_s: f64,
    pub remaining_s: f64,
    pub absolute_energy_defect_j: f64,
    pub steps: Vec<QuadraticClearanceStep>,
    /// None means the full interval was consumed. A guard stop publishes only
    /// the separated prefix and leaves collision handling to the caller.
    pub stop_reason: Option<&'static str>,
}
impl QuadraticDynamics {
    /// Consume separated prefixes until completion or a bounded guard stop.
    /// Guard exhaustion returns an explicit partial-progress report. All other
    /// failures roll back the whole interval, including earlier valid prefixes.
    pub fn advance_loaded_until_fragment_clearance(
        &mut self,
        interval_s: f64,
        minimum_dt_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        energy_tolerance_j: f64,
        clearance_m: f64,
        limits: QuadraticSweepLimits,
        max_queries: usize,
        max_attempts_per_step: usize,
        max_steps: usize,
    ) -> Result<QuadraticClearanceAdvance, &'static str> {
        if !interval_s.is_finite()
            || interval_s <= 0.
            || !minimum_dt_s.is_finite()
            || minimum_dt_s <= 0.
            || minimum_dt_s > interval_s
            || max_steps == 0
        {
            return Err("invalid fragment clearance interval");
        }
        let mut candidate = self.clone();
        let mut report = QuadraticClearanceAdvance {
            advanced_s: 0.,
            remaining_s: interval_s,
            absolute_energy_defect_j: 0.,
            steps: Vec::new(),
            stop_reason: None,
        };
        while report.remaining_s > 0. {
            if report.steps.len() == max_steps {
                report.stop_reason = Some("fragment clearance interval step limit");
                break;
            }
            if report.remaining_s < minimum_dt_s {
                report.stop_reason = Some("fragment clearance remaining time below minimum");
                break;
            }
            match candidate.step_loaded_before_fragment_clearance(
                report.remaining_s,
                minimum_dt_s,
                loads,
                acceleration,
                energy_tolerance_j * (report.remaining_s / interval_s),
                clearance_m,
                limits,
                max_queries,
                max_attempts_per_step,
            ) {
                Ok(step) => {
                    let remaining = report.remaining_s - step.accepted_dt_s;
                    let advanced = report.advanced_s + step.accepted_dt_s;
                    if remaining >= report.remaining_s || advanced <= report.advanced_s {
                        return Err("unrepresentable fragment clearance time increment");
                    }
                    report.remaining_s = remaining;
                    report.advanced_s = advanced;
                    report.absolute_energy_defect_j += step.energy_defect_j.abs();
                    if !report.absolute_energy_defect_j.is_finite()
                        || report.absolute_energy_defect_j > energy_tolerance_j
                    {
                        return Err("fragment clearance interval energy budget exceeded");
                    }
                    report.steps.push(step);
                }
                Err(
                    reason @ ("fragment clearance minimum step reached"
                    | "fragment clearance attempt limit"),
                ) => {
                    report.stop_reason = Some(reason);
                    break;
                }
                Err(error) => return Err(error),
            }
        }
        *self = candidate;
        Ok(report)
    }
}
