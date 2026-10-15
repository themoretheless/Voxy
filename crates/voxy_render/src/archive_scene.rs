//! Render-ready immutable imported geometry; authoring and physics remain separate owners.
use crate::{Bc7ImageAsset, ComputeMemoryBudget, ComputeStorage};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use std::sync::Arc;
/// Original display panorama and separately prepared indirect-light panorama.
#[derive(Debug)]
pub struct ArchiveEnvironmentSource {
    pub lighting: crate::HdrImageAsset,
    pub background: crate::HdrImageAsset,
    pub intensity: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveInstance {
    pub world: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 3],
}
impl ArchiveInstance {
    pub fn new(world: Mat4) -> Self {
        let normal = world.inverse().transpose();
        Self {
            world: world.to_cols_array_2d(),
            normal: [
                normal.x_axis.to_array(),
                normal.y_axis.to_array(),
                normal.z_axis.to_array(),
            ],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveMaterial {
    pub diffuse: [f32; 4],
    pub specular_gloss: [f32; 4],
    pub emissive_normal: [f32; 4],
    /// metallic, roughness, transmission, alpha cutoff (-1 opaque, -2 blend)
    pub surface: [f32; 4],
    /// 0 specular/glossiness, 1 metallic/roughness; reserved
    pub workflow: [f32; 4],
}
impl ArchiveMaterial {
    /// Metal/roughness parameters with no scanned maps or procedural surface.
    pub const fn metallic_roughness(colour: [f32; 4], metallic: f32, roughness: f32, transmission: f32, cutoff: f32) -> Self {
        Self {
            diffuse: colour,
            specular_gloss: [1.; 4],
            emissive_normal: [0., 0., 0., 1.],
            surface: [metallic, roughness, transmission, cutoff],
            workflow: [1., 0., 0., 0.],
        }
    }
    /// Silvered mirror: screen-traced near-ideal metal reflection.
    pub const MIRROR: Self = Self::metallic_roughness([0.97, 0.97, 0.96, 1.], 1., 0.02, 0., -1.);
    /// Clear window glass (F0 0.04): reflects the scene and refracts what is behind it.
    pub const GLASS: Self = Self::metallic_roughness([0.92, 0.96, 0.95, 0.1], 0., 0.02, 0.92, -2.);
    /// Dark standing-water sheet using the default dielectric F0.
    pub const WATER: Self = Self::metallic_roughness([0.25, 0.3, 0.3, 1.], 0., 0.03, 0., -1.);
}
/// Source pixels for an archive material; colour transfer is selected by the shader role.
#[derive(Debug)]
pub enum ArchiveImageSource {
    Bc7(Bc7ImageAsset),
    Rgba(crate::ImageAsset),
    /// Linear float32 pixels retain the precision of 16-bit scanned source maps.
    Float { width: u32, height: u32, pixels: Vec<f32> },
}
impl ArchiveImageSource {
    /// Upload source pixels and every mip level; float data retains float32 precision.
    pub fn upload(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Arc<voxy_gpu_core::ManagedTexture>, crate::ComputeError> {
        match self {
            Self::Bc7(image) => image.upload(device, queue),
            Self::Float { width, height, pixels } => {
                if width == 0 || height == 0 || width > device.limits().max_texture_dimension_2d
                    || height > device.limits().max_texture_dimension_2d
                    || u64::from(width) * u64::from(height) * 4 != pixels.len() as u64
                    || pixels.iter().any(|p| !p.is_finite())
                    || !device.features().contains(wgpu::Features::FLOAT32_FILTERABLE) {
                    return Err(crate::ComputeError::Validation("invalid float material or missing FLOAT32_FILTERABLE".into()));
                }
                let mips = width.max(height).ilog2() + 1;
                let texture = ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
                    label: Some("full precision scanned material"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    mip_level_count: mips, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba32Float,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })?;
                let mut pixels = image::Rgba32FImage::from_raw(width, height, pixels).ok_or_else(|| crate::ComputeError::Validation("invalid float pixels".into()))?;
                for mip in 0..mips {
                    let (w,h) = pixels.dimensions();
                    queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: mip, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                        bytemuck::cast_slice(pixels.as_raw()), wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 16), rows_per_image: Some(h) },
                        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
                    if mip + 1 < mips {
                        pixels = image::imageops::resize(&pixels, (w/2).max(1), (h/2).max(1), image::imageops::FilterType::Triangle);
                    }
                }
                Ok(texture)
            }
            Self::Rgba(image) => {
                let width = image.width();
                let height = image.height();
                let mips = width.max(height).ilog2() + 1;
                let texture = ComputeMemoryBudget::for_device(device).allocate_texture(
                    &wgpu::TextureDescriptor {
                        label: Some("archive RGBA material image"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: mips,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[
                            wgpu::TextureFormat::Rgba8Unorm,
                            wgpu::TextureFormat::Rgba8UnormSrgb,
                        ],
                    },
                )?;
                let mut pixels = image::RgbaImage::from_raw(width, height, image.rgba().to_vec())
                    .ok_or_else(|| {
                    crate::ComputeError::Validation("invalid RGBA image".into())
                })?;
                for mip in 0..mips {
                    let w = pixels.width();
                    let h = pixels.height();
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: mip,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        pixels.as_raw(),
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(w * 4),
                            rows_per_image: Some(h),
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    if mip + 1 < mips {
                        pixels = image::imageops::resize(
                            &pixels,
                            (w / 2).max(1),
                            (h / 2).max(1),
                            image::imageops::FilterType::Triangle,
                        );
                    }
                }
                Ok(texture)
            }
        }
    }
}
#[derive(Debug)]
pub struct ArchiveMaterialSource {
    pub parameters: ArchiveMaterial,
    /// diffuse, specular/glossiness OR metallic/roughness, normal, emissive
    pub images: [Option<usize>; 4],
    pub double_sided: bool,
}
#[derive(Debug)]
pub struct ArchiveMeshSource {
    pub vertices: Vec<ArchiveVertex>,
    pub indices: Vec<u32>,
    pub instances: Vec<ArchiveInstance>,
    pub material: usize,
    pub center: Vec3,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveLight {
    pub position_kind: [f32; 4],
    pub colour_intensity: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Camera {
    vp: [[f32; 4]; 4],
    shadow: [[f32; 4]; 4],
    inverse_vp: [[f32; 4]; 4],
    eye: [f32; 4],
    sun: [f32; 4],
    sun_colour: [f32; 4],
    viewport: [f32; 4],
    weather: [f32; 4],
    atmosphere: [f32; 4],
    wind: [f32; 4],
    /// Jittered view-projection of the frame whose opaque radiance is in `background`.
    previous_vp: [[f32; 4]; 4],
    previous_inverse_vp: [[f32; 4]; 4],
    /// [surface wetness, puddle coverage, history valid, vertical pixel angular size].
    surface: [f32; 4],
}
#[derive(Debug)]
struct Material {
    bind: wgpu::BindGroup,
    _uniform: ComputeStorage,
    transparent: bool,
    double_sided: bool,
    masked: bool,
}
#[derive(Debug)]
struct Mesh {
    culling: Option<crate::ArchiveInstanceCulling>,
    frustum: crate::archive_frustum::ArchiveFrustumGroups,
    scratch_ids: Vec<usize>,
    scratch_instances: Vec<ArchiveInstance>,
    vertices: ComputeStorage,
    indices: ComputeStorage,
    instances: ComputeStorage,
    visible_instances: ComputeStorage,
    source_instances: Vec<ArchiveInstance>,
    spheres: Vec<(Vec3, f32)>,
    visible_ids: Vec<usize>,
    visible_count: u32,
    count: u32,
    copies: u32,
    material: usize,
    center: Vec3,
}
#[derive(Debug)]
pub struct ArchiveSceneRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    _textures: Vec<Arc<voxy_gpu_core::ManagedTexture>>,
    _lights: ComputeStorage,
    camera: ComputeStorage,
    camera_bind: wgpu::BindGroup,
    opaque: wgpu::RenderPipeline,
    opaque_double: wgpu::RenderPipeline,
    masked: wgpu::RenderPipeline,
    masked_double: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    transparent_double: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    depth_pipeline: wgpu::RenderPipeline,
    depth_double: wgpu::RenderPipeline,
    timing: Option<Timing>,
    pub gpu_times_ms: Vec<f64>,
    /// [opaque shading, transparent shading] GPU intervals, excluding presentation.
    pub gpu_shading_times_ms: Vec<[f64; 2]>,
    exposure: f32,
    sky_strength: f32,
    _environment: crate::ImportedEnvironment,
    environment_bindings: crate::environment_lighting::EnvironmentBindings,
    environment_enabled: bool,
    _sky: Arc<voxy_gpu_core::ManagedTexture>,
    post: wgpu::RenderPipeline,
    post_linear: wgpu::RenderPipeline,
    output_format: wgpu::TextureFormat,
    antialiasing: crate::ArchiveAntialiasing,
    temporal: Option<crate::archive_temporal::ArchiveTemporal>,
    post_bind: wgpu::BindGroup,
    bloom: crate::bloom::Bloom,
    display_parameters: ComputeStorage,
    hdr_output: bool,
    linear_output: bool,
    vertical_fov: f32,
    camera_up: Vec3,
    tone_mapper: ArchiveToneMapper,
    camera_lens: crate::archive_temporal::ArchiveCameraLens,
    display_headroom: f32,
    spatial_antialiasing: bool,
    frame_bind: wgpu::BindGroup,
    probe_sh: ComputeStorage,
    probe_grid: ComputeStorage,
    hdr: Arc<voxy_gpu_core::ManagedTexture>,
    background: Arc<voxy_gpu_core::ManagedTexture>,
    depth: wgpu::Texture,
    history_depth: Arc<voxy_gpu_core::ManagedTexture>,
    shadow: wgpu::Texture,
    shadow_matrix: Mat4,
    weather: crate::weather::Weather,
    weather_time: f32,
    surface_wetness: Option<f32>,
    puddle_coverage: f32,
    previous_vp: Option<Mat4>,
    shadow_bounds: (Vec3, Vec3),
    sun: Vec3,
    sun_colour: [f32; 4],
    width: u32,
    height: u32,
    shadow_ready: bool,
}
fn buffer(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> Result<ComputeStorage, crate::ComputeError> {
    let [b] = ComputeMemoryBudget::for_device(device).allocate_buffers([
        voxy_gpu_core::ManagedBufferDescriptor {
            label,
            size: data.len() as u64,
            contents: Some(data),
            usage,
        },
    ])?;
    Ok(b)
}
fn target(
    device: &wgpu::Device,
    w: u32,
    h: u32,
) -> Result<Arc<voxy_gpu_core::ManagedTexture>, crate::ComputeError> {
    ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
        label: Some("imported scene linear HDR"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
fn depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("imported scene depth"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
fn texture_entry(binding: u32, depth: bool) -> wgpu::BindGroupLayoutEntry {
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
fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
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
impl ArchiveSceneRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        images: Vec<Bc7ImageAsset>,
        sources: Vec<ArchiveMaterialSource>,
        geometry: Vec<ArchiveMeshSource>,
        lights: &[ArchiveLight],
        sun: Vec3,
        sun_colour: [f32; 4],
        bounds: (Vec3, Vec3),
    ) -> Result<Self, crate::ComputeError> {
        Self::new_with_images(
            device,
            queue,
            format,
            width,
            height,
            images.into_iter().map(ArchiveImageSource::Bc7).collect(),
            sources,
            geometry,
            lights,
            sun,
            sun_colour,
            bounds,
        )
    }
    /// Render imported or generated RGBA images with the same material/shadow pipeline.
    pub fn new_with_images(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        images: Vec<ArchiveImageSource>,
        sources: Vec<ArchiveMaterialSource>,
        geometry: Vec<ArchiveMeshSource>,
        lights: &[ArchiveLight],
        sun: Vec3,
        sun_colour: [f32; 4],
        bounds: (Vec3, Vec3),
    ) -> Result<Self, crate::ComputeError> {
        Self::new_with_environment(device, queue, format, width, height, images, sources, geometry, lights, sun, sun_colour, bounds, None)
    }
    pub fn new_with_environment(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        images: Vec<ArchiveImageSource>,
        sources: Vec<ArchiveMaterialSource>,
        geometry: Vec<ArchiveMeshSource>,
        lights: &[ArchiveLight],
        sun: Vec3,
        sun_colour: [f32; 4],
        bounds: (Vec3, Vec3),
        environment: Option<ArchiveEnvironmentSource>,
    ) -> Result<Self, crate::ComputeError> {
        let environment_enabled = environment.is_some();
        let (panorama, background_panorama, intensity) = match environment {
            Some(source) => (source.lighting, Some(source.background), source.intensity),
            None => (crate::HdrImageAsset::from_rgb(2, 1, vec![0.;6], crate::ImageLimits::default())
                .map_err(|e| crate::ComputeError::Validation(e.to_string()))?, None, 1.),
        };
        if !intensity.is_finite() || intensity <= 0. { return Err(crate::ComputeError::Validation("invalid environment intensity".into())); }
        let sky = if let Some(panorama) = background_panorama {
            ArchiveImageSource::Float { width: panorama.width(), height: panorama.height(),
                pixels: panorama.rgb().chunks_exact(3).flat_map(|p| [p[0],p[1],p[2],1.]).collect() }.upload(device,queue)?
        } else { ArchiveImageSource::Rgba(crate::ImageAsset::from_rgba(1,1,vec![0,0,0,255],crate::ImageLimits::default())
            .map_err(|e|crate::ComputeError::Validation(e.to_string()))?).upload(device,queue)? };
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor { address_mode_u: wgpu::AddressMode::Repeat,
            min_filter: wgpu::FilterMode::Linear, mag_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let environment = if environment_enabled { crate::ImportedEnvironment::from_panorama_full_precision(device, queue, &panorama,
            1024, crate::ImageLimits { pixel_bytes: 512 * 1024 * 1024, ..Default::default() }) } else { crate::ImportedEnvironment::from_panorama(device, queue, &panorama, 1, crate::ImageLimits::default()) }
            .map_err(|e| crate::ComputeError::Validation(e.to_string()))?;
        let environment_bindings = crate::environment_lighting::EnvironmentBindings::new(device,
            environment.specular(), environment.dfg(), Some(environment.diffuse()), intensity, None)
            .map_err(|e| crate::ComputeError::Validation(e.to_string()))?;
        let mut preparation = device.create_command_encoder(&Default::default());
        environment.encode(&mut preparation);
        queue.submit([preparation.finish()]);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("archive camera and lights"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let mut material_entries = vec![uniform_entry(0, wgpu::ShaderStages::FRAGMENT)];
        for i in 1..=4 {
            material_entries.push(texture_entry(i, false));
        }
        material_entries.push(wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("authored four-map PBR material"),
            entries: &material_entries,
        });
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("opaque transmission and sun shadows"),
            entries: &[
                texture_entry(0, false),
                texture_entry(1, true),
                texture_entry(3, true),
                texture_entry(4, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("archive HDR display"),
            entries: &[
                texture_entry(0, false),
                uniform_entry(1, wgpu::ShaderStages::FRAGMENT),
                texture_entry(2, true),
                uniform_entry(3, wgpu::ShaderStages::FRAGMENT),
                texture_entry(4, false),
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                texture_entry(6, false),
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archive PBR"),
            source: wgpu::ShaderSource::Wgsl(concat!(include_str!("bsdf.wgsl"), include_str!("archive_environment.wgsl"), include_str!("soft_shadow.wgsl"), include_str!("probe_sampling.wgsl"), include_str!("archive_history.wgsl"), include_str!("archive_scene.wgsl")).into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("archive scene layout"),
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&material_layout),
                Some(&frame_layout),
                Some(&environment_bindings.layout),
            ],
            immediate_size: 0,
        });
        let vertex_attrs =
            wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x4,3=>Float32x2];
        let instance_attrs = wgpu::vertex_attr_array![4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4,9=>Float32x4,10=>Float32x4];
        let vertex_buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: 48,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attrs,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 112,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attrs,
            }),
        ];
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let pipeline = |blend: bool, double: bool, shadow: bool, predepth: bool, masked: bool| {
            let targets = [Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: if blend {
                    Some(wgpu::BlendState::ALPHA_BLENDING)
                } else {
                    None
                },
                write_mask: wgpu::ColorWrites::ALL,
            })];
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("archive instanced material pipeline"),
                layout: Some(if shadow || predepth {
                    &shadow_layout
                } else {
                    &layout
                }),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if shadow { "vs_shadow" } else { "vs_main" }),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffers,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: if double { None } else { Some(wgpu::Face::Back) },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(shadow || predepth),
                    depth_compare: Some(if !shadow && !predepth && !blend {
                        wgpu::CompareFunction::Equal
                    } else {
                        wgpu::CompareFunction::LessEqual
                    }),
                    stencil: Default::default(),
                    bias: if shadow {
                        wgpu::DepthBiasState {
                            constant: 2,
                            slope_scale: 2.,
                            clamp: 0.,
                        }
                    } else {
                        Default::default()
                    },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if shadow || predepth {
                        "fs_shadow"
                    } else if masked {
                        "fs_main"
                    } else {
                        "fs_opaque"
                    }),
                    compilation_options: Default::default(),
                    targets: if shadow || predepth { &[] } else { &targets },
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let opaque = pipeline(false, false, false, false, false);
        let opaque_double = pipeline(false, true, false, false, false);
        let masked = pipeline(false, false, false, false, true);
        let masked_double = pipeline(false, true, false, false, true);
        let transparent = pipeline(true, false, false, false, false);
        let transparent_double = pipeline(true, true, false, false, false);
        let shadow_pipeline = pipeline(false, true, true, false, false);
        let depth_pipeline = pipeline(false, false, false, true, false);
        let depth_double = pipeline(false, true, false, true, false);
        let timing = Timing::new(device)?;
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archive display transform"),
            source: wgpu::ShaderSource::Wgsl(concat!(include_str!("display_transform.wgsl"), include_str!("archive_environment.wgsl"), include_str!("archive_post.wgsl")).into()),
        });
        let post_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&post_layout), Some(&environment_bindings.layout)],
            immediate_size: 0,
        });
        let create_post = |format| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("archive HDR display pass"),
            layout: Some(&post_pl),
            vertex: wgpu::VertexState {
                module: &post_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &post_shader,
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
        let post = create_post(format);
        let post_linear = create_post(wgpu::TextureFormat::Rgba16Float);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("authored scene anisotropic sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        let mut textures = Vec::new();
        for image in images {
            textures.push(image.upload(device, queue)?);
        }
        // Absent glTF maps have specified neutral values; these are not replacements for source images.
        let mut defaults = Vec::new();
        for colour in [[255, 255, 255, 255], [128, 128, 255, 255]] {
            let t = ComputeMemoryBudget::for_device(device).allocate_texture(
                &wgpu::TextureDescriptor {
                    label: Some("absent glTF map identity"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
            )?;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &t,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &colour,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            defaults.push(t);
        }
        let mut materials = Vec::new();
        for source in sources {
            let uniform = buffer(
                device,
                "authored PBR factors",
                bytemuck::bytes_of(&source.parameters),
                wgpu::BufferUsages::UNIFORM,
            )?;
            let views: Vec<_> = source
                .images
                .iter()
                .enumerate()
                .map(|(slot, index)| {
                    index
                        .map_or(&defaults[usize::from(slot == 2)], |i| &textures[i])
                        .create_view(&wgpu::TextureViewDescriptor {
                            format: index.map(|i| {
                                if slot == 0
                                    || slot == 3
                                    || (slot == 1 && source.parameters.workflow[0] < 0.5)
                                {
                                    textures[i].format().add_srgb_suffix()
                                } else {
                                    textures[i].format().remove_srgb_suffix()
                                }
                            }),
                            ..Default::default()
                        })
                })
                .collect();
            let mut entries = vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }];
            for (i, view) in views.iter().enumerate() {
                entries.push(wgpu::BindGroupEntry {
                    binding: i as u32 + 1,
                    resource: wgpu::BindingResource::TextureView(view),
                });
            }
            entries.push(wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(&sampler),
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("authored material maps"),
                layout: &material_layout,
                entries: &entries,
            });
            materials.push(Material {
                bind,
                _uniform: uniform,
                transparent: source.parameters.surface[2] > 0.
                    || source.parameters.surface[3] == -2.,
                double_sided: source.double_sided,
                masked: source.parameters.surface[3] >= 0.,
            });
        }
        textures.extend(defaults);
        let camera = buffer(
            device,
            "archive frame camera",
            bytemuck::bytes_of(&Camera::zeroed()),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        )?;
        if lights.len() > 128 {
            return Err(crate::ComputeError::Validation(
                "archive light uniform capacity exceeded; no lights dropped".into(),
            ));
        }
        let mut light_data = vec![0u8; 16 + 128 * std::mem::size_of::<ArchiveLight>()];
        light_data[..4].copy_from_slice(&(lights.len() as u32).to_le_bytes());
        light_data[16..16 + std::mem::size_of_val(lights)]
            .copy_from_slice(bytemuck::cast_slice(lights));
        let light_buffer = buffer(
            device,
            "unchanged authored punctual light constants",
            &light_data,
            wgpu::BufferUsages::UNIFORM,
        )?;
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });
        let meshes = geometry
            .into_iter()
            .filter(|s| !s.instances.is_empty())
            .map(|s| {
                let mut low = Vec3::splat(f32::INFINITY);
                let mut high = Vec3::splat(f32::NEG_INFINITY);
                for v in &s.vertices {
                    let p = Vec3::from_array(v.position);
                    low = low.min(p);
                    high = high.max(p);
                }
                let centre = (low + high) * 0.5;
                let radius = (high - low).length() * 0.5;
                let spheres: Vec<(Vec3,f32)> = s
                    .instances
                    .iter()
                    .map(|instance| {
                        let m = Mat4::from_cols_array_2d(&instance.world);
                        let scale = (m.x_axis.truncate().length_squared()
                            + m.y_axis.truncate().length_squared()
                            + m.z_axis.truncate().length_squared())
                        .sqrt();
                        let c = m.transform_point3(centre);
                        (
                            c,
                            radius * scale
                                + 64.
                                    * f32::EPSILON
                                    * (1. + c.abs().max_element() + radius * scale),
                        )
                    })
                    .collect();
                let instances = buffer(device,"scene instances",bytemuck::cast_slice(&s.instances),wgpu::BufferUsages::VERTEX|wgpu::BufferUsages::STORAGE)?;
                let visible_instances = buffer(device,"visible authored instances",bytemuck::cast_slice(&s.instances),wgpu::BufferUsages::VERTEX|wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::COPY_SRC)?;
                let culling = if std::env::var_os("VOXY_STREET_GPU_CULLING").is_some() && s.instances.len()>=1024 && !materials[s.material].transparent {
                    Some(crate::ArchiveInstanceCulling::new(device,&instances,&visible_instances,&spheres,s.indices.len() as u32)?)
                } else {None};
                Ok(Mesh {
                    culling,
                    frustum: crate::archive_frustum::ArchiveFrustumGroups::new(&spheres),
                    scratch_ids: Vec::with_capacity(s.instances.len()),
                    scratch_instances: Vec::new(),
                    vertices: buffer(
                        device,
                        "authored vertices",
                        bytemuck::cast_slice(&s.vertices),
                        wgpu::BufferUsages::VERTEX,
                    )?,
                    indices: buffer(
                        device,
                        "authored indices",
                        bytemuck::cast_slice(&s.indices),
                        wgpu::BufferUsages::INDEX,
                    )?,
                    instances,
                    visible_instances,
                    visible_ids: (0..s.instances.len()).collect(),
                    visible_count: s.instances.len() as u32,
                    source_instances: s.instances.clone(),
                    spheres,
                    count: s.indices.len() as u32,
                    copies: s.instances.len() as u32,
                    material: s.material,
                    center: s.center,
                })
            })
            .collect::<Result<Vec<_>, crate::ComputeError>>()?;
        let centre = (bounds.0 + bounds.1) * 0.5;
        let extent = (bounds.1 - bounds.0).length() * 0.5;
        let shadow_matrix =
            Mat4::orthographic_rh(-extent, extent, -extent, extent, 0.1, extent * 4.)
                * Mat4::look_at_rh(centre + sun * extent * 2., centre, Vec3::Y);
        let hdr = target(device, width, height)?;
        let background = target(device, width, height)?;
        let depth_texture = depth(device, width, height);
        let history_depth=ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
            label:Some("opaque radiance history depth"),size:wgpu::Extent3d {width,height,depth_or_array_layers:1},
            mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::Depth32Float,
            usage:wgpu::TextureUsages::TEXTURE_BINDING|wgpu::TextureUsages::COPY_DST,view_formats:&[],
        })?;
        let shadow = depth(device, 8192, 8192);
        let comparison = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        // Irradiance probes start disabled (origin.w = 0) until baked.
        let probe_sh = buffer(
            device,
            "irradiance probe SH",
            &vec![0; crate::probe_gi::MAX_PROBES * crate::probe_gi::PROBE_BYTES as usize],
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        )?;
        let probe_grid = buffer(
            device,
            "irradiance probe grid",
            &[0; std::mem::size_of::<crate::probe_gi::ProbeGrid>()],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        )?;
        let frame_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &background.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &shadow.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&comparison),
                },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&depth_texture.create_view(&Default::default())) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&history_depth.create_view(&Default::default())) },
                wgpu::BindGroupEntry { binding: 5, resource: probe_sh.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: probe_grid.as_entire_binding() },
            ],
        });
        let hdr_output = format == wgpu::TextureFormat::Rgba16Float;
        let display_parameters = buffer(
            device,
            "street HDR display parameters",
            bytemuck::cast_slice(&[display_mode(hdr_output, ArchiveToneMapper::default()), 1., 1., 0.]),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        )?;
        let bloom = crate::bloom::Bloom::new(device, width, height)?;
        let post_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &hdr.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: display_parameters.as_entire_binding(),
                },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&depth_texture.create_view(&Default::default())) },
                wgpu::BindGroupEntry { binding: 3, resource: camera.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&sky.create_view(&Default::default())) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&sky_sampler) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&bloom.output()) },
            ],
        });
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            meshes,
            materials,
            _textures: textures,
            _lights: light_buffer,
            camera,
            camera_bind,
            opaque,
            opaque_double,
            masked,
            masked_double,
            transparent,
            transparent_double,
            shadow_pipeline,
            depth_pipeline,
            depth_double,
            timing,
            gpu_times_ms: Vec::new(),
            gpu_shading_times_ms: Vec::new(),
            exposure: std::env::var("VOXY_STREET_EXPOSURE")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite() && *v > 0.)
                .unwrap_or(if environment_enabled { 1. / 600. } else { 1. / 1200. }),
            sky_strength: 4.,
            _environment: environment,
            environment_bindings,
            environment_enabled,
            _sky: sky,
            post,
            post_linear,
            output_format: format,
            antialiasing: crate::ArchiveAntialiasing::Spatial,
            temporal: None,
            post_bind,
            bloom,
            display_parameters,
            hdr_output,
            linear_output: false,
            vertical_fov: 65_f32.to_radians(),
            camera_up: Vec3::Y,
            tone_mapper: ArchiveToneMapper::default(),
            camera_lens: Default::default(),
            display_headroom: 1.,
            spatial_antialiasing: true,
            frame_bind,
            probe_sh,
            probe_grid,
            hdr,
            background,
            depth: depth_texture,
            history_depth,
            shadow,
            shadow_matrix,
            weather: Default::default(),
            weather_time: 0.,
            surface_wetness: None,
            puddle_coverage: 0.45,
            previous_vp: None,
            shadow_bounds: bounds,
            sun,
            sun_colour,
            width,
            height,
            shadow_ready: false,
        })
    }
    /// Update atmosphere without rebuilding geometry or allocating particles.
    pub fn set_weather(&mut self, weather: crate::weather::Weather) -> Result<(), crate::ComputeError> {
        if !weather.is_valid() {return Err(crate::ComputeError::Validation("invalid weather".into()));}
        if self.weather != weather { self.reset_temporal_history(); }
        self.weather = weather;
        Ok(())
    }
    /// Override ground wetness in [0, 1]; `None` follows `Weather::rain`.
    pub fn set_surface_wetness(&mut self, wetness: Option<f32>) -> Result<(), crate::ComputeError> {
        if wetness.is_some_and(|w| !w.is_finite() || !(0. ..=1.).contains(&w)) {
            return Err(crate::ComputeError::Validation("invalid surface wetness".into()));
        }
        self.surface_wetness = wetness;
        Ok(())
    }
    /// Fraction of fully wet upward-facing ground that pools into flat puddles.
    pub fn set_puddle_coverage(&mut self, coverage: f32) -> Result<(), crate::ComputeError> {
        if !coverage.is_finite() || !(0. ..=1.).contains(&coverage) {
            return Err(crate::ComputeError::Validation("invalid puddle coverage".into()));
        }
        self.puddle_coverage = coverage;
        Ok(())
    }
    pub fn surface_wetness(&self) -> f32 {self.surface_wetness.unwrap_or(self.weather.rain)}
    pub fn set_weather_time(&mut self, seconds: f32) {
        if seconds.is_finite() { self.weather_time = seconds.rem_euclid(4096.); }
    }
    /// Direction points toward the sun. Rebuild the matching directional shadow map.
    pub fn set_sun(&mut self, direction: Vec3, colour: [f32; 4]) -> Result<(), crate::ComputeError> {
        if !direction.is_finite() || direction.length_squared() < 1e-8 || colour.iter().any(|v| !v.is_finite() || *v < 0.) {
            return Err(crate::ComputeError::Validation("invalid sun".into()));
        }
        self.sun = direction.normalize();
        self.sun_colour = colour;
        let centre = (self.shadow_bounds.0 + self.shadow_bounds.1) * 0.5;
        let extent = ((self.shadow_bounds.1 - self.shadow_bounds.0).length() * 0.5).max(1.);
        let up = if self.sun.dot(Vec3::Y).abs() > 0.99 {Vec3::Z} else {Vec3::Y};
        self.shadow_matrix = Mat4::orthographic_rh(-extent, extent, -extent, extent, 0.1, extent * 4.)
            * Mat4::look_at_rh(centre + self.sun * extent * 2., centre, up);
        self.shadow_ready = false;
        self.reset_temporal_history();
        Ok(())
    }
    /// Linear extended-sRGB output uses 1.0 as SDR reference white. No invented nits.
    /// Write scene-linear HDR into an `Rgba16Float` target instead of the
    /// display transform, for an external camera post-process such as
    /// [`crate::BodycamPostProcess`].
    pub fn set_linear_output(&mut self, linear: bool) {
        if self.linear_output != linear {
            self.linear_output = linear;
            self.write_display_parameters();
        }
    }
    /// Vertical field of view and camera up vector (roll) used by [`Self::draw`].
    pub fn set_camera_projection(&mut self, vertical_fov_radians: f32, up: Vec3) {
        if vertical_fov_radians.is_finite() && vertical_fov_radians > 0. && up.is_finite() {
            self.vertical_fov = vertical_fov_radians.clamp(1_f32.to_radians(), 170_f32.to_radians());
            self.camera_up = up.normalize_or(Vec3::Y);
        }
    }
    fn output_mode(&self) -> f32 {
        if self.linear_output { 3. } else { display_mode(self.hdr_output, self.tone_mapper) }
    }
    pub fn set_display_headroom(&mut self, headroom: f32) -> Result<(), crate::ComputeError> {
        if !headroom.is_finite() || headroom < 1. {
            return Err(crate::ComputeError::Validation(
                "invalid live EDR headroom".into(),
            ));
        }
        self.display_headroom = headroom;
        self.write_display_parameters();
        Ok(())
    }
    /// Photometric exposure from EV100 (Lagarde & de Rousiers 2014):
    /// exposure = 1 / (1.2 * 2^EV100). Lights are in lux / candela.
    pub fn set_exposure_ev100(&mut self, ev100: f32) -> Result<(), crate::ComputeError> {
        if !ev100.is_finite() {
            return Err(crate::ComputeError::Validation("invalid EV100".into()));
        }
        self.exposure = exposure_from_ev100(ev100);
        self.reset_temporal_history();
        Ok(())
    }
    /// Physical camera: f-number, shutter time in seconds and ISO sensitivity.
    pub fn set_camera_exposure(&mut self, aperture: f32, shutter_s: f32, iso: f32) -> Result<(), crate::ComputeError> {
        if !(aperture > 0. && shutter_s > 0. && iso > 0.) {
            return Err(crate::ComputeError::Validation("invalid camera exposure".into()));
        }
        self.set_exposure_ev100(ev100_from_camera(aperture, shutter_s, iso))
    }
    pub fn exposure_ev100(&self) -> f32 {(1. / (1.2 * self.exposure)).log2()}
    /// Thin-lens depth of field and shutter motion blur, applied in the
    /// temporal display pass (Temporal/MetalFx antialiasing only).
    pub fn set_camera_lens(&mut self, lens: crate::archive_temporal::ArchiveCameraLens) -> Result<(), crate::ComputeError> {
        let values = [lens.focal_length_m, lens.f_number, lens.focus_distance_m, lens.shutter_fraction, lens.sensor_width_m];
        if values.iter().any(|v| !v.is_finite() || *v < 0.) {
            return Err(crate::ComputeError::Validation("invalid camera lens".into()));
        }
        self.camera_lens = lens;
        if let Some(temporal) = &mut self.temporal {temporal.lens = lens;}
        Ok(())
    }
    /// Bake order-1 SH irradiance probes over `min..=max` by rendering the
    /// scene into six 90-degree faces per probe and projecting them on the
    /// GPU. Probes then replace the distant environment's diffuse term inside
    /// the grid (screen-space GI still refines on-screen bounces). Static
    /// scenes need one bake; rebake after lighting changes. Returns the count.
    pub fn bake_irradiance_probes(&mut self, min: Vec3, max: Vec3, spacing: f32) -> Result<usize, crate::ComputeError> {
        let (grid, points) = crate::probe_gi::grid(min, max, spacing)
            .ok_or_else(|| crate::ComputeError::Validation("invalid probe grid".into()))?;
        let face = ComputeMemoryBudget::for_device(&self.device).allocate_texture(&wgpu::TextureDescriptor {
            label: Some("irradiance probe face"),
            size: wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })?;
        let view = face.create_view(&Default::default());
        let projector = crate::probe_gi::ProbeProjector::new(&self.device);
        let saved = (self.antialiasing, self.linear_output, self.vertical_fov, self.camera_up);
        // Probes see the scene without themselves, history or temporal AA.
        self.queue.write_buffer(&self.probe_grid, 0, &[0; std::mem::size_of::<crate::probe_gi::ProbeGrid>()]);
        self.set_antialiasing(crate::ArchiveAntialiasing::Off)?;
        self.set_linear_output(true);
        let faces = [
            (Vec3::X, Vec3::Y), (Vec3::NEG_X, Vec3::Y),
            (Vec3::Z, Vec3::Y), (Vec3::NEG_Z, Vec3::Y),
            (Vec3::Y, Vec3::NEG_Z), (Vec3::NEG_Y, Vec3::Z),
        ];
        for (index, &point) in points.iter().enumerate() {
            for (face_index, &(forward, up)) in faces.iter().enumerate() {
                self.set_camera_projection(90_f32.to_radians(), up);
                self.previous_vp = None;
                self.draw(&view, point, forward);
                let right = forward.cross(up).normalize();
                projector.project(
                    &self.device, &self.queue, &view, (self.width, self.height), &self.probe_sh,
                    index, face_index, [forward, right, up], 1. / self.exposure,
                );
            }
        }
        self.queue.write_buffer(&self.probe_grid, 0, bytemuck::bytes_of(&grid));
        self.set_linear_output(saved.1);
        self.set_camera_projection(saved.2, saved.3);
        self.set_antialiasing(saved.0)?;
        self.previous_vp = None;
        self.reset_temporal_history();
        Ok(points.len())
    }
    /// SDR display transform. HDR output keeps its hue-preserving highlight knee.
    pub fn set_tone_mapper(&mut self, tone_mapper: ArchiveToneMapper) {
        self.tone_mapper = tone_mapper;
        self.write_display_parameters();
    }
    pub fn tone_mapper(&self) -> ArchiveToneMapper {self.tone_mapper}
    /// Directional spatial AA, enabled by default on all backends including Metal.
    /// Disable for native-pixel comparisons or when applying an external AA pass.
    pub fn set_spatial_antialiasing(&mut self, enabled: bool) {
        self.temporal = None;
        self.antialiasing = if enabled {crate::ArchiveAntialiasing::Spatial} else {crate::ArchiveAntialiasing::Off};
        self.spatial_antialiasing = enabled;
        self.write_display_parameters();
    }
    /// Select temporal AA explicitly. Unsupported MetalFX returns an error and
    /// preserves the current mode; it never silently falls back to spatial AA.
    pub fn set_antialiasing(&mut self, mode: crate::ArchiveAntialiasing) -> Result<(), crate::ComputeError> {
        if mode == self.antialiasing {return Ok(());}
        let temporal = if matches!(mode, crate::ArchiveAntialiasing::Temporal | crate::ArchiveAntialiasing::MetalFx) {
            Some(crate::archive_temporal::ArchiveTemporal::new(&self.device,self.width,self.height,self.output_format,mode)?)
        } else {None};
        self.temporal = temporal.map(|mut t| {t.lens = self.camera_lens; t});
        self.antialiasing = mode;
        self.spatial_antialiasing = mode == crate::ArchiveAntialiasing::Spatial;
        self.write_display_parameters();
        Ok(())
    }
    /// Diagnostic previous-UV minus current-UV camera motion, excluding jitter.
    /// Valid only after drawing a temporal frame; immutable polygon meshes only.
    pub fn temporal_motion_texture(&self) -> Option<&wgpu::Texture> {
        self.temporal.as_ref().map(|t| t.motion_texture())
    }
    pub fn antialiasing(&self) -> crate::ArchiveAntialiasing {self.antialiasing}
    /// Call on camera cuts or changes to scene lighting/materials/exposure.
    pub fn reset_temporal_history(&mut self) {
        if let Some(temporal) = &mut self.temporal {temporal.reset();}
        self.previous_vp = None;
    }
    fn write_display_parameters(&self) {
        self.queue.write_buffer(
            &self.display_parameters,
            0,
            bytemuck::cast_slice(&[
                self.output_mode(),
                self.display_headroom,
                f32::from(self.spatial_antialiasing),
                f32::from(self.temporal.is_some()),
            ]),
        );
    }
    /// Spatially supersampled temporal AA. The renderer dimensions must match
    /// the resolve's render size. Invalid targets leave scene/history untouched.
    pub fn draw_supersampled(
        &mut self,
        resolve: &crate::SupersampleResolve,
        output: &wgpu::TextureView,
        eye: Vec3,
        direction: Vec3,
    ) -> Result<wgpu::SubmissionIndex, crate::ComputeError> {
        resolve.validate(output)?;
        if self.device != resolve.device || (self.width, self.height) != resolve.render_size() {
            return Err(crate::ComputeError::InvalidBuffer);
        }
        let linear = self.linear_output;
        self.set_linear_output(true);
        self.draw(&resolve.view, eye, direction);
        self.set_linear_output(linear);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        resolve.encode(
            &self.queue, &mut encoder, output,
            if linear { 3. } else { display_mode(output.texture().format() == wgpu::TextureFormat::Rgba16Float, self.tone_mapper) },
            self.display_headroom,
        );
        Ok(self.queue.submit([encoder.finish()]))
    }
    pub fn draw(
        &mut self,
        output: &wgpu::TextureView,
        eye: Vec3,
        direction: Vec3,
    ) -> wgpu::SubmissionIndex {
        if let Some(timing) = &mut self.timing {
            if let Some(ms) = timing.collect(self.queue.get_timestamp_period()) {
                self.gpu_times_ms.push(ms[0]);
                self.gpu_shading_times_ms.push([ms[1],ms[2]]);
            }
        }
        let time_this_frame = self.timing.as_ref().is_some_and(|t| !t.pending);
        let vp = Mat4::perspective_rh(
            self.vertical_fov,
            self.width as f32 / self.height as f32,
            0.05,
            1000.,
        ) * Mat4::look_at_rh(eye, eye + direction, self.camera_up);
        let rows = vp.transpose().to_cols_array_2d().map(Vec4::from_array);
        // Keep the entire temporal-jitter footprint inside the conservative
        // culling volume; otherwise edge leaves may disappear between AA samples.
        let margin = if self.temporal.is_some() { 2. } else { 0. };
        let horizontal = rows[3] * (1. + margin / self.width as f32);
        let vertical = rows[3] * (1. + margin / self.height as f32);
        let planes = [
            horizontal + rows[0],
            horizontal - rows[0],
            vertical + rows[1],
            vertical - rows[1],
            rows[2],
            rows[3] - rows[2],
        ];
        let raw_planes=planes;
        let planes = planes.map(|p| (p, p.truncate().length()));
        for mesh in &mut self.meshes {
            if let Some(culling)=&mesh.culling {culling.prepare(&self.queue,raw_planes);mesh.visible_count=mesh.copies;continue;}
            mesh.frustum.collect(&planes,&mut mesh.scratch_ids);
            let ids=&mut mesh.scratch_ids;
            if self.materials[mesh.material].transparent {
                ids.sort_by(|&a, &b| {
                    mesh.spheres[b]
                        .0
                        .distance_squared(eye)
                        .total_cmp(&mesh.spheres[a].0.distance_squared(eye))
                });
            }
            if *ids != mesh.visible_ids {
                if !ids.is_empty() {
                    mesh.scratch_instances.clear();
                    mesh.scratch_instances.extend(ids.iter().map(|&i|mesh.source_instances[i]));
                    self.queue.write_buffer(
                        &mesh.visible_instances,
                        0,
                        bytemuck::cast_slice(&mesh.scratch_instances),
                    );
                }
                mesh.visible_count = ids.len() as u32;
                std::mem::swap(&mut mesh.visible_ids,&mut mesh.scratch_ids);
            }
        }
        let mut jittered_vp = vp;
        let output_mode = self.output_mode();
        if let Some(temporal) = &mut self.temporal {
            let jitter = temporal.jitter();
            let offset = glam::Vec2::new(2. * jitter.x / self.width as f32, -2. * jitter.y / self.height as f32);
            jittered_vp = Mat4::from_translation(Vec3::new(offset.x, offset.y, 0.)) * vp;
            temporal.prepare(&self.queue, vp, jittered_vp, eye, direction, output_mode, self.display_headroom);
        }
        self.queue.write_buffer(
            &self.camera,
            0,
            bytemuck::bytes_of(&Camera {
                vp: jittered_vp.to_cols_array_2d(),
                shadow: self.shadow_matrix.to_cols_array_2d(),
                inverse_vp: jittered_vp.inverse().to_cols_array_2d(),
                eye: [eye.x, eye.y, eye.z, f32::from(self.environment_enabled)],
                sun: [self.sun.x, self.sun.y, self.sun.z, 0.],
                sun_colour: { let mut c = self.sun_colour; c[3] *= (1. - self.weather.coverage * 0.75) * (1. - 0.9 * self.weather.rain); c },
                weather: [self.weather.coverage, self.weather.density, self.weather.rain, self.weather.snow],
                atmosphere: [self.weather_time, self.weather.cloud_base, self.weather.cloud_thickness, 0.],
                wind: [self.weather.wind[0], self.weather.wind[1], self.weather.wind_gust, 0.],
                previous_vp: self.previous_vp.unwrap_or(jittered_vp).to_cols_array_2d(),
                previous_inverse_vp: self.previous_vp.unwrap_or(jittered_vp).inverse().to_cols_array_2d(),
                surface: [self.surface_wetness(), self.puddle_coverage, f32::from(u8::from(self.previous_vp.is_some())), 2.*(self.vertical_fov*0.5).tan()/self.height as f32],
                viewport: [
                    self.width as f32,
                    self.height as f32,
                    self.exposure,
                    self.sky_strength * twilight_sky(self.sun.y),
                ],
            }),
        );
        // `background` receives this frame's opaque radiance before transparents;
        // the next frame's opaque reflections reproject into it.
        self.previous_vp = Some(jittered_vp);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for mesh in &self.meshes {if let Some(culling)=&mesh.culling {culling.encode(&mut encoder);}}
        if !self.shadow_ready {
            let shadow_view = self.shadow.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("static authored sun shadows"),
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
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.camera_bind, &[]);
            for mesh in &self.meshes {
                if self.materials[mesh.material].transparent {
                    continue;
                }
                pass.set_bind_group(1, &self.materials[mesh.material].bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.count, 0, 0..mesh.copies);
            }
            drop(pass);
            self.shadow_ready = true;
        }
        let hdr_view = self.hdr.create_view(&Default::default());
        let depth_view = self.depth.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("exact opaque visibility prepass"),
                color_attachments: &[],
                timestamp_writes: if time_this_frame {
                    self.timing
                        .as_ref()
                        .map(|t| wgpu::RenderPassTimestampWrites {
                            query_set: &t.set,
                            beginning_of_pass_write_index: Some(0),
                            end_of_pass_write_index: None,
                        })
                } else {
                    None
                },
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.camera_bind, &[]);
            for mesh in &self.meshes {
                let material = &self.materials[mesh.material];
                if material.transparent || mesh.visible_count == 0 {
                    continue;
                }
                pass.set_pipeline(if material.double_sided {
                    &self.depth_double
                } else {
                    &self.depth_pipeline
                });
                pass.set_bind_group(1, &material.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.visible_instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(culling)=&mesh.culling {pass.draw_indexed_indirect(culling.indirect(),0);}else{pass.draw_indexed(0..mesh.count, 0, 0..mesh.visible_count);}
            }
        }
        for transparent in [false, true] {
            if transparent {
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.hdr,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.background,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: self.width,
                        height: self.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if transparent {
                    "authored transparent surfaces"
                } else {
                    "authored opaque surfaces"
                }),
                timestamp_writes: if time_this_frame { self.timing.as_ref().map(|t| wgpu::RenderPassTimestampWrites {
                    query_set: &t.set,
                    beginning_of_pass_write_index: Some(if transparent {3} else {1}),
                    end_of_pass_write_index: Some(if transparent {4} else {2}),
                }) } else {None},
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &hdr_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if transparent {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.22,
                                g: 0.35,
                                b: 0.55,
                                a: 1.,
                            })
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.camera_bind, &[]);
            pass.set_bind_group(2, &self.frame_bind, &[]);
            pass.set_bind_group(3, &self.environment_bindings.group, &[]);
            let mut order: Vec<_> = self
                .meshes
                .iter()
                .filter(|m| {
                    m.visible_count > 0 && self.materials[m.material].transparent == transparent
                })
                .collect();
            if transparent {
                order.sort_by(|a, b| {
                    b.center
                        .distance_squared(eye)
                        .total_cmp(&a.center.distance_squared(eye))
                });
            }
            for mesh in order {
                let material = &self.materials[mesh.material];
                pass.set_pipeline(
                    match (transparent, material.double_sided, material.masked) {
                        (false, false, false) => &self.opaque,
                        (false, false, true) => &self.masked,
                        (false, true, false) => &self.opaque_double,
                        (false, true, true) => &self.masked_double,
                        (true, false, _) => &self.transparent,
                        (true, true, _) => &self.transparent_double,
                    },
                );
                pass.set_bind_group(1, &material.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.visible_instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(culling)=&mesh.culling {pass.draw_indexed_indirect(culling.indirect(),0);}else{pass.draw_indexed(0..mesh.count, 0, 0..mesh.visible_count);}
            }
        }
        let temporal_view = self.temporal.as_ref().map(|t| t.current.create_view(&Default::default()));
        // Without temporal AA the display pass applies bloom itself; the
        // temporal display pass has its own chain over the resolved image.
        if self.temporal.is_none() && !self.linear_output {
            self.bloom.encode(&self.device, &mut encoder, &self.hdr.create_view(&Default::default()));
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("street HDR display"),
                timestamp_writes: if time_this_frame && self.temporal.is_none() {
                    self.timing
                        .as_ref()
                        .map(|t| wgpu::RenderPassTimestampWrites {
                            query_set: &t.set,
                            beginning_of_pass_write_index: None,
                            end_of_pass_write_index: Some(5),
                        })
                } else {
                    None
                },
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: temporal_view.as_ref().unwrap_or(output),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(if self.temporal.is_some() || self.linear_output {&self.post_linear} else {&self.post});
            pass.set_bind_group(0, &self.post_bind, &[]);
            pass.set_bind_group(1, &self.environment_bindings.group, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut preceding_commands = Vec::new();
        if let Some(temporal) = &mut self.temporal {
            let timestamps = if time_this_frame {self.timing.as_ref().map(|t| wgpu::RenderPassTimestampWrites {
                query_set: &t.set, beginning_of_pass_write_index: None, end_of_pass_write_index: Some(5),
            })} else {None};
            temporal.encode_guides(&self.device, &mut encoder, &self.depth);
            #[cfg(target_os="macos")]
            if temporal.native() {
                preceding_commands.push(encoder.finish());
                let mut native = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {label:Some("native MetalFX only")});
                temporal.encode_native(&mut native, &self.depth);
                preceding_commands.push(native.finish());
                encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {label:Some("MetalFX final HDR display")});
            }
            temporal.encode_display(&self.device, &mut encoder, output, timestamps);
        }
        if time_this_frame {
            let timing = self.timing.as_ref().unwrap();
            encoder.resolve_query_set(&timing.set, 0..6, &timing.resolve, 0);
            encoder.copy_buffer_to_buffer(&timing.resolve, 0, &timing.readback, 0, 48);
        }
        // Save depth only after all old-radiance reads. Both histories then
        // describe the same opaque frame and use the same previous camera.
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {texture:&self.depth,mip_level:0,origin:wgpu::Origin3d::ZERO,aspect:wgpu::TextureAspect::All},
            wgpu::TexelCopyTextureInfo {texture:&self.history_depth,mip_level:0,origin:wgpu::Origin3d::ZERO,aspect:wgpu::TextureAspect::All},
            wgpu::Extent3d {width:self.width,height:self.height,depth_or_array_layers:1});
        preceding_commands.push(encoder.finish());
        let submission = self.queue.submit(preceding_commands);
        if time_this_frame {
            let timing = self.timing.as_mut().unwrap();
            timing.pending = true;
            let ready = timing.ready.clone();
            timing
                .readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    ready.store(
                        if result.is_ok() { 1 } else { 2 },
                        std::sync::atomic::Ordering::Release,
                    );
                });
        }
        submission
    }
}

