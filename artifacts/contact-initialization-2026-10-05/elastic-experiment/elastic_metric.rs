//! Positive spatial elastic search metric, not a replacement constitutive law.
use super::{Body, Vec3, dot};
use crate::biomechanics::{columns, det, inverse, mm, mv, sub, transpose};

pub(super) struct ElasticStencil {
    pub nodes: [usize; 4],
    gradients: [Vec3; 4],
    shear_volume: f64,
    bulk_volume: f64,
}
impl ElasticStencil {
    pub fn diagonal(&self, corner: usize) -> Vec3 {
        let g = self.gradients[corner];
        std::array::from_fn(|axis| {
            self.shear_volume * dot(g, g)
                + (self.bulk_volume + self.shear_volume / 3.) * g[axis].powi(2)
        })
    }
    pub fn apply(&self, displacement: [Vec3; 4]) -> [Vec3; 4] {
        let gradient: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                (1..4)
                    .map(|k| (displacement[k][i] - displacement[0][i]) * self.gradients[k][j])
                    .sum()
            })
        });
        let trace = gradient[0][0] + gradient[1][1] + gradient[2][2];
        let stress = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                if i == j {
                    let deviator = ((gradient[i][i] - gradient[(i + 1) % 3][(i + 1) % 3])
                        + (gradient[i][i] - gradient[(i + 2) % 3][(i + 2) % 3]))
                        / 3.;
                    self.shear_volume * (2. * deviator) + self.bulk_volume * trace
                } else {
                    self.shear_volume * (gradient[i][j] + gradient[j][i])
                }
            })
        });
        let mut action = self.gradients.map(|g| mv(stress, g));
        action[0] =
            std::array::from_fn(|axis| -(action[1][axis] + action[2][axis] + action[3][axis]));
        action
    }
}
pub(super) fn elastic_stencils(
    body: &Body,
    points: &[Vec3],
) -> Result<Vec<ElasticStencil>, &'static str> {
    if points.len() != body.positions.len() {
        return Err("elastic metric vertex count mismatch");
    }
    body.elements
        .iter()
        .map(|element| {
            let [a, b, c, d] = element.nodes.map(|i| points[i]);
            let deformation = mm(columns(sub(b, a), sub(c, a), sub(d, a)), element.inv_rest);
            let jacobian = det(deformation);
            if !jacobian.is_finite() || jacobian <= 0. {
                return Err("invalid elastic metric deformation");
            }
            let inverse_transpose = transpose(inverse(deformation)?);
            let mut gradients = element.gradients.map(|g| mv(inverse_transpose, g));
            gradients[0] = std::array::from_fn(|axis| {
                -(gradients[1][axis] + gradients[2][axis] + gradients[3][axis])
            });
            let volume = element.volume * jacobian;
            let stencil = ElasticStencil {
                nodes: element.nodes,
                gradients,
                shear_volume: element.material.shear_pa * volume,
                bulk_volume: element.material.bulk_pa * volume,
            };
            if !volume.is_finite()
                || volume <= 0.
                || !stencil.shear_volume.is_finite()
                || stencil.shear_volume <= 0.
                || !stencil.bulk_volume.is_finite()
                || stencil.bulk_volume <= 0.
                || gradients.iter().flatten().any(|v| !v.is_finite())
                || (0..4)
                    .flat_map(|i| stencil.diagonal(i))
                    .any(|v| !v.is_finite() || v < 0.)
            {
                return Err("elastic metric overflow");
            }
            Ok(stencil)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::Material;
    fn fixture() -> Body {
        Body::new(
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            vec![false; 4],
            vec![(
                [0, 1, 2, 3],
                Material {
                    shear_pa: 5000.,
                    bulk_pa: 1e6,
                    fibers: vec![],
                },
            )],
        )
        .unwrap()
    }
    #[test]
    fn elastic_action_matches_independent_constitutive_tangent() {
        let body = fixture();
        let metric = elastic_stencils(&body, body.positions()).unwrap();
        let u = [
            [0.01, 0.02, -0.03],
            [-0.04, 0.03, 0.02],
            [0.02, -0.01, 0.04],
            [-0.03, 0.04, -0.02],
        ];
        let gradient = columns(sub(u[1], u[0]), sub(u[2], u[0]), sub(u[3], u[0]));
        let h = 1e-6;
        let f = |sign: f64| {
            std::array::from_fn(|i| {
                std::array::from_fn(|j| f64::from(i == j) + sign * h * gradient[i][j])
            })
        };
        let plus = body.elements[0]
            .material
            .response(f(1.), 0.)
            .unwrap()
            .first_piola;
        let minus = body.elements[0]
            .material
            .response(f(-1.), 0.)
            .unwrap()
            .first_piola;
        let tangent =
            std::array::from_fn(|i| std::array::from_fn(|j| (plus[i][j] - minus[i][j]) / (2. * h)));
        let expected = body.elements[0]
            .gradients
            .map(|g| mv(tangent, g).map(|v| v / 6.));
        let actual = metric[0].apply(u);
        for node in 0..4 {
            for axis in 0..3 {
                assert!((actual[node][axis] - expected[node][axis]).abs() < 1e-4);
            }
        }
        let work: f64 = u.iter().zip(actual).map(|(u, a)| dot(*u, a)).sum();
        assert!(work > 0.);
    }
    #[test]
    fn spatial_metric_is_rotation_covariant_and_rigid_modes_have_zero_action() {
        let body = fixture();
        let rotate = |p: Vec3| [-p[1], p[0], p[2]];
        let points: Vec<_> = body.positions().iter().map(|&p| rotate(p)).collect();
        let original = elastic_stencils(&body, body.positions()).unwrap();
        let rotated = elastic_stencils(&body, &points).unwrap();
        let u = [
            [0.01, 0.02, -0.03],
            [-0.04, 0.03, 0.02],
            [0.02, -0.01, 0.04],
            [-0.03, 0.04, -0.02],
        ];
        let a = original[0].apply(u).map(rotate);
        let b = rotated[0].apply(u.map(rotate));
        for node in 0..4 {
            for axis in 0..3 {
                assert!((a[node][axis] - b[node][axis]).abs() < 1e-9);
            }
        }
        assert_eq!(original[0].apply([[0.25, -0.5, 1.]; 4]), [[0.; 3]; 4]);
        let spin: [Vec3; 4] = std::array::from_fn(|i| rotate(body.positions()[i]));
        // Infinitesimal spin about Z has no symmetric strain.
        let spin = spin.map(|p| [p[0], p[1], 0.]);
        assert_eq!(original[0].apply(spin), [[0.; 3]; 4]);
    }
    #[test]
    fn hydrostatic_action_retains_small_bulk_modulus_under_large_shear_contrast() {
        let mut body = fixture();
        body.elements[0].material.shear_pa = 1e12;
        body.elements[0].material.bulk_pa = 1e-9;
        let metric = elastic_stencils(&body, body.positions()).unwrap();
        let u: [Vec3; 4] = std::array::from_fn(|i| body.positions()[i].map(|v| v * 0.01));
        let action = metric[0].apply(u);
        let pressure_volume = (1e-9 / 6.) * 0.03;
        for node in 0..4 {
            for axis in 0..3 {
                let expected = pressure_volume * body.elements[0].gradients[node][axis];
                assert!((action[node][axis] - expected).abs() < 1e-23);
            }
        }
        assert!(u.iter().zip(action).map(|(u, a)| dot(*u, a)).sum::<f64>() > 0.);
    }
}
