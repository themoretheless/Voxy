use physics::gravity::{Error, Gravity};
use physics::gravity_field::{GravityField, NewtonianField, Source};
use physics::{
    AnchoredAabb, CharacterConfig, CharacterInput, CharacterState, CollisionWorld, Origin,
    SweepResult, step_character, step_character_in_field,
};

fn source(anchor: Origin, radius: f64) -> Source {
    Source {
        anchor,
        position: [0.0; 3],
        mass: 8.0,
        radius,
    }
}
fn settings() -> Gravity {
    Gravity {
        constant: 1.0,
        ..Gravity::default()
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}

#[test]
fn sphere_has_linear_interior_and_inverse_square_exterior() {
    let sources = [source(Origin::default(), 2.0)];
    let field = NewtonianField {
        gravity: settings(),
        sources: &sources,
    };
    near(
        field
            .acceleration(Origin::default(), [1.0, 0.0, 0.0])
            .unwrap()[0],
        -1.0,
    );
    near(
        field
            .acceleration(Origin::default(), [2.0, 0.0, 0.0])
            .unwrap()[0],
        -2.0,
    );
    near(
        field
            .acceleration(Origin::default(), [4.0, 0.0, 0.0])
            .unwrap()[0],
        -0.5,
    );
    for component in field.acceleration(Origin::default(), [0.0; 3]).unwrap() {
        near(component, 0.0);
    }
}

#[test]
fn far_origins_preserve_nearby_fractional_distance() {
    let anchor = Origin {
        x: 1_i64 << 60,
        y: i64::MIN,
        z: i64::MAX,
    };
    let sources = [source(anchor, 0.0)];
    let field = NewtonianField {
        gravity: settings(),
        sources: &sources,
    };
    let query = Origin {
        x: anchor.x + 1,
        ..anchor
    };
    near(
        field.acceleration(query, [0.5, 0.0, 0.0]).unwrap()[0],
        -8.0 / 2.25,
    );
}

#[test]
fn source_fields_superpose_and_singularities_report_errors() {
    let sources = [
        source(
            Origin {
                x: -2,
                ..Origin::default()
            },
            0.0,
        ),
        source(
            Origin {
                x: 2,
                ..Origin::default()
            },
            0.0,
        ),
    ];
    let field = NewtonianField {
        gravity: Gravity {
            uniform_acceleration: [0.0, -9.81, 0.0],
            ..settings()
        },
        sources: &sources,
    };
    for (actual, expected) in field
        .acceleration(Origin::default(), [0.0; 3])
        .unwrap()
        .into_iter()
        .zip([0.0, -9.81, 0.0])
    {
        near(actual, expected);
    }
    assert_eq!(
        field.acceleration(sources[0].anchor, [0.0; 3]),
        Err(Error::SingularPair)
    );
}

struct Empty;
impl CollisionWorld for Empty {
    type Obstacle = ();
    type Error = ();
    fn sweep_aabb(
        &self,
        _body: AnchoredAabb,
        _d: [f64; 3],
        _budget: usize,
    ) -> Result<SweepResult<()>, ()> {
        Ok(SweepResult {
            fraction: 1.0,
            normal: [0; 3],
            obstacle: None,
        })
    }
}
fn character() -> CharacterState {
    CharacterState {
        body: AnchoredAabb {
            anchor: Origin::default(),
            min: [0.0; 3],
            max: [1.0; 3],
        },
        velocity: [0.0; 3],
        grounded: false,
    }
}

#[test]
fn uniform_character_field_is_identical_and_radial_field_moves_in_three_axes() {
    let input = CharacterInput {
        planar_velocity: [0.2, -0.3],
        jump_pressed: false,
    };
    let config = CharacterConfig::default();
    let mut old = character();
    let mut sampled = old;
    for _ in 0..20 {
        let a = step_character(&Empty, &mut old, input, 0.01, config).unwrap();
        let b = step_character_in_field(
            &Empty,
            &mut sampled,
            input,
            0.01,
            config,
            &[0.0, config.gravity, 0.0],
        )
        .unwrap();
        assert_eq!(a, b);
        assert_eq!(old, sampled);
    }
    let sources = [source(Origin { x: 3, y: 3, z: 3 }, 1.0)];
    let field = NewtonianField {
        gravity: settings(),
        sources: &sources,
    };
    let mut radial = character();
    step_character_in_field(
        &Empty,
        &mut radial,
        CharacterInput {
            planar_velocity: [0.0; 2],
            jump_pressed: false,
        },
        0.1,
        config,
        &field,
    )
    .unwrap();
    assert!(radial.velocity.iter().all(|v| *v > 0.0));
    let before = radial;
    assert!(
        step_character_in_field(
            &Empty,
            &mut radial,
            input,
            0.1,
            config,
            &[f64::NAN, 0.0, 0.0]
        )
        .is_err()
    );
    assert_eq!(radial, before);
}
