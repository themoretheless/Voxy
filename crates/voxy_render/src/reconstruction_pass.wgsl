struct Camera { view_projection: mat4x4<f32>, position: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var specular_distance: texture_2d<f32>;
@group(0) @binding(2) var base_color: texture_2d<f32>;
@group(0) @binding(3) var material_sampler: sampler;
struct Vertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>, @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>, @location(3) material: vec2<f32>, @location(4) uv: vec2<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>, @location(3) material: vec2<f32>, @location(4) uv: vec2<f32>) -> Vertex {
    var out: Vertex;
    out.clip = camera.view_projection * vec4<f32>(position, 1.0);
    out.world = position; out.normal = normal; out.color = color; out.material = material; out.uv = uv;
    return out;
}
fn material_color(in: Vertex) -> vec3<f32> {
    if camera.position.w == 0.0 { return in.color; }
    return in.color * textureSample(base_color, material_sampler, in.uv).rgb;
}
fn reflectance(f0: vec3<f32>, alpha: f32, cosine: f32) -> vec3<f32> {
    let c2 = cosine * cosine; let c3 = c2 * cosine; let a3 = alpha * alpha * alpha;
    let bias_num = dot(vec2<f32>(0.99044 - 1.28514 * cosine, 1.29678 - 0.755907 * cosine), vec2<f32>(1.0, alpha));
    let bias_den = dot(vec3<f32>(dot(vec3<f32>(1.0,2.92338,59.4188),vec3<f32>(1.0,cosine,c3)), dot(vec3<f32>(20.3225,-27.0302,222.592),vec3<f32>(1.0,cosine,c3)), dot(vec3<f32>(121.563,626.13,316.627),vec3<f32>(1.0,cosine,c3))),vec3<f32>(1.0,alpha,a3));
    let scale_num = dot(vec2<f32>(0.0365463 + 3.32707 * cosine, 9.0632 - 9.04756 * cosine),vec2<f32>(1.0,alpha));
    let scale_den = dot(vec3<f32>(dot(vec3<f32>(1.0,3.59685,-1.36772),vec3<f32>(1.0,c2,c3)),dot(vec3<f32>(9.04401,-16.3174,9.22949),vec3<f32>(1.0,c2,c3)),dot(vec3<f32>(5.56589,19.7886,-20.2123),vec3<f32>(1.0,c2,c3))),vec3<f32>(1.0,alpha,a3));
    return clamp(f0 * max(0.0, scale_num / scale_den) + vec3<f32>(max(0.0,bias_num / bias_den) * clamp(f0.g * 50.0,0.0,1.0)),vec3<f32>(0.0),vec3<f32>(1.0));
}
struct Guides { @location(0) normal_roughness: vec4<f32>, @location(1) diffuse: vec4<f32>, @location(2) specular: vec4<f32>, @location(3) hit_distance: f32 }
@fragment fn fs_main(in: Vertex) -> Guides {
    let normal = normalize(in.normal);
    let direction = camera.position.xyz - in.world;
    let view = direction / max(length(direction), 0.000001);
    let f0 = mix(vec3<f32>(0.04), material_color(in), in.material.x);
    var out: Guides;
    out.normal_roughness = vec4<f32>(normal, in.material.y);
    out.diffuse = vec4<f32>(material_color(in) * (1.0 - in.material.x), 1.0);
    out.specular = vec4<f32>(reflectance(f0, in.material.y * in.material.y, abs(dot(normal,view))), 1.0);
    out.hit_distance = textureLoad(specular_distance, vec2<i32>(in.clip.xy), 0).r;
    return out;
}

@fragment fn fs_f0(in: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(mix(vec3<f32>(0.04),material_color(in),in.material.x),1.0);
}

struct ObjectIdentity { value: vec4<u32> }
@group(1) @binding(0) var<uniform> object: ObjectIdentity;
@fragment fn fs_object() -> @location(0) u32 { return object.value.x; }
