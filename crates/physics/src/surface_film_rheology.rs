//! No-slip lubrication profile for a Herschel–Bulkley material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilmRheology {
    /// Consistency K in Pa s^n, strictly positive.
    pub consistency: f64,
    pub flow_index: f64,
    pub yield_stress: f64,
    /// Midpoint quadrature slices for combined pressure and surface shear (1..4096).
    pub profile_samples: usize,
}
/// Steady confined-film mechanical powers per wetted area, W/m².
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlidingFilmBalance {
    pub flux: f64,
    pub upper_traction: f64,
    /// Work supplied by the moving upper wall; may be negative.
    pub wall_power: f64,
    /// Work supplied by the pressure gradient; may be negative.
    pub pressure_power: f64,
    pub dissipated_power: f64,
}
impl FilmRheology {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.consistency.is_finite()
            || self.consistency <= 0.0
            || !self.flow_index.is_finite()
            || self.flow_index <= 0.0
            || !self.yield_stress.is_finite()
            || self.yield_stress < 0.0
            || !(1..=4096).contains(&self.profile_samples)
        {
            return Err("invalid film rheology");
        }
        Ok(())
    }
    /// Flux per edge width in m²/s. With s measured from the free surface,
    /// stress(s)=surface_stress+pressure_gradient*s and q=integral(s*shear_rate ds).
    /// Exact for pure pressure, pure shear and the Newtonian limit; otherwise uses
    /// explicit midpoint quadrature, including yield regions and stress reversal.
    pub fn flow_rate(
        self,
        height: f64,
        pressure_gradient: f64,
        surface_stress: f64,
    ) -> Result<f64, &'static str> {
        self.validate()?;
        if !height.is_finite()
            || height < 0.0
            || !pressure_gradient.is_finite()
            || !surface_stress.is_finite()
        {
            return Err("invalid film profile");
        }
        let power = 1.0 / self.flow_index;
        let rate = |stress: f64| {
            stress.signum()
                * ((stress.abs() - self.yield_stress).max(0.0) / self.consistency).powf(power)
        };
        let flow = if pressure_gradient == 0.0 {
            0.5 * height * height * rate(surface_stress)
        } else if self.flow_index == 1.0 && self.yield_stress == 0.0 {
            (pressure_gradient * height.powi(3) / 3.0 + surface_stress * height * height / 2.0)
                / self.consistency
        } else if surface_stress == 0.0 {
            let gradient = pressure_gradient.abs();
            let excess = (gradient * height - self.yield_stress).max(0.0);
            if excess == 0.0 {
                return Ok(0.0);
            }
            pressure_gradient.signum()
                * ((excess / self.consistency).powf(power))
                * (excess * excess / (power + 2.0) + self.yield_stress * excess / (power + 1.0))
                / gradient.powi(2)
        } else {
            let step = height / self.profile_samples as f64;
            (0..self.profile_samples)
                .map(|i| {
                    let s = (i as f64 + 0.5) * step;
                    s * rate(surface_stress + pressure_gradient * s) * step
                })
                .sum::<f64>()
        };
        if !flow.is_finite() {
            return Err("film rheology overflow");
        }
        Ok(flow)
    }
    /// Vector profile: yielding uses total tangential stress magnitude, so an
    /// oblique traction does not acquire an artificial edge-dependent threshold.
    /// `normal` is the unit in-surface edge direction; return flux is its component.
    pub fn flow_rate_vector(
        self,
        height: f64,
        gradient: f64,
        normal: [f64; 3],
        traction: [f64; 3],
    ) -> Result<f64, &'static str> {
        self.validate()?;
        let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        let magnitude = dot(traction, traction).sqrt();
        if !height.is_finite()
            || height < 0.0
            || !gradient.is_finite()
            || !magnitude.is_finite()
            || normal.iter().any(|v| !v.is_finite())
            || (dot(normal, normal) - 1.0).abs() > 1e-12
        {
            return Err("invalid vector film profile");
        }
        let component = dot(traction, normal);
        if magnitude == 0.0 {
            return self.flow_rate(height, gradient, 0.0);
        }
        if gradient == 0.0 {
            return self
                .flow_rate(height, 0.0, magnitude)
                .map(|q| q * component / magnitude);
        }
        if self.flow_index == 1.0 && self.yield_stress == 0.0 {
            return self.flow_rate(height, gradient, component);
        }
        let step = height / self.profile_samples as f64;
        let flow = (0..self.profile_samples)
            .map(|i| {
                let s = (i as f64 + 0.5) * step;
                let stress = std::array::from_fn(|k| traction[k] + gradient * s * normal[k]);
                let length = dot(stress, stress).sqrt();
                if length == 0.0 {
                    0.0
                } else {
                    s * dot(stress, normal) / length
                        * ((length - self.yield_stress).max(0.0) / self.consistency)
                            .powf(1.0 / self.flow_index)
                        * step
                }
            })
            .sum::<f64>();
        if !flow.is_finite() {
            return Err("vector film profile overflow");
        }
        Ok(flow)
    }
    /// Godunov flux for the frozen edge profile q(h), including an interior
    /// extremum when pressure and surface traction oppose each other.
    /// The derivative has sign (surface_traction dot normal + gradient*h),
    /// outside yielded plateaux. Endpoints plus that critical height suffice.
    pub fn interface_flow(
        self,
        left: f64,
        right: f64,
        gradient: f64,
        normal: [f64; 3],
        traction: [f64; 3],
    ) -> Result<f64, &'static str> {
        let first = self.flow_rate_vector(left, gradient, normal, traction)?;
        let second = self.flow_rate_vector(right, gradient, normal, traction)?;
        let select = |a: f64, b: f64| if left <= right { a.min(b) } else { a.max(b) };
        let mut flow = select(first, second);
        if gradient != 0.0 {
            let component = traction.iter().zip(normal).map(|(a, b)| a * b).sum::<f64>();
            let critical = -component / gradient;
            if critical > left.min(right) && critical < left.max(right) {
                flow = select(
                    flow,
                    self.flow_rate_vector(critical, gradient, normal, traction)?,
                );
            }
        }
        Ok(flow)
    }
    /// Confined no-slip film with prescribed upper-wall speed (m/s), lower wall
    /// stationary. Returns (flux per width, upper-wall traction). Gap is positive.
    /// Pure Couette and Newtonian Couette/Poiseuille are exact; mixed nonlinear
    /// loading uses profile quadrature and a bracketed traction inversion.
    /// This is a local profile, not a body contact or normal-pressure solve.
    pub fn sliding_profile(
        self,
        gap: f64,
        gradient: f64,
        speed: f64,
    ) -> Result<(f64, f64), &'static str> {
        self.validate()?;
        if !gap.is_finite() || gap <= 0.0 || !gradient.is_finite() || !speed.is_finite() {
            return Err("invalid sliding film profile");
        }
        let stress = if gradient == 0.0 {
            if speed == 0.0 {
                0.0
            } else {
                speed.signum()
                    * (self.yield_stress
                        + self.consistency * (speed.abs() / gap).powf(self.flow_index))
            }
        } else if self.flow_index == 1.0 && self.yield_stress == 0.0 {
            self.consistency * speed / gap - gradient * gap / 2.0
        } else {
            let velocity = |traction: f64| -> Result<f64, &'static str> {
                let step = gap / self.profile_samples as f64;
                let value = (0..self.profile_samples)
                    .map(|i| {
                        let stress = traction + gradient * (i as f64 + 0.5) * step;
                        stress.signum()
                            * ((stress.abs() - self.yield_stress).max(0.0) / self.consistency)
                                .powf(1.0 / self.flow_index)
                            * step
                    })
                    .sum::<f64>();
                if !value.is_finite() {
                    return Err("sliding profile overflow");
                }
                Ok(value)
            };
            let scale = self.yield_stress
                + self.consistency * (speed.abs() / gap).powf(self.flow_index)
                + (gradient * gap).abs()
                + 1.0;
            if !scale.is_finite() {
                return Err("sliding traction overflow");
            }
            let mut lower = -scale;
            let mut upper = scale;
            if velocity(lower)? > speed || velocity(upper)? < speed {
                return Err("sliding traction not bracketed");
            }
            for _ in 0..96 {
                let middle = 0.5 * lower + 0.5 * upper;
                if middle == lower || middle == upper {
                    break;
                }
                if velocity(middle)? < speed {
                    lower = middle;
                } else {
                    upper = middle;
                }
            }
            0.5 * lower + 0.5 * upper
        };
        if !stress.is_finite() {
            return Err("sliding traction overflow");
        }
        let flow = if gradient == 0.0 {
            gap * speed / 2.0
        } else if self.flow_index == 1.0 && self.yield_stress == 0.0 {
            gap * speed / 2.0 + gradient * gap.powi(3) / (12.0 * self.consistency)
        } else {
            self.flow_rate(gap, gradient, stress)?
        };
        if !flow.is_finite() {
            return Err("sliding flux overflow");
        }
        Ok((flow, stress))
    }
    /// Local steady work balance; no thermal/body state is advanced. Upper body
    /// receives opposite traction. Multiply powers by wetted area and time to
    /// obtain work/heat; do not also debit the same work from a traction stage.
    pub fn sliding_balance(
        self,
        gap: f64,
        gradient: f64,
        speed: f64,
    ) -> Result<SlidingFilmBalance, &'static str> {
        let (flux, traction) = self.sliding_profile(gap, gradient, speed)?;
        let wall = traction * speed;
        let pressure = gradient * flux;
        let heat = wall + pressure;
        if !wall.is_finite()
            || !pressure.is_finite()
            || !heat.is_finite()
            || heat < -128.0 * f64::EPSILON * (wall.abs() + pressure.abs()).max(f64::MIN_POSITIVE)
        {
            return Err("invalid sliding film work balance");
        }
        Ok(SlidingFilmBalance {
            flux,
            upper_traction: traction,
            wall_power: wall,
            pressure_power: pressure,
            dissipated_power: heat.max(0.0),
        })
    }
}
