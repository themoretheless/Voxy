//! Reflecting-box walls as a prescribed-temperature heat reservoir (plan
//! step F3). Particles whose surface lies within `contact_distance` of a box
//! face conduct through a per-face area `π r²`; everything else reuses the
//! exact reservoir exchange, so the energy ledger and latent plateau hold.
use super::{Error, Liquid};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallThermal {
    /// Prescribed wall temperature, K.
    pub temperature: f64,
    /// Conductance per unit contact area, W/(m²·K).
    pub conductance_w_m2_k: f64,
    /// Surface-to-wall gap at or below which a particle touches the wall, m.
    pub contact_distance: f64,
}

/// Outcome of one wall exchange.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallHeatReport {
    /// Heat gained by the liquid from the walls, J (negative when cooling).
    pub transferred_j: f64,
    pub touching_particles: usize,
    /// Per-particle conductances used, W/K, for diagnostics.
    pub total_conductance_w_k: f64,
}

impl Liquid {
    /// Per-particle wall conductances from the current positions: each face
    /// within reach adds `conductance_w_m2_k · π r²` with `r` the particle radius.
    /// # Errors
    /// Requires a reflecting box and valid wall settings.
    pub fn box_wall_conductances(&self, wall: WallThermal) -> Result<Vec<f64>, Error> {
        let bounds = self.reflecting_box().ok_or(Error::InvalidBoundary)?;
        if !wall.temperature.is_finite()
            || wall.temperature < 0.
            || !wall.conductance_w_m2_k.is_finite()
            || wall.conductance_w_m2_k < 0.
            || !wall.contact_distance.is_finite()
            || wall.contact_distance < 0.
        {
            return Err(Error::InvalidTransport);
        }
        let radius = self.config.particle_radius;
        let area = std::f64::consts::PI * radius * radius;
        Ok(self
            .particles
            .iter()
            .map(|p| {
                let faces = (0..3)
                    .map(|axis| {
                        let lower = p.position[axis] - bounds.min[axis] - radius;
                        let upper = bounds.max[axis] - p.position[axis] - radius;
                        usize::from(lower <= wall.contact_distance)
                            + usize::from(upper <= wall.contact_distance)
                    })
                    .sum::<usize>();
                wall.conductance_w_m2_k * area * faces as f64
            })
            .collect())
    }

    /// Exchanges heat between touching particles and the walls for `dt`.
    /// Atomic through `exchange_reservoir_heat`; no motion is advanced.
    /// # Errors
    /// Missing box or transport, invalid step or wall settings.
    pub fn exchange_box_wall_heat(
        &mut self,
        dt: f64,
        wall: WallThermal,
    ) -> Result<WallHeatReport, Error> {
        let conductances = self.box_wall_conductances(wall)?;
        let touching = conductances.iter().filter(|g| **g > 0.).count();
        let total: f64 = conductances.iter().sum();
        let transferred = self.exchange_reservoir_heat(dt, wall.temperature, &conductances)?;
        Ok(WallHeatReport {
            transferred_j: transferred,
            touching_particles: touching,
            total_conductance_w_k: total,
        })
    }
}
