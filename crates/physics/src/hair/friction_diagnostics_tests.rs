use super::*;
#[test]
fn friction_observation_preserves_equal_opposite_response_bitwise() {
    let a = HairRod::new(
        vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
        Default::default(),
    )
    .unwrap();
    let mut rods = vec![a.clone(), a];
    rods[0].velocity[1] = [1., -0.25, 0.];
    rods[1].velocity[1] = [-0.5, 0.5, 0.];
    let responses = [StrandResponse {
        a: (0, 1, 0.),
        b: (1, 1, 0.),
        normal: [0., 1., 0.],
        impulse: 1e-9,
    }];
    let mut observed = rods.clone();
    let mut diagnostics = Vec::new();
    finish_strand_contacts(&mut rods, &responses, 1. / 240.);
    finish_strand_contacts_with_diagnostics(
        &mut observed,
        &responses,
        1. / 240.,
        1,
        Some(&mut diagnostics),
    );
    for (a, b) in rods.iter().zip(&observed) {
        assert_eq!(a.velocity, b.velocity);
        assert_eq!(a.x, b.x);
        assert_eq!(a.q, b.q);
    }
    assert_eq!(diagnostics.len(), 1);
    let entry = &diagnostics[0];
    assert_eq!(entry.substep, 1);
    assert_eq!(entry.relative_velocity, [1.5, -0.75, 0.]);
    assert_eq!(entry.position_impulse, 1e-9);
    assert!(entry.normal_impulse > 0. && entry.tangent_impulse > 0.);
    let momentum = add(
        mul(observed[0].velocity[1], 1. / observed[0].inv_mass[1]),
        mul(observed[1].velocity[1], 1. / observed[1].inv_mass[1]),
    );
    let original = mul([0.5, 0.25, 0.], 1. / observed[0].inv_mass[1]);
    assert!(len(sub(momentum, original)) < 1e-20);
}
