use physics::liquid::{
    Config, DropletCoalescenceControl, DropletSplit, Liquid, Material, Particle, ParticleInput,
};
fn fluid() -> Liquid {
    Liquid::new(
        vec![
            Particle {
                position: [0.1, 0.0, 0.0],
                velocity: [2.0, 0.0, 0.0],
                mass: 1e-6,
                material: 0,
            },
            Particle {
                position: [-0.1, 0.0, 0.0],
                velocity: [-2.0, 0.0, 0.0],
                mass: 1e-6,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
fn control() -> DropletCoalescenceControl {
    DropletCoalescenceControl {
        dt: 0.1,
        surface_tension: 0.072,
        maximum_normal_speed: 10.0,
        max_events: 4,
    }
}
#[test]
fn only_two_marked_droplets_can_coalesce() {
    for (flags, events) in [
        (vec![false, false], 0),
        (vec![true, false], 0),
        (vec![true, true], 1),
    ] {
        let mut l = fluid();
        l.configure_droplet_population(Some(flags)).unwrap();
        let r = l
            .coalesce_swept_droplets(&[[-0.1, 0.0, 0.0], [0.1, 0.0, 0.0]], control())
            .unwrap();
        assert_eq!(r.events.len(), events);
        assert_eq!(l.droplet_population().unwrap().len(), l.particles().len());
        if events == 1 {
            assert_eq!(l.droplet_population().unwrap(), &[true]);
        }
    }
}
#[test]
fn source_defaults_and_survivors_keep_population_alignment() {
    let mut l = fluid();
    l.configure_droplet_population(Some(vec![true, false]))
        .unwrap();
    let source = ParticleInput {
        particle: Particle {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 1e-6,
            material: 0,
        },
        field: None,
        phase_fraction: None,
    };
    l.exchange_particles(&[1], &[source]).unwrap();
    assert_eq!(l.droplet_population().unwrap(), &[true, false]);
    l.exchange_particles(&[0], &[]).unwrap();
    assert_eq!(l.droplet_population().unwrap(), &[false]);
}
#[test]
fn explicit_fragmentation_marks_children_without_marking_survivors() {
    let mut l = fluid();
    l.configure_droplet_population(Some(vec![false, false]))
        .unwrap();
    l.split_droplet(
        0,
        DropletSplit {
            children: 3,
            axis: [0.0, 1.0, 0.0],
            position_radius: 0.001,
            surface_tension: 0.072,
            available_energy: 1e-5,
        },
    )
    .unwrap();
    assert_eq!(l.droplet_population().unwrap(), &[false, true, true, true]);
}
#[test]
fn invalid_mask_and_failed_split_leave_classification_and_state_unchanged() {
    let mut l = fluid();
    l.configure_droplet_population(Some(vec![true, false]))
        .unwrap();
    let original = l.clone();
    assert!(l.configure_droplet_population(Some(vec![true])).is_err());
    assert_eq!(l, original);
    assert!(
        l.split_droplet(
            0,
            DropletSplit {
                children: 3,
                axis: [0.0, 1.0, 0.0],
                position_radius: 0.001,
                surface_tension: 0.072,
                available_energy: 0.0
            }
        )
        .is_err()
    );
    assert_eq!(l, original);
}

#[test]
fn flow_step_retains_population_marks() {
    let mut l = fluid();
    l.configure_droplet_population(Some(vec![true, false]))
        .unwrap();
    l.step(0.001, None).unwrap();
    assert_eq!(l.droplet_population().unwrap(), &[true, false]);
}
