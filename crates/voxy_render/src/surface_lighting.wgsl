enable wgpu_ray_query;
struct Surface { position_valid: vec4<f32>, normal_roughness: vec4<f32> }
struct Light { position_bias: vec4<f32>, intensity: vec4<f32>, camera: vec4<f32> }
@group(0) @binding(0) var scene: acceleration_structure;
@group(0) @binding(1) var<storage, read> surfaces: array<Surface>;
@group(0) @binding(2) var diffuse_map: texture_2d<f32>;
@group(0) @binding(3) var<uniform> light: Light;
@group(0) @binding(4) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var f0_map: texture_2d<f32>;
fn smith_lambda(c: f32, alpha2: f32) -> f32 {
    return (sqrt(1.0 + alpha2 * max(1.0-c*c,0.0)/(c*c))-1.0)*0.5;
}
fn specular_brdf(surface: Surface, direction: vec3<f32>, f0: vec3<f32>) -> vec3<f32> {
    let r = surface.normal_roughness.w;
    let delta = light.camera.xyz - surface.position_valid.xyz;
    if light.camera.w == 0.0 || r < 0.001 || length(delta) <= 0.0 { return vec3(0.0); }
    let view = normalize(delta);
    let n = surface.normal_roughness.xyz;
    let nv = dot(n,view);
    let nl = dot(n,direction);
    if nv <= 0.0 || nl <= 0.0 { return vec3(0.0); }
    let h = normalize(view+direction);
    let nh = clamp(dot(n,h),0.0,1.0);
    let vh = clamp(dot(view,h),0.0,1.0);
    let alpha2 = r*r*r*r;
    let den = (1.0-nh*nh) + alpha2*nh*nh;
    let d = alpha2/(3.141592653589793*den*den);
    let g = 1.0/(1.0+smith_lambda(nv,alpha2)+smith_lambda(nl,alpha2));
    return (f0+(vec3(1.0)-f0)*pow(1.0-vh,5.0))*(d*g/(4.0*nv*nl));
}
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output);
    if id.x >= size.x || id.y >= size.y { return; }
    let surface = surfaces[id.y * size.x + id.x];
    let diffuse = textureLoad(diffuse_map,vec2<i32>(id.xy),0).rgb;
    let delta = light.position_bias.xyz - surface.position_valid.xyz;
    let distance = length(delta);
    var color = vec3<f32>(0.0);
    if surface.position_valid.w == 1.0 && distance > max(2.0 * light.position_bias.w,1e-10) && distance < 1e20
        && all(diffuse >= vec3<f32>(0.0)) && all(diffuse <= vec3<f32>(1.0)) {
        let direction = delta / distance;
        let cosine = max(dot(surface.normal_roughness.xyz,direction),0.0);
        if cosine > 0.0 {
            var query: ray_query;
            rayQueryInitialize(&query,scene,RayDesc(0u,255u,light.position_bias.w,distance-light.position_bias.w,surface.position_valid.xyz,direction));
            rayQueryProceed(&query);
            if rayQueryGetCommittedIntersection(&query).kind == 0u {
                var brdf = diffuse / 3.141592653589793;
                if light.camera.w != 0.0 {
                    let f0 = textureLoad(f0_map,vec2<i32>(id.xy),0).rgb;
                    if all(f0 >= vec3(0.0)) && all(f0 <= vec3(1.0)) { brdf += specular_brdf(surface,direction,f0); }
                }
                color = min(brdf * light.intensity.rgb * (cosine / (distance * distance)),vec3<f32>(65504.0));
            }
        }
    }
    textureStore(output,vec2<i32>(id.xy),vec4<f32>(color,1.0));
}
