// SceneRenderer prefix ABI; second matrix is capture MVP, not motion history.
struct Transform { mvp: mat4x4<f32>, capture_mvp: mat4x4<f32>, eye: vec4<f32>, world: mat4x4<f32>, tint: vec4<f32>, light: vec4<f32>, authored: vec4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) capture_clip: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>, @location(3) normal: vec3<f32>,
    @location(4) @interpolate(flat) eye: vec3<f32>,
    @location(5) @interpolate(flat) f0: f32,
    @location(6) @interpolate(flat) roughness: f32,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(2) color: vec4<f32>, @location(3) normal: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip = transform.mvp * vec4(position, 1.0);
    out.capture_clip = transform.capture_mvp * vec4(position, 1.0);
    out.color = color;
    out.world = (transform.world * vec4(position, 1.0)).xyz;
    out.normal = (transform.world * vec4(normal, 0.0)).xyz;
    out.eye = transform.eye.xyz;
    let metallic = select(0.0, transform.authored.z, transform.authored.w > 0.5);
    // Scalar reflectance supports dielectrics and neutral metallic reflectors.
    let neutral = dot(clamp(transform.tint.rgb, vec3(0.0), vec3(1.0)), vec3(1.0 / 3.0));
    out.f0 = mix(0.04, neutral, metallic);
    out.roughness = select(0.0, transform.authored.y, transform.authored.w > 0.5);
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.capture_clip.w <= 0.0 { discard; }
    let ndc = in.capture_clip.xyz / in.capture_clip.w;
    if any(ndc.xy < vec2(-1.0)) || any(ndc.xy > vec2(1.0)) || ndc.z < 0.0 || ndc.z > 1.0 { discard; }
    let uv = ndc.xy * vec2(0.5, -0.5) + vec2(0.5);
    let reflected = textureSampleLevel(image, image_sampler, uv, in.roughness * in.roughness * f32(textureNumLevels(image) - 1u)) * in.color;
    let v = in.eye - in.world;
    let n = in.normal / max(length(in.normal), 0.0001);
    let nv = clamp(dot(n, v / max(length(v), 0.0001)), 0.0, 1.0);
    let fresnel = in.f0 + (1.0 - in.f0) * pow(1.0 - nv, 5.0);
    return vec4(reflected.rgb, reflected.a * fresnel);
}
