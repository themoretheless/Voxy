// Enclosed water bodies (pools) for the archive scene, driven by the FFT wave field of
// the ocean renderer. Prepended: archive_environment.wgsl (analytic daylight).
// The surface replaces the opaque radiance behind it: refraction samples the copied
// opaque frame at the refracted depth, with Beer-Lambert absorption, in-scattering and
// first-order caustics on whatever the refracted ray reaches.
struct Camera { vp:mat4x4<f32>, shadow:mat4x4<f32>, inverse_vp:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, sun_colour:vec4<f32>, viewport:vec4<f32>, weather:vec4<f32>, atmosphere:vec4<f32>, wind:vec4<f32>, previous_vp:mat4x4<f32>, previous_inverse_vp:mat4x4<f32>, surface:vec4<f32> }
struct Water {
  body_min: vec4<f32>,    // basin interior min corner, w = edge falloff (m)
  body_max: vec4<f32>,    // basin interior max corner, y = water level, w = 1 when the eye is in this body
  absorption: vec4<f32>,  // rgb 1/m, w = vertical displacement scale
  scatter: vec4<f32>,     // rgb in-scattered fraction of ambient light, w = caustic strength
  casc_inv_l: array<vec4<f32>, 2>,
};
@group(0) @binding(0) var<uniform> camera:Camera;
@group(1) @binding(0) var<uniform> water:Water;
@group(1) @binding(1) var disp_tex:texture_2d_array<f32>;
@group(1) @binding(2) var deriv_tex:texture_2d_array<f32>;
@group(1) @binding(3) var moment_tex:texture_2d_array<f32>;
@group(1) @binding(4) var wave_sampler:sampler;
@group(1) @binding(5) var background:texture_2d<f32>;
@group(1) @binding(6) var opaque_depth:texture_depth_2d;
@group(1) @binding(7) var shadow_map:texture_depth_2d;
@group(1) @binding(8) var shadow_sampler:sampler_comparison;
@group(2) @binding(0) var environment_specular:texture_cube<f32>;
@group(2) @binding(2) var environment_sampler:sampler;
@group(2) @binding(4) var<uniform> environment_intensity:vec4<f32>;

const CASCADES:u32 = 6u;
const IOR:f32 = 1.333;

fn inv_l(c:u32)->f32 { return water.casc_inv_l[c >> 2u][c & 3u]; }

struct VsOut { @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32> };

// Waves fade out within `body_min.w` of the basin walls, so the surface meets the tiles.
fn edge_weight(xz:vec2<f32>)->f32 {
  let d = min(min(xz.x - water.body_min.x, water.body_max.x - xz.x), min(xz.y - water.body_min.z, water.body_max.z - xz.y));
  return smoothstep(0., water.body_min.w, d);
}

@vertex fn vs_surface(@location(0) xz:vec2<f32>)->VsOut {
  var h = 0.;
  for (var c = 0u; c < CASCADES; c++) {
    h += textureSampleLevel(disp_tex, wave_sampler, xz * inv_l(c), c, 0.).y;
  }
  let world = vec3<f32>(xz.x, water.body_max.y + h * water.absorption.w * edge_weight(xz), xz.y);
  var o:VsOut;
  o.world = world;
  o.clip = camera.vp * vec4<f32>(world, 1.);
  return o;
}

struct Slopes { s:vec2<f32>, cov:vec3<f32>, j:vec2<f32> };
fn slopes(xz:vec2<f32>)->Slopes {
  var o:Slopes;
  for (var c = 0u; c < CASCADES; c++) {
    let uv = xz * inv_l(c);
    let d = textureSample(deriv_tex, wave_sampler, uv, c);
    let m = textureSample(moment_tex, wave_sampler, uv, c);
    o.s += d.xy;
    o.j += d.zw;
    o.cov += vec3<f32>(max(m.x - d.x * d.x, 0.), max(m.y - d.y * d.y, 0.), m.z - d.x * d.y);
  }
  return o;
}
// Slopes of the cascades that focus light into visible caustic cells; the finest
// ripples only add sub-centimetre grain, the coarsest barely curve the surface.
fn caustic_slope(xz:vec2<f32>)->vec2<f32> {
  var s = vec2<f32>(0.);
  for (var c = 1u; c < 4u; c++) {
    s += textureSampleLevel(deriv_tex, wave_sampler, xz * inv_l(c), c, 3.).xy;
  }
  return s;
}

