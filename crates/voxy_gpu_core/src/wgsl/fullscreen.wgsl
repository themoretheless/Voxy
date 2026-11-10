// Canonical single-triangle full-screen vertex stage (`0..3` vertices, no vertex buffer).
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    let uv_raw = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var out: FullscreenVertexOutput;
    out.position = vec4<f32>(uv_raw * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv_raw.x, 1.0 - uv_raw.y);
    return out;
}
