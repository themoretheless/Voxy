use physics::{
    astrophysics_spin::Spin,
    contact::{ContactBody, resolve_normal_impact},
    gravity::Body,
};
fn body(position: [f64; 3], velocity: [f64; 3], spin: bool) -> ContactBody {
    ContactBody {
        motion: Body {
            mass: 1.,
            position,
            velocity,
        },
        spin: spin.then_some(Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0.; 3],
            inertia: [1.; 3],
        }),
    }
}
fn angular(b: ContactBody) -> [f64; 3] {
    let p = b.motion.position;
    let m = b.motion.velocity.map(|v| b.motion.mass * v);
    let spin = b.spin.map_or([0.; 3], |s| s.angular_momentum);
    [
        p[1] * m[2] - p[2] * m[1] + spin[0],
        p[2] * m[0] - p[0] * m[2] + spin[1],
        p[0] * m[1] - p[1] * m[0] + spin[2],
    ]
}
#[test]
fn off_center_particle_impact_closes_linear_angular_momentum_and_energy() {
    for restitution in [0., 0.4, 1.] {
        let mut particle = body([-1., 1., 0.], [3., 0., 0.], false);
        let mut rigid = body([0.; 3], [0.; 3], true);
        let before_l = angular(particle);
        let energy = particle.energy().unwrap();
        let result = resolve_normal_impact(
            &mut particle,
            Some(&mut rigid),
            [-1., 1., 0.],
            [-1., 0., 0.],
            restitution,
        )
        .unwrap();
        let impulse = 1. + restitution;
        assert!((rigid.motion.velocity[0] - impulse).abs() < 1e-12);
        assert!((rigid.spin.unwrap().angular_momentum[2] + impulse).abs() < 1e-12);
        assert!((result.inverse_effective_mass - 3.).abs() < 1e-12);
        assert!((particle.motion.velocity[0] + rigid.motion.velocity[0] - 3.).abs() < 1e-12);
        for a in 0..3 {
            assert!((angular(particle)[a] + angular(rigid)[a] - before_l[a]).abs() < 1e-12);
        }
        assert!(
            (particle.energy().unwrap() + rigid.energy().unwrap() + result.dissipated_energy
                - energy)
                .abs()
                < 1e-12
        );
        let relative = particle.point_velocity([-1., 1., 0.]).unwrap()[0]
            - rigid.point_velocity([-1., 1., 0.]).unwrap()[0];
        assert!((relative + 3. * restitution).abs() < 1e-12);
    }
}
#[test]
fn two_rotating_bodies_share_point_impulse_and_static_boundary_receives_torque() {
    let mut first = body([-1., 0., 0.], [2., 0., 0.], true);
    let mut second = body([1., 0., 0.], [0.; 3], true);
    let before = first.energy().unwrap();
    let l = angular(first);
    let report = resolve_normal_impact(
        &mut first,
        Some(&mut second),
        [0., 1., 0.],
        [-1., 0., 0.],
        1.,
    )
    .unwrap();
    for a in 0..3 {
        assert!((angular(first)[a] + angular(second)[a] - l[a]).abs() < 1e-12);
    }
    assert!((first.energy().unwrap() + second.energy().unwrap() - before).abs() < 1e-12);
    assert_eq!(report.dissipated_energy, 0.);
    let mut wall_body = body([0.; 3], [2., 0., 0.], true);
    let report =
        resolve_normal_impact(&mut wall_body, None, [0., 1., 0.], [-1., 0., 0.], 0.).unwrap();
    assert_eq!(wall_body.motion.velocity, [1., 0., 0.]);
    assert_eq!(wall_body.spin.unwrap().angular_momentum, [0., 0., 1.]);
    assert!((wall_body.energy().unwrap() + report.dissipated_energy - 2.).abs() < 1e-12);
}
#[test]
fn orientation_changes_contact_effective_mass_and_spin_drifts_using_existing_integrator() {
    let mut a = body([0.; 3], [1., 0., 0.], true);
    a.spin.as_mut().unwrap().inertia = [1., 2., 3.];
    let q = std::f64::consts::FRAC_PI_4;
    a.spin.as_mut().unwrap().orientation = [q.sin(), 0., 0., q.cos()];
    let result = resolve_normal_impact(&mut a, None, [0., 1., 0.], [-1., 0., 0.], 0.).unwrap();
    assert!((result.inverse_effective_mass - 1.5).abs() < 1e-12);
    let mut spin = a.spin.unwrap();
    let before = spin;
    spin.step([0.; 3], 0.01).unwrap();
    assert_ne!(spin.orientation, before.orientation);
    assert_eq!(spin.angular_momentum, before.angular_momentum);
    assert!((spin.energy().unwrap() - before.energy().unwrap()).abs() < 1e-12);
}
#[test]
fn late_second_body_overflow_and_invalid_contact_preserve_both() {
    let mut a = body([-1., 1., 0.], [3., 0., 0.], false);
    let mut b = body([0.; 3], [0.; 3], true);
    // Contact velocity is representable, but second-owner kinetic energy is not.
    b.motion.velocity = [0., 1e200, 0.];
    let before = (a, b);
    assert!(resolve_normal_impact(&mut a, Some(&mut b), [-1., 1., 0.], [-1., 0., 0.], 0.).is_err());
    assert_eq!((a, b), before);
    assert!(resolve_normal_impact(&mut a, Some(&mut b), [-1., 1., 0.], [-2., 0., 0.], 0.).is_err());
    assert_eq!((a, b), before);
}

#[test]
fn small_mass_large_speed_has_representable_impulse_and_energy() {
    let mut particle = body([0.; 3], [1e200, 0., 0.], false);
    particle.motion.mass = 1e-300;
    let energy = particle.energy().unwrap();
    assert!((energy / 5e99 - 1.).abs() < 1e-12);
    let report = resolve_normal_impact(&mut particle, None, [0.; 3], [-1., 0., 0.], 0.).unwrap();
    assert!(particle.motion.velocity[0].abs() / 1e200 < 1e-15);
    assert!((report.dissipated_energy / energy - 1.).abs() < 1e-12);
}
