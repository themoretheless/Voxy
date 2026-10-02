//! GLSL depth sampling fallback; native backends retain direct nearest depth samples.
pub(crate) fn shader(
    device: &wgpu::Device,
    source: &'static str,
) -> std::borrow::Cow<'static, str> {
    if device.adapter_info().backend != wgpu::Backend::Gl {
        return source.into();
    }
    let mut source = source
        .replace(
            "var depth_sampler: sampler;",
            "var depth_sampler: sampler_comparison;",
        )
        .replace(
            "textureSampleLevel(depth, depth_sampler, uv, 0)",
            "read_depth(depth, depth_sampler, uv)",
        )
        .replace(
            "textureSampleLevel(depth,depth_sampler,uv,0)",
            "read_depth(depth,depth_sampler,uv)",
        );
    source.push_str(
        r"
fn read_depth(source: texture_depth_2d, sampling: sampler_comparison, uv: vec2<f32>) -> f32 {
    // Strict Less distinguishes exact reverse-Z background zero.
    if textureSampleCompareLevel(source,sampling,uv,0.0) == 0.0 { return 0.0; }
    var low = 0.0; var high = 1.0;
    for (var step = 0; step < 24; step++) {
        let middle = (low+high)*0.5;
        if textureSampleCompareLevel(source,sampling,uv,middle) > 0.5 { low = middle; }
        else { high = middle; }
    }
    return high;
}
",
    );
    source.into()
}
pub(crate) fn sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("nearest primary depth"),
        compare: (device.adapter_info().backend == wgpu::Backend::Gl)
            .then_some(wgpu::CompareFunction::Less),
        ..Default::default()
    })
}

pub(crate) fn module(
    device: &wgpu::Device,
    label: &str,
    source: &'static str,
) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader(device, source)),
    })
}
