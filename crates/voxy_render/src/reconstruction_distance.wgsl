@group(0) @binding(0) var distance: texture_2d<f32>;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>,3>(vec2<f32>(-1.0,-1.0),vec2<f32>(3.0,-1.0),vec2<f32>(-1.0,3.0));
    return vec4<f32>(positions[index],0.0,1.0);
}
@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) f32 {
    return textureLoad(distance,vec2<i32>(position.xy),0).r;
}
