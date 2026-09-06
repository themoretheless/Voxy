use physics::{
    AnchoredAabb, CharacterConfig, CharacterError, CharacterInput, CharacterState, CollisionWorld,
    Origin, SweepResult, step_character, sweep_box,
};

// Deliberately no voxel/world dependency: arbitrary non-grid static boxes.
struct Boxes(Vec<([f64; 3], [f64; 3])>);

impl CollisionWorld for Boxes {
    type Obstacle = usize;
    type Error = &'static str;

    fn sweep_aabb(
        &self,
        body: AnchoredAabb,
        displacement: [f64; 3],
        budget: usize,
    ) -> Result<SweepResult<usize>, Self::Error> {
        if self.0.len() > budget {
            return Err("budget");
        }
        #[allow(clippy::cast_precision_loss)]
        let origin = [
            body.anchor.x as f64,
            body.anchor.y as f64,
            body.anchor.z as f64,
        ];
        let mut nearest = SweepResult {
            fraction: 1.0,
            normal: [0; 3],
            obstacle: None,
        };
        for (id, (min, max)) in self.0.iter().enumerate() {
            let min = std::array::from_fn(|axis| min[axis] - origin[axis]);
            let max = std::array::from_fn(|axis| max[axis] - origin[axis]);
            if let Some((fraction, normal)) = sweep_box(body, displacement, min, max)
                && (fraction < nearest.fraction || nearest.obstacle.is_none())
            {
                nearest = SweepResult {
                    fraction,
                    normal,
                    obstacle: Some(id),
                };
            }
        }
        Ok(nearest)
    }
}

fn state() -> CharacterState {
    CharacterState {
        body: AnchoredAabb {
            anchor: Origin::default(),
            min: [0.15, 0.35, 0.15],
            max: [0.85, 2.15, 0.85],
        },
        velocity: [0.0; 3],
        grounded: false,
    }
}

fn input() -> CharacterInput {
    CharacterInput {
        planar_velocity: [0.0; 2],
        jump_pressed: false,
    }
}

#[test]
fn lands_jumps_and_lands_on_non_grid_floor() {
    let world = Boxes(vec![([-20.5, -0.7, -20.5], [20.5, 0.35, 20.5])]);
    let mut body = state();
    step_character(
        &world,
        &mut body,
        input(),
        1.0 / 60.0,
        CharacterConfig::default(),
    )
    .unwrap();
    assert!(body.grounded);
    step_character(
        &world,
        &mut body,
        CharacterInput {
            jump_pressed: true,
            ..input()
        },
        1.0 / 60.0,
        CharacterConfig::default(),
    )
    .unwrap();
    assert!(!body.grounded);
    assert!(body.velocity[1] > 0.0);
    for _ in 0..120 {
        step_character(
            &world,
            &mut body,
            input(),
            1.0 / 60.0,
            CharacterConfig::default(),
        )
        .unwrap();
    }
    assert!(body.grounded);
    assert!((body.body.min[1] - 0.35).abs() < 1e-10);
}

#[test]
fn slides_against_arbitrary_box() {
    let world = Boxes(vec![([1.25, -1.7, -20.5], [1.8, 10.35, 20.5])]);
    let mut body = state();
    let report = step_character(
        &world,
        &mut body,
        CharacterInput {
            planar_velocity: [8.0, 4.0],
            ..input()
        },
        0.25,
        CharacterConfig {
            gravity: 0.0,
            ..CharacterConfig::default()
        },
    )
    .unwrap();
    assert!((report.applied_displacement[0] - 0.4).abs() < 1e-10);
    assert!((report.applied_displacement[2] - 1.0).abs() < 1e-10);
    assert_eq!(report.contacts[0].obstacle, 0);
}

#[test]
fn query_error_does_not_publish_partial_state() {
    let world = Boxes(vec![([0.0; 3], [1.0; 3]); 2]);
    let mut body = state();
    let before = body;
    let result = step_character(
        &world,
        &mut body,
        input(),
        0.1,
        CharacterConfig {
            max_candidates_per_sweep: 1,
            ..CharacterConfig::default()
        },
    );
    assert_eq!(result, Err(CharacterError::Sweep("budget")));
    assert_eq!(body, before);
}

#[test]
fn overflow_does_not_publish_partial_state() {
    let mut body = state();
    body.body.anchor.x = i64::MAX;
    let before = body;
    let result = step_character(
        &Boxes(vec![]),
        &mut body,
        CharacterInput {
            planar_velocity: [20.0, 0.0],
            ..input()
        },
        0.1,
        CharacterConfig::default(),
    );
    assert_eq!(result, Err(CharacterError::CoordinateOverflow));
    assert_eq!(body, before);
}

#[test]
fn invalid_bounds_rejected_even_with_empty_backend() {
    let mut body = state();
    body.body.max = body.body.min;
    let before = body;
    assert_eq!(
        step_character(
            &Boxes(vec![]),
            &mut body,
            input(),
            0.1,
            CharacterConfig::default()
        ),
        Err(CharacterError::InvalidBounds)
    );
    assert_eq!(body, before);
}