fn fresnel(c:f32)->f32 { let x = 1. - clamp(c, 0., 1.); let x2 = x * x; return 0.02 + 0.98 * x2 * x2 * x; }

fn sky(direction:vec3<f32>, roughness:f32)->vec3<f32> {
  if camera.eye.w > 0.5 {
    let lod = roughness * f32(textureNumLevels(environment_specular) - 1u);
    return textureSampleLevel(environment_specular, environment_sampler, direction, lod).rgb * environment_intensity.x;
  }
  return daylight(direction, camera.sun.xyz) * camera.viewport.w;
}

// The copied frame is half float: clamp overflowed highlights so Inf never meets a zero
// weight (Inf * 0 = NaN).
fn opaque_radiance(pixel:vec2<i32>)->vec3<f32> {
  let c = textureLoad(background, pixel, 0).rgb;
  return select(min(c, vec3<f32>(6.0e4)), vec3<f32>(0.), c != c) / camera.viewport.z;
}
fn world_at(pixel:vec2<i32>, depth:f32)->vec3<f32> {
  let ndc = (vec2<f32>(pixel) + 0.5) / camera.viewport.xy * vec2<f32>(2., -2.) + vec2<f32>(-1., 1.);
  let h = camera.inverse_vp * vec4<f32>(ndc, depth, 1.);
  return h.xyz / h.w;
}
fn to_pixel(p:vec3<f32>)->vec3<f32> {
  let q = camera.vp * vec4<f32>(p, 1.);
  return vec3<f32>((q.xy / q.w * vec2<f32>(0.5, -0.5) + 0.5) * camera.viewport.xy, q.z / q.w);
}
fn clamp_pixel(p:vec2<f32>)->vec2<i32> {
  return clamp(vec2<i32>(p), vec2<i32>(0), vec2<i32>(camera.viewport.xy) - 1);
}

fn sun_visible(p:vec3<f32>)->f32 {
  let s = camera.shadow * vec4<f32>(p, 1.);
  let q = s.xyz / s.w;
  let uv = q.xy * vec2<f32>(0.5, -0.5) + 0.5;
  if any(uv < vec2<f32>(0.)) || any(uv > vec2<f32>(1.)) || q.z < 0. || q.z > 1. { return 1.; }
  return textureSampleCompareLevel(shadow_map, shadow_sampler, uv, q.z - 0.0015);
}

// Sunlight refracted through the surface converges or spreads with the curvature of
// the waves: intensity ~ 1/det(I + d (1 - 1/n) H) for a floor `d` metres below.
fn caustic(p:vec3<f32>)->f32 {
  let sun = camera.sun.xyz;
  if sun.y <= 0. || water.scatter.w <= 0. { return 1.; }
  let depth = water.body_max.y - p.y;
  if depth <= 0. { return 1.; }
  let t = refract(-sun, vec3<f32>(0., 1., 0.), 1. / IOR);
  let above = p.xz - t.xz / t.y * depth;
  let e = 0.05;
  let sx = (caustic_slope(above + vec2<f32>(e, 0.)) - caustic_slope(above - vec2<f32>(e, 0.))) / (2. * e);
  let sz = (caustic_slope(above + vec2<f32>(0., e)) - caustic_slope(above - vec2<f32>(0., e))) / (2. * e);
  let k = depth * (1. - 1. / IOR) * edge_weight(above);
  let det = (1. + k * sx.x) * (1. + k * sz.y) - k * k * sx.y * sz.x;
  return mix(1., clamp(1. / max(abs(det), 0.1), 0.25, 3.5), water.scatter.w);
}

