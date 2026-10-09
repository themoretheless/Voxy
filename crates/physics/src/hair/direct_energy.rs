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
pub(super) fn implicit(rod: &HairRod, dt: f64) -> Result<f64, &'static str> {
    if !dt.is_finite() || dt <= 0. {
        return Err("invalid hair energy timestep");
    }
    let mut energy = 0.;
    for i in 1..rod.x.len() {
        let delta = sub(rod.x[i], rod.predicted_x[i]);
        energy += 0.5 * dot(delta, delta) / rod.inv_mass[i] / (dt * dt);
    }
    for i in 1..rod.q.len() {
        let rotation = log(qm(rod.q[i], conj(rod.predicted_q[i])));
        energy += 0.5 * dot(rotation, rotation) / rod.inv_inertia[i] / (dt * dt);
    }
    let ea = rod.material.young_modulus * rod.material.area();
    let ga = ea / (2. * (1. + rod.material.poisson_ratio));
    for i in 0..rod.q.len() {
        let basis: [V; 3] = std::array::from_fn(|axis| {
            let mut e = [0.; 3];
            e[axis] = 1.;
            rotate(rod.q[i], e)
        });
        let delta = sub(sub(rod.x[i + 1], rod.x[i]), mul(basis[2], rod.lengths[i]));
        for axis in 0..3 {
            energy += 0.5 * (if axis == 2 { ea } else { ga }) / rod.lengths[i]
                * dot(delta, basis[axis]).powi(2);
        }
    }
    for i in 0..rod.rest_relative.len() {
        let mut relative = qm(conj(rod.q[i]), rod.q[i + 1]);
        if relative
            .iter()
            .zip(rod.rest_relative[i])
            .map(|(a, b)| a * b)
            .sum::<f64>()
            < 0.
        {
            relative = relative.map(|x| -x);
        }
        let length = (rod.lengths[i] + rod.lengths[i + 1]) * 0.5;
        for axis in 0..3 {
            let rigidity = if axis == 2 {
                rod.material.twisting_rigidity()
            } else {
                rod.material.bending_rigidity()
            };
            energy += 0.5 * rigidity / length
                * (2. * (relative[axis] - rod.rest_relative[i][axis])).powi(2);
        }
    }
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
    fn actual_energy_gradient_matches_physical_assembly() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0.003, 0.01, 0.], [0.006, 0.018, 0.004]],
            super::super::super::HairMaterial::default(),
        )
        .unwrap();
        rod.x[1][0] += 0.0007;
        apply(&mut rod.q[1], [0.03, -0.02, 0.01]);
        let target=add(rod.x[1],[0.001,0.,0.]);
        let c=rod.record_contact(1,0.,[1.,0.,0.],target,crate::hair::ContactSource::Mesh(0));
        rod.contacts[c].metric_scale=0.2;
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
