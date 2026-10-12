//! Newtonian spherical hydrostatic polytropes. Not a stellar evolution model.
//! Lane–Emden: theta'=-m/xi², m'=xi² theta^n, regular at the centre.
use crate::astrophysics::Error;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub xi: f64,
    pub theta: f64,
    /// Dimensionless enclosed mass -xi² theta'.
    pub mass: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub index: f64,
    pub points: Vec<Point>,
    /// First zero, if reached within the requested integration domain.
    pub surface: Option<Point>,
}
fn derivative(x: f64, y: [f64; 2], n: f64) -> [f64; 2] {
    [-y[1] / (x * x), x * x * y[0].max(0.0).powf(n)]
}
fn rk(xi: f64, state: [f64; 2], step: f64, index: f64) -> [f64; 2] {
    let first = derivative(xi, state, index);
    let second = derivative(
        xi + step / 2.0,
        std::array::from_fn(|i| state[i] + step * first[i] / 2.0),
        index,
    );
    let third = derivative(
        xi + step / 2.0,
        std::array::from_fn(|i| state[i] + step * second[i] / 2.0),
        index,
    );
    let fourth = derivative(
        xi + step,
        std::array::from_fn(|i| state[i] + step * third[i]),
        index,
    );
    std::array::from_fn(|i| {
        state[i] + step * (first[i] + 2.0 * second[i] + 2.0 * third[i] + fourth[i]) / 6.0
    })
}
/// Integrate with RK4 and a centre series; surface is refined by bisection.
/// Step controls numerical accuracy; `max_steps` bounds CPU and profile size.
/// # Errors
/// Invalid parameters, numerical overflow, or exhausted work budget.
pub fn lane_emden(index: f64, step: f64, max_xi: f64, max_steps: usize) -> Result<Profile, Error> {
    if ![index, step, max_xi].into_iter().all(f64::is_finite)
        || !(0.0..=5.0).contains(&index)
        || step <= 0.0
        || step > 0.1
        || max_xi <= 0.0
        || max_steps == 0
    {
        return Err(Error::InvalidInput);
    }
    let mut profile = Profile {
        index,
        points: vec![Point {
            xi: 0.0,
            theta: 1.0,
            mass: 0.0,
        }],
        surface: None,
    };
    let mut x = step.min(max_xi).min(1e-4);
    let mut y = [
        1.0 - x * x / 6.0 + index * x.powi(4) / 120.0,
        x.powi(3) / 3.0 - index * x.powi(5) / 30.0,
    ];
    profile.points.push(Point {
        xi: x,
        theta: y[0],
        mass: y[1],
    });
    let mut count = 0;
    while x < max_xi {
        if count >= max_steps {
            return Err(Error::NoConvergence);
        }
        let h = step.min(max_xi - x);
        if x + h <= x {
            return Err(Error::NumericalOverflow);
        }
        let next = rk(x, y, h, index);
        if !next.into_iter().all(f64::is_finite) {
            return Err(Error::NumericalOverflow);
        }
        if next[0] <= 0.0 {
            let mut lo: f64 = 0.0;
            let mut hi = h;
            for _ in 0..60 {
                let mid = lo.midpoint(hi);
                if rk(x, y, mid, index)[0] > 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let h = lo.midpoint(hi);
            let state = rk(x, y, h, index);
            let surface = Point {
                xi: x + h,
                theta: 0.0,
                mass: state[1],
            };
            profile.points.push(surface);
            profile.surface = Some(surface);
            return Ok(profile);
        }
        x += h;
        y = next;
        profile.points.push(Point {
            xi: x,
            theta: y[0],
            mass: y[1],
        });
        count += 1;
    }
    Ok(profile)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scaling {
    pub length: f64,
    pub central_density: f64,
    pub central_pressure: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalPoint {
    pub radius: f64,
    pub density: f64,
    pub pressure: f64,
    pub enclosed_mass: f64,
}
impl Scaling {
    /// Construct alpha²=(n+1) Pc/(4 pi G `rho_c²`). Uses central pressure,
    /// so index zero (incompressible limit) has a well-defined scaling too.
    /// # Errors
    /// Invalid/nonpositive parameters or overflow.
    pub fn new(
        index: f64,
        central_density: f64,
        central_pressure: f64,
        g: f64,
    ) -> Result<Self, Error> {
        if ![index, central_density, central_pressure, g]
            .into_iter()
            .all(f64::is_finite)
            || !(0.0..=5.0).contains(&index)
            || central_density <= 0.0
            || central_pressure <= 0.0
            || g <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        let length = ((index + 1.0) * central_pressure
            / (4.0 * std::f64::consts::PI * g * central_density * central_density))
            .sqrt();
        if !length.is_finite() || length <= 0.0 {
            return Err(Error::NumericalOverflow);
        }
        Ok(Self {
            length,
            central_density,
            central_pressure,
        })
    }
    /// Scale a dimensionless profile point to consistent physical units.
    /// # Errors
    /// Invalid point/scaling or numerical overflow.
    pub fn point(self, index: f64, point: Point) -> Result<PhysicalPoint, Error> {
        if ![
            self.length,
            self.central_density,
            self.central_pressure,
            index,
            point.xi,
            point.theta,
            point.mass,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.length <= 0.0
            || self.central_density <= 0.0
            || self.central_pressure <= 0.0
            || !(0.0..=5.0).contains(&index)
            || point.xi < 0.0
            || point.theta < 0.0
            || point.mass < 0.0
        {
            return Err(Error::InvalidInput);
        }
        let result = PhysicalPoint {
            radius: self.length * point.xi,
            density: self.central_density * point.theta.powf(index),
            pressure: self.central_pressure * point.theta.powf(index + 1.0),
            enclosed_mass: 4.0
                * std::f64::consts::PI
                * self.length.powi(3)
                * self.central_density
                * point.mass,
        };
        if [
            result.radius,
            result.density,
            result.pressure,
            result.enclosed_mass,
        ]
        .into_iter()
        .all(f64::is_finite)
        {
            Ok(result)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
}

impl Profile {
    /// Linearly interpolate theta and enclosed mass inside the stored domain.
    /// No extrapolation or density floor is applied. The public profile is
    /// validated so caller-modified points cannot silently corrupt sampling.
    /// # Errors
    /// Invalid coordinates, malformed profile, or a query outside its domain.
    pub fn sample(&self, xi: f64) -> Result<Point, Error> {
        if !xi.is_finite()
            || xi < 0.0
            || !self.index.is_finite()
            || !(0.0..=5.0).contains(&self.index)
            || self.points.len() < 2
            || self.points[0].xi != 0.0
            || self.points.iter().any(|p| {
                ![p.xi, p.theta, p.mass].into_iter().all(f64::is_finite)
                    || p.xi < 0.0
                    || p.theta < 0.0
                    || p.mass < 0.0
            })
            || self
                .points
                .windows(2)
                .any(|p| p[1].xi <= p[0].xi || p[1].theta > p[0].theta || p[1].mass < p[0].mass)
            || xi > self.points[self.points.len() - 1].xi
        {
            return Err(Error::InvalidInput);
        }
        let upper = self.points.partition_point(|p| p.xi < xi);
        if self.points[upper].xi == xi {
            return Ok(self.points[upper]);
        }
        let a = self.points[upper - 1];
        let b = self.points[upper];
        let f = (xi - a.xi) / (b.xi - a.xi);
        Ok(Point {
            xi,
            theta: a.theta + f * (b.theta - a.theta),
            mass: a.mass + f * (b.mass - a.mass),
        })
    }
}

/// Dimensionless volume-averaged density and pressure of a radial shell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellAverage {
    pub density_over_central: f64,
    pub pressure_over_central: f64,
}
impl Profile {
    /// Average theta^n and theta^(n+1) with the spherical volume weight.
    /// Composite two-point Gauss quadrature samples the interpolated profile.
    /// Accuracy depends on both Lane–Emden resolution and `intervals`.
    /// # Errors
    /// Invalid shell, zero quadrature budget, malformed profile or overflow.
    pub fn shell_average(
        &self,
        inner: f64,
        outer: f64,
        intervals: usize,
    ) -> Result<ShellAverage, Error> {
        if !inner.is_finite()
            || !outer.is_finite()
            || inner < 0.0
            || outer <= inner
            || intervals == 0
        {
            return Err(Error::InvalidInput);
        }
        self.sample(inner)?;
        self.sample(outer)?;
        // Normalize radii by the outer radius to avoid cubing physical scales.
        let start = inner / outer;
        let width = (1.0 - start) / intervals as f64;
        let normalization = (1.0 - start) * (1.0 + start + start * start) / 3.0;
        if width <= 0.0 || normalization <= 0.0 {
            return Err(Error::NumericalOverflow);
        }
        let mut density = 0.0;
        let mut pressure = 0.0;
        for i in 0..intervals {
            let midpoint = start + (i as f64 + 0.5) * width;
            for sign in [-1.0, 1.0] {
                let x = midpoint + sign * width / (2.0 * 3.0_f64.sqrt());
                let theta = self.sample(x * outer)?.theta;
                let weight = 0.5 * width * x * x / normalization;
                density += weight * theta.powf(self.index);
                pressure += weight * theta.powf(self.index + 1.0);
            }
        }
        if !density.is_finite() || !pressure.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        Ok(ShellAverage {
            density_over_central: density,
            pressure_over_central: pressure,
        })
    }
}
