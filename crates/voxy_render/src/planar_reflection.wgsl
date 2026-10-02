struct Hit { position_distance: vec4<f32>, identity: vec4<u32>, barycentrics_valid: vec4<f32> }
struct Camera { current: mat4x4<f32>, previous: mat4x4<f32>, current_plane: vec4<f32>, previous_plane: vec4<f32> }
@group(0) @binding(0) var<storage, read> hits: array<Hit>;
@group(0) @binding(1) var<storage, read> previous_positions: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> camera: Camera;
@group(0) @binding(3) var motion: texture_storage_2d<rg32float, write>;
@group(0) @binding(4) var expected_depth: texture_storage_2d<r32float, write>;
@group(0) @binding(5) var current_depth: texture_storage_2d<r32float, write>;
fn finite4(v: vec4<f32>) -> bool {
    return all((bitcast<vec4<u32>>(v) & vec4<u32>(0x7f800000u)) != vec4<u32>(0x7f800000u));
}
fn virtual_point(p: vec3<f32>, plane: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(p - 2.0 * (dot(plane.xyz, p) + plane.w) * plane.xyz, 1.0);
}
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(motion);
    if any(id.xy >= size) { return; }
    let pixel = vec2<i32>(id.xy);
    textureStore(motion, pixel, vec4<f32>(0.0));
    textureStore(expected_depth, pixel, vec4<f32>(0.0));
    textureStore(current_depth, pixel, vec4<f32>(0.0));
    let index = id.y * size.x + id.x;
    let hit = hits[index];
    if hit.barycentrics_valid.w != 1.0 { return; }
    let current_clip = camera.current * virtual_point(hit.position_distance.xyz, camera.current_plane);
    if !finite4(current_clip) || current_clip.w <= 0.0 { return; }
    let c = current_clip.xyz / current_clip.w;
    let uv = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / vec2<f32>(size);
    let projected_uv = vec2<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
    // Require mirror correspondence to this primary pixel; rough samples/foreign
    // planes cannot silently be projected as a valid reflecting surface.
    if c.z <= 0.0 || c.z >= 1.0 || any(abs(projected_uv - uv) * vec2<f32>(size) > vec2<f32>(0.01)) { return; }
    textureStore(current_depth, pixel, vec4<f32>(c.z,0.0,0.0,1.0));
    let previous = previous_positions[index];
    if previous.w != 1.0 || !finite4(previous) { return; }
    let previous_clip = camera.previous * virtual_point(previous.xyz, camera.previous_plane);
    if !finite4(previous_clip) || previous_clip.w <= 0.0 { return; }
    let p = previous_clip.xyz / previous_clip.w;
    let previous_uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    if p.z <= 0.0 || p.z >= 1.0 || any(previous_uv < vec2<f32>(0.0)) || any(previous_uv >= vec2<f32>(1.0)) { return; }
    textureStore(motion, pixel, vec4<f32>(previous_uv - uv,0.0,0.0));
    textureStore(expected_depth, pixel, vec4<f32>(p.z,0.0,0.0,1.0));
}
