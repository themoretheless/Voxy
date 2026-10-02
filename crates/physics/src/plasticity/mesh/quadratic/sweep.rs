//! Continuous point-to-T6 clearance query for linear nodal trajectories.
use super::{QuadraticClosestLimits, QuadraticClosestPoint, QuadraticFace, Vec3};
#[derive(Clone, Copy, Debug)]
pub struct QuadraticSweepLimits {
    pub closest: QuadraticClosestLimits,
    pub minimum_time_fraction: f64,
    pub max_intervals: usize,
}
#[derive(Clone, Debug)]
pub enum QuadraticSweep {
    /// Every time interval has a clearance lower bound above the requested value.
    /// Bounds are floating-point estimates, not interval certified.
    Separated { intervals: usize },
    /// A feasible witness at this fraction; not necessarily first time of impact.
    WithinClearance {
        time_fraction: f64,
        closest: QuadraticClosestPoint,
        intervals: usize,
    },
    /// Neither separation nor a feasible witness proved within the configured work.
    Unresolved {
        time_interval: [f64; 2],
        intervals: usize,
    },
}
impl QuadraticFace {
    /// Continuously check a linearly moving point against linearly moving T6 nodes.
    /// A 7/4 shape-weight motion bound limits the change in closest distance over
    /// each time interval; endpoint samples alone are not used as a safety proof.
    /// The complete normalized interval [0,1] is searched. Clearance is nonnegative.
    /// # Errors
    /// Invalid limits/indices/geometry/query, singular intermediate tangent frame,
    /// closest-point failure or arithmetic overflow.
    pub fn swept_point_clearance_at(
        &self,
        start: &[Vec3],
        end: &[Vec3],
        point_start: Vec3,
        point_end: Vec3,
        clearance_m: f64,
        limits: QuadraticSweepLimits,
    ) -> Result<QuadraticSweep, &'static str> {
        if start.len() != end.len()
            || !clearance_m.is_finite()
            || clearance_m < 0.
            || point_start.iter().chain(&point_end).any(|x| !x.is_finite())
            || !limits.minimum_time_fraction.is_finite()
            || limits.minimum_time_fraction <= 0.
            || limits.minimum_time_fraction > 1.
            || limits.max_intervals == 0
            || limits.max_intervals > 65536
        {
            return Err("invalid quadratic sweep query");
        }
        let mut first = [[0.; 3]; 6];
        let mut last = [[0.; 3]; 6];
        for (i, &node) in self.nodes.iter().enumerate() {
            let a = start.get(node).ok_or("invalid quadratic sweep node")?;
            let b = end.get(node).ok_or("invalid quadratic sweep node")?;
            first[i] = std::array::from_fn(|axis| a[axis] - point_start[axis]);
            last[i] = std::array::from_fn(|axis| b[axis] - point_end[axis]);
        }
        if first.iter().chain(&last).flatten().any(|x| !x.is_finite()) {
            return Err("quadratic sweep coordinate overflow");
        }
        let mut speed = 0_f64;
        for (a, b) in first.iter().zip(&last) {
            let delta: Vec3 = std::array::from_fn(|i| b[i] - a[i]);
            speed = speed.max(delta[0].hypot(delta[1]).hypot(delta[2]));
        }
        // Relative trajectories cancel any common uniform observer translation.
        let lipschitz = 1.75 * speed;
        if !lipschitz.is_finite() {
            return Err("quadratic sweep motion overflow");
        }
        let face = QuadraticFace {
            nodes: [0, 1, 2, 3, 4, 5],
            ..*self
        };
        let mut pending = vec![[0., 1.]];
        let mut intervals = 0;
        while let Some([lo, hi]) = pending.pop() {
            if intervals == limits.max_intervals {
                return Ok(QuadraticSweep::Unresolved {
                    time_interval: [lo, hi],
                    intervals,
                });
            }
            intervals += 1;
            let mid = lo.midpoint(hi);
            let positions = std::array::from_fn::<_, 6, _>(|i| {
                std::array::from_fn(|axis| (1. - mid) * first[i][axis] + mid * last[i][axis])
            });
            let mut closest = face.closest_point_at(&positions, [0.; 3], limits.closest)?;
            if closest.distance_m <= clearance_m {
                let query: Vec3 =
                    std::array::from_fn(|i| (1. - mid) * point_start[i] + mid * point_end[i]);
                closest.point_m = std::array::from_fn(|i| closest.point_m[i] + query[i]);
                if closest.point_m.iter().any(|x| !x.is_finite()) {
                    return Err("quadratic sweep witness overflow");
                }
                return Ok(QuadraticSweep::WithinClearance {
                    time_fraction: mid,
                    closest,
                    intervals,
                });
            }
            let radius = 0.5 * (hi - lo) * lipschitz;
            if closest.lower_distance_m - radius > clearance_m {
                continue;
            }
            if hi - lo <= limits.minimum_time_fraction || mid <= lo || mid >= hi {
                return Ok(QuadraticSweep::Unresolved {
                    time_interval: [lo, hi],
                    intervals,
                });
            }
            pending.push([mid, hi]);
            pending.push([lo, mid]);
        }
        Ok(QuadraticSweep::Separated { intervals })
    }
}
