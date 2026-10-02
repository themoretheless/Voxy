struct Parameters { color: vec4<f32>, count: u32, pad0: u32, pad1: u32, pad2: u32 }
@group(0) @binding(0) var<storage, read> source: array<f32>;
@group(0) @binding(1) var<storage, read> joints: array<mat4x4<f32>>;
@group(0) @binding(2) var<storage, read_write> vertices: array<f32>;
@group(0) @binding(3) var<storage, read_write> normals: array<f32>;
@group(0) @binding(4) var<uniform> parameters: Parameters;

@compute @workgroup_size(64)
fn deform(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= parameters.count { return; }
    let s = i * 16u;
    var skin = mat4x4<f32>(vec4<f32>(0.), vec4<f32>(0.), vec4<f32>(0.), vec4<f32>(0.));
    for (var k = 0u; k < 4u; k++) {
        skin += joints[u32(source[s + 8u + k])] * source[s + 12u + k];
    }
    let p = skin * vec4<f32>(source[s], source[s+1u], source[s+2u], 1.);
    let transformed = (skin * vec4<f32>(source[s+3u], source[s+4u], source[s+5u], 0.)).xyz;
    var n = vec3<f32>(0., 0., 1.);
    if dot(transformed, transformed) > 0. { n = normalize(transformed); }
    let v = i * 9u;
    vertices[v] = p.x; vertices[v+1u] = p.y; vertices[v+2u] = p.z;
    vertices[v+3u] = source[s+6u]; vertices[v+4u] = source[s+7u];
    for (var k = 0u; k < 4u; k++) { vertices[v+5u+k] = parameters.color[k]; }
    normals[i*3u] = n.x; normals[i*3u+1u] = n.y; normals[i*3u+2u] = n.z;
}
