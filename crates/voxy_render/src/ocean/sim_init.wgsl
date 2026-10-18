@group(0) @binding(1) var<storage, read_write> h0: array<vec4f>;

fn rand01(s: u32) -> f32 { return (f32(pcgHash(s) >> 8u) + 0.5) / 16777216.0; }
fn gauss2(seed: u32) -> vec2f {
  let u1 = rand01(seed * 2u + 17u);
  let u2 = rand01(seed * 2u + 18u + pcgHash(seed));
  let r = sqrt(-2.0 * log(u1));
  let a = 2.0 * PI * u2;
  return r * vec2f(cos(a), sin(a));
}
fn wrapAngle(a: f32) -> f32 {
  var x = a;
  x = x - 2.0 * PI * floor((x + PI) / (2.0 * PI));
  return x;
}
fn jonswap(w: f32, wp: f32, alpha: f32, gamma: f32) -> f32 {
  let sigma = select(0.09, 0.07, w <= wp);
  let d = (w - wp) / (sigma * wp);
  let r = exp(-0.5 * d * d);
  let q = wp / w;
  let q2 = q * q;
  return alpha * GRAV * GRAV / (w * w * w * w * w) * exp(-1.25 * q2 * q2) * pow(gamma, r);
}
// Donelan-Banner directional spreading (normalised over [-pi, pi])
fn donelanBanner(theta: f32, w: f32, wp: f32) -> f32 {
  let ratio = w / wp;
  var beta: f32;
  if (ratio < 0.95) {
    beta = 2.61 * pow(ratio, 1.3);
  } else if (ratio < 1.6) {
    beta = 2.28 * pow(ratio, -1.3);
  } else {
    let eps = -0.4 + 0.8393 * exp(-0.567 * log(ratio * ratio));
    beta = pow(10.0, eps);
  }
  let sh = 1.0 / cosh(clamp(beta * theta, -20.0, 20.0));
  return beta / (2.0 * tanh(beta * PI)) * sh * sh;
}
// cos^2s(theta/2) spreading, normalised
fn cosSpread(theta: f32, s: f32) -> f32 {
  let q = sqrt(s + 0.25) / (2.0 * sqrt(PI));
  return q * pow(abs(cos(theta * 0.5)), 2.0 * s);
}

fn spectrum(k: vec2f) -> f32 {
  let kl = length(k);
  let w = omegaOf(kl);
  let theta = atan2(k.y, k.x);
  // fetch-limited wind sea (JONSWAP)
  let U = S.wind.x;
  let F = S.wind.z;
  var Sw = 0.0;
  if (U > 0.1) {
    let alpha = 0.076 * pow(U * U / (F * GRAV), 0.22);
    let wp = 22.0 * pow(GRAV * GRAV / (U * F), 1.0 / 3.0);
    Sw = jonswap(w, wp, alpha, S.wind.w) * donelanBanner(wrapAngle(theta - S.wind.y), w, wp);
  }
  // swell: narrow, long-crested JONSWAP
  if (S.swell.x > 0.001) {
    let ws = S.swell.z;
    let gs = 6.0;
    let hs4 = S.swell.x * 0.25;
    let alphaS = hs4 * hs4 * 5.0 * ws * ws * ws * ws / (GRAV * GRAV) * (1.0 - 0.287 * log(gs));
    Sw += jonswap(w, ws, alphaS, gs) * cosSpread(wrapAngle(theta - S.swell.y), S.swell.w);
  }
  // damp the very shortest waves
  let sc = S.misc.y;
  Sw *= exp(-kl * kl * sc * sc);
  return Sw * dOmegaOf(kl) / kl * S.timeP.w;
}

fn amplitude(k: vec2f, c: u32) -> f32 {
  let kl = length(k);
  if (kl < 1e-5 || kl < S.kMin[c >> 2u][c & 3u] || kl >= S.kMax[c >> 2u][c & 3u]) { return 0.0; }
  let dk = 2.0 * PI / S.L[c >> 2u][c & 3u];
  // the finest cascades are toned down (they otherwise read as crumpled foil)
  let g = select(1.0, select(0.85, 0.6, c == 5u), c >= 4u);
  return sqrt(max(spectrum(k), 0.0)) * dk * 0.5 * g;
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let c = id.z;
  let L = S.L[c >> 2u][c & 3u];
  let k = waveVec(id.x, id.y, L);
  let mx = (N - id.x) % N;
  let my = (N - id.y) % N;
  let seed = u32(S.misc.w) * 7919u + c * 1000003u;
  let a = gauss2(seed + (id.y * N + id.x)) * amplitude(k, c);
  let b = gauss2(seed + (my * N + mx)) * amplitude(-k, c);
  h0[(c * N + id.y) * N + id.x] = vec4f(a, b.x, -b.y);
}
