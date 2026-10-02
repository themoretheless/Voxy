//! Uses the actual render cameras to select objects from viewport coordinates.
use glam::{Vec2, Vec3};
use voxy_render::{SceneCamera, SceneProjection};
use voxy_scene::{PickBounds, PickRay, PickViewport, SceneGraph, Transform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = SceneGraph::new(1);
    let cube = scene.spawn(None, Transform::default())?;
    scene.insert_component(
        cube,
        PickBounds {
            min: -Vec3::ONE,
            max: Vec3::ONE,
            layers: 1,
        },
    )?;
    let projections = [
        SceneProjection::Perspective {
            vertical_fov: std::f32::consts::FRAC_PI_2,
            aspect: 1.0,
            near: 1.0,
            far: 20.0,
        },
        SceneProjection::Orthographic {
            left: -5.0,
            right: 5.0,
            bottom: -5.0,
            top: 5.0,
            near: 1.0,
            far: 20.0,
        },
    ];
    for projection in projections {
        let vp = SceneCamera {
            eye: Vec3::Z * 10.0,
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection,
        }
        .view_projection()?;
        let ray = PickRay::from_viewport(vp, Vec2::splat(400.0), Vec2::splat(800.0))?;
        let viewport = PickViewport {
            origin: Vec2::new(240.0, 80.0),
            size: Vec2::splat(800.0),
        };
        let physical_cursor = (viewport.origin + viewport.size * 0.5) * 2.0;
        let mapped = viewport.ray(vp, physical_cursor, 2.0)?.unwrap();
        assert_eq!(scene.pick(mapped, 1)?.hit.unwrap().node, cube);
        assert!(viewport.ray(vp, Vec2::ZERO, 2.0)?.is_none());
        let hit = scene.pick(ray, 1)?.hit.unwrap();
        assert_eq!(hit.node, cube);
        assert!((hit.distance - 8.0).abs() < 1e-4);
        let outside = PickRay::from_viewport(vp, Vec2::new(0.0, 0.0), Vec2::splat(800.0))?;
        assert!(scene.pick(outside, 1)?.hit.is_none());
    }
    println!("CAMERA PICKING PASS: perspective/orthographic center hits and corner misses");
    Ok(())
}
