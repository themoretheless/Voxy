//! Continuum hyperelastic constitutive kernel, invariants and spatial stress diagnostics.

pub use crate::math::Matrix;
use crate::math::{cross, scale};

pub type Vec3 = [f64; 3];
pub const IDENTITY: Matrix = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn transpose(a: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}

fn mv(a: Matrix, x: Vec3) -> Vec3 {
    a.map(|r| dot(r, x))
}

fn mm(a: Matrix, b: Matrix) -> Matrix {
    [
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0] + a[0][2] * b[2][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1] + a[0][2] * b[2][1],
            a[0][0] * b[0][2] + a[0][1] * b[1][2] + a[0][2] * b[2][2],
        ],
        [
            a[1][0] * b[0][0] + a[1][1] * b[1][0] + a[1][2] * b[2][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1] + a[1][2] * b[2][1],
            a[1][0] * b[0][2] + a[1][1] * b[1][2] + a[1][2] * b[2][2],
        ],
        [
            a[2][0] * b[0][0] + a[2][1] * b[1][0] + a[2][2] * b[2][0],
            a[2][0] * b[0][1] + a[2][1] * b[1][1] + a[2][2] * b[2][1],
            a[2][0] * b[0][2] + a[2][1] * b[1][2] + a[2][2] * b[2][2],
        ],
    ]
}

fn outer(a: Vec3, b: Vec3) -> Matrix {
    std::array::from_fn(|i| b.map(|x| a[i] * x))
}

fn det(a: Matrix) -> f64 {
    dot(a[0], cross(a[1], a[2]))
}

fn inverse(a: Matrix) -> Result<Matrix, &'static str> {
    let d = det(a);
    if !d.is_finite() || d.abs() < 1e-18 {
        return Err("singular deformation");
    }
    Ok(transpose([
        scale(cross(a[1], a[2]), 1. / d),
        scale(cross(a[2], a[0]), 1. / d),
        scale(cross(a[0], a[1]), 1. / d),
    ]))
}

#[doc(hidden)]
#[must_use]
pub fn isochoric_excess(f: Matrix, q: f64, isochoric_trace: f64) -> f64 {
    isochoric_metric_excess(
        mm(transpose(f), f).map(|r| r.map(|v| q * v)),
        isochoric_trace,
    )
}

/// Reuse an already admitted isochoric metric without rebuilding `F^T F`.
#[doc(hidden)]
#[must_use]
pub fn isochoric_metric_excess(metric: Matrix, isochoric_trace: f64) -> f64 {
    let mut deviation = metric;
    for (i, row) in deviation.iter_mut().enumerate() {
        row[i] -= 1.;
    }
    if deviation.iter().flatten().all(|v| v.abs() < 0.1) {
        let second = deviation[0][0] * deviation[1][1]
            + deviation[0][0] * deviation[2][2]
            + deviation[1][1] * deviation[2][2]
            - deviation[0][1] * deviation[1][0]
            - deviation[0][2] * deviation[2][0]
            - deviation[1][2] * deviation[2][1];
        (-second - det(deviation)).max(0.)
    } else {
        isochoric_trace - 3.
    }
}

/// Computes the $\overline{F}$-projected deformation gradient
/// $\overline{F} = (\overline{J} / J)^{1/3} F$ (de Souza Neto et al.) to eliminate
/// volumetric locking in nearly incompressible 4-node tetrahedral patches where
/// $\overline{J} > 0$ is the patch-averaged Jacobian determinant.
///
/// # Errors
///
/// Rejects non-finite matrices, inverted elements ($J \le 0$), or non-positive $\overline{J}$.
pub fn f_bar_modified_gradient(f: Matrix, j_bar: f64) -> Result<Matrix, &'static str> {
    if f.iter().flatten().any(|v| !v.is_finite()) || !j_bar.is_finite() || j_bar <= 0.0 {
        return Err("invalid F-bar deformation input");
    }
    let j = det(f);
    if !j.is_finite() || j <= 0.0 {
        return Err("inverted element in F-bar projection");
    }
    let scale_factor = (j_bar / j).cbrt();
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err("F-bar scale overflow");
    }
    Ok(f.map(|row| row.map(|v| v * scale_factor)))
}

