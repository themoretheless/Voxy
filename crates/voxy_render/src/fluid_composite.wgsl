// SI coefficients, physical spheres and camera-relative optical reconstruction.
struct Camera {
    view: mat4x4f,
    projection: mat4x4f,
    inverse_projection: mat4x4f,
    inverse_view: mat4x4f,
    viewport: vec4f,
    controls: vec4f,
}
@group(0) @binding(0) var<uniform> camera: Camera;
fn eye_position(pixel: vec2f, device_depth: f32) -> vec3f {
    let uv = pixel / camera.viewport.xy;
    let p = camera.inverse_projection * vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, device_depth, 1.0);
    return p.xyz / p.w;
}
fn ray(pixel: vec2f) -> vec3f { return normalize(eye_position(pixel, 1.0)); }
fn pixel_index(pixel: vec2f) -> vec2i { return clamp(vec2i(pixel), vec2i(0), vec2i(camera.viewport.xy) - 1); }
struct FullInput { @builtin(position) position: vec4f, @location(0) uv: vec2f }
@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> FullInput {
    let corners = array<vec2f, 3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
    var out: FullInput; out.position = vec4f(corners[index], 0, 1); out.uv = vec2f(corners[index].x * 0.5 + 0.5, 0.5 - corners[index].y * 0.5); return out;
}

@group(0) @binding(1) var depth_radius: texture_2d<f32>;
@group(0) @binding(2) var thickness: texture_2d<f32>;
@group(0) @binding(3) var material: texture_2d<f32>;
@group(0) @binding(4) var background: texture_2d<f32>;
@group(0) @binding(5) var depth: texture_depth_2d;
@group(0) @binding(6) var optical_depth: texture_2d<f32>;
fn position_at(pixel: vec2f, depth: f32) -> vec3f {
    let direction = ray(pixel); return direction * depth / -direction.z;
}
fn tangent(pixel: vec2f, axis: vec2f, centre: vec2f) -> vec3f {
    let a = textureLoad(depth_radius, pixel_index(pixel+axis), 0).xy;
    let b = textureLoad(depth_radius, pixel_index(pixel-axis), 0).xy;
    let origin = position_at(pixel, centre.x);
    if (a.x > 0.0 && abs(a.x-centre.x) < centre.y*1.5 && (b.x <= 0.0 || abs(a.x-centre.x) <= abs(b.x-centre.x))) {
        return position_at(pixel+axis, a.x)-origin;
    }
    if (b.x > 0.0 && abs(b.x-centre.x) < centre.y*1.5) { return origin-position_at(pixel-axis,b.x); }
    return position_at(pixel+axis, centre.x)-origin;
}
@fragment fn fs_composite(input: FullInput) -> @location(0) vec4f {
    let pixel = input.uv * camera.viewport.xy; let index = pixel_index(pixel);
    let base = textureLoad(background,index,0);
    let surface = textureLoad(depth_radius,index,0).xy;
    let world_depth = -eye_position(pixel,scene_depth_at(pixel)).z;
    if (surface.x <= 0.0 || surface.x >= world_depth) { return base; }
    let dx = tangent(pixel,vec2f(1,0),surface);
    let dy = tangent(pixel,vec2f(0,1),surface);
    var normal = normalize(cross(dx,dy));
    let view = -ray(pixel);
    if (dot(normal,view)<0.0) { normal = -normal; }
    let optical = textureLoad(material,index,0);
    let path = textureLoad(thickness,index,0).x;
    let absorption_path = textureLoad(optical_depth,index,0).xyz;
    let f0 = pow((optical.w-1.0)/(optical.w+1.0),2.0);
    let fresnel = f0+(1.0-f0)*pow(1.0-clamp(dot(normal,view),0.0,1.0),5.0);
    let offset = normal.xy * vec2f(1,-1) * path * camera.viewport.z * camera.viewport.y / max(surface.x,0.01) * (1.0-1.0/optical.w);
    let q = pixel_index(pixel+offset);
    var refracted = textureLoad(background,q,0).rgb;
    if (-eye_position(vec2f(q)+0.5,scene_depth_at(vec2f(q)+0.5)).z < surface.x) { refracted=base.rgb; }
    let reflected = reflect(-view,normal);
    let sky = mix(vec3f(0.025,0.04,0.06),vec3f(0.65,0.75,0.9),clamp(reflected.y*0.5+0.5,0.0,1.0));
    // Analytic studio panel: a documented environment fallback, independent of scene depth.
    let panel = exp(-pow(abs((reflected.x+0.35)/0.18),4.0)) * exp(-pow(abs((reflected.y-0.4)/0.65),4.0));
    let environment = sky + vec3f(5.0,4.8,4.4)*panel;
    let light = normalize(vec3f(-0.4,0.7,1.0));
    let specular = pow(max(dot(reflect(-light,normal),view),0.0),100.0)*2.0;
    let transmission = exp(-absorption_path);
    return vec4f(refracted*transmission*(1.0-fresnel)+environment*fresnel+vec3f(specular*fresnel),1);
}
@fragment fn fs_depth_debug(input: FullInput) -> @location(0) vec4f {
    let d=textureLoad(depth_radius,pixel_index(input.position.xy),0).x;
    if (d<=0) { return vec4f(0,0,0,1); }
    return vec4f(vec3f(1.0/(1.0+d)),1);
}
@fragment fn fs_thickness_debug(input: FullInput) -> @location(0) vec4f {
    let h=textureLoad(thickness,pixel_index(input.position.xy),0).x;
    let v=1.0-exp(-h*20.0); return vec4f(v,v*v,0,1);
}

@group(0) @binding(7) var depth_sampler: sampler;
fn scene_depth_at(pixel: vec2f) -> f32 {
    let uv = (vec2f(pixel_index(pixel)) + 0.5) / camera.viewport.xy;
    return textureSampleLevel(depth, depth_sampler, uv, 0);
}
