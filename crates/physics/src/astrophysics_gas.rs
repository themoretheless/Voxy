//! One-dimensional finite-volume ideal gas Euler solver (first-order Rusanov).
//! No vacuum floors: invalid/negative states fail atomically. Units are consistent
//! application units; boundaries explicitly determine flux conservation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub density: f64,
    pub momentum: f64,
    pub energy: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NonPhysicalState,
    NumericalOverflow,
    BudgetExceeded,
    Eos(crate::astrophysics_eos::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    Periodic,
    Outflow,
    Reflecting,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Gas {
    pub cells: Vec<Cell>,
    pub spacing: f64,
    pub gamma: f64,
    pub boundary: Boundary,
}
/// Integrated finite-volume exchanges per cross-sectional area.
#[derive(Clone, Debug, PartialEq)]
pub struct Transport {
    pub steps: usize,
    /// Rightward mass crossing each interface, including both boundaries.
    pub mass: Vec<f64>,
    /// Net outgoing mass, momentum and gas energy, signed for inflow.
    pub boundary: [f64; 3],
}
impl Cell {
    /// Construct conserved densities from positive density and pressure.
    /// # Errors
    /// Rejects invalid parameters, nonphysical states and floating-point overflow.
    pub fn from_primitive(
        density: f64,
        velocity: f64,
        pressure: f64,
        gamma: f64,
    ) -> Result<Self, Error> {
        if !velocity.is_finite()
            || !pressure.is_finite()
            || pressure <= 0.0
            || !gamma.is_finite()
            || gamma <= 1.0
        {
            return Err(Error::InvalidInput);
        }
        let value = Self {
            density,
            momentum: density * velocity,
            energy: pressure / (gamma - 1.0) + 0.5 * density * velocity * velocity,
        };
        value.pressure(gamma)?;
        Ok(value)
    }
    /// Ideal-gas pressure from conserved densities.
    /// # Errors
    /// Rejects invalid gamma, nonpositive density/pressure and overflow.
    pub fn pressure(self, gamma: f64) -> Result<f64, Error> {
        if !gamma.is_finite() || gamma <= 1.0 {
            return Err(Error::InvalidInput);
        }
        if ![self.density, self.momentum, self.energy]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        if self.density <= 0.0 {
            return Err(Error::NonPhysicalState);
        }
        let pressure =
            (gamma - 1.0) * (self.energy - 0.5 * self.momentum * (self.momentum / self.density));
        if !pressure.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        if pressure <= 0.0 {
            return Err(Error::NonPhysicalState);
        }
        Ok(pressure)
    }
    fn values(self) -> [f64; 3] {
        [self.density, self.momentum, self.energy]
    }
    pub(crate) fn wave(self, gamma: f64) -> Result<f64, Error> {
        let speed = (self.momentum / self.density).abs()
            + (gamma * self.pressure(gamma)? / self.density).sqrt();
        if speed.is_finite() {
            Ok(speed)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    fn flux(self, gamma: f64) -> Result<[f64; 3], Error> {
        let p = self.pressure(gamma)?;
        let u = self.momentum / self.density;
        Ok([self.momentum, self.momentum * u + p, (self.energy + p) * u])
    }
}
pub(crate) fn interface(left: Cell, right: Cell, gamma: f64) -> Result<[f64; 3], Error> {
    let speed = left.wave(gamma)?.max(right.wave(gamma)?);
    let l = left.values();
    let r = right.values();
    let fl = left.flux(gamma)?;
    let fr = right.flux(gamma)?;
    Ok(std::array::from_fn(|i| {
        0.5 * (fl[i] + fr[i]) - 0.5 * speed * (r[i] - l[i])
    }))
}
impl Gas {
    fn validate(&self) -> Result<(), Error> {
        if self.cells.len() < 2
            || !self.spacing.is_finite()
            || self.spacing <= 0.0
            || !self.gamma.is_finite()
            || self.gamma <= 1.0
        {
            return Err(Error::InvalidInput);
        }
        for cell in &self.cells {
            cell.pressure(self.gamma)?;
        }
        Ok(())
    }
    /// Integrals per unit cross-sectional area: mass, momentum, total energy.
    /// # Errors
    /// Rejects invalid mesh/state or overflowing integrals.
    pub fn totals(&self) -> Result<[f64; 3], Error> {
        self.validate()?;
        let mut totals = [0.0; 3];
        for cell in &self.cells {
            for (total, value) in totals.iter_mut().zip(cell.values()) {
                *total += value * self.spacing;
            }
        }
        if totals.into_iter().all(f64::is_finite) {
            Ok(totals)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Adaptive CFL <= 0.4. Budget or invalid evolution leaves all cells unchanged.
    /// # Errors
    /// Rejects invalid parameters/states, exhausted budget or numerical overflow.
    pub fn step(&mut self, dt: f64, max_substeps: usize) -> Result<usize, Error> {
        Ok(self.advance(dt, max_substeps)?.steps)
    }
    /// Advance and return actual mass transport plus net boundary exchanges.
    /// # Errors
    /// Same validation and atomic budget failures as `step`.
    pub fn advance(&mut self, dt: f64, max_substeps: usize) -> Result<Transport, Error> {
        self.advance_impl(dt, max_substeps, false)
    }
    /// Evolve with a contact-resolving HLLC flux and the same atomic CFL budget.
    /// Returned transport contains the actual HLLC/fallback interface fluxes.
    /// # Errors
    /// Invalid input/state, exhausted budget or nonphysical evolution.
    pub fn advance_contact(&mut self, dt: f64, max_substeps: usize) -> Result<Transport, Error> {
        self.advance_impl(dt, max_substeps, true)
    }
    fn advance_impl(
        &mut self,
        dt: f64,
        max_substeps: usize,
        contact: bool,
    ) -> Result<Transport, Error> {
        self.validate()?;
        if !dt.is_finite() || dt < 0.0 {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let mut remaining = dt;
        let mut steps = 0;
        let mut mass = vec![0.0; self.cells.len() + 1];
        let mut boundary = [0.0; 3];
        while remaining > 0.0 {
            if steps >= max_substeps {
                return Err(Error::BudgetExceeded);
            }
            let mut speed: f64 = 0.0;
            for cell in &next.cells {
                speed = speed.max(cell.wave(next.gamma)?);
            }
            let h = remaining.min(0.4 * next.spacing / speed);
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let n = next.cells.len();
            let first = next.cells[0];
            let last = next.cells[n - 1];
            let (left, right) = match next.boundary {
                Boundary::Periodic => (last, first),
                Boundary::Outflow => (first, last),
                Boundary::Reflecting => (
                    Cell {
                        momentum: -first.momentum,
                        ..first
                    },
                    Cell {
                        momentum: -last.momentum,
                        ..last
                    },
                ),
            };
            let solve = if contact { contact_flux } else { interface };
            let mut fluxes = Vec::with_capacity(n + 1);
            fluxes.push(solve(left, first, next.gamma)?);
            for pair in next.cells.windows(2) {
                fluxes.push(solve(pair[0], pair[1], next.gamma)?);
            }
            fluxes.push(solve(last, right, next.gamma)?);
            for (transport, flux) in mass.iter_mut().zip(&fluxes) {
                *transport += h * flux[0];
            }
            for (i, value) in boundary.iter_mut().enumerate() {
                *value += h * (fluxes[n][i] - fluxes[0][i]);
            }
            for (i, cell) in next.cells.iter_mut().enumerate() {
                let values = cell.values();
                let q: [f64; 3] = std::array::from_fn(|j| {
                    values[j] - h / next.spacing * (fluxes[i + 1][j] - fluxes[i][j])
                });
                *cell = Cell {
                    density: q[0],
                    momentum: q[1],
                    energy: q[2],
                };
                cell.pressure(next.gamma)?;
            }
            remaining -= h;
            steps += 1;
        }
        if !mass.iter().all(|m| m.is_finite()) || !boundary.into_iter().all(f64::is_finite) {
            return Err(Error::NumericalOverflow);
        }
        *self = next;
        Ok(Transport {
            steps,
            mass,
            boundary,
        })
    }
}

/// Contact-resolving HLLC flux for the ideal-gas Euler equations.
/// Uses bounding acoustic speeds; falls back to Rusanov when a star state
/// is nonphysical or a wave denominator is degenerate.
/// # Errors
/// Invalid gas states or numerical overflow.
pub fn contact_flux(left: Cell, right: Cell, gamma: f64) -> Result<[f64; 3], Error> {
    let pl = left.pressure(gamma)?;
    let pr = right.pressure(gamma)?;
    contact_flux_thermal(
        left,
        right,
        (pl, (gamma * pl / left.density).sqrt()),
        (pr, (gamma * pr / right.density).sqrt()),
    )
}
/// HLLC with local fully ionized gas plus trapped-radiation thermodynamics.
/// Both states use their own composition; radiation inertia is neglected.
/// # Errors
/// Invalid state/EOS, relativistic sound speed or numerical overflow.
pub fn contact_flux_ionized(
    left: Cell,
    right: Cell,
    left_mixture: crate::astrophysics_eos::Mixture,
    right_mixture: crate::astrophysics_eos::Mixture,
) -> Result<[f64; 3], Error> {
    let state = |cell: Cell, mixture: crate::astrophysics_eos::Mixture| {
        let internal = cell.pressure(5.0 / 3.0)? / (2.0 / 3.0);
        let temperature = mixture
            .temperature(cell.density, internal)
            .map_err(Error::Eos)?;
        let thermal = mixture.at(cell.density, temperature).map_err(Error::Eos)?;
        if thermal.sound_speed_squared >= crate::astrophysics_eos::LIGHT_SPEED.powi(2) {
            return Err(Error::InvalidInput);
        }
        Ok((
            thermal.gas_pressure + thermal.radiation_pressure,
            thermal.sound_speed_squared.sqrt(),
        ))
    };
    contact_flux_thermal(
        left,
        right,
        state(left, left_mixture)?,
        state(right, right_mixture)?,
    )
}
pub(crate) fn contact_flux_thermal(
    left: Cell,
    right: Cell,
    l: (f64, f64),
    r: (f64, f64),
) -> Result<[f64; 3], Error> {
    let (pl, cl) = l;
    let (pr, cr) = r;
    if left.momentum == 0.0 && right.momentum == 0.0 && pl == pr {
        return Ok([0.0, pl, 0.0]);
    }
    let ul = left.momentum / left.density;
    let ur = right.momentum / right.density;
    let fallback = || {
        let speed = (ul.abs() + cl).max(ur.abs() + cr);
        let ql = left.values();
        let qr = right.values();
        let fl = [
            left.momentum,
            left.momentum * ul + pl,
            (left.energy + pl) * ul,
        ];
        let fr = [
            right.momentum,
            right.momentum * ur + pr,
            (right.energy + pr) * ur,
        ];
        let flux: [f64; 3] =
            std::array::from_fn(|i| 0.5 * (fl[i] + fr[i]) - 0.5 * speed * (qr[i] - ql[i]));
        if flux.into_iter().all(f64::is_finite) {
            Ok(flux)
        } else {
            Err(Error::NumericalOverflow)
        }
    };
    let sl = (ul - cl).min(ur - cr);
    let sr = (ul + cl).max(ur + cr);
    let fl = [
        left.momentum,
        left.momentum * ul + pl,
        (left.energy + pl) * ul,
    ];
    let fr = [
        right.momentum,
        right.momentum * ur + pr,
        (right.energy + pr) * ur,
    ];
    if ![sl, sr].into_iter().all(f64::is_finite) {
        return Err(Error::NumericalOverflow);
    }
    if sl >= 0.0 {
        return Ok(fl);
    }
    if sr <= 0.0 {
        return Ok(fr);
    }
    let dl = left.density * (sl - ul);
    let dr = right.density * (sr - ur);
    let contact = (pr - pl + dl * ul - dr * ur) / (dl - dr);
    if !contact.is_finite() || contact <= sl || contact >= sr {
        return fallback();
    }
    let (cell, pressure, velocity, speed, flux) = if contact >= 0.0 {
        (left, pl, ul, sl, fl)
    } else {
        (right, pr, ur, sr, fr)
    };
    let density = cell.density * (speed - velocity) / (speed - contact);
    let star = Cell {
        density,
        momentum: density * contact,
        energy: density
            * (cell.energy / cell.density
                + (contact - velocity)
                    * (contact + pressure / (cell.density * (speed - velocity)))),
    };
    if star.pressure(5.0 / 3.0).is_err() {
        return fallback();
    }
    let q = cell.values();
    let qs = star.values();
    let result = std::array::from_fn(|i| flux[i] + speed * (qs[i] - q[i]));
    if result.into_iter().all(f64::is_finite) {
        Ok(result)
    } else {
        Err(Error::NumericalOverflow)
    }
}
