//! High-velocity multi-liquid jet nozzles, Rayleigh–Plateau primary breakup,
//! nozzle recoil forces, and oblique surface impact decomposition into
//! deposited thin films, tangential smear tractions, and crown splash sprays.

use std::f64::consts::PI;

use super::{Error, Material, MaxwellFluid, Particle, ShearThinning};
use crate::math::{add, cross, dot, norm, scale, sub};

/// Complete rheological and interfacial preset for shooting a Newtonian or
/// non-Newtonian liquid jet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JetFluidPreset {
    pub material: Material,
    /// Gas–liquid surface tension $\sigma$ (N/m).
    pub surface_tension_n_m: f64,
    /// Herschel–Bulkley / Bingham yield stress $\tau_0$ (Pa). Zero for Newtonian liquids.
    pub yield_stress_pa: f64,
    /// Optional power-law / Carreau shear-thinning viscosity profile.
    pub shear_thinning: Option<ShearThinning>,
    /// Optional upper-convected Maxwell viscoelastic polymer relaxation profile.
    pub maxwell: Option<MaxwellFluid>,
}

impl JetFluidPreset {
    /// Pure water at 20 °C ($\rho = 1000\text{ kg/m}^3$, $\mu = 1\text{ mPa}\cdot\text{s}$, $\sigma = 0.0728\text{ N/m}$).
    pub const WATER: Self = Self {
        material: Material::WATER,
        surface_tension_n_m: 0.0728,
        yield_stress_pa: 0.0,
        shear_thinning: None,
        maxwell: None,
    };

    /// Light mineral/hydrocarbon oil ($\rho = 800\text{ kg/m}^3$, $\mu = 0.1\text{ Pa}\cdot\text{s}$, $\sigma = 0.032\text{ N/m}$).
    pub const OIL: Self = Self {
        material: Material::OIL,
        surface_tension_n_m: 0.032,
        yield_stress_pa: 0.0,
        shear_thinning: None,
        maxwell: None,
    };

    /// High-viscosity glycerin ($\rho = 1260\text{ kg/m}^3$, $\mu = 1.412\text{ Pa}\cdot\text{s}$, $\sigma = 0.063\text{ N/m}$).
    pub const GLYCERIN: Self = Self {
        material: Material::GLYCERIN,
        surface_tension_n_m: 0.063,
        yield_stress_pa: 0.0,
        shear_thinning: None,
        maxwell: None,
    };

    /// Dense saline brine ($\rho = 1200\text{ kg/m}^3$, $\mu = 0.002\text{ Pa}\cdot\text{s}$, $\sigma = 0.082\text{ N/m}$).
    pub const BRINE: Self = Self {
        material: Material::BRINE,
        surface_tension_n_m: 0.082,
        yield_stress_pa: 0.0,
        shear_thinning: None,
        maxwell: None,
    };

    /// Dense viscoplastic mud slurry with yield stress ($\tau_0 = 45\text{ Pa}$).
    pub const MUD_SLURRY: Self = Self {
        material: Material {
            rest_density: 1650.0,
            sound_speed: 25.0,
            viscosity: 0.85,
        },
        surface_tension_n_m: 0.055,
        yield_stress_pa: 45.0,
        shear_thinning: None,
        maxwell: None,
    };

    /// Viscoelastic polymer gel with elastic relaxation and moderate yield stress.
    pub const VISCOELASTIC_GEL: Self = Self {
        material: Material {
            rest_density: 1080.0,
            sound_speed: 22.0,
            viscosity: 2.5,
        },
        surface_tension_n_m: 0.048,
        yield_stress_pa: 12.0,
        shear_thinning: None,
        maxwell: Some(MaxwellFluid {
            modulus: 180.0,
            relaxation_time: 0.08,
        }),
    };

