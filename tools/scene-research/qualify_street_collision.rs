//! Standalone regressions exercising the actual engine collision modules.
//! rustc --edition=2024 --test tools/scene-research/qualify_street_collision.rs -o /tmp/voxy-street-collision-tests
#[path = "../../crates/physics/src/static_triangle_world.rs"]
mod static_triangle_world;
#[path = "../../crates/physics/src/triangle_index.rs"]
mod triangle_index;
#[cfg(test)]
mod street_regressions {
    use super::static_triangle_world::StaticTriangleWorld;
    fn floor() -> StaticTriangleWorld {
        StaticTriangleWorld::new(vec![
            [[-10., 0., -10.], [10., 0., -10.], [10., 0., 10.]],
            [[-10., 0., -10.], [10., 0., 10.], [-10., 0., 10.]],
        ])
        .unwrap()
    }
    #[test]
    fn recover_embedded_spawn_then_fall_and_walk() {
        let world = floor();
        let recovered = world
            .recover_capsule([0., -0.1, 0.], 1.8, 0.3, [0., 1., 0.])
            .unwrap();
        assert!(recovered[1] >= 0.);
        let (ground, grounded) = world
            .move_capsule(recovered, 1.8, 0.3, [0., -1., 0.])
            .unwrap();
        assert!(grounded);
        let (walk, grounded) = world
            .move_capsule(ground, 1.8, 0.3, [5., -0.01, 0.])
            .unwrap();
        assert!(grounded);
        assert!((walk[0] - 5.).abs() < 1e-6);
        assert!(walk[1] >= -1e-6);
    }
    #[test]
    fn upward_jump_separates_from_floor() {
        let world = floor();
        let (p, grounded) = world
            .move_capsule([0., 0., 0.], 1.8, 0.3, [0., 1., 0.])
            .unwrap();
        assert!(!grounded);
        assert!((p[1] - 1.).abs() < 1e-6);
    }
}
