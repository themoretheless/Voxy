//! Orthotropic passive myocardium: fiber, sheet and fiber-sheet coupling.
//! Holzapfel–Ogden-type isochoric invariants with a volumetric penalty.
//! Parameters require experimental calibration; no universal human defaults.
use super::{Matrix, Response, Vec3, det, dot, inverse, mm, mv, outer, transpose};

/// Exponential energy coefficient in Pa and dimensionless exponent.
#[derive(Clone, Copy, Debug)]
pub struct ExponentialTerm {
    pub scale_pa: f64,
    pub exponent: f64,
}
/// Passive orthotropic law plus an optional constant nominal active fiber tension.
/// Active tension is a mechanical input, not an electrophysiological model.
#[derive(Clone, Copy, Debug)]
pub struct Myocardium {
    pub matrix: ExponentialTerm,
    pub fiber: ExponentialTerm,
    pub sheet: ExponentialTerm,
    pub fiber_sheet: ExponentialTerm,
    pub bulk_pa: f64,
    pub fiber_direction: Vec3,
    pub sheet_direction: Vec3,
    pub active_tension_pa: f64,
}
impl Myocardium {
    /// Evaluate energy per reference volume and its exact first Piola derivative.
    /// # Errors
    /// Rejects inversion, non-finite inputs, invalid material parameters and
    /// directions that do not form an orthonormal reference frame.
    pub fn response(&self, f: Matrix, activation: f64) -> Result<Response, &'static str> {
        let terms = [self.matrix, self.fiber, self.sheet, self.fiber_sheet];
        if terms.iter().any(|t| {
            !t.scale_pa.is_finite()
                || t.scale_pa < 0.
                || !t.exponent.is_finite()
                || t.exponent <= 0.
        }) || !self.bulk_pa.is_finite()
            || self.bulk_pa <= 0.
            || !self.active_tension_pa.is_finite()
            || self.active_tension_pa < 0.
            || !activation.is_finite()
            || !(0.0..=1.0).contains(&activation)
            || f.iter().flatten().any(|v| !v.is_finite())
            || self
                .fiber_direction
                .iter()
                .chain(&self.sheet_direction)
                .any(|v| !v.is_finite())
            || (dot(self.fiber_direction, self.fiber_direction) - 1.).abs() > 1e-8
            || (dot(self.sheet_direction, self.sheet_direction) - 1.).abs() > 1e-8
            || dot(self.fiber_direction, self.sheet_direction).abs() > 1e-8
        {
            return Err("invalid orthotropic myocardium parameters");
        }
        let j = det(f);
        if !j.is_finite() || j <= 0. {
            return Err("inverted myocardium element");
        }
        let inverse_t = transpose(inverse(f)?);
        let q = j.powf(-2. / 3.);
        let ff = mv(f, self.fiber_direction);
        let fs = mv(f, self.sheet_direction);
        let i1 = q * f.iter().flatten().map(|v| v * v).sum::<f64>();
        let i4f = q * dot(ff, ff);
        let i4s = q * dot(fs, fs);
        let i8 = q * dot(ff, fs);
        // det(Cbar)=1: tr(Cbar-I)=-I2(Cbar-I)-det(Cbar-I).
        // Recover quadratic strain energy without subtracting O(1) invariants.
        let mut deviation = mm(transpose(f), f).map(|r| r.map(|v| q * v));
        for (i, row) in deviation.iter_mut().enumerate() {
            row[i] -= 1.;
        }
        let excess = if deviation.iter().flatten().all(|v| v.abs() < 0.1) {
            let second = deviation[0][0] * deviation[1][1]
                + deviation[0][0] * deviation[2][2]
                + deviation[1][1] * deviation[2][2]
                - deviation[0][1] * deviation[1][0]
                - deviation[0][2] * deviation[2][0]
                - deviation[1][2] * deviation[2][1];
            (-second - det(deviation)).max(0.)
        } else {
            i1 - 3.
        };
        let matrix_argument = self.matrix.exponent * excess;
        let matrix_exp = matrix_argument.exp();
        let mut energy = self.matrix.scale_pa / (2. * self.matrix.exponent)
            * matrix_argument.exp_m1()
            + 0.5 * self.bulk_pa * (j - 1.).powi(2);
        let mut p: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                self.matrix.scale_pa * matrix_exp * (q * f[i][k] - i1 / 3. * inverse_t[i][k])
                    + self.bulk_pa * (j - 1.) * j * inverse_t[i][k]
            })
        });
        for (invariant, deformed, reference, term) in [
            (i4f, ff, self.fiber_direction, self.fiber),
            (i4s, fs, self.sheet_direction, self.sheet),
        ] {
            let strain = (invariant - 1.).max(0.);
            let exponential = (term.exponent * strain * strain).exp();
            energy +=
                term.scale_pa / (2. * term.exponent) * (term.exponent * strain * strain).exp_m1();
            let product = outer(deformed, reference);
            let coefficient = term.scale_pa * strain * exponential;
            for i in 0..3 {
                for k in 0..3 {
                    p[i][k] += coefficient
                        * (2. * q * product[i][k] - 2. / 3. * invariant * inverse_t[i][k]);
                }
            }
        }
        let coupling_exp = (self.fiber_sheet.exponent * i8 * i8).exp();
        energy += self.fiber_sheet.scale_pa / (2. * self.fiber_sheet.exponent)
            * (self.fiber_sheet.exponent * i8 * i8).exp_m1();
        let a = outer(fs, self.fiber_direction);
        let b = outer(ff, self.sheet_direction);
        let coupling = self.fiber_sheet.scale_pa * i8 * coupling_exp;
        let stretch = dot(ff, ff).sqrt();
        let active = self.active_tension_pa * activation;
        energy += active * (stretch - 1.);
        let active_product = outer(ff, self.fiber_direction);
        for i in 0..3 {
            for k in 0..3 {
                p[i][k] += coupling * (q * (a[i][k] + b[i][k]) - 2. / 3. * i8 * inverse_t[i][k])
                    + active * active_product[i][k] / stretch;
            }
        }
        if !energy.is_finite() || p.iter().flatten().any(|v| !v.is_finite()) {
            return Err("myocardium constitutive overflow");
        }
        Ok(Response {
            energy_density: energy,
            first_piola: p,
            volume_ratio: j,
        })
    }
}
