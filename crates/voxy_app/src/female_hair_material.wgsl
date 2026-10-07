// Exact material for the opaque prelit fibre tag (-7, 0): clamp-edge texel times colour.
struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, camera: vec4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct Output { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32> }
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(2) color: vec4<f32>) -> Output {
    var out: Output;
    out.position = transform.mvp * vec4<f32>(position,1.0);
    out.color = color;
    return out;
}
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
    return textureSampleLevel(image,image_sampler,vec2<f32>(0.0),0.0)*in.color;
}
