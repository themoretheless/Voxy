@group(0) @binding(0) var dst: texture_storage_2d<rgba8unorm, write>;
const SIZE: f32 = 512.0;

fn h2(i: vec2i, salt: u32) -> vec2f {
  let h = pcgHash((u32(i.x) * 73856093u) ^ pcgHash(u32(i.y) * 19349663u + salt));
  let h2v = pcgHash(h + 0x9e3779b9u);
  return vec2f(f32(h >> 8u), f32(h2v >> 8u)) / 16777216.0;
}
// "swiss cheese" foam: 1 in foam material, 0 inside bubbles; per-bubble radius
fn bubbles(uv: vec2f, period: i32, salt: u32, rmin: f32, rmax: f32) -> f32 {
  let w = vec2f(pfbm(uv * f32(period) * 0.5, period / 2, 3), pfbm(uv * f32(period) * 0.5 + 17.0, period / 2, 3));
  let p = uv * f32(period) + w * 0.9;
  let ip = vec2i(floor(p));
  let fp = p - floor(p);
  var v = 9.0;
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let o = vec2i(i, j);
      let cell = ((ip + o) % period + period) % period;
      let hh = h2(cell, salt);
      let r = vec2f(o) + hh * 0.9 + 0.05 - fp;
      let rad = mix(rmin, rmax, h2(cell, salt + 7u).x);
      v = min(v, length(r) / rad);
    }
  }
  return smoothstep(0.8, 1.25, v);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let uv = (vec2f(id.xy) + 0.5) / SIZE;
  // irregular holes from warped fbm (big sheets with ragged openings) ...
  let w1 = vec2f(pfbm(uv * 5.0, 5, 3), pfbm(uv * 5.0 + vec2f(3.7, 9.1), 5, 3));
  let m1 = smoothstep(-0.14, 0.06, pfbm(uv * 10.0 + w1 * 1.4, 10, 5));
  let w2 = vec2f(pfbm(uv * 12.0 + 1.3, 12, 2), pfbm(uv * 12.0 + 7.9, 12, 2));
  let m2 = smoothstep(-0.16, 0.04, pfbm(uv * 27.0 + w2 * 0.9, 27, 4));
  // ... plus small round bubbles
  let b3 = bubbles(uv, 29, 37u, 0.06, 0.45);
  let b4 = bubbles(uv, 61, 53u, 0.05, 0.42);
  let dens = pfbm(uv * 4.0, 4, 5) * 0.5 + 0.5;
  let fineD = pfbm(uv * 16.0, 16, 3) * 0.5 + 0.5;
  let big = m1 * (0.55 + 0.45 * m2) * (0.75 + 0.25 * b3);
  let fine = 0.45 + 0.55 * b3 * (0.7 + 0.3 * b4);
  let streak = pfbm(uv * vec2f(4.0, 16.0), 4, 5) * 0.5 + 0.5;
  textureStore(dst, id.xy, vec4f(clamp(big * 0.6 + dens * 0.4, 0.0, 1.0), clamp(fine * 0.8 + fineD * 0.2, 0.0, 1.0), dens, streak));
}
