//! GPU simulation: the FFT wave field and the interactive ripple field. Both use only
//! WebGPU-baseline compute (storage buffers and `rgba16float` storage textures), so they
//! run on desktop backends and in browsers alike.

mod ripples;
mod waves;

pub use ripples::{RippleField, RIPPLE_RES};
pub use waves::WaveSimulation;

#[cfg(test)]
pub(crate) fn waves_sources() -> [&'static str; 4] {
    [waves::EVOLVE, waves::FFT_ROWS, waves::FFT_COLS, waves::MIP_CHAIN]
}
#[cfg(test)]
pub(crate) fn ripples_source() -> &'static str {
    ripples::SHADER
}

pub(crate) fn compute_pipeline(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
}

pub(crate) fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    resources: &[wgpu::BindingResource],
) -> wgpu::BindGroup {
    let entries: Vec<_> = resources
        .iter()
        .enumerate()
        .map(|(binding, resource)| wgpu::BindGroupEntry { binding: binding as u32, resource: resource.clone() })
        .collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(label), layout, entries: &entries })
}

pub(crate) fn texture_array(device: &wgpu::Device, label: &str, size: u32, layers: u32, mips: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: layers },
        mip_level_count: mips,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

pub(crate) fn view(texture: &wgpu::Texture, mip: Option<u32>, dimension: wgpu::TextureViewDimension) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(dimension),
        base_mip_level: mip.unwrap_or(0),
        mip_level_count: mip.map(|_| 1),
        ..Default::default()
    })
}
