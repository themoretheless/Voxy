//! Conventional aligned, tension-only isochoric HGO reinforcement.
//! No fiber dispersion, activation or calibrated human tissue defaults.
use super::{Fiber, Material, Matrix, Response, det, dot, inverse, mv, outer, transpose};

#[derive(Clone, Debug)]
pub struct HgoMaterial {
    pub shear_pa: f64,
    pub bulk_pa: f64,
    /// Passive coefficients: k1=stiffness_pa, k2=exponent; active_pa must be zero.
    pub fibers: Vec<Fiber>,
}
impl HgoMaterial {
    /// Energy and its derivative 2*dPsi/dCbar before the isochoric projection.
    /// Cbar must be a symmetric positive-definite unit-determinant metric.
    /// This is not the physical second Piola stress before projection to C.
    pub fn isochoric_metric_response(&self, c: Matrix) -> Result<(f64, Matrix), &'static str> {
        // Validate coefficients using the same constitutive path.
        self.response(super::IDENTITY)?;
        if c.iter().flatten().any(|x| !x.is_finite())
            || !(0..3).all(|i| (0..3).all(|j| c[i][j] == c[j][i]))
            || c[0][0] <= 0.
            || c[0][0] * c[1][1] - c[0][1] * c[1][0] <= 0.
            || !det(c).is_finite()
            || (det(c) - 1.).abs() > 1e-8
        {
            return Err("invalid isochoric HGO metric");
        }
        let mut energy = 0.5 * self.shear_pa * (c[0][0] + c[1][1] + c[2][2] - 3.);
        let mut stress = super::IDENTITY.map(|row| row.map(|x| self.shear_pa * x));
        for fiber in &self.fibers {
            let invariant = dot(fiber.direction, mv(c, fiber.direction));
            let strain = (invariant - 1.).max(0.);
            let argument = fiber.exponent * strain * strain;
            energy += fiber.stiffness_pa / (2. * fiber.exponent) * argument.exp_m1();
            let tensor = outer(fiber.direction, fiber.direction);
            let coefficient = 2. * fiber.stiffness_pa * strain * argument.exp();
            for i in 0..3 {
                for j in 0..3 {
                    stress[i][j] += coefficient * tensor[i][j];
                }
            }
        }
        if !energy.is_finite() || stress.iter().flatten().any(|x| !x.is_finite()) {
            return Err("isochoric HGO metric response overflow");
        }
        Ok((energy, stress))
    }
    /// Uses k1/(2*k2)*(exp(k2*max(I4bar-1,0)^2)-1).
    /// This explicit convention is not claimed to reproduce an ambiguous paper formula.
    pub fn response(&self, f: Matrix) -> Result<Response, &'static str> {
        let base = Material {
            shear_pa: self.shear_pa,
            bulk_pa: self.bulk_pa,
            fibers: vec![],
        };
        let mut response = base.response(f, 0.)?;
        let j = det(f);
        let q = j.powf(-2. / 3.);
        let inv_t = transpose(inverse(f)?);
        for fiber in &self.fibers {
            if !fiber.stiffness_pa.is_finite()
                || fiber.stiffness_pa < 0.
                || !fiber.exponent.is_finite()
                || fiber.exponent <= 0.
                || fiber.active_pa != 0.
                || fiber.direction.iter().any(|x| !x.is_finite())
                || (dot(fiber.direction, fiber.direction) - 1.).abs() > 1e-8
            {
                return Err("invalid passive HGO fiber");
            }
            let a = mv(f, fiber.direction);
            let invariant = q * dot(a, a);
            let strain = (invariant - 1.).max(0.);
            let argument = fiber.exponent * strain * strain;
            response.energy_density +=
                fiber.stiffness_pa / (2. * fiber.exponent) * argument.exp_m1();
            let coefficient = fiber.stiffness_pa * strain * argument.exp();
            let product = outer(a, fiber.direction);
            for i in 0..3 {
                for k in 0..3 {
                    response.first_piola[i][k] +=
                        coefficient * (2. * q * product[i][k] - 2. / 3. * invariant * inv_t[i][k]);
                }
            }
        }
        if !response.energy_density.is_finite()
            || response
                .first_piola
                .iter()
                .flatten()
                .any(|x| !x.is_finite())
        {
            return Err("HGO response overflow");
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::IDENTITY;
    fn material() -> HgoMaterial {
        HgoMaterial {
            shear_pa: 3000.,
            bulk_pa: 50000.,
            fibers: vec![Fiber {
                direction: [1., 0., 0.],
                stiffness_pa: 9000.,
                exponent: 0.2,
                active_pa: 0.,
            }],
        }
    }
    #[test]
    fn metric_stress_projects_to_deformation_response() {
        let m = material();
        let f = [[1.2, 0.12, 0.03], [0.04, 0.9, 0.07], [0., 0.02, 1.05]];
        let j = det(f);
        let q = j.powf(-2. / 3.);
        let c = super::super::mm(transpose(f), f);
        let cbar = c.map(|row| row.map(|x| q * x));
        let (energy, stress) = m.isochoric_metric_response(cbar).unwrap();
        let r = m.response(f).unwrap();
        assert!((energy + 0.5 * m.bulk_pa * (j - 1.).powi(2) - r.energy_density).abs() < 1e-9);
        let fs = super::super::mm(f, stress);
        let inv_t = transpose(inverse(f).unwrap());
        let contraction = (0..3)
            .flat_map(|i| (0..3).map(move |k| cbar[i][k] * stress[i][k]))
            .sum::<f64>();
        for i in 0..3 {
            for k in 0..3 {
                let p = q * fs[i][k] - contraction / 3. * inv_t[i][k]
                    + m.bulk_pa * (j - 1.) * j * inv_t[i][k];
                assert!((p - r.first_piola[i][k]).abs() < 1e-8);
            }
        }
        assert!(
            m.isochoric_metric_response([[2., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
                .is_err()
        );
    }
    #[test]
    fn rigid_rotation_covariance_and_compression_slackness() {
        let m = material();
        let f = [[1.2, 0.12, 0.03], [0.04, 0.9, 0.07], [0., 0.02, 1.05]];
        let rotation = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
        let rotated_f = super::super::mm(rotation, f);
        let original = m.response(f).unwrap();
        let rotated = m.response(rotated_f).unwrap();
        assert!((original.energy_density - rotated.energy_density).abs() < 1e-9);
        let expected = super::super::mm(rotation, original.first_piola);
        for i in 0..3 {
            for j in 0..3 {
                assert!((rotated.first_piola[i][j] - expected[i][j]).abs() < 1e-8);
            }
        }
        let base = HgoMaterial {
            fibers: vec![],
            ..m.clone()
        };
        let compression = [[0.8, 0., 0.], [0., 1.1, 0.], [0., 0., 1.1]];
        let reinforced = m.response(compression).unwrap();
        let unreinforced = base.response(compression).unwrap();
        assert_eq!(reinforced.energy_density, unreinforced.energy_density);
        assert_eq!(reinforced.first_piola, unreinforced.first_piola);
    }
    #[test]
    fn invalid_inputs_and_fiber_overflow_are_rejected() {
        let mut m = material();
        m.fibers[0].active_pa = 1.;
        assert!(m.response(IDENTITY).is_err());
        m.fibers[0].active_pa = 0.;
        m.fibers[0].direction = [2., 0., 0.];
        assert!(m.response(IDENTITY).is_err());
        let m = material();
        assert!(m.response([[0.; 3]; 3]).is_err());
        assert!(
            m.response([[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
                .is_err()
        );
        assert!(
            m.response([[100., 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]])
                .is_err()
        );
    }
    #[test]
    fn energy_derivative_and_volume_independent_reinforcement() {
        let m = material();
        let f = [[1.2, 0.12, 0.03], [0.04, 0.9, 0.07], [0., 0.02, 1.05]];
        let r = m.response(f).unwrap();
        let h = 1e-6;
        for i in 0..3 {
            for j in 0..3 {
                let mut plus = f;
                let mut minus = f;
                plus[i][j] += h;
                minus[i][j] -= h;
                let fd = (m.response(plus).unwrap().energy_density
                    - m.response(minus).unwrap().energy_density)
                    / (2. * h);
                assert!((fd - r.first_piola[i][j]).abs() < 1e-4);
            }
        }
        let base = HgoMaterial {
            fibers: vec![],
            ..m.clone()
        };
        let scaled = f.map(|row| row.map(|x| 1.1 * x));
        let reinforcement = r.energy_density - base.response(f).unwrap().energy_density;
        let scaled_reinforcement = m.response(scaled).unwrap().energy_density
            - base.response(scaled).unwrap().energy_density;
        assert!((reinforcement - scaled_reinforcement).abs() < 1e-9);
        let rest = m.response(IDENTITY).unwrap();
        assert_eq!(rest.energy_density, 0.);
        assert!(rest.first_piola.iter().flatten().all(|x| x.abs() < 1e-10));
    }
}
