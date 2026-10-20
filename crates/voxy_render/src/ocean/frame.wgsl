struct Frame {
  viewProj: mat4x4f,   // camera-relative view-projection (reverse-Z, infinite)
  camPos:   vec4f,     // xyz metres, w = time (s)
  camRight: vec4f,     // xyz * tan(fovX/2)
  camUp:    vec4f,     // xyz * tan(fovY/2)
  camFwd:   vec4f,     // xyz
  sunDir:   vec4f,     // xyz towards sun, w = solar illuminance scale
  screen:   vec4f,     // w, h, 1/w, 1/h
  cascL:    array<vec4f, 2>,  // cascade patch sizes (m), index [c>>2][c&3]
  cascInvL: array<vec4f, 2>,
  windP:    array<vec4f, 2>,  // [0].xy = wind direction (unit), [0].z = wind speed
  lodP:     vec4f,     // CDLOD base range, grid quads per node, -, -
  post:     vec4f,     // exposure, bloom strength, frame index, -
  atmo:     vec4f,     // haze (mie multiplier), camera altitude km, -, -
  waterCol: vec4f,     // body scatter colour rgb, unresolved slope variance
  sssCol:   vec4f,     // sss colour rgb, sss strength
  foamP:    vec4f,     // J threshold, J gain, foam intensity, lace sharpness
  fogExt:   vec4f,     // rgb extinction per metre near sea level
  cloudP:   vec4f,     // altitude km, coverage, optical density, wind time
};
