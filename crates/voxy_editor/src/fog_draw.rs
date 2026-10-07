//! HDR scene/fog preparation owned by the existing editor graphics session.
use crate::DirectionalLight;
use voxy_render::{
    DropletExtinctionFrame, DropletExtinctionPass, ProcessedColorTarget, SceneCamera, SceneDraw,
    SceneRenderer, TextureBlit,
};

#[derive(Debug)]
pub(super) struct FogDraw {
    renderer: SceneRenderer,
    transport: DropletExtinctionPass,
    display: TextureBlit,
    targets: std::collections::BTreeMap<u8, Targets>,
    overlay_depth: Option<(wgpu::Texture, wgpu::TextureView)>,
    frames: Vec<DropletExtinctionFrame>,
}
#[derive(Debug)]
struct Targets {
    color: ProcessedColorTarget,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    xray: wgpu::TextureView,
}
impl FogDraw {
    pub async fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut renderer = SceneRenderer::new(device, wgpu::TextureFormat::Rgba16Float);
        renderer
            .reload_shader(device, include_str!("material.wgsl"))
            .await?;
        Ok(Self {
            renderer,
            transport: DropletExtinctionPass::new(device).await?,
            display: TextureBlit::tone_mapped(device, format, 1.)
                .ok_or("invalid fog display exposure")?,
            targets: Default::default(),
            overlay_depth: None,
            frames: Vec::new(),
        })
    }
    pub fn allocation_bytes(&self) -> u64 {
        self.targets
            .values()
            .map(|t| u64::from(t.depth.width()) * u64::from(t.depth.height()) * 16)
            .sum::<u64>()
            + self
                .overlay_depth
                .as_ref()
                .map_or(0, |(t, _)| u64::from(t.width()) * u64::from(t.height()) * 4)
            + self
                .frames
                .iter()
                .map(DropletExtinctionFrame::allocation_bytes)
                .sum::<u64>()
    }
    pub fn retire(&mut self, device: &wgpu::Device) -> Result<(), wgpu::PollError> {
        device.poll(wgpu::PollType::wait_indefinitely())?;
        self.frames.clear();
        Ok(())
    }
    pub fn prepare_targets(
        &mut self,
        device: &wgpu::Device,
        window: [u32; 2],
        views: &[(u8, [u32; 4], glam::Mat4, Option<SceneCamera>)],
        max_bytes: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // The single-sample path keeps source HDR, ordinary/X-ray depths and
        // bounded optical storage/output separate. No UI enters transport.
        let required = views.iter().try_fold(
            u64::from(window[0]) * u64::from(window[1]) * 4,
            |sum, (_, region, _, camera)| {
                if 48 * u64::from(region[2]) * u64::from(region[3]) + 176
                    > u64::from(device.limits().max_storage_buffer_binding_size)
                {
                    return Err("fog view exceeds GPU storage binding capacity");
                }
                if camera.is_none() {
                    return Err("fog requires a three-dimensional camera");
                }
                sum.checked_add(u64::from(region[2]) * u64::from(region[3]) * 72 + 176)
                    .ok_or("fog byte count overflow")
            },
        )?;
        if required > max_bytes {
            return Err("fog presentation memory budget exceeded".into());
        }
        self.retire(device)?;
        let old_sources = self.allocation_bytes();
        if old_sources
            .checked_add(required)
            .is_none_or(|bytes| bytes > max_bytes)
        {
            return Err("fog resize peak memory budget exceeded".into());
        }
        let depth = |width, height| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("editor fog scene depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        self.targets
            .retain(|id, _| views.iter().any(|v| v.0 == *id));
        for (id, region, _, _) in views {
            if self
                .targets
                .get(id)
                .is_none_or(|t| [t.depth.width(), t.depth.height()] != region[2..])
            {
                let d = depth(region[2], region[3]);
                let depth_view = d.create_view(&Default::default());
                let xray = depth(region[2], region[3]).create_view(&Default::default());
                let color = ProcessedColorTarget::new(device, region[2], region[3], true)?;
                self.targets.insert(
                    *id,
                    Targets {
                        color,
                        depth: d,
                        depth_view,
                        xray,
                    },
                );
            }
        }
        if self
            .overlay_depth
            .as_ref()
            .is_none_or(|(t, _)| [t.width(), t.height()] != window)
        {
            let d = depth(window[0], window[1]);
            let view = d.create_view(&Default::default());
            self.overlay_depth = Some((d, view));
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        ordinary: &SceneRenderer,
        views: &[(u8, [u32; 4], glam::Mat4, Option<SceneCamera>)],
        draws: &[Vec<SceneDraw<'_>>],
        overlays: &[SceneDraw<'_>],
        volume: voxy_scene::FogVolume,
        bounds: voxy_scene::FogBounds,
        light: Option<DirectionalLight>,
    ) -> Result<(), voxy_render::RendererError> {
        let err = |e: String| voxy_render::RendererError::TemporalConsumer(e);
        let depth = &self
            .overlay_depth
            .as_ref()
            .ok_or_else(|| err("missing fog UI target".into()))?
            .1;
        {
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("editor fog window clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
        }
        for ((id, region, _, camera), draws) in views.iter().zip(draws) {
            let targets = &self.targets[id];
            let world: Vec<_> = draws
                .iter()
                .filter(|d| !d.overlay)
                .map(|d| SceneDraw {
                    geometry: d.geometry,
                    texture: d.texture,
                    transform: d.transform,
                    overlay: false,
                })
                .collect();
            self.renderer.encode_with_xray_depth(
                encoder,
                targets.color.view(),
                &targets.depth_view,
                &targets.xray,
                wgpu::Color::BLACK,
                &world,
            );
            let mut input = voxy_render::DropletExtinctionSceneInput::new(
                voxy_render::ExtinctionGridView {
                    origin: bounds.origin,
                    spacing: bounds.extent,
                    shape: [1; 3],
                    extinction_m_inverse: &[volume.extinction_m_inverse],
                },
                camera.ok_or_else(|| err("fog camera unavailable".into()))?,
                region[2],
                region[3],
                device.limits().max_storage_buffer_binding_size as usize,
            )
            .map_err(|e| err(format!("fog scene input: {e}")))?;
            if let Some(light) = light {
                input = input
                    .with_directional_scattering(
                        voxy_render::DirectionalScatteringOptions {
                            direction_to_light: light.direction,
                            irradiance_rgb: [light.intensity; 3],
                            albedo: volume.single_scattering_albedo,
                            asymmetry: volume.asymmetry,
                            samples: volume.samples,
                        },
                        device.limits().max_storage_buffer_binding_size as usize,
                        1_000_000_000,
                    )
                    .map_err(|e| err(e.to_string()))?;
            }
            let frame = pollster::block_on(self.transport.prepare(
                &input,
                targets.color.texture(),
                &targets.depth,
                64 * 1024 * 1024,
            ))
            .map_err(|e| err(e.to_string()))?;
            pollster::block_on(frame.encode(encoder)).map_err(|e| err(e.to_string()))?;
            self.display
                .encode_viewport(
                    device,
                    encoder,
                    &frame.output().create_view(&Default::default()),
                    output,
                    *region,
                )
                .map_err(voxy_render::RendererError::Scene)?;
            ordinary.encode_overlays_viewport(encoder, output, depth, draws, *region);
            self.frames.push(frame);
        }
        ordinary.encode_overlays(encoder, output, depth, overlays);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a physical GPU adapter"]
    fn hdr_fog_views_preserve_local_and_global_editor_overlays() {
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        println!("EDITOR FOG GPU {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut ordinary = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        pollster::block_on(ordinary.reload_shader(&device, include_str!("material.wgsl"))).unwrap();
        let mesh = |points: [[f32; 3]; 4], color| {
            voxy_render::SceneMesh::new(
                points
                    .into_iter()
                    .map(|position| voxy_render::SceneVertex {
                        position,
                        uv: [0.; 2],
                        color,
                    })
                    .collect(),
                vec![0, 2, 1, 0, 3, 2],
            )
            .unwrap()
        };
        let world = ordinary
            .upload_mesh(
                &device,
                &mesh(
                    [[0., 0., 2.], [2., 0., 2.], [2., 1., 2.], [0., 1., 2.]],
                    [1.; 4],
                ),
            )
            .unwrap();
        let local = ordinary
            .upload_mesh(
                &device,
                &mesh(
                    [
                        [-1., -1., 0.],
                        [-0.5, -1., 0.],
                        [-0.5, -0.5, 0.],
                        [-1., -0.5, 0.],
                    ],
                    [0., 0., 1., 1.],
                ),
            )
            .unwrap();
        let panel = ordinary
            .upload_mesh(
                &device,
                &mesh(
                    [
                        [-1., -1., 0.],
                        [1., -1., 0.],
                        [1., -0.6, 0.],
                        [-1., -0.6, 0.],
                    ],
                    [1., 0., 0., 1.],
                ),
            )
            .unwrap();
        let white = ordinary
            .upload_texture(&device, &queue, 1, 1, &[255; 4])
            .unwrap();
        let camera = SceneCamera {
            eye: glam::Vec3::new(0.5, 0.5, -1.),
            target: glam::Vec3::new(0.5, 0.5, 0.),
            up: glam::Vec3::Y,
            projection: voxy_render::SceneProjection::Orthographic {
                left: -0.5,
                right: 0.5,
                bottom: -0.5,
                top: 0.5,
                near: 0.,
                far: 3.,
            },
        };
        let other = SceneCamera {
            eye: camera.eye + glam::Vec3::X,
            target: camera.target + glam::Vec3::X,
            ..camera
        };
        let left = ordinary
            .create_transform(&device, camera.view_projection().unwrap())
            .unwrap();
        let right = ordinary
            .create_transform(&device, other.view_projection().unwrap())
            .unwrap();
        let ui = ordinary
            .create_transform(&device, glam::Mat4::IDENTITY)
            .unwrap();
        let views = vec![
            (
                0,
                [0, 0, 32, 32],
                camera.view_projection().unwrap(),
                Some(camera),
            ),
            (
                1,
                [32, 0, 32, 32],
                other.view_projection().unwrap(),
                Some(other),
            ),
        ];
        let draws = vec![
            vec![
                SceneDraw {
                    geometry: &world,
                    texture: &white,
                    transform: &left,
                    overlay: false,
                },
                SceneDraw {
                    geometry: &local,
                    texture: &white,
                    transform: &ui,
                    overlay: true,
                },
            ],
            vec![
                SceneDraw {
                    geometry: &world,
                    texture: &white,
                    transform: &right,
                    overlay: false,
                },
                SceneDraw {
                    geometry: &local,
                    texture: &white,
                    transform: &ui,
                    overlay: true,
                },
            ],
        ];
        let overlays = [SceneDraw {
            geometry: &panel,
            texture: &white,
            transform: &ui,
            overlay: true,
        }];
        let mut fog =
            pollster::block_on(FogDraw::new(&device, wgpu::TextureFormat::Rgba8Unorm)).unwrap();
        fog.prepare_targets(&device, [64, 40], &views, 1_000_000)
            .unwrap();
        assert!(fog.prepare_targets(&device, [64, 40], &views, 0).is_err());
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("editor fog views proof"),
            size: wgpu::Extent3d {
                width: 64,
                height: 40,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let volume = voxy_scene::FogVolume::default();
        let mut encoder = device.create_command_encoder(&Default::default());
        fog.encode(
            &device,
            &mut encoder,
            &target.create_view(&Default::default()),
            &ordinary,
            &views,
            &draws,
            &overlays,
            volume,
            volume.world_bounds(glam::Mat4::IDENTITY).unwrap(),
            None,
        )
        .unwrap();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("editor fog readback"),
            size: 256 * 40,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(40),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = staging.slice(..).get_mapped_range().unwrap();
        let transmission = (-0.5_f32).exp();
        let fogged = (255. * transmission / (1. + transmission)).round() as u8;
        for y in 0..40 {
            for x in 0..64 {
                let expected = if y >= 32 {
                    [255, 0, 0, 255]
                } else if y >= 24 && x % 32 < 8 {
                    [0, 0, 255, 255]
                } else {
                    let value = if x < 32 { fogged } else { 128 };
                    [value, value, value, 255]
                };
                let offset = y * 256 + x * 4;
                let actual = &bytes[offset..offset + 4];
                assert!(
                    actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                    "editor fog x={x} y={y} actual={actual:?} expected={expected:?}"
                );
            }
        }
        drop(bytes);
        staging.unmap();
        assert!(pollster::block_on(scope.pop()).is_none());
        println!(
            "EDITOR FOG VIEWS PASS pixels=2560 views=2 local_overlay_pixels=128 global_ui_pixels=512 fogged_view_matches_slab=true missed_view_unattenuated=true"
        );
    }
}
