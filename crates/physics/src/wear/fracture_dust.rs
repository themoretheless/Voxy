//! Airborne fines from fracture: the deformable kinetic energy that rigid
//! pieces cannot carry is spent creating new particle surface. Mass is
//! conserved by returning the fraction each donor piece must lose.

use crate::plasticity::mesh::DebrisRelease;
use crate::suspension::Particle;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractureDustSettings {
    /// Grain material density.
    pub density_kg_m3: f64,
    /// Specific energy of fresh single surface, J/m².
    pub surface_energy_j_m2: f64,
    /// Fraction of the release's deformation kinetic energy spent on fines.
    pub energy_fraction: f64,
    /// Extra caller-owned energy (e.g. impact work) also available, J.
    pub extra_energy_j: f64,
    /// Emitted grain radius.
    pub grain_radius_m: f64,
    /// Upper bound on dust mass as a fraction of the released piece mass.
    pub max_mass_fraction: f64,
}

#[derive(Clone, Debug)]
pub struct FractureDust {
    /// Grains at hull vertices of the donor pieces, inheriting piece velocities.
    pub particles: Vec<Particle>,
    pub mass_kg: f64,
    /// Energy actually spent creating surface (≤ available).
    pub surface_energy_j: f64,
    pub created_surface_m2: f64,
    /// Fraction of each released piece's mass that moved into the fines.
    pub donor_mass_fraction: f64,
}

