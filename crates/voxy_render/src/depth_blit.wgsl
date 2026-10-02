override TONE_MAPPING: bool = false;
override EXPOSURE: f32 = 1.0;
@group(0) @binding(0) var source: texture_depth_2d;
@group(0) @binding(1) var source_sampler: sampler_comparison;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var output: VertexOutput;
    let position = positions[index];
    output.position = vec4(position, 0.0, 1.0);
    output.uv = vec2((position.x + 1.0) * 0.5, (1.0 - position.y) * 0.5);
    return output;
}
@fragment fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let size = textureDimensions(source);
    let pixel = clamp(vec2<i32>(input.uv * vec2<f32>(size)), vec2<i32>(0), vec2<i32>(size) - vec2<i32>(1));
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(size);
    var low = 0.0;
    var high = 1.0;
    for (var step = 0; step < 24; step++) {
        let middle = (low + high) * 0.5;
        if textureSampleCompareLevel(source, source_sampler, uv, middle) > 0.5 {
            low = middle;
        } else {
            high = middle;
        }
    }
    let depth = (low + high) * 0.5 * EXPOSURE;
    let value = select(depth, clamp(depth, 0.0, 1.0), TONE_MAPPING);
    return vec4<f32>(value, value, value, 1.0);
}