    /// Shear-thinning biological/pigment suspension (blood or paint).
    pub const PAINT_OR_BLOOD: Self = Self {
        material: Material {
            rest_density: 1060.0,
            sound_speed: 20.0,
            viscosity: 0.035,
        },
        surface_tension_n_m: 0.052,
        yield_stress_pa: 3.5,
        shear_thinning: Some(ShearThinning {
            reference_rate: 10.0,
            flow_index: 0.65,
            minimum_rate: 0.1,
            minimum_viscosity: 0.004,
            maximum_viscosity: 0.12,
        }),
        maxwell: None,
    };

    pub fn validate(&self) -> Result<(), Error> {
        if !self.material.rest_density.is_finite()
            || self.material.rest_density <= 0.0
            || !self.material.sound_speed.is_finite()
            || self.material.sound_speed <= 0.0
            || !self.material.viscosity.is_finite()
            || self.material.viscosity <= 0.0
            || !self.surface_tension_n_m.is_finite()
            || self.surface_tension_n_m <= 0.0
            || !self.yield_stress_pa.is_finite()
            || self.yield_stress_pa < 0.0
        {
            return Err(Error::InvalidMaterial);
        }
        Ok(())
    }
}

/// Pressurized nozzle shooting a continuous or pulsed stream of liquid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidJetNozzle {
    pub position_m: [f64; 3],
    pub direction: [f64; 3],
    pub orifice_diameter_m: f64,
    pub exit_speed_m_s: f64,
    pub cone_half_angle_rad: f64,
    pub preset: JetFluidPreset,
}

/// Output of advancing a [`LiquidJetNozzle`] over a timestep `dt_s`.
#[derive(Clone, Debug)]
pub struct JetEmissionStep {
    /// Newly emitted coherent core SPH particles.
    pub core_particles: Vec<Particle>,
    /// Newly atomized peripheral droplets from Rayleigh–Plateau / aerodynamic stripping.
    pub spray_droplets: Vec<crate::suspension::Particle>,
    /// Total mass (kg) discharged during `dt_s`.
    pub discharged_mass_kg: f64,
    /// Reaction force (N) exerted by the jet on the nozzle body ($-\dot{m} \mathbf{v}_{\text{exit}}$).
    pub nozzle_recoil_force_n: [f64; 3],
    /// Intact liquid core breakup length $L_b$ (m).
    pub breakup_length_m: f64,
    /// Dimensionless jet Weber number $\text{We} = \rho U^2 D / \sigma$.
    pub weber_number: f64,
    /// Dimensionless jet Ohnesorge number $\text{Oh} = \mu / \sqrt{\rho \sigma D}$.
    pub ohnesorge_number: f64,
}

impl LiquidJetNozzle {
    pub fn validate(&self) -> Result<[f64; 3], Error> {
        self.preset.validate()?;
        if self.position_m.iter().any(|x| !x.is_finite())
            || self.direction.iter().any(|x| !x.is_finite())
            || !self.orifice_diameter_m.is_finite()
            || self.orifice_diameter_m <= 0.0
            || !self.exit_speed_m_s.is_finite()
            || self.exit_speed_m_s <= 0.0
            || !self.cone_half_angle_rad.is_finite()
            || !(0.0..=(0.45 * PI)).contains(&self.cone_half_angle_rad)
        {
            return Err(Error::InvalidConfig);
        }
        let d_norm = norm(self.direction);
        if !d_norm.is_finite() || d_norm <= 1e-12 {
            return Err(Error::InvalidConfig);
        }
        Ok(scale(self.direction, 1.0 / d_norm))
    }

    /// Jet Weber number $\text{We} = \rho U^2 D / \sigma$.
    pub fn weber_number(&self) -> Result<f64, Error> {
        self.validate()?;
        let rho = self.preset.material.rest_density;
        let u = self.exit_speed_m_s;
        let d = self.orifice_diameter_m;
        let sigma = self.preset.surface_tension_n_m;
        Ok(rho * u * u * d / sigma)
    }

