@group(0) @binding(0) var primary: texture_2d<f32>;
@group(0) @binding(1) var reflected: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<rgba16float, write>;
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output);
    if id.x >= size.x || id.y >= size.y { return; }
    let pixel = vec2<i32>(id.xy);
    let color = min(textureLoad(primary, pixel, 0).rgb + textureLoad(reflected, pixel, 0).rgb, vec3<f32>(65504.0));
    textureStore(output, pixel, vec4<f32>(color, 1.0));
}
