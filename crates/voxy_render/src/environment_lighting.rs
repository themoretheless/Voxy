use crate::dfg::DfgSampling;
use crate::{GgxDfgLut, GgxEnvironmentPrefilter, SceneError};
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub(crate) struct EnvironmentBindings {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) group: wgpu::BindGroup,
    pub(crate) diffuse: bool,
    pub(crate) sampling: DfgSampling,
    pub(crate) intensity: f32,
    pub(crate) intensity_buffer: wgpu::Buffer,
}
impl EnvironmentBindings {
    pub(crate) fn new(
        device: &wgpu::Device,
        environment: &GgxEnvironmentPrefilter,
        dfg: &GgxDfgLut,
        diffuse: Option<&crate::DiffuseEnvironmentConvolution>,
        intensity: f32,
        retained_intensity: Option<&wgpu::Buffer>,
    ) -> Result<Self, SceneError> {
        if !environment.belongs_to(device)
            || !dfg.belongs_to(device)
            || diffuse.is_some_and(|d| !d.belongs_to(device))
        {
            return Err(SceneError::DeviceMismatch);
        }
        let texture = |binding, dimension| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float {
                    filterable: binding != 1 || !dfg.has_energy_moments(),
                },
                view_dimension: dimension,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene specular IBL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
                texture(0, wgpu::TextureViewDimension::Cube),
                texture(1, wgpu::TextureViewDimension::D2),
                texture(3, wgpu::TextureViewDimension::Cube),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let intensity_buffer = retained_intensity.cloned().unwrap_or_else(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("environment intensity"),
                contents: bytemuck::cast_slice(&[intensity, 0.0, 0.0, 0.0]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("retained scene IBL"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: intensity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(diffuse.map_or(
                        environment.view(),
                        crate::DiffuseEnvironmentConvolution::view,
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(environment.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(dfg.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Ok(Self {
            layout,
            group,
            diffuse: diffuse.is_some(),
            sampling: dfg.sampling(),
            intensity,
            intensity_buffer,
        })
    }
}
pub(crate) fn shader_with_environment(environment: Option<&EnvironmentBindings>) -> String {
    environment.map_or_else(crate::shadow_visibility::shader, |e| {
        shader_combined_profile(true, e.diffuse, e.sampling)
    })
}
pub(crate) fn shader_for_dfg(shadows: bool, diffuse: bool, dfg: &GgxDfgLut) -> String {
    shader_combined_profile(shadows, diffuse, dfg.sampling())
}

fn shader_combined_profile(shadows: bool, diffuse: bool, mode: DfgSampling) -> String {
    let source = if shadows {
        crate::shadow_visibility::shader()
    } else {
        crate::TEXTURED_POINT_LIGHT_SHADER.to_owned()
    };
    let source = source.replace(
        "+ base * 0.02, texel.a)",
        "+ base * 0.02 + environment_specular(n, v, material_nv, material_roughness, f0), texel.a)",
    );
    let source = if diffuse {
        source.replace("environment_specular(n, v, material_nv, material_roughness, f0)", "environment_specular(n, v, material_nv, material_roughness, f0) + environment_diffuse(n, material_nv, f0, base, metallic)")
    } else {
        source
    };
    let lookup = if mode == DfgSampling::EnergyPatches {
        format!(
            "{}\n{}",
            include_str!("material_energy_patch.wgsl"),
            include_str!("material_energy_atlas.wgsl")
        )
    } else {
        include_str!("material_energy_lookup.wgsl").to_owned()
    };
    let sampling = match mode {
        DfgSampling::EnergyPatches => {
            "fn sample_environment_dfg(uv:vec2<f32>)->vec2<f32>{return lookup_ggx_energy_atlas(environment_dfg,uv).rg;}"
        }
        DfgSampling::EnergyGrid => {
            "fn sample_environment_dfg(uv:vec2<f32>)->vec2<f32>{return lookup_ggx_energy(environment_dfg,uv).rg;}"
        }
        DfgSampling::Legacy => {
            "fn sample_environment_dfg(uv:vec2<f32>)->vec2<f32>{return textureSampleLevel(environment_dfg,environment_sampler,uv,0.0).rg;}"
        }
    };
    format!("{source}\n{IBL}\n{DIFFUSE}\n{lookup}\n{sampling}")
}
const IBL: &str = r"
@group(3) @binding(0) var environment_cube: texture_cube<f32>;
@group(3) @binding(1) var environment_dfg: texture_2d<f32>;
@group(3) @binding(2) var environment_sampler: sampler;
@group(3) @binding(4) var<uniform> environment_settings: vec4<f32>;
fn environment_specular(n: vec3<f32>, v: vec3<f32>, nv: f32, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let r = reflect(-v,n);
    let radiance = textureSampleLevel(environment_cube, environment_sampler, r,
        roughness*f32(textureNumLevels(environment_cube)-1u)).rgb;
    let ab = sample_environment_dfg(vec2(nv,roughness));
    return radiance*(f0*ab.x+vec3(ab.y))*environment_settings.x;
}
";

const DIFFUSE: &str = r"
@group(3) @binding(3) var diffuse_cube: texture_cube<f32>;
fn environment_diffuse(n: vec3<f32>, nv: f32, f0: vec3<f32>, base: vec3<f32>, metallic: f32) -> vec3<f32> {
    let fresnel = f0 + (vec3(1.)-f0)*pow(1.-nv,5.);
    return textureSampleLevel(diffuse_cube,environment_sampler,n,0.).rgb * base * (1.-metallic) * (vec3(1.)-fresnel) * environment_settings.x;
}
";
