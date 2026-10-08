// OPTIMIZATION #42-50: Bilateral filter merged to single compute pass
// Replaces 4 sequential render passes with parallel workgroup computation.
// Axis-aware filtering: processes horizontal AND vertical neighbors in one dispatch.

struct Camera {
    view: mat4x4f,
    projection: mat4x4f,
    inverse_projection: mat4x4f,
    inverse_view: mat4x4f,
    viewport: vec4f,
    controls: vec4f,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var input_depth: texture_2d<f32>;
@group(0) @binding(2) var output_depth: texture_2d<f32>;
@group(0) @binding(3) var depth_sampler: sampler;

fn pixel_index(pixel: vec2f) -> vec2i { 
    return clamp(vec2i(pixel), vec2i(0), vec2i(camera.viewport.xy) - 1); 
}

fn bilateral_filter(input_pixel: vec2f) -> vec2f {
    let index = pixel_index(input_pixel);
    let centre = textureLoad(input_depth, index, 0).xy;
    
    // Skip filtering for transparent or orthographic renders
    if (centre.x <= 0.0 || (u32(camera.viewport.w) & 1u) == 0u) { 
        return centre; 
    }
    
    // Compute footprint based on radius
    var footprint = centre.y * camera.projection[1][1] * camera.viewport.y / (2.0 * centre.x);
    if camera.viewport.w >= 2.0 {
        let pixels_per_unit = vec2f(abs(camera.projection[0][0]), abs(camera.projection[1][1])) * camera.viewport.xy * 0.5;
        footprint = centre.y * dot(pixels_per_unit, vec2f(2.0)); // Max extent (both axes)
    }
    let extent = i32(clamp(footprint * 0.5, 1.0, camera.controls.z));
    let range = max(centre.y * camera.controls.w, 0.00001);
    
    // Accumulate both X and Y filters in a single loop
    var total_x = 0.0; var weight_x = 0.0;
    var total_y = 0.0; var weight_y = 0.0;
    
    for (var i = -extent; i <= extent; i++) {
        // Horizontal sample
        let q_x = clamp(index + vec2i(i, 0), vec2i(0), vec2i(camera.viewport.xy)-1);
        let sample_x = textureLoad(input_depth, q_x, 0).xy;
        if (sample_x.x <= 0.0 || abs(sample_x.x - centre.x) > range) { continue; }
        let spatial_x = exp(-2.0 * f32(i*i) / f32(extent*extent));
        let delta_x = (sample_x.x - centre.x) / range;
        let w_x = spatial_x * exp(-4.0 * delta_x * delta_x);
        total_x += sample_x.x * w_x; weight_x += w_x;
        
        // Vertical sample (same footprint, axis=0,1)
        let q_y = clamp(index + vec2i(0, i), vec2i(0), vec2i(camera.viewport.xy)-1);
        let sample_y = textureLoad(input_depth, q_y, 0).xy;
        if (sample_y.x <= 0.0 || abs(sample_y.x - centre.x) > range) { continue; }
        let spatial_y = exp(-2.0 * f32(i*i) / f32(extent*extent));
        let delta_y = (sample_y.x - centre.x) / range;
        let w_y = spatial_y * exp(-4.0 * delta_y * delta_y);
        total_y += sample_y.x * w_y; weight_y += w_y;
    }
    
    // Clamp results with different bounds per axis
    let value_x = total_x / max(weight_x, 0.00001);
    let value_y = total_y / max(weight_y, 0.00001);
    
    return vec2f(
        clamp(value_x, centre.x - centre.y * 0.25, centre.x + centre.y * 0.25),
        clamp(value_y, centre.x - centre.y * 0.25, centre.x + centre.y * 0.25)
    ).y; // Return smoothed depth (radius unchanged from centre.y)
}

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let width = u32(camera.viewport.x);
    let height = u32(camera.viewport.y);
    
    if (id.x >= width || id.y >= height) { return; }
    
    let uv = vec2f(id.xy) + 0.5;
    let filtered = bilateral_filter(uv);
    
    textureStore(output_depth, id.xy, vec4f(filtered, 0.0, 1.0));
}