/// Converts fracture energy into fines. Spherical grains of radius `r` cost
/// `3 γ / (ρ r)` joules per kilogram of surface, so `m = E ρ r / (3 γ)`,
/// capped by `max_mass_fraction` of the released mass.
/// # Errors
/// Invalid settings, a release without pieces, or more than 4096 emission sites.
pub fn fracture_dust(
    release: &DebrisRelease,
    settings: FractureDustSettings,
) -> Result<FractureDust, &'static str> {
    let s = settings;
    if !s.density_kg_m3.is_finite()
        || s.density_kg_m3 <= 0.
        || !s.surface_energy_j_m2.is_finite()
        || s.surface_energy_j_m2 <= 0.
        || !s.energy_fraction.is_finite()
        || !(0. ..=1.).contains(&s.energy_fraction)
        || !s.extra_energy_j.is_finite()
        || s.extra_energy_j < 0.
        || !s.grain_radius_m.is_finite()
        || s.grain_radius_m <= 0.
        || !s.max_mass_fraction.is_finite()
        || !(0. ..=1.).contains(&s.max_mass_fraction)
    {
        return Err("invalid fracture dust settings");
    }
    if release.pieces.is_empty() {
        return Err("fracture dust needs released pieces");
    }
    let sites: usize = release.pieces.iter().map(|p| p.hull.len()).sum();
    if sites == 0 || sites > 4096 {
        return Err("invalid fracture dust emission sites");
    }
    let released_mass: f64 = release.pieces.iter().map(|p| p.mass_kg).sum();
    let available_j = s.energy_fraction * release.deformation_kinetic_j + s.extra_energy_j;
    let specific_per_kg = 3. * s.surface_energy_j_m2 / (s.density_kg_m3 * s.grain_radius_m);
    let energy_limited = available_j / specific_per_kg;
    let mass_kg = energy_limited.min(s.max_mass_fraction * released_mass);
    if !mass_kg.is_finite() || mass_kg < 0. {
        return Err("fracture dust mass overflow");
    }
    let surface_energy_j = mass_kg * specific_per_kg;
    let created_surface_m2 = surface_energy_j / s.surface_energy_j_m2;
    let donor_mass_fraction = if released_mass > 0. {
        mass_kg / released_mass
    } else {
        0.
    };
    let mut particles = Vec::with_capacity(sites);
    if mass_kg > 0. {
        // Distribute mass over sites in proportion to each piece's share.
        for piece in &release.pieces {
            let piece_dust = mass_kg * piece.mass_kg / released_mass;
            let per_site = piece_dust / piece.hull.len() as f64;
            // One representative grain per site carries `per_site` mass; its
            // radius scales from the nominal grain so mass stays exact.
            let grain_mass =
                (4. / 3.) * std::f64::consts::PI * s.grain_radius_m.powi(3) * s.density_kg_m3;
            for &position in &piece.hull {
                let radius = (per_site / grain_mass).cbrt() * s.grain_radius_m;
                particles.push(Particle::new(
                    radius,
                    s.density_kg_m3,
                    position,
                    piece.velocity_m_s,
                )?);
            }
        }
        let emitted: f64 = particles.iter().map(Particle::mass_kg).sum();
        if (emitted - mass_kg).abs() > 1e-9 * mass_kg.max(f64::MIN_POSITIVE) {
            return Err("fracture dust mass mismatch");
        }
    }
    Ok(FractureDust {
        particles,
        mass_kg,
        surface_energy_j,
        created_surface_m2,
        donor_mass_fraction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plasticity::Material;
    use crate::plasticity::mesh::QuadraticBody;

    fn release() -> DebrisRelease {
        let material = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
        let body = QuadraticBody::from_linear(
            vec![
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., -1.],
            ],
            vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
        )
        .unwrap();
        // Non-rigid velocity field: nodes stretch away from the origin.
        let velocities: Vec<[f64; 3]> = body
            .positions()
            .iter()
            .map(|p| p.map(|x| 0.3 * x))
            .collect();
        body.release_debris(&[2000., 2000.], &velocities, 0.)
            .unwrap()
    }

    fn settings() -> FractureDustSettings {
        FractureDustSettings {
            density_kg_m3: 2000.,
            surface_energy_j_m2: 1.,
            energy_fraction: 1.,
            extra_energy_j: 0.,
            grain_radius_m: 1e-4,
            max_mass_fraction: 0.5,
        }
    }

    #[test]
    fn mass_follows_the_surface_energy_budget_and_is_conserved() {
        let release = release();
        assert!(release.deformation_kinetic_j > 0.);
        let dust = fracture_dust(&release, settings()).unwrap();
        let specific = 3. * 1. / (2000. * 1e-4);
        let expected = release.deformation_kinetic_j / specific;
        assert!(
            (dust.mass_kg - expected).abs() <= 1e-12 * expected,
            "{} vs {expected}",
            dust.mass_kg
        );
        let emitted: f64 = dust.particles.iter().map(Particle::mass_kg).sum();
        assert!((emitted - dust.mass_kg).abs() <= 1e-9 * dust.mass_kg);
        assert!((dust.surface_energy_j - release.deformation_kinetic_j).abs() <= 1e-12);
        assert!((dust.created_surface_m2 - dust.surface_energy_j).abs() <= 1e-12);
        let released: f64 = release.pieces.iter().map(|p| p.mass_kg).sum();
        assert!((dust.donor_mass_fraction * released - dust.mass_kg).abs() <= 1e-12);
        // Grains sit at hull vertices with piece velocities.
        assert_eq!(
            dust.particles.len(),
            release.pieces.iter().map(|p| p.hull.len()).sum::<usize>()
        );
    }

    #[test]
    fn cap_and_validation_apply() {
        let release = release();
        let capped = fracture_dust(
            &release,
            FractureDustSettings {
                extra_energy_j: 1e12,
                ..settings()
            },
        )
        .unwrap();
        let released: f64 = release.pieces.iter().map(|p| p.mass_kg).sum();
        assert!((capped.mass_kg - 0.5 * released).abs() <= 1e-12 * released);
        assert!(capped.surface_energy_j < 1e12);
        let none = fracture_dust(
            &release,
            FractureDustSettings {
                energy_fraction: 0.,
                ..settings()
            },
        )
        .unwrap();
        assert_eq!(none.mass_kg, 0.);
        assert!(none.particles.is_empty());
        assert!(
            fracture_dust(
                &release,
                FractureDustSettings {
                    grain_radius_m: 0.,
                    ..settings()
                }
            )
            .is_err()
        );
        let empty = DebrisRelease {
            pieces: Vec::new(),
            retained_nodes: Vec::new(),
            kinetic_j: 0.,
            deformation_kinetic_j: 0.,
            surface_created_m2: 0.,
        };
        assert!(fracture_dust(&empty, settings()).is_err());
    }
}
