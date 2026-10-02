@group(0) @binding(0) var environment: texture_cube<f32>;
@group(0) @binding(1) var environment_sampler: sampler;
struct Parameters { face: u32, size: u32, roughness: f32, samples: u32 }
@group(0) @binding(2) var<uniform> params: Parameters;
struct VertexOutput { @builtin(position) position: vec4<f32> }
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let positions = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    out.position = vec4(positions[index], 0.0, 1.0);
    return out;
}
fn face_direction(uv: vec2<f32>) -> vec3<f32> {
    switch params.face {
        case 0u: { return normalize(vec3(1.0, -uv.y, -uv.x)); }
        case 1u: { return normalize(vec3(-1.0, -uv.y, uv.x)); }
        case 2u: { return normalize(vec3(uv.x, 1.0, uv.y)); }
        case 3u: { return normalize(vec3(uv.x, -1.0, -uv.y)); }
        case 4u: { return normalize(vec3(uv.x, -uv.y, 1.0)); }
        default: { return normalize(vec3(-uv.x, -uv.y, -1.0)); }
    }
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = face_direction(in.position.xy / f32(params.size) * 2.0 - 1.0);
    if params.roughness == 0.0 { return textureSampleLevel(environment, environment_sampler, n, 0.0); }
    let up = select(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0), abs(n.z) > 0.999);
    let tangent = normalize(cross(up, n));
    let bitangent = cross(n, tangent);
    let alpha2 = pow(params.roughness, 4.0);
    var total = vec3(0.0);
    var weight = 0.0;
    for (var i = 0u; i < params.samples; i++) {
        let xi = vec2(f32(i) / f32(params.samples), f32(reverseBits(i)) * 2.3283064365386963e-10);
        let phi = 6.283185307179586 * xi.x;
        let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (alpha2 - 1.0) * xi.y));
        let sin_theta = sqrt(max(0.0, 1.0 - cos_theta * cos_theta));
        let h = tangent * cos(phi) * sin_theta + bitangent * sin(phi) * sin_theta + n * cos_theta;
        let l = normalize(2.0 * dot(n, h) * h - n);
        let nl = max(dot(n, l), 0.0);
        total += textureSampleLevel(environment, environment_sampler, l, 0.0).rgb * nl;
        weight += nl;
    }
    return vec4(total / max(weight, 0.000001), 1.0);
}
