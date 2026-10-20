@group(0) @binding(0) var src: texture_2d_array<f32>;
@group(0) @binding(1) var dst: texture_storage_2d_array<rgba16float, write>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let size = textureDimensions(dst);
  if (id.x >= size.x || id.y >= size.y) { return; }
  let p = vec2i(id.xy) * 2;
  let v = 0.25 * (textureLoad(src, p, id.z, 0) + textureLoad(src, p + vec2i(1, 0), id.z, 0) +
                  textureLoad(src, p + vec2i(0, 1), id.z, 0) + textureLoad(src, p + vec2i(1, 1), id.z, 0));
  textureStore(dst, id.xy, id.z, v);
}
