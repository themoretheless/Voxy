enable wgpu_ray_query;
struct Surface { position_valid: vec4<f32>, normal_roughness: vec4<f32> }
struct ReflectionHit { position_distance: vec4<f32>, identity: vec4<u32>, barycentrics_valid: vec4<f32> }
struct Camera { f0: vec4<f32>, position_bias: vec4<f32>, range: vec4<f32> }
@group(0) @binding(0) var scene: acceleration_structure;
@group(0) @binding(1) var<storage, read> surfaces: array<Surface>;
@group(0) @binding(2) var<storage, read> emission: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> camera: Camera;
@group(0) @binding(4) var distances: texture_storage_2d<r32float, write>;
@group(0) @binding(5) var colors: texture_storage_2d<rgba16float, write>;
@group(0) @binding(6) var material_f0: texture_2d<f32>;
@group(0) @binding(7) var<storage, read_write> reflection_hits: array<ReflectionHit>;
fn hash(value: u32) -> u32 {
    var x = value;
    x = (x ^ (x >> 16u)) * 0x7feb352du;
    x = (x ^ (x >> 15u)) * 0x846ca68bu;
    return x ^ (x >> 16u);
}
fn lambda(c: f32, alpha2: f32) -> f32 {
    return (sqrt(1.0 + alpha2 * max(1.0 - c*c, 0.0) / (c*c)) - 1.0) * 0.5;
}
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(distances);
    if id.x >= size.x || id.y >= size.y { return; }
    let pixel = id.y * size.x + id.x;
    reflection_hits[pixel] = ReflectionHit(vec4<f32>(0.0), vec4<u32>(0u), vec4<f32>(0.0));
    let surface = surfaces[pixel];
    var color = vec3<f32>(0.0);
    var distance = 0.0;
    let to_camera = camera.position_bias.xyz - surface.position_valid.xyz;
    let view_length = length(to_camera);
    var f0 = camera.f0;
    if camera.range.y == 1.0 { f0 = textureLoad(material_f0, vec2<i32>(id.xy), 0); }
    if surface.position_valid.w == 1.0 && (surface.normal_roughness.w == 0.0 || camera.range.z == 1.0) && f0.a > 0.0 && all(f0.rgb >= vec3<f32>(0.0)) && all(f0.rgb <= vec3<f32>(1.0)) && view_length > 0.0 && view_length < 1e20 {
        let view = to_camera / view_length;
        let normal = surface.normal_roughness.xyz;
        var direction = reflect(-view, normal);
        var half_cosine = clamp(abs(dot(normal,view)), 0.0, 1.0);
        var weight_scale = 1.0;
        let roughness = surface.normal_roughness.w;
        if camera.range.z == 1.0 && roughness >= 0.001 && roughness <= 1.0 {
            let nv = dot(normal, view);
            if nv <= 0.0 { textureStore(distances, vec2<i32>(id.xy), vec4(0.0)); textureStore(colors, vec2<i32>(id.xy), vec4(0.0,0.0,0.0,1.0)); return; }
            let key = hash((id.y * size.x + id.x) ^ hash(bitcast<u32>(camera.range.w) + 0x9e3779b9u));
            let u = f32(key >> 8u) / 16777216.0;
            let phi = 6.28318530718 * f32(hash(key) >> 8u) / 16777216.0;
            let alpha2 = roughness * roughness * roughness * roughness;
            let tan2 = alpha2 * u / (1.0-u);
            let c = inverseSqrt(1.0+tan2);
            let sn = sqrt(tan2/(1.0+tan2));
            let axis = select(vec3(1.0,0.0,0.0), vec3(0.0,0.0,1.0), abs(normal.z)<0.9);
            let tangent = normalize(cross(axis, normal));
            let h = tangent * (sn*cos(phi)) + cross(normal,tangent) * (sn*sin(phi)) + normal*c;
            let vh = dot(view,h);
            direction = reflect(-view,h);
            let nl = dot(normal,direction);
            if vh <= 0.0 || nl <= 0.0 { textureStore(distances, vec2<i32>(id.xy), vec4(0.0)); textureStore(colors, vec2<i32>(id.xy), vec4(0.0,0.0,0.0,1.0)); return; }
            half_cosine = clamp(vh,0.0,1.0);
            weight_scale = vh / (nv*c*(1.0+lambda(nv,alpha2)+lambda(nl,alpha2)));
        }
        var query: ray_query;
        rayQueryInitialize(&query, scene, RayDesc(0u,255u,camera.position_bias.w,camera.range.x,surface.position_valid.xyz,direction));
        rayQueryProceed(&query);
        let hit = rayQueryGetCommittedIntersection(&query);
        if hit.kind != 0u {
            reflection_hits[pixel] = ReflectionHit(
                vec4<f32>(surface.position_valid.xyz + direction * hit.t, hit.t),
                vec4<u32>(hit.instance_index, hit.instance_custom_data, hit.geometry_index, hit.primitive_index),
                vec4<f32>(hit.barycentrics, 0.0, 1.0));
        }
        if hit.kind != 0u && hit.primitive_index < arrayLength(&emission) {
            distance = hit.t;
            let cosine = half_cosine;
            let weight = f0.rgb + (vec3<f32>(1.0) - f0.rgb) * pow(1.0 - cosine,5.0);
            color = emission[hit.primitive_index].rgb * weight * weight_scale;
        }
    }
    textureStore(distances, vec2<i32>(id.xy), vec4<f32>(distance,0.0,0.0,1.0));
    textureStore(colors, vec2<i32>(id.xy), vec4<f32>(color,1.0));
}
