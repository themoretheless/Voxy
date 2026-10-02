//! Explicit dry/saturated property calibration. No universal wet softening law.
use super::Cell;
#[derive(Clone, Copy, Debug)]
pub struct Properties {
    pub young_pa: f64,
    pub poisson: f64,
    pub yield_pa: f64,
    pub hardening_pa: f64,
    pub hardness_pa: f64,
    pub wear_coefficient: f64,
}
impl Properties {
    /// # Errors
    /// Invalid elastic/plastic parameters or derived moduli.
    pub fn plastic_material(self) -> Result<crate::plasticity::Material, &'static str> {
        crate::plasticity::Material::new(
            self.young_pa,
            self.poisson,
            self.yield_pa,
            self.hardening_pa,
        )
    }
    /// # Errors
    /// Invalid hardness or empirical wear coefficient.
    pub fn wear_material(self) -> Result<crate::wear::Material, &'static str> {
        crate::wear::Material::new(self.hardness_pa, self.wear_coefficient)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Calibration {
    dry: Properties,
    saturated: Properties,
}
impl Calibration {
    /// User-provided calibrated endpoints with an explicitly linear saturation interpolation.
    /// Either strengthening or weakening is permitted. This is a chosen empirical
    /// interpolation, not a constitutive prediction or hysteretic sorption law.
    /// # Errors
    /// Invalid endpoint material parameters.
    pub fn new(dry: Properties, saturated: Properties) -> Result<Self, &'static str> {
        for p in [dry, saturated] {
            p.plastic_material()?;
            p.wear_material()?;
        }
        Ok(Self { dry, saturated })
    }
    /// # Errors
    /// Invalid saturation or unrepresentable interpolated material.
    pub fn at(self, saturation: f64) -> Result<Properties, &'static str> {
        if !saturation.is_finite() || !(0. ..=1.).contains(&saturation) {
            return Err("invalid material saturation");
        }
        let mix = |a: f64, b: f64| (1. - saturation) * a + saturation * b;
        let result = Properties {
            young_pa: mix(self.dry.young_pa, self.saturated.young_pa),
            poisson: mix(self.dry.poisson, self.saturated.poisson),
            yield_pa: mix(self.dry.yield_pa, self.saturated.yield_pa),
            hardening_pa: mix(self.dry.hardening_pa, self.saturated.hardening_pa),
            hardness_pa: mix(self.dry.hardness_pa, self.saturated.hardness_pa),
            wear_coefficient: mix(self.dry.wear_coefficient, self.saturated.wear_coefficient),
        };
        result.plastic_material()?;
        result.wear_material()?;
        Ok(result)
    }
}
impl Cell {
    /// # Errors
    /// Invalid water capacity or inventory.
    pub fn saturation(self) -> Result<f64, &'static str> {
        if !self.capacity_kg.is_finite()
            || self.capacity_kg <= 0.
            || !self.water_kg.is_finite()
            || self.water_kg < 0.
            || self.water_kg > self.capacity_kg
        {
            return Err("invalid moisture inventory");
        }
        Ok(self.water_kg / self.capacity_kg)
    }
    /// Add actual absorbed water mass to dry skeleton mass per reference volume.
    /// This does not assume that saturation changes material volume. Momentum of
    /// incoming water and changing constitutive energy must be handled by the
    /// mechanical adapter when applying updated properties/inertia to a live body.
    /// # Errors
    /// Invalid inventory/mass/volume or nonrepresentable wet density.
    pub fn wet_density_kg_m3(
        self,
        dry_mass_kg: f64,
        reference_volume_m3: f64,
    ) -> Result<f64, &'static str> {
        self.saturation()?;
        let density = (dry_mass_kg + self.water_kg) / reference_volume_m3;
        if !dry_mass_kg.is_finite()
            || dry_mass_kg <= 0.
            || !reference_volume_m3.is_finite()
            || reference_volume_m3 <= 0.
            || !density.is_finite()
            || density <= 0.
        {
            return Err("invalid wet reference density");
        }
        Ok(density)
    }
    /// # Errors
    /// Invalid inventory or interpolated properties.
    pub fn properties(self, calibration: Calibration) -> Result<Properties, &'static str> {
        calibration.at(self.saturation()?)
    }
}