    /// Jet Ohnesorge number $\text{Oh} = \mu / \sqrt{\rho \sigma D}$.
    pub fn ohnesorge_number(&self) -> Result<f64, Error> {
        self.validate()?;
        let rho = self.preset.material.rest_density;
        let mu = self.preset.material.viscosity;
        let d = self.orifice_diameter_m;
        let sigma = self.preset.surface_tension_n_m;
        Ok(mu / (rho * sigma * d).sqrt())
    }

    /// Rayleigh–Plateau intact core breakup length $L_b = C_b D \sqrt{\text{We}} (1 + 3\,\text{Oh})$.
    pub fn rayleigh_plateau_breakup_length_m(&self) -> Result<f64, Error> {
        let we = self.weber_number()?;
        let oh = self.ohnesorge_number()?;
        let c_b = 3.2;
        Ok(c_b * self.orifice_diameter_m * we.sqrt() * (1.0 + 3.0 * oh))
    }

    /// Continuous mass flow rate $\dot{m} = \rho (\pi D^2 / 4) U$ in kg/s.
    pub fn mass_flow_rate_kg_s(&self) -> Result<f64, Error> {
        self.validate()?;
        let area = 0.25 * PI * self.orifice_diameter_m * self.orifice_diameter_m;
        Ok(self.preset.material.rest_density * area * self.exit_speed_m_s)
    }

    /// Reaction thrust vector (N) on the nozzle mount: $\mathbf{F}_{\text{recoil}} = -\dot{m} U \hat{\mathbf{d}}$.
    pub fn nozzle_recoil_force_n(&self) -> Result<[f64; 3], Error> {
        let dir = self.validate()?;
        let m_dot = self.mass_flow_rate_kg_s()?;
        Ok(scale(dir, -m_dot * self.exit_speed_m_s))
    }

    /// Emits one timestep `dt_s` of jet core SPH particles and peripheral Rayleigh–Plateau
    /// spray droplets, conserving total discharged mass and axial momentum.
    pub fn emit_jet_step(
        &self,
        dt_s: f64,
        material_index: usize,
    ) -> Result<JetEmissionStep, Error> {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        let dir = self.validate()?;
        let we = self.weber_number()?;
        let oh = self.ohnesorge_number()?;
        let breakup_length_m = self.rayleigh_plateau_breakup_length_m()?;
        let m_dot = self.mass_flow_rate_kg_s()?;
        let discharged_mass_kg = m_dot * dt_s;
        let recoil = self.nozzle_recoil_force_n()?;

        // Atomization fraction increases with Weber number above Rayleigh-Plateau onset (We > 12)
        // and decreases with high viscosity (Ohnesorge damping) and yield stress.
        let yield_damping = 1.0
            / (1.0
                + self.preset.yield_stress_pa
                    / (0.5 * self.preset.material.rest_density * self.exit_speed_m_s.powi(2))
                        .max(1.0));
        let spray_fraction = if we > 12.0 {
            (((we - 12.0) / (we + 120.0)) * (1.0 / (1.0 + 2.5 * oh)) * yield_damping)
                .clamp(0.0, 0.35)
        } else {
            0.0
        };

        let spray_mass_kg = discharged_mass_kg * spray_fraction;
        let core_mass_kg = discharged_mass_kg - spray_mass_kg;

        // Build orthonormal basis (dir, tangent_u, tangent_v)
        let ref_axis = if dir[0].abs() < 0.8 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let u_raw = cross(dir, ref_axis);
        let tangent_u = scale(u_raw, 1.0 / norm(u_raw));
        let tangent_v = cross(dir, tangent_u);

        let core_count = 4usize;
        let particle_mass = core_mass_kg / (core_count as f64);
        let mut core_particles = Vec::with_capacity(core_count);
        for k in 0..core_count {
            let s = ((k as f64) + 0.5) / (core_count as f64);
            let axial_offset = s * self.exit_speed_m_s * dt_s;
            let pos = add(self.position_m, scale(dir, axial_offset));
            core_particles.push(Particle {
                position: pos,
                velocity: scale(dir, self.exit_speed_m_s),
                mass: particle_mass,
                material: material_index,
            });
        }

        let mut spray_droplets = Vec::new();
        if spray_mass_kg > 0.0 {
            let droplet_count = 4usize;
            let single_droplet_mass = spray_mass_kg / (droplet_count as f64);
            let rho = self.preset.material.rest_density;
            let droplet_radius = ((3.0 * single_droplet_mass) / (4.0 * PI * rho)).cbrt();
            let lateral_speed = self.exit_speed_m_s * self.cone_half_angle_rad.tan();
            let ring_dirs = [
                tangent_u,
                scale(tangent_u, -1.0),
                tangent_v,
                scale(tangent_v, -1.0),
            ];
            for &r_dir in &ring_dirs {
                let pos = add(
                    add(
                        self.position_m,
                        scale(dir, 0.5 * self.exit_speed_m_s * dt_s),
                    ),
                    scale(r_dir, 0.5 * self.orifice_diameter_m),
                );
                // Symmetric opposing pairs cancel lateral momentum exactly so axial momentum is conserved
                let vel = add(scale(dir, self.exit_speed_m_s), scale(r_dir, lateral_speed));
                let droplet = crate::suspension::Particle::new(droplet_radius, rho, pos, vel)
                    .map_err(|_| Error::InvalidParticle)?;
                spray_droplets.push(droplet);
            }
        }

        Ok(JetEmissionStep {
            core_particles,
            spray_droplets,
            discharged_mass_kg,
            nozzle_recoil_force_n: recoil,
            breakup_length_m,
            weber_number: we,
            ohnesorge_number: oh,
        })
    }
}

