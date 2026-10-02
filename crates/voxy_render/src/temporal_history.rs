//! Retained ping-pong radiance/depth for caller-confirmed presented frames.
use crate::ComputeError;

#[derive(Debug)]
pub struct TemporalHistory {
    device: wgpu::Device,
    color: [wgpu::Texture; 2],
    depth: [wgpu::Texture; 2],
    read: usize,
    valid: bool,
    depth_copy: wgpu::ComputePipeline,
    depth_sampler: wgpu::Sampler,
}
impl TemporalHistory {
    pub(crate) fn validate_device(&self, device: &wgpu::Device) -> Result<(), crate::RaySceneError> {
        if self.device != *device { return Err(crate::RaySceneError::DeviceMismatch); }
        Ok(())
    }

    /// Prepare a resolve against the last caller-confirmed presented frame.
    /// The current NDC depth must separately be encoded into `output_depth`.
    /// `expected_previous_depth` must describe the matching previous surface,
    /// including deformation; current-frame depth is not a substitute.
    /// First frame/reset forces history rejection. Commit only after presentation.
    /// # Errors
    /// Rejects incompatible inputs/outputs or invalid resolve settings.
    pub fn prepare_resolve(
        &self,
        resolver: &crate::TemporalResolve,
        current: &wgpu::Texture,
        motion: &wgpu::Texture,
        expected_previous_depth: &wgpu::Texture,
        mut options: crate::TemporalResolveOptions,
        clip_history: bool,
    ) -> Result<crate::TemporalResolveFrame, ComputeError> {
        options.reset_history |= !self.valid;
        resolver.prepare_into(
            crate::TemporalResolveInputs {
                current,
                motion,
                history: self.color(),
                expected_previous_depth,
                history_depth: self.depth(),
            },
            options,
            self.output(),
            clip_history,
        )
    }
    /// # Errors
    /// Rejects zero dimensions or dimensions exceeding the device limit.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, ComputeError> {
        if width == 0
            || height == 0
            || width > device.limits().max_texture_dimension_2d
            || height > device.limits().max_texture_dimension_2d
        {
            return Err(ComputeError::InvalidDispatch);
        }
        let create = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("retained temporal history"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let limits = device.limits();
        if limits.max_storage_textures_per_shader_stage < 1
            || limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
            || width.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || height.div_ceil(8) > limits.max_compute_workgroups_per_dimension
        {
            return Err(ComputeError::Unsupported);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("temporal depth conversion"),
            source: wgpu::ShaderSource::Wgsl(crate::depth_sample::shader(device,
                r"
@group(0) @binding(0) var depth: texture_depth_2d;
@group(0) @binding(1) var output: texture_storage_2d<r32float,write>;
@group(0) @binding(2) var depth_sampler: sampler;
@compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) p:vec3<u32>) {
    if any(p.xy >= textureDimensions(output)) {return;}
    let uv = (vec2<f32>(p.xy)+0.5)/vec2<f32>(textureDimensions(output));
    textureStore(output,vec2<i32>(p.xy),vec4<f32>(textureSampleLevel(depth, depth_sampler, uv, 0),0.,0.,0.));
}"
            )),
        });
        let depth_copy = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("temporal depth conversion"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Ok(Self {
            depth_sampler: crate::depth_sample::sampler(device),
            depth_copy,
            device: device.clone(),
            color: std::array::from_fn(|_| {
                create(
                    wgpu::TextureFormat::Rgba32Float,
                    wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                )
            }),
            depth: std::array::from_fn(|_| {
                create(
                    wgpu::TextureFormat::R32Float,
                    wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::STORAGE_BINDING,
                )
            }),
            read: 0,
            valid: false,
        })
    }
    #[must_use]
    pub fn valid(&self) -> bool {
        self.valid
    }
    #[must_use]
    pub fn color(&self) -> &wgpu::Texture {
        &self.color[self.read]
    }
    #[must_use]
    pub fn depth(&self) -> &wgpu::Texture {
        &self.depth[self.read]
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.color[1 - self.read]
    }
    /// Depth paired with the pending radiance; becomes readable history on commit.
    #[must_use]
    pub fn output_depth(&self) -> &wgpu::Texture {
        &self.depth[1 - self.read]
    }
    /// Copy the current frame's NDC depth into its pending history slot.
    /// Source/encoder must belong to this device (wgpu validation).
    /// # Errors
    /// Rejects incompatible depth or a copy feedback alias before encoding.
    pub fn encode_depth(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        current: &wgpu::Texture,
    ) -> Result<(), ComputeError> {
        let output = &self.depth[1 - self.read];
        if current.size() != output.size()
            || current.format() != wgpu::TextureFormat::R32Float
            || current.dimension() != wgpu::TextureDimension::D2
            || current.sample_count() != 1
            || !current.usage().contains(wgpu::TextureUsages::COPY_SRC)
            || current == output
        {
            return Err(ComputeError::InvalidBuffer);
        }
        encoder.copy_texture_to_texture(
            current.as_image_copy(),
            output.as_image_copy(),
            output.size(),
        );
        Ok(())
    }
    /// Convert a sampled Depth32Float attachment into pending R32Float history.
    /// Preserves the stored NDC depth, including background; no linearization.
    /// # Errors
    /// Rejects incompatible size, format, samples or missing texture binding.
    pub fn encode_depth_attachment(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        current: &wgpu::Texture,
    ) -> Result<(), ComputeError> {
        if current.size() != self.output_depth().size()
            || current.format() != wgpu::TextureFormat::Depth32Float
            || current.dimension() != wgpu::TextureDimension::D2
            || current.sample_count() != 1
            || !current
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let source = current.create_view(&Default::default());
        let output = self.output_depth().create_view(&Default::default());
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temporal depth source"),
            layout: &self.depth_copy.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&output),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("temporal depth conversion"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.depth_copy);
        pass.set_bind_group(0, &bindings, &[]);
        pass.dispatch_workgroups(current.width().div_ceil(8), current.height().div_ceil(8), 1);
        Ok(())
    }
    /// Call only after pending radiance/depth were submitted and successfully presented.
    /// Skipped/failed presentation must leave this owner unchanged.
    pub fn presented(&mut self) {
        self.read = 1 - self.read;
        self.valid = true;
    }
    /// Camera cuts/material changes invalidate sampling without reallocating textures.
    pub fn reset(&mut self) {
        self.valid = false;
    }
    /// Replace both pairs atomically and invalidate history on a changed size.
    /// # Errors
    /// Invalid dimensions preserve the previous resources and history validity.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<bool, ComputeError> {
        if self.color().width() == width && self.color().height() == height {
            return Ok(false);
        }
        let replacement = Self::new(&self.device, width, height)?;
        *self = replacement;
        Ok(true)
    }
}
