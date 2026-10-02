//! Spatial stress diagnostics. Positive normal stress denotes tension.
use super::{Body, Material, Matrix, columns, det, mm, sub, transpose};

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
        let scale = if magnitude == 0. { 1. } else { magnitude };
        let mut a = cauchy_pa.map(|row| row.map(|v| v / scale));
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
        // Cyclic Jacobi avoids acos cancellation at repeated eigenvalues.
        let mut eigen = a;
        let mut directions = super::IDENTITY;
        for _ in 0..16 {
            for (i, j) in [(0, 1), (0, 2), (1, 2)] {
                if eigen[i][j].abs() <= 1e-16 {
                    continue;
                }
                let angle = 0.5 * (2. * eigen[i][j]).atan2(eigen[j][j] - eigen[i][i]);
                let (sin, cos) = angle.sin_cos();
                let (ii, jj, ij) = (eigen[i][i], eigen[j][j], eigen[i][j]);
                eigen[i][i] = cos * cos * ii - 2. * sin * cos * ij + sin * sin * jj;
                eigen[j][j] = sin * sin * ii + 2. * sin * cos * ij + cos * cos * jj;
                eigen[i][j] = 0.;
                eigen[j][i] = 0.;
                let k = 3 - i - j;
                let (ik, jk) = (eigen[i][k], eigen[j][k]);
                eigen[i][k] = cos * ik - sin * jk;
                eigen[k][i] = eigen[i][k];
                eigen[j][k] = sin * ik + cos * jk;
                eigen[k][j] = eigen[j][k];
                for row in &mut directions {
                    let (vi, vj) = (row[i], row[j]);
                    row[i] = cos * vi - sin * vj;
                    row[j] = sin * vi + cos * vj;
                }
            }
        }
        let mut order = [0, 1, 2];
        order.sort_by(|&i, &j| eigen[j][j].total_cmp(&eigen[i][i]));
        let principal = order.map(|i| eigen[i][i]);
        let result = Self {
            cauchy_pa: a.map(|row| row.map(|v| v * scale)),
            principal_pa: principal.map(|v| v * scale),
            principal_directions: directions.map(|row| order.map(|i| row[i])),
            pressure_pa: -mean * scale,
            von_mises_pa: (1.5 * norm2).sqrt() * scale,
            max_shear_pa: (principal[0] - principal[2]) / 2. * scale,
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
    /// This is a diagnostic envelope, not a damage law or a universal brittle
    /// material model; calibrated multiaxial envelopes may be necessary.
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
        material.response(super::IDENTITY, 0.)?;
        Ok(material)
    }
    /// Spatial stress sigma = P F^T / det(F), including active fiber stress.
    /// # Errors
    /// Rejects invalid materials/deformations and nonfinite stress diagnostics.
    pub fn stress(&self, deformation: Matrix, activation: f64) -> Result<Stress, &'static str> {
        let response = self.response(deformation, activation)?;
        let mut spatial = mm(response.first_piola, transpose(deformation))
            .map(|row| row.map(|v| v / response.volume_ratio));
        // The constitutive law has symmetric Cauchy stress analytically. Remove
        // multiplication roundoff, including at a stress-free rigid rotation.
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            let mean = spatial[i][j].midpoint(spatial[j][i]);
            spatial[i][j] = mean;
            spatial[j][i] = mean;
        }
        Stress::from_cauchy(spatial)
    }
}
impl Body {
    /// One spatial stress per constant-strain tetrahedron, without nodal smoothing.
    /// # Errors
    /// Rejects invalid positions, inverted elements and stress overflow.
    pub fn stresses_at(&self, positions: &[[f64; 3]]) -> Result<Vec<ElementStress>, &'static str> {
        if positions.len() != self.positions.len()
            || positions.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid stress evaluation positions");
        }
        let pore_pressure = self.pore_fields_at(positions)?.0;
        self.elements
            .iter()
            .enumerate()
            .map(|(index, e)| {
                let [a, b, c, d] = e.nodes.map(|i| positions[i]);
                let f = mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest);
                Ok(ElementStress {
                    nodes: e.nodes,
                    reference_volume_m3: e.volume,
                    volume_ratio: det(f),
                    stress: {
                        let r = e.response(f)?;
                        let mut spatial = mm(r.first_piola, transpose(f))
                            .map(|row| row.map(|v| v / r.volume_ratio));
                        for (i, row) in spatial.iter_mut().enumerate() {
                            row[i] -= pore_pressure[index];
                        }
                        Stress::from_cauchy(spatial)?
                    },
                })
            })
            .collect()
    }
}
