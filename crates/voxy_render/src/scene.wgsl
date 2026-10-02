struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = transform.mvp * vec4<f32>(position, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(image, image_sampler, in.uv).a * in.color.a;
    if alpha <= 0.0 { discard; }
    return textureSample(image, image_sampler, in.uv) * in.color;
}
