// Interactive ripples: the 2D wave equation on a camera-centred, toroidally addressed
// grid. The texel for world cell g lives at g mod RES, so the window can follow the eye
// without copying; cells that scroll in start at rest. r = height, g = previous height.
const RES: i32 = 256;
const MAX_IMPULSES: u32 = 64u;
struct Ripple {
  snap: vec4<i32>,   // current window centre cell (x, z), previous centre (x, z)
  step: vec4<f32>,   // cell size (m), (c dt / dx)^2, damping per step, impulse count
  mask_min: vec4<f32>, // xz of a rectangle that confines the water, z = 1 when used
  mask_max: vec4<f32>,
};
@group(0) @binding(0) var<uniform> U: Ripple;
@group(0) @binding(1) var previous: texture_2d<f32>;
@group(0) @binding(2) var next: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<storage, read> impulses: array<vec4<f32>>; // x, z, radius, height

fn wrap(i: i32, lo: i32) -> i32 { return lo + (((i - lo) % RES) + RES) % RES; }

// Height of world cell `g` if it lay inside last step's window.
fn state(g: vec2<i32>) -> vec2<f32> {
  let lo = U.snap.zw - vec2<i32>(RES / 2);
  if any(g < lo) || any(g >= lo + vec2<i32>(RES)) { return vec2<f32>(0.); }
  let a = ((g % RES) + RES) % RES;
  return textureLoad(previous, a, 0).rg;
}

fn wet(p: vec2<f32>) -> bool {
  return U.mask_min.z < 0.5 || (all(p >= U.mask_min.xy) && all(p <= U.mask_max.xy));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if id.x >= u32(RES) || id.y >= u32(RES) { return; }
  let lo = U.snap.xy - vec2<i32>(RES / 2);
  let g = vec2<i32>(wrap(i32(id.x), lo.x), wrap(i32(id.y), lo.y));
  let p = (vec2<f32>(g) + 0.5) * U.step.x;
  if !wet(p) {
    textureStore(next, vec2<i32>(id.xy), vec4<f32>(0., 0., 0., 1.));
    return;
  }
  let c = state(g);
  // reflecting walls: dry neighbours mirror the centre cell
  var lap = -4. * c.x;
  for (var k = 0; k < 4; k++) {
    let o = array<vec2<i32>, 4>(vec2<i32>(1, 0), vec2<i32>(-1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1))[k];
    let q = g + o;
    lap += select(c.x, state(q).x, wet((vec2<f32>(q) + 0.5) * U.step.x));
  }
  var h = (2. * c.x - c.y + U.step.y * lap) * U.step.z;
  for (var i = 0u; i < min(u32(U.step.w), MAX_IMPULSES); i++) {
    let imp = impulses[i];
    let d = distance(p, imp.xy) / max(imp.z, 1e-3);
    h += imp.w * exp(-d * d);
  }
  textureStore(next, vec2<i32>(id.xy), vec4<f32>(clamp(h, -0.5, 0.5), c.x, 0., 1.));
}
