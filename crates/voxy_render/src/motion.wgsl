// Backward motion in normalized top-left texture coordinates (previous - current).
// Use unjittered MVPs; multiply by output dimensions for pixel-space motion.
struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
struct MotionVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) current_clip: vec4<f32>,
    @location(1) previous_clip: vec4<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>) -> MotionVertex {
    var out: MotionVertex;
    out.current_clip = transform.mvp * vec4<f32>(position, 1.0);
    out.previous_clip = transform.previous_mvp * vec4<f32>(position, 1.0);
    out.position = out.current_clip;
    return out;
}
@fragment fn fs_main(in: MotionVertex) -> @location(0) vec4<f32> {
    if in.current_clip.w <= 0.0 || in.previous_clip.w <= 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let current = in.current_clip.xy / in.current_clip.w;
    let previous = in.previous_clip.xy / in.previous_clip.w;
    let motion = (previous - current) * vec2<f32>(0.5, -0.5);
    return vec4<f32>(motion, 0.0, 1.0);
}
