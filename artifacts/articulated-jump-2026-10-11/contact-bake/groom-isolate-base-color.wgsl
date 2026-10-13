// Explicit solved material director; independent tangent stream following TressFX data flow.
// Existing two-lobe lighting model is retained; this is not full Marschner shading.
struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, camera: vec4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
struct Output {
 @builtin(position) position: vec4<f32>,
 @location(0) color: vec4<f32>,
 @location(1) point: vec3<f32>,
 @location(2) normal: vec3<f32>,
 @location(3) arc: f32,
 @location(4) to_camera: vec3<f32>,
 @location(5) tangent: vec3<f32>,
}
fn unit(v: vec3<f32>) -> vec3<f32> { return v * inverseSqrt(max(dot(v,v),1e-12)); }
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
 @location(2) color: vec4<f32>, @location(3) normal: vec3<f32>, @location(6) tangent: vec4<f32>) -> Output {
 var out: Output;
 out.position = transform.mvp*vec4<f32>(position,1.0);
 out.color = color;
 out.point = position;
 out.normal = normal;
 out.tangent = tangent.xyz;
 out.arc = uv.y;
 out.to_camera = transform.camera.xyz-position;
 return out;
}
// Diagnostic only: isolate base-color, preserving all vertex transforms.
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> { return in.color; }
