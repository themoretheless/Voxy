const SKY_W: f32 = 256.0;
const SKY_H: f32 = 144.0;

fn camR() -> f32 { return RG + F.atmo.y; }

fn sampleTrans(r: f32, mu: f32) -> vec3f {
  return textureSampleLevel(transLUT, sampC, transRMuToUV(r, mu), 0.0).rgb;
}

fn sunIlluminance() -> vec3f {
  return sampleTrans(camR(), F.sunDir.y) * F.sunDir.w;
}

fn skyViewUV(dir: vec3f) -> vec2f {
  let r = camR();
  let vHorizon = rhoOf(r);
  let cosBeta = vHorizon / r;
  let beta = acos(clamp(cosBeta, -1.0, 1.0));
  let zha = PI - beta;
  let vzc = clamp(dir.y, -1.0, 1.0);
  var uv: vec2f;
  if (vzc >= -cosBeta) {
    var coord = acos(vzc) / zha;
    coord = 1.0 - coord;
    coord = sqrt(max(coord, 0.0));
    coord = 1.0 - coord;
    uv.y = coord * 0.5;
  } else {
    var coord = (acos(vzc) - zha) / beta;
    coord = sqrt(max(coord, 0.0));
    uv.y = coord * 0.5 + 0.5;
  }
  let dl = length(dir.xz);
  let sl = length(F.sunDir.xz);
  var lvc = 1.0;
  if (dl > 1e-6 && sl > 1e-6) { lvc = clamp(dot(dir.xz, F.sunDir.xz) / (dl * sl), -1.0, 1.0); }
  uv.x = sqrt(max(-lvc * 0.5 + 0.5, 0.0));
  return vec2f(fromUnitToSubUvs(uv.x, SKY_W), fromUnitToSubUvs(uv.y, SKY_H));
}

fn skyAtmo(dir: vec3f) -> vec3f {
  return textureSampleLevel(skyLUT, sampC, skyViewUV(dir), 0.0).rgb * F.sunDir.w;
}
