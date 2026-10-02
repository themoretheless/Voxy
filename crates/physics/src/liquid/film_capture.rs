//! Discrete point-trajectory absorption onto a stationary single-material thin film.
use super::{ExchangeTotals, Liquid};
use crate::surface_film::SurfaceFilm;
/// Conservation ledger for complete, perfectly sticking particles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilmCapture {
    pub particles: usize,
    pub deposited_volume: f64,
    /// Removed mass becomes film mass. Momentum is transferred to the prescribed
    /// stationary substrate; kinetic and thermal energy must be accounted by caller.
    /// The current isothermal film has no momentum or thermal state of its own.
    pub absorbed: ExchangeTotals,
}
impl Liquid {
    /// Captures intersections from pre-step positions to current particle centers.
    /// First intersected triangle receives each whole particle, irrespective of side.
    /// Fluid and film commit together; misses remain unchanged. Starting exactly on
    /// the plane does not count as a new impact. This operation does not advance flow.
    ///
    /// This discrete-time sticking rule does not solve impact-time forces, wetting,
    /// splashing, recoil, or particle-radius contact. Use small path intervals and
    /// do not also bounce against the same substrate before calling it.
    /// # Errors
    /// Invalid positions, geometry/ledger overflow, particle drain failure, or a
    /// mixture/phase/gas incompatible with the single-material isothermal film.
    pub fn capture_surface_film(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &mut SurfaceFilm,
    ) -> Result<FilmCapture, &'static str> {
        if previous_positions.len() != self.particles.len()
            || self.species_names().is_some()
            || self.phase_fractions().is_some()
            || self.gas_active()
            || self
                .fields()
                .is_some_and(|fields| fields.iter().any(|f| f.concentration != 0.0))
        {
            return Err("incompatible film capture state");
        }
        let density = film.material().density;
        let materials = self
            .effective_materials()
            .map_err(|_| "invalid capture material")?;
        let mut remove = Vec::new();
        let mut deposits = Vec::new();
        for (i, (start, particle)) in previous_positions.iter().zip(&self.particles).enumerate() {
            if let Some((cell, _)) = film.first_segment_hit(*start, particle.position)? {
                if (materials[i].rest_density - density).abs() > 1e-10 * density {
                    return Err("film and incident liquid densities differ");
                }
                let volume = particle.mass / density;
                if !volume.is_finite() || volume <= 0.0 {
                    return Err("invalid capture volume");
                }
                remove.push(i);
                deposits.push((cell, volume));
            }
        }
        let mut candidate = self.clone();
        let ledger = candidate
            .exchange_particles(&remove, &[])
            .map_err(|_| "film capture drain failed")?;
        let deposited_volume = film.deposit_batch(&deposits)?;
        // No fallible operation follows the film mutation.
        *self = candidate;
        Ok(FilmCapture {
            particles: remove.len(),
            deposited_volume,
            absorbed: ledger.removed,
        })
    }
}
