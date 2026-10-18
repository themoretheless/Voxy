const N: u32 = 256u;
const LOGN: u32 = 8u;
const PI: f32 = 3.14159265358979;
const GRAV: f32 = 9.81;
const SURF: f32 = 7.4e-5;   // surface tension / density

struct Sim {
  L:          array<vec4f, 2>,  // patch size per cascade ([c>>2][c&3])
  kMin:       array<vec4f, 2>,
  kMax:       array<vec4f, 2>,
  foamDecay:  array<vec4f, 2>,  // 1/s per cascade
  foamThresh: array<vec4f, 2>,  // Jacobian threshold per cascade
  wind:       vec4f,  // U10 m/s, direction rad, fetch m, gamma
  swell:      vec4f,  // Hs m, direction rad, peak omega, spreading exponent
  misc:       vec4f,  // choppiness, short wave cut (m), loop period s, seed
  timeP:      vec4f,  // time (looped), dt, foam gain, spectrum gain
};
@group(0) @binding(0) var<uniform> S: Sim;

fn waveVec(x: u32, y: u32, L: f32) -> vec2f {
  let nx = select(i32(x), i32(x) - i32(N), x >= N / 2u);
  let ny = select(i32(y), i32(y) - i32(N), y >= N / 2u);
  return vec2f(f32(nx), f32(ny)) * (2.0 * PI / L);
}
fn omegaOf(k: f32) -> f32 { return sqrt(GRAV * k + SURF * k * k * k); }
fn dOmegaOf(k: f32) -> f32 { return (GRAV + 3.0 * SURF * k * k) / (2.0 * omegaOf(k)); }
fn cmul(a: vec2f, b: vec2f) -> vec2f { return vec2f(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x); }
fn mulI(a: vec2f) -> vec2f { return vec2f(-a.y, a.x); }
