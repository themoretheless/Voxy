//! Single-scattering split-sum GGX BRDF integration lookup.
//! Coordinates are (`NdotV`, perceptual roughness); RG stores A,B for F0*A+B.
use crate::RendererError;
use wgpu::util::DeviceExt;
#[path = "dfg_build.rs"]
mod build;
pub use build::{
    GgxEnergyAtlasBuild, GgxEnergyAtlasSlot, GgxEnergyBuildBudget, GgxEnergyBuildError,
    GgxEnergyBuildPhase, GgxEnergyBuildProgress, GgxEnergyOracleSample, ReadyGgxEnergyAtlas,
};
#[path = "dfg_gpu.rs"]
mod gpu;
pub use gpu::{
    EncodedGgxEnergyBatch, GGX_ENERGY_GPU_BATCH_LIMIT, GgxEnergyBatchKey, GgxEnergyBatchOutput,
    GgxEnergyBatchStage, GgxEnergyGpuPrecompute, PendingGgxEnergyBatch,
};
#[path = "dfg_patch.rs"]
mod patch;
pub use patch::{GgxEnergyPatchData, GgxEnergyPatchError};
#[path = "dfg_refinement.rs"]
mod refinement;
pub use refinement::{
    GgxEnergyRefinementAssessment, GgxEnergyRefinementDecision, GgxEnergyRefinementError,
    GgxEnergyRefinementObservation, GgxEnergyRefinementPlan, GgxEnergyRefinementPolicy,
    GgxEnergyRefinementProbe,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DfgSampling {
    Legacy,
    EnergyGrid,
    EnergyPatches,
}
#[derive(Debug)]
pub struct GgxDfgLut {
    device: wgpu::Device,
    sampling: DfgSampling,
    upload: Option<wgpu::Buffer>,
    output: wgpu::Texture,
    view: wgpu::TextureView,
    passes: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    pipeline: Option<wgpu::RenderPipeline>,
}
impl GgxDfgLut {
    /// Allocate a square `RGBA16Float` LUT. Default integration budget is 256.
    /// Encode once before sampling; no submission or resource allocation in encode.
    /// # Errors
    /// Rejects zero/excessive dimensions.
    pub fn new(device: &wgpu::Device, size: u32) -> Result<Self, RendererError> {
        Self::with_samples(device, size, 256)
    }
    /// Set a fixed quality/work budget of 1..=4096 importance samples per output texel.
    /// Resources and the selected sample count remain immutable across encode calls.
    /// # Errors
    /// Rejects invalid sample counts before allocating, and invalid cube dimensions.
    pub fn with_samples(
        device: &wgpu::Device,
        size: u32,
        samples: u32,
    ) -> Result<Self, RendererError> {
        Self::build(device, size, samples, false)
    }
    /// Allocate RGBA32F A/B/direct-loss/validity moments at the qualified
    /// 524288-resolution budget. Both physical endpoints are stored on each
    /// axis: N.V=(i/(size-1))^2 and roughness=j/(size-1). Minimum size two.
    /// Encode before sampling. This expensive
    /// preintegration is not a per-frame operation. Selected-grid qualification
    /// does not certify arbitrary LUT dimensions or interpolation accuracy.
    pub fn with_energy_moments(device: &wgpu::Device, size: u32) -> Result<Self, RendererError> {
        Self::build(device, size, 524288, true)
    }
    pub(crate) fn has_energy_moments(&self) -> bool {
        self.sampling != DfgSampling::Legacy
    }
    pub(crate) fn sampling(&self) -> DfgSampling {
        self.sampling
    }
    /// Own an immutable sparse experimental patch atlas. Structural validation
    /// is mandatory; scientific whole-domain admission is still the caller's
    /// responsibility. Encode its retained upload before sampling. Uncovered
    /// positive-roughness regions fail closed; this is not the default mode.
    pub fn with_experimental_energy_patches(
        device: &wgpu::Device,
        patches: &[GgxEnergyPatchData],
    ) -> Result<Self, GgxEnergyPatchError> {
        let (height, pixels) =
            patch::pack_atlas(patches, device.limits().max_texture_dimension_2d)?;
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("experimental GGX energy patch atlas"),
            size: wgpu::Extent3d {
                width: 6,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = output.create_view(&Default::default());
        let upload = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("retained GGX atlas upload"),
            contents: bytemuck::cast_slice(&pixels),
            usage: wgpu::BufferUsages::COPY_SRC,
        });
        Ok(Self {
            device: device.clone(),
            sampling: DfgSampling::EnergyPatches,
            upload: Some(upload),
            output,
            view,
            passes: Vec::new(),
            pipeline: None,
        })
    }
    fn build(
        device: &wgpu::Device,
        size: u32,
        samples: u32,
        energy_moments: bool,
    ) -> Result<Self, RendererError> {
        if !energy_moments && !(1..=4096).contains(&samples) {
            return Err(RendererError::InvalidPrefilterSamples(samples));
        }
        let format = if energy_moments {
            wgpu::TextureFormat::Rgba32Float
        } else {
            wgpu::TextureFormat::Rgba16Float
        };
        let max_dimension = device.limits().max_texture_dimension_2d;
        if size == 0 || size > max_dimension || (energy_moments && size < 2) {
            return Err(RendererError::InvalidSurfaceSize {
                width: size,
                height: size,
                max_dimension,
            });
        }
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GGX DFG LUT"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = output.create_view(&Default::default());
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GGX environment layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("DFG parameters"),
            contents: bytemuck::cast_slice(&[size, samples, 0_u32, 0]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("retained DFG integration"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            }],
        });
        let passes = vec![(view.clone(), group)];
        let shader_source = if energy_moments {
            let vertex = include_str!("dfg.wgsl")
                .split("@fragment")
                .next()
                .expect("DFG vertex source");
            format!(
                "{}\n{vertex}\n{}",
                include_str!("material_energy_integrate.wgsl"),
                r"
@fragment fn fs_main(@builtin(position) p:vec4<f32>)->@location(0) vec4<f32>{
 let coordinate=(p.xy-vec2<f32>(0.5))/f32(params.size-1u);
 let m=integrate_ggx_energy(coordinate.x*coordinate.x,coordinate.y,params.samples);
 return vec4<f32>(m.value,f32(m.valid));
}
"
            )
        } else {
            include_str!("dfg.wgsl").to_owned()
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GGX DFG integration"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GGX DFG pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GGX DFG prefilter"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            device: device.clone(),
            sampling: if energy_moments {
                DfgSampling::EnergyGrid
            } else {
                DfgSampling::Legacy
            },
            upload: None,
            output,
            view,
            passes,
            pipeline: Some(pipeline),
        })
    }
    pub(crate) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
    /// Integrate the LUT into its retained target. Encoder ownership is validated by wgpu.
    /// This table uses height-correlated Smith visibility and Schlick Fresnel.
    /// Multiple-scattering compensation and local visibility are separate.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_area(encoder, None);
    }
    /// Encode a bounded rectangle with global texel coordinates unchanged.
    /// Submit and complete small regions separately for expensive preintegration.
    /// Other texels are retained; a new texture remains invalid (alpha zero)
    /// outside completed regions. This does not admit the full table as ready.
    /// # Errors
    /// Rejects empty, overflowing or out-of-bounds rectangles before encoding.
    pub fn encode_region(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        region: [u32; 4],
    ) -> Result<(), RendererError> {
        let [x, y, width, height] = region;
        if width == 0
            || height == 0
            || x.checked_add(width)
                .is_none_or(|end| end > self.output.width())
            || y.checked_add(height)
                .is_none_or(|end| end > self.output.height())
        {
            return Err(RendererError::InvalidSurfaceSize {
                width,
                height,
                max_dimension: self.output.width().min(self.output.height()),
            });
        }
        self.encode_area(encoder, Some(region));
        Ok(())
    }
    fn encode_area(&self, encoder: &mut wgpu::CommandEncoder, region: Option<[u32; 4]>) {
        if let Some(upload) = &self.upload {
            let [x, y, width, height] =
                region.unwrap_or([0, 0, self.output.width(), self.output.height()]);
            encoder.copy_buffer_to_texture(
                wgpu::TexelCopyBufferInfo {
                    buffer: upload,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: u64::from(y) * 256 + u64::from(x) * 16,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.output,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            return;
        }
        for (view, group) in &self.passes {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GGX DFG integration"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: if region.is_some() {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(if self.has_energy_moments() {
                                wgpu::Color::TRANSPARENT
                            } else {
                                wgpu::Color::BLACK
                            })
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(self.pipeline.as_ref().expect("integration pipeline"));
            pass.set_bind_group(0, group, &[]);
            if let Some([x, y, width, height]) = region {
                pass.set_scissor_rect(x, y, width, height);
            }
            pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod energy_tests {
    #[test]
    #[ignore = "requires physical GPU; precise DFG scene and planar binding qualification"]
    fn energy_moment_owner_attaches_without_float32_filtering() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        assert!(
            !device
                .features()
                .contains(wgpu::Features::FLOAT32_FILTERABLE)
        );
        assert!(super::GgxDfgLut::with_energy_moments(&device, 1).is_err());
        let precise = super::GgxDfgLut::with_energy_moments(&device, 4).unwrap();
        let legacy = super::GgxDfgLut::new(&device, 4).unwrap();
        let data = super::GgxEnergyPatchData::try_new(
            [0.2, 0.2, 0.8, 0.8],
            [0., 0.2, 0.4, 0.6, 0.8, 1.],
            [0.05, 0.2, 0.4, 0.6, 0.8, 1.],
            [[0.5, 0.25, 0.25, 1.]; 36],
        )
        .unwrap();
        let atlas = super::GgxDfgLut::with_experimental_energy_patches(&device, &[data]).unwrap();
        let environment = crate::GgxEnvironmentPrefilter::new(&device, 2).unwrap();
        let diffuse = crate::DiffuseEnvironmentConvolution::new(&device, 2).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        atlas.encode(&mut encoder);
        environment.encode(&mut encoder);
        diffuse.encode(&mut encoder);
        assert!(precise.encode_region(&mut encoder, [4, 0, 1, 1]).is_err());
        assert!(precise.encode_region(&mut encoder, [0, 0, 0, 4]).is_err());
        assert!(
            precise
                .encode_region(&mut encoder, [u32::MAX, 0, 2, 1])
                .is_err()
        );
        legacy.encode(&mut encoder);
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        // Follow the documented bounded integration contract: every region
        // has its own completed submission, rather than one large GPU job.
        for region in [[0, 0, 2, 2], [2, 0, 2, 2], [0, 2, 2, 2], [2, 2, 2, 2]] {
            let mut encoder = device.create_command_encoder(&Default::default());
            precise.encode_region(&mut encoder, region).unwrap();
            let submission = queue.submit([encoder.finish()]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
        }
        // The final region must not clear coefficients written by the first.
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("retained DFG region regression"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: precise.output(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        {
            let bytes = staging.slice(..).get_mapped_range().unwrap();
            let m = std::array::from_fn::<_, 4, _>(|c| {
                f32::from_le_bytes(bytes[c * 4..c * 4 + 4].try_into().unwrap())
            });
            assert_eq!(m[3], 1.);
            assert!(m.iter().all(|x| x.is_finite() && *x >= 0. && *x <= 1.));
            assert!((m[0] + m[1] + m[2] - 1.).abs() < 5e-6);
        }
        let mut scene = crate::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let mut capture = crate::PlanarReflectionCapture::new(&device, 4, 4).unwrap();
        for dfg in [&precise, &legacy, &atlas] {
            pollster::block_on(scene.enable_environment_lighting(&device, &environment, dfg))
                .unwrap();
            pollster::block_on(scene.enable_full_environment_lighting(
                &device,
                &environment,
                dfg,
                &diffuse,
            ))
            .unwrap();
            pollster::block_on(capture.enable_environment_lighting(&environment, dfg)).unwrap();
            pollster::block_on(capture.enable_full_environment_lighting(
                &environment,
                dfg,
                &diffuse,
            ))
            .unwrap();
        }
    }
}
