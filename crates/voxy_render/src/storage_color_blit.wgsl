@group(0) @binding(0) var<storage, read> colors: array<u32>;
@group(0) @binding(1) var<uniform> dimensions: vec4<u32>;
@vertex fn vs_main(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(positions[vertex], 0.0, 1.0);
}
@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<u32>(position.xy);
    if pixel.x >= dimensions.y || pixel.y >= dimensions.z { discard; }
    let base = dimensions.x + 4u * (pixel.y * dimensions.y + pixel.x);
    return vec4(bitcast<f32>(colors[base]), bitcast<f32>(colors[base + 1u]),
        bitcast<f32>(colors[base + 2u]), bitcast<f32>(colors[base + 3u]));
}
