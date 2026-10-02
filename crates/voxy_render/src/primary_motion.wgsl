struct Camera { current: mat4x4<f32>, previous: mat4x4<f32>, dimensions_reset: vec4<u32>, position_origin: vec4<f32> }
struct Surface { position_valid: vec4<f32>, normal_roughness: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> surfaces: array<Surface>;
@group(0) @binding(2) var<storage, read> models: array<mat4x4<f32>>;
@group(0) @binding(3) var object_ids: texture_2d<u32>;
@group(0) @binding(4) var previous_positions: texture_2d<f32>;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>,3>(vec2<f32>(-1.0,-1.0),vec2<f32>(3.0,-1.0),vec2<f32>(-1.0,3.0));
    return vec4<f32>(positions[index],0.0,1.0);
}
struct MotionResult { @location(0) motion: vec2<f32>, @location(1) depth: f32 }
fn invalid_motion()->MotionResult {return MotionResult(vec2<f32>(0.0),0.0);}
@fragment fn fs_main(@builtin(position) pixel: vec4<f32>) -> MotionResult {
    let index = u32(pixel.y) * camera.dimensions_reset.x + u32(pixel.x);
    let surface = surfaces[index];
    if surface.position_valid.w != 1.0 || camera.dimensions_reset.z != 0u { return invalid_motion(); }
    let current = camera.current * vec4<f32>(surface.position_valid.xyz,1.0);
    var previous_world = vec4<f32>(surface.position_valid.xyz,1.0);
    if camera.dimensions_reset.w == 2u {
        let correspondence = textureLoad(previous_positions,vec2<i32>(pixel.xy),0);
        if correspondence.w != 1.0 { return invalid_motion(); }
        previous_world = vec4<f32>(correspondence.xyz+camera.position_origin.xyz,1.0);
    } else if camera.dimensions_reset.w == 1u {
        let object = textureLoad(object_ids,vec2<i32>(pixel.xy),0).r;
        if object >= arrayLength(&models) { return invalid_motion(); }
        previous_world = models[object] * previous_world;
    }
    let previous = camera.previous * previous_world;
    if current.w <= 0.0 || previous.w <= 0.0 { return invalid_motion(); }
    let motion = (previous.xy / previous.w - current.xy / current.w) * vec2<f32>(0.5,-0.5);
    if all(abs(motion) <= vec2<f32>(65504.0)) {
        let depth=previous.z/previous.w;
        return MotionResult(motion,select(0.0,depth,depth>0.0 && depth<1.0));
    }
    return invalid_motion();
}
