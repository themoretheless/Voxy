// SI coefficients, physical spheres and camera-relative optical reconstruction.
struct Camera {
    view: mat4x4f,
    projection: mat4x4f,
    inverse_projection: mat4x4f,
    inverse_view: mat4x4f,
    viewport: vec4f,
    controls: vec4f,
}
struct Particle { @location(0) position_radius: vec4f, @location(1) absorption_ior: vec4f }
@group(0) @binding(0) var<uniform> camera: Camera;
// Particle entrypoints use this layout. Filter and composite entrypoints have
// separate, explicitly supplied layouts with the same binding numbers.
@group(0) @binding(2) var depth: texture_depth_2d;

struct SphereInput {
    @builtin(position) position: vec4f,
    @location(0) @interpolate(flat) centre: vec3f,
    @location(1) @interpolate(flat) radius: f32,
    @location(2) @interpolate(flat) material: vec4f,
}
@vertex
fn vs_particle(@builtin(vertex_index) vertex: u32, p: Particle) -> SphereInput {
    let corners = array<vec2f, 6>(vec2f(-1,-1),vec2f(1,-1),vec2f(-1,1),vec2f(-1,1),vec2f(1,-1),vec2f(1,1));
    let centre = (camera.view * vec4f(p.position_radius.xyz, 1)).xyz;
    let radius = p.position_radius.w;
    let distance = -centre.z;
    var out: SphereInput;
    // Conservative perspective bounds. Near-camera spheres use the full screen.
    var expansion = max(1.0, (distance + length(centre.xy)) / max(distance - radius, 0.00001));
    if orthographic() { expansion = 1.0; }
    var clip = camera.projection * vec4f(centre + vec3f(corners[vertex] * radius * expansion, 0), 1);
    if (distance - radius <= camera.controls.x) {
        clip = vec4f(corners[vertex], 0.0, 1.0);
    }
    out.position = clip;
    out.centre = centre;
    out.radius = radius;
    out.material = p.absorption_ior;
    return out;
}
fn eye_position(pixel: vec2f, device_depth: f32) -> vec3f {
    let uv = pixel / camera.viewport.xy;
    let p = camera.inverse_projection * vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, device_depth, 1.0);
    return p.xyz / p.w;
}
fn orthographic() -> bool { return camera.viewport.w >= 2.0; }
fn ray_origin(pixel: vec2f) -> vec3f {
    if orthographic() { return vec3f(eye_position(pixel, 0.0).xy, 0.0); }
    return vec3f(0.0);
}
fn ray(pixel: vec2f) -> vec3f {
    if orthographic() { return normalize(eye_position(pixel, 1.0)-eye_position(pixel, 0.0)); }
    return normalize(eye_position(pixel, 1.0));
}
fn pixel_index(pixel: vec2f) -> vec2i { return clamp(vec2i(pixel), vec2i(0), vec2i(camera.viewport.xy) - 1); }
fn sphere_interval(input: SphereInput) -> vec2f {
    let direction = ray(input.position.xy);
    let relative = input.centre-ray_origin(input.position.xy);
    let b = dot(direction, relative);
    let discriminant = b*b - dot(relative, relative) + input.radius*input.radius;
    if (discriminant <= 0) { discard; }
    let root = sqrt(discriminant);
    let near_t = camera.controls.x / -direction.z;
    let far_t = camera.controls.y / -direction.z;
    let scene_z = scene_depth_at(input.position.xy);
    let scene_t = dot(eye_position(input.position.xy, scene_z)-ray_origin(input.position.xy), direction);
    let start = max(b-root, near_t);
    let end = min(min(b+root, scene_t), far_t);
    if (end <= start) { discard; }
    return vec2f(start, end);
}
struct DepthOutput {
    @location(0) depth_radius: vec2f,
    @location(1) material: vec4f,
    @builtin(frag_depth) depth: f32,
}
@fragment
fn fs_depth(input: SphereInput) -> DepthOutput {
    let interval = sphere_interval(input);
    let position = ray_origin(input.position.xy)+ray(input.position.xy) * interval.x;
    let clip = camera.projection * vec4f(position, 1);
    var out: DepthOutput;
    out.depth_radius = vec2f(-position.z, input.radius);
    out.material = input.material;
    out.depth = clip.z / clip.w;
    return out;
}
struct PathOutput {
    @location(0) thickness: f32,
    @location(1) optical_depth: vec4f,
}
@fragment
fn fs_thickness(input: SphereInput) -> PathOutput {
    let interval = sphere_interval(input);
    let path = (interval.y - interval.x) / camera.viewport.z;
    var out: PathOutput;
    out.thickness = path;
    out.optical_depth = vec4f(input.material.xyz * path, 0);
    return out;
}

struct FullInput { @builtin(position) position: vec4f }
@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> FullInput {
    let corners = array<vec2f, 3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
    var out: FullInput; out.position = vec4f(corners[index], 0, 1); return out;
}