// Light reaching a point under water: what the opaque pass saw, attenuated along `path`,
// plus in-scattered ambient light. Caustics modulate the directly sunlit share.
fn under_water(radiance:vec3<f32>, point:vec3<f32>, path:f32, ambient:vec3<f32>)->vec3<f32> {
  let sunlit = sun_visible(point) * clamp(camera.sun.y * 3., 0., 1.);
  let lit = radiance * mix(1., caustic(point), 0.75 * sunlit);
  let t = exp(-water.absorption.rgb * path);
  return lit * t + water.scatter.rgb * ambient * (1. - t);
}

fn ambient_light()->vec3<f32> {
  return sky(vec3<f32>(0., 1., 0.), 1.) + camera.sun_colour.rgb * camera.sun_colour.a * max(camera.sun.y, 0.) * 0.25;
}

// March against the opaque depth copy; returns radiance (unexposed) and confidence.
// Hits above `ceiling` are rejected (reflections under the surface stay under it).
fn screen_reflection(origin:vec3<f32>, direction:vec3<f32>, ceiling:f32)->vec4<f32> {
  var t = 0.06;
  var previous = 0.;
  for (var i = 0u; i < 56u; i++) {
    let q = to_pixel(origin + direction * t);
    if any(q.xy <= vec2<f32>(0.)) || any(q.xy >= camera.viewport.xy) || q.z <= 0. || q.z >= 1. { return vec4<f32>(0.); }
    let depth = textureLoad(opaque_depth, vec2<i32>(q.xy), 0);
    if depth < 1. && q.z >= depth {
      var lo = previous; var hi = t;
      for (var r = 0u; r < 6u; r++) {
        let mid = (lo + hi) * 0.5;
        let m = to_pixel(origin + direction * mid);
        let z = textureLoad(opaque_depth, clamp_pixel(m.xy), 0);
        if z < 1. && m.z >= z { hi = mid; } else { lo = mid; }
      }
      let hit = origin + direction * hi;
      let pixel = clamp_pixel(to_pixel(hit).xy);
      let surface = world_at(pixel, textureLoad(opaque_depth, pixel, 0));
      if distance(surface, hit) > 0.25 || surface.y > ceiling { return vec4<f32>(0.); }
      let uv = vec2<f32>(pixel) / camera.viewport.xy;
      let edge = min(min(uv.x, uv.y), min(1. - uv.x, 1. - uv.y));
      return vec4<f32>(opaque_radiance(pixel), smoothstep(0., 0.05, edge) * (1. - smoothstep(25., 40., hi)));
    }
    previous = t;
    t += 0.05 + 0.07 * t;
    if t > 40. { break; }
  }
  return vec4<f32>(0.);
}

