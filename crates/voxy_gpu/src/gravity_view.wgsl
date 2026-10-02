// Header occupies 8 words, then each committed body occupies 8 words.
@group(0) @binding(0) var<storage, read> words: array<u32>;
@vertex fn vertex(@builtin(vertex_index) index: u32,
                  @builtin(instance_index) body: u32) -> @builtin(position) vec4<f32> {
    let corners = array<vec2<f32>, 6>(vec2(-0.08,-0.08),vec2(0.08,-0.08),vec2(0.08,0.08),
                                    vec2(-0.08,-0.08),vec2(0.08,0.08),vec2(-0.08,0.08));
    let base = 8u + body * 8u;
    let position = bitcast<vec2<f32>>(vec2<u32>(words[base], words[base+1u]));
    return vec4<f32>(position + corners[index], 0.0, 1.0);
}
@fragment fn fragment() -> @location(0) vec4<f32> { return vec4(0.0,1.0,0.0,1.0); }
