@group(0) @binding(1) var<storage, read> h0: array<vec4f>;
@group(0) @binding(2) var<storage, read_write> specA: array<vec4f>;
@group(0) @binding(3) var<storage, read_write> specB: array<vec4f>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let c = id.z;
  let idx = (c * N + id.y) * N + id.x;
  let k = waveVec(id.x, id.y, S.L[c >> 2u][c & 3u]);
  let kl = length(k);
  if (kl < 1e-5) {
    specA[idx] = vec4f(0.0);
    specB[idx] = vec4f(0.0);
    return;
  }
  let w0 = 2.0 * PI / S.misc.z;
  let w = floor(omegaOf(kl) / w0) * w0;
  let ph = w * S.timeP.x;
  let e = vec2f(cos(ph), sin(ph));
  let H0 = h0[idx];
  // h0(k) travels along +k: h(k,t) = h0(k) e^{-iwt} + conj(h0(-k)) e^{iwt}
  let h = cmul(H0.xy, vec2f(e.x, -e.y)) + cmul(H0.zw, e);
  let ih = mulI(h);
  let lam = S.misc.x;
  let ik = 1.0 / kl;
  let Dx = ih * (lam * k.x * ik);
  let Dz = ih * (lam * k.y * ik);
  let jxz = h * (-lam * k.x * k.y * ik);
  let sx = ih * k.x;
  let sz = ih * k.y;
  let jxx = h * (-lam * k.x * k.x * ik);
  let jzz = h * (-lam * k.y * k.y * ik);
  specA[idx] = vec4f(Dx + mulI(Dz), h + mulI(jxz));
  specB[idx] = vec4f(sx + mulI(sz), jxx + mulI(jzz));
}