@fragment fn fs_surface(f:VsOut, @builtin(front_facing) front:bool)->@location(0) vec4<f32> {
  let sl = slopes(f.world.xz);
  let w = edge_weight(f.world.xz);
  let jx = max(1. + sl.j.x, 0.2);
  let jz = max(1. + sl.j.y, 0.2);
  let s = vec2<f32>(sl.s.x / jx, sl.s.y / jz) * mix(0.25, 1., w);
  var cov = sl.cov + vec3<f32>(0.00015, 0.00015, 0.);
  // specular anti-aliasing: unresolved slope variation widens the lobe
  let dsx = dpdx(s); let dsy = dpdy(s);
  cov += vec3<f32>(dsx.x * dsx.x + dsy.x * dsy.x, dsx.y * dsx.y + dsy.y * dsy.y, dsx.x * dsx.y + dsy.x * dsy.y) * 0.5;
  let n_up = normalize(vec3<f32>(-s.x, 1., -s.y));
  let pixel = vec2<i32>(f.clip.xy);
  let to_eye = camera.eye.xyz - f.world;
  let dist = length(to_eye);
  let v = to_eye / dist;
  let roughness = clamp(sqrt(sqrt(max(cov.x + cov.y, 1e-6) * 0.5)) * 0.6, 0.02, 0.6);
  let ambient = ambient_light();
  let surface_depth = f.clip.z;
  let straight_depth = textureLoad(opaque_depth, pixel, 0);

  if !front || camera.eye.y < f.world.y {
    // Seen from below: Snell's window shows the world above, total internal reflection
    // elsewhere shows the water body itself.
    let n = -n_up;
    // Total internal reflection mirrors the basin; the water hides what the trace misses.
    let down = reflect(-v, n);
    var colour = water.scatter.rgb * ambient;
    let mirrored = screen_reflection(f.world + n * 0.02, down, water.body_max.y - 0.03);
    // Sunlit tiles seen in the mirror are far brighter than the water body; unresolved
    // ripples would turn them into isolated glints, so the trace fades with slope spread.
    let t_mirror = exp(-water.absorption.rgb * 3.);
    let mirror_weight = mirrored.a * (1. - smoothstep(0.04, 0.15, length(sqrt(max(cov.xy, vec2<f32>(0.))))));
    colour = mix(colour, mirrored.rgb * t_mirror + colour * (1. - t_mirror), mirror_weight);
    // Average the window over the slope distribution so ripples do not flip single
    // pixels in and out of it; Schlick from the air side fades it at the critical angle.
    let spread_below = sqrt(max(cov.xy, vec2<f32>(0.)));
    var tr = 0.;
    var up_sum = vec3<f32>(0.);
    for (var k = 0; k < 5; k++) {
      let o = array<vec2<f32>, 5>(vec2<f32>(0.), vec2<f32>(1., 0.), vec2<f32>(-1., 0.), vec2<f32>(0., 1.), vec2<f32>(0., -1.))[k];
      let nk = -normalize(vec3<f32>(-(s.x + o.x * spread_below.x), 1., -(s.y + o.y * spread_below.y)));
      let uk = refract(-v, nk, IOR);
      if dot(uk, uk) > 0. {
        let w = select(0.15, 0.4, k == 0);
        tr += w * (1. - fresnel(dot(uk, -nk)));
        up_sum += w * uk;
      }
    }
    // Far outside the flat-water window a ripple only glimpses the bright world above in
    // sub-pixel glints; keep the window where the unrippled view would find it.
    let cos_flat = abs(v.y);
    tr *= smoothstep(0., 0.3, 1. - IOR * IOR * (1. - cos_flat * cos_flat));
    if tr > 0. {
      let up = normalize(up_sum);
      var above = sky(up, roughness);
      let q = to_pixel(f.world + up * 4.);
      if all(q.xy > vec2<f32>(0.)) && all(q.xy < camera.viewport.xy) {
        let p = clamp_pixel(q.xy);
        if textureLoad(opaque_depth, p, 0) < 1. { above = opaque_radiance(p); }
      }
      colour = mix(colour, above, tr);
    }
    return vec4<f32>(colour * camera.viewport.z, 1.);
  }

  // ---- refraction: march the bent ray the distance the straight ray travels under water ----
  let straight_world = world_at(pixel, straight_depth);
  let under = max(distance(straight_world, f.world), 0.01);
  let bent = refract(-v, n_up, 1. / IOR);
  var hit_point = f.world + bent * under;
  var q = to_pixel(hit_point);
  var behind_pixel = pixel;
  var path = under;
  if all(q.xy > vec2<f32>(0.)) && all(q.xy < camera.viewport.xy) {
    let p = clamp_pixel(q.xy);
    let d = textureLoad(opaque_depth, p, 0);
    // only accept surfaces behind the water surface (not foreground occluders)
    if d > surface_depth {
      behind_pixel = p;
      let hit = world_at(p, d);
      path = clamp(distance(hit, f.world), 0.01, 2. * under + 0.5);
      hit_point = hit;
    }
  }
  if behind_pixel.x == pixel.x && behind_pixel.y == pixel.y { hit_point = straight_world; }
  let behind = opaque_radiance(behind_pixel);
  let refracted = under_water(behind, hit_point, path, ambient);

  // ---- reflection ----
  // Unresolved ripples spread the reflection: average the sky over the slope
  // distribution (the analytic sky has no prefiltered mips) and fade the mirror trace.
  let spread = sqrt(max(cov.xy, vec2<f32>(0.)));
  var reflected = vec3<f32>(0.);
  for (var k = 0; k < 5; k++) {
    let o = array<vec2<f32>, 5>(vec2<f32>(0.), vec2<f32>(1., 0.), vec2<f32>(-1., 0.), vec2<f32>(0., 1.), vec2<f32>(0., -1.))[k];
    let rk = reflect(-v, normalize(vec3<f32>(-(s.x + o.x * spread.x), 1., -(s.y + o.y * spread.y))));
    reflected += sky(vec3<f32>(rk.x, max(rk.y, 0.01), rk.z), roughness) * select(0.15, 0.4, k == 0);
  }
  let r = reflect(-v, n_up);
  let ssr = screen_reflection(f.world + n_up * 0.02, normalize(vec3<f32>(r.x, max(r.y, 0.02), r.z)), 1e9);
  reflected = mix(reflected, ssr.rgb, ssr.a * (1. - smoothstep(0.06, 0.2, length(spread))));
  let nv = max(dot(n_up, v), 0.);
  let fr = fresnel(nv);

  // ---- sun glitter: anisotropic Gaussian slope distribution ----
  let l = camera.sun.xyz;
  var glitter = vec3<f32>(0.);
  if l.y > 0. {
    let hv = normalize(v + l);
    let hs = -hv.xz / max(hv.y, 1e-3);
    let dsl = hs - s;
    let det = max(cov.x * cov.y - cov.z * cov.z, 1e-10);
    let e = (dsl.x * dsl.x * cov.y + dsl.y * dsl.y * cov.x - 2. * dsl.x * dsl.y * cov.z) / det;
    let pdf = exp(-0.5 * e) / (6.2831853 * sqrt(det));
    let hy2 = hv.y * hv.y;
    let d = pdf / max(hy2 * hy2, 1e-6);
    glitter = camera.sun_colour.rgb * camera.sun_colour.a * sun_visible(f.world)
      * fresnel(dot(v, hv)) * d / (4. * max(v.y, 0.05));
  }
  let colour = refracted * (1. - fr) + reflected * fr + glitter;
  return vec4<f32>(colour * camera.viewport.z, 1.);
}

