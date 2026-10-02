// SceneRenderer ABI. light stores a normalized world-space clipping plane.
struct Transform {
    mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, eye: vec4<f32>,
    world: mat4x4<f32>, tint: vec4<f32>, plane: vec4<f32>, authored: vec4<f32>,
}
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) distance: f32,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = transform.mvp * vec4(position, 1.0);
    out.uv = uv;
    out.color = color * transform.tint;
    out.distance = dot(transform.plane, transform.world * vec4(position, 1.0));
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSampleLevel(image, image_sampler, in.uv, 0.0) * in.color;
    if in.distance < 0.0 || texel.a <= 0.0 { discard; }
    return texel;
}
