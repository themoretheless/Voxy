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
struct FullInput { @builtin(position) position: vec4f }
@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> FullInput {
    let corners = array<vec2f, 3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
    var out: FullInput; out.position = vec4f(corners[index], 0, 1); return out;
}

@group(0) @binding(1) var input_depth: texture_2d<f32>;
fn filter_depth(pixel: vec2f, axis: vec2i) -> vec2f {
    let index = pixel_index(pixel);
    let centre = textureLoad(input_depth, index, 0).xy;
    if (centre.x <= 0.0 || (u32(camera.viewport.w) & 1u) == 0u) { return centre; }
    var footprint = centre.y * camera.projection[1][1] * camera.viewport.y / (2.0 * centre.x);
    if camera.viewport.w >= 2.0 {
        let pixels_per_unit = vec2f(abs(camera.projection[0][0]),abs(camera.projection[1][1])) * camera.viewport.xy * 0.5;
        footprint = centre.y * dot(pixels_per_unit,vec2f(abs(axis)));
    }
    let extent = i32(clamp(footprint * 0.5, 1.0, camera.controls.z));
    let range = max(centre.y * camera.controls.w, 0.00001);
    var total = 0.0; var weight = 0.0;
    for (var i = -extent; i <= extent; i++) {
        let q = clamp(index + axis*i, vec2i(0), vec2i(camera.viewport.xy)-1);
        let sample = textureLoad(input_depth, q, 0).xy;
        if (sample.x <= 0.0 || abs(sample.x-centre.x) > range) { continue; }
        let spatial = exp(-2.0 * f32(i*i) / f32(extent*extent));
        let delta = (sample.x-centre.x)/range;
        let w = spatial * exp(-4.0*delta*delta);
        total += sample.x*w; weight += w;
    }
    // Keep disconnected silhouettes empty and bound movement to a fraction of a radius.
    let value = total / max(weight, 0.00001);
    return vec2f(clamp(value, centre.x-centre.y*0.25, centre.x+centre.y*0.25), centre.y);
}
@fragment fn fs_filter_x(input: FullInput) -> @location(0) vec2f { return filter_depth(input.position.xy, vec2i(1,0)); }
@fragment fn fs_filter_y(input: FullInput) -> @location(0) vec2f { return filter_depth(input.position.xy, vec2i(0,1)); }
