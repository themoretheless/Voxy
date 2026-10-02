@group(0) @binding(0) var<uniform> camera: mat4x4<f32>;
struct Vertex { @builtin(position) clip: vec4<f32>, @location(0) previous: vec3<f32> }
@vertex fn vs_main(@location(0) current: vec3<f32>, @location(1) previous: vec3<f32>) -> Vertex {
    var out: Vertex;
    out.clip = camera * vec4<f32>(current,1.0);
    out.previous = previous;
    return out;
}
@fragment fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(in.previous,1.0);
}
