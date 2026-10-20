// Shared water functions for renderers built on voxy_water. No bindings.

const WATER_TAU: f32 = 6.2831853;

// Breakers on a beach; mirrors `ShoreWaves::height` in shore.rs.
// Returns (height, slope along the shoreward direction, foam, remaining FFT fraction).
fn shore_wave(depth: f32, distance: f32, time: f32, height: f32, period: f32, reach: f32) -> vec4<f32> {
  let fft_scale = clamp(depth / 2.5, 0., 1.);
  if distance <= -2. || distance > reach || height <= 0. { return vec4<f32>(0., 0., 0., fft_scale); }
  let d = max(depth, 0.02);
  let k0 = WATER_TAU / (9.81 * period * period / WATER_TAU);
  let k = k0 / sqrt(max(tanh(d * k0), 0.05));
  let phase = distance * k + WATER_TAU * time / period;
  let shoaled = height * min(pow(2.5 / d, 0.25), 1.8);
  let h = min(shoaled, 0.78 * d) * clamp(1. - distance / reach, 0., 1.);
  let s = sin(phase) * 0.5 + 0.5;
  let crest = s * s * s;
  let dcrest = 1.5 * s * s * cos(phase) * k;
  let breaking = clamp(shoaled / (0.78 * d) - 0.9, 0., 1.);
  let swash = clamp(1. - abs(distance) / 1.5, 0., 1.);
  let s2 = s * s;
  let s8 = s2 * s2 * s2 * s2;
  return vec4<f32>(h * crest, h * dcrest, clamp(breaking * s8 * 1.3 + swash * 0.6, 0., 1.), fft_scale);
}

// Dual-phase flow advection: two copies of a pattern scrolled with the current, each
// reset every `period` seconds half a period apart and cross-faded, so the pattern
// moves downstream without ever stretching. Returns (age 0, age 1, weight of copy 1).
fn flow_phases(time: f32, period: f32) -> vec3<f32> {
  let a = fract(time / period);
  let b = fract(time / period + 0.5);
  return vec3<f32>(a * period, b * period, abs(1. - 2. * a));
}

fn water_hash(p: vec2<f32>) -> f32 {
  var q = fract(vec3<f32>(p.xyx) * 0.1031);
  q += dot(q, q.yzx + 33.33);
  return fract((q.x + q.y) * q.z);
}

fn water_value_noise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let u = f * f * (3. - 2. * f);
  return mix(mix(water_hash(i), water_hash(i + vec2<f32>(1., 0.)), u.x),
             mix(water_hash(i + vec2<f32>(0., 1.)), water_hash(i + vec2<f32>(1., 1.)), u.x), u.y);
}

// Bubbly white-water coverage for a requested amount 0..1 at surface coordinates `p`.
fn foam_pattern(p: vec2<f32>, amount: f32) -> f32 {
  let n = water_value_noise(p * 2.3) * 0.55 + water_value_noise(p * 7.1) * 0.3 + water_value_noise(p * 19.) * 0.15;
  return smoothstep(1. - amount, 1.15 - amount * 0.6, n) * min(amount * 1.5, 1.);
}
