struct Camera {
    view_proj: mat4x4<f32>,
};

struct Object {
    model: mat4x4<f32>,
    material: vec4<u32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var materials: texture_2d_array<f32>;
@group(1) @binding(1) var material_sampler: sampler;
@group(2) @binding(0) var<uniform> object: Object;
@group(2) @binding(1) var<storage, read> joints: array<mat4x4<f32>>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) joint: vec4<u32>,
    @location(4) weight: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let skin = joints[input.joint.x] * input.weight.x
        + joints[input.joint.y] * input.weight.y
        + joints[input.joint.z] * input.weight.z
        + joints[input.joint.w] * input.weight.w;
    let world = object.model * skin;
    var output: VertexOutput;
    output.position = camera.view_proj * world * vec4<f32>(input.position, 1.0);
    output.world_position = (world * vec4<f32>(input.position, 1.0)).xyz;
    output.normal = vec3<f32>(0.);
    let transformed = (world * vec4<f32>(input.normal, 0.0)).xyz;
    let magnitude = max(max(abs(transformed.x), abs(transformed.y)), abs(transformed.z));
    if magnitude > 0. { output.normal = normalize(transformed / magnitude); }
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(materials, material_sampler, input.uv, i32(object.material.x));
    let sun = normalize(vec3<f32>(0.45, 0.82, 0.35));
    var normal = input.normal;
    if dot(normal, normal) == 0. {
        let face = cross(dpdx(input.world_position), dpdy(input.world_position));
        normal = face / max(length(face), 0.000001);
    }
    let light = 0.18 + 0.82 * max(dot(normal, sun), 0.0);
    let source = dot(albedo.rgb, vec3<f32>(0.299, 0.587, 0.114)) * light;
    let level = floor(clamp(source, 0.0, 0.999) * 32.0) / 31.0;
    let lcd_paper = vec3<f32>(0.78, 0.78, 0.76);
    let lcd_ink = vec3<f32>(0.035, 0.035, 0.035);
    let color = mix(lcd_ink, lcd_paper, level);
    return vec4<f32>(color, albedo.a);
}