struct Film { @location(0) a_thickness: vec4f, @location(1) b: vec4f, @location(2) c: vec4f, @location(3) absorption_ior: vec4f }
struct FilmInput {
    @builtin(position) position: vec4f,
    @location(0) @interpolate(flat) a_thickness: vec4f,
    @location(1) @interpolate(flat) b: vec4f,
    @location(2) @interpolate(flat) c: vec4f,
    @location(3) @interpolate(flat) material: vec4f,
}
@vertex
fn vs_film(@builtin(vertex_index) vertex: u32, f: Film) -> FilmInput {
    let a = f.a_thickness.xyz;
    let b = f.b.xyz;
    let c = f.c.xyz;
    let n = normalize(cross(b-a, c-a));
    let extrusion = n * f.a_thickness.w;
    let points = array<vec3f, 6>(a,b,c,a+extrusion,b+extrusion,c+extrusion);
    var lower = vec2f(1.0);
    var upper = vec2f(-1.0);
    var full = false;
    for (var i = 0u; i < 6u; i++) {
        let eye = camera.view * vec4f(points[i], 1);
        if (-eye.z <= camera.controls.x) { full = true; }
        let clip = camera.projection * eye;
        let xy = clip.xy / clip.w;
        lower = min(lower, xy);
        upper = max(upper, xy);
    }
    if (full) { lower = vec2f(-1); upper = vec2f(1); }
    lower = clamp(lower, vec2f(-1), vec2f(1));
    upper = clamp(upper, vec2f(-1), vec2f(1));
    let corners = array<vec2f, 6>(vec2f(0,0),vec2f(1,0),vec2f(0,1),vec2f(0,1),vec2f(1,0),vec2f(1,1));
    var out: FilmInput;
    out.position = vec4f(mix(lower, upper, corners[vertex]), 0, 1);
    out.a_thickness = f.a_thickness;
    out.b = f.b;
    out.c = f.c;
    out.material = f.absorption_ior;
    return out;
}
// Outward plane: dot(normal, x - point) <= 0 is inside.
fn clip_film_plane(interval: vec2f, direction: vec3f, origin: vec3f, normal: vec3f, point: vec3f) -> vec2f {
    let denominator = dot(normal, direction);
    let bound = dot(normal, point-origin);
    if (denominator == 0.0) {
        if (bound < 0.0) { discard; }
        return interval;
    }
    let t = bound / denominator;
    if (denominator < 0.0) { return vec2f(max(interval.x,t), interval.y); }
    return vec2f(interval.x, min(interval.y,t));
}
fn film_interval(input: FilmInput) -> vec2f {
    let a = (camera.view * vec4f(input.a_thickness.xyz,1)).xyz;
    let b = (camera.view * vec4f(input.b.xyz,1)).xyz;
    let c = (camera.view * vec4f(input.c.xyz,1)).xyz;
    let n = normalize(cross(b-a,c-a));
    let direction = ray(input.position.xy);
    let scene_z = scene_depth_at(input.position.xy);
    let scene_t = dot(eye_position(input.position.xy, scene_z)-ray_origin(input.position.xy), direction);
    var interval = vec2f(camera.controls.x / -direction.z, min(camera.controls.y / -direction.z, scene_t));
    interval = clip_film_plane(interval,direction,ray_origin(input.position.xy),-n,a);
    interval = clip_film_plane(interval,direction,ray_origin(input.position.xy),n,a+n*input.a_thickness.w);
    interval = clip_film_plane(interval,direction,ray_origin(input.position.xy),cross(b-a,n),a);
    interval = clip_film_plane(interval,direction,ray_origin(input.position.xy),cross(c-b,n),b);
    interval = clip_film_plane(interval,direction,ray_origin(input.position.xy),cross(a-c,n),c);
    if (interval.y <= interval.x) { discard; }
    return interval;
}
@fragment
fn fs_film_depth(input: FilmInput) -> DepthOutput {
    let interval = film_interval(input);
    let position = ray_origin(input.position.xy)+ray(input.position.xy) * interval.x;
    let clip = camera.projection * vec4f(position,1);
    var out: DepthOutput;
    out.depth_radius = vec2f(-position.z, input.a_thickness.w);
    out.material = input.material;
    out.depth = clip.z / clip.w;
    return out;
}
@fragment
fn fs_film_thickness(input: FilmInput) -> PathOutput {
    let interval = film_interval(input);
    let path = (interval.y-interval.x) / camera.viewport.z;
    var out: PathOutput;
    out.thickness = path;
    out.optical_depth = vec4f(input.material.xyz * path, 0);
    return out;
}

@group(0) @binding(7) var depth_sampler: sampler;
fn scene_depth_at(pixel: vec2f) -> f32 {
    let uv = (vec2f(pixel_index(pixel)) + 0.5) / camera.viewport.xy;
    return textureSampleLevel(depth, depth_sampler, uv, 0);
}
