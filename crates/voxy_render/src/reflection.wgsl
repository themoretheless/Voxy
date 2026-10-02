enable wgpu_ray_query;
struct SpecularRay { origin_bias: vec4<f32>, direction_distance: vec4<f32> }
@group(0) @binding(0) var scene: acceleration_structure;
@group(0) @binding(1) var<storage, read> rays: array<SpecularRay>;
@group(0) @binding(2) var distances: texture_storage_2d<r32float, write>;
@group(0) @binding(3) var<storage, read> emission: array<vec4<f32>>;
@group(0) @binding(4) var radiance: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var<storage, read> throughput: array<vec4<f32>>;
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(distances);
    if id.x >= size.x || id.y >= size.y { return; }
    let index = id.y * size.x + id.x;
    if index >= arrayLength(&rays) { return; }
    let weight_index = select(0u, index, arrayLength(&throughput) > 1u);
    if throughput[weight_index].w == 0.0 {
        textureStore(distances, vec2<i32>(id.xy), vec4<f32>(0.0,0.0,0.0,1.0));
        textureStore(radiance, vec2<i32>(id.xy), vec4<f32>(0.0,0.0,0.0,1.0));
        return;
    }
    let ray = rays[index];
    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(0u,255u,ray.origin_bias.w,ray.direction_distance.w,ray.origin_bias.xyz,ray.direction_distance.xyz));
    rayQueryProceed(&query);
    let hit = rayQueryGetCommittedIntersection(&query);
    let distance = select(0.0, hit.t, hit.kind != 0u);
    textureStore(distances, vec2<i32>(id.xy), vec4<f32>(distance,0.0,0.0,1.0));
    var color = vec3<f32>(0.0);
    if hit.kind != 0u && hit.primitive_index < arrayLength(&emission) {
        color = emission[hit.primitive_index].rgb;
    }
    color *= throughput[weight_index].rgb;
    textureStore(radiance, vec2<i32>(id.xy), vec4<f32>(color,1.0));
}
