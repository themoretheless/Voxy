use physics::strand::SphereCollider;
use physics::tissue::{Material, Tissue, TissueKind, sample};
#[test]
fn rigid_translation_preserves_volume() {
    let mut b = Tissue::new(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        vec![1.0; 4],
        vec![],
        vec![[0, 1, 2, 3]],
        Material::default(),
    )
    .unwrap();
    for _ in 0..240 {
        b.step(1.0 / 240.0, [1.0, -9.81, 0.0], &[], 16).unwrap();
    }
    assert!((b.volumes()[0] - 1.0 / 6.0).abs() < 1e-10);
}
#[test]
fn activation_contracts_ring() {
    let mut relaxed = sample(TissueKind::Sphincter, [0.0; 3]).unwrap();
    let mut active = relaxed.clone();
    active.set_activation(1.0).unwrap();
    for _ in 0..480 {
        relaxed.step(1.0 / 240.0, [0.0; 3], &[], 24).unwrap();
        active.step(1.0 / 240.0, [0.0; 3], &[], 24).unwrap();
    }
    let radius = |b: &Tissue| b.positions().iter().map(|p| p[0].hypot(p[1])).sum::<f64>() / 24.0;
    assert!(radius(&active) < radius(&relaxed) * 0.75);
    active.set_activation(0.0).unwrap();
    for _ in 0..480 {
        active.step(1.0 / 240.0, [0.0; 3], &[], 24).unwrap();
    }
    assert!((radius(&active) - 0.35).abs() < 0.01);
}
#[test]
fn invalid_step_is_atomic_and_contact_projects() {
    let mut b = Tissue::new(
        vec![[0.0; 3]],
        vec![1.0],
        vec![],
        vec![],
        Material::default(),
    )
    .unwrap();
    let old = b.positions().to_vec();
    assert!(b.step(f64::NAN, [0.0; 3], &[], 24).is_err());
    assert_eq!(b.positions(), old);
    b.step(
        1.0 / 240.0,
        [0.0; 3],
        &[SphereCollider {
            center: [0.0; 3],
            radius: 0.3,
        }],
        24,
    )
    .unwrap();
    assert!((b.positions()[0][0] - 0.31).abs() < 1e-12);
}
#[test]
fn pinned_samples_remain_finite_and_keep_volume() {
    for kind in [
        TissueKind::Skin,
        TissueKind::Buttock,
        TissueKind::Breast,
        TissueKind::Lip,
    ] {
        let mut b = sample(kind, [0.0; 3]).unwrap();
        let initial = b.volumes();
        for _ in 0..2400 {
            b.step(1.0 / 240.0, [0.0, -2.0, 0.0], &[], 24).unwrap();
        }
        for (v, r) in b.volumes().iter().zip(initial) {
            assert!(v / r > 0.9 && v / r < 1.1, "volume ratio {}", v / r);
        }
    }
}
#[test]
fn rejects_degenerate_topology() {
    assert!(
        Tissue::new(
            vec![[0.0; 3]; 4],
            vec![1.0; 4],
            vec![],
            vec![[0, 1, 2, 3]],
            Material::default()
        )
        .is_err()
    );
}

#[test]
fn compliant_link_matches_static_load_at_two_timesteps() {
    for dt in [1.0 / 120.0, 1.0 / 240.0] {
        let mut b = Tissue::new(
            vec![[0.0; 3], [0.0, -1.0, 0.0]],
            vec![0.0, 1.0],
            vec![([0, 1], false)],
            vec![],
            Material {
                stretch_compliance: 0.01,
                damping: 8.0,
                ..Material::default()
            },
        )
        .unwrap();
        let mut time = 0.0;
        while time < 5.0 {
            b.step(dt, [0.0, -9.81, 0.0], &[], 16).unwrap();
            time += dt;
        }
        // Static extension = compliance * mass * gravity; damping introduces O(dt) bias.
        assert!((-b.positions()[1][1] - 1.0981).abs() < 0.007);
        assert_eq!(b.positions()[0], [0.0; 3]);
    }
}