#[derive(Debug)]
struct Timing {
    set: wgpu::QuerySet,
    resolve: ComputeStorage,
    readback: ComputeStorage,
    ready: Arc<std::sync::atomic::AtomicU8>,
    pending: bool,
}
impl Timing {
    fn new(device: &wgpu::Device) -> Result<Option<Self>, crate::ComputeError> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Ok(None);
        }
        let [resolve, readback] = ComputeMemoryBudget::for_device(device).allocate_buffers([
            voxy_gpu_core::ManagedBufferDescriptor {
                label: "street timestamp resolve",
                size: 256,
                contents: None,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            },
            voxy_gpu_core::ManagedBufferDescriptor {
                label: "street timestamp readback",
                size: 48,
                contents: None,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            },
        ])?;
        Ok(Some(Self {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("street GPU frame timing"),
                ty: wgpu::QueryType::Timestamp,
                count: 6,
            }),
            resolve,
            readback,
            ready: Arc::new(std::sync::atomic::AtomicU8::new(0)),
            pending: false,
        }))
    }
    fn collect(&mut self, period: f32) -> Option<[f64;3]> {
        if self.ready.swap(0, std::sync::atomic::Ordering::AcqRel) != 1 {
            return None;
        }
        let bytes = self.readback.slice(..).get_mapped_range().ok()?;
        let ticks: Vec<u64> = bytes.chunks_exact(8).map(|chunk|u64::from_le_bytes(chunk.try_into().unwrap())).collect();
        drop(bytes);
        self.readback.unmap();
        self.pending = false;
        // Unsupported/invalid timestamp results must not look like a zero-cost frame.
        if ticks.len()!=6 || !period.is_finite() || period<=0. || ticks.windows(2).any(|w|w[1]<w[0]) || ticks[5]==ticks[0] {return None;}
        Some([ticks[5]-ticks[0],ticks[2]-ticks[1],ticks[4]-ticks[3]].map(|ticks|ticks as f64 * f64::from(period) / 1e6))
    }
}

