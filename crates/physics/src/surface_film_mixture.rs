//! Conservative equal-density component transport in a thin surface film.
use super::{FilmRheology, SlidingAdvanceReport, SlidingPatch, SurfaceFilm};
use crate::liquid::ThermalTranslatingBody;

/// Owns the film so volume cannot change without its component inventories.
/// All components share the film's constant density and surface tension.
/// Newtonian viscosity can depend on composition through a logarithmic blend.
/// Mass is canonical; volume is derived at the current constant density.
/// Variable-density EOS and temperature are not yet coupled to mechanics.
/// Diffusion is a separate equal-diffusivity composition stage.
#[derive(Clone, Debug)]
pub struct FilmMixture {
    film: SurfaceFilm,
    names: Vec<String>,
    component_mass: Vec<Vec<f64>>,
    component_volume_cache: Vec<Vec<f64>>,
    component_viscosities: Option<Vec<f64>>,
}
/// Actual extensive inventory transferred across the film boundary (SI units).
/// The caller balances its other reservoir; no heat or momentum law is implied.
#[derive(Clone, Debug, PartialEq)]
pub struct FilmTransfer {
    pub volume_m3: f64,
    pub mass_kg: f64,
    pub component_volumes_m3: Vec<f64>,
    pub component_masses_kg: Vec<f64>,
}
/// Compatibility name for an outward inventory transfer.
pub type FilmWithdrawal = FilmTransfer;
impl FilmMixture {
    /// Fractions are per triangle, including initially dry cells. Rows must be
    /// nonnegative and sum to one; accepted rounding error is normalized once.
    pub fn new(
        film: SurfaceFilm,
        names: Vec<String>,
        fractions: Vec<Vec<f64>>,
    ) -> Result<Self, &'static str> {
        if !film.total_mass().is_finite()
            || names.is_empty()
            || names.len() > 64
            || fractions.len() != film.volume.len()
            || names
                .iter()
                .enumerate()
                .any(|(i, n)| n.is_empty() || names[..i].contains(n))
        {
            return Err("invalid film component schema");
        }
        let mut component_mass = Vec::with_capacity(fractions.len());
        for (fraction, volume) in fractions.iter().zip(&film.volume) {
            let fraction = Self::normalized(fraction, names.len())?;
            let mut row = Vec::with_capacity(names.len());
            for f in fraction {
                let mass = (volume * film.material.density) * f;
                if !mass.is_finite() || (*volume > 0. && f > 0. && mass == 0.) {
                    return Err("film component mass cannot be represented");
                }
                row.push(mass);
            }
            component_mass.push(row);
        }
        let mut state = Self {
            film,
            names,
            component_mass,
            component_volume_cache: Vec::new(),
            component_viscosities: None,
        };
        state.refresh_volumes()?;
        Ok(state)
    }

    fn normalized(fraction: &[f64], count: usize) -> Result<Vec<f64>, &'static str> {
        let sum = fraction.iter().sum::<f64>();
        if fraction.len() != count
            || fraction.iter().any(|f| !f.is_finite() || *f < 0.0)
            || !sum.is_finite()
            || (sum - 1.0).abs() > 1e-12
        {
            return Err("invalid film component fractions");
        }
        Ok(fraction.iter().map(|f| f / sum).collect())
    }
    /// Enable an illustrative logarithmic Newtonian blend mu=exp(sum(Y*ln(mu_k))).
    /// Every component viscosity is positive, finite and in Pa s. None restores
    /// the original uniform film viscosity. Configuration changes no inventories.
    pub fn configure_viscosities(&mut self, values: Option<Vec<f64>>) -> Result<(), &'static str> {
        if let Some(values) = &values {
            if values.len() != self.names.len()
                || values.iter().any(|v| !v.is_finite() || *v <= 0.0)
            {
                return Err("invalid film component viscosities");
            }
        }
        self.component_viscosities = values;
        Ok(())
    }
    /// Current per-cell Newtonian viscosity. Dry cells use the base material.
    pub fn effective_viscosities(&self) -> Vec<f64> {
        self.component_viscosities.as_ref().map_or_else(
            || vec![self.film.material.viscosity; self.component_mass.len()],
            |values| blend_viscosities(&self.component_mass, values, self.film.material.viscosity),
        )
    }
    /// Refresh substrate geometry at fixed canonical component mass and volume.
    /// This supplies no mechanical work law for a deforming wet surface.
    pub fn update_geometry(&mut self, points: &[[f64; 3]]) -> Result<(), &'static str> {
        self.film.update_geometry(points)
    }
    pub fn film(&self) -> &SurfaceFilm {
        &self.film
    }
    pub fn component_names(&self) -> &[String] {
        &self.names
    }
    /// Dry cells have no composition and return an all-zero row.
    pub fn fractions(&self) -> Vec<Vec<f64>> {
        self.component_mass
            .iter()
            .map(|row| {
                let total = row.iter().sum::<f64>();
                row.iter()
                    .map(|v| if total > 0.0 { v / total } else { 0.0 })
                    .collect()
            })
            .collect()
    }
    pub fn component_masses(&self) -> Result<Vec<f64>, &'static str> {
        let mut masses = vec![0.; self.names.len()];
        for row in &self.component_mass {
            for (total, mass) in masses.iter_mut().zip(row) {
                *total += mass;
                if !total.is_finite() {
                    return Err("film component mass overflow");
                }
            }
        }
        Ok(masses)
    }

    /// Read-only extensive component inventories, per cell, in cubic metres.
    pub fn component_volumes_m3(&self) -> &[Vec<f64>] {
        &self.component_volume_cache
    }
    /// Canonical extensive component inventories, per cell, in kilograms.
    pub fn component_masses_kg(&self) -> &[Vec<f64>] {
        &self.component_mass
    }
    fn refresh_volumes(&mut self) -> Result<(), &'static str> {
        let density = self.film.material.density;
        let mut cache = Vec::with_capacity(self.component_mass.len());
        let mut bulk = Vec::with_capacity(self.component_mass.len());
        for row in &self.component_mass {
            let mut volumes = Vec::with_capacity(row.len());
            for mass in row {
                let volume = mass / density;
                if !mass.is_finite()
                    || *mass < 0.
                    || !volume.is_finite()
                    || (*mass > 0. && volume == 0.)
                {
                    return Err("film component volume cannot be represented");
                }
                volumes.push(volume);
            }
            let total = row.iter().sum::<f64>() / density;
            if !total.is_finite() {
                return Err("film volume overflow");
            }
            cache.push(volumes);
            bulk.push(total);
        }
        if !(bulk.iter().sum::<f64>() * density).is_finite() {
            return Err("film mass overflow");
        }
        self.component_volume_cache = cache;
        self.film.volume = bulk;
        Ok(())
    }
    fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, &'static str>,
    ) -> Result<T, &'static str> {
        let mut staged = self.clone();
        let result = operation(&mut staged)?;
        staged.refresh_volumes()?;
        *self = staged;
        Ok(result)
    }
    fn change_component_masses(
        &mut self,
        requests: &[(usize, Vec<f64>)],
        deposit: bool,
    ) -> Result<FilmTransfer, &'static str> {
        let density = self.film.material.density;
        self.transaction(|staged| {
            let mut components = vec![0.; staged.names.len()];
            let mut changed = vec![0.; staged.component_mass.len()];
            for (cell, amounts) in requests {
                if amounts.len() != staged.names.len() {
                    return Err("invalid film transfer component count");
                }
                let row = staged
                    .component_mass
                    .get_mut(*cell)
                    .ok_or("invalid film transfer cell")?;
                for ((entry, amount), total) in row.iter_mut().zip(amounts).zip(&mut components) {
                    if !amount.is_finite() || *amount < 0. || (!deposit && amount > entry) {
                        return Err("invalid film component mass transfer");
                    }
                    if *amount == 0. {
                        continue;
                    }
                    let next = if deposit {
                        *entry + amount
                    } else {
                        *entry - amount
                    };
                    let actual = if deposit {
                        next - *entry
                    } else {
                        *entry - next
                    };
                    if !next.is_finite() || !actual.is_finite() {
                        return Err("film component mass overflow");
                    }
                    *entry = next;
                    *total += actual;
                    changed[*cell] += actual;
                    if !total.is_finite() || !changed[*cell].is_finite() {
                        return Err("film transfer overflow");
                    }
                }
            }
            let before = staged.film.volume.clone();
            staged.refresh_volumes()?;
            let mut volume = 0.;
            for (cell, mass) in changed.iter().enumerate() {
                if *mass == 0. {
                    continue;
                }
                let delta = if deposit {
                    staged.film.volume[cell] - before[cell]
                } else {
                    before[cell] - staged.film.volume[cell]
                };
                let expected = mass / density;
                let tolerance = 16. * f64::EPSILON * before[cell].max(staged.film.volume[cell]);
                if !expected.is_finite()
                    || expected == 0.
                    || delta <= 0.
                    || (delta - expected).abs() > tolerance
                {
                    return Err(if deposit {
                        "film deposit balance cannot be represented"
                    } else {
                        "film withdrawal balance cannot be represented"
                    });
                }
                volume += delta;
            }
            let mass = components.iter().sum::<f64>();
            let volumes: Vec<_> = components.iter().map(|m| m / density).collect();
            if !mass.is_finite() || !volume.is_finite() || volumes.iter().any(|v| !v.is_finite()) {
                return Err("film transfer cannot be represented");
            }
            Ok(FilmTransfer {
                volume_m3: volume,
                mass_kg: mass,
                component_volumes_m3: volumes,
                component_masses_kg: components,
            })
        })
    }

    /// Atomically accept kilograms per component and cell. Receipts describe
    /// representable additions, including zero when an addition is below storage
    /// resolution. A species change without matching bulk change is rejected.
    /// This boundary still uses the current constant-density volume storage.
    pub fn deposit_component_masses_batch(
        &mut self,
        deposits: &[(usize, Vec<f64>)],
    ) -> Result<FilmTransfer, &'static str> {
        self.change_component_masses(deposits, true)
    }

    /// Withdraw selected species directly from canonical kilogram inventories.
    /// Exact depletion consumes stored mass without a volume round trip.
    /// Receipts still report actual representable removal; requests are not receipts.
    pub fn withdraw_component_masses_batch(
        &mut self,
        withdrawals: &[(usize, Vec<f64>)],
    ) -> Result<FilmTransfer, &'static str> {
        self.change_component_masses(withdrawals, false)
    }

    /// Atomically withdraw explicitly selected component volumes from multiple cells.
    /// Repeated cells share the staged inventory; no request may overdraw a species.
    /// Returned quantities reflect representable removal, not merely requested flux.
    /// Empty inventories produce exactly dry cells. Bulk/species balance is checked
    /// at floating-point roundoff scale. No evaporation kinetics, heat or momentum
    /// transfer is modeled here; destination coupling must stage its own transaction.
    pub fn withdraw_components_batch(
        &mut self,
        withdrawals: &[(usize, Vec<f64>)],
    ) -> Result<FilmWithdrawal, &'static str> {
        let density = self.film.material.density;
        let mut staged = self.component_mass.clone();
        let mut requests = Vec::with_capacity(withdrawals.len());
        for (cell, volumes) in withdrawals {
            if volumes.len() != self.names.len() {
                return Err("invalid film withdrawal component count");
            }
            let row = staged
                .get_mut(*cell)
                .ok_or("invalid film withdrawal cell")?;
            let mut amounts = Vec::with_capacity(volumes.len());
            for (mass, volume) in row.iter_mut().zip(volumes) {
                let available = *mass / density;
                if !volume.is_finite() || *volume < 0. || *volume > available {
                    return Err("invalid film component withdrawal");
                }
                let amount = if *volume == 0. {
                    0.
                } else if *volume == available {
                    *mass
                } else {
                    *volume * density
                };
                if !amount.is_finite() || (*volume > 0. && amount == 0.) {
                    return Err("film withdrawal mass cannot be represented");
                }
                if amount > *mass {
                    return Err("invalid film component withdrawal");
                }
                if amount > 0. {
                    *mass -= amount;
                }
                amounts.push(amount);
            }
            requests.push((*cell, amounts));
        }
        self.withdraw_component_masses_batch(&requests)
    }

    /// A volume deposit converts once to extensive component mass, never by averaging fractions.
    /// Invalid composition or overflow leaves both the film and inventories intact.
    pub fn deposit(
        &mut self,
        cell: usize,
        volume: f64,
        fractions: &[f64],
    ) -> Result<(), &'static str> {
        self.deposit_batch(&[(cell, volume, fractions.to_vec())])?;
        Ok(())
    }
    /// Atomic deposits with independently supplied composition rows. Returns
    /// deposited volume; no film or component mutation occurs on any error.
    /// Positive species additions below mass resolution are rejected: legacy
    /// volume callers cannot book the actual kilogram receipt themselves.
    pub fn deposit_batch(
        &mut self,
        deposits: &[(usize, f64, Vec<f64>)],
    ) -> Result<f64, &'static str> {
        let density = self.film.material.density;
        let mut requests = Vec::with_capacity(deposits.len());
        let mut availability = self.component_mass.clone();
        for (cell, volume, fractions) in deposits {
            let fractions = Self::normalized(fractions, self.names.len())?;
            if !volume.is_finite() || *volume < 0. {
                return Err("invalid film component deposit");
            }
            let mut masses = Vec::with_capacity(fractions.len());
            let row = availability
                .get_mut(*cell)
                .ok_or("invalid film component cell")?;
            for (available, fraction) in row.iter_mut().zip(fractions) {
                let mass = (volume * density) * fraction;
                if !mass.is_finite() || (*volume > 0. && fraction > 0. && mass == 0.) {
                    return Err("film deposit mass cannot be represented");
                }
                if mass > 0. {
                    let next = *available + mass;
                    if next == *available {
                        return Err("film deposit mass change cannot be represented");
                    }
                    if !next.is_finite() {
                        return Err("film component mass overflow");
                    }
                    *available = next;
                }
                masses.push(mass);
            }
            requests.push((*cell, masses));
        }
        Ok(self.deposit_component_masses_batch(&requests)?.volume_m3)
    }

    /// Translates a planar body while transferring the mixture under its footprint.
    /// Local Newtonian viscosities set drag sum(mu_i*A_i/gap_i), recomputed each
    /// interval alongside coverage and composition. All body kinetic loss heats
    /// the body; film inertia, squeeze pressure and normal contact remain absent.
    /// Any failure restores body, film volume and every component inventory.
    pub fn advance_sliding_patch(
        &mut self,
        dt: f64,
        patch: SlidingPatch,
        body: &mut ThermalTranslatingBody,
        max_substep: f64,
    ) -> Result<SlidingAdvanceReport, &'static str> {
        let mut next_body = *body;
        let result = self.transaction(|staged| {
            let body = &mut next_body;

            if !dt.is_finite()
                || dt <= 0.0
                || dt > 0.1
                || !max_substep.is_finite()
                || max_substep <= 0.0
                || max_substep > 0.001
            {
                return Err("invalid moving mixture patch interval");
            }
            let count = (dt / max_substep).ceil() as usize;
            if count > 100000 {
                return Err("moving mixture patch substep budget");
            }
            let step = dt / count as f64;
            let old_volume = staged.film.volume.clone();
            let old_components = staged.component_mass.clone();
            let mut candidate = *body;
            let mut report = SlidingAdvanceReport {
                mean_wetted_area: 0.0,
                substrate_impulse: [0.0; 3],
                dissipated_heat: 0.0,
                substeps: count,
            };
            let result = (|| -> Result<(), &'static str> {
                for _ in 0..count {
                    let before = candidate.mechanics.velocity;
                    let viscosity = staged.effective_viscosities();
                    let (current, velocity) = staged.film.sliding_patch_exchange_viscosities(
                        step,
                        patch,
                        &mut candidate,
                        staged
                            .component_viscosities
                            .as_ref()
                            .map(|_| viscosity.as_slice()),
                    )?;
                    staged.step_with_advection(step, [0.0; 3], &velocity, step)?;
                    report.mean_wetted_area += current.wetted_area / count as f64;
                    report.dissipated_heat += current.exchange.dissipated_heat;
                    for k in 0..3 {
                        report.substrate_impulse[k] += current.exchange.substrate_impulse[k];
                        candidate.mechanics.position[k] +=
                            0.5 * step * (before[k] + candidate.mechanics.velocity[k]);
                    }
                    if candidate
                        .mechanics
                        .position
                        .iter()
                        .chain(&report.substrate_impulse)
                        .any(|v| !v.is_finite())
                        || !report.mean_wetted_area.is_finite()
                        || !report.dissipated_heat.is_finite()
                    {
                        return Err("moving mixture patch state overflow");
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                staged.film.volume = old_volume;
                staged.component_mass = old_components;
                return Err(error);
            }
            *body = candidate;
            Ok(report)
        })?;
        *body = next_body;
        Ok(result)
    }

    /// Prescribed vented squeezing carries all component inventories and uses
    /// local Newtonian viscosity on every pressure solve. Vented components are
    /// returned as external mass inventories. Entire call rolls back on error.
    pub fn step_squeeze(
        &mut self,
        dt: f64,
        closing_speed: &[f64],
        control: super::SqueezePressureControl,
        max_substep: f64,
    ) -> Result<super::SqueezeTransportReport, &'static str> {
        let result = self.transaction(|staged| {
            staged.film.step_squeeze_components(
                dt,
                closing_speed,
                control,
                max_substep,
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
            )
        })?;

        Ok(result)
    }

    /// Normal-only pressure feedback for a plane covering the whole filled mesh.
    /// Body, film and all component inventories commit together or all roll back.
    pub fn advance_squeeze_body(
        &mut self,
        dt: f64,
        normal: [f64; 3],
        body: &mut ThermalTranslatingBody,
        control: super::SqueezePressureControl,
        max_substep: f64,
    ) -> Result<super::SqueezeBodyReport, &'static str> {
        let mut next_body = *body;
        let result = self.transaction(|staged| {
            let body = &mut next_body;

            staged.film.advance_squeeze_body_components(
                dt,
                normal,
                body,
                control,
                max_substep,
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
            )
        })?;
        *body = next_body;
        Ok(result)
    }

    /// Fickian diffusion with one common diffusivity (m²/s) for all components.
    /// Conductance is D*edge_length*harmonic_height/centroid_distance. Dry edges
    /// do not diffuse. Symmetric exact pair sweeps preserve positivity without
    /// a diffusive CFL limit; substeps control splitting error on multi-edge meshes.
    /// Volume is fixed. Any error leaves all component inventories unchanged.
    pub fn diffuse(
        &mut self,
        dt: f64,
        diffusivity: f64,
        max_substep: f64,
    ) -> Result<(), &'static str> {
        self.diffuse_with_energy(dt, diffusivity, max_substep, None)
    }
    pub(super) fn diffuse_with_energy(
        &mut self,
        dt: f64,
        diffusivity: f64,
        max_substep: f64,
        thermal: Option<(&mut Vec<f64>, &[f64])>,
    ) -> Result<(), &'static str> {
        let result = self.transaction(|staged| {
            if !dt.is_finite()
                || dt <= 0.0
                || dt > 0.1
                || !diffusivity.is_finite()
                || diffusivity < 0.0
                || !max_substep.is_finite()
                || max_substep <= 0.0
                || max_substep > 0.001
            {
                return Err("invalid film diffusion controls");
            }
            let count = (dt / max_substep).ceil() as usize;
            if count > 100000 {
                return Err("film diffusion substep budget");
            }
            let half_step = 0.5 * dt / count as f64;
            let mut pairs = Vec::new();
            for &(a, b, length, distance) in &staged.film.edges {
                let va = staged.film.volume[a];
                let vb = staged.film.volume[b];
                if va == 0.0 || vb == 0.0 {
                    continue;
                }
                let ha = va / staged.film.area[a];
                let hb = vb / staged.film.area[b];
                let height = ha.min(hb) / (0.5 + 0.5 * ha.min(hb) / ha.max(hb));
                let conductance = diffusivity * length * height / distance;
                let exponent = (conductance / va + conductance / vb) * half_step;
                let capacity = va.min(vb) / (1.0 + va.min(vb) / va.max(vb));
                if !exponent.is_finite() || !capacity.is_finite() {
                    return Err("film diffusion rate overflow");
                }
                pairs.push((a, b, va, vb, capacity * -(-exponent).exp_m1()));
            }
            let mut energy = thermal.as_ref().map(|(e, _)| (**e).clone());
            if let Some((values, capacities)) = &thermal {
                if values.len() != staged.component_mass.len()
                    || capacities.len() != staged.names.len()
                    || capacities.iter().any(|c| !c.is_finite() || *c <= 0.)
                    || values.iter().any(|e| !e.is_finite() || *e < 0.)
                {
                    return Err("invalid film diffusion thermal inventory");
                }
            }
            let mut next = staged.component_mass.clone();
            for _ in 0..count {
                for &(a, b, va, vb, exchange) in pairs.iter().chain(pairs.iter().rev()) {
                    let mut carried_heat = 0.;
                    let temperatures =
                        if let (Some(values), Some((_, capacities))) = (&energy, &thermal) {
                            let capacity = |row: &[f64]| {
                                row.iter().zip(*capacities).map(|(m, c)| m * c).sum::<f64>()
                            };
                            let ca = capacity(&next[a]);
                            let cb = capacity(&next[b]);
                            let ta = values[a] / ca;
                            let tb = values[b] / cb;
                            if !ta.is_finite() || !tb.is_finite() || ta <= 0. || tb <= 0. {
                                return Err("invalid film diffusion temperature");
                            }
                            Some([ta, tb])
                        } else {
                            None
                        };

                    for k in 0..staged.names.len() {
                        let moved = exchange * (next[a][k] / va - next[b][k] / vb);
                        // Limit only floating-point overshoot at exact depletion.
                        let moved = moved.clamp(-next[b][k], next[a][k]);
                        if let (Some(t), Some((_, capacities))) = (temperatures, &thermal) {
                            carried_heat +=
                                moved * capacities[k] * if moved >= 0. { t[0] } else { t[1] };
                        }
                        next[a][k] -= moved;
                        next[b][k] += moved;
                        if !next[a][k].is_finite() || !next[b][k].is_finite() {
                            return Err("film diffusion inventory overflow");
                        }
                    }
                    if let Some(values) = &mut energy {
                        let ea = values[a] - carried_heat;
                        let eb = values[b] + carried_heat;
                        if !ea.is_finite() || !eb.is_finite() || ea <= 0. || eb <= 0. {
                            return Err("film diffusion thermal overflow");
                        }
                        values[a] = ea;
                        values[b] = eb;
                    }
                }
            }
            staged.component_mass = next;
            staged.refresh_volumes()?;
            if let (Some((target, _)), Some(values)) = (thermal, energy) {
                *target = values;
            }
            Ok(())
        })?;

        Ok(result)
    }

    pub(super) fn step_with_energy(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        velocity: &[[f64; 3]],
        max_substep: f64,
        energy: &mut Vec<f64>,
    ) -> Result<(), &'static str> {
        let mut next_energy = energy.clone();
        let result = self.transaction(|staged| {
            let energy = &mut next_energy;

            staged.film.step_driven_components(
                dt,
                gravity,
                max_substep,
                None,
                None,
                Some(velocity),
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
                Some(energy),
            )
        })?;
        *energy = next_energy;
        Ok(result)
    }

    pub(super) fn step_with_shear_energy(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
        energy: &mut Vec<f64>,
    ) -> Result<(), &'static str> {
        let mut next_energy = energy.clone();
        let result = self.transaction(|staged| {
            let energy = &mut next_energy;

            staged.film.step_driven_components(
                dt,
                gravity,
                max_substep,
                Some(traction),
                None,
                None,
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
                Some(energy),
            )
        })?;
        *energy = next_energy;
        Ok(result)
    }

    pub fn step(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        let result = self.transaction(|staged| {
            staged.film.step_driven_components(
                dt,
                gravity,
                max_substep,
                None,
                None,
                None,
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
                None,
            )
        })?;

        Ok(result)
    }

    pub fn step_with_advection(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        velocity: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        let result = self.transaction(|staged| {
            staged.film.step_driven_components(
                dt,
                gravity,
                max_substep,
                None,
                None,
                Some(velocity),
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
                None,
            )
        })?;

        Ok(result)
    }

    pub fn step_with_surface_shear(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        let result = self.transaction(|staged| {
            staged.film.step_driven_components(
                dt,
                gravity,
                max_substep,
                Some(traction),
                None,
                None,
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
                None,
            )
        })?;

        Ok(result)
    }

    pub fn step_with_rheology(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
        model: FilmRheology,
    ) -> Result<(), &'static str> {
        let result = self.transaction(|staged| {
            staged.film.step_driven_components(
                dt,
                gravity,
                max_substep,
                Some(traction),
                Some(model),
                None,
                Some(&mut staged.component_mass),
                staged.component_viscosities.as_deref(),
                None,
            )
        })?;

        Ok(result)
    }
}

// Convex logarithmic blending, recomputed from extensive inventories each interval.
pub(super) fn blend_viscosities(rows: &[Vec<f64>], values: &[f64], dry: f64) -> Vec<f64> {
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(0.0, f64::max);
    rows.iter()
        .map(|row| {
            let total = row.iter().sum::<f64>();
            if total == 0.0 {
                return dry;
            }
            if low == high {
                return low;
            }
            let log = row
                .iter()
                .zip(values)
                .map(|(v, mu)| (v / total) * mu.ln())
                .sum::<f64>();
            // Clamp rounding at the mathematical convex bounds, including exp overflow
            // when ln(f64::MAX) rounds just above the representable exponential range.
            log.exp().clamp(low, high)
        })
        .collect()
}
