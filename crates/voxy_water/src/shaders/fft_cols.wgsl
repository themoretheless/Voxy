@group(0) @binding(0) var<storage, read> inA: array<vec4f>;
@group(0) @binding(1) var<storage, read> inB: array<vec4f>;
@group(0) @binding(2) var dispOut: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var derivOut: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var momOut: texture_storage_2d_array<rgba16float, write>;

fn emit(x: u32, y: u32, c: u32, a: vec4f, b: vec4f) {
  // a = (Dx, Dz, Dy, jxz), b = (sx, sz, jxx, jzz)
  let p = vec2u(x, y);
  textureStore(dispOut, p, c, vec4f(a.x, a.z, a.y, a.w));
  textureStore(derivOut, p, c, b);
  textureStore(momOut, p, c, vec4f(b.x * b.x, b.y * b.y, b.x * b.y, 0.0));
}

@compute @workgroup_size(128)
fn main(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) t: u32) {
  let x = wg.x;
  let c = wg.z;
  let e0 = t;
  let e1 = t + N / 2u;
  let i0 = (c * N + e0) * N + x;
  let i1 = (c * N + e1) * N + x;
  wa[bitrev(e0)] = inA[i0];
  wa[bitrev(e1)] = inA[i1];
  wb[bitrev(e0)] = inB[i0];
  wb[bitrev(e1)] = inB[i1];
  workgroupBarrier();
  butterflies(t);
  emit(x, e0, c, wa[e0], wb[e0]);
  emit(x, e1, c, wa[e1], wb[e1]);
}