/// SDR tone mapping curve applied by the archive display pass.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ArchiveToneMapper {
    /// Narkowicz ACES fit: punchy contrast, skews saturated highlights.
    Aces,
    /// AgX base: desaturates toward white in highlights like film.
    #[default]
    Agx,
}

/// Mode selector read by `display_transform` in display_transform.wgsl.
fn display_mode(hdr_output: bool, tone_mapper: ArchiveToneMapper) -> f32 {
    match (hdr_output, tone_mapper) {
        (true, _) => 1.,
        (false, ArchiveToneMapper::Aces) => 0.,
        (false, ArchiveToneMapper::Agx) => 2.,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_shader_validates() {
        let source = concat!(include_str!("bsdf.wgsl"), include_str!("archive_environment.wgsl"), include_str!("soft_shadow.wgsl"), include_str!("probe_sampling.wgsl"), include_str!("archive_history.wgsl"), include_str!("archive_scene.wgsl"));
        let module = naga::front::wgsl::parse_str(source).unwrap();
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap();
    }
    #[test]
    fn camera_layout_matches_shader() {
        // 5 matrices and 8 vectors, as declared by archive_scene.wgsl.
        assert_eq!(std::mem::size_of::<Camera>(), 5 * 64 + 8 * 16);
    }
    #[test]
    fn reflective_presets_route_to_expected_passes() {
        assert_eq!(ArchiveMaterial::MIRROR.surface[2], 0.);
        assert!(ArchiveMaterial::GLASS.surface[2] > 0.);
        assert_eq!(ArchiveMaterial::GLASS.surface[3], -2.);
    }
}

/// Analytic-sky ambient relative to daylight: 1 with the sun 3 degrees up or
/// higher, falling log-linearly to 1e-4 at the end of nautical twilight.
fn twilight_sky(sun_y: f32) -> f32 {
    let elevation = sun_y.clamp(-1., 1.).asin().to_degrees();
    10_f32.powf(-4. * ((3. - elevation) / 15.).clamp(0., 1.))
}
/// Saturation-based exposure: maximum luminance 1.2 * 2^EV100 maps to 1.
pub fn exposure_from_ev100(ev100: f32) -> f32 {
    1. / (1.2 * 2_f32.powf(ev100))
}

/// EV100 = log2(N^2 / t * 100 / S).
pub fn ev100_from_camera(aperture: f32, shutter_s: f32, iso: f32) -> f32 {
    (aperture * aperture / shutter_s * 100. / iso).log2()
}

#[cfg(test)]
mod exposure_tests {
    use super::*;

    #[test]
    fn sunny_sixteen_is_about_ev15() {
        let ev = ev100_from_camera(16., 1. / 100., 100.);
        assert!((ev - 14.64).abs() < 0.01, "{ev}");
    }

    #[test]
    fn ev100_round_trips_exposure() {
        for ev in [-2., 0., 8.97, 15.] {
            let e = exposure_from_ev100(ev);
            assert!(((1. / (1.2 * e)).log2() - ev).abs() < 1e-4);
        }
    }

    #[test]
    fn display_mode_selects_transform() {
        assert_eq!(display_mode(true, ArchiveToneMapper::Agx), 1.);
        assert_eq!(display_mode(false, ArchiveToneMapper::Aces), 0.);
        assert_eq!(display_mode(false, ArchiveToneMapper::Agx), 2.);
    }
}
