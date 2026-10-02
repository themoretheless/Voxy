//! Production sprite extraction shares logical layout with pointer hit routing.
use glam::Vec2;
use voxy_render::{Sprite, SpriteBatch};
use voxy_ui::{Axis, LayoutItem, Length, PointerAction, PointerRouter, WidgetId, layout_linear};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = [
        LayoutItem {
            id: WidgetId(1),
            length: Length::Fixed(80.0),
            enabled: true,
        },
        LayoutItem {
            id: WidgetId(2),
            length: Length::Flex(1.0),
            enabled: true,
        },
    ];
    let mut pointer = PointerRouter::new(2);
    for width in [320.0, 640.0] {
        let viewport = Vec2::new(width, 100.0);
        let regions = layout_linear(
            [0.0; 2],
            viewport.to_array(),
            Axis::Horizontal,
            8.0,
            8.0,
            &items,
            2,
        )?;
        pointer.set_regions(&regions)?;
        let mut batch = SpriteBatch::new(2);
        for region in &regions {
            batch.push(Sprite::from_logical_rect(
                Vec2::from_array(region.origin),
                Vec2::from_array(region.size),
                viewport,
                [0.2, 0.5, 0.8, 1.0],
            )?)?;
        }
        let mesh = batch.mesh()?;
        assert_eq!(mesh.indices().len(), 12);
        for (region, vertices) in regions.iter().zip(mesh.vertices().chunks_exact(4)) {
            let xmin = vertices
                .iter()
                .map(|v| v.position[0])
                .fold(f32::INFINITY, f32::min);
            let ymax = vertices
                .iter()
                .map(|v| v.position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            let pixel_origin = [(xmin + 1.0) * width * 0.5, (1.0 - ymax) * 100.0 * 0.5];
            assert!((pixel_origin[0] - region.origin[0]).abs() < 1e-4);
            assert!((pixel_origin[1] - region.origin[1]).abs() < 1e-4);
            pointer.move_to(Some([region.origin[0] + region.size[0] * 0.5, 50.0]));
            pointer.press();
            assert_eq!(
                pointer.release().action,
                PointerAction::Release {
                    id: region.id,
                    clicked: true
                }
            );
        }
    }
    let full =
        Sprite::from_logical_rect(Vec2::ZERO, Vec2::splat(100.0), Vec2::splat(100.0), [1.0; 4])?;
    let crop = full.cropped(Vec2::new(0.25, 0.5), Vec2::new(0.75, 1.0))?;
    assert!(crop.size.abs_diff_eq(Vec2::ONE, 1e-6));
    assert!(crop.center.abs_diff_eq(Vec2::new(0.0, -0.5), 1e-6));
    assert!(crop.uv_min.abs_diff_eq(Vec2::new(0.25, 0.5), 1e-6));
    assert!(crop.uv_max.abs_diff_eq(Vec2::new(0.75, 1.0), 1e-6));
    assert!(full.cropped(Vec2::ZERO, Vec2::ZERO).is_err());
    assert!(Sprite::from_logical_rect(Vec2::ZERO, Vec2::ONE, Vec2::ZERO, [1.0; 4]).is_err());
    println!(
        "UI GEOMETRY PASS: shared layout maps to actual sprite vertices and pointer targets after resize"
    );
    Ok(())
}