/// Outcome of an oblique liquid jet parcel impacting a solid or deformable surface.
#[derive(Clone, Debug)]
pub struct JetSurfaceImpactReport {
    /// Mass (kg) captured into the surface film at the impact patch.
    pub deposited_film_mass_kg: f64,
    /// Volume (m³) captured into the surface film.
    pub deposited_film_volume_m3: f64,
    /// Tangential momentum impulse (N·s) delivered to smear the surface film along the wall.
    pub tangential_smear_impulse_n_s: [f64; 3],
    /// Normal impact impulse (N·s) delivered into the solid substrate.
    pub normal_substrate_impulse_n_s: [f64; 3],
    /// Secondary crown-splash droplets ejected back into the gas medium.
    pub splash_particles: Vec<crate::suspension::Particle>,
    /// Total mass (kg) carried away by `splash_particles`.
    pub splashed_mass_kg: f64,
    /// Mundo–Sommerfeld splash parameter $K = \text{We}_n^{1/2} \text{Re}_n^{1/4}$.
    pub mundo_sommerfeld_k: f64,
}

/// Resolves an oblique jet parcel impact against a surface with outward unit `surface_normal`:
/// - Computes normal Weber ($\text{We}_n$) and Reynolds ($\text{Re}_n$) numbers and the
///   Mundo–Sommerfeld splash parameter $K = \text{We}_n^{1/2} \text{Re}_n^{1/4}$.
/// - Splits incoming mass strictly between deposited thin-film mass and secondary crown-splash
///   droplets (`deposited_film_mass_kg + splashed_mass_kg == parcel_mass_kg`).
/// - Extracts the tangential momentum impulse driving directional smearing (`размазывание`)
///   along the substrate.
pub fn resolve_jet_surface_impact(
    parcel_mass_kg: f64,
    parcel_velocity_m_s: [f64; 3],
    impact_point_m: [f64; 3],
    surface_normal: [f64; 3],
    preset: JetFluidPreset,
    critical_splash_k: f64,
) -> Result<JetSurfaceImpactReport, Error> {
    preset.validate()?;
    if !parcel_mass_kg.is_finite()
        || parcel_mass_kg <= 0.0
        || parcel_velocity_m_s.iter().any(|v| !v.is_finite())
        || impact_point_m.iter().any(|p| !p.is_finite())
        || surface_normal.iter().any(|n| !n.is_finite())
        || !critical_splash_k.is_finite()
        || critical_splash_k <= 0.0
    {
        return Err(Error::InvalidConfig);
    }
    let n_len = norm(surface_normal);
    if !n_len.is_finite() || n_len <= 1e-12 {
        return Err(Error::InvalidSurface);
    }
    let normal = scale(surface_normal, 1.0 / n_len);
    let vn_signed = dot(parcel_velocity_m_s, normal);
    if vn_signed >= 0.0 {
        return Err(Error::InvalidConfig);
    }
    let vn = -vn_signed;
    let v_tangent = sub(parcel_velocity_m_s, scale(normal, vn_signed));

    let rho = preset.material.rest_density;
    let mu = preset.material.viscosity;
    let sigma = preset.surface_tension_n_m;
    let equiv_diameter = ((6.0 * parcel_mass_kg) / (PI * rho)).cbrt();

    let we_n = rho * vn * vn * equiv_diameter / sigma;
    let re_n = rho * vn * equiv_diameter / mu;
    let mundo_sommerfeld_k = we_n.sqrt() * re_n.powf(0.25);

    // Yield-stress and viscoelastic fluids suppress splash breakup and favor smearing/deposition
    let rheology_suppression = 1.0
        / (1.0
            + preset.yield_stress_pa / (0.5 * rho * vn * vn).max(1.0)
            + preset.maxwell.map_or(0.0, |m| m.relaxation_time * 4.0));

    let splash_fraction = if mundo_sommerfeld_k > critical_splash_k {
        let excess =
            (mundo_sommerfeld_k - critical_splash_k) / (mundo_sommerfeld_k + critical_splash_k);
        (0.55 * excess * rheology_suppression).clamp(0.0, 0.5)
    } else {
        0.0
    };

    let mut splash_particles = Vec::new();
    let splashed_mass_kg = if splash_fraction > 0.0 {
        let count = 4usize;
        let target_splash_mass = parcel_mass_kg * splash_fraction;
        let drop_mass = target_splash_mass / (count as f64);
        let drop_radius = ((3.0 * drop_mass) / (4.0 * PI * rho)).cbrt();

        let ref_axis = if normal[0].abs() < 0.8 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let u_raw = cross(normal, ref_axis);
        let tangent_u = scale(u_raw, 1.0 / norm(u_raw));
        let tangent_v = cross(normal, tangent_u);
        let azimuths = [
            tangent_u,
            scale(tangent_u, -1.0),
            tangent_v,
            scale(tangent_v, -1.0),
        ];

        for &az in &azimuths {
            let pos = add(
                add(impact_point_m, scale(normal, 2.0 * drop_radius)),
                scale(az, equiv_diameter),
            );
            // Crown splash ejects at ~35 deg above wall + carries tangential forward velocity
            let vel = add(
                add(scale(v_tangent, 0.65), scale(normal, 0.35 * vn)),
                scale(az, 0.30 * vn),
            );
            let p = crate::suspension::Particle::new(drop_radius, rho, pos, vel)
                .map_err(|_| Error::InvalidParticle)?;
            splash_particles.push(p);
        }
        splash_particles
            .iter()
            .map(crate::suspension::Particle::mass_kg)
            .sum()
    } else {
        0.0
    };

    let deposited_film_mass_kg = (parcel_mass_kg - splashed_mass_kg).max(0.0);
    let deposited_film_volume_m3 = deposited_film_mass_kg / rho;

    // Tangential smear impulse transferred into the deposited film
    let tangential_smear_impulse_n_s = scale(v_tangent, deposited_film_mass_kg);
    // Normal impulse transferred into the solid substrate (including splash rebound reaction)
    let rebound_normal_momentum: f64 = splash_particles
        .iter()
        .map(|p| p.mass_kg() * dot(p.velocity_m_s(), normal))
        .sum();
    let normal_substrate_impulse_n_s =
        scale(normal, -(parcel_mass_kg * vn + rebound_normal_momentum));

    Ok(JetSurfaceImpactReport {
        deposited_film_mass_kg,
        deposited_film_volume_m3,
        tangential_smear_impulse_n_s,
        normal_substrate_impulse_n_s,
        splash_particles,
        splashed_mass_kg,
        mundo_sommerfeld_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_liquid_presets_emit_jets_with_recoil_and_breakup_scaling() {
        let presets = [
            JetFluidPreset::WATER,
            JetFluidPreset::OIL,
            JetFluidPreset::GLYCERIN,
            JetFluidPreset::BRINE,
            JetFluidPreset::MUD_SLURRY,
            JetFluidPreset::VISCOELASTIC_GEL,
            JetFluidPreset::PAINT_OR_BLOOD,
        ];
        for (idx, preset) in presets.into_iter().enumerate() {
            let nozzle = LiquidJetNozzle {
                position_m: [0.0, 1.2, 0.0],
                direction: [1.0, 0.0, 0.0],
                orifice_diameter_m: 0.008,
                exit_speed_m_s: 14.0,
                cone_half_angle_rad: 0.08,
                preset,
            };
            let step = nozzle.emit_jet_step(0.01, idx).unwrap();
            let core_mass: f64 = step.core_particles.iter().map(|p| p.mass).sum();
            let spray_mass: f64 = step
                .spray_droplets
                .iter()
                .map(crate::suspension::Particle::mass_kg)
                .sum();
            assert!((core_mass + spray_mass - step.discharged_mass_kg).abs() < 1e-12);
            assert!(step.nozzle_recoil_force_n[0] < 0.0);
            assert!(step.breakup_length_m > 0.0);
        }

        // High-viscosity glycerin has a higher Ohnesorge number and longer intact breakup length than water
        let water_nozzle = LiquidJetNozzle {
            position_m: [0.0; 3],
            direction: [0.0, 0.0, 1.0],
            orifice_diameter_m: 0.006,
            exit_speed_m_s: 12.0,
            cone_half_angle_rad: 0.05,
            preset: JetFluidPreset::WATER,
        };
        let glycerin_nozzle = LiquidJetNozzle {
            preset: JetFluidPreset::GLYCERIN,
            ..water_nozzle
        };
        assert!(
            glycerin_nozzle.ohnesorge_number().unwrap() > water_nozzle.ohnesorge_number().unwrap()
        );
        assert!(
            glycerin_nozzle.rayleigh_plateau_breakup_length_m().unwrap()
                > water_nozzle.rayleigh_plateau_breakup_length_m().unwrap()
        );
    }

    #[test]
    fn oblique_jet_surface_impact_conserves_mass_and_splits_film_smear_and_splash() {
        let parcel_mass = 0.004;
        let vel = [8.0, -10.0, 2.0];
        let normal = [0.0, 1.0, 0.0];
        let water_impact = resolve_jet_surface_impact(
            parcel_mass,
            vel,
            [0.0, 0.0, 0.0],
            normal,
            JetFluidPreset::WATER,
            57.7,
        )
        .unwrap();
        assert!(water_impact.mundo_sommerfeld_k > 57.7);
        assert!(!water_impact.splash_particles.is_empty());
        assert!(
            (water_impact.deposited_film_mass_kg + water_impact.splashed_mass_kg - parcel_mass)
                .abs()
                < 1e-14
        );
        assert!(water_impact.tangential_smear_impulse_n_s[0] > 0.0);
        assert!(water_impact.normal_substrate_impulse_n_s[1] < 0.0);

        // Viscoplastic mud slurry at the same velocity suppresses splashing and deposits more film mass
        let mud_impact = resolve_jet_surface_impact(
            parcel_mass,
            vel,
            [0.0, 0.0, 0.0],
            normal,
            JetFluidPreset::MUD_SLURRY,
            57.7,
        )
        .unwrap();
        assert!(
            (mud_impact.deposited_film_mass_kg + mud_impact.splashed_mass_kg - parcel_mass).abs()
                < 1e-14
        );
        assert!(mud_impact.deposited_film_mass_kg > water_impact.deposited_film_mass_kg);
    }
}
