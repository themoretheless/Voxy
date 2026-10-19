@group(0) @binding(0) var<storage, read> inA: array<vec4f>;
@group(0) @binding(1) var<storage, read> inB: array<vec4f>;
@group(0) @binding(2) var<storage, read_write> outA: array<vec4f>;
@group(0) @binding(3) var<storage, read_write> outB: array<vec4f>;

@compute @workgroup_size(128)
fn main(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) t: u32) {
  let base = (wg.z * N + wg.x) * N;
  let e0 = t;
  let e1 = t + N / 2u;
  wa[bitrev(e0)] = inA[base + e0];
  wa[bitrev(e1)] = inA[base + e1];
  wb[bitrev(e0)] = inB[base + e0];
  wb[bitrev(e1)] = inB[base + e1];
  workgroupBarrier();
  butterflies(t);
  outA[base + e0] = wa[e0];
  outA[base + e1] = wa[e1];
  outB[base + e0] = wb[e0];
  outB[base + e1] = wb[e1];
}
