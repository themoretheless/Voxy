use physics::{
    Origin,
    gravity::Gravity,
    gravity_character::{
        self as controller, CollisionWorld, Config, Error, Hit, Input, SphereWorld, State,
        SurfaceSphere,
    },
    gravity_field::{NewtonianField, Source},
};

fn length(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn planet() -> SurfaceSphere {
    SurfaceSphere {
        anchor: Origin::default(),
        center: [0.0; 3],
        radius: 2.0,
    }
}
fn source() -> Source {
    Source {
        anchor: Origin::default(),
        position: [0.0; 3],
        radius: 2.0,
        mass: 50.0,
    }
}
fn position(state: &State) -> [f64; 3] {
    #[allow(clippy::cast_precision_loss)]
    let origin = [
        state.anchor.x as f64,
        state.anchor.y as f64,
        state.anchor.z as f64,
    ];
    std::array::from_fn(|k| origin[k] + state.center[k])
}
fn initial(up: [f64; 3]) -> State {
    State {
        anchor: Origin::default(),
        center: up.map(|v| v * 2.2),
        radius: 0.2,
        velocity: [0.0; 3],
        up,
        grounded: true,
    }
}

#[test]
fn walks_all_the_way_around_planet_without_a_global_up_axis() {
    let surfaces = [planet()];
    let sources = [source()];
    let world = SphereWorld {
        surfaces: &surfaces,
    };
    let field = NewtonianField {
        gravity: Gravity {
            constant: 1.0,
            ..Default::default()
        },
        sources: &sources,
    };
    let mut state = initial([0.0, 1.0, 0.0]);
    let mut saw_bottom = false;
    let mut saw_left = false;
    let mut saw_right = false;
    for _ in 0..4000 {
        let up = state.up;
        let input = Input {
            tangent_velocity: Some([up[1], -up[0], 0.0]),
            jump_pressed: false,
        };
        controller::step(
            &world,
            &field,
            &mut state,
            input,
            1.0 / 240.0,
            Config::default(),
        )
        .unwrap();
        let p = position(&state);
        assert!((length(p) - 2.2).abs() < 1e-8, "position {p:?}");
        assert!(state.grounded);
        assert!((length(state.up) - 1.0).abs() < 1e-12);
        saw_bottom |= p[1] < -2.0;
        saw_left |= p[0] < -2.0;
        saw_right |= p[0] > 2.0;
    }
    assert!(saw_bottom && saw_left && saw_right);
}

#[test]
fn jumps_and_lands_in_six_radial_directions() {
    let surfaces = [planet()];
    let sources = [source()];
    let world = SphereWorld {
        surfaces: &surfaces,
    };
    let field = NewtonianField {
        gravity: Gravity {
            constant: 1.0,
            ..Default::default()
        },
        sources: &sources,
    };
    for up in [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ] {
        let mut state = initial(up);
        let report = controller::step(
            &world,
            &field,
            &mut state,
            Input {
                tangent_velocity: None,
                jump_pressed: true,
            },
            1.0 / 240.0,
            Config::default(),
        )
        .unwrap();
        assert!(report.jumped);
        assert!(!state.grounded);
        assert!(length(position(&state)) > 2.2);
        let mut max_radius = 2.2_f64;
        let mut landed = false;
        for _ in 0..600 {
            controller::step(
                &world,
                &field,
                &mut state,
                Input::default(),
                1.0 / 240.0,
                Config::default(),
            )
            .unwrap();
            max_radius = max_radius.max(length(position(&state)));
            if state.grounded {
                landed = true;
                break;
            }
        }
        assert!(
            landed && max_radius > 3.0,
            "jump direction {up:?}, max {max_radius}"
        );
        assert!((length(position(&state)) - 2.2).abs() < 1e-8);
    }
}

#[test]
fn sphere_sweep_prevents_tunneling_and_retains_far_precision() {
    let anchor = Origin {
        x: 1_i64 << 60,
        y: 0,
        z: 0,
    };
    let surfaces = [SurfaceSphere { anchor, ..planet() }];
    let world = SphereWorld {
        surfaces: &surfaces,
    };
    let state = State {
        anchor,
        center: [-10.0, 0.0, 0.0],
        ..initial([-1.0, 0.0, 0.0])
    };
    let hit = world.sweep(&state, [100.0, 0.0, 0.0], 1).unwrap();
    assert!((hit.fraction - 0.078).abs() < 1e-12);
    assert!(
        (hit.normal[0] + 1.0).abs() < 1e-12
            && hit.normal[1].abs() < 1e-12
            && hit.normal[2].abs() < 1e-12
    );
    assert_eq!(
        world.sweep(&state, [100.0, 0.0, 0.0], 0),
        Err(Error::BudgetExceeded)
    );
}

struct Empty;
impl CollisionWorld for Empty {
    fn sweep(&self, _state: &State, _displacement: [f64; 3], _budget: usize) -> Result<Hit, Error> {
        Ok(Hit {
            fraction: 1.0,
            normal: [0.0; 3],
            obstacle: None,
        })
    }
}
#[test]
fn zero_gravity_preserves_momentum_and_failures_are_atomic() {
    let mut state = initial([0.0, 0.0, 1.0]);
    state.grounded = false;
    state.velocity = [1.0, 2.0, 3.0];
    let start = position(&state);
    controller::step(
        &Empty,
        &[0.0; 3],
        &mut state,
        Input::default(),
        0.1,
        Config::default(),
    )
    .unwrap();
    let end = position(&state);
    for k in 0..3 {
        assert!((end[k] - start[k] - state.velocity[k] * 0.1).abs() < 1e-12);
    }
    assert!(
        state.up[0].abs() < 1e-12 && state.up[1].abs() < 1e-12 && (state.up[2] - 1.0).abs() < 1e-12
    );
    let before = state;
    assert!(
        controller::step(
            &Empty,
            &[f64::NAN, 0.0, 0.0],
            &mut state,
            Input::default(),
            0.1,
            Config::default()
        )
        .is_err()
    );
    assert_eq!(state, before);
    let surfaces = [SurfaceSphere {
        anchor: state.anchor,
        center: state.center,
        radius: 2.0,
    }];
    assert_eq!(
        controller::step(
            &SphereWorld {
                surfaces: &surfaces
            },
            &[0.0, -10.0, 0.0],
            &mut state,
            Input::default(),
            0.1,
            Config::default()
        ),
        Err(Error::InitialOverlap)
    );
    assert_eq!(state, before);
}

#[test]
fn invalid_backend_and_far_coordinate_overflow_do_not_publish_state() {
    struct PartialMiss;
    impl CollisionWorld for PartialMiss {
        fn sweep(&self, _s: &State, _d: [f64; 3], _budget: usize) -> Result<Hit, Error> {
            Ok(Hit {
                fraction: 0.5,
                normal: [0.0; 3],
                obstacle: None,
            })
        }
    }
    let mut state = initial([0.0, 1.0, 0.0]);
    let before = state;
    assert_eq!(
        controller::step(
            &PartialMiss,
            &[0.0; 3],
            &mut state,
            Input::default(),
            0.1,
            Config::default()
        ),
        Err(Error::InvalidContact)
    );
    assert_eq!(state, before);
    state.anchor.x = i64::MAX;
    state.center[0] = 0.5;
    state.velocity[0] = 2.0;
    state.grounded = false;
    let before = state;
    assert_eq!(
        controller::step(
            &Empty,
            &[0.0; 3],
            &mut state,
            Input::default(),
            1.0,
            Config::default()
        ),
        Err(Error::CoordinateOverflow)
    );
    assert_eq!(state, before);
}

#[test]
fn extreme_finite_field_keeps_a_unit_up_vector() {
    let mut state = initial([0.0, 1.0, 0.0]);
    state.grounded = false;
    controller::step(
        &Empty,
        &[f64::MAX; 3],
        &mut state,
        Input::default(),
        1e-300,
        Config::default(),
    )
    .unwrap();
    assert!((length(state.up) - 1.0).abs() < 1e-12);
}
