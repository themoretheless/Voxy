@group(0) @binding(0) var<uniform> F: Frame;
@group(0) @binding(1) var transLUT: texture_2d<f32>;
@group(0) @binding(2) var skyLUT: texture_2d<f32>;
@group(0) @binding(3) var sampC: sampler;
@group(0) @binding(4) var<storage, read_write> sunVis: array<f32, 4>;

var<workgroup> part: array<f32, 8>;
// one ray per thread (7 rays across the disc), then a tiny reduction
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) k: u32) {
  let sd = F.sunDir.xyz;
  let tx = normalize(cross(sd, vec3f(0.0, 1.0, 0.0)));
  let ty = cross(tx, sd);
  var t = 1.0;
  if (k < 7u) {
    let a = f32(k) * 0.8975979;
    let o = select(vec2f(cos(a), sin(a)) * 0.65, vec2f(0.0), k == 0u);
    let dir = normalize(sd + (tx * o.x + ty * o.y) * SUN_ANG_R);
    let cl = cloudLayers(dir, 0.0, vec2f(-1.0), &t);
  }
  part[k] = t;
  workgroupBarrier();
  if (k == 0u) {
    var acc = 0.0;
    for (var i = 0u; i < 7u; i++) { acc += part[i]; }
    sunVis[0] = acc / 7.0;
  }
}
