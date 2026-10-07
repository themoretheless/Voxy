//! HDR scene/fog preparation owned by the existing editor graphics session.
use crate::DirectionalLight;
use std::sync::{Arc, mpsc};
use voxy_render::{
    DropletExtinctionFrame, DropletExtinctionPass, ProcessedColorTarget, SceneCamera, SceneDraw,
    SceneRenderer, TextureBlit,
};

#[derive(Debug)]
pub(super) struct FogDraw {
    renderer: SceneRenderer,
    transport: DropletExtinctionPass,
    display: TextureBlit,
    targets: std::collections::BTreeMap<u8, Arc<Targets>>,
    overlay_depth: Option<Arc<(wgpu::Texture, wgpu::TextureView)>>,
    frames: Vec<DropletExtinctionFrame>,
    submitted: Vec<SubmittedFog>,
    shadow: Option<Arc<voxy_render::ShadowMap>>,
    shadow_transport: Option<DropletExtinctionPass>,
    shadow_draw_bytes: u64,
}
#[derive(Debug)]
struct SubmittedFog {
    frames: Vec<DropletExtinctionFrame>,
    targets: std::collections::BTreeMap<u8, Arc<Targets>>,
    overlay_depth: Option<Arc<(wgpu::Texture, wgpu::TextureView)>>,
    completed: mpsc::Receiver<()>,
    shadow: Option<Arc<voxy_render::ShadowMap>>,
    shadow_draw_bytes: u64,
}
#[derive(Debug)]
pub(super) struct OpaqueCaster<'a> {
    pub geometry: &'a voxy_render::SceneGeometry,
    pub world: glam::Mat4,
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
            submitted: Vec::new(),
            shadow: None,
            shadow_transport: None,
            shadow_draw_bytes: 0,
        })
    }
    pub fn allocation_bytes(&self) -> u64 {
        // Shared sources are charged once even when several ordered queue
        // submissions reference them. Replaced sources stay charged until done.
        let mut sources = std::collections::HashSet::new();
        let mut overlays = std::collections::HashSet::new();
        let mut bytes = 0;
        for targets in
            std::iter::once(&self.targets).chain(self.submitted.iter().map(|s| &s.targets))
        {
            for t in targets.values() {
                if sources.insert(Arc::as_ptr(t)) {
                    bytes += u64::from(t.depth.width()) * u64::from(t.depth.height()) * 16;
                }
            }
        }
        for overlay in std::iter::once(&self.overlay_depth)
            .chain(self.submitted.iter().map(|s| &s.overlay_depth))
            .flatten()
        {
            if overlays.insert(Arc::as_ptr(overlay)) {
                bytes += u64::from(overlay.0.width()) * u64::from(overlay.0.height()) * 4;
            }
        }
        let mut maps = std::collections::HashSet::new();
        for map in std::iter::once(&self.shadow)
            .chain(self.submitted.iter().map(|s| &s.shadow))
            .flatten()
        {
            if maps.insert(Arc::as_ptr(map)) {
                bytes += u64::from(map.texture().width()) * u64::from(map.texture().height()) * 4;
            }
        }
        bytes += self.shadow_draw_bytes
            + self
                .submitted
                .iter()
                .map(|s| s.shadow_draw_bytes)
                .sum::<u64>();
        if self.shadow_transport.is_some() {
            bytes += 80;
        }
        bytes
            + self
                .frames
                .iter()
                .chain(self.submitted.iter().flat_map(|s| &s.frames))
                .map(DropletExtinctionFrame::allocation_bytes)
                .sum::<u64>()
    }
    pub fn retire(&mut self, device: &wgpu::Device) -> Result<(), wgpu::PollError> {
        device.poll(wgpu::PollType::Poll)?;
        self.collect_completed();
        Ok(())
    }
    fn collect_completed(&mut self) {
        // Disconnected notification is not evidence of GPU completion.
        self.submitted.retain(|s| s.completed.try_recv().is_err());
    }
    /// Call only after submission of the encoder containing `encode` succeeded.
    pub fn submitted(&mut self, queue: &wgpu::Queue) {
        if self.frames.is_empty() {
            return;
        }
        let (tx, completed) = mpsc::channel();
        self.submitted.push(SubmittedFog {
            frames: std::mem::take(&mut self.frames),
            targets: self.targets.clone(),
            overlay_depth: self.overlay_depth.clone(),
            completed,
            shadow: self.shadow.clone(),
            shadow_draw_bytes: std::mem::take(&mut self.shadow_draw_bytes),
        });
        queue.on_submitted_work_done(move || {
            let _ = tx.send(());
        });
    }
    /// The caller must discard the failed encoder before releasing its frames.
    pub fn discard_unsubmitted(&mut self) {
        self.frames.clear();
        self.shadow_draw_bytes = 0;
    }
    pub fn deactivate(&mut self, device: &wgpu::Device) -> Result<bool, wgpu::PollError> {
        self.discard_unsubmitted();
        self.targets.clear();
        self.overlay_depth = None;
        self.shadow = None;
        self.shadow_transport = None;
        self.retire(device)?;
        Ok(self.submitted.is_empty())
    }
    pub fn prepare_targets(
        &mut self,
        device: &wgpu::Device,
        window: [u32; 2],
        views: &[(u8, [u32; 4], glam::Mat4, Option<SceneCamera>)],
        max_bytes: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.prepare_targets_with_shadow(device, window, views, None, 0, max_bytes)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_targets_with_shadow(
        &mut self,
        device: &wgpu::Device,
        window: [u32; 2],
        views: &[(u8, [u32; 4], glam::Mat4, Option<SceneCamera>)],
        shadow: Option<(crate::DirectionalShadow, voxy_render::ShadowSettings)>,
        caster_draws: usize,
        max_bytes: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.retire(device)?;
        if !self.frames.is_empty() {
            return Err("fog has an unsubmitted encoder".into());
        }
        // Admission includes all pending submissions, new frame storage and
        // replacement sources, before any allocation or source mutation.
        let mut required = self.allocation_bytes();
        if self
            .overlay_depth
            .as_ref()
            .is_none_or(|t| [t.0.width(), t.0.height()] != window)
        {
            required = required
                .checked_add(u64::from(window[0]) * u64::from(window[1]) * 4)
                .ok_or("fog byte count overflow")?;
        }
        let mut ids = std::collections::HashSet::new();
        for (id, region, _, camera) in views {
            if !ids.insert(*id) {
                return Err("duplicate fog view identity".into());
            }
            let pixels = u64::from(region[2]) * u64::from(region[3]);
            let storage = pixels
                .checked_mul(48)
                .and_then(|n| n.checked_add(176))
                .ok_or("fog byte count overflow")?;
            if storage > u64::from(device.limits().max_storage_buffer_binding_size) {
                return Err("fog view exceeds GPU storage binding capacity".into());
            }
            if camera.is_none() {
                return Err("fog requires a three-dimensional camera".into());
            }
            let replaced = self
                .targets
                .get(id)
                .is_none_or(|t| [t.depth.width(), t.depth.height()] != region[2..]);
            let new_bytes = pixels
                .checked_mul(if replaced { 72 } else { 56 })
                .and_then(|n| n.checked_add(176))
                .ok_or("fog byte count overflow")?;
            required = required
                .checked_add(new_bytes)
                .ok_or("fog byte count overflow")?;
        }
        if let Some((config, _)) = shadow {
            config.validate()?;
            if config.resolution > device.limits().max_texture_dimension_2d {
                return Err("shadow resolution exceeds GPU texture capacity".into());
            }
            let map_changed = self
                .shadow
                .as_ref()
                .is_none_or(|map| map.texture().width() != config.resolution);
            let map_bytes = if map_changed {
                u64::from(config.resolution) * u64::from(config.resolution) * 4
            } else {
                0
            };
            let draws = u64::try_from(caster_draws)?
                .checked_mul(64)
                .ok_or("shadow draw byte count overflow")?;
            required = required
                .checked_add(map_bytes)
                .and_then(|n| n.checked_add(draws))
                .and_then(|n| n.checked_add(80 * views.len() as u64))
                .and_then(|n| {
                    n.checked_add(if self.shadow_transport.is_none() {
                        80
                    } else {
                        0
                    })
                })
                .ok_or("shadow byte count overflow")?;
        }
        if required > max_bytes {
            return Err("fog presentation pending submissions exceed memory budget".into());
        }
        if let Some((config, settings)) = shadow {
            let map = if let Some(map) = &self.shadow
                && map.texture().width() == config.resolution
            {
                map.clone()
            } else {
                Arc::new(voxy_render::ShadowMap::new(
                    device,
                    config.resolution,
                    config.resolution,
                )?)
            };
            if let Some(transport) = &mut self.shadow_transport {
                transport.update_directional_shadow(&map, settings)?;
            } else {
                self.shadow_transport = Some(pollster::block_on(
                    DropletExtinctionPass::with_directional_shadow(device, &map, settings),
                )?);
            }
            self.shadow = Some(map);
        } else {
            self.shadow_transport = None;
            self.shadow = None;
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
                    Arc::new(Targets {
                        color,
                        depth: d,
                        depth_view,
                        xray,
                    }),
                );
            }
        }
        if self
            .overlay_depth
            .as_ref()
            .is_none_or(|t| [t.0.width(), t.0.height()] != window)
        {
            let d = depth(window[0], window[1]);
            let view = d.create_view(&Default::default());
            self.overlay_depth = Some(Arc::new((d, view)));
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
        casters: &[Vec<OpaqueCaster<'_>>],
        shadow_settings: Option<voxy_render::ShadowSettings>,
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
        for (view_index, ((id, region, _, camera), draws)) in views.iter().zip(draws).enumerate() {
            if let Some(map) = &self.shadow {
                let settings =
                    shadow_settings.ok_or_else(|| err("missing shadow projection".into()))?;
                let casters = casters
                    .get(view_index)
                    .ok_or_else(|| err("missing shadow caster view".into()))?;
                let shadow_draws: Vec<_> = casters
                    .iter()
                    .map(|caster| {
                        map.prepare(caster.geometry, settings.light_from_world * caster.world)
                    })
                    .collect::<Result<_, _>>()
                    .map_err(voxy_render::RendererError::Scene)?;
                map.encode(encoder, &shadow_draws)
                    .map_err(voxy_render::RendererError::Scene)?;
                self.shadow_draw_bytes += shadow_draws.len() as u64 * 64;
            }
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
            let transport = self.shadow_transport.as_ref().unwrap_or(&self.transport);
            let frame = pollster::block_on(transport.prepare(
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
            &[],
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
        let frame_bytes = fog
            .frames
            .iter()
            .map(DropletExtinctionFrame::allocation_bytes)
            .sum::<u64>();
        let old_source = Arc::downgrade(&fog.targets[&views[0].0]);
        fog.submitted(&queue);
        // Delay observation deterministically: fast GPUs may finish immediately,
        // but unobserved completion must keep memory charged across resize.
        let (completion_tx, completion_rx) = mpsc::channel();
        let real_completion = std::mem::replace(&mut fog.submitted[0].completed, completion_rx);
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
        real_completion.recv().unwrap();
        let retained = fog.allocation_bytes();
        // A late encoding error releases only its unsubmitted frames, after
        // discarding the encoder; earlier submitted storage remains charged.
        let mut bad_views = views.clone();
        bad_views[1].3 = None;
        let mut failed_encoder = device.create_command_encoder(&Default::default());
        assert!(
            fog.encode(
                &device,
                &mut failed_encoder,
                &target.create_view(&Default::default()),
                &ordinary,
                &bad_views,
                &draws,
                &overlays,
                volume,
                volume.world_bounds(glam::Mat4::IDENTITY).unwrap(),
                None,
                &[],
                None,
            )
            .is_err()
        );
        assert_eq!(fog.frames.len(), 1);
        assert!(fog.allocation_bytes() > retained);
        drop(failed_encoder);
        fog.discard_unsubmitted();
        assert_eq!(fog.allocation_bytes(), retained);
        assert_eq!(fog.submitted.len(), 1);
        assert_eq!(retained, 64 * 32 * 16 + 64 * 40 * 4 + frame_bytes);
        let mut resized = [views[0]];
        resized[0].1[2] = 31;
        let before_targets = Arc::as_ptr(&fog.targets[&views[0].0]);
        assert!(
            fog.prepare_targets(&device, [65, 40], &resized, retained)
                .is_err()
        );
        assert_eq!(fog.allocation_bytes(), retained);
        assert_eq!(Arc::as_ptr(&fog.targets[&views[0].0]), before_targets);
        fog.prepare_targets(&device, [65, 40], &resized, 1_000_000)
            .unwrap();
        // Both replaced and removed sources remain alive with the old UI depth.
        assert_eq!(
            fog.allocation_bytes(),
            retained + 65 * 40 * 4 + 31 * 32 * 16
        );
        assert!(!fog.deactivate(&device).unwrap());
        assert_eq!(fog.allocation_bytes(), retained);
        assert!(old_source.upgrade().is_some());
        completion_tx.send(()).unwrap();
        fog.retire(&device).unwrap();
        assert_eq!(fog.allocation_bytes(), 0);
        assert!(old_source.upgrade().is_none());
        // A lost callback cannot release an unproven submission.
        let (lost_tx, lost_rx) = mpsc::channel();
        drop(lost_tx);
        fog.submitted.push(SubmittedFog {
            frames: vec![],
            targets: Default::default(),
            overlay_depth: None,
            completed: lost_rx,
            shadow: None,
            shadow_draw_bytes: 0,
        });
        fog.collect_completed();
        assert_eq!(fog.submitted.len(), 1);
        fog.submitted.clear();
        assert!(pollster::block_on(scope.pop()).is_none());
        println!(
            "EDITOR FOG VIEWS PASS retirement_nonblocking=true failed_encoder_preserves_submitted=true pending_resize_accounted=true disabled_sources_retained=true completion_releases=true pixels=2560 views=2 local_overlay_pixels=128 global_ui_pixels=512 fogged_view_matches_slab=true missed_view_unattenuated=true"
        );
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;
    #[test]
    #[ignore = "requires a physical GPU adapter"]
    fn opaque_shadow_views_use_current_geometry_and_preserve_pending_maps() {
        verify_caster_views(false);
    }
    #[test]
    #[ignore = "requires a physical GPU adapter"]
    fn skeletal_shadow_views_consume_independently_deformed_gpu_streams() {
        verify_caster_views(true);
    }
    fn verify_caster_views(skinned: bool) {
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        println!("EDITOR SHADOW GPU {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut ordinary = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        pollster::block_on(ordinary.reload_shader(&device, include_str!("material.wgsl"))).unwrap();
        let mesh = |z, xmax| {
            voxy_render::SceneMesh::new(
                [[0., 0., z], [xmax, 0., z], [xmax, 1., z], [0., 1., z]]
                    .into_iter()
                    .map(|position| voxy_render::SceneVertex {
                        position,
                        uv: [0.; 2],
                        color: [0., 0., 0., 1.],
                    })
                    .collect(),
                vec![0, 2, 1, 0, 3, 2],
            )
            .unwrap()
        };
        let back = ordinary.upload_mesh(&device, &mesh(2., 1.)).unwrap();
        let half = ordinary.upload_mesh(&device, &mesh(1.25, 0.5)).unwrap();
        let skinner = skinned.then(|| voxy_render::SceneSkinner::new(&ordinary).unwrap());
        let skin_source = skinner.as_ref().map(|skinner| {
            let vertices = [
                [0., 0., 1.25],
                [0.5, 0., 1.25],
                [0.5, 1., 1.25],
                [0., 1., 1.25],
            ]
            .into_iter()
            .map(|position| voxy_render::SkinnedVertex {
                position,
                normal: [0.; 3],
                uv: [0.; 2],
                joints: [u16::from(position[1] > 0.), 0, 0, 0],
                weights: [65535, 0, 0, 0],
            })
            .collect();
            skinner
                .upload_source(
                    Arc::new(
                        voxy_render::SkinnedMesh::new(vertices, vec![0, 2, 1, 0, 3, 2], 2).unwrap(),
                    ),
                    0,
                    100_000,
                )
                .unwrap()
        });
        let skin_instances: Vec<_> = skin_source
            .iter()
            .flat_map(|source| {
                (0..2).map(|_| {
                    skinner
                        .as_ref()
                        .unwrap()
                        .create_instance(
                            &ordinary,
                            source.clone(),
                            &[glam::Mat4::IDENTITY; 2],
                            [0., 0., 0., 1.],
                            0,
                            100_000,
                        )
                        .unwrap()
                })
            })
            .collect();
        let caster_geometries: Vec<_> = if skinned {
            skin_instances.iter().map(|skin| skin.geometry()).collect()
        } else {
            vec![&half, &half]
        };
        let texture = ordinary
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
                far: 4.,
            },
        };
        let vp = camera.view_projection().unwrap();
        let worlds = [
            glam::Mat4::IDENTITY,
            glam::Mat4::from_translation(glam::Vec3::new(0.5, 0., 0.)),
        ];
        let background = ordinary.create_transform(&device, vp).unwrap();
        let transforms: Vec<_> = worlds
            .iter()
            .map(|world| ordinary.create_transform(&device, vp * world).unwrap())
            .collect();
        for (transform, world) in transforms.iter().zip(worlds) {
            transform
                .update_scene_material(&queue, world, [1.; 4], [0., 0., 1., 0.])
                .unwrap();
        }
        let draws: Vec<_> = transforms
            .iter()
            .enumerate()
            .map(|(index, transform)| {
                vec![
                    SceneDraw {
                        geometry: &back,
                        texture: &texture,
                        transform: &background,
                        overlay: false,
                    },
                    SceneDraw {
                        geometry: caster_geometries[index],
                        texture: &texture,
                        transform,
                        overlay: false,
                    },
                ]
            })
            .collect();
        let casters: Vec<_> = worlds
            .iter()
            .enumerate()
            .map(|(index, world)| {
                vec![OpaqueCaster {
                    geometry: caster_geometries[index],
                    world: *world,
                }]
            })
            .collect();
        let views = [
            (0, [0, 0, 16, 16], vp, Some(camera)),
            (1, [16, 0, 16, 16], vp, Some(camera)),
        ];
        let volume = voxy_scene::FogVolume::default();
        let light = DirectionalLight {
            direction: [0., 0., 1.],
            intensity: 4.,
        };
        let config = crate::DirectionalShadow {
            center: [0.5; 3],
            extent: [1., 1., 3.],
            resolution: 64,
            bias: 0.,
            filter: crate::DirectionalShadowFilter::Hard,
            ..Default::default()
        };
        let settings = config.settings(light).unwrap();
        let mut fog =
            pollster::block_on(FogDraw::new(&device, wgpu::TextureFormat::Rgba8Unorm)).unwrap();
        fog.prepare_targets_with_shadow(
            &device,
            [32, 16],
            &views,
            Some((config, settings)),
            2,
            100_000,
        )
        .unwrap();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("editor shadow proof"),
            size: wgpu::Extent3d {
                width: 32,
                height: 16,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        if let Some(skinner) = &skinner {
            for (index, instance) in skin_instances.iter().enumerate() {
                let shift = if index == 0 { 0.3 } else { -0.3 };
                skinner
                    .encode_pose(
                        &queue,
                        &mut encoder,
                        instance,
                        &[
                            glam::Mat4::IDENTITY,
                            glam::Mat4::from_translation(glam::Vec3::new(shift, 0., 0.)),
                        ],
                    )
                    .unwrap();
            }
        }
        fog.encode(
            &device,
            &mut encoder,
            &target.create_view(&Default::default()),
            &ordinary,
            &views,
            &draws,
            &[],
            volume,
            volume.world_bounds(glam::Mat4::IDENTITY).unwrap(),
            Some(light),
            &casters,
            Some(settings),
        )
        .unwrap();
        let frame_bytes = fog
            .frames
            .iter()
            .map(DropletExtinctionFrame::allocation_bytes)
            .sum::<u64>();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("editor shadow pixels"),
            size: 256 * 16,
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
                    rows_per_image: Some(16),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        fog.submitted(&queue);
        let (gate_tx, gate_rx) = mpsc::channel();
        let real_completion = std::mem::replace(&mut fog.submitted[0].completed, gate_rx);
        let (tx, rx) = mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        real_completion.recv().unwrap();
        let bytes = staging.slice(..).get_mapped_range().unwrap();
        let radiance = 4. * 0.8 * (-0.5_f32).exp() / (8. * std::f32::consts::PI);
        let lit = (255. * radiance / (1. + radiance)).round() as u8;
        let mut checked = 0;
        for y in 0..16 {
            for x in 0..32 {
                let blocked = if skinned {
                    let world_x = 1. - (x % 16) as f32 / 16. - 0.5 / 16.;
                    let world_y = 1. - y as f32 / 16. - 0.5 / 16.;
                    let lower = if x < 16 {
                        0.3 * world_y
                    } else {
                        0.5 - 0.3 * world_y
                    };
                    let upper = lower + 0.5;
                    // Hard raster sampling uses texel centres; exclude the
                    // narrow boundary strip rather than loosen RGB tolerance.
                    if (world_x - lower).abs() < 2. / 64. || (world_x - upper).abs() < 2. / 64. {
                        continue;
                    }
                    world_x > lower && world_x < upper
                } else {
                    if x < 16 { x >= 8 } else { x < 24 }
                };
                let expected = if blocked { 0 } else { lit };
                let offset = y * 256 + x * 4;
                assert!(
                    bytes[offset..offset + 3]
                        .iter()
                        .all(|value| value.abs_diff(expected) <= 1),
                    "x={x} y={y} actual={:?} expected={expected}",
                    &bytes[offset..offset + 4]
                );
                assert_eq!(bytes[offset + 3], 255);
                checked += 1;
            }
        }
        drop(bytes);
        staging.unmap();
        assert!(checked >= 400);
        let retained = fog.allocation_bytes();
        assert_eq!(
            retained,
            2 * 16 * 16 * 16 + 32 * 16 * 4 + 64 * 64 * 4 + 80 + 2 * 64 + frame_bytes
        );
        let old_map = Arc::downgrade(fog.shadow.as_ref().unwrap());
        let resized = crate::DirectionalShadow {
            resolution: 32,
            ..config
        };
        assert!(
            fog.prepare_targets_with_shadow(
                &device,
                [32, 16],
                &views,
                Some((resized, settings)),
                2,
                retained
            )
            .is_err()
        );
        assert_eq!(fog.shadow.as_ref().unwrap().texture().width(), 64);
        assert_eq!(fog.allocation_bytes(), retained);
        fog.prepare_targets_with_shadow(
            &device,
            [32, 16],
            &views,
            Some((resized, settings)),
            2,
            100_000,
        )
        .unwrap();
        assert_eq!(fog.allocation_bytes(), retained + 32 * 32 * 4);
        assert!(!fog.deactivate(&device).unwrap());
        assert!(old_map.upgrade().is_some());
        gate_tx.send(()).unwrap();
        fog.retire(&device).unwrap();
        assert_eq!(fog.allocation_bytes(), 0);
        assert!(old_map.upgrade().is_none());
        assert!(pollster::block_on(scope.pop()).is_none());
        println!(
            "EDITOR FOG SHADOW PASS pixels={checked} skinned={skinned} views=2 opposite_caster_poses=true per_view_shadow_order=true map_resize_retained=true budget_rejection_preserves_map=true completion_releases_maps=true"
        );
    }
}
