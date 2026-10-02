// SI coefficients, physical spheres and camera-relative optical reconstruction.
struct Camera {
    view: mat4x4f,
    projection: mat4x4f,
    inverse_projection: mat4x4f,
    inverse_view: mat4x4f,
    viewport: vec4f,
    controls: vec4f,
}
struct Particle { position_radius: vec4f, absorption_ior: vec4f }
@group(0) @binding(0) var<uniform> camera: Camera;
// Particle entrypoints use this layout. Filter and composite entrypoints have
// separate, explicitly supplied layouts with the same binding numbers.
@group(0) @binding(1) var<storage, read> particles: array<Particle>;
@group(0) @binding(2) var scene_depth: texture_depth_2d;

struct SphereInput {
    @builtin(position) position: vec4f,
    @location(0) @interpolate(flat) centre: vec3f,
    @location(1) @interpolate(flat) radius: f32,
    @location(2) @interpolate(flat) material: vec4f,
}
@vertex
fn vs_particle(@builtin(vertex_index) vertex: u32, @builtin(instance_index) index: u32) -> SphereInput {
    let corners = array<vec2f, 6>(vec2f(-1,-1),vec2f(1,-1),vec2f(-1,1),vec2f(-1,1),vec2f(1,-1),vec2f(1,1));
    let p = particles[index];
    let centre = (camera.view * vec4f(p.position_radius.xyz, 1)).xyz;
    let radius = p.position_radius.w;
    let distance = -centre.z;
    var out: SphereInput;
    // Conservative perspective bounds. Near-camera spheres use the full screen.
    let expansion = max(1.0, (distance + length(centre.xy)) / max(distance - radius, 0.00001));
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
fn ray(pixel: vec2f) -> vec3f { return normalize(eye_position(pixel, 1.0)); }
fn pixel_index(pixel: vec2f) -> vec2i { return clamp(vec2i(pixel), vec2i(0), vec2i(camera.viewport.xy) - 1); }
fn sphere_interval(input: SphereInput) -> vec2f {
    let direction = ray(input.position.xy);
    let b = dot(direction, input.centre);
    let discriminant = b*b - dot(input.centre, input.centre) + input.radius*input.radius;
    if (discriminant <= 0) { discard; }
    let root = sqrt(discriminant);
    let near_t = camera.controls.x / -direction.z;
    let far_t = camera.controls.y / -direction.z;
    let scene_z = textureLoad(scene_depth, pixel_index(input.position.xy), 0);
    let scene_t = dot(eye_position(input.position.xy, scene_z), direction);
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
    let position = ray(input.position.xy) * interval.x;
    let clip = camera.projection * vec4f(position, 1);
    var out: DepthOutput;
    out.depth_radius = vec2f(-position.z, input.radius);
    out.material = input.material;
    out.depth = clip.z / clip.w;
    return out;
}
@fragment
fn fs_thickness(input: SphereInput) -> @location(0) f32 {
    let interval = sphere_interval(input);
    return (interval.y - interval.x) / camera.viewport.z;
}

struct FullInput { @builtin(position) position: vec4f }
@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> FullInput {
    let corners = array<vec2f, 3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
    var out: FullInput; out.position = vec4f(corners[index], 0, 1); return out;
}
