//! Native instanced vegetation layered into the resident world's reverse-Z depth.
use crate::archive_scene::{
    ArchiveImageSource, ArchiveInstance, ArchiveMaterialSource, ArchiveMeshSource,
};
use crate::{ComputeError, ComputeMemoryBudget, ComputeStorage};
use glam::camera::rh::{proj::directx::orthographic, view::look_at_mat4};
use glam::{Mat4, Vec3};
use std::sync::Arc;
mod culling;
mod shadow_wind;
use culling::{ClipVolume, PlantBounds, wind_bounds};
use shadow_wind::{ShadowWind, WindBuffer};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Camera {
    vp: [[f32; 4]; 4],
    inverse_vp: [[f32; 4]; 4],
    shadow: [[f32; 4]; 4],
    eye: [f32; 4],
    sun: [f32; 4],
    flags: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PlantMaterial {
    base: crate::archive_scene::ArchiveMaterial,
    canopy: [f32; 4],
    extent_density: [f32; 4],
}
fn canopy_metadata(geometry: &[ArchiveMeshSource], material: usize) -> ([f32; 4], [f32; 4]) {
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    let mut area = 0.;
    for mesh in geometry.iter().filter(|m| m.material == material) {
        for v in &mesh.vertices {
            let p = Vec3::from_array(v.position);
            low = low.min(p);
            high = high.max(p);
        }
        for ids in mesh.indices.chunks_exact(3) {
            let p = |i: u32| Vec3::from_array(mesh.vertices[i as usize].position);
            area += (p(ids[1]) - p(ids[0]))
                .cross(p(ids[2]) - p(ids[0]))
                .length()
                * 0.5;
        }
    }
    if !low.is_finite() {
        return ([0.; 4], [1., 1., 1., 0.]);
    }
    let centre = (low + high) * 0.5;
    let extent = ((high - low) * 0.5).max(Vec3::splat(0.01));
    let volume = 4. / 3. * std::f32::consts::PI * extent.x * extent.y * extent.z;
    (
        centre.extend(0.).to_array(),
        extent.extend(area / volume).to_array(),
    )
}
struct Mesh {
    vertices: ComputeStorage,
    indices: ComputeStorage,
    instances: ComputeStorage,
    instance_ring: Option<crate::SharedUploadRing>,
    snapshot: Vec<ArchiveInstance>,
    generated: Option<crate::SharedUploadSnapshot>,
    local_bounds: Option<(glam::DVec3, glam::DVec3)>,
    instance_bounds: Vec<Option<PlantBounds>>,
    shadow_wind: Option<WindBuffer>,
    count: u32,
    instance_count: u32,
    capacity: usize,
    material: usize,
}
struct Material {
    alpha_cutoff: f32,
    bind: wgpu::BindGroup,
    _uniform: ComputeStorage,
}
/// Geometry and PBR images retain native GPU ownership; terrain and actors keep their own owners.
pub struct NativeVegetation {
    /// Clock for wind animation in the vertex shader.
    started: std::time::Instant,
    #[cfg(test)]
    frozen_benchmark_time: Option<f32>,
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    _images: Vec<Arc<voxy_gpu_core::ManagedTexture>>,
    camera: ComputeStorage,
    camera_bind: wgpu::BindGroup,
    camera_ring: Option<crate::SharedUploadRing>,
    camera_groups: Vec<wgpu::BindGroup>,
    shadow: Arc<voxy_gpu_core::ManagedTexture>,
    shadow_bind: wgpu::BindGroup,
    receiver_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
    hdr_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    opaque_shadow_pipeline: wgpu::RenderPipeline,
    opaque_shadow_fast: bool,
    shadow_wind: Option<(ShadowWind, wgpu::RenderPipeline)>,
    shadow_wind_enabled: bool,
    receiver: wgpu::RenderPipeline,
    hdr_receiver: wgpu::RenderPipeline,
    shadow_vp: Mat4,
    cull_instances: bool,
    cull_shadow_receivers: bool,
    shadow_capture_override: Option<(bool, f32)>,
}
#[derive(Debug)]
pub struct NativeVegetationWindow {
    pub instances: Vec<Vec<ArchiveInstance>>,
    pub bounds: (Vec3, Vec3),
}

struct PreparedPlantInstances {
    buffer: ComputeStorage,
    ring: Option<crate::SharedUploadRing>,
    values: Vec<ArchiveInstance>,
    bounds: Vec<Option<PlantBounds>>,
    wind: Option<WindBuffer>,
}

pub(crate) struct PreparedVegetationWindow {
    meshes: Vec<PreparedPlantInstances>,
    shadow_vp: Mat4,
}

fn vegetation_shadow_matrix(bounds: (Vec3, Vec3)) -> Result<Mat4, ComputeError> {
    if !bounds.0.is_finite() || !bounds.1.is_finite() || !bounds.0.cmple(bounds.1).all() {
        return Err(ComputeError::Validation("invalid vegetation bounds".into()));
    }
    let centre = (bounds.0 + bounds.1) * 0.5;
    let extent = (bounds.1 - bounds.0).length().max(1.);
    let sun = Vec3::new(0.42, 0.82, 0.38).normalize();
    let matrix = orthographic(
        -extent * 0.5,
        extent * 0.5,
        -extent * 0.5,
        extent * 0.5,
        0.1,
        extent * 3.,
    ) * look_at_mat4(centre + sun * extent, centre, Vec3::Y);
    if !matrix.is_finite() {
        return Err(ComputeError::Validation(
            "invalid vegetation shadow matrix".into(),
        ));
    }
    Ok(matrix)
}

impl std::fmt::Debug for NativeVegetation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeVegetation")
            .field("meshes", &self.meshes.len())
            .finish_non_exhaustive()
    }
}
fn buffer(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> Result<ComputeStorage, ComputeError> {
    let [b] = ComputeMemoryBudget::for_device(device).allocate_buffers([
        voxy_gpu_core::ManagedBufferDescriptor {
            label,
            size: data.len().max(4) as u64,
            contents: if data.is_empty() { None } else { Some(data) },
            usage,
        },
    ])?;
    Ok(b)
}
fn voxy_render_identity_mr(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Arc<voxy_gpu_core::ManagedTexture>, ComputeError> {
    let image = crate::ImageAsset::from_rgba(1, 1, vec![255, 255, 0, 255], Default::default())
        .map_err(|e| ComputeError::Validation(e.to_string()))?;
    ArchiveImageSource::Rgba(image).upload(device, queue)
}
fn uniform(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn texture(binding: u32, depth: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: if depth {
                wgpu::TextureSampleType::Depth
            } else {
                wgpu::TextureSampleType::Float { filterable: true }
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
fn sampler(binding: u32, comparison: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(if comparison {
            wgpu::SamplerBindingType::Comparison
        } else {
            wgpu::SamplerBindingType::Filtering
        }),
        count: None,
    }
}
impl NativeVegetation {
    /// # Errors
    /// Rejects invalid source references, invalid bounds, or exhausted GPU memory.
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        images: Vec<ArchiveImageSource>,
        sources: Vec<ArchiveMaterialSource>,
        geometry: Vec<ArchiveMeshSource>,
        bounds: (Vec3, Vec3),
    ) -> Result<Self, ComputeError> {
        let shadow_vp = vegetation_shadow_matrix(bounds)?;
        for mesh in &geometry {
            if mesh.material >= sources.len()
                || mesh.indices.len() % 3 != 0
                || mesh
                    .indices
                    .iter()
                    .any(|i| *i as usize >= mesh.vertices.len())
                || mesh
                    .vertices
                    .iter()
                    .any(|v| v.position.iter().any(|v| !v.is_finite()))
            {
                return Err(ComputeError::Validation(
                    "invalid canopy source geometry".into(),
                ));
            }
        }
        let canopies: Vec<_> = (0..sources.len())
            .map(|i| {
                let leaf = if sources[i].parameters.workflow[3] > 0. {
                    i
                } else if sources
                    .get(i + 1)
                    .is_some_and(|m| m.parameters.workflow[3] > 0.)
                {
                    i + 1
                } else {
                    i
                };
                canopy_metadata(&geometry, leaf)
            })
            .collect();
        let shadow_wind_enabled =
            std::env::var("VOXY_VEGETATION_SHADOW_WIND").as_deref() == Ok("1");
        let prepare_shadow_wind = shadow_wind_enabled || cfg!(test);
        let instance_usage = wgpu::BufferUsages::VERTEX
            | if prepare_shadow_wind {
                wgpu::BufferUsages::STORAGE
            } else {
                wgpu::BufferUsages::empty()
            };
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("native vegetation camera"),
            entries: &[uniform(
                0,
                wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE,
            )],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("native vegetation materials"),
            entries: &[
                uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                texture(1, false),
                texture(2, false),
                sampler(3, false),
                texture(4, false),
            ],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("native vegetation shadows"),
            entries: &[texture(0, true), sampler(1, true)],
        });
        let receiver_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("native vegetation shadow receivers"),
            entries: &[texture(0, true), texture(1, true), sampler(2, true)],
        });
        let camera = buffer(
            device,
            "native vegetation camera",
            bytemuck::bytes_of(&Camera::zeroed()),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        )?;
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let camera_ring =
            crate::SharedUploadRing::new(device, camera.size(), wgpu::BufferUsages::UNIFORM);
        let camera_groups = camera_ring
            .as_ref()
            .map(|ring| {
                ring.buffers()
                    .map(|buffer| {
                        device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("shared vegetation camera"),
                            layout: &camera_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: buffer.as_entire_binding(),
                            }],
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let shadow =
            ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
                label: Some("native vegetation shadow map"),
                size: wgpu::Extent3d {
                    width: 4096,
                    height: 4096,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | if cfg!(test) {
                        wgpu::TextureUsages::COPY_SRC
                    } else {
                        wgpu::TextureUsages::empty()
                    },
                view_formats: &[],
            })?;
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shadow_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &shadow.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let mut textures = Vec::new();
        for image in images {
            textures.push(image.upload(device, queue)?);
        }
        let default_mr = voxy_render_identity_mr(device, queue)?;
        let colour_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let mut materials = Vec::new();
        for (material_id, source) in sources.into_iter().enumerate() {
            if bytemuck::cast_slice::<crate::archive_scene::ArchiveMaterial, f32>(
                std::slice::from_ref(&source.parameters),
            )
            .iter()
            .any(|v| !v.is_finite())
            {
                return Err(ComputeError::Validation(
                    "nonfinite botanical material".into(),
                ));
            }
            let colour = source.images[0]
                .and_then(|i| textures.get(i))
                .ok_or_else(|| ComputeError::Validation("missing botanical colour map".into()))?;
            let normal = source.images[2]
                .and_then(|i| textures.get(i))
                .ok_or_else(|| ComputeError::Validation("missing botanical normal map".into()))?;
            let mr = match source.images[1] {
                Some(i) => textures.get(i).ok_or_else(|| {
                    ComputeError::Validation("invalid plant roughness map".into())
                })?,
                None => &default_mr,
            };
            let mr = mr.create_view(&wgpu::TextureViewDescriptor {
                format: Some(mr.format().remove_srgb_suffix()),
                ..Default::default()
            });
            let (canopy, extent_density) = canopies[material_id];
            let parameters = PlantMaterial {
                base: source.parameters,
                canopy,
                extent_density,
            };
            let data = buffer(
                device,
                "native plant PBR material",
                bytemuck::bytes_of(&parameters),
                wgpu::BufferUsages::UNIFORM,
            )?;
            let colour = colour.create_view(&wgpu::TextureViewDescriptor {
                format: Some(colour.format().add_srgb_suffix()),
                ..Default::default()
            });
            let normal = normal.create_view(&wgpu::TextureViewDescriptor {
                format: Some(normal.format().remove_srgb_suffix()),
                ..Default::default()
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &material_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: data.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&colour),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&colour_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&mr),
                    },
                ],
            });
            materials.push(Material {
                alpha_cutoff: parameters.base.surface[3],
                bind,
                _uniform: data,
            });
        }
        textures.push(default_mr);
        let mut meshes = Vec::new();
        for mesh in geometry {
            if mesh.instances.iter().any(|i| {
                i.world
                    .iter()
                    .flatten()
                    .chain(i.normal.iter().flatten())
                    .any(|v| !v.is_finite())
            }) {
                return Err(ComputeError::Validation(
                    "nonfinite botanical transform".into(),
                ));
            }
            if mesh.material >= materials.len()
                || mesh.indices.len() % 3 != 0
                || mesh
                    .indices
                    .iter()
                    .any(|i| *i as usize >= mesh.vertices.len())
                || mesh.vertices.iter().any(|v| {
                    v.position
                        .iter()
                        .chain(&v.normal)
                        .chain(&v.tangent)
                        .chain(&v.uv)
                        .any(|v| !v.is_finite())
                })
            {
                return Err(ComputeError::Validation("invalid botanical mesh".into()));
            }
            let instance_ring = crate::SharedUploadRing::new(
                device,
                (mesh.instances.len() * size_of::<ArchiveInstance>()) as u64,
                instance_usage,
            );
            let snapshot = if instance_ring.is_some() {
                mesh.instances.clone()
            } else {
                Vec::new()
            };
            let local_bounds = wind_bounds(
                mesh.vertices
                    .iter()
                    .map(|vertex| Vec3::from_array(vertex.position)),
            );
            let instance_bounds = mesh
                .instances
                .iter()
                .map(|instance| PlantBounds::transformed(local_bounds, instance))
                .collect();
            meshes.push(Mesh {
                instance_ring,
                generated: None,
                local_bounds,
                instance_bounds,
                shadow_wind: None,
                snapshot,
                vertices: buffer(
                    device,
                    "native plant vertices",
                    bytemuck::cast_slice(&mesh.vertices),
                    wgpu::BufferUsages::VERTEX,
                )?,
                indices: buffer(
                    device,
                    "native plant indices",
                    bytemuck::cast_slice(&mesh.indices),
                    wgpu::BufferUsages::INDEX,
                )?,
                instances: buffer(
                    device,
                    "native plant instances",
                    bytemuck::cast_slice(&mesh.instances),
                    instance_usage | wgpu::BufferUsages::COPY_DST,
                )?,
                count: mesh.indices.len() as u32,
                instance_count: mesh.instances.len() as u32,
                capacity: mesh.instances.len(),
                material: mesh.material,
            });
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native plant PBR"),
            source: wgpu::ShaderSource::Wgsl(include_str!("native_vegetation.wgsl").into()),
        });
        let receiver_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native plant world shadows"),
            source: wgpu::ShaderSource::Wgsl(include_str!("native_vegetation_shadow.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&material_layout),
                Some(&shadow_layout),
            ],
            immediate_size: 0,
        });
        let depth_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let receive_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&receiver_layout)],
            immediate_size: 0,
        });
        let vertex_attrs =
            wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x4,3=>Float32x2];
        let instance_attrs = wgpu::vertex_attr_array![4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4,9=>Float32x4,10=>Float32x4];
        let buffers = [
            wgpu::VertexBufferLayout {
                array_stride: 48,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attrs,
            },
            wgpu::VertexBufferLayout {
                array_stride: 112,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attrs,
            },
        ];
        let buffers = buffers.map(Some);
        let shadow_vertex_attrs = [vertex_attrs[0], vertex_attrs[3]];
        let shadow_buffers = [
            Some(wgpu::VertexBufferLayout {
                attributes: &shadow_vertex_attrs,
                ..buffers[0].clone().unwrap()
            }),
            Some(wgpu::VertexBufferLayout {
                attributes: &instance_attrs[..4],
                ..buffers[1].clone().unwrap()
            }),
        ];
        let opaque_shadow_buffers = [
            Some(wgpu::VertexBufferLayout {
                attributes: &shadow_vertex_attrs[..1],
                ..shadow_buffers[0].clone().unwrap()
            }),
            shadow_buffers[1].clone(),
        ];
        let wind = prepare_shadow_wind.then(|| ShadowWind::new(device, &camera_layout, &shader));
        let wind_layout = wind.as_ref().map(|wind| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("prepared wind shadow layout"),
                bind_group_layouts: &[
                    Some(&camera_layout),
                    Some(&material_layout),
                    Some(&wind.vertex_layout),
                ],
                immediate_size: 0,
            })
        });
        let make = |format, depth_only: bool, opaque_shadow: bool, prepared: bool| {
            let targets = [Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })];
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("native vegetation surface"),
                layout: Some(if prepared {
                    wind_layout.as_ref().unwrap()
                } else if depth_only {
                    &depth_layout
                } else {
                    &layout
                }),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if prepared {
                        "vs_shadow_prepared"
                    } else if opaque_shadow {
                        "vs_shadow_opaque"
                    } else if depth_only {
                        "vs_shadow"
                    } else {
                        "vs_main"
                    }),
                    compilation_options: Default::default(),
                    buffers: if opaque_shadow {
                        &opaque_shadow_buffers
                    } else if depth_only {
                        &shadow_buffers
                    } else {
                        &buffers
                    },
                },
                fragment: if opaque_shadow {
                    None
                } else if depth_only {
                    Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_shadow"),
                        compilation_options: Default::default(),
                        targets: &[],
                    })
                } else {
                    Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &targets,
                    })
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(if depth_only {
                        wgpu::CompareFunction::LessEqual
                    } else {
                        wgpu::CompareFunction::GreaterEqual
                    }),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = make(format, false, false, false);
        let hdr_pipeline = make(wgpu::TextureFormat::Rgba16Float, false, false, false);
        let shadow_pipeline = make(format, true, false, false);
        let opaque_shadow_pipeline = make(format, true, true, false);
        let shadow_wind = if let Some(wind) = wind {
            let pipeline = make(format, true, false, true);
            for mesh in &mut meshes {
                mesh.shadow_wind = Some(wind.allocate(device, mesh.capacity)?);
            }
            Some((wind, pipeline))
        } else {
            None
        };
        let receiver_make = |format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("native plant shadow receiver"),
                layout: Some(&receive_layout),
                vertex: wgpu::VertexState {
                    module: &receiver_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &receiver_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::Zero,
                                dst_factor: wgpu::BlendFactor::Src,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::COLOR,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let receiver = receiver_make(format);
        let hdr_receiver = receiver_make(wgpu::TextureFormat::Rgba16Float);
        Ok(Self {
            meshes,
            materials,
            _images: textures,
            camera,
            camera_bind,
            camera_ring,
            camera_groups,
            shadow,
            shadow_bind,
            receiver_layout,
            sampler,
            pipeline,
            hdr_pipeline,
            shadow_pipeline,
            opaque_shadow_pipeline,
            shadow_wind,
            shadow_wind_enabled,
            opaque_shadow_fast: std::env::var("VOXY_VEGETATION_OPAQUE_SHADOW").as_deref()
                == Ok("1"),
            started: std::time::Instant::now(),
            #[cfg(test)]
            frozen_benchmark_time: None,
            receiver,
            hdr_receiver,
            shadow_vp,
            shadow_capture_override: None,
            cull_shadow_receivers: std::env::var("VOXY_VEGETATION_CULL_SHADOW_RECEIVERS")
                .as_deref()
                != Ok("0"),
            cull_instances: std::env::var_os("VOXY_VEGETATION_FULL_DRAWS").is_none(),
        })
    }
    pub(crate) fn prepare_window(
        &self,
        device: &wgpu::Device,
        window: NativeVegetationWindow,
    ) -> Result<PreparedVegetationWindow, ComputeError> {
        let shadow_vp = vegetation_shadow_matrix(window.bounds)?;
        if window.instances.len() != self.meshes.len() {
            return Err(ComputeError::Validation(
                "invalid botanical group count".into(),
            ));
        }
        let mut meshes = Vec::with_capacity(self.meshes.len());
        for (mesh, values) in self.meshes.iter().zip(window.instances) {
            if u32::try_from(values.len()).is_err()
                || values.iter().any(|i| {
                    i.world
                        .iter()
                        .flatten()
                        .chain(i.normal.iter().flatten())
                        .any(|v| !v.is_finite())
                })
            {
                return Err(ComputeError::Validation(
                    "invalid botanical instances".into(),
                ));
            }
            let usage = mesh.instances.usage();
            let data = bytemuck::cast_slice(&values);
            let ring = crate::SharedUploadRing::new(
                device,
                data.len() as u64,
                usage - wgpu::BufferUsages::COPY_DST,
            );
            let bounds = values
                .iter()
                .map(|value| PlantBounds::transformed(mesh.local_bounds, value))
                .collect();
            let wind = self
                .shadow_wind
                .as_ref()
                .map(|(wind, _)| wind.allocate(device, values.len()))
                .transpose()?;
            meshes.push(PreparedPlantInstances {
                buffer: buffer(device, "native plant instances", data, usage)?,
                ring,
                values,
                bounds,
                wind,
            });
        }
        Ok(PreparedVegetationWindow { meshes, shadow_vp })
    }

    pub(crate) fn publish_window(&mut self, prepared: PreparedVegetationWindow) {
        for (mesh, next) in self.meshes.iter_mut().zip(prepared.meshes) {
            mesh.capacity = next.values.len();
            mesh.instance_count = next.values.len() as u32;
            mesh.instances = next.buffer;
            mesh.snapshot = if next.ring.is_some() {
                next.values
            } else {
                Vec::new()
            };
            mesh.instance_ring = next.ring;
            mesh.instance_bounds = next.bounds;
            mesh.shadow_wind = next.wind;
            mesh.generated = None;
        }
        self.shadow_vp = prepared.shadow_vp;
    }

    /// Atomically validate all instance lists before uploading changed plant visibility.
    /// # Errors
    /// Rejects expanded lists or invalid transforms.
    pub fn update_instances(
        &mut self,
        queue: &wgpu::Queue,
        instances: &[Vec<ArchiveInstance>],
    ) -> Result<(), ComputeError> {
        if instances.len() != self.meshes.len()
            || instances.iter().zip(&self.meshes).any(|(v, m)| {
                v.len() > m.capacity
                    || v.iter().any(|i| {
                        i.world
                            .iter()
                            .flatten()
                            .chain(i.normal.iter().flatten())
                            .any(|x| !x.is_finite())
                    })
            })
        {
            return Err(ComputeError::Validation(
                "invalid botanical instances".into(),
            ));
        }
        for (mesh, values) in self.meshes.iter_mut().zip(instances) {
            if mesh.instance_ring.is_some() {
                mesh.snapshot.clone_from(values);
            } else if !values.is_empty() {
                queue.write_buffer(&mesh.instances, 0, bytemuck::cast_slice(values));
            }
            mesh.instance_bounds = values
                .iter()
                .map(|value| PlantBounds::transformed(mesh.local_bounds, value))
                .collect();
            mesh.generated = None;
            mesh.instance_count = values.len() as u32;
        }
        Ok(())
    }
    /// Generate final instance bytes directly in shared memory on Apple Silicon.
    /// Other devices retain the validated slice upload. Failed groups publish none.
    pub fn update_instances_generated<I, J>(
        &mut self,
        queue: &wgpu::Queue,
        groups: I,
    ) -> Result<(), ComputeError>
    where
        I: ExactSizeIterator<Item = (usize, J)>,
        J: Iterator<Item = ArchiveInstance>,
    {
        if groups.len() != self.meshes.len() {
            return Err(ComputeError::Validation(
                "invalid botanical group count".into(),
            ));
        }
        let mut pending = Vec::with_capacity(self.meshes.len());
        for (mesh, (count, values)) in self.meshes.iter().zip(groups) {
            if count > mesh.capacity {
                return Err(ComputeError::Validation("botanical capacity".into()));
            }
            let mut bounds = Vec::with_capacity(count);
            let mut validate = |value: ArchiveInstance| {
                // Validate complete SIMD rows while they are in registers. The
                // scalar flattened iterator prevents vectorization when interleaved
                // with direct write-only stores for each generated instance.
                if !Mat4::from_cols_array_2d(&value.world).is_finite()
                    || !glam::Vec4::from_array(value.normal[0]).is_finite()
                    || !glam::Vec4::from_array(value.normal[1]).is_finite()
                    || !glam::Vec4::from_array(value.normal[2]).is_finite()
                {
                    Err(ComputeError::Validation(
                        "invalid botanical transform".into(),
                    ))
                } else {
                    bounds.push(PlantBounds::transformed(mesh.local_bounds, &value));
                    Ok(value)
                }
            };
            let (generated, legacy) = if let Some(ring) =
                mesh.instance_ring.as_ref().filter(|_| count != 0)
            {
                (
                    Some(
                        ring.prepare((count * size_of::<ArchiveInstance>()) as u64, |bytes| {
                            let mut chunks = bytes
                                .into_chunks::<{ size_of::<ArchiveInstance>() }>()
                                .0
                                .into_iter();
                            for value in values {
                                let Some(target) = chunks.next() else {
                                    return Err(ComputeError::InvalidBuffer);
                                };
                                let value = validate(value)?;
                                target.write(bytemuck::cast(value));
                            }
                            if chunks.next().is_some() {
                                return Err(ComputeError::InvalidBuffer);
                            }
                            Ok::<_, ComputeError>(())
                        })?,
                    ),
                    Vec::new(),
                )
            } else {
                (None, values.map(validate).collect::<Result<Vec<_>, _>>()?)
            };
            if generated.is_none() && legacy.len() != count {
                return Err(ComputeError::InvalidBuffer);
            }
            pending.push((count, generated, legacy, bounds));
        }
        for (mesh, (count, generated, legacy, bounds)) in self.meshes.iter_mut().zip(pending) {
            if generated.is_none() && !legacy.is_empty() {
                queue.write_buffer(&mesh.instances, 0, bytemuck::cast_slice(&legacy));
            }
            mesh.generated = generated;
            mesh.instance_bounds = bounds;
            mesh.snapshot = Vec::new();
            mesh.instance_count = count as u32;
        }
        Ok(())
    }
    fn input_buffers(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        camera: &Camera,
    ) -> (wgpu::BindGroup, Vec<wgpu::Buffer>) {
        let camera_group = if let Some(ring) = &self.camera_ring {
            let (slot, _) = ring
                .write_or_stage(encoder, &self.camera, |mut bytes| {
                    bytes.copy_from_slice(bytemuck::bytes_of(camera));
                })
                .expect("live shared vegetation camera");
            self.camera_groups
                .get(slot)
                .unwrap_or(&self.camera_bind)
                .clone()
        } else {
            queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(camera));
            self.camera_bind.clone()
        };
        (camera_group, self.instance_buffers(encoder))
    }
    fn instance_buffers(&self, encoder: &mut wgpu::CommandEncoder) -> Vec<wgpu::Buffer> {
        self.meshes
            .iter()
            .map(|mesh| {
                if let Some(snapshot) = &mesh.generated {
                    return snapshot.buffer_for_encoder(encoder, &mesh.instances);
                }
                if let Some(ring) = mesh
                    .instance_ring
                    .as_ref()
                    .filter(|_| mesh.instance_count != 0)
                {
                    ring.write_or_stage(encoder, &mesh.instances, |mut bytes| {
                        bytes
                            .slice(..mesh.snapshot.len() * size_of::<ArchiveInstance>())
                            .copy_from_slice(bytemuck::cast_slice(&mesh.snapshot));
                    })
                    .expect("live shared vegetation instances")
                    .1
                } else {
                    (*mesh.instances).clone()
                }
            })
            .collect()
    }
    pub(crate) fn shadow_capture_caster_counts(&self, vp: Mat4) -> Option<[u32; 2]> {
        let receivers = ClipVolume::shadow_receivers(vp, self.shadow_vp, self.shadow.width())?;
        if !self.cull_instances {
            return None;
        }
        let light = ClipVolume::new(self.shadow_vp);
        let count = |additional| {
            self.meshes
                .iter()
                .flat_map(|mesh| light.visible_ranges_with(&mesh.instance_bounds, additional))
                .map(|range| range.end - range.start)
                .sum()
        };
        Some([count(None), count(Some(&receivers))])
    }

    pub(crate) fn set_shadow_capture_override(&mut self, capture: Option<(bool, f32)>) {
        self.shadow_capture_override = capture;
    }

    #[expect(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub(crate) fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        vp: Mat4,
        eye: Vec3,
        hdr: bool,
        [shadow_timing, receiver_timing, surface_timing]: [Option<wgpu::RenderPassTimestampWrites<'_>>;
            3],
    ) {
        let wind_time = self.shadow_capture_override.map_or_else(
            || self.started.elapsed().as_secs_f32() % 3600.,
            |(_, wind_time)| wind_time,
        );
        #[cfg(test)]
        let wind_time = self.frozen_benchmark_time.unwrap_or(wind_time);
        let camera = Camera {
            vp: vp.to_cols_array_2d(),
            inverse_vp: vp.inverse().to_cols_array_2d(),
            shadow: self.shadow_vp.to_cols_array_2d(),
            eye: eye.extend(1.).to_array(),
            sun: Vec3::new(0.42, 0.82, 0.38)
                .normalize()
                .extend(0.)
                .to_array(),
            flags: [f32::from(hdr), wind_time, 0., 0.],
        };
        let (camera_group, instances) = self.input_buffers(encoder, queue, &camera);
        let prepared = self
            .shadow_wind
            .as_ref()
            .filter(|_| self.shadow_wind_enabled);
        if let Some((wind, _)) = prepared {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("prepare plant shadow wind"),
                timestamp_writes: None,
            });
            for (mesh, instances) in self.meshes.iter().zip(&instances) {
                if mesh.instance_count != 0 {
                    wind.encode(
                        device,
                        &mut pass,
                        &camera_group,
                        instances,
                        mesh.shadow_wind.as_ref().expect("prepared wind buffer"),
                        mesh.instance_count,
                    );
                }
            }
        }
        let shadow_view = self.shadow.create_view(&Default::default());
        let shadow_clip = ClipVolume::new(self.shadow_vp);
        let cull_shadow_receivers = self
            .shadow_capture_override
            .map_or(self.cull_shadow_receivers, |(enabled, _)| enabled);
        let shadow_receivers = if cull_shadow_receivers {
            ClipVolume::shadow_receivers(vp, self.shadow_vp, self.shadow.width())
        } else {
            None
        };
        let camera_clip = ClipVolume::new(vp);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("native plant shadow casters"),
                timestamp_writes: shadow_timing,
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &camera_group, &[]);
            for (mesh, instances) in self.meshes.iter().zip(&instances) {
                if mesh.instance_count == 0 {
                    continue;
                }
                let material = &self.materials[mesh.material];
                pass.set_pipeline(if let Some((_, pipeline)) = prepared {
                    pipeline
                } else if self.opaque_shadow_fast && material.alpha_cutoff < 0. {
                    &self.opaque_shadow_pipeline
                } else {
                    &self.shadow_pipeline
                });
                pass.set_bind_group(1, &material.bind, &[]);
                if prepared.is_some() {
                    pass.set_bind_group(2, &mesh.shadow_wind.as_ref().unwrap().vertex_bind, &[]);
                }
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if self.cull_instances {
                    for range in shadow_clip
                        .visible_ranges_with(&mesh.instance_bounds, shadow_receivers.as_ref())
                    {
                        pass.draw_indexed(0..mesh.count, 0, range);
                    }
                } else {
                    pass.draw_indexed(0..mesh.count, 0, 0..mesh.instance_count);
                }
            }
        }
        let receiver_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.receiver_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("native plant ground/actor shadows"),
                timestamp_writes: receiver_timing,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: colour,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(if hdr {
                &self.hdr_receiver
            } else {
                &self.receiver
            });
            pass.set_bind_group(0, &camera_group, &[]);
            pass.set_bind_group(1, &receiver_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("native plant world surfaces"),
                timestamp_writes: surface_timing,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: colour,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(if hdr {
                &self.hdr_pipeline
            } else {
                &self.pipeline
            });
            pass.set_bind_group(0, &camera_group, &[]);
            pass.set_bind_group(2, &self.shadow_bind, &[]);
            for (mesh, instances) in self.meshes.iter().zip(&instances) {
                if mesh.instance_count == 0 {
                    continue;
                }
                pass.set_bind_group(1, &self.materials[mesh.material].bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if self.cull_instances {
                    for range in camera_clip.visible_ranges(&mesh.instance_bounds) {
                        pass.draw_indexed(0..mesh.count, 0, range);
                    }
                } else {
                    pass.draw_indexed(0..mesh.count, 0, 0..mesh.instance_count);
                }
            }
        }
    }
}
use bytemuck::Zeroable;