/// Explicit compact bell-shaped active force/stretch curve; parameters are not fitted.
#[derive(Clone, Copy, Debug)]
pub struct ActiveFiberLengthLaw {
    pub optimal_stretch: f64,
    pub half_width: f64,
}

impl ActiveFiberLengthLaw {
    #[doc(hidden)]
    pub fn validate(self) -> Result<(), &'static str> {
        if [self.optimal_stretch, self.half_width]
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.)
        {
            return Err("invalid active fiber length law");
        }
        Ok(())
    }

    /// Returns the normalized active tension and its dimensionless potential,
    /// with zero potential at reference stretch one. Derivative of potential is tension.
    /// # Errors
    /// Invalid parameters/stretch or derived nonfinite response.
    pub fn response(self, stretch: f64) -> Result<(f64, f64), &'static str> {
        self.validate()?;
        if !stretch.is_finite() || stretch <= 0. {
            return Err("invalid active fiber stretch");
        }
        let x = (stretch - self.optimal_stretch) / self.half_width;
        let primitive = |v: f64| {
            let t = v.clamp(-1., 1.);
            t - 2. * t.powi(3) / 3. + t.powi(5) / 5.
        };
        let tension = if x.abs() < 1. {
            (1. - x * x).powi(2)
        } else {
            0.
        };
        let potential = self.half_width
            * (primitive(x) - primitive((1. - self.optimal_stretch) / self.half_width));
        if !potential.is_finite() || !tension.is_finite() {
            return Err("active fiber response overflow");
        }
        Ok((tension, potential))
    }
}

/// Tension-only aligned exponential reinforcement, plus constant nominal active
/// tension. Direction is in the reference configuration; no dispersion/Hill law.
#[derive(Clone, Copy, Debug)]
pub struct Fiber {
    pub direction: Vec3,
    pub stiffness_pa: f64,
    pub exponent: f64,
    pub active_pa: f64,
}

#[derive(Clone, Debug)]
pub struct Material {
    pub shear_pa: f64,
    pub bulk_pa: f64,
    pub fibers: Vec<Fiber>,
}

#[derive(Clone, Copy, Debug)]
pub struct Response {
    pub energy_density: f64,
    pub first_piola: Matrix,
    pub volume_ratio: f64,
}

