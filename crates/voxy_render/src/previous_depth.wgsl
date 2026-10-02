struct Camera { current:mat4x4<f32>, previous:mat4x4<f32>, reset:vec4<u32> }
@group(0) @binding(0) var<uniform> camera:Camera;
struct Vertex { @builtin(position) clip:vec4<f32>, @location(0) previous:vec3<f32> }
@vertex fn vs_main(@location(0) current:vec3<f32>, @location(1) previous:vec3<f32>)->Vertex {
    var out:Vertex;
    out.clip=camera.current*vec4<f32>(current,1.0);
    out.previous=previous;
    return out;
}
@fragment fn fs_main(in:Vertex)->@location(0) f32 {
    let clip=camera.previous*vec4<f32>(in.previous,1.0);
    let depth=clip.z/clip.w;
    if clip.w>0.0 && depth>0.0 && depth<1.0 {return depth;}
    return 0.0;
}
