//! Actual implicit Cosserat energy in joules, not the quadratic model.
use super::*;
// Conservative squared-residual evaluation noise, scaled by actual SI stiffness.
// Coordinate subtraction/rotation roundoff must not be judged relative to an
// almost-zero energy alone. This does not change force or contact admission.
pub(super) fn noise_floor(rod: &HairRod, dt: f64) -> Result<f64, &'static str> {
    let eps = 32. * f64::EPSILON;
    let ea = rod.material.young_modulus * rod.material.area();
    let mut floor = 0.;
    for i in 1..rod.x.len() {
        let size = rod.x[i]
            .iter()
            .chain(&rod.predicted_x[i])
            .map(|x| x.abs())
            .fold(0., f64::max);
        floor += 3. * (eps * size).powi(2) / rod.inv_mass[i] / (dt * dt);
    }
    for i in 0..rod.lengths.len() {
        let size = rod.x[i]
            .iter()
            .chain(&rod.x[i + 1])
            .map(|x| x.abs())
            .fold(rod.lengths[i], f64::max);
        floor += 3. * ea / rod.lengths[i] * (eps * (2. * size + rod.lengths[i])).powi(2);
    }
    for i in 1..rod.q.len() {
        floor += 3. * eps * eps / rod.inv_inertia[i] / (dt * dt);
    }
    for i in 0..rod.rest_relative.len() {
        floor += 12.
            * eps
            * eps
            * (2. * rod.material.bending_rigidity() + rod.material.twisting_rigidity())
            / ((rod.lengths[i] + rod.lengths[i + 1]) * 0.5);
    }
    for contact in &rod.contacts {
        let i = contact.segment;
        let t = contact.fraction;
        let size = rod.x[i]
            .iter()
            .chain(&rod.x[i + 1])
            .chain(&contact.target)
            .map(|x| x.abs())
            .fold(0., f64::max);
        floor += ea
            * 100.
            * ((1. - t) / rod.lengths[i.saturating_sub(1)] + t / rod.lengths[i])
            * (eps * 3. * size * contact.metric_scale).powi(2);
    }
    if !floor.is_finite() {
        return Err("hair energy roundoff bound overflow");
    }
    Ok(floor)
}
// Preserve the original inertia evaluation order. Forming its reciprocal
// coefficient first can overflow although the weighted small residual is finite.
#[derive(Clone, Copy)]
enum ResidualWeight {
    Inertia { inverse: f64, dt_squared: f64 },
    Elastic(f64),
}
impl ResidualWeight {
    fn apply(self, value: f64) -> f64 {
        match self {
            Self::Inertia {
                inverse,
                dt_squared,
            } => 0.5 * value / inverse / dt_squared,
            Self::Elastic(coefficient) => coefficient * value,
        }
    }
}
// One residual owner for the potential and its signed change. N=1 evaluates
// energy; N=2 streams paired poses without allocating per-rod term buffers.
fn residuals<const N: usize>(
    rod: &HairRod,
    poses: [(&[V], &[Q]); N],
    dt: f64,
    mut visit: impl FnMut(ResidualWeight, [V; N]),
) -> Result<(), &'static str> {
    if !dt.is_finite() || dt <= 0. {
        return Err("invalid hair energy timestep");
    }
    if poses
        .iter()
        .any(|(x, q)| x.len() != rod.x.len() || q.len() != rod.q.len())
    {
        return Err("hair energy pose shape mismatch");
    }
    for i in 1..rod.x.len() {
        visit(
            ResidualWeight::Inertia {
                inverse: rod.inv_mass[i],
                dt_squared: dt * dt,
            },
            poses.map(|(x, _)| sub(x[i], rod.predicted_x[i])),
        );
    }
    for i in 1..rod.q.len() {
        visit(
            ResidualWeight::Inertia {
                inverse: rod.inv_inertia[i],
                dt_squared: dt * dt,
            },
            poses.map(|(_, q)| log(qm(q[i], conj(rod.predicted_q[i])))),
        );
    }
    let ea = rod.material.young_modulus * rod.material.area();
    let ga = ea / (2. * (1. + rod.material.poisson_ratio));
    for i in 0..rod.q.len() {
        let basis: [[V; 3]; N] = poses.map(|(_, q)| {
            std::array::from_fn(|axis| {
                let mut e = [0.; 3];
                e[axis] = 1.;
                rotate(q[i], e)
            })
        });
        let delta: [V; N] = std::array::from_fn(|p| {
            sub(
                sub(poses[p].0[i + 1], poses[p].0[i]),
                mul(basis[p][2], rod.lengths[i]),
            )
        });
        for axis in 0..3 {
            visit(
                ResidualWeight::Elastic(0.5 * (if axis == 2 { ea } else { ga }) / rod.lengths[i]),
                std::array::from_fn(|p| [dot(delta[p], basis[p][axis]), 0., 0.]),
            );
        }
    }
    for i in 0..rod.rest_relative.len() {
        let relative = poses.map(|(_, q)| {
            let mut r = qm(conj(q[i]), q[i + 1]);
            if r.iter()
                .zip(rod.rest_relative[i])
                .map(|(a, b)| a * b)
                .sum::<f64>()
                < 0.
            {
                r = r.map(|v| -v);
            }
            r
        });
        let length = (rod.lengths[i] + rod.lengths[i + 1]) * 0.5;
        for axis in 0..3 {
            let rigidity = if axis == 2 {
                rod.material.twisting_rigidity()
            } else {
                rod.material.bending_rigidity()
            };
            visit(
                ResidualWeight::Elastic(0.5 * rigidity / length),
                relative.map(|r| [2. * (r[axis] - rod.rest_relative[i][axis]), 0., 0.]),
            );
        }
    }
    Ok(())
}
pub(super) fn structural(rod: &HairRod, dt: f64) -> Result<f64, &'static str> {
    let mut energy = 0.;
    residuals(rod, [(&rod.x, &rod.q)], dt, |weight, [r]| {
        energy += weight.apply(dot(r, r))
    })?;
    if !energy.is_finite() {
        return Err("hair implicit energy overflow");
    }
    Ok(energy)
}
/// Signed actual potential change; do not subtract rounded total energies.
/// The material, predictions and timestep belong to the same frozen solve.
pub(super) fn structural_delta(
    rod: &HairRod,
    old_x: &[V],
    old_q: &[Q],
    dt: f64,
) -> Result<f64, &'static str> {
    let mut sum = 0f64;
    let mut correction = 0f64;
    residuals(
        rod,
        [(old_x, old_q), (&rod.x, &rod.q)],
        dt,
        |weight, [a, b]| {
            let term = weight.apply(dot(sub(b, a), add(b, a)));
            // Neumaier accumulation preserves small signed terms across rods'
            // inertia, stretch/shear and bend/twist energy scales.
            let next = sum + term;
            correction += if sum.abs() >= term.abs() {
                (sum - next) + term
            } else {
                (term - next) + sum
            };
            sum = next;
        },
    )?;
    let delta = sum + correction;
    if !delta.is_finite() {
        return Err("hair energy difference overflow");
    }
    Ok(delta)
}

