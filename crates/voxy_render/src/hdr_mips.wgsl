@group(0) @binding(0) var source: texture_2d<f32>;
struct VertexOutput { @builtin(position) position: vec4<f32> }
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let positions = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    out.position = vec4(positions[index], 0.0, 1.0);
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = textureDimensions(source);
    let destination = max(size / 2u, vec2(1u));
    let p = vec2<u32>(in.position.xy);
    let lo = p * size / destination;
    let hi = (p + 1u) * size / destination;
    var sum = vec4(0.0);
    for (var y = lo.y; y < hi.y; y++) {
        for (var x = lo.x; x < hi.x; x++) {
            sum += textureLoad(source, vec2<i32>(i32(x), i32(y)), 0);
        }
    }
    return sum / f32((hi.x - lo.x) * (hi.y - lo.y));
}
