struct Camera {
    view_proj: mat4x4<f32>,
};

struct ChunkMeta {
    relative_origin: vec4<i32>,
    light_info: vec4<u32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> chunk_meta: array<ChunkMeta>;
@group(0) @binding(2) var<storage, read> light_words: array<u32>;
@group(1) @binding(0) var materials: texture_2d_array<f32>;
@group(1) @binding(1) var material_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) origin: vec4<u32>,
    @location(1) extent_face: vec4<u32>,
    @location(2) material_layer: vec2<u32>,
    @location(3) ao_diagonal: vec4<u32>,
    @location(4) chunk_slot: vec2<u32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) normal: vec3<f32>,
    @location(3) ao: f32,
    @location(4) @interpolate(flat) light: vec2<f32>,
};

fn sample_light(slot: u32, local: vec3<u32>) -> vec2<f32> {
    let voxel = min(local, vec3<u32>(31u));
    let index = voxel.x + 32u * (voxel.z + 32u * voxel.y);
    let word = light_words[chunk_meta[slot].light_info.x + index / 4u];
    let packed = (word >> ((index & 3u) * 8u)) & 0xffu;
    return vec2<f32>(f32(packed >> 4u), f32(packed & 0x0fu)) / 15.0;
}

fn face_normal(face: u32) -> vec3<f32> {
    switch face {
        case 0u: { return vec3<f32>(-1.0, 0.0, 0.0); }
        case 1u: { return vec3<f32>( 1.0, 0.0, 0.0); }
        case 2u: { return vec3<f32>(0.0, -1.0, 0.0); }
        case 3u: { return vec3<f32>(0.0,  1.0, 0.0); }
        case 4u: { return vec3<f32>(0.0, 0.0, -1.0); }
        default: { return vec3<f32>(0.0, 0.0, 1.0); }
    }
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let corner = corners[input.vertex_index];
    let face = input.extent_face.z;
    var axis_u = vec3<f32>(1.0, 0.0, 0.0);
    var axis_v = vec3<f32>(0.0, 1.0, 0.0);
    if (face == 0u || face == 1u) {
        axis_u = vec3<f32>(0.0, 0.0, 1.0);
        axis_v = vec3<f32>(0.0, 1.0, 0.0);
    } else if (face == 2u || face == 3u) {
        axis_u = vec3<f32>(1.0, 0.0, 0.0);
        axis_v = vec3<f32>(0.0, 0.0, 1.0);
    }
    let extent = vec2<f32>(f32(input.extent_face.x), f32(input.extent_face.y));
    let local = vec3<f32>(chunk_meta[input.chunk_slot.x].relative_origin.xyz)
        + vec3<f32>(input.origin.xyz)
        + axis_u * corner.x * extent.x
        + axis_v * corner.y * extent.y;
    let ao_values = vec4<f32>(input.ao_diagonal & vec4<u32>(0x7fu)) / 3.0;
    let ao = select(
        select(ao_values.x, ao_values.y, corner.x > 0.5),
        select(ao_values.w, ao_values.z, corner.x > 0.5),
        corner.y > 0.5,
    );
    var output: VertexOutput;
    output.position = camera.view_proj * vec4<f32>(local, 1.0);
    output.uv = corner * extent;
    output.material = input.material_layer.x;
    output.normal = face_normal(face);
    output.ao = ao;
    output.light = sample_light(input.chunk_slot.x, input.origin.xyz);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(materials, material_sampler, input.uv, i32(input.material));
    let sun = normalize(vec3<f32>(0.45, 0.82, 0.35));
    let diffuse = 0.18 + 0.82 * max(dot(input.normal, sun), 0.0);
    let illumination = max(0.08, max(input.light.y, input.light.x * diffuse));
    let source = dot(albedo.rgb, vec3<f32>(0.299, 0.587, 0.114))
        * illumination * mix(0.58, 1.0, input.ao);

    // Thirty-two LCD ink densities preserve silhouettes and terrain depth while
    // keeping the whole image inside one neutral monochrome display palette.
    let level = floor(clamp(source, 0.0, 0.999) * 32.0) / 31.0;
    let lcd_paper = vec3<f32>(0.78, 0.78, 0.76);
    let lcd_ink = vec3<f32>(0.035, 0.035, 0.035);
    let color = mix(lcd_ink, lcd_paper, level);
    return vec4<f32>(color, albedo.a);
}
