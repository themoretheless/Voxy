override TONE_MAPPING: bool = false;
override LINEAR_EXPOSURE: bool = false;
override OUTPUT_MAX: f32 = 65504.0;
@group(0) @binding(2) var<uniform> auto_exposure: vec4<f32>;
@group(0) @binding(1) var<uniform> display_parameter: vec4<f32>;
@group(0) @binding(0) var source: texture_2d<f32>;
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
    let color = textureLoad(source, pixel, 0);
    let exposure = min(display_parameter.x * auto_exposure.x, 3.402823e38);
    if LINEAR_EXPOSURE {
        return vec4(min(max(color.rgb, vec3(0.0)) * exposure, vec3(OUTPUT_MAX)), color.a);
    }
    if TONE_MAPPING {
        // Saturate overflow before division, preserving low-light precision.
        let exposed = min(max(color.rgb, vec3<f32>(0.0)) * exposure, vec3<f32>(3.402823e38));
        // Keep divisors small: fast GPU division may flush reciprocals of very
        // large HDR values to zero. The low branch avoids cancellation near black.
        let low = exposed / (vec3<f32>(1.0) + exposed);
        let high = vec3<f32>(1.0) / (vec3<f32>(1.0) + vec3<f32>(1.0) / exposed);
        let mapped = select(low, high, exposed > vec3<f32>(1.0));
        return vec4<f32>(mapped, color.a);
    }
    return color;
}