impl Material {
    /// Isotropic hyperelastic matrix with the requested small-strain E and nu.
    /// # Errors
    /// Requires finite E > 0, -1 < nu < 0.5 and representable moduli.
    pub fn from_young_poisson(young_pa: f64, poisson: f64) -> Result<Self, &'static str> {
        if !young_pa.is_finite()
            || young_pa <= 0.
            || !poisson.is_finite()
            || poisson <= -1.
            || poisson >= 0.5
        {
            return Err("invalid elastic constants");
        }
        let material = Self {
            shear_pa: young_pa / (2. * (1. + poisson)),
            bulk_pa: young_pa / (3. * (1. - 2. * poisson)),
            fibers: Vec::new(),
        };
        material.response(IDENTITY, 0.)?;
        Ok(material)
    }

    /// Isochoric neo-Hookean matrix + volumetric penalty + exponential fibers.
    /// # Errors
    /// Rejects invalid parameters, activation, inversion and numerical overflow.
    pub fn response(&self, f: Matrix, activation: f64) -> Result<Response, &'static str> {
        self.response_with_active_length(f, activation, None)
    }

    /// Aligned-fiber response with an optional explicit active force/stretch curve.
    /// # Errors
    /// Invalid material, length law, deformation or numerical overflow.
    pub fn response_with_active_length(
        &self,
        f: Matrix,
        activation: f64,
        length_law: Option<ActiveFiberLengthLaw>,
    ) -> Result<Response, &'static str> {
        if let Some(law) = length_law {
            law.validate()?;
        }

        if !activation.is_finite()
            || !(0.0..=1.0).contains(&activation)
            || !self.shear_pa.is_finite()
            || self.shear_pa <= 0.
            || !self.bulk_pa.is_finite()
            || self.bulk_pa <= 0.
            || f.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid material input");
        }
        let j = det(f);
        if j <= 0. || !j.is_finite() {
            return Err("inverted element");
        }
        let inv_t = transpose(inverse(f)?);
        let i1: f64 = f.iter().flatten().map(|x| x * x).sum();
        let q = j.powf(-2. / 3.);
        let excess = isochoric_excess(f, q, q * i1);
        let mut energy = self.shear_pa / 2. * excess + self.bulk_pa / 2. * (j - 1.).powi(2);
        let mut p: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                self.shear_pa * q * (f[i][k] - i1 / 3. * inv_t[i][k])
                    + self.bulk_pa * (j - 1.) * j * inv_t[i][k]
            })
        });
        for fiber in &self.fibers {
            if fiber.direction.iter().any(|x| !x.is_finite())
                || (dot(fiber.direction, fiber.direction) - 1.).abs() > 1e-8
                || !fiber.stiffness_pa.is_finite()
                || fiber.stiffness_pa < 0.
                || !fiber.exponent.is_finite()
                || fiber.exponent <= 0.
                || !fiber.active_pa.is_finite()
                || fiber.active_pa < 0.
            {
                return Err("invalid fiber");
            }
            let a = mv(f, fiber.direction);
            let l2 = dot(a, a);
            let l = l2.sqrt();
            if l < 1e-12 {
                return Err("collapsed fiber");
            }
            let e = (l2 - 1.).max(0.);
            let exp = (fiber.exponent * e * e).exp();
            let (active_factor, active_potential) = match length_law {
                Some(law) => law.response(l)?,
                None => (1., l - 1.),
            };
            energy += fiber.stiffness_pa / (2. * fiber.exponent)
                * (fiber.exponent * e * e).exp_m1()
                + activation * fiber.active_pa * active_potential;
            let factor = 2. * fiber.stiffness_pa * e * exp
                + activation * fiber.active_pa * active_factor / l;
            let pa = outer(a, fiber.direction);
            for i in 0..3 {
                for k in 0..3 {
                    p[i][k] += factor * pa[i][k];
                }
            }
        }
        if !energy.is_finite() || p.iter().flatten().any(|x| !x.is_finite()) {
            return Err("material overflow");
        }
        Ok(Response {
            energy_density: energy,
            first_piola: p,
            volume_ratio: j,
        })
    }

    /// Spatial stress sigma = P F^T / det(F), including active fiber stress.
    /// # Errors
    /// Rejects invalid materials/deformations and nonfinite stress diagnostics.
    pub fn stress(&self, deformation: Matrix, activation: f64) -> Result<Stress, &'static str> {
        let response = self.response(deformation, activation)?;
        let mut spatial = mm(response.first_piola, transpose(deformation))
            .map(|row| row.map(|v| v / response.volume_ratio));
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            let mean = spatial[i][j].midpoint(spatial[j][i]);
            spatial[i][j] = mean;
            spatial[j][i] = mean;
        }
        Stress::from_cauchy(spatial)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Stress {
    pub cauchy_pa: Matrix,
    /// Descending principal stresses; positive means tension.
    pub principal_pa: [f64; 3],
    /// Orthonormal spatial eigenvectors in columns, matching `principal_pa`.
    /// Each sign is arbitrary; repeated eigenvalues define a subspace rather
    /// than a unique direction. Do not infer a unique crack normal in that case.
    pub principal_directions: Matrix,
    /// Positive in compression.
    pub pressure_pa: f64,
    pub von_mises_pa: f64,
    pub max_shear_pa: f64,
}