// ---- underwater view: fog the opaque frame between the eye and each surface ----
struct FsOut { @builtin(position) clip:vec4<f32> };
@vertex fn vs_fullscreen(@builtin(vertex_index) i:u32)->FsOut {
  let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2. - 1.;
  var o:FsOut;
  o.clip = vec4<f32>(p, 0., 1.);
  return o;
}
@fragment fn fs_underwater(f:FsOut)->@location(0) vec4<f32> {
  let pixel = vec2<i32>(f.clip.xy);
  let depth = textureLoad(opaque_depth, pixel, 0);
  let radiance = opaque_radiance(pixel);
  let p = select(world_at(pixel, depth), camera.eye.xyz + normalize(world_at(pixel, 0.5) - camera.eye.xyz) * 60., depth >= 1.);
  // the part of the view ray below the water level
  let e = camera.eye.xyz;
  let level = water.body_max.y;
  var span = distance(p, e);
  if p.y > level {
    span *= clamp((level - e.y) / max(p.y - e.y, 1e-4), 0., 1.);
  }
  let ambient = ambient_light();
  // basin surfaces get caustics; the water surface itself (seen from below) only fog
  if p.y < level - 0.05 {
    return vec4<f32>(under_water(radiance, p, span, ambient) * camera.viewport.z, 1.);
  }
  let t = exp(-water.absorption.rgb * span);
  return vec4<f32>((radiance * t + water.scatter.rgb * ambient * (1. - t)) * camera.viewport.z, 1.);
}
