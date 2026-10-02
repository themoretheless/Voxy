@group(0) @binding(0) var<storage, read> visibility: array<u32>;
@group(0) @binding(1) var<storage, read> radiance: array<vec4<f32>>;
@group(0) @binding(2) var output: texture_storage_2d<rgba16float, write>;
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output);
    if id.x >= size.x || id.y >= size.y { return; }
    let index = id.y * size.x + id.x;
    textureStore(output, vec2<i32>(id.xy), vec4<f32>(radiance[index].rgb * f32(visibility[index]), 1.0));
}
