// SceneRenderer prefix ABI; second matrix is capture MVP, not motion history.
struct Transform { mvp: mat4x4<f32>, capture_mvp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) capture_clip: vec4<f32>,
    @location(1) color: vec4<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(2) color: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip = transform.mvp * vec4(position, 1.0);
    out.capture_clip = transform.capture_mvp * vec4(position, 1.0);
    out.color = color;
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.capture_clip.w <= 0.0 { discard; }
    let ndc = in.capture_clip.xyz / in.capture_clip.w;
    if any(ndc.xy < vec2(-1.0)) || any(ndc.xy > vec2(1.0)) || ndc.z < 0.0 || ndc.z > 1.0 { discard; }
    let uv = ndc.xy * vec2(0.5, -0.5) + vec2(0.5);
    return textureSampleLevel(image, image_sampler, uv, 0.0) * in.color;
}
