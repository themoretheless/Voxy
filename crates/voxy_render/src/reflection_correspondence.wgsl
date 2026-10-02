struct Hit { position_distance: vec4<f32>, identity: vec4<u32>, barycentrics_valid: vec4<f32> }
struct Triangle { identity: vec4<u32>, a: vec4<f32>, b: vec4<f32>, c: vec4<f32> }
struct Options { count: u32, reset: u32, width: u32, height: u32 }
@group(0) @binding(0) var<storage, read> hits: array<Hit>;
@group(0) @binding(1) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(2) var<storage, read_write> previous: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> options: Options;
fn less(a: vec4<u32>, b: vec4<u32>) -> bool {
    for (var i = 0u; i < 4u; i++) {
        if a[i] != b[i] { return a[i] < b[i]; }
    }
    return false;
}
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= options.width || id.y >= options.height { return; }
    let index = id.y * options.width + id.x;
    previous[index] = vec4<f32>(0.0);
    let hit = hits[index];
    if options.reset != 0u || hit.barycentrics_valid.w != 1.0 { return; }
    let bc = hit.barycentrics_valid.xy;
    if any((bitcast<vec2<u32>>(bc) & vec2<u32>(0x7f800000u)) == vec2<u32>(0x7f800000u)) || any(bc < vec2<f32>(-0.00001)) || bc.x + bc.y > 1.00001 { return; }
    var low = 0u;
    var high = options.count;
    while low < high {
        let mid = low + (high - low) / 2u;
        if less(triangles[mid].identity, hit.identity) { low = mid + 1u; }
        else { high = mid; }
    }
    if low >= options.count { return; }
    let triangle = triangles[low];
    if any(triangle.identity != hit.identity) { return; }
    let p = triangle.a.xyz * (1.0 - bc.x - bc.y) + triangle.b.xyz * bc.x + triangle.c.xyz * bc.y;
    if any((bitcast<vec3<u32>>(p) & vec3<u32>(0x7f800000u)) == vec3<u32>(0x7f800000u)) { return; }
    previous[index] = vec4<f32>(p, 1.0);
}
