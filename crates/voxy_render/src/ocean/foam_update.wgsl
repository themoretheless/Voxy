const RES: i32 = 1024;
struct FoamU {
  snap: array<vec4i, 2>,     // per level: current camera texel (x, z), previous (x, z)
  p0: vec4f,                 // texel size level 0, level 1, dt, decay rate
  p1: vec4f,                 // J threshold, gain, growth, -
  cascL: array<vec4f, 2>,
  cascInvL: array<vec4f, 2>,
  drift: vec4f,              // foam drift velocity (m/s) xz
};
@group(0) @binding(0) var<uniform> U: FoamU;
@group(0) @binding(1) var prevTex: texture_2d_array<f32>;
@group(0) @binding(2) var dst: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var dispTex: texture_2d_array<f32>;
@group(0) @binding(4) var derivTex: texture_2d_array<f32>;
@group(0) @binding(5) var samp: sampler;
@group(0) @binding(6) var sampPrev: sampler;

fn wrapIdx(i: i32, lo: i32) -> i32 { return lo + (((i - lo) % RES) + RES) % RES; }

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let lvl = id.z;
  let sn = U.snap[lvl];
  let ts = select(U.p0.x, U.p0.y, lvl == 1u);
  let lo = sn.xy - vec2i(RES / 2);
  let g = vec2i(wrapIdx(i32(id.x), lo.x), wrapIdx(i32(id.y), lo.y));
  let plo = sn.zw - vec2i(RES / 2);
  let inPrev = all(g >= plo) && all(g < plo + vec2i(RES));
  let p = (vec2f(g) + 0.5) * ts;
  var jxx = 0.0; var jzz = 0.0; var jxz = 0.0;
  for (var c = 0u; c < 6u; c++) {
    let texel = U.cascL[c >> 2u][c & 3u] / 256.0;
    let lod = log2(max(ts / texel, 1.0));
    if (lod < 7.5) {
      let uv = p * U.cascInvL[c >> 2u][c & 3u];
      let d = textureSampleLevel(derivTex, samp, uv, c, lod);
      let a = textureSampleLevel(dispTex, samp, uv, c, lod);
      jxx += d.z; jzz += d.w; jxz += a.w;
    }
  }
  let J = (1.0 + jxx) * (1.0 + jzz) - jxz * jxz;
  var prev = vec4f(0.0);
  if (inPrev) {
    // semi-Lagrangian advection: foam slowly drifts downwind (and spreads a little)
    let back = (vec2f(id.xy) + 0.5 - U.drift.xy * U.p0.z / ts) / f32(RES);
    prev = textureSampleLevel(prevTex, sampPrev, back, lvl, 0.0);
  }
  let dt = U.p0.z;
  let inject = clamp((U.p1.x - J) * U.p1.y, 0.0, 1.0);
  var f = prev.r * exp(-dt * U.p0.w);
  f = max(f, inject);
  f = min(f + inject * dt * U.p1.z, 1.6);
  // g: freshness (recently injected) used for brighter, denser crest foam
  let fresh = max(prev.g * exp(-dt * 1.6), inject);
  textureStore(dst, id.xy, lvl, vec4f(f, fresh, J, 1.0));
}
