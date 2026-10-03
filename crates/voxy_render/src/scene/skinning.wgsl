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
    // Scale before cofactors to avoid overflow. Preflight rejects numerically
    // singular blended matrices; inverse transpose preserves tangent orthogonality.
    var a = skin[0].xyz; var b = skin[1].xyz; var c = skin[2].xyz;
    let scale = max(max(max(abs(a.x), abs(a.y)), abs(a.z)),
        max(max(max(abs(b.x), abs(b.y)), abs(b.z)), max(max(abs(c.x), abs(c.y)), abs(c.z))));
    a /= scale; b /= scale; c /= scale;
    let determinant = dot(a, cross(b, c));
    let cofactor = mat3x3<f32>(cross(b, c), cross(c, a), cross(a, b));
    var original = vec3<f32>(source[s+3u], source[s+4u], source[s+5u]);
    let normal_scale = max(max(abs(original.x), abs(original.y)), abs(original.z));
    // Zero denotes absent authored normals; lighting derives a flat posed face.
    var n = vec3<f32>(0.);
    if normal_scale > 0. {
        original /= normal_scale;
        let transformed = (cofactor * original) * sign(determinant);
        let result_scale = max(max(abs(transformed.x), abs(transformed.y)), abs(transformed.z));
        if result_scale > 0. { n = normalize(transformed / result_scale); }
    }
    let v = i * 9u;
    vertices[v] = p.x; vertices[v+1u] = p.y; vertices[v+2u] = p.z;
    vertices[v+3u] = source[s+6u]; vertices[v+4u] = source[s+7u];
    for (var k = 0u; k < 4u; k++) { vertices[v+5u+k] = parameters.color[k]; }
    normals[i*3u] = n.x; normals[i*3u+1u] = n.y; normals[i*3u+2u] = n.z;
}
