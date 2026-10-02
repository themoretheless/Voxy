struct Camera { inverse_current: mat4x4<f32>, current: mat4x4<f32>, previous: mat4x4<f32>, settings: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var depth_sampler: sampler;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>,3>(vec2<f32>(-1.0,-1.0),vec2<f32>(3.0,-1.0),vec2<f32>(-1.0,3.0));
    return vec4<f32>(positions[index],0.0,1.0);
}
@fragment fn fs_main(@builtin(position) pixel: vec4<f32>) -> @location(0) vec2<f32> {
    let uv = pixel.xy/vec2<f32>(textureDimensions(depth));
    let z = textureSampleLevel(depth,depth_sampler,uv,0);
    if z == camera.settings.x || camera.settings.y != 0.0 { return vec2<f32>(0.0); }
    let world = camera.inverse_current * vec4<f32>(uv*vec2<f32>(2.0,-2.0)+vec2<f32>(-1.0,1.0),z,1.0);
    if world.w == 0.0 { return vec2<f32>(0.0); }
    let position = vec4<f32>(world.xyz/world.w,1.0);
    let current = camera.current * position;
    let previous = camera.previous * position;
    if current.w <= 0.0 || previous.w <= 0.0 { return vec2<f32>(0.0); }
    let previous_uv = previous.xy/previous.w*vec2<f32>(0.5,-0.5)+vec2<f32>(0.5);
    let current_uv = current.xy/current.w*vec2<f32>(0.5,-0.5)+vec2<f32>(0.5);
    let motion = previous_uv-current_uv;
    if all(abs(motion) <= vec2<f32>(65504.0)) { return motion; }
    return vec2<f32>(0.0);
}
