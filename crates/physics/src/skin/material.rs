use super::dual::D;
/// A Maxwell branch acting on the objective Green strain. Modulus in Pa, time in s.
#[derive(Clone, Copy, Debug)]
pub struct Relaxation {
    pub modulus: f64,
    pub time: f64,
}
/// A homogeneous epidermal or dermal layer; all fields use SI units.
#[derive(Clone, Debug)]
pub struct Layer {
    pub thickness: f64,
    pub density: f64,
    pub shear_modulus: f64,
    pub collagen_modulus: f64,
    pub collagen_exponent: f64,
    /// GOH dispersion, between 0 (aligned) and 1/3 (isotropic).
    pub dispersion: f64,
    /// Two fiber directions are symmetric about the material direction, in radians.
    pub fiber_angle: f64,
    pub relaxation: Vec<Relaxation>,
}
/// Layers share the midsurface strain, with incompressible thickness stretch.
/// Defaults are an illustrative parameter set, not a patient-specific calibration.
#[derive(Clone, Debug)]
pub struct SkinMaterial {
    pub layers: Vec<Layer>,
}
impl Default for SkinMaterial {
    fn default() -> Self {
        Self {
            layers: vec![
                Layer {
                    thickness: 0.00015,
                    density: 1100.0,
                    shear_modulus: 40_000.0,
                    collagen_modulus: 0.0,
                    collagen_exponent: 4.0,
                    dispersion: 0.14,
                    fiber_angle: 0.7156,
                    relaxation: vec![Relaxation {
                        modulus: 20_000.0,
                        time: 0.1,
                    }],
                },
                Layer {
                    thickness: 0.0015,
                    density: 1100.0,
                    shear_modulus: 20_000.0,
                    collagen_modulus: 200_000.0,
                    collagen_exponent: 8.0,
                    dispersion: 0.14,
                    fiber_angle: 0.7156,
                    relaxation: vec![
                        Relaxation {
                            modulus: 40_000.0,
                            time: 0.2,
                        },
                        Relaxation {
                            modulus: 20_000.0,
                            time: 2.0,
                        },
                    ],
                },
            ],
        }
    }
}
impl SkinMaterial {
    pub(super) fn validate(&self) -> Result<(), &'static str> {
        if self.layers.is_empty() || self.layers.len() > 16 {
            return Err("skin needs 1..=16 layers");
        }
        for l in &self.layers {
            if [l.thickness, l.density, l.shear_modulus, l.collagen_exponent]
                .iter()
                .any(|v| !v.is_finite() || *v <= 0.0)
                || !l.collagen_modulus.is_finite()
                || l.collagen_modulus < 0.0
                || !l.dispersion.is_finite()
                || !(0.0..=1.0 / 3.0).contains(&l.dispersion)
                || !l.fiber_angle.is_finite()
                || l.relaxation.len() > 16
                || l.relaxation.iter().any(|r| {
                    !r.modulus.is_finite()
                        || r.modulus < 0.0
                        || !r.time.is_finite()
                        || r.time <= 0.0
                })
            {
                return Err("invalid skin layer");
            }
        }
        if !self.thickness().is_finite()
            || !self.area_density().is_finite()
            || self.area_density() <= 0.0
            || !self.bending_rigidity().is_finite()
        {
            return Err("overflowing skin laminate properties");
        }
        Ok(())
    }
    #[must_use]
    pub fn thickness(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness).sum()
    }
    #[must_use]
    pub fn area_density(&self) -> f64 {
        self.layers.iter().map(|l| l.density * l.thickness).sum()
    }
    /// Flexural rigidity about the stiffness-weighted neutral axis, in N m.
    /// Uses the matrix small-strain E=3 mu, nu=0.5, not the recruited collagen tangent.
    #[must_use]
    pub fn bending_rigidity(&self) -> f64 {
        let mut z = 0.0;
        let mut weighted = 0.0;
        let mut stiffness = 0.0;
        for l in &self.layers {
            let q = 4.0 * l.shear_modulus;
            weighted += q * l.thickness * (z + l.thickness * 0.5);
            stiffness += q * l.thickness;
            z += l.thickness;
        }
        let neutral = weighted / stiffness;
        z = 0.0;
        let mut d = 0.0;
        for l in &self.layers {
            d += 4.0
                * l.shear_modulus
                * ((z + l.thickness - neutral).powi(3) - (z - neutral).powi(3))
                / 3.0;
            z += l.thickness;
        }
        d
    }
    /// Elastic energy per undeformed midsurface area, in J/m². C=[C11,C22,C12].
    /// # Errors
    /// Rejects invalid materials and non-positive or overflowing metrics.
    pub fn response(&self, c: [f64; 3]) -> Result<MaterialResponse, &'static str> {
        self.validate()?;
        let x = std::array::from_fn(|i| D::<3>::variable(c[i], i));
        let energy = self.energy(x, 0.0);
        if !energy.finite() || c[0] <= 0.0 || c[1] <= 0.0 || c[0] * c[1] - c[2] * c[2] <= 1e-12 {
            return Err("invalid skin metric");
        }
        let thickness = self.thickness() / (c[0] * c[1] - c[2] * c[2]).sqrt();
        if !thickness.is_finite() {
            return Err("skin thickness overflow");
        }
        Ok(MaterialResponse {
            energy: energy.v,
            gradient: energy.g,
            tangent: energy.h,
            thickness,
        })
    }
    pub(super) fn energy<const N: usize>(&self, c: [D<N>; 3], direction: f64) -> D<N> {
        let [a, b, s] = c;
        let determinant = a * b - s * s;
        let i1 = a + b + determinant.reciprocal();
        let mut energy = D::c(0.0);
        for l in &self.layers {
            let mut w = (i1 - D::c(3.0)) * (0.5 * l.shear_modulus);
            for sign in [-1.0, 1.0] {
                let angle = direction + sign * l.fiber_angle;
                let (y, x) = angle.sin_cos();
                let i4 = a * (x * x) + b * (y * y) + s * (2.0 * x * y);
                let e = ((i1 - D::c(3.0)) * l.dispersion
                    + (i4 - D::c(1.0)) * (1.0 - 3.0 * l.dispersion))
                    .positive();
                w = w
                    + ((e.square() * l.collagen_exponent).exp() - D::c(1.0))
                        * (l.collagen_modulus / (2.0 * l.collagen_exponent));
            }
            energy = energy + w * l.thickness;
        }
        energy
    }
}
#[derive(Clone, Copy, Debug)]
pub struct MaterialResponse {
    pub energy: f64,
    pub gradient: [f64; 3],
    pub tangent: [[f64; 3]; 3],
    pub thickness: f64,
}
