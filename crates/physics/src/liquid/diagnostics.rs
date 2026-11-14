//! SPH kernel operators, instantaneous fluid diagnostics, and capillary pressure helpers.
use std::f64::consts::PI;

use super::{Error, Liquid, Particle, pairs, sub};

/// Coupled density and pressure discretization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formulation {
    /// Standard mass-weighted SPH, retained as the default.
    MassDensity,
    /// Neighbor reference volumes support the particle's own material density.
    /// Pressure differentiates the volume-density poly6 sum. Experimental multiphase mode.
    RestVolume,
    /// Reference-volume density with normalized 3D Wendland C2 kernel and its gradient.
    /// Experimental alternative intended to avoid kernel pairing; not a calibrated solver.
    RestVolumeWendland,
}

impl Liquid {
    /// Selects density/pressure operators without changing mass, velocities or fields.
    /// Switching in a running simulation can change pressure abruptly.
    /// A reflecting box stays valid across formulations; `MassDensity` images use the
    /// symmetric spiky operator with the owner's pressure.
    pub fn set_formulation(&mut self, formulation: Formulation) {
        self.formulation = formulation;
    }
}

/// Instantaneous lab-space state, before integration and collisions.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidDiagnostics {
    pub densities: Vec<f64>,
    /// Forces divided by particle mass. Exact thermal pair kicks are not included.
    pub accelerations: Vec<[f64; 3]>,
    pub viscous_rate: f64,
}

impl Liquid {
    /// # Errors
    /// Invalid effective properties, exhausted neighbor budgets or numerical failure.
    pub fn diagnostics(&self) -> Result<FluidDiagnostics, Error> {
        let neighbors = pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let properties = self.effective_materials()?;
        let (densities, accelerations, viscous_rate) =
            self.forces(&self.particles, &neighbors, &properties)?;
        Ok(FluidDiagnostics {
            densities,
            accelerations,
            viscous_rate,
        })
    }
}

pub(super) fn pressure_magnitude(
    mass: [f64; 2],
    density: [f64; 2],
    pressure: [f64; 2],
    reference: [f64; 2],
    support: f64,
    radius: f64,
    formulation: Formulation,
) -> f64 {
    let (ratio, gradient) = match formulation {
        Formulation::MassDensity => (
            1.0,
            45.0 / (PI * support.powi(6)) * (support - radius).max(0.0).powi(2),
        ),
        // Differentiate the same kernel used by the volume density sum.
        Formulation::RestVolume | Formulation::RestVolumeWendland => (
            reference[0] / reference[1],
            density_kernel_gradient(formulation, support, radius),
        ),
    };
    mass[0]
        * mass[1]
        * (pressure[0] / density[0].powi(2) * ratio + pressure[1] / density[1].powi(2) / ratio)
        * gradient
}

pub(super) fn density_kernel(formulation: Formulation, support: f64, radius: f64) -> f64 {
    if formulation == Formulation::RestVolumeWendland {
        let q = radius / support;
        21.0 / (2.0 * PI * support.powi(3)) * (1.0 - q).max(0.0).powi(4) * (1.0 + 4.0 * q)
    } else {
        315.0 / (64.0 * PI * support.powi(9))
            * (support * support - radius * radius).max(0.0).powi(3)
    }
}

pub(super) fn density_kernel_gradient(formulation: Formulation, support: f64, radius: f64) -> f64 {
    if formulation == Formulation::RestVolumeWendland {
        let q = radius / support;
        210.0 / (PI * support.powi(4)) * q * (1.0 - q).max(0.0).powi(3)
    } else {
        6.0 * 315.0 / (64.0 * PI * support.powi(9))
            * radius
            * (support * support - radius * radius).max(0.0).powi(2)
    }
}

pub(super) fn viscous_conductance(
    masses: [f64; 2],
    viscosity: f64,
    densities: [f64; 2],
    separation: f64,
    support: f64,
) -> f64 {
    if separation <= 1e-12 {
        return 0.0;
    }
    5.0 * masses[0] * masses[1] * viscosity * 45.0 / (PI * support.powi(6))
        * (support - separation).max(0.0)
        / (densities[0] * densities[1])
}

