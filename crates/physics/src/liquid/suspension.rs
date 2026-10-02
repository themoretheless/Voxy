use super::Liquid;
use crate::suspension::{Carrier, CloudExchange, FiniteCarrier, Particle, exchange_forced_cloud};

/// Additive phase volumes at the current reference thermodynamic density.
/// This inventory is not a replacement for SPH kernel density or its EOS.
#[derive(Clone, Copy, Debug)]
pub struct SuspensionCellInventory {
    pub liquid_mass_kg: f64,
    pub liquid_volume_m3: f64,
    pub solid_mass_kg: f64,
    pub solid_volume_m3: f64,
    pub solid_volume_fraction: f64,
    pub mixture_density_kg_m3: f64,
}

impl Liquid {
    /// Physical drag heat awaiting a representable enthalpy increment, per carrier.
    #[must_use]
    pub fn suspension_heat_buffer(&self) -> &[f64] {
        &self.suspension_heat_buffer
    }
    /// Low parts of the compensated per-carrier heat inventory.
    pub fn suspension_heat_correction(&self) -> &[f64] {
        &self.suspension_heat_correction
    }
    fn accumulate_suspension_heat(&mut self, index: usize, heat: f64) -> Result<f64, &'static str> {
        let old = self.suspension_heat_buffer[index];
        let sum = old + heat;
        let recovered = sum - old;
        let error = (old - (sum - recovered)) + (heat - recovered);
        let low = self.suspension_heat_correction[index] + error;
        let high = sum + low;
        let correction = low - (high - sum);
        if !high.is_finite()
            || !correction.is_finite()
            || (heat > 0. && high == old && correction == self.suspension_heat_correction[index])
        {
            return Err("unrepresentable suspension heat buffer");
        }
        self.suspension_heat_buffer[index] = high;
        self.suspension_heat_correction[index] = correction;
        Ok(high)
    }
    /// Deposits nonnegative dissipative heat atomically, retaining sub-ULP heat
    /// in the per-carrier buffer. Budget bounds repeated enthalpy validation.
    /// # Errors
    /// Missing transport, invalid dimensions/energy, budget, property or precision failure.
    pub fn deposit_dissipation_heat(&mut self, heat: &[f64]) -> Result<(), &'static str> {
        if self.transport.is_none() {
            return Err("heat requires transport");
        }
        if heat.len() != self.particles.len()
            || heat.iter().any(|v| !v.is_finite() || *v < 0.)
            || heat
                .iter()
                .filter(|v| **v > 0.)
                .count()
                .saturating_mul(heat.len())
                > self.config.max_neighbor_checks
        {
            return Err("invalid or over-budget dissipative heat");
        }
        let mut candidate = self.clone();
        let mut request = vec![0.; heat.len()];
        for (index, incoming) in heat.iter().enumerate() {
            if *incoming == 0. && candidate.suspension_heat_buffer[index] == 0. {
                continue;
            }
            let old = candidate
                .transport
                .clone()
                .ok_or("heat requires transport")?;
            let before = old
                .energy(&candidate.particles[index], &old.fields[index], index)
                .map_err(|_| "invalid carrier energy")?;
            let available = candidate.accumulate_suspension_heat(index, *incoming)?;
            request[index] = available;
            candidate
                .add_heat(&request)
                .map_err(|_| "dissipative enthalpy update failed")?;
            request[index] = 0.;
            let updated = candidate
                .transport
                .as_ref()
                .ok_or("missing heated carrier")?;
            let after = updated
                .energy(&candidate.particles[index], &updated.fields[index], index)
                .map_err(|_| "invalid heated carrier energy")?;
            if after == before {
                candidate.transport = Some(old);
                candidate.suspension_heat_buffer[index] = available;
            } else if after < before
                || (after - before - available).abs()
                    > 16. * f64::EPSILON * (before.abs() + after.abs() + available)
            {
                return Err("dissipative heat balance failure");
            } else {
                candidate.suspension_heat_buffer[index] = 0.;
                // Only the high part was deposited; retain its rounding residue.
            }
        }
        *self = candidate;
        Ok(())
    }
    /// One inventory per fluid particle, using the same spatial assignment as drag.
    /// Mixture density = total phase mass / total phase reference volume;
    /// concentration is a solid volume fraction, separate from dissolved solute.
    /// # Errors
    /// Unsupported grains, neighbor budgets, invalid properties, nondilute cells
    /// or unrepresentable mass/volume. Read-only; no phase inventory is changed.
    pub fn suspension_inventory(
        &self,
        particles: &[Particle],
    ) -> Result<Vec<SuspensionCellInventory>, &'static str> {
        let assignment = self.suspension_assignment(particles)?;
        let materials = self
            .effective_materials()
            .map_err(|_| "invalid carrier properties")?;
        let mut cells: Vec<_> = self
            .particles
            .iter()
            .zip(materials)
            .map(|(p, m)| SuspensionCellInventory {
                liquid_mass_kg: p.mass,
                liquid_volume_m3: p.mass / m.rest_density,
                solid_mass_kg: 0.,
                solid_volume_m3: 0.,
                solid_volume_fraction: 0.,
                mixture_density_kg_m3: m.rest_density,
            })
            .collect();
        for (grain, index) in particles.iter().zip(assignment) {
            cells[index].solid_mass_kg += grain.mass_kg();
            cells[index].solid_volume_m3 += grain.volume_m3();
        }
        for cell in &mut cells {
            let volume = cell.liquid_volume_m3 + cell.solid_volume_m3;
            let mass = cell.liquid_mass_kg + cell.solid_mass_kg;
            cell.solid_volume_fraction = cell.solid_volume_m3 / volume;
            cell.mixture_density_kg_m3 = mass / volume;
            if ![volume, mass, cell.mixture_density_kg_m3]
                .iter()
                .all(|x| x.is_finite() && *x > 0.)
                || !cell.solid_volume_fraction.is_finite()
                || cell.solid_volume_m3 / cell.liquid_volume_m3 > 0.01
            {
                return Err("invalid or nondilute suspension inventory");
            }
        }
        Ok(cells)
    }

    /// Joint free-flight fluid/cloud interval, with caller-selected coupling steps.
    /// Each subinterval exchanges drag/heat and drifts the cloud, then advances
    /// fluid with its symmetric pressure/thermal/viscous integrator. Coupling is
    /// first order despite the symmetric fluid integrator. Assignment is refreshed
    /// per subinterval. Returns accumulated fluid statistics and drag energy ledger.
    /// # Errors
    /// Requires heated viscosity, pressure work, zero gravity and no boundaries:
    /// shared-cloud buoyancy, particle gravity and wall deposition are not yet
    /// implemented. Invalid budgets or any intermediate failure roll back both.
    pub fn step_suspension_free(
        &mut self,
        particles: &mut [Particle],
        dt_s: f64,
        coupling_steps: usize,
    ) -> Result<(super::StepStats, CloudExchange), &'static str> {
        if !dt_s.is_finite()
            || dt_s <= 0.
            || dt_s > 0.1
            || coupling_steps == 0
            || coupling_steps > self.config.max_substeps
        {
            return Err("invalid coupled suspension interval or budget");
        }
        if self.config.gravity != [0.; 3]
            || !self.boundaries.is_empty()
            || self.boundary_coupling.is_some()
            || self.reflecting_box.is_some()
        {
            return Err("coupled suspension requires free flight without gravity");
        }
        if !self.viscous_heating || !self.pressure_work || self.transport.is_none() {
            return Err("coupled suspension requires heated viscosity and pressure work");
        }
        let dt = dt_s / coupling_steps as f64;
        if dt <= 0. {
            return Err("unrepresentable coupled suspension timestep");
        }
        let mut candidate = self.clone();
        let mut cloud = particles.to_vec();
        let mut stats = super::StepStats {
            substeps: 0,
            neighbor_pairs: 0,
            max_density_ratio: 0.,
        };
        let mut total = CloudExchange {
            viscous_heat_j: 0.,
            numerical_loss_j: 0.,
            body_force_work_j: 0.,
            energy_defect_j: 0.,
        };
        for _ in 0..coupling_steps {
            let (_, drag) = candidate.advance_suspension(&mut cloud, dt)?;
            let fluid = candidate
                .step_symmetric_free(dt)
                .map_err(|_| "coupled suspension fluid advance failed")?;
            stats.substeps = stats
                .substeps
                .checked_add(fluid.substeps)
                .ok_or("coupled fluid substep count overflow")?;
            if stats.substeps > self.config.max_substeps {
                return Err("coupled fluid substep budget exceeded");
            }
            stats.neighbor_pairs = stats.neighbor_pairs.max(fluid.neighbor_pairs);
            stats.max_density_ratio = stats.max_density_ratio.max(fluid.max_density_ratio);
            total.viscous_heat_j += drag.viscous_heat_j;
            total.numerical_loss_j += drag.numerical_loss_j;
            total.body_force_work_j += drag.body_force_work_j;
            total.energy_defect_j += drag.energy_defect_j;
        }
        if ![
            total.viscous_heat_j,
            total.numerical_loss_j,
            total.body_force_work_j,
            total.energy_defect_j,
        ]
        .iter()
        .all(|x| x.is_finite())
        {
            return Err("coupled suspension energy overflow");
        }
        *self = candidate;
        particles.clone_from_slice(&cloud);
        Ok((stats, total))
    }

    /// Advances a cloud using nearest-carrier cells within the smoothing radius.
    /// Assignment uses positions at the start of the interval, with lower-index
    /// tie breaking, and is recomputed on the next call after particle drift.
    /// This is piecewise-constant spatial coupling (Voronoi cells), not SPH kernel
    /// interpolation. Returned indices identify the carriers used this interval.
    /// # Errors
    /// Unsupported particles, neighbor budget exhaustion, or any cell exchange
    /// failure. All carrier cells and all suspended particles roll back together.
    pub fn advance_suspension(
        &mut self,
        particles: &mut [Particle],
        dt_s: f64,
    ) -> Result<(Vec<usize>, CloudExchange), &'static str> {
        if !dt_s.is_finite() || dt_s <= 0. || particles.len() > 4096 {
            return Err("invalid spatial suspension step");
        }
        let assignments = self.suspension_assignment(particles)?;
        let mut groups = std::collections::BTreeMap::<usize, Vec<usize>>::new();
        for (index, cell) in assignments.iter().copied().enumerate() {
            groups.entry(cell).or_default().push(index);
        }
        let mut candidate = self.clone();
        let mut cloud = particles.to_vec();
        let mut total = CloudExchange {
            viscous_heat_j: 0.,
            numerical_loss_j: 0.,
            body_force_work_j: 0.,
            energy_defect_j: 0.,
        };
        for (cell, indices) in groups {
            let mut local: Vec<_> = indices.iter().map(|i| cloud[*i].clone()).collect();
            let report = candidate.exchange_suspension_cell(cell, &mut local, dt_s)?;
            total.viscous_heat_j += report.viscous_heat_j;
            total.numerical_loss_j += report.numerical_loss_j;
            total.body_force_work_j += report.body_force_work_j;
            total.energy_defect_j += report.energy_defect_j;
            for (index, grain) in indices.into_iter().zip(local) {
                cloud[index] = grain;
            }
        }
        if ![
            total.viscous_heat_j,
            total.numerical_loss_j,
            total.body_force_work_j,
            total.energy_defect_j,
        ]
        .iter()
        .all(|x| x.is_finite())
        {
            return Err("spatial suspension energy overflow");
        }
        *self = candidate;
        particles.clone_from_slice(&cloud);
        Ok((assignments, total))
    }

    /// Exchanges dilute Stokes drag with one caller-selected SPH control particle.
    /// SI units are required. Its reference volume is mass / effective rest density;
    /// this is a local homogeneous carrier approximation, not kernel deposition.
    /// Physical drag heat enters the existing enthalpy/latent-heat state. Returned
    /// backward-Euler numerical loss is not converted into heat. Fluid position is
    /// advanced by the normal liquid step; suspended drift is first order here.
    /// # Errors
    /// Missing transport, invalid index/regime, non-Newtonian carrier, or failed
    /// representable energy transfer. Fluid and entire cloud roll back together.
    pub fn exchange_suspension_cell(
        &mut self,
        index: usize,
        particles: &mut [Particle],
        dt_s: f64,
    ) -> Result<CloudExchange, &'static str> {
        if particles.len() > 4096 {
            return Err("suspension cloud budget exceeded");
        }
        self.exchange_suspension_cell_forced(
            index,
            particles,
            dt_s,
            &vec![[0.; 3]; particles.len()],
            [0.; 3],
        )
    }

    /// Local fluid/cloud exchange with caller-prescribed constant accelerations.
    /// Particle and carrier forces are specified separately; a pressure-force
    /// provider must include its reaction on the carrier and avoid duplicate
    /// forcing in the subsequent fluid step. Work remains a separate ledger;
    /// only drag dissipation enters fluid enthalpy. No hydrostatic assumption.
    /// # Errors
    /// Invalid force dimensions or any drag/thermal validation failure. Both
    /// liquid and cloud commit together, including force-induced particle drift.
    pub fn exchange_suspension_cell_forced(
        &mut self,
        index: usize,
        particles: &mut [Particle],
        dt_s: f64,
        particle_accelerations: &[[f64; 3]],
        carrier_acceleration: [f64; 3],
    ) -> Result<CloudExchange, &'static str> {
        if particles.len() > 4096 || particle_accelerations.len() != particles.len() {
            return Err("invalid suspension force dimensions or budget");
        }
        let fluid = self
            .particles
            .get(index)
            .ok_or("invalid suspension carrier index")?;
        if self.shear_thinning[fluid.material].is_some()
            || self.yield_stresses[fluid.material] != 0.
            || self.thixotropy[fluid.material].is_some()
        {
            return Err("Stokes suspension requires a Newtonian carrier");
        }
        let transport = self
            .transport
            .as_ref()
            .ok_or("suspension heat requires liquid transport")?;
        let before = transport
            .energy(fluid, &transport.fields[index], index)
            .map_err(|_| "invalid carrier enthalpy")?;
        let material = self
            .effective_materials()
            .map_err(|_| "invalid carrier properties")?[index];
        let mut carrier = FiniteCarrier::new(
            Carrier {
                density_kg_m3: material.rest_density,
                viscosity_pa_s: material.viscosity,
                velocity_m_s: fluid.velocity,
            },
            fluid.mass / material.rest_density,
        )?;
        if (carrier.mass_kg() - fluid.mass).abs() > 4. * f64::EPSILON * fluid.mass {
            return Err("unrepresentable carrier reference volume");
        }
        let mut cloud = particles.to_vec();
        let report = exchange_forced_cloud(
            &mut cloud,
            &mut carrier,
            dt_s,
            particle_accelerations,
            carrier_acceleration,
        )?;
        let mut candidate = self.clone();
        candidate.particles[index].velocity = carrier.velocity_m_s();
        let mut heat = vec![0.; candidate.particles.len()];
        let available = candidate.accumulate_suspension_heat(index, report.viscous_heat_j)?;
        heat[index] = available;
        candidate
            .add_heat(&heat)
            .map_err(|_| "suspension enthalpy update failed")?;
        let transport = candidate
            .transport
            .as_ref()
            .ok_or("missing candidate transport")?;
        let after = transport
            .energy(&candidate.particles[index], &transport.fields[index], index)
            .map_err(|_| "invalid accepted carrier enthalpy")?;
        if after == before {
            // No representable change: retain all heat in owned state.
            candidate.transport = self.transport.clone();
            candidate.suspension_heat_buffer[index] = available;
        } else if after < before
            || (after - before - available).abs()
                > 16. * f64::EPSILON * (before.abs() + after.abs() + available)
        {
            return Err("unrepresentable suspension heat deposition");
        } else {
            candidate.suspension_heat_buffer[index] = 0.;
            // Only the high part was deposited; retain its rounding residue.
        }
        *self = candidate;
        particles.clone_from_slice(&cloud);
        Ok(report)
    }
    fn suspension_assignment(&self, particles: &[Particle]) -> Result<Vec<usize>, &'static str> {
        if particles.len() > 4096 {
            return Err("suspension cloud budget exceeded");
        }
        let checks = particles
            .len()
            .checked_mul(self.particles.len())
            .ok_or("suspension neighbor budget overflow")?;
        if checks > self.config.max_neighbor_checks {
            return Err("suspension neighbor budget exceeded");
        }
        let mut assignments = Vec::with_capacity(particles.len());
        for grain in particles {
            let position = grain.position_m();
            let mut nearest = None;
            let mut distance = self.config.smoothing_radius;
            for (cell, fluid) in self.particles.iter().enumerate() {
                let d = (position[0] - fluid.position[0])
                    .hypot(position[1] - fluid.position[1])
                    .hypot(position[2] - fluid.position[2]);
                if d.is_finite()
                    && d <= self.config.smoothing_radius
                    && (nearest.is_none() || d < distance)
                {
                    nearest = Some(cell);
                    distance = d;
                }
            }
            let cell = nearest.ok_or("suspended particle outside carrier support")?;
            assignments.push(cell);
        }
        Ok(assignments)
    }
}