impl Stress {
    /// Analyse a symmetric spatial stress tensor, in pascals.
    /// # Errors
    /// Rejects nonfinite, asymmetric or overflowing stresses.
    pub fn from_cauchy(cauchy_pa: Matrix) -> Result<Self, &'static str> {
        if cauchy_pa.iter().flatten().any(|v| !v.is_finite()) {
            return Err("nonfinite Cauchy stress");
        }
        let magnitude = cauchy_pa
            .iter()
            .flatten()
            .fold(0_f64, |a, b| a.max(b.abs()));
        let scale_val = if magnitude == 0. { 1. } else { magnitude };
        let mut a = cauchy_pa.map(|row| row.map(|v| v / scale_val));
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            if (a[i][j] - a[j][i]).abs() > 1e-10 {
                return Err("asymmetric Cauchy stress");
            }
            let mean = a[i][j].midpoint(a[j][i]);
            a[i][j] = mean;
            a[j][i] = mean;
        }
        let mean = (a[0][0] + a[1][1] + a[2][2]) / 3.;
        let mut dev = a;
        for (i, row) in dev.iter_mut().enumerate() {
            row[i] -= mean;
        }
        let norm2: f64 = dev.iter().flatten().map(|v| v * v).sum();
        let (principal, directions) = crate::symmetric_eigen::normalized(a)?;
        let result = Self {
            cauchy_pa: a.map(|row| row.map(|v| v * scale_val)),
            principal_pa: principal.map(|v| v * scale_val),
            principal_directions: directions,
            pressure_pa: -mean * scale_val,
            von_mises_pa: (1.5 * norm2).sqrt() * scale_val,
            max_shear_pa: (principal[0] - principal[2]) / 2. * scale_val,
        };
        if result.principal_pa.iter().any(|v| !v.is_finite())
            || !result.pressure_pa.is_finite()
            || !result.von_mises_pa.is_finite()
            || !result.max_shear_pa.is_finite()
        {
            return Err("stress diagnostic overflow");
        }
        Ok(result)
    }

    /// Demand / yield stress for isotropic ductile materials. >= 1 denotes yield onset.
    /// Diagnoses onset only; does not introduce plastic strain or fracture.
    /// # Errors
    /// Rejects nonpositive/nonfinite yield strength and overflowing ratios.
    pub fn yield_utilization(&self, yield_pa: f64) -> Result<f64, &'static str> {
        let ratio = self.von_mises_pa / yield_pa;
        if !yield_pa.is_finite() || yield_pa <= 0. || !ratio.is_finite() || ratio < 0. {
            return Err("invalid yield strength or stress");
        }
        Ok(ratio)
    }

    /// Maximum-normal-stress (Rankine) onset estimate with independent tensile
    /// and compressive strengths. Positive stress denotes tension. A ratio >= 1
    /// denotes reaching a supplied strength, including hydrostatic loading.
    /// # Errors
    /// Rejects invalid strengths, unordered/nonfinite principal stresses or overflow.
    pub fn normal_strength_utilization(
        &self,
        tensile_pa: f64,
        compressive_pa: f64,
    ) -> Result<f64, &'static str> {
        if !tensile_pa.is_finite()
            || tensile_pa <= 0.
            || !compressive_pa.is_finite()
            || compressive_pa <= 0.
            || self.principal_pa.iter().any(|v| !v.is_finite())
            || self.principal_pa[0] < self.principal_pa[1]
            || self.principal_pa[1] < self.principal_pa[2]
        {
            return Err("invalid normal strengths or principal stresses");
        }
        let ratio = (self.principal_pa[0].max(0.) / tensile_pa)
            .max((-self.principal_pa[2]).max(0.) / compressive_pa);
        if !ratio.is_finite() {
            return Err("normal strength utilization overflow");
        }
        Ok(ratio)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ElementStress {
    pub nodes: [usize; 4],
    pub reference_volume_m3: f64,
    pub volume_ratio: f64,
    pub stress: Stress,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f_bar_replaces_element_determinant_with_patch_average() {
        let f = [[1.2, 0.0, 0.0], [0.0, 0.9, 0.0], [0.0, 0.0, 1.1]];
        let modified = f_bar_modified_gradient(f, 1.0).unwrap();
        assert!((det(modified) - 1.0).abs() < 1e-12);
        assert!(f_bar_modified_gradient(IDENTITY, -0.5).is_err());
    }
}
