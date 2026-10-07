use crate::{SceneError, ShadowMap};
use glam::Mat4;
use wgpu::util::DeviceExt;
/// Fixed square percentage-closer kernels, measured in shadow-map texels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShadowFilter {
    #[default]
    Hard,
    Pcf3x3,
    Pcf5x5,
}
#[derive(Clone, Copy, Debug)]
pub struct ShadowSettings {
    pub light_from_world: Mat4,
    /// Constant conventional clip-depth comparison bias, in [0,1].
    pub bias: f32,
    pub enabled: bool,
    pub filter: ShadowFilter,
}
impl ShadowSettings {
    fn values(self) -> Result<[f32; 20], SceneError> {
        if !self.light_from_world.is_finite()
            || !self.bias.is_finite()
            || !(0.0..=1.0).contains(&self.bias)
        {
            return Err(SceneError::InvalidTransform);
        }
        let mut values = [0.0; 20];
        values[..16].copy_from_slice(&self.light_from_world.to_cols_array());
        values[16] = self.bias;
        values[17] = if self.enabled { 1.0 } else { 0.0 };
        values[18] = match self.filter {
            ShadowFilter::Hard => 0.0,
            ShadowFilter::Pcf3x3 => 1.0,
            ShadowFilter::Pcf5x5 => 2.0,
        };
        Ok(values)
    }
}
#[derive(Debug)]
pub(crate) struct ShadowBindings {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}
impl ShadowBindings {
    pub(crate) fn new(
        device: &wgpu::Device,
        map: &ShadowMap,
        settings: ShadowSettings,
    ) -> Result<Self, SceneError> {
        Self::with_visibility(device, map, settings, wgpu::ShaderStages::FRAGMENT)
    }
    pub(crate) fn with_visibility(
        device: &wgpu::Device,
        map: &ShadowMap,
        settings: ShadowSettings,
        visibility: wgpu::ShaderStages,
    ) -> Result<Self, SceneError> {
        if !map.belongs_to(device) {
            return Err(SceneError::DeviceMismatch);
        }
        let values = settings.values()?;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene shadow visibility"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(80),
                    },
                    count: None,
                },
            ],
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene shadow settings"),
            contents: bytemuck::cast_slice(&values),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene shadow inputs"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(map.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            layout,
            group,
            uniform,
        })
    }
    pub(crate) fn update(
        &self,
        queue: &wgpu::Queue,
        settings: ShadowSettings,
    ) -> Result<(), SceneError> {
        queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&settings.values()?));
        Ok(())
    }
}
pub(crate) fn shader() -> String {
    let lit = crate::TEXTURED_POINT_LIGHT_SHADER.replace(
        "* nl * in.light.w / distance2",
        "* nl * in.light.w / distance2 * shadow_visibility(in.world)",
    );
    format!("{lit}\n{SHADOW}")
}
pub(crate) const SHADOW: &str = r"
struct ShadowSettings { light_from_world: mat4x4<f32>, options: vec4<f32> }
@group(2) @binding(0) var shadow_depth: texture_depth_2d;
@group(2) @binding(1) var<uniform> shadow: ShadowSettings;
fn shadow_visibility(world: vec3<f32>) -> f32 {
    if shadow.options.y == 0.0 { return 1.0; }
    let clip = shadow.light_from_world * vec4(world,1.0);
    if clip.w <= 0.0 { return 1.0; }
    let p = clip.xyz / clip.w;
    let uv = vec2(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) || p.z < 0.0 || p.z > 1.0 { return 1.0; }
    let size = textureDimensions(shadow_depth);
    let pixel = vec2<i32>(uv * vec2<f32>(size));
    let radius = i32(clamp(shadow.options.z,0.0,2.0));
    var visibility = 0.0;
    for (var y = -radius; y <= radius; y++) {
        for (var x = -radius; x <= radius; x++) {
            let tap = pixel + vec2(x,y);
            if any(tap < vec2<i32>(0)) || any(tap >= vec2<i32>(size)) {
                visibility += 1.0;
            } else {
                let stored = textureLoad(shadow_depth,tap,0);
                visibility += select(0.0,1.0,p.z-shadow.options.x <= stored);
            }
        }
    }
    let width = f32(2 * radius + 1);
    return visibility / (width * width);
}
";
