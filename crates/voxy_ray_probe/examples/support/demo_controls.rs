//! Three icon controls rendered after tone mapping through the common scene API.
use glam::{Mat4, Vec2};
use voxy_render::*;
use voxy_ui::{HitRegion, WidgetId};

pub struct DemoControls {
    renderer: SceneRenderer,
    geometry: SceneGeometry,
    texture: SceneTexture,
    transform: SceneTransform,
    pub regions: Vec<HitRegion>,
}
impl DemoControls {
    pub fn new(host: &SceneSurface) -> Result<Self, Box<dyn std::error::Error>> {
        let renderer = host.create_scene_renderer();
        Ok(Self {
            geometry: renderer.reserve_geometry(host.device(), 64, 96)?,
            texture: renderer.upload_texture(host.device(), host.queue(), 1, 1, &[255; 4])?,
            transform: renderer.create_transform(host.device(), Mat4::IDENTITY)?,
            renderer,
            regions: Vec::new(),
        })
    }
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        viewport: Vec2,
        paused: bool,
        temporal: bool,
        temporal_available: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.regions.clear();
        let mut batch = SpriteBatch::new(16);
        for i in 0..3 {
            let origin = Vec2::new(12.0 + i as f32 * 56.0, 12.0);
            let region = HitRegion {
                id: WidgetId(i + 1),
                origin: origin.to_array(),
                size: [48.0; 2],
                enabled: i != 2 || temporal_available,
            };
            let Some(clipped) = region.clipped([0.0; 2], viewport.to_array())? else {
                continue;
            };
            self.regions.push(clipped);
            let active = (i == 0 && paused) || (i == 2 && temporal && temporal_available);
            batch.push(Sprite::from_logical_rect(
                origin,
                Vec2::splat(48.0),
                viewport,
                if active {
                    [0.04, 0.4, 0.6, 0.95]
                } else {
                    [0.04, 0.06, 0.1, 0.95]
                },
            )?)?;
            let icons: &[([f32; 2], [f32; 2])] = match i {
                // Pause; reset cross; three temporal samples.
                0 => &[([15.0, 13.0], [6.0, 22.0]), ([27.0, 13.0], [6.0, 22.0])],
                1 => &[([12.0, 21.0], [24.0, 6.0]), ([21.0, 12.0], [6.0, 24.0])],
                _ => &[
                    ([12.0, 12.0], [24.0, 4.0]),
                    ([12.0, 22.0], [24.0, 4.0]),
                    ([12.0, 32.0], [24.0, 4.0]),
                ],
            };
            for &(offset, size) in icons {
                batch.push(Sprite::from_logical_rect(
                    origin + Vec2::from_array(offset),
                    Vec2::from_array(size),
                    viewport,
                    [1.0; 4],
                )?)?;
            }
        }
        if batch.is_empty() {
            self.geometry.clear();
        } else {
            self.geometry.update(queue, &batch.mesh()?)?;
        }
        Ok(())
    }
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) {
        self.renderer.encode_overlays(
            encoder,
            target,
            depth,
            &[SceneDraw {
                geometry: &self.geometry,
                texture: &self.texture,
                transform: &self.transform,
                overlay: true,
            }],
        );
    }
}
