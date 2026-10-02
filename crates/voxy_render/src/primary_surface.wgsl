struct Camera { inverse: mat4x4<f32>, clear_depth: vec4<f32> }
struct Surface { position_valid: vec4<f32>, normal_roughness: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var normals: texture_2d<f32>;
@group(0) @binding(3) var<storage, read_write> surfaces: array<Surface>;
@group(0) @binding(4) var depth_sampler: sampler;
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(depth);
    if id.x >= size.x || id.y >= size.y { return; }
    let index = id.y * size.x + id.x;
    var surface: Surface;
    surface.position_valid = vec4<f32>(0.0);
    surface.normal_roughness = vec4<f32>(0.0);
    let pixel = vec2<i32>(id.xy);
    let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size);
    let z = textureSampleLevel(depth, depth_sampler, uv, 0);
    let normal = textureLoad(normals, pixel, 0);
    let world = camera.inverse * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, z, 1.0);
    let normal_length = length(normal.xyz);
    let position = world.xyz / world.w;
    if z >= 0.0 && z <= 1.0 && z != camera.clear_depth.x
        && normal_length > 0.0 && normal_length < 1e20
        && normal.w >= 0.0 && normal.w <= 1.0
        && all(abs(position) < vec3<f32>(3.402823e38)) {
        surface.position_valid = vec4<f32>(position, 1.0);
        surface.normal_roughness = vec4<f32>(normal.xyz / normal_length, normal.w);
    }
    surfaces[index] = surface;
}
