//! Temporal AA for immutable polygon meshes, including camera reprojection and sky.
use crate::{ComputeError, ComputeMemoryBudget};
use glam::{Mat4, Vec2, Vec3};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ArchiveAntialiasing {
    Off,
    #[default]
    Spatial,
    Temporal,
    /// Native MetalFX temporal AA at the scene's full render resolution.
    MetalFx,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Frame {
    inverse_vp: [[f32; 4]; 4],
    previous_vp: [[f32; 4]; 4],
    current_vp: [[f32; 4]; 4],
    eye: [f32; 4],
    jitter: [f32; 4],
    settings: [f32; 4],
    // aperture diameter m, focus distance m, focal length m, motion blur shutter fraction
    lens: [f32; 4],
    // sensor width m, max circle of confusion px, unused, unused
    sensor: [f32; 4],
}
/// Physical thin-lens camera for depth of field and shutter motion blur.
/// All zero disables both effects.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArchiveCameraLens {
    pub focal_length_m: f32,
    pub f_number: f32,
    pub focus_distance_m: f32,
    /// Fraction of the frame interval the shutter is open (0.5 = 180 degrees).
    pub shutter_fraction: f32,
    pub sensor_width_m: f32,
}
#[derive(Debug)]
pub(crate) struct ArchiveTemporal {
    pub current: Arc<voxy_gpu_core::ManagedTexture>,
    color: [Arc<voxy_gpu_core::ManagedTexture>; 2],
    depths: [Arc<voxy_gpu_core::ManagedTexture>; 2],
    motion: Arc<voxy_gpu_core::ManagedTexture>,
    expected: Arc<voxy_gpu_core::ManagedTexture>,
    uniform: crate::ComputeStorage,
    guides: wgpu::RenderPipeline,
    resolve: wgpu::ComputePipeline,
    display: wgpu::RenderPipeline,
    display_linear: wgpu::RenderPipeline,
    linear_output: bool,
    previous: Mat4,
    previous_eye: Vec3,
    previous_direction: Vec3,
    previous_jitter: Vec2,
    read: usize,
    frames: u32,
    reset: bool,
    native: bool,
    pub lens: ArchiveCameraLens,
    bloom: crate::bloom::Bloom,
    #[cfg(target_os = "macos")]
    metalfx: Option<crate::metalfx::MetalFxTemporal>,
}
fn texture(
    device: &wgpu::Device,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
) -> Result<Arc<voxy_gpu_core::ManagedTexture>, ComputeError> {
    ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
        label: Some("polygon temporal AA"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | if format == wgpu::TextureFormat::Rg16Float {
                wgpu::TextureUsages::empty()
            } else {
                wgpu::TextureUsages::STORAGE_BINDING
            },
        view_formats: &[],
    })
}
impl ArchiveTemporal {
    pub fn new(
        device: &wgpu::Device,
        w: u32,
        h: u32,
        format: wgpu::TextureFormat,
        mode: ArchiveAntialiasing,
    ) -> Result<Self, ComputeError> {
        #[cfg(target_os = "macos")]
        let metalfx = if mode == ArchiveAntialiasing::MetalFx {
            Some(crate::metalfx::MetalFxTemporal::new(device, w, h)?)
        } else {
            None
        };
        #[cfg(not(target_os = "macos"))]
        if mode == ArchiveAntialiasing::MetalFx {
            return Err(ComputeError::Unsupported);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("polygon TAA"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("display_transform.wgsl"),
                    include_str!("archive_temporal.wgsl")
                )
                .into(),
            ),
        });
        let render = |entry: &str, targets: &[Option<wgpu::ColorTargetState>]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets,
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let target = |format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        };
        let guides = render(
            "fs_guides",
            &[
                target(wgpu::TextureFormat::Rg16Float),
                target(wgpu::TextureFormat::R32Float),
            ],
        );
        let display = render("fs_display", &[target(format)]);
        let display_linear = render("fs_display", &[target(wgpu::TextureFormat::Rgba16Float)]);
        let resolve = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("polygon TAA resolve"),
            layout: None,
            module: &shader,
            entry_point: Some("cs_resolve"),
            compilation_options: Default::default(),
            cache: None,
        });
        let [uniform] = ComputeMemoryBudget::for_device(device).allocate_buffers([
            voxy_gpu_core::ManagedBufferDescriptor {
                label: "polygon TAA matrices",
                size: std::mem::size_of::<Frame>() as u64,
                contents: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        ])?;
        Ok(Self {
            current: texture(device, w, h, wgpu::TextureFormat::Rgba16Float)?,
            color: [
                texture(device, w, h, wgpu::TextureFormat::Rgba16Float)?,
                texture(device, w, h, wgpu::TextureFormat::Rgba16Float)?,
            ],
            depths: [
                texture(device, w, h, wgpu::TextureFormat::R32Float)?,
                texture(device, w, h, wgpu::TextureFormat::R32Float)?,
            ],
            motion: texture(device, w, h, wgpu::TextureFormat::Rg16Float)?,
            expected: texture(device, w, h, wgpu::TextureFormat::R32Float)?,
            uniform,
            guides,
            resolve,
            display,
            display_linear,
            linear_output: false,
            previous: Mat4::IDENTITY,
            previous_eye: Vec3::ZERO,
            previous_direction: Vec3::NEG_Z,
            previous_jitter: Vec2::ZERO,
            read: 0,
            frames: 0,
            reset: true,
            native: mode == ArchiveAntialiasing::MetalFx,
            lens: ArchiveCameraLens::default(),
            bloom: crate::bloom::Bloom::new(device, w, h)?,
            #[cfg(target_os = "macos")]
            metalfx,
        })
    }
    pub fn motion_texture(&self) -> &wgpu::Texture {
        &self.motion
    }
    pub fn reset(&mut self) {
        self.reset = true;
        self.frames = 0;
    }
    pub fn jitter(&self) -> Vec2 {
        halton_jitter(self.frames)
    }
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        vp: Mat4,
        jittered: Mat4,
        eye: Vec3,
        direction: Vec3,
        display_mode: f32,
        headroom: f32,
    ) {
        self.linear_output = display_mode > 2.5;
        self.reset |= eye.distance(self.previous_eye) > 2.
            || direction.normalize().dot(self.previous_direction) < 0.5;
        let static_camera = !self.reset && vp == self.previous;
        let jitter =
            self.jitter() / Vec2::new(self.current.width() as f32, self.current.height() as f32);
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&Frame {
                inverse_vp: jittered.inverse().to_cols_array_2d(),
                previous_vp: if self.reset { vp } else { self.previous }.to_cols_array_2d(),
                current_vp: vp.to_cols_array_2d(),
                eye: [eye.x, eye.y, eye.z, f32::from(static_camera)],
                jitter: [
                    jitter.x,
                    jitter.y,
                    self.previous_jitter.x,
                    self.previous_jitter.y,
                ],
                settings: [
                    f32::from(self.reset),
                    if self.native { 0. } else { 0.9 },
                    display_mode,
                    headroom,
                ],
                lens: [
                    if self.lens.f_number > 0. {
                        self.lens.focal_length_m / self.lens.f_number
                    } else {
                        0.
                    },
                    self.lens.focus_distance_m,
                    self.lens.focal_length_m,
                    self.lens.shutter_fraction,
                ],
                sensor: [self.lens.sensor_width_m, 24., 0., 0.],
            }),
        );
        self.previous = vp;
        self.previous_eye = eye;
        self.previous_direction = direction.normalize();
        self.previous_jitter = jitter;
    }
    pub fn encode_guides(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::Texture,
    ) {
        let depth_view = depth.create_view(&Default::default());
        let group =
            |layout: &wgpu::BindGroupLayout, views: &[&wgpu::TextureView], uniform: bool| {
                let mut entries: Vec<_> = views
                    .iter()
                    .enumerate()
                    .map(|(i, v)| wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: wgpu::BindingResource::TextureView(v),
                    })
                    .collect();
                if uniform {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.uniform.as_entire_binding(),
                    });
                }
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("polygon temporal bindings"),
                    layout,
                    entries: &entries,
                })
            };
        let motion = self.motion.create_view(&Default::default());
        let expected = self.expected.create_view(&Default::default());
        let guide_bind = group(&self.guides.get_bind_group_layout(0), &[&depth_view], true);
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("polygon camera motion and depth"),
                color_attachments: &[attachment(&motion), attachment(&expected)],
                ..Default::default()
            });
            pass.set_pipeline(&self.guides);
            pass.set_bind_group(0, &guide_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        let write = 1 - self.read;
        let views = [
            &*self.current,
            &*self.color[self.read],
            &*self.depths[self.read],
            &*self.motion,
            &*self.expected,
            &*self.color[write],
            &*self.depths[write],
        ]
        .map(|t| t.create_view(&Default::default()));
        let resolve_bind = group(
            &self.resolve.get_bind_group_layout(1),
            &views.iter().collect::<Vec<_>>(),
            false,
        );
        let frame_bind = group(&self.resolve.get_bind_group_layout(0), &[&depth_view], true);
        // This pass also registers every native MetalFX resource with wgpu's
        // lifetime/usage tracking before borrowing the underlying command buffer.
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("polygon temporal resolve"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.resolve);
            pass.set_bind_group(0, &frame_bind, &[]);
            pass.set_bind_group(1, &resolve_bind, &[]);
            pass.dispatch_workgroups(
                self.current.width().div_ceil(8),
                self.current.height().div_ceil(8),
                1,
            );
        }
    }
    pub fn native(&self) -> bool {
        self.native
    }
    #[cfg(target_os = "macos")]
    pub fn encode_native(&mut self, encoder: &mut wgpu::CommandEncoder, depth: &wgpu::Texture) {
        let jitter = self.jitter();
        if let Some(metalfx) = &mut self.metalfx {
            metalfx.encode(
                encoder,
                &self.current,
                depth,
                &self.motion,
                &self.color[1 - self.read],
                jitter,
                self.reset,
            );
        }
    }
    pub fn encode_display(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let pipeline = if self.linear_output {
            &self.display_linear
        } else {
            &self.display
        };
        let write = 1 - self.read;
        let view = self.color[write].create_view(&Default::default());
        let expected = self.expected.create_view(&Default::default());
        let motion = self.motion.create_view(&Default::default());
        self.bloom.encode(device, encoder, &view);
        let bloom = self.bloom.output();
        let display_frame = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("polygon temporal display settings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 1,
                resource: self.uniform.as_entire_binding(),
            }],
        });
        let display_color = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("polygon temporal display color"),
            layout: &pipeline.get_bind_group_layout(2),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&expected),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&motion),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&bloom),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(self.bloom.sampler()),
                },
            ],
        });
        let empty = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("polygon temporal HDR display"),
                timestamp_writes,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &display_frame, &[]);
            pass.set_bind_group(1, &empty, &[]);
            pass.set_bind_group(2, &display_color, &[]);
            pass.draw(0..3, 0..1);
        }
        self.read = write;
        self.reset = false;
        self.frames = self.frames.wrapping_add(1);
    }
}
pub fn halton_jitter(frame: u32) -> Vec2 {
    fn halton(mut i: u32, base: u32) -> f32 {
        let mut f = 1.;
        let mut r = 0.;
        while i > 0 {
            f /= base as f32;
            r += f * (i % base) as f32;
            i /= base;
        }
        r
    }
    let i = frame % 8 + 1;
    Vec2::new(halton(i, 2) - 0.5, halton(i, 3) - 0.5)
}
