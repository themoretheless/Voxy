const N: u32 = 256u;
const LOGN: u32 = 8u;
const PI: f32 = 3.14159265358979;
var<workgroup> wa: array<vec4f, 256>;
var<workgroup> wb: array<vec4f, 256>;
fn cmul(a: vec2f, b: vec2f) -> vec2f { return vec2f(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x); }
fn bitrev(i: u32) -> u32 { return reverseBits(i) >> (32u - LOGN); }
// In-place radix-2 inverse FFT over shared memory (input already bit-reversed).
fn butterflies(t: u32) {
  for (var s = 0u; s < LOGN; s++) {
    let half = 1u << s;
    let pos = t & (half - 1u);
    let i0 = ((t >> s) << (s + 1u)) + pos;
    let i1 = i0 + half;
    let ang = PI * f32(pos) / f32(half);
    let w = vec2f(cos(ang), sin(ang));
    let a0 = wa[i0]; let a1 = wa[i1];
    let b0 = wb[i0]; let b1 = wb[i1];
    let ta = vec4f(cmul(a1.xy, w), cmul(a1.zw, w));
    let tb = vec4f(cmul(b1.xy, w), cmul(b1.zw, w));
    wa[i0] = a0 + ta; wa[i1] = a0 - ta;
    wb[i0] = b0 + tb; wb[i1] = b0 - tb;
    workgroupBarrier();
  }
}
