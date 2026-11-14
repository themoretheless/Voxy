// #include "fullscreen.wgsl"
// #include "tonemap.wgsl"
override TONE_MAPPING: bool = false;
override LINEAR_EXPOSURE: bool = false;
override OUTPUT_MAX: f32 = 65504.0;
@group(0) @binding(2) var<uniform> auto_exposure: vec4<f32>;
@group(0) @binding(1) var<uniform> display_parameter: vec4<f32>;
@group(0) @binding(0) var source: texture_2d<f32>;
@fragment fn fs_main(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = textureDimensions(source);
    let pixel = clamp(vec2<i32>(input.uv * vec2<f32>(size)), vec2<i32>(0), vec2<i32>(size) - vec2<i32>(1));
    let color = textureLoad(source, pixel, 0);
    let exposure = min(display_parameter.x * auto_exposure.x, 3.402823e38);
    if LINEAR_EXPOSURE {
        return vec4(min(max(color.rgb, vec3(0.0)) * exposure, vec3(OUTPUT_MAX)), color.a);
    }
    if TONE_MAPPING {
        return vec4<f32>(tonemap_reinhard_safe(color.rgb, exposure), color.a);
    }
    return color;
}
