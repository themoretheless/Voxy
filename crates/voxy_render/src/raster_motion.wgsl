struct Camera { current: mat4x4<f32>, previous: mat4x4<f32>, reset: vec4<u32> }
@group(0) @binding(0) var<uniform> camera: Camera;
struct Vertex { @builtin(position) clip: vec4<f32>, @location(0) current: vec3<f32>, @location(1) previous: vec3<f32> }
@vertex fn vs_main(@location(0) current: vec3<f32>, @location(1) previous: vec3<f32>) -> Vertex {
    var out: Vertex;
    out.clip = camera.current * vec4<f32>(current,1.0);
    out.current = current; out.previous = previous;
    return out;
}
@fragment fn fs_main(in: Vertex) -> @location(0) vec2<f32> {
    if camera.reset.x != 0u { return vec2<f32>(0.0); }
    let current = camera.current * vec4<f32>(in.current,1.0);
    let previous = camera.previous * vec4<f32>(in.previous,1.0);
    if current.w <= 0.0 || previous.w <= 0.0 { return vec2<f32>(0.0); }
    let motion = (previous.xy/previous.w-current.xy/current.w)*vec2<f32>(0.5,-0.5);
    if all(abs(motion) <= vec2<f32>(65504.0)) { return motion; }
    return vec2<f32>(0.0);
}
