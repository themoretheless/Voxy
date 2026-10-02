//! Conservative equal-density component transport in a thin surface film.
use super::{FilmRheology, SlidingAdvanceReport, SlidingPatch, SurfaceFilm};
use crate::liquid::ThermalTranslatingBody;

/// Owns the film so volume cannot change without its component inventories.
/// All components share the film's constant density and surface tension.
/// Newtonian viscosity can depend on composition through a logarithmic blend.
/// Variable density, reactions and temperature are not yet coupled to mechanics.
/// Diffusion is a separate equal-diffusivity composition stage.
#[derive(Debug)]
pub struct FilmMixture {
    film: SurfaceFilm,
    names: Vec<String>,
    component_volume: Vec<Vec<f64>>,
    component_viscosities: Option<Vec<f64>>,
}
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
                .any(|(i, name)| name.is_empty() || names[..i].contains(name))
        {
            return Err("invalid film component schema");
        }
        let mut component_volume = Vec::with_capacity(fractions.len());
        for (fraction, volume) in fractions.iter().zip(&film.volume) {
            let fraction = Self::normalized(fraction, names.len())?;
            component_volume.push(fraction.iter().map(|f| volume * f).collect());
        }
        Ok(Self {
            film,
            names,
            component_volume,
            component_viscosities: None,
        })
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
            || vec![self.film.material.viscosity; self.component_volume.len()],
            |values| {
                blend_viscosities(&self.component_volume, values, self.film.material.viscosity)
            },
        )
    }
    pub fn film(&self) -> &SurfaceFilm {
        &self.film
    }
    pub fn component_names(&self) -> &[String] {
        &self.names
    }
    /// Dry cells have no composition and return an all-zero row.
    pub fn fractions(&self) -> Vec<Vec<f64>> {
        self.component_volume
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
        let mut masses = vec![0.0; self.names.len()];
        for row in &self.component_volume {
            for (mass, volume) in masses.iter_mut().zip(row) {
                *mass += volume * self.film.material.density;
                if !mass.is_finite() {
                    return Err("film component mass overflow");
                }
            }
        }
        Ok(masses)
    }
    /// A deposit mixes by extensive component volume, never by averaging fractions.
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
    pub fn deposit_batch(
        &mut self,
        deposits: &[(usize, f64, Vec<f64>)],
    ) -> Result<f64, &'static str> {
        let mut inventory = self.component_volume.clone();
        let mut volumes = Vec::with_capacity(deposits.len());
        for (cell, volume, fractions) in deposits {
            let fractions = Self::normalized(fractions, self.names.len())?;
            let row = inventory
                .get_mut(*cell)
                .ok_or("invalid film component cell")?;
            if !volume.is_finite() || *volume < 0.0 {
                return Err("invalid film component deposit");
            }
            for (entry, fraction) in row.iter_mut().zip(fractions) {
                *entry += volume * fraction;
                if !entry.is_finite() {
                    return Err("film component deposit overflow");
                }
            }
            volumes.push((*cell, *volume));
        }
        let added = self.film.deposit_batch(&volumes)?;
        self.component_volume = inventory;
        Ok(added)
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
        let old_volume = self.film.volume.clone();
        let old_components = self.component_volume.clone();
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
                let viscosity = self.effective_viscosities();
                let (current, velocity) = self.film.sliding_patch_exchange_viscosities(
                    step,
                    patch,
                    &mut candidate,
                    self.component_viscosities
                        .as_ref()
                        .map(|_| viscosity.as_slice()),
                )?;
                self.step_with_advection(step, [0.0; 3], &velocity, step)?;
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
            self.film.volume = old_volume;
            self.component_volume = old_components;
            return Err(error);
        }
        *body = candidate;
        Ok(report)
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
        self.film.step_squeeze_components(
            dt,
            closing_speed,
            control,
            max_substep,
            Some(&mut self.component_volume),
            self.component_viscosities.as_deref(),
        )
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
        self.film.advance_squeeze_body_components(
            dt,
            normal,
            body,
            control,
            max_substep,
            Some(&mut self.component_volume),
            self.component_viscosities.as_deref(),
        )
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
        for &(a, b, length, distance) in &self.film.edges {
            let va = self.film.volume[a];
            let vb = self.film.volume[b];
            if va == 0.0 || vb == 0.0 {
                continue;
            }
            let ha = va / self.film.area[a];
            let hb = vb / self.film.area[b];
            let height = ha.min(hb) / (0.5 + 0.5 * ha.min(hb) / ha.max(hb));
            let conductance = diffusivity * length * height / distance;
            let exponent = (conductance / va + conductance / vb) * half_step;
            let capacity = va.min(vb) / (1.0 + va.min(vb) / va.max(vb));
            if !exponent.is_finite() || !capacity.is_finite() {
                return Err("film diffusion rate overflow");
            }
            pairs.push((a, b, va, vb, capacity * -(-exponent).exp_m1()));
        }
        let mut next = self.component_volume.clone();
        for _ in 0..count {
            for &(a, b, va, vb, exchange) in pairs.iter().chain(pairs.iter().rev()) {
                for k in 0..self.names.len() {
                    let moved = exchange * (next[a][k] / va - next[b][k] / vb);
                    // Limit only floating-point overshoot at exact depletion.
                    let moved = moved.clamp(-next[b][k], next[a][k]);
                    next[a][k] -= moved;
                    next[b][k] += moved;
                    if !next[a][k].is_finite() || !next[b][k].is_finite() {
                        return Err("film diffusion inventory overflow");
                    }
                }
            }
        }
        self.component_volume = next;
        Ok(())
    }

    pub fn step(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        self.film.step_driven_components(
            dt,
            gravity,
            max_substep,
            None,
            None,
            None,
            Some(&mut self.component_volume),
            self.component_viscosities.as_deref(),
        )
    }
    pub fn step_with_advection(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        velocity: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        self.film.step_driven_components(
            dt,
            gravity,
            max_substep,
            None,
            None,
            Some(velocity),
            Some(&mut self.component_volume),
            self.component_viscosities.as_deref(),
        )
    }
    pub fn step_with_surface_shear(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        self.film.step_driven_components(
            dt,
            gravity,
            max_substep,
            Some(traction),
            None,
            None,
            Some(&mut self.component_volume),
            self.component_viscosities.as_deref(),
        )
    }
    pub fn step_with_rheology(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
        model: FilmRheology,
    ) -> Result<(), &'static str> {
        self.film.step_driven_components(
            dt,
            gravity,
            max_substep,
            Some(traction),
            Some(model),
            None,
            Some(&mut self.component_volume),
            self.component_viscosities.as_deref(),
        )
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
