//! Camera and per-object affine backward motion from GPU world-space surfaces.
use crate::{PrimarySurfaceJob, RaySceneError};
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub struct PrimaryMotionPass {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
    previous_depth: wgpu::Texture,
}
#[derive(Debug, Clone, Copy)]
struct MotionObjects<'a> {
    transforms: &'a [[f32; 16]],
    ids: &'a wgpu::Texture,
}
impl PrimaryMotionPass {
    /// Generate RG16 backward UV motion from current/previous unjittered cameras.
    /// Primary world positions may be reconstructed using the jittered camera.
    /// Both input matrices must project the same world coordinate system.
    /// This pass does not handle object animation or deformation. History reset,
    /// background, behind-camera or unrepresentable motion samples write zero.
    /// Use SDK motion scale [1,1] for this normalized top-left UV convention.
    /// # Errors
    /// Rejects nonfinite/singular matrices or insufficient fragment storage limits.
    pub fn new(
        device: &wgpu::Device,
        primary: &PrimarySurfaceJob,
        current: glam::Mat4,
        previous: glam::Mat4,
        reset_history: bool,
    ) -> Result<Self, RaySceneError> {
        Self::create(
            device,
            primary,
            [current, previous],
            reset_history,
            None,
            None,
            [0.0; 3],
        )
    }
    /// Reproject independently moving affine objects selected by an `R32Uint` ID map.
    /// IDs index `models`; unknown IDs write zero. IDs must match the primary opaque
    /// surfaces and world positions already transformed by their current model.
    /// Reprojection uses `previous_model * inverse(current_model)`, including
    /// rotation and nonuniform scale. Mesh deformation requires additional
    /// previous-position data and is not represented by these affine pairs.
    /// # Errors
    /// Rejects invalid models, ID map format/size/usage and storage capacity.
    pub fn for_objects(
        device: &wgpu::Device,
        primary: &PrimarySurfaceJob,
        cameras: [glam::Mat4; 2],
        models: &[[glam::Mat4; 2]],
        ids: &wgpu::Texture,
        reset_history: bool,
    ) -> Result<Self, RaySceneError> {
        if models.is_empty()
            || [ids.width(), ids.height()] != primary.dimensions()
            || ids.format() != wgpu::TextureFormat::R32Uint
            || ids.sample_count() != 1
            || ids.dimension() != wgpu::TextureDimension::D2
            || ids.depth_or_array_layers() != 1
            || !ids.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let bytes = u64::try_from(models.len()).map_err(|_| RaySceneError::Capacity)? * 64;
        if bytes > device.limits().max_buffer_size
            || bytes > device.limits().max_storage_buffer_binding_size
        {
            return Err(RaySceneError::Capacity);
        }
        let transforms = models
            .iter()
            .map(|pair| previous_world(*pair).map(|matrix| matrix.to_cols_array()))
            .collect::<Result<Vec<_>, _>>()?;
        Self::create(
            device,
            primary,
            cameras,
            reset_history,
            Some(MotionObjects {
                transforms: &transforms,
                ids,
            }),
            None,
            [0.0; 3],
        )
    }
    /// Use previous world positions corresponding to each current primary pixel.
    /// The map is RGBA16/32Float: XYZ is the previous position, W is exactly 1
    /// for valid correspondence. Other W values produce zero motion. This is
    /// current-frame correspondence data, not the previous frame's position image.
    /// The caller must produce it from matching previous skinned/deformed geometry.
    /// # Errors
    /// Rejects incompatible textures and invalid camera matrices or device limits.
    pub fn for_previous_positions(
        device: &wgpu::Device,
        primary: &PrimarySurfaceJob,
        cameras: [glam::Mat4; 2],
        positions: &wgpu::Texture,
        reset_history: bool,
    ) -> Result<Self, RaySceneError> {
        Self::for_previous_positions_relative(
            device,
            primary,
            cameras,
            positions,
            [0.0; 3],
            reset_history,
        )
    }
    /// Decode previous-position correspondence relative to a finite world origin.
    /// Supply the producer's `position_origin()`; XYZ values are offsets, W remains validity.
    /// # Errors
    /// Rejects invalid origin, texture, camera matrices or device capacity.
    pub fn for_previous_positions_relative(
        device: &wgpu::Device,
        primary: &PrimarySurfaceJob,
        cameras: [glam::Mat4; 2],
        positions: &wgpu::Texture,
        origin: [f32; 3],
        reset_history: bool,
    ) -> Result<Self, RaySceneError> {
        if !glam::Vec3::from_array(origin).is_finite() {
            return Err(RaySceneError::InvalidGeometry);
        }
        if [positions.width(), positions.height()] != primary.dimensions()
            || !matches!(
                positions.format(),
                wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
            )
            || positions.dimension() != wgpu::TextureDimension::D2
            || positions.depth_or_array_layers() != 1
            || positions.sample_count() != 1
            || !positions
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        Self::create(
            device,
            primary,
            cameras,
            reset_history,
            None,
            Some(positions),
            origin,
        )
    }
    fn create(
        device: &wgpu::Device,
        primary: &PrimarySurfaceJob,
        cameras: [glam::Mat4; 2],
        reset_history: bool,
        objects: Option<MotionObjects<'_>>,
        previous_positions: Option<&wgpu::Texture>,
        position_origin: [f32; 3],
    ) -> Result<Self, RaySceneError> {
        primary.validate_device(device)?;
        let [current, previous] = cameras;
        for matrix in [current, previous] {
            let determinant = matrix.determinant();
            if !matrix.is_finite() || !determinant.is_finite() || determinant == 0.0 {
                return Err(RaySceneError::InvalidGeometry);
            }
        }
        if device.limits().max_storage_buffers_per_shader_stage < 2
            || device.limits().max_sampled_textures_per_shader_stage < 2
            || device.limits().max_color_attachments < 2
            || device.limits().max_color_attachment_bytes_per_sample < 8
        {
            return Err(RaySceneError::Capacity);
        }
        let dimensions = primary.dimensions();
        let mut data = Vec::new();
        data.extend_from_slice(bytemuck::cast_slice(&current.to_cols_array()));
        data.extend_from_slice(bytemuck::cast_slice(&previous.to_cols_array()));
        data.extend_from_slice(bytemuck::cast_slice(&[
            dimensions[0],
            dimensions[1],
            u32::from(reset_history),
            if previous_positions.is_some() {
                2
            } else {
                u32::from(objects.is_some())
            },
        ]));
        data.extend_from_slice(bytemuck::cast_slice(&[
            position_origin[0],
            position_origin[1],
            position_origin[2],
            0.0,
        ]));
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("primary motion cameras"),
            contents: &data,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("static primary RG16 UV motion"),
            size: wgpu::Extent3d {
                width: dimensions[0],
                height: dimensions[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let previous_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("expected previous surface depth"),
            size: output.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: previous_depth_format(device),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let pipeline = motion_pipeline(device, output.format());
        let (models, ids) = object_resources(device, objects);
        let positions = previous_position_view(device, previous_positions);
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("primary static motion inputs"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&positions),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: primary.output().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: models.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&ids),
                },
            ],
        });
        Ok(Self {
            pipeline,
            bindings,
            output,
            previous_depth,
        })
    }
    /// Include a single object's affine motion for all primary samples in this buffer.
    /// World positions must already contain the current model transform. Reprojects
    /// them through `previous_model * inverse(current_model)` before the previous camera.
    /// For multiple independently moving objects, provide separate surface batches;
    /// this does not represent skeletal deformation or changing topology.
    /// # Errors
    /// Rejects invalid/non-affine/singular models and preserves `new` validation.
    pub fn for_object(
        device: &wgpu::Device,
        primary: &PrimarySurfaceJob,
        cameras: [glam::Mat4; 2],
        models: [glam::Mat4; 2],
        reset_history: bool,
    ) -> Result<Self, RaySceneError> {
        Self::new(
            device,
            primary,
            cameras[0],
            cameras[1] * previous_world(models)?,
            reset_history,
        )
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let view = self
            .output
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.previous_depth.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("primary static motion"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &depth_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.draw(0..3, 0..1);
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
    /// Previous-camera NDC depth at current primary coverage, including supplied
    /// affine/deformation correspondence. Invalid/reset/background writes zero.
    /// R32Float normally; OpenGL uses renderable R16Float with half precision.
    #[must_use]
    pub fn previous_depth(&self) -> &wgpu::Texture {
        &self.previous_depth
    }
}

fn previous_world(models: [glam::Mat4; 2]) -> Result<glam::Mat4, RaySceneError> {
    for model in models {
        let determinant = model.determinant();
        if !model.is_finite()
            || !determinant.is_finite()
            || determinant == 0.0
            || model.row(3) != glam::Vec4::W
        {
            return Err(RaySceneError::InvalidGeometry);
        }
    }
    let transform = models[1] * models[0].inverse();
    if !transform.is_finite() {
        return Err(RaySceneError::InvalidGeometry);
    }
    Ok(transform)
}

fn object_resources(
    device: &wgpu::Device,
    objects: Option<MotionObjects<'_>>,
) -> (wgpu::Buffer, wgpu::TextureView) {
    let identity = [glam::Mat4::IDENTITY.to_cols_array()];
    let models = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("previous world transforms"),
        contents: bytemuck::cast_slice(objects.as_ref().map_or(&identity[..], |o| o.transforms)),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let fallback = objects.is_none().then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("unused motion IDs"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    });
    let ids = objects
        .as_ref()
        .map(|o| o.ids)
        .or(fallback.as_ref())
        .expect("ID map or fallback")
        .create_view(&wgpu::TextureViewDescriptor::default());
    (models, ids)
}

fn previous_position_view(
    device: &wgpu::Device,
    previous_positions: Option<&wgpu::Texture>,
) -> wgpu::TextureView {
    let fallback_positions = previous_positions.is_none().then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("unused previous position map"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    });
    previous_positions
        .or(fallback_positions.as_ref())
        .expect("position map or fallback")
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn motion_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("primary static camera motion"),
        source: wgpu::ShaderSource::Wgsl(include_str!("primary_motion.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("primary static camera motion"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[
                Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: previous_depth_format(device),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn previous_depth_format(device: &wgpu::Device) -> wgpu::TextureFormat {
    if device.adapter_info().backend == wgpu::Backend::Gl {
        wgpu::TextureFormat::R16Float
    } else {
        wgpu::TextureFormat::R32Float
    }
}
