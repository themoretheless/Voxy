struct Transform {
    mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, view: vec4<f32>,
    world: mat4x4<f32>, tint: vec4<f32>, light: vec4<f32>, flags: vec4<f32>,
}
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) color: vec4<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) tint: vec4<f32>, @location(4) light: vec4<f32>, @location(5) flags: vec4<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> Output {
    var out: Output;
    out.position = transform.mvp * vec4<f32>(position, 1.0);
    out.world_position = (transform.world * vec4<f32>(position, 1.0)).xyz;
    out.uv = uv; out.color = color; out.tint = transform.tint; out.light = transform.light; out.flags = transform.flags;
    return out;
}
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
    var color = textureSample(image, image_sampler, in.uv) * in.color * in.tint;
    if in.flags.x > 0.0 { color.a = 1.0; }
    if color.a <= 0.0 { discard; }
    let normal_cross = cross(dpdx(in.world_position), dpdy(in.world_position));
    let normal = normal_cross / max(length(normal_cross), 0.000001);
    let direction = in.light.xyz / max(length(in.light.xyz), 0.000001);
    // Flat, double-sided Lambert lighting; no PBR/shadow claims.
    let illumination = 0.2 + abs(dot(normal, direction)) * in.light.w;
    color = vec4<f32>(color.rgb * select(1.0, illumination, in.light.w > 0.0), color.a);
    return color;
}
