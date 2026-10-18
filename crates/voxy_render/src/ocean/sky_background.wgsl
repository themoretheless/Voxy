@group(0) @binding(0) var<uniform> F: Frame;
@group(0) @binding(1) var transLUT: texture_2d<f32>;
@group(0) @binding(2) var skyLUT: texture_2d<f32>;
@group(0) @binding(3) var sampC: sampler;
struct VO { @builtin(position) pos: vec4f, @location(0) ndc: vec2f };
@vertex fn vs(@builtin(vertex_index) i: u32) -> VO {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
  var o: VO;
  o.pos = vec4f(p, 0.0, 1.0);
  o.ndc = p;
  return o;
}
@fragment fn fs(in: VO) -> @location(0) vec4f {
  let dir = normalize(F.camFwd.xyz + in.ndc.x * F.camRight.xyz + in.ndc.y * F.camUp.xyz);
  let pixAng = 2.0 * length(F.camUp.xyz) * F.screen.w;
  var d2 = dir;
  d2.y = max(d2.y, 0.0005);
  d2 = normalize(d2);
  // edge functions at this pixel (uniform control flow, so fwidth is valid)
  let hl = hitLayer(d2, F.cloudP.x);
  let el = lowEdge(hl.q, hl.t * pixAng / hl.mu, 7);
  let hh = hitLayer(d2, 4.5);
  let eh = highEdge(hh.q, hh.t * pixAng / hh.mu, 6);
  let aa = vec2f(fwidth(el), fwidth(eh));
  var sunT = 1.0;
  let cl = cloudLayers(d2, pixAng, aa, &sunT);
  let L = skyAtmo(d2) * (1.0 - cl.a) + cl.rgb + sunDisk(dir) * sunT;
  return vec4f(L, 1.0);
}
