//! Calibrated initial cohesive law at a prescribed material saturation.
#[derive(Clone, Copy, Debug)]
pub struct CohesiveProperties {
    pub stiffness_pa_m: f64,
    pub closure_pa_m: f64,
    pub peak_pa: f64,
    pub fracture_j_m2: f64,
}
impl CohesiveProperties {
    /// # Errors
    /// Invalid stiffness/strength/toughness or separation range.
    pub fn material(self) -> Result<crate::cohesive::Material, &'static str> {
        crate::cohesive::Material::new(
            self.stiffness_pa_m,
            self.closure_pa_m,
            self.peak_pa,
            self.fracture_j_m2,
        )
    }
}
#[derive(Clone, Copy, Debug)]
pub struct CohesiveCalibration {
    dry: CohesiveProperties,
    saturated: CohesiveProperties,
}
impl CohesiveCalibration {
    /// Empirical linear interpolation of stiffness, peak traction and toughness.
    /// Use for newly initialized interfaces. This does not migrate a damaged
    /// history between laws; replacing an accepted law can otherwise heal damage.
    /// # Errors
    /// Invalid endpoint laws. Each interpolated law is validated again at use.
    pub fn new(
        dry: CohesiveProperties,
        saturated: CohesiveProperties,
    ) -> Result<Self, &'static str> {
        dry.material()?;
        saturated.material()?;
        Ok(Self { dry, saturated })
    }
    /// # Errors
    /// Invalid saturation or invalid interpolated cohesive separation range.
    pub fn at(self, saturation: f64) -> Result<crate::cohesive::Material, &'static str> {
        if !saturation.is_finite() || !(0. ..=1.).contains(&saturation) {
            return Err("invalid cohesive saturation");
        }
        let mix = |a: f64, b: f64| (1. - saturation) * a + saturation * b;
        CohesiveProperties {
            stiffness_pa_m: mix(self.dry.stiffness_pa_m, self.saturated.stiffness_pa_m),
            closure_pa_m: mix(self.dry.closure_pa_m, self.saturated.closure_pa_m),
            peak_pa: mix(self.dry.peak_pa, self.saturated.peak_pa),
            fracture_j_m2: mix(self.dry.fracture_j_m2, self.saturated.fracture_j_m2),
        }
        .material()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ThermalCohesiveCalibration {
    cold_k: f64,
    hot_k: f64,
    cold: CohesiveCalibration,
    hot: CohesiveCalibration,
}
impl ThermalCohesiveCalibration {
    /// Bounded empirical interpolation of dry/saturated cohesive endpoints.
    /// No extrapolation, thermal expansion or assumed universal weakening law.
    pub fn new(
        cold_k: f64,
        hot_k: f64,
        cold: CohesiveCalibration,
        hot: CohesiveCalibration,
    ) -> Result<Self, &'static str> {
        if !cold_k.is_finite()
            || !hot_k.is_finite()
            || cold_k <= 0.
            || hot_k <= cold_k
            || !(hot_k - cold_k).is_finite()
        {
            return Err("invalid cohesive temperature calibration domain");
        }
        Ok(Self {
            cold_k,
            hot_k,
            cold,
            hot,
        })
    }
    pub fn at_temperature(self, temperature_k: f64) -> Result<CohesiveCalibration, &'static str> {
        if !temperature_k.is_finite() || temperature_k < self.cold_k || temperature_k > self.hot_k {
            return Err("cohesive temperature outside calibration domain");
        }
        let t = (temperature_k - self.cold_k) / (self.hot_k - self.cold_k);
        let mix = |a: CohesiveProperties, b: CohesiveProperties| {
            let lerp = |x: f64, y: f64| (1. - t) * x + t * y;
            CohesiveProperties {
                stiffness_pa_m: lerp(a.stiffness_pa_m, b.stiffness_pa_m),
                closure_pa_m: lerp(a.closure_pa_m, b.closure_pa_m),
                peak_pa: lerp(a.peak_pa, b.peak_pa),
                fracture_j_m2: lerp(a.fracture_j_m2, b.fracture_j_m2),
            }
        };
        CohesiveCalibration::new(
            mix(self.cold.dry, self.hot.dry),
            mix(self.cold.saturated, self.hot.saturated),
        )
    }
}
