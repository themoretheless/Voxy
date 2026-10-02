enable wgpu_ray_query;
struct Segment { origin_bias: vec4<f32>, destination: vec4<f32> }
@group(0) @binding(0) var scene: acceleration_structure;
@group(0) @binding(1) var<storage, read> segments: array<Segment>;
@group(0) @binding(2) var<storage, read_write> visibility: array<u32>;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&segments) || id.x >= arrayLength(&visibility) { return; }
    let segment = segments[id.x];
    let delta = segment.destination.xyz - segment.origin_bias.xyz;
    let distance = length(delta);
    let bias = segment.origin_bias.w;
    if distance <= 2.0 * bias { visibility[id.x] = 1u; return; }
    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(0u, 255u, bias, distance - bias,
        segment.origin_bias.xyz, delta / distance));
    rayQueryProceed(&query);
    let hit = rayQueryGetCommittedIntersection(&query);
    visibility[id.x] = select(0u, 1u, hit.kind == 0u);
}