/// Evaluates foliage thin-slab translucent backlighting combining Beer-Lambert slab transmittance
/// `exp(-extinction * thickness)` with the Henyey-Greenstein forward scattering phase function:
/// `exp(-sigma_t * d) * (1 - g^2) / (4 * pi * (1 + g^2 - 2 * g * cos_theta)^1.5)`.
#[must_use]
pub fn foliage_translucent_backlighting(
    cos_theta: f32,
    anisotropy_g: f32,
    thickness: f32,
    extinction: f32,
) -> f32 {
    if !cos_theta.is_finite()
        || !anisotropy_g.is_finite()
        || !thickness.is_finite()
        || !extinction.is_finite()
    {
        return 0.0;
    }
    let mu = cos_theta.clamp(-1.0, 1.0);
    let g = anisotropy_g.clamp(-0.99, 0.99);
    let g2 = g * g;
    let denom = (1.0 + g2 - 2.0 * g * mu).max(1e-5).powf(1.5);
    let phase = (1.0 - g2) / (4.0 * std::f32::consts::PI * denom);
    let optical_depth = (thickness.max(0.0) * extinction.max(0.0)).min(80.0);
    (phase * (-optical_depth).exp()).max(0.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn plant_shader_with_wind_and_alpha_shadows_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("native_vegetation.wgsl")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        for entry in ["vs_main", "vs_shadow", "fs_main", "fs_shadow"] {
            assert!(
                module.entry_points.iter().any(|e| e.name == entry),
                "{entry}"
            );
        }
        let fwd = super::foliage_translucent_backlighting(1.0, 0.6, 0.5, 1.2);
        let back = super::foliage_translucent_backlighting(-1.0, 0.6, 0.5, 1.2);
        assert!(fwd > back && back > 0.0);
    }
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "native_vegetation_shared_tests.rs"]
mod shared_tests;