/// Diagnostic partition of the same frozen implicit potential, never an
/// alternative acceptance metric. Grouping changes summation roundoff only.
#[cfg(test)]
pub(super) fn structural_delta_parts(
    rod: &HairRod,
    old_x: &[V],
    old_q: &[Q],
    dt: f64,
) -> Result<[f64; 2], &'static str> {
    let mut sums = [0f64; 2];
    let mut corrections = [0f64; 2];
    residuals(
        rod,
        [(old_x, old_q), (&rod.x, &rod.q)],
        dt,
        |weight, [a, b]| {
            let group = match weight {
                ResidualWeight::Inertia { .. } => 0,
                ResidualWeight::Elastic(_) => 1,
            };
            let term = weight.apply(dot(sub(b, a), add(b, a)));
            let sum = sums[group];
            let next = sum + term;
            corrections[group] += if sum.abs() >= term.abs() {
                (sum - next) + term
            } else {
                (term - next) + sum
            };
            sums[group] = next;
        },
    )?;
    let result = std::array::from_fn(|i| sums[i] + corrections[i]);
    if result.iter().all(|v| v.is_finite()) {
        Ok(result)
    } else {
        Err("hair energy inventory overflow")
    }
}
pub(super) fn implicit(rod: &HairRod, dt: f64) -> Result<f64, &'static str> {
    let mut energy = structural(rod, dt)?;
    let ea = rod.material.young_modulus * rod.material.area();
    for contact in &rod.contacts {
        let i = contact.segment;
        let t = contact.fraction;
        let p = add(mul(rod.x[i], 1. - t), mul(rod.x[i + 1], t));
        let gap = contact.physical_gap(p).min(0.);
        let stiffness =
            ea * 100. * ((1. - t) / rod.lengths[i.saturating_sub(1)] + t / rod.lengths[i]);
        energy += 0.5 * stiffness * gap * gap;
    }
    if !energy.is_finite() {
        return Err("hair implicit energy overflow");
    }
    Ok(energy)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_small_inertial_energy_does_not_overflow_its_coefficient() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            super::super::super::HairMaterial::default(),
        )
        .unwrap();
        let dt = 1e-6;
        rod.inv_mass[1] = 1e-300;
        let old_x = rod.x.clone();
        let old_q = rod.q.clone();
        assert_eq!(structural(&rod, dt).unwrap(), 0.);
        assert_eq!(structural_delta(&rod, &old_x, &old_q, dt).unwrap(), 0.);
        rod.x[1][0] = 1e-155;
        let expected = 0.5 * (1e-155 * 1e-155) / rod.inv_mass[1] / (dt * dt);
        assert!(expected.is_finite() && expected > 0.);
        assert!((structural(&rod, dt).unwrap() - expected).abs() < expected * 1e-12);
        assert!(
            (structural_delta(&rod, &old_x, &old_q, dt).unwrap() - expected).abs()
                < expected * 1e-12
        );
    }
    #[test]
    fn residual_difference_preserves_changes_hidden_by_total_energy() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            super::super::super::HairMaterial::default(),
        )
        .unwrap();
        let dt = 1. / 240.;
        // A large independent inertia term hides the small change in another
        // station; it must not decide whether that station's trial descends.
        rod.predicted_x[1][0] = 1e5;
        rod.x[2][0] = 1e-5;
        let old_x = rod.x.clone();
        let old_q = rod.q.clone();
        let before = structural(&rod, dt).unwrap();
        rod.x[2][0] += 1e-12;
        assert_eq!(structural(&rod, dt).unwrap(), before);
        let delta = structural_delta(&rod, &old_x, &old_q, dt).unwrap();
        assert!(delta > 0. && delta.is_finite());
        let new_x = rod.x.clone();
        let new_q = rod.q.clone();
        rod.x = old_x;
        rod.q = old_q;
        assert_eq!(structural_delta(&rod, &new_x, &new_q, dt).unwrap(), -delta);
        assert_eq!(structural_delta(&rod, &rod.x, &rod.q, dt).unwrap(), 0.);
        // At ordinary scales the same owner agrees with direct subtraction.
        rod.predicted_x[1][0] = 0.;
        let before = structural(&rod, dt).unwrap();
        let old_x = rod.x.clone();
        let old_q = rod.q.clone();
        rod.x[2][0] += 1e-5;
        apply(&mut rod.q[1], [0.01, -0.02, 0.03]);
        let expected = structural(&rod, dt).unwrap() - before;
        let actual = structural_delta(&rod, &old_x, &old_q, dt).unwrap();
        assert!((actual - expected).abs() < expected.abs() * 1e-12);
        assert!(structural_delta(&rod, &old_x[..2], &old_q, dt).is_err());
    }
    #[test]
    fn actual_energy_gradient_matches_physical_assembly() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0.003, 0.01, 0.], [0.006, 0.018, 0.004]],
            super::super::super::HairMaterial::default(),
        )
        .unwrap();
        rod.x[1][0] += 0.0007;
        apply(&mut rod.q[1], [0.03, -0.02, 0.01]);
        let target = add(rod.x[1], [0.001, 0., 0.]);
        let c = rod.record_contact(
            1,
            0.,
            [1., 0., 0.],
            target,
            crate::hair::ContactSource::Mesh(0),
        );
        rod.contacts[c].metric_scale = 0.2;
        let (_, rhs) = super::super::assemble(&mut rod, 1. / 240.).unwrap();
        for dof in 6..rhs.len() - 3 {
            let h = if dof % 6 < 3 { 1e-8 } else { 1e-6 };
            let mut plus = rod.clone();
            let mut minus = rod.clone();
            if dof % 6 < 3 {
                plus.x[dof / 6][dof % 6] += h;
                minus.x[dof / 6][dof % 6] -= h;
            } else {
                let mut angle = [0.; 3];
                angle[dof % 6 - 3] = h;
                apply(&mut plus.q[dof / 6], angle);
                apply(&mut minus.q[dof / 6], mul(angle, -1.));
            }
            let derivative = (implicit(&plus, 1. / 240.).unwrap()
                - implicit(&minus, 1. / 240.).unwrap())
                / (2. * h);
            assert!(
                (derivative + rhs[dof]).abs() < 1e-7 * derivative.abs().max(1e-4),
                "dof={dof} derivative={derivative} rhs={}",
                rhs[dof]
            );
        }
        let before = implicit(&rod, 1. / 240.).unwrap();
        super::super::solve(&mut rod, 1. / 240.).unwrap();
        assert!(implicit(&rod, 1. / 240.).unwrap() <= before);
    }
}
