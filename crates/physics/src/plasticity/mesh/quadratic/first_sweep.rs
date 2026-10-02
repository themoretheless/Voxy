//! Earliest-clearance time bracketing using repeated bounded prefix searches.
use super::{QuadraticClosestPoint, QuadraticFace, QuadraticSweep, QuadraticSweepLimits, Vec3};
#[derive(Clone, Debug)]
pub struct QuadraticFirstClearance {
    /// All queried times strictly before the lower endpoint were separated.
    /// Upper endpoint is a feasible witness when witness is Some, otherwise
    /// merely the end of the earliest unresolved interval.
    pub time_interval: [f64; 2],
    pub witness: Option<QuadraticClosestPoint>,
    pub prefix_searches: usize,
    pub converged: bool,
}
impl QuadraticFace {
    /// Bracket the earliest clearance event on fixed linear trajectories.
    /// None means the whole trajectory was separated. Search uncertainty is
    /// retained, never treated as absence of contact. Floating-point geometric
    /// bounds have the same limitations as swept_point_clearance_at.
    pub fn first_point_clearance_at(
        &self,
        start: &[Vec3],
        end: &[Vec3],
        point_start: Vec3,
        point_end: Vec3,
        clearance_m: f64,
        limits: QuadraticSweepLimits,
        time_tolerance: f64,
        max_prefix_searches: usize,
    ) -> Result<Option<QuadraticFirstClearance>, &'static str> {
        if !time_tolerance.is_finite()
            || time_tolerance <= 0.
            || time_tolerance > 1.
            || max_prefix_searches == 0
            || max_prefix_searches > 65536
        {
            return Err("invalid first-clearance limits");
        }
        let initial =
            self.swept_point_clearance_at(start, end, point_start, point_end, clearance_m, limits)?;
        let (mut lower, mut upper, mut witness) = match initial {
            QuadraticSweep::Separated { .. } => return Ok(None),
            QuadraticSweep::WithinClearance {
                time_fraction,
                closest,
                ..
            } => (0., time_fraction, Some(closest)),
            QuadraticSweep::Unresolved { time_interval, .. } => {
                (time_interval[0], time_interval[1], None)
            }
        };
        let mut searches = 1;
        while witness.is_some() && upper - lower > time_tolerance && searches < max_prefix_searches
        {
            let mid = lower.midpoint(upper);
            if mid <= lower || mid >= upper {
                break;
            }
            let prefix: Vec<Vec3> = start
                .iter()
                .zip(end)
                .map(|(a, b)| std::array::from_fn(|k| (1. - mid) * a[k] + mid * b[k]))
                .collect();
            let point = std::array::from_fn(|k| (1. - mid) * point_start[k] + mid * point_end[k]);
            let query = self.swept_point_clearance_at(
                start,
                &prefix,
                point_start,
                point,
                clearance_m,
                limits,
            )?;
            searches += 1;
            match query {
                QuadraticSweep::Separated { .. } => lower = mid,
                QuadraticSweep::WithinClearance {
                    time_fraction,
                    closest,
                    ..
                } => {
                    upper = mid * time_fraction;
                    witness = Some(closest);
                }
                QuadraticSweep::Unresolved { time_interval, .. } => {
                    let proven = mid * time_interval[0];
                    if proven <= lower {
                        break;
                    }
                    lower = proven;
                }
            }
        }
        Ok(Some(QuadraticFirstClearance {
            time_interval: [lower, upper],
            converged: witness.is_some() && upper - lower <= time_tolerance,
            witness,
            prefix_searches: searches,
        }))
    }
}
