@group(0) @binding(0) var srcA: texture_2d_array<f32>;
@group(0) @binding(1) var srcB: texture_2d_array<f32>;
@group(0) @binding(2) var srcC: texture_2d_array<f32>;
@group(0) @binding(3) var dstA: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var dstB: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var dstC: texture_storage_2d_array<rgba16float, write>;

fn box(t: texture_2d_array<f32>, p: vec2i, l: u32) -> vec4f {
  return 0.25 * (textureLoad(t, p, l, 0) + textureLoad(t, p + vec2i(1, 0), l, 0) +
                 textureLoad(t, p + vec2i(0, 1), l, 0) + textureLoad(t, p + vec2i(1, 1), l, 0));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let size = textureDimensions(dstA);
  if (id.x >= size.x || id.y >= size.y) { return; }
  let p = vec2i(id.xy) * 2;
  textureStore(dstA, id.xy, id.z, box(srcA, p, id.z));
  textureStore(dstB, id.xy, id.z, box(srcB, p, id.z));
  textureStore(dstC, id.xy, id.z, box(srcC, p, id.z));
}
