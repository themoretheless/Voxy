//! Osmotic pressure law shared by the vascular-pore and lymphatic models.

/// Explicit nonnegative virial polynomial Pi(C)=a*C+b*C²+c*C³.
/// C is kg/m³; coefficient units are Pa m³/kg, Pa m⁶/kg², Pa m⁹/kg³.
/// Coefficients require material calibration; no physiological defaults are supplied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OsmoticPressureLaw {
    pub linear: f64,
    pub quadratic: f64,
    pub cubic: f64,
}
impl OsmoticPressureLaw {
    /// Evaluate with checked coefficients and concentration; never clip overflow.
    pub fn pressure_pa(self, concentration_kg_per_m3: f64) -> Result<f64, &'static str> {
        if [
            self.linear,
            self.quadratic,
            self.cubic,
            concentration_kg_per_m3,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("invalid osmotic pressure law");
        }
        let c = concentration_kg_per_m3;
        let pressure = c * (self.linear + c * (self.quadratic + c * self.cubic));
        if !pressure.is_finite() {
            return Err("osmotic pressure overflow");
        }
        Ok(pressure)
    }
    pub(crate) fn active(self) -> bool {
        self.linear > 0. || self.quadratic > 0. || self.cubic > 0.
    }
}
