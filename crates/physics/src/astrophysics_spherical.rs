//! Radial finite-volume Euler gas with Newtonian spherical self-gravity.
//! Fixed radial shells; regular centre, reflecting or open outer boundary.
use crate::astrophysics_gas::{Boundary, Cell, interface};
const FOUR_PI: f64 = 4.0 * std::f64::consts::PI;
#[derive(Clone, Debug, PartialEq)]
pub struct Sphere {
    pub cells: Vec<Cell>,
    pub spacing: f64,
    pub gamma: f64,
    /// Nonnegative G; zero disables gravity.
    pub g: f64,
    pub outer: Boundary,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub acceleration: Vec<f64>,
    pub potential: Vec<f64>,
    pub energy: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Gas(crate::astrophysics_gas::Error),
    Eos(crate::astrophysics_eos::Error),
    Nuclear(crate::astrophysics_nuclear::Error),
    Stellar(crate::astrophysics::Error),
    NumericalOverflow,
    BudgetExceeded,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exchange {
    pub steps: usize,
    pub escaped_mass: f64,
    pub escaped_energy: f64,
}
#[derive(Clone, Copy)]
struct Geometry {
    volume: f64,
    left_area: f64,
    right_area: f64,
    inverse_radius: f64,
    self_coefficient: f64,
    gravity_integral: f64,
    delta_cube: f64,
}
impl Sphere {
    fn geometry(&self) -> Result<Vec<Geometry>, Error> {
        if self.cells.len() < 2
            || ![self.spacing, self.gamma, self.g]
                .into_iter()
                .all(f64::is_finite)
            || self.spacing <= 0.0
            || self.gamma <= 1.0
            || self.g < 0.0
            || self.outer == Boundary::Periodic
        {
            return Err(Error::InvalidInput);
        }
        let mut result = Vec::with_capacity(self.cells.len());
        let mut a: f64 = 0.0;
        let h = self.spacing;
        for cell in &self.cells {
            cell.pressure(self.gamma).map_err(Error::Gas)?;
            let b = a + h;
            let d = h * (a * a + a * b + b * b);
            // Stable exact shell integrals in powers of thickness, avoiding cancellation.
            let integral =
                1.5 * a.powi(3) * h * h + 2.0 * a * a * h.powi(3) + a * h.powi(4) + h.powi(5) / 5.0;
            let geo = Geometry {
                volume: FOUR_PI * d / 3.0,
                left_area: FOUR_PI * a * a,
                right_area: FOUR_PI * b * b,
                inverse_radius: 1.5 * (a + b) / (a * a + a * b + b * b),
                self_coefficient: 3.0 * integral / (d * d),
                gravity_integral: 1.5 * a * a * h * h + a * h.powi(3) + h.powi(4) / 4.0,
                delta_cube: d,
            };
            if ![
                geo.volume,
                geo.right_area,
                geo.inverse_radius,
                geo.self_coefficient,
                geo.gravity_integral,
                geo.delta_cube,
            ]
            .into_iter()
            .all(f64::is_finite)
                || geo.volume <= 0.0
                || geo.delta_cube <= 0.0
            {
                return Err(Error::NumericalOverflow);
            }
            result.push(geo);
            a = b;
        }
        Ok(result)
    }
    /// Exact shell-averaged field, potential and binding energy for uniform cells.
    /// # Errors
    /// Invalid state, incompatible boundary or numerical overflow.
    pub fn field(&self) -> Result<Field, Error> {
        let geometry = self.geometry()?;
        let mass: Vec<_> = self
            .cells
            .iter()
            .zip(&geometry)
            .map(|(c, v)| c.density * v.volume)
            .collect();
        let mut outer: f64 = mass
            .iter()
            .zip(&geometry)
            .map(|(m, v)| m * v.inverse_radius)
            .sum();
        let mut inner = 0.0;
        let mut energy = 0.0;
        let mut potential = Vec::new();
        let mut acceleration = Vec::new();
        for (m, v) in mass.iter().zip(&geometry) {
            outer -= m * v.inverse_radius;
            energy -= self.g * (inner * m * v.inverse_radius + m * m * v.self_coefficient);
            potential
                .push(-self.g * (inner * v.inverse_radius + outer + 2.0 * m * v.self_coefficient));
            acceleration.push(
                -3.0 * self.g * (inner * self.spacing + m / v.delta_cube * v.gravity_integral)
                    / v.delta_cube,
            );
            inner += m;
        }
        if !energy.is_finite() || !potential.iter().chain(&acceleration).all(|x| x.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(Field {
            acceleration,
            potential,
            energy,
        })
    }
    /// Integrated mass, radial momentum and gas energy; radial momentum is not
    /// a conserved vector momentum because shell directions differ.
    /// # Errors
    /// Invalid state or numerical overflow.
    pub fn totals(&self) -> Result<[f64; 3], Error> {
        let geometry = self.geometry()?;
        let mut result = [0.0; 3];
        for (cell, v) in self.cells.iter().zip(geometry) {
            for (sum, q) in result
                .iter_mut()
                .zip([cell.density, cell.momentum, cell.energy])
            {
                *sum += q * v.volume;
            }
        }
        if result.into_iter().all(f64::is_finite) {
            Ok(result)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Gas plus binding energy, zero potential at infinity.
    /// # Errors
    /// Invalid state or numerical overflow.
    pub fn energy(&self) -> Result<f64, Error> {
        let energy = self.totals()?[2] + self.field()?.energy;
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    fn kick(&mut self, h: f64) -> Result<Vec<f64>, Error> {
        let field = self.field()?;
        let mut work = Vec::new();
        for (cell, a) in self.cells.iter_mut().zip(field.acceleration) {
            let internal = cell.pressure(self.gamma).map_err(Error::Gas)? / (self.gamma - 1.0);
            let old = cell.energy;
            cell.momentum += h * cell.density * a;
            cell.energy = internal + 0.5 * cell.momentum * (cell.momentum / cell.density);
            cell.pressure(self.gamma).map_err(Error::Gas)?;
            work.push(cell.energy - old);
        }
        Ok(work)
    }
    /// Adaptive first-order Rusanov evolution with geometric pressure source,
    /// gravity kicks and conservative mass-transport work. Entire failure rolls
    /// back state; returned signed exchange audits open boundaries.
    /// # Errors
    /// Invalid input, exhausted budget, nonphysical evolution or overflow.
    pub fn step(&mut self, dt: f64, max_step: f64, budget: usize) -> Result<Exchange, Error> {
        self.step_eos(dt, max_step, budget, None)
    }
    /// Evolve a fixed fully-ionized composition with gas + trapped radiation EOS.
    /// Internal energy includes radiation once; pressure and CFL use that same EOS.
    /// Radiation inertia is neglected; relativistic sound speeds are rejected.
    /// # Errors
    /// Same atomic failures as `step`, plus invalid/overflowing EOS states.
    pub fn step_ionized(
        &mut self,
        dt: f64,
        max_step: f64,
        budget: usize,
        mixture: crate::astrophysics_eos::Mixture,
    ) -> Result<Exchange, Error> {
        self.step_eos(dt, max_step, budget, Some(mixture))
    }
    pub(crate) fn step_eos(
        &mut self,
        dt: f64,
        max_step: f64,
        budget: usize,
        mixture: Option<crate::astrophysics_eos::Mixture>,
    ) -> Result<Exchange, Error> {
        self.step_composition_impl(dt, max_step, budget, mixture, None, None, None, None, false)
            .map(|r| r.0)
    }
    /// Advect passive mass fractions with the actual radial mass flux.
    /// Returns hydrodynamic exchange and signed escaping mass per species.
    /// Composition and gas roll back together on any failure.
    /// # Errors
    /// Invalid fractions, negative transported species, or hydrodynamic failure.
    pub fn step_composition(
        &mut self,
        fractions: &mut [Vec<f64>],
        dt: f64,
        max_step: f64,
        budget: usize,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        let mut next = fractions.to_vec();
        let result = self.step_composition_impl(
            dt,
            max_step,
            budget,
            None,
            Some(&mut next),
            None,
            None,
            None,
            false,
        )?;
        fractions.clone_from_slice(&next);
        Ok(result)
    }
    /// Advect composition with a local fully ionized gas+radiation EOS.
    /// Nuclear reaction metadata supplies species A and Z; reactions are not run.
    /// # Errors
    /// Invalid local EOS/composition, transport failure or exhausted budget.
    pub fn step_composition_ionized(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        let mut next = fractions.to_vec();
        let result = self.step_composition_impl(
            dt,
            max_step,
            budget,
            None,
            Some(&mut next),
            Some(network),
            None,
            None,
            false,
        )?;
        fractions.clone_from_slice(&next);
        Ok(result)
    }
    /// Contact-resolving spherical transport with local gas+radiation EOS.
    /// Optional exterior is a fixed matter reservoir. Gravity work uses actual
    /// mass fluxes. This does not yet reconstruct hydrostatic interface states.
    /// # Errors
    /// Invalid input/state, nonphysical transport or exhausted work budget.
    pub fn step_composition_contact(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        exterior: Option<&CompositionExterior>,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        let external = exterior
            .map(|e| {
                network
                    .mixture(&e.fractions)
                    .map(|mixture| Exterior {
                        cell: e.cell,
                        mixture,
                    })
                    .map_err(Error::Nuclear)
            })
            .transpose()?;
        let mut rows = fractions.to_vec();
        let result = self.step_composition_impl(
            dt,
            max_step,
            budget,
            None,
            Some(&mut rows),
            Some(network),
            external,
            exterior.map(|e| e.fractions.as_slice()),
            true,
        )?;
        fractions.clone_from_slice(&rows);
        Ok(result)
    }
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)] // Keep the transactional gravity/transport step together.
    fn step_composition_impl(
        &mut self,
        dt: f64,
        max_step: f64,
        budget: usize,
        mixture: Option<crate::astrophysics_eos::Mixture>,
        mut fractions: Option<&mut [Vec<f64>]>,
        network: Option<&crate::astrophysics_nuclear::Network>,
        exterior: Option<Exterior>,
        exterior_fractions: Option<&[f64]>,
        contact: bool,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        self.energy()?;
        let external_thermal = exterior
            .map(|e| thermodynamics(e.cell, self.gamma, Some(e.mixture)))
            .transpose()?;
        if exterior.is_some()
            && (self.outer != Boundary::Outflow
                || (fractions.is_some() && exterior_fractions.is_none()))
        {
            return Err(Error::InvalidInput);
        }
        let species = fractions
            .as_ref()
            .map_or(0, |x| x.first().map_or(0, Vec::len));
        if let Some(x) = fractions.as_ref() {
            validate_composition(x, self.cells.len(), species)?;
        }
        let mut escaped_species = vec![0.0; species];
        if !dt.is_finite() || dt < 0.0 || !max_step.is_finite() || max_step <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let mut remaining = dt;
        let mut report = Exchange {
            steps: 0,
            escaped_mass: 0.0,
            escaped_energy: 0.0,
        };
        let geometry = self.geometry()?;
        while remaining > 0.0 {
            if report.steps >= budget {
                return Err(Error::BudgetExceeded);
            }
            let mut speed: f64 = 0.0;
            let mut peak: f64 = 0.0;
            let mut thermal = Vec::with_capacity(next.cells.len());
            for (i, cell) in next.cells.iter().enumerate() {
                let local = local_mixture(mixture, network, fractions.as_deref(), i)?;
                let state = thermodynamics(*cell, next.gamma, local)?;
                speed = speed.max((cell.momentum / cell.density).abs() + state.1);
                thermal.push(state);
                peak = peak.max(cell.density);
            }
            if let (Some(exterior), Some(thermal)) = (exterior, external_thermal) {
                speed =
                    speed.max((exterior.cell.momentum / exterior.cell.density).abs() + thermal.1);
            }
            let h = remaining
                .min(max_step)
                .min(0.2 * next.spacing / speed)
                .min(if next.g == 0.0 {
                    f64::INFINITY
                } else {
                    0.1 / (FOUR_PI * next.g * peak).sqrt()
                });
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let old = next.field()?;
            let first = next.kick(h / 2.0)?;
            let n = next.cells.len();
            let mut flux = radial_fluxes(
                &next.cells,
                &thermal,
                next.gamma,
                next.outer,
                mixture.is_some() || network.is_some(),
                contact,
            )?;
            if let (Some(exterior), Some(state)) = (exterior, external_thermal) {
                flux[n] = if contact {
                    crate::astrophysics_gas::contact_flux_thermal(
                        next.cells[n - 1],
                        exterior.cell,
                        thermal[n - 1],
                        state,
                    )
                    .map_err(Error::Gas)?
                } else {
                    interface_thermal(next.cells[n - 1], exterior.cell, thermal[n - 1], state)
                };
            }
            let old_density: Vec<_> = next.cells.iter().map(|c| c.density).collect();
            let mut mass = vec![0.0; n + 1];
            for (i, (cell, v)) in next.cells.iter_mut().zip(&geometry).enumerate() {
                let p = thermal[i].0;
                let q = [cell.density, cell.momentum, cell.energy];
                let q: [f64; 3] = std::array::from_fn(|j| {
                    q[j] - h / v.volume * (v.right_area * flux[i + 1][j] - v.left_area * flux[i][j])
                });
                *cell = Cell {
                    density: q[0],
                    momentum: q[1] + h * p * (v.right_area - v.left_area) / v.volume,
                    energy: q[2],
                };
                cell.pressure(next.gamma).map_err(Error::Gas)?;
                mass[i + 1] = h * v.right_area * flux[i + 1][0];
            }
            if let Some(rows) = fractions.as_deref_mut() {
                transport_composition(
                    rows,
                    &old_density,
                    &next.cells,
                    &geometry,
                    &mass,
                    &mut escaped_species,
                    exterior_fractions,
                )?;
            }
            let new = next.field()?;
            let second = next.kick(h / 2.0)?;
            let potential: Vec<_> = old
                .potential
                .iter()
                .zip(new.potential)
                .map(|(a, b)| a.midpoint(b))
                .collect();
            let mut work = vec![0.0; n];
            for i in 0..n - 1 {
                let w = -mass[i + 1] * (potential[i + 1] - potential[i]);
                work[i] += w / 2.0;
                work[i + 1] += w / 2.0;
            }
            for (i, cell) in next.cells.iter_mut().enumerate() {
                cell.energy += work[i] / geometry[i].volume - first[i] - second[i];
                cell.pressure(next.gamma).map_err(Error::Gas)?;
            }
            report.escaped_mass += mass[n];
            report.escaped_energy +=
                h * geometry[n - 1].right_area * flux[n][2] + mass[n] * potential[n - 1];
            remaining -= h;
            report.steps += 1;
        }
        next.energy()?;
        for (i, cell) in next.cells.iter().enumerate() {
            let local = local_mixture(mixture, network, fractions.as_deref(), i)?;
            thermodynamics(*cell, next.gamma, local)?;
        }
        if ![report.escaped_mass, report.escaped_energy]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        *self = next;
        Ok((report, escaped_species))
    }
}

fn thermodynamics(
    cell: Cell,
    gamma: f64,
    mixture: Option<crate::astrophysics_eos::Mixture>,
) -> Result<(f64, f64), Error> {
    let internal = cell.pressure(gamma).map_err(Error::Gas)? / (gamma - 1.0);
    if let Some(mixture) = mixture {
        let temperature = mixture
            .temperature(cell.density, internal)
            .map_err(Error::Eos)?;
        let state = mixture.at(cell.density, temperature).map_err(Error::Eos)?;
        if state.sound_speed_squared >= crate::astrophysics_eos::LIGHT_SPEED.powi(2) {
            return Err(Error::InvalidInput);
        }
        Ok((
            state.gas_pressure + state.radiation_pressure,
            state.sound_speed_squared.sqrt(),
        ))
    } else {
        let pressure = cell.pressure(gamma).map_err(Error::Gas)?;
        Ok((pressure, (gamma * pressure / cell.density).sqrt()))
    }
}
fn interface_thermal(left: Cell, right: Cell, l: (f64, f64), r: (f64, f64)) -> [f64; 3] {
    let ul = left.momentum / left.density;
    let ur = right.momentum / right.density;
    let speed = (ul.abs() + l.1).max(ur.abs() + r.1);
    let fl = [
        left.momentum,
        left.momentum * ul + l.0,
        (left.energy + l.0) * ul,
    ];
    let fr = [
        right.momentum,
        right.momentum * ur + r.0,
        (right.energy + r.0) * ur,
    ];
    let ql = [left.density, left.momentum, left.energy];
    let qr = [right.density, right.momentum, right.energy];
    std::array::from_fn(|i| 0.5 * (fl[i] + fr[i]) - 0.5 * speed * (qr[i] - ql[i]))
}

fn radial_fluxes(
    cells: &[Cell],
    thermal: &[(f64, f64)],
    gamma: f64,
    outer: Boundary,
    ionized: bool,
    contact: bool,
) -> Result<Vec<[f64; 3]>, Error> {
    let mut flux = vec![[0.0; 3]];
    for (i, pair) in cells.windows(2).enumerate() {
        flux.push(if contact {
            crate::astrophysics_gas::contact_flux_thermal(
                pair[0],
                pair[1],
                thermal[i],
                thermal[i + 1],
            )
            .map_err(Error::Gas)?
        } else if ionized {
            interface_thermal(pair[0], pair[1], thermal[i], thermal[i + 1])
        } else {
            interface(pair[0], pair[1], gamma).map_err(Error::Gas)?
        });
    }
    let last = cells[cells.len() - 1];
    let outside = if outer == Boundary::Reflecting {
        Cell {
            momentum: -last.momentum,
            ..last
        }
    } else {
        last
    };
    let state = thermal[thermal.len() - 1];
    flux.push(if contact {
        crate::astrophysics_gas::contact_flux_thermal(last, outside, state, state)
            .map_err(Error::Gas)?
    } else if ionized {
        interface_thermal(last, outside, state, state)
    } else {
        interface(last, outside, gamma).map_err(Error::Gas)?
    });
    Ok(flux)
}

fn validate_composition(x: &[Vec<f64>], cells: usize, species: usize) -> Result<(), Error> {
    if x.len() != cells
        || species == 0
        || x.iter().any(|row| {
            row.len() != species
                || row.iter().any(|v| !v.is_finite() || *v < 0.0)
                || (row.iter().sum::<f64>() - 1.0).abs() > 1e-10
        })
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
fn transport_composition(
    rows: &mut [Vec<f64>],
    old_density: &[f64],
    cells: &[Cell],
    geometry: &[Geometry],
    mass: &[f64],
    escaped_species: &mut [f64],
    exterior_fractions: Option<&[f64]>,
) -> Result<(), Error> {
    let n = cells.len();
    let species = escaped_species.len();
    let mut transported = vec![vec![0.0; species]; n + 1];
    for face in 1..=n {
        let donor = if mass[face] >= 0.0 {
            face - 1
        } else {
            face.min(n - 1)
        };
        let donor_fractions = if face == n && mass[face] < 0.0 {
            exterior_fractions.unwrap_or(&rows[donor])
        } else {
            &rows[donor]
        };
        for (value, fraction) in transported[face].iter_mut().zip(donor_fractions) {
            *value = mass[face] * fraction;
        }
    }
    for i in 0..n {
        for (j, fraction) in rows[i].iter_mut().enumerate() {
            let amount = old_density[i] * geometry[i].volume * *fraction + transported[i][j]
                - transported[i + 1][j];
            *fraction = amount / (cells[i].density * geometry[i].volume);
            if !fraction.is_finite() || *fraction < 0.0 {
                return Err(Error::NumericalOverflow);
            }
        }
    }
    for (total, amount) in escaped_species.iter_mut().zip(&transported[n]) {
        *total += amount;
        if !total.is_finite() {
            return Err(Error::NumericalOverflow);
        }
    }
    Ok(())
}

fn local_mixture(
    uniform: Option<crate::astrophysics_eos::Mixture>,
    network: Option<&crate::astrophysics_nuclear::Network>,
    fractions: Option<&[Vec<f64>]>,
    index: usize,
) -> Result<Option<crate::astrophysics_eos::Mixture>, Error> {
    if let Some(network) = network {
        let row = fractions
            .and_then(|rows| rows.get(index))
            .ok_or(Error::InvalidInput)?;
        Ok(Some(network.mixture(row).map_err(Error::Nuclear)?))
    } else {
        Ok(uniform)
    }
}

/// Integrated reactive spherical step; energies in joules.
#[derive(Clone, Debug, PartialEq)]
pub struct ReactiveExchange {
    pub hydro_steps: usize,
    pub burn_steps: usize,
    pub fit_evaluations: usize,
    pub escaped_species: Vec<f64>,
    /// Gas plus gravitational boundary energy, excluding nuclear binding.
    pub escaped_energy: f64,
    pub escaped_binding: f64,
    pub escaped_neutrinos: f64,
    pub deposited_energy: f64,
}
impl Sphere {
    /// Gas, gravity and nuclear binding reservoir in joules.
    /// # Errors
    /// Invalid geometry, gas, network or composition.
    pub fn reactive_energy(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
    ) -> Result<f64, Error> {
        if fractions.len() != self.cells.len() {
            return Err(Error::InvalidInput);
        }
        let geometry = self.geometry()?;
        let mut energy = self.energy()?;
        for ((cell, shell), row) in self.cells.iter().zip(geometry).zip(fractions) {
            energy +=
                cell.density * shell.volume * network.reservoir(row).map_err(Error::Nuclear)?;
        }
        if !energy.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        Ok(energy)
    }
    /// One split interval: half hydrodynamics, isochoric local burn, half
    /// hydrodynamics. All phases share their respective work budgets and commit
    /// atomically. Caller must refine dt as well as burn `max_step` for convergence.
    /// # Errors
    /// Invalid state, local fit/EOS failure, transport failure or budget exhaustion.
    pub fn step_reactive(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        hydro_max_step: f64,
        hydro_budget: usize,
        burn_budget: crate::astrophysics_nuclear::Budget,
    ) -> Result<ReactiveExchange, Error> {
        self.step_reactive_impl(
            fractions,
            network,
            dt,
            ReactiveBudget {
                hydro_max_step,
                hydro_steps: hydro_budget,
                burn: burn_budget,
            },
            None,
            None,
        )
    }
    /// Reactive dynamics with a fixed exterior composition reservoir.
    /// # Errors
    /// Any local burning, boundary, composition, work-budget or EOS failure.
    pub fn step_reactive_exterior(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        budget: ReactiveBudget,
        exterior: &CompositionExterior,
    ) -> Result<ReactiveExchange, Error> {
        self.step_reactive_impl(fractions, network, dt, budget, Some(exterior), None)
    }
    /// Reactive evolution using an immutable hydrostatic reference.
    /// The reference is not updated by burning or transport.
    /// # Errors
    /// Invalid reference/state, burning failure or shared budget exhaustion.
    #[allow(clippy::too_many_arguments)]
    pub fn step_reactive_balanced(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        budget: ReactiveBudget,
        reference: &HydrostaticReference,
        exterior: Option<&CompositionExterior>,
    ) -> Result<ReactiveExchange, Error> {
        self.step_reactive_impl(fractions, network, dt, budget, exterior, Some(reference))
    }
    #[allow(clippy::too_many_arguments)]
    fn step_reactive_impl(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        budget: ReactiveBudget,
        exterior: Option<&CompositionExterior>,
        reference: Option<&HydrostaticReference>,
    ) -> Result<ReactiveExchange, Error> {
        let hydro_max_step = budget.hydro_max_step;
        let hydro_budget = budget.hydro_steps;
        let burn_budget = budget.burn;
        let hydro = |sphere: &mut Self, rows: &mut [Vec<f64>], h, steps| {
            if let Some(reference) = reference {
                if let Some(exterior) = exterior {
                    sphere.step_composition_balanced_exterior(
                        rows,
                        network,
                        h,
                        hydro_max_step,
                        steps,
                        reference,
                        exterior,
                    )
                } else {
                    sphere
                        .step_composition_balanced(
                            rows,
                            network,
                            h,
                            hydro_max_step,
                            steps,
                            reference,
                        )
                        .map(|report| (report, vec![0.0; network.nuclei.len()]))
                }
            } else if let Some(exterior) = exterior {
                sphere.step_composition_exterior(rows, network, h, hydro_max_step, steps, exterior)
            } else {
                sphere.step_composition_ionized(rows, network, h, hydro_max_step, steps)
            }
        };
        let mut next = self.clone();
        let mut rows = fractions.to_vec();
        let (first, mut escaped_species) = hydro(&mut next, &mut rows, dt / 2.0, hydro_budget)?;
        let geometry = next.geometry()?;
        let mut result = ReactiveExchange {
            hydro_steps: first.steps,
            burn_steps: 0,
            fit_evaluations: 0,
            escaped_species: Vec::new(),
            escaped_energy: first.escaped_energy,
            escaped_binding: 0.0,
            escaped_neutrinos: 0.0,
            deposited_energy: 0.0,
        };
        for ((cell, shell), row) in next.cells.iter_mut().zip(&geometry).zip(&mut rows) {
            let kinetic = 0.5 * cell.momentum.powi(2) / cell.density;
            let mut energy = (cell.energy - kinetic) / cell.density;
            let burn = network
                .burn_isochoric(
                    row,
                    cell.density,
                    &mut energy,
                    dt,
                    crate::astrophysics_nuclear::Budget {
                        max_step: burn_budget.max_step,
                        steps: burn_budget.steps - result.burn_steps,
                        fit_evaluations: burn_budget.fit_evaluations - result.fit_evaluations,
                    },
                )
                .map_err(Error::Nuclear)?;
            cell.energy = cell.density * energy + kinetic;
            let mass = cell.density * shell.volume;
            result.escaped_neutrinos += mass * burn.escaped_neutrinos;
            result.deposited_energy += mass * burn.deposited_energy;
            result.burn_steps += burn.steps;
            result.fit_evaluations += burn.fit_evaluations;
        }
        let (second, escaped) = hydro(&mut next, &mut rows, dt / 2.0, hydro_budget - first.steps)?;
        result.hydro_steps += second.steps;
        result.escaped_energy += second.escaped_energy;
        for (mass, value) in escaped_species.iter_mut().zip(escaped) {
            *mass += value;
        }
        for (mass, nucleus) in escaped_species.iter().zip(&network.nuclei) {
            result.escaped_binding -=
                mass * 1000.0 * crate::astrophysics_nuclear::AVOGADRO * nucleus.binding_energy
                    / f64::from(nucleus.mass_number);
        }
        result.escaped_species = escaped_species;
        if ![
            result.escaped_energy,
            result.escaped_binding,
            result.escaped_neutrinos,
            result.deposited_energy,
        ]
        .into_iter()
        .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        next.reactive_energy(&rows, network)?;
        *self = next;
        fractions.clone_from_slice(&rows);
        Ok(result)
    }
}

impl Sphere {
    /// Static pressure-plus-gravity residual acceleration in m/s² per shell.
    /// Uses the same averaged face pressure and spherical geometric source as
    /// the zero-velocity Euler momentum update. Outer face pressure equals the
    /// last shell pressure, matching the existing outer boundary closure.
    /// Does not include velocity advection or radiation momentum forces.
    /// # Errors
    /// Invalid gas, composition, local EOS or gravitational geometry.
    pub fn hydrostatic_residual(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
    ) -> Result<Vec<f64>, Error> {
        self.hydrostatic_residual_impl(fractions, network, None)
    }
    /// Zero-velocity force residual with a fixed exterior's thermal pressure.
    /// Excludes advective momentum flux; the reservoir is not self-gravitating.
    /// # Errors
    /// Invalid exterior/composition, incompatible boundary or local EOS failure.
    pub fn hydrostatic_residual_exterior(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        exterior: &CompositionExterior,
    ) -> Result<Vec<f64>, Error> {
        if self.outer != Boundary::Outflow {
            return Err(Error::InvalidInput);
        }
        let mixture = network
            .mixture(&exterior.fractions)
            .map_err(Error::Nuclear)?;
        let pressure = thermodynamics(exterior.cell, self.gamma, Some(mixture))?.0;
        self.hydrostatic_residual_impl(fractions, network, Some(pressure))
    }
    fn hydrostatic_residual_impl(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        exterior_pressure: Option<f64>,
    ) -> Result<Vec<f64>, Error> {
        if fractions.len() != self.cells.len() {
            return Err(Error::InvalidInput);
        }
        let geometry = self.geometry()?;
        let field = self.field()?;
        let pressures = self
            .cells
            .iter()
            .zip(fractions)
            .map(|(cell, row)| {
                let mixture = network.mixture(row).map_err(Error::Nuclear)?;
                thermodynamics(*cell, self.gamma, Some(mixture)).map(|state| state.0)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let count = pressures.len();
        self.cells
            .iter()
            .zip(geometry)
            .enumerate()
            .map(|(i, (cell, shell))| {
                let left = if i == 0 {
                    pressures[i]
                } else {
                    pressures[i - 1].midpoint(pressures[i])
                };
                let right = if i + 1 == count {
                    exterior_pressure.map_or(pressures[i], |p| pressures[i].midpoint(p))
                } else {
                    pressures[i].midpoint(pressures[i + 1])
                };
                let force = shell.right_area * (right - pressures[i])
                    - shell.left_area * (left - pressures[i]);
                let acceleration = field.acceleration[i] - force / shell.volume / cell.density;
                if acceleration.is_finite() {
                    Ok(acceleration)
                } else {
                    Err(Error::NumericalOverflow)
                }
            })
            .collect()
    }
}

/// Consistent continuum hydrostatic pressures for piecewise constant densities.
#[derive(Clone, Debug, PartialEq)]
pub struct HydrostaticPressures {
    pub cells: Vec<f64>,
    /// Face pressures, including centre and prescribed outer surface.
    pub faces: Vec<f64>,
}
impl Sphere {
    /// Initialize gas from volume-averaged Lane–Emden density and pressure.
    /// The radial grid must lie inside the profile. Scaling must agree with G.
    /// Local composition sets the gas+radiation EOS; momentum is zeroed.
    /// Atomic. Thermal balance and discrete hydrostatic balance are not imposed.
    /// # Errors
    /// Invalid scaling/grid/composition, quadrature failure, EOS or overflow.
    pub fn initialize_polytrope(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        profile: &crate::astrophysics_star::Profile,
        scaling: crate::astrophysics_star::Scaling,
        intervals: usize,
    ) -> Result<(), Error> {
        self.geometry()?;
        let expected = crate::astrophysics_star::Scaling::new(
            profile.index,
            scaling.central_density,
            scaling.central_pressure,
            self.g,
        )
        .map_err(Error::Stellar)?;
        if fractions.len() != self.cells.len()
            || !scaling.length.is_finite()
            || scaling.length <= 0.0
            || (scaling.length / expected.length - 1.0).abs() > 1e-12
        {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let mut inner = 0.0;
        for (cell, row) in next.cells.iter_mut().zip(fractions) {
            let outer = inner + self.spacing;
            let average = profile
                .shell_average(inner / scaling.length, outer / scaling.length, intervals)
                .map_err(Error::Stellar)?;
            let density = scaling.central_density * average.density_over_central;
            let pressure = scaling.central_pressure * average.pressure_over_central;
            let mixture = network.mixture(row).map_err(Error::Nuclear)?;
            let temperature = mixture
                .temperature_from_pressure(density, pressure)
                .map_err(Error::Eos)?;
            cell.density = density;
            cell.momentum = 0.0;
            cell.energy = mixture
                .at(density, temperature)
                .map_err(Error::Eos)?
                .internal_energy_density;
            thermodynamics(*cell, next.gamma, Some(mixture))?;
            inner = outer;
        }
        next.energy()?;
        *self = next;
        Ok(())
    }
    /// Integrate continuum hydrostatic balance for piecewise constant shell
    /// densities, assigning exact volume-averaged total pressure in each shell.
    /// Density and composition are prescribed; this does not solve thermal
    /// transport or enforce the numerical outer boundary's force balance.
    /// Momentum is reset to zero. The entire initialization is atomic.
    /// # Errors
    /// Invalid surface pressure/composition, EOS failure or numerical overflow.
    pub fn initialize_hydrostatic(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        surface_pressure: f64,
    ) -> Result<(), Error> {
        if fractions.len() != self.cells.len() {
            return Err(Error::InvalidInput);
        }
        let pressures = self.hydrostatic_pressures(surface_pressure)?;
        let mut next = self.clone();
        for ((cell, row), pressure) in next.cells.iter_mut().zip(fractions).zip(pressures.cells) {
            let mixture = network.mixture(row).map_err(Error::Nuclear)?;
            let temperature = mixture
                .temperature_from_pressure(cell.density, pressure)
                .map_err(Error::Eos)?;
            cell.momentum = 0.0;
            cell.energy = mixture
                .at(cell.density, temperature)
                .map_err(Error::Eos)?
                .internal_energy_density;
            thermodynamics(*cell, next.gamma, Some(mixture))?;
        }
        next.energy()?;
        *self = next;
        Ok(())
    }
    /// Integrate face and volume-average pressures from the same shell model.
    /// The given density profile is fixed; this does not change gas state.
    /// # Errors
    /// Invalid geometry/surface pressure or numerical overflow.
    pub fn hydrostatic_pressures(
        &self,
        surface_pressure: f64,
    ) -> Result<HydrostaticPressures, Error> {
        let geometry = self.geometry()?;
        if !surface_pressure.is_finite() || surface_pressure <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let mut enclosed = Vec::with_capacity(self.cells.len());
        let mut mass = 0.0;
        for (cell, geo) in self.cells.iter().zip(&geometry) {
            enclosed.push(mass);
            mass += cell.density * geo.volume;
        }
        let mut cells = vec![0.0; self.cells.len()];
        let mut faces = vec![0.0; self.cells.len() + 1];
        faces[self.cells.len()] = surface_pressure;
        let h = self.spacing;
        let mut b = self.cells.iter().fold(0.0, |r, _| r + h);
        let mut edge_pressure = surface_pressure;
        for i in (0..self.cells.len()).rev() {
            let a = if i == 0 { 0.0 } else { b - h };
            let density = self.cells[i].density;
            let c = FOUR_PI * density / 3.0;
            // Positive thickness polynomials avoid subtracting powers of radii.
            let kernel = h * h * (b + 2.0 * a) / (2.0 * b);
            let self_integral = h.powi(3) / b
                * (3.0 * a.powi(3) + 3.0 * a * a * h + 1.2 * a * h * h + 0.2 * h.powi(3));
            let pressure = edge_pressure
                + self.g * density * (enclosed[i] * kernel + c * self_integral)
                    / geometry[i].delta_cube;
            let central_term = if i == 0 {
                0.0
            } else {
                enclosed[i] * h / (a * b)
            };
            edge_pressure += self.g * density * (central_term + c * kernel);
            if !pressure.is_finite() || !edge_pressure.is_finite() {
                return Err(Error::NumericalOverflow);
            }
            cells[i] = pressure;
            faces[i] = edge_pressure;
            b = a;
        }
        Ok(HydrostaticPressures { cells, faces })
    }
    /// Initialize the continuum constant-density hydrostatic pressure profile
    /// `P(r)=P_surface+(2*pi/3)*G*rho²*(R²-r²)`, using shell-averaged pressure.
    /// Local EOS determines temperature and energy; momentum is reset to zero.
    /// Atomic. The existing outer boundary closure is not changed, so its
    /// residual remains; this is not an exactly well-balanced discrete solution.
    /// # Errors
    /// Nonuniform density, invalid surface pressure/composition, EOS or overflow.
    pub fn initialize_uniform_hydrostatic(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        surface_pressure: f64,
    ) -> Result<(), Error> {
        self.geometry()?;
        if fractions.len() != self.cells.len()
            || !surface_pressure.is_finite()
            || surface_pressure <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        let density = self.cells[0].density;
        if self
            .cells
            .iter()
            .any(|c| c.density.to_bits() != density.to_bits())
        {
            return Err(Error::InvalidInput);
        }
        let radius = self.cells.iter().fold(0.0, |r, _| r + self.spacing);
        let coefficient =
            (2.0 * std::f64::consts::PI / 3.0) * self.g * density * density * radius * radius;
        if !coefficient.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        let mut next = self.clone();
        let mut inner = 0.0;
        for (cell, row) in next.cells.iter_mut().zip(fractions) {
            let outer = inner + self.spacing;
            let a = inner / radius;
            let b = outer / radius;
            let mean_r2 = 0.6
                * (b.powi(4) + b.powi(3) * a + b * b * a * a + b * a.powi(3) + a.powi(4))
                / (b * b + b * a + a * a);
            let pressure = surface_pressure + coefficient * (1.0 - mean_r2);
            let mixture = network.mixture(row).map_err(Error::Nuclear)?;
            let temperature = mixture
                .temperature_from_pressure(density, pressure)
                .map_err(Error::Eos)?;
            cell.momentum = 0.0;
            cell.energy = mixture
                .at(density, temperature)
                .map_err(Error::Eos)?
                .internal_energy_density;
            thermodynamics(*cell, next.gamma, Some(mixture))?;
            inner = outer;
        }
        next.energy()?;
        *self = next;
        Ok(())
    }
}

/// Terms of the scalar virial relation, in joules. For a static sphere with
/// prescribed uniform surface pressure their sum vanishes in equilibrium.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Virial {
    pub twice_kinetic: f64,
    pub pressure: f64,
    pub gravity: f64,
    pub surface: f64,
}
impl Virial {
    #[must_use]
    pub fn balance(self) -> f64 {
        self.twice_kinetic + self.pressure + self.gravity + self.surface
    }
}
impl Sphere {
    /// Scalar virial terms `2K + 3 integral(P dV) + W - 3 P_surface V`.
    /// Uses total local EOS pressure. Surface pressure is supplied explicitly;
    /// open-boundary mass/momentum flux terms are not included in this diagnostic.
    /// # Errors
    /// Invalid gas/composition, negative surface pressure or numerical overflow.
    pub fn virial(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        surface_pressure: f64,
    ) -> Result<Virial, Error> {
        if fractions.len() != self.cells.len()
            || !surface_pressure.is_finite()
            || surface_pressure < 0.0
        {
            return Err(Error::InvalidInput);
        }
        let geometry = self.geometry()?;
        let mut report = Virial {
            twice_kinetic: 0.0,
            pressure: 0.0,
            gravity: self.field()?.energy,
            surface: 0.0,
        };
        for ((cell, shell), row) in self.cells.iter().zip(geometry).zip(fractions) {
            let mixture = network.mixture(row).map_err(Error::Nuclear)?;
            report.twice_kinetic += cell.momentum.powi(2) / cell.density * shell.volume;
            report.pressure +=
                3.0 * thermodynamics(*cell, self.gamma, Some(mixture))?.0 * shell.volume;
            report.surface -= 3.0 * surface_pressure * shell.volume;
        }
        if ![
            report.twice_kinetic,
            report.pressure,
            report.gravity,
            report.surface,
            report.balance(),
        ]
        .into_iter()
        .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        Ok(report)
    }
}

/// Fixed exterior reservoir state; its mass is excluded from sphere gravity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exterior {
    pub cell: Cell,
    pub mixture: crate::astrophysics_eos::Mixture,
}
impl Sphere {
    /// Uniform ionized EOS dynamics with a fixed exterior reservoir at the open
    /// outer face. Incoming and outgoing mass/energy share the boundary ledger.
    /// Exterior sound speed participates in the CFL limit. Atomic.
    /// # Errors
    /// Non-open boundary, invalid exterior/EOS, budget or numerical failure.
    pub fn step_ionized_exterior(
        &mut self,
        dt: f64,
        max_step: f64,
        budget: usize,
        mixture: crate::astrophysics_eos::Mixture,
        exterior: Exterior,
    ) -> Result<Exchange, Error> {
        self.step_composition_impl(
            dt,
            max_step,
            budget,
            Some(mixture),
            None,
            None,
            Some(exterior),
            None,
            false,
        )
        .map(|r| r.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompositionExterior {
    pub cell: Cell,
    pub fractions: Vec<f64>,
}
impl Sphere {
    /// Composition-dependent open boundary with explicit inflowing species.
    /// Exterior EOS is rebuilt from the same network and exterior fractions.
    /// Returns signed mass escaping for each species. Atomic.
    /// # Errors
    /// Invalid exterior/composition, boundary, EOS, budget or transport failure.
    pub fn step_composition_exterior(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        exterior: &CompositionExterior,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        let mixture = network
            .mixture(&exterior.fractions)
            .map_err(Error::Nuclear)?;
        let mut rows = fractions.to_vec();
        let result = self.step_composition_impl(
            dt,
            max_step,
            budget,
            None,
            Some(&mut rows),
            Some(network),
            Some(Exterior {
                cell: exterior.cell,
                mixture,
            }),
            Some(&exterior.fractions),
            false,
        )?;
        fractions.clone_from_slice(&rows);
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactiveBudget {
    pub hydro_max_step: f64,
    pub hydro_steps: usize,
    pub burn: crate::astrophysics_nuclear::Budget,
}

/// Immutable hydrostatic reference for perturbation reconstruction.
#[derive(Clone, Debug)]
pub struct HydrostaticReference {
    sphere: Sphere,
    pressures: HydrostaticPressures,
    cell_pressures: Vec<f64>,
    nuclei: Vec<crate::astrophysics_nuclear::Nucleus>,
    exterior_pressure: Option<f64>,
}
impl Sphere {
    /// Initialize a confined hydrostatic sphere and retain a consistent reference.
    /// This first equilibrium-preserving path requires a reflecting outer wall.
    /// # Errors
    /// Invalid boundary, composition, pressure, geometry or EOS.
    pub fn initialize_balanced(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        surface_pressure: f64,
    ) -> Result<HydrostaticReference, Error> {
        self.initialize_balanced_impl(fractions, network, surface_pressure, None)
    }
    /// Initialize an open equilibrium with a stationary exterior matching surface pressure.
    /// # Errors
    /// Invalid/mismatched reservoir, geometry, composition or EOS.
    pub fn initialize_balanced_exterior(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        surface_pressure: f64,
        exterior: &CompositionExterior,
    ) -> Result<HydrostaticReference, Error> {
        self.initialize_balanced_impl(fractions, network, surface_pressure, Some(exterior))
    }
    fn initialize_balanced_impl(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        surface_pressure: f64,
        exterior: Option<&CompositionExterior>,
    ) -> Result<HydrostaticReference, Error> {
        if (self.outer == Boundary::Reflecting) != exterior.is_none()
            || self.outer == Boundary::Periodic
        {
            return Err(Error::InvalidInput);
        }
        let exterior_pressure = exterior
            .map(|e| {
                let pressure = thermodynamics(
                    e.cell,
                    self.gamma,
                    Some(network.mixture(&e.fractions).map_err(Error::Nuclear)?),
                )?
                .0;
                if e.cell.momentum != 0.0 || (pressure / surface_pressure - 1.0).abs() > 1e-12 {
                    return Err(Error::InvalidInput);
                }
                Ok(pressure)
            })
            .transpose()?;
        let pressures = self.hydrostatic_pressures(surface_pressure)?;
        let mut next = self.clone();
        next.initialize_hydrostatic(fractions, network, surface_pressure)?;
        let cell_pressures = next
            .cells
            .iter()
            .zip(fractions)
            .map(|(c, row)| {
                thermodynamics(
                    *c,
                    next.gamma,
                    Some(network.mixture(row).map_err(Error::Nuclear)?),
                )
                .map(|t| t.0)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let reference = HydrostaticReference {
            sphere: next.clone(),
            pressures,
            cell_pressures,
            nuclei: network.nuclei.clone(),
            exterior_pressure,
        };
        *self = next;
        Ok(reference)
    }
    /// Evolve perturbations with reference-pressure reconstruction and a matching
    /// momentum source. Mass-flux gravity work conserves gas plus binding energy.
    /// State and composition roll back together on every failure.
    /// # Errors
    /// Incompatible reference, invalid reconstruction/state or exhausted budget.
    #[allow(clippy::too_many_lines)] // Atomic reconstruction/transport/energy ledger.
    pub fn step_composition_balanced(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        reference: &HydrostaticReference,
    ) -> Result<Exchange, Error> {
        self.step_composition_balanced_impl(
            fractions, network, dt, max_step, budget, reference, None,
        )
        .map(|r| r.0)
    }
    /// Balanced open transport with signed gas/gravity and species boundary ledgers.
    /// # Errors
    /// Invalid/mismatched reference, reservoir, reconstruction or work budget.
    #[allow(clippy::too_many_arguments)]
    pub fn step_composition_balanced_exterior(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        reference: &HydrostaticReference,
        exterior: &CompositionExterior,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        self.step_composition_balanced_impl(
            fractions,
            network,
            dt,
            max_step,
            budget,
            reference,
            Some(exterior),
        )
    }
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn step_composition_balanced_impl(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        reference: &HydrostaticReference,
        exterior: Option<&CompositionExterior>,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        let base = &reference.sphere;
        if self.outer != base.outer
            || exterior.is_some() != reference.exterior_pressure.is_some()
            || self.cells.len() != base.cells.len()
            || self.spacing != base.spacing
            || self.g != base.g
            || self.gamma != base.gamma
            || network.nuclei != reference.nuclei
            || fractions.len() != self.cells.len()
            || !dt.is_finite()
            || dt < 0.0
            || !max_step.is_finite()
            || max_step <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        if let Some(e) = exterior {
            thermodynamics(
                e.cell,
                self.gamma,
                Some(network.mixture(&e.fractions).map_err(Error::Nuclear)?),
            )?;
        }
        let geometry = self.geometry()?;
        let base_field = base.field()?;
        let mut next = self.clone();
        let mut rows = fractions.to_vec();
        for (cell, row) in next.cells.iter().zip(&rows) {
            thermodynamics(
                *cell,
                next.gamma,
                Some(network.mixture(row).map_err(Error::Nuclear)?),
            )?;
        }
        let mut remaining = dt;
        let mut report = Exchange {
            steps: 0,
            escaped_mass: 0.0,
            escaped_energy: 0.0,
        };
        let mut escaped = vec![0.0; network.nuclei.len()];
        while remaining > 0.0 {
            if report.steps >= budget {
                return Err(Error::BudgetExceeded);
            }
            let n = next.cells.len();
            let old_field = next.field()?;
            let mut pressure_delta = Vec::with_capacity(n);
            let mut states = Vec::with_capacity(n);
            let mut speed = 0.0_f64;
            let mut peak = 0.0_f64;
            for (i, (cell, row)) in next.cells.iter().zip(&rows).enumerate() {
                let mixture = network.mixture(row).map_err(Error::Nuclear)?;
                let pressure = thermodynamics(*cell, next.gamma, Some(mixture))?.0;
                let delta = pressure - reference.cell_pressures[i];
                pressure_delta.push(delta);
                let mut faces = Vec::with_capacity(2);
                for face in [i, i + 1] {
                    let p = reference.pressures.faces[face] + delta;
                    let t = mixture
                        .temperature_from_pressure(cell.density, p)
                        .map_err(Error::Eos)?;
                    let thermal = mixture.at(cell.density, t).map_err(Error::Eos)?;
                    let velocity = cell.momentum / cell.density;
                    let reconstructed = Cell {
                        density: cell.density,
                        momentum: cell.momentum,
                        energy: thermal.internal_energy_density
                            + 0.5 * cell.density * velocity * velocity,
                    };
                    let thermo = thermodynamics(reconstructed, next.gamma, Some(mixture))?;
                    speed = speed.max(velocity.abs() + thermo.1);
                    faces.push((reconstructed, (p, thermo.1)));
                }
                states.push(faces);
                peak = peak.max(cell.density);
            }
            let mut flux = vec![[0.0; 3]; n + 1];
            for i in 1..n {
                flux[i] = crate::astrophysics_gas::contact_flux_thermal(
                    states[i - 1][1].0,
                    states[i][0].0,
                    states[i - 1][1].1,
                    states[i][0].1,
                )
                .map_err(Error::Gas)?;
            }
            let (last, thermo) = states[n - 1][1];
            if let Some(exterior) = exterior {
                let mixture = network
                    .mixture(&exterior.fractions)
                    .map_err(Error::Nuclear)?;
                let external = thermodynamics(exterior.cell, next.gamma, Some(mixture))?;
                let pressure = reference.pressures.faces[n]
                    + (external.0 - reference.exterior_pressure.ok_or(Error::InvalidInput)?);
                let t = mixture
                    .temperature_from_pressure(exterior.cell.density, pressure)
                    .map_err(Error::Eos)?;
                let velocity = exterior.cell.momentum / exterior.cell.density;
                let outside = Cell {
                    energy: mixture
                        .at(exterior.cell.density, t)
                        .map_err(Error::Eos)?
                        .internal_energy_density
                        + 0.5 * exterior.cell.density * velocity * velocity,
                    ..exterior.cell
                };
                let sound = thermodynamics(outside, next.gamma, Some(mixture))?.1;
                speed = speed.max(velocity.abs() + sound);
                flux[n] = crate::astrophysics_gas::contact_flux_thermal(
                    last,
                    outside,
                    thermo,
                    (pressure, sound),
                )
                .map_err(Error::Gas)?;
            } else {
                let reflected = Cell {
                    momentum: -last.momentum,
                    ..last
                };
                flux[n] =
                    crate::astrophysics_gas::contact_flux_thermal(last, reflected, thermo, thermo)
                        .map_err(Error::Gas)?;
                flux[n][0] = 0.0;
                flux[n][2] = 0.0;
            }
            let h = remaining
                .min(max_step)
                .min(0.2 * next.spacing / speed)
                .min(if next.g == 0.0 {
                    f64::INFINITY
                } else {
                    0.1 / (FOUR_PI * next.g * peak).sqrt()
                });
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let old_density: Vec<_> = next.cells.iter().map(|c| c.density).collect();
            let mut mass = vec![0.0; n + 1];
            for (i, (cell, geo)) in next.cells.iter_mut().zip(&geometry).enumerate() {
                let q = [cell.density, cell.momentum, cell.energy];
                let divergence: [f64; 3] = std::array::from_fn(|j| {
                    geo.right_area * flux[i + 1][j] - geo.left_area * flux[i][j]
                });
                let reference_force = geo.right_area * reference.pressures.faces[i + 1]
                    - geo.left_area * reference.pressures.faces[i];
                let source_difference = pressure_delta[i] * (geo.right_area - geo.left_area)
                    + geo.volume
                        * (cell.density * old_field.acceleration[i]
                            - base.cells[i].density * base_field.acceleration[i]);
                cell.density = q[0] - h * divergence[0] / geo.volume;
                cell.momentum =
                    q[1] + h * (reference_force - divergence[1] + source_difference) / geo.volume;
                cell.energy = q[2] - h * divergence[2] / geo.volume;
                cell.pressure(next.gamma).map_err(Error::Gas)?;
                mass[i + 1] = h * geo.right_area * flux[i + 1][0];
            }
            transport_composition(
                &mut rows,
                &old_density,
                &next.cells,
                &geometry,
                &mass,
                &mut escaped,
                exterior.map(|e| e.fractions.as_slice()),
            )?;
            let new_field = next.field()?;
            let potential: Vec<_> = old_field
                .potential
                .iter()
                .zip(new_field.potential)
                .map(|(a, b)| a.midpoint(b))
                .collect();
            for i in 0..n - 1 {
                let work = -mass[i + 1] * (potential[i + 1] - potential[i]) / 2.0;
                next.cells[i].energy += work / geometry[i].volume;
                next.cells[i + 1].energy += work / geometry[i + 1].volume;
            }
            for (cell, row) in next.cells.iter().zip(&rows) {
                thermodynamics(
                    *cell,
                    next.gamma,
                    Some(network.mixture(row).map_err(Error::Nuclear)?),
                )?;
            }
            report.escaped_mass += mass[n];
            report.escaped_energy +=
                h * geometry[n - 1].right_area * flux[n][2] + mass[n] * potential[n - 1];
            remaining -= h;
            report.steps += 1;
        }
        next.energy()?;
        if ![report.escaped_mass, report.escaped_energy]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        *self = next;
        fractions.clone_from_slice(&rows);
        Ok((report, escaped))
    }
}
