// Display-referred BT.709 -> BT.2020, D65; inverse ST 2084 EOTF (BT.2100).
// #include "fullscreen.wgsl"
@group(0) @binding(2) var<uniform> auto_exposure: vec4<f32>;
@group(0) @binding(1) var<uniform> display_parameter: vec4<f32>;
@group(0) @binding(0) var source: texture_2d<f32>;
@fragment fn fs_main(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = textureDimensions(source);
    let pixel = clamp(vec2<i32>(input.uv * vec2<f32>(size)), vec2<i32>(0), vec2<i32>(size) - vec2<i32>(1));
    let color = textureLoad(source, pixel, 0);
    let gamut = mat3x3<f32>(
        vec3(0.6274039, 0.0690973, 0.0163914),
        vec3(0.3292830, 0.9195404, 0.0880133),
        vec3(0.0433131, 0.0113623, 0.8955953)
    );
    let luminance = clamp(gamut * (max(color.rgb, vec3(0.0)) * (display_parameter.x / 10000.0) * auto_exposure.x), vec3(0.0), vec3(1.0));
    let p = pow(luminance, vec3(2610.0 / 16384.0));
    let pq = pow((vec3(3424.0 / 4096.0) + (2413.0 / 128.0) * p) / (vec3(1.0) + (2392.0 / 128.0) * p), vec3(2523.0 / 32.0));
    return vec4(pq, color.a);
}
