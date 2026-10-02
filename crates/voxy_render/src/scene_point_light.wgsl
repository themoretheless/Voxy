// SceneRenderer ABI. World must be rigid or uniformly scaled for these normals.
struct Transform {
    mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, eye: vec4<f32>,
    world: mat4x4<f32>, tint: vec4<f32>, light: vec4<f32>, authored: vec4<f32>,
}
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>, @location(3) normal: vec3<f32>,
    @location(4) @interpolate(flat) eye: vec3<f32>,
    @location(5) @interpolate(flat) light: vec4<f32>,
    @location(6) @interpolate(flat) material: vec3<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>, @location(3) normal: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip = transform.mvp * vec4<f32>(position, 1.0);
    out.world = (transform.world * vec4<f32>(position, 1.0)).xyz;
    out.normal = (transform.world * vec4<f32>(normal, 0.0)).xyz;
    out.uv = uv; out.color = color * transform.tint;
    out.eye = transform.eye.xyz; out.light = transform.light;
    out.material = transform.authored.yzw;
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(image, image_sampler, in.uv) * in.color;
    if texel.a <= 0.0 { discard; }
    let base = clamp(texel.rgb, vec3(0.0), vec3(1.0));
    let n = in.normal / max(length(in.normal), 0.0001);
    let delta = in.light.xyz - in.world;
    let distance2 = max(dot(delta, delta), 0.0001);
    let l = delta / sqrt(distance2);
    let view_delta = in.eye - in.world;
    let v = view_delta / max(length(view_delta), 0.0001);
    let h = (l + v) / max(length(l + v), 0.0001);
    // Roundoff can put normalized dot products above one; pow needs a nonnegative base.
    let nl = clamp(dot(n, l), 0.0, 1.0);
    let nv = clamp(dot(n, v), 0.0001, 1.0);
    let nh = clamp(dot(n, h), 0.0, 1.0);
    let vh = clamp(dot(v, h), 0.0, 1.0);
    let roughness = max(select(0.35, in.material.x, in.material.z > 0.5), 0.04);
    let metallic = select(0.5, in.material.y, in.material.z > 0.5);
    let a = roughness * roughness; let a2 = a * a;
    let d = a2 / (3.14159265 * pow(nh * nh * (a2 - 1.0) + 1.0, 2.0));
    let k = pow(roughness + 1.0, 2.0) / 8.0;
    let g = (nl / max(nl * (1.0 - k) + k, 0.0001)) * (nv / (nv * (1.0 - k) + k));
    let f0 = mix(vec3(0.04), base, metallic);
    let f = f0 + (vec3(1.0) - f0) * pow(1.0 - vh, 5.0);
    let specular = d * g * f / max(4.0 * nl * nv, 0.0001);
    let diffuse = (vec3(1.0) - f) * (1.0 - metallic) * base / 3.14159265;
    return vec4((diffuse + specular) * nl * in.light.w / distance2 + base * 0.02, texel.a);
}