#[test]
fn shaft_bends_recovers_and_preserves_anchored_base() {
    let mut b = sample(TissueKind::Penis, [0.0; 3]).unwrap();
    let rest = b.positions().to_vec();
    let volumes = b.volumes();
    for _ in 0..720 {
        b.step(1.0 / 240.0, [0.0, -2.0, 0.0], &[], 24).unwrap();
    }
    let tip_y = |b: &Tissue| b.positions()[20..].iter().map(|p| p[1]).sum::<f64>() / 4.0;
    assert!(tip_y(&b) < -0.005, "shaft must bend under load");
    assert_eq!(&b.positions()[..4], &rest[..4]);
    for (v, r) in b.volumes().iter().zip(&volumes) {
        assert!(v / r > 0.9 && v / r < 1.1, "volume ratio {}", v / r);
    }
    for _ in 0..2400 {
        b.step(1.0 / 240.0, [0.0; 3], &[], 24).unwrap();
    }
    assert!(tip_y(&b).abs() < 0.005, "shaft must recover after release");
}
#[test]
fn nonlinear_edge_hardens_under_load() {
    let make = || {
        Tissue::new(
            vec![[0.; 3], [0.1, 0., 0.]],
            vec![0., 1.],
            vec![([0, 1], false)],
            vec![],
            Material {
                stretch_compliance: 0.001,
                damping: 8.0,
                ..Material::default()
            },
        )
        .unwrap()
    };
    let mut linear = make();
    let mut hard = make();
    hard.set_hardening(20.).unwrap();
    for _ in 0..2000 {
        linear.step(1. / 240., [10., 0., 0.], &[], 24).unwrap();
        hard.step(1. / 240., [10., 0., 0.], &[], 24).unwrap();
    }
    assert!(hard.positions()[1][0] < linear.positions()[1][0]);
    assert!(hard.positions()[1][0] > 0.1);
}
#[test]
fn friction_reduces_sliding_and_invalid_contact_is_atomic() {
    let make = || {
        Tissue::new(
            vec![[0., 0.51, 0.]],
            vec![1.],
            vec![],
            vec![],
            Material {
                particle_radius: 0.01,
                damping: 0.,
                ..Material::default()
            },
        )
        .unwrap()
    };
    let sphere = SphereCollider {
        center: [0.; 3],
        radius: 0.5,
    };
    let mut free = make();
    let mut sticky = make();
    for _ in 0..20 {
        free.step_with_contact(1. / 240., [1., -9.81, 0.], &[sphere], 24, 0.)
            .unwrap();
        sticky
            .step_with_contact(1. / 240., [1., -9.81, 0.], &[sphere], 24, 1.)
            .unwrap();
    }
    assert!(sticky.positions()[0][0] < free.positions()[0][0] * 0.3);
    let old = sticky.positions().to_vec();
    assert!(
        sticky
            .step_with_contact(1. / 240., [0.; 3], &[sphere], 24, f64::NAN)
            .is_err()
    );
    assert_eq!(old, sticky.positions());
}
#[test]
fn sphere_contacts_triangle_interior_without_vertex_overlap() {
    let mut body = Tissue::new(
        vec![[-1., 0., -1.], [1., 0., -1.], [0., 0., 1.], [0., 1., 0.]],
        vec![1.; 4],
        vec![],
        vec![[0, 1, 2, 3]],
        Material {
            volume_compliance: 0.01,
            particle_radius: 0.0,
            ..Material::default()
        },
    )
    .unwrap();
    let sphere = SphereCollider {
        center: [0., -0.05, 0.],
        radius: 0.2,
    };
    assert!(body.positions().iter().all(|p| {
        (0..3)
            .map(|k| (p[k] - sphere.center[k]).powi(2))
            .sum::<f64>()
            > 0.04
    }));
    body.step_with_contact(1. / 240., [0.; 3], &[sphere], 64, 0.3)
        .unwrap();
    assert!(
        body.positions()[0][1] > 0.05
            || body.positions()[1][1] > 0.05
            || body.positions()[2][1] > 0.05
    );
    assert_eq!(body.surface_triangles().len(), 4);
}
#[test]
fn triangle_contact_friction_reduces_tangential_motion() {
    let make = || {
        Tissue::new(
            vec![
                [-1., 0.2, -1.],
                [1., 0.2, -1.],
                [0., 0.2, 1.],
                [0., 1.2, 0.],
            ],
            vec![1.; 4],
            vec![],
            vec![[0, 1, 2, 3]],
            Material {
                volume_compliance: 0.1,
                particle_radius: 0.0,
                damping: 0.0,
                ..Material::default()
            },
        )
        .unwrap()
    };
    let mut free = make();
    let mut friction = make();
    let sphere = SphereCollider {
        center: [0.; 3],
        radius: 0.2,
    };
    for _ in 0..30 {
        free.step_with_contact(1. / 240., [1., -9.81, 0.], &[sphere], 64, 0.0)
            .unwrap();
        friction
            .step_with_contact(1. / 240., [1., -9.81, 0.], &[sphere], 64, 0.8)
            .unwrap();
    }
    let drift = |body: &Tissue| body.positions()[..3].iter().map(|p| p[0]).sum::<f64>();
    assert!(drift(&friction) < drift(&free));
}
