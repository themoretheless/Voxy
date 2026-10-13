//! Smearing a thin film with a sliding contact: per-cell normal load sets a
//! Coulomb limit through the friction law, the film thickness sets a Couette
//! (lubrication) limit `η v / h`, and the smaller of the two drives the film's
//! surface traction. Mass is conserved by the film solver; work is booked.

use super::{SurfaceFilm, dot};
use crate::friction;

/// Sliding contact over the whole film mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmearContact {
    /// Unit outward normal of the film surface (towards the sliding body).
    pub normal: [f64; 3],
    /// Body velocity relative to the substrate; only its tangential part acts.
    pub body_velocity_m_s: [f64; 3],
}

/// Per-cell outcome of the traction selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmearRegime {
    /// No load or no slip: nothing applied.
    Idle,
    /// Lubricated: Couette shear `η v / h` was the smaller limit.
    Lubricated,
    /// Dry: the Coulomb limit from the friction law was the smaller limit.
    Coulomb,
}

#[derive(Clone, Debug)]
pub struct SmearReport {
    pub traction_pa: Vec<[f64; 3]>,
    pub regimes: Vec<SmearRegime>,
    /// Work done on the film by the applied tractions, J.
    pub slip_work_j: f64,
    /// Friction dissipation booked by the friction law over Coulomb cells, J.
    pub coulomb_dissipated_j: f64,
    pub substeps: usize,
}

impl SurfaceFilm {
    /// Advances the film under a sliding body for `dt`. `pressure_pa` is the
    /// per-cell normal load (e.g. from a squeeze solve); `states` carry the
    /// friction history per cell and are only committed on success.
    /// # Errors
    /// Invalid controls, mismatched lengths, invalid friction state, or a film
    /// transport failure (the film and states are then unchanged).
    pub fn advance_smear(
        &mut self,
        dt: f64,
        contact: SmearContact,
        pressure_pa: &[f64],
        material: &friction::Material,
        states: &mut [friction::State],
        gravity: [f64; 3],
        max_substep: f64,
    ) -> Result<SmearReport, &'static str> {
        let cells = self.volume.len();
        if !dt.is_finite()
            || dt <= 0.
            || !max_substep.is_finite()
            || max_substep <= 0.
            || pressure_pa.len() != cells
            || states.len() != cells
            || pressure_pa.iter().any(|p| !p.is_finite() || *p < 0.)
            || contact.normal.iter().any(|v| !v.is_finite())
            || (dot(contact.normal, contact.normal) - 1.).abs() > 1e-9
            || contact.body_velocity_m_s.iter().any(|v| !v.is_finite())
        {
            return Err("invalid smear controls");
        }
        let normal = contact.normal;
        let along = dot(contact.body_velocity_m_s, normal);
        let tangential: [f64; 3] =
            std::array::from_fn(|i| contact.body_velocity_m_s[i] - along * normal[i]);
        let speed = dot(tangential, tangential).sqrt();
        let thickness = self.thickness();
        let viscosity = self.material.viscosity;
        let mut traction = vec![[0.; 3]; cells];
        let mut regimes = vec![SmearRegime::Idle; cells];
        let mut next_states = states.to_vec();
        let mut coulomb_dissipated_j = 0.;
        for cell in 0..cells {
            if speed == 0. || pressure_pa[cell] == 0. {
                continue;
            }
            // Coulomb limit through the friction law at the load's penetration.
            let gap = -pressure_pa[cell] / material.normal_stiffness();
            let slip: [f64; 3] = std::array::from_fn(|i| tangential[i] * dt + gap * normal[i]);
            let jump: [f64; 3] = std::array::from_fn(|i| next_states[cell].slip()[i] + slip[i]);
            let (state, response) = material.response(&next_states[cell], jump, normal)?;
            let coulomb = dot(
                response.tangential_traction_pa,
                response.tangential_traction_pa,
            )
            .sqrt();
            // Lubrication limit; an empty cell has no film to shear.
            let couette = if thickness[cell] > 0. {
                viscosity * speed / thickness[cell]
            } else {
                f64::INFINITY
            };
            let (magnitude, regime) = if couette <= coulomb {
                (couette, SmearRegime::Lubricated)
            } else {
                (coulomb, SmearRegime::Coulomb)
            };
            if !magnitude.is_finite() {
                return Err("smear traction overflow");
            }
            if regime == SmearRegime::Coulomb {
                coulomb_dissipated_j += (state.dissipated_j_m2()
                    - next_states[cell].dissipated_j_m2())
                    * self.area[cell];
                next_states[cell] = state;
            }
            traction[cell] = tangential.map(|v| v / speed * magnitude);
            regimes[cell] = regime;
        }
        let mut candidate = self.clone();
        candidate.step_with_surface_shear(dt, gravity, &traction, max_substep)?;
        let slip_work_j: f64 = traction
            .iter()
            .zip(&self.area)
            .map(|(t, area)| dot(*t, tangential) * area * dt)
            .sum();
        if !slip_work_j.is_finite() || !coulomb_dissipated_j.is_finite() {
            return Err("smear work overflow");
        }
        *self = candidate;
        states.copy_from_slice(&next_states);
        Ok(SmearReport {
            traction_pa: traction,
            regimes,
            slip_work_j,
            coulomb_dissipated_j,
            substeps: (dt / max_substep).ceil() as usize,
        })
    }
}