pub(super) fn color_normals(
    particles: &[Particle],
    pairs: &[(usize, usize)],
    density: &[f64],
    h: f64,
) -> Vec<[f64; 3]> {
    let poly6 = 315.0 / (64.0 * PI * h.powi(9));
    // Color-field normals from same-phase neighbors. Poly6 is differentiable at zero.
    let mut normals = vec![[0.0; 3]; particles.len()];
    for &(i, j) in pairs {
        if particles[i].material != particles[j].material {
            continue;
        }
        let delta = sub(particles[i].position, particles[j].position);
        let radius_squared: f64 = delta.iter().map(|v| v * v).sum();
        let gradient_scale = -6.0 * poly6 * (h * h - radius_squared).max(0.0).powi(2) * h;
        for (axis, component) in delta.into_iter().enumerate() {
            normals[i][axis] += particles[j].mass / density[j] * gradient_scale * component;
            normals[j][axis] -= particles[i].mass / density[i] * gradient_scale * component;
        }
    }
    normals
}

/// Evaluates the Young–Laplace capillary pressure jump across a curved fluid interface
/// $\Delta P = \sigma \left(\frac{1}{R_1} + \frac{1}{R_2}\right)$ in Pascals (`Pa`).
#[must_use]
pub fn young_laplace_capillary_pressure_pa(
    surface_tension_n_m: f64,
    r1_m: f64,
    r2_m: f64,
) -> Option<f64> {
    if !surface_tension_n_m.is_finite()
        || !r1_m.is_finite()
        || !r2_m.is_finite()
        || surface_tension_n_m < 0.0
        || r1_m.abs() <= 1e-12
        || r2_m.abs() <= 1e-12
    {
        return None;
    }
    let dp = surface_tension_n_m * (1.0 / r1_m + 1.0 / r2_m);
    dp.is_finite().then_some(dp)
}

#[cfg(test)]
mod density_kernel_tests {
    use super::{Formulation, PI, density_kernel};
    #[test]
    fn normalized_volume_kernels_and_wendland_nonnegative_spectral_samples() {
        for formulation in [Formulation::RestVolume, Formulation::RestVolumeWendland] {
            for support in [0.17_f64, 1.0, 3.0] {
                let intervals = 4000;
                let integral: f64 = (0..intervals)
                    .map(|i| {
                        let radius = support * (f64::from(i) + 0.5) / f64::from(intervals);
                        4.0 * PI
                            * radius.powi(2)
                            * density_kernel(formulation, support, radius)
                            * support
                            / f64::from(intervals)
                    })
                    .sum();
                assert!((integral - 1.0).abs() < 1e-8);
            }
        }
        // Radial 3D Fourier integral samples; these check the implemented shape,
        // not a proof of spectral nonnegativity for all wavenumbers.
        for frequency in [1.0_f64, 5.0, 10.0, 20.0, 30.0, 50.0] {
            let intervals = 4000;
            let transform: f64 = (0..intervals)
                .map(|i| {
                    let radius = (f64::from(i) + 0.5) / f64::from(intervals);
                    let phase = frequency * radius;
                    4.0 * PI
                        * radius.powi(2)
                        * density_kernel(Formulation::RestVolumeWendland, 1.0, radius)
                        * phase.sin()
                        / phase
                        / f64::from(intervals)
                })
                .sum();
            assert!(transform > 0.0, "frequency {frequency}: {transform}");
        }
    }
}

#[cfg(test)]
mod young_laplace_tests {
    use super::*;

    #[test]
    fn young_laplace_spherical_droplet_equals_two_sigma_over_r() {
        let dp = young_laplace_capillary_pressure_pa(0.072, 0.001, 0.001).unwrap();
        assert!((dp - 144.0).abs() < 1e-9);
        assert!(young_laplace_capillary_pressure_pa(0.072, 0.0, 0.001).is_none());
    }
}
