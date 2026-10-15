//▫~•◦------------------------------------------------------------------------------------◦•~▫//
// █████████████████████████████████████ VOXY ENGINE █████████████████████████████████████ //
//▫~•◦------------------------------------------------------------------------------------◦•~▫//
//
// Bodycam / Unrecord Physical Optics, CMOS Sensor & ISP Post-Processing Shader
//
// Implements the full light-transport & hardware camera degradation chain:
// 1. Kannala-Brandt (TPAMI 2006) + Brown-Conrady Plumb-Bob wide-angle fisheye barrel
//    distortion with tangential lens-element decentering (p1, p2) & EIS mount shift.
// 2. 180-degree shutter angular/linear velocity motion blur + CMOS sequential scanline
//    rolling shutter skew ("jello" effect during rapid whip-pans).
// 3. Cubic radial dispersion Lateral + Longitudinal Chromatic Aberration (Cauchy glass law).
// 4. Jorge Jimenez (SIGGRAPH 2014) 13-tap Karis-weighted HDR bloom + procedural lens dirt,
//    sebum thumbprint smears, dried water droplets, and polycarbonate micro-scratches.
// 5. Radiometric cos^4(theta) optical falloff + recessed protective housing bezel cutoff.
// 6. Small-sensor (1/2.8" CMOS) Full-Well Capacity highlight charge bleed & soft-knee clip.
// 7. Bradford Chromatic Adaptation (AWB temperature & green/magenta tint shift).
// 8. Heteroscedastic Poisson-Gaussian CMOS sensor noise (1x1 photon shot luma grain +
//    4x4 ISP-filtered read chroma blotches + horizontal row readout banding).
// 9. AgX Display Transform (Troy Sobotka / Benjamin Wrensch 6th-order polynomial) with
//    inset/outset matrices ("Path to White") and Punchy ASC-CDL look.
// 10. Hardware ISP 3x3 Unsharp Mask (USM) with noise coring and asymmetric bright/dark
//     edge overshoot halos — the signature visual cue of real action-cam / police footage.
// 11. 4:2:0 YCbCr chroma subsampling & velocity-modulated 8x8 DCT quantization stepping.

struct BodycamPostUniforms {
    // xy: resolution, zw: inv_resolution
    resolution: vec4<f32>,
    // x: time_seconds, y: frame_index, z: exposure_gain, w: iso_gain (1.0..16.0)
    sensor_timing: vec4<f32>,
    // x: k1 (-0.21), y: k2 (0.058), z: p1 (0.0014), w: p2 (-0.0011)
    lens_distortion: vec4<f32>,
    // x: ca_strength (0.0095), y: vignette_strength (0.82), z: bloom_intensity (0.14), w: dirt_intensity (0.42)
    optics_params: vec4<f32>,
    // x: unsharp_amount (1.75), y: highlight_clip (4.2), z: compression_q (96.0), w: rolling_shutter (0.24)
    isp_params: vec4<f32>,
    // xy: angular_velocity_uv (yaw, pitch screen velocity), zw: eis_mount_offset
    kinematics: vec4<f32>,
    // xyz: awb_rgb_gains (Bradford white-balance multipliers), w: lens_zoom (0.88)
    white_balance: vec4<f32>,
};

@group(0) @binding(0) var hdr_scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> u_post: BodycamPostUniforms;

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    // Fullscreen triangle covering [-1, 1]^2 without index buffer
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var out: FullscreenVertexOutput;
    out.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

// -----------------------------------------------------------------------------
// Hash & Statistical Noise Primitives (Poisson-Gaussian CMOS + Lens Micro-Dirt)
// -----------------------------------------------------------------------------
fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn hash13(p3_in: vec3<f32>) -> f32 {
    var p3 = fract(p3_in * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn gaussian_rand(seed: vec3<f32>) -> f32 {
    let u1 = max(hash13(seed), 1e-6);
    let u2 = hash13(seed + vec3<f32>(19.19, 47.73, 101.3));
    return sqrt(-2.0 * log(u1)) * cos(6.2831853 * u2);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash12(i + vec2<f32>(0.0, 0.0));
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// -----------------------------------------------------------------------------
// 1. Kannala-Brandt + Brown-Conrady Plumb-Bob Fisheye Lens Distortion
// -----------------------------------------------------------------------------
struct DistortedLensCoord {
    uv: vec2<f32>,
    p_aspect: vec2<f32>,
    radius: f32,
    bezel_mask: f32,
};

fn compute_bodycam_lens_coord(screen_uv: vec2<f32>) -> DistortedLensCoord {
    let aspect = u_post.resolution.x * u_post.resolution.w;
    let eis_shift = u_post.kinematics.zw;
    let center = vec2<f32>(0.5, 0.5) + eis_shift;

    // Aspect-corrected sensor plane coordinate p in [-aspect, aspect] x [-1, 1]
    let raw_p = (screen_uv - center) * vec2<f32>(aspect, 1.0) * 2.0;
    let r_raw = length(raw_p);

    // Recessed polycarbonate housing + rubberized corner bezel mask
    // Rounded super-ellipse + radial cutoff in the extreme corners (active in Bodycam mode when 0.1 < zoom < 0.96)
    let is_camcorder = u_post.white_balance.w < 0.0;
    let zoom = select(
        select(0.88, u_post.white_balance.w, u_post.white_balance.w > 0.1),
        0.965,
        is_camcorder
    );
    let abs_p = abs(raw_p / vec2<f32>(aspect, 1.0));
    let corner_box = pow(abs_p.x, 6.0) + pow(abs_p.y, 6.0);
    let raw_bezel = (1.0 - smoothstep(1.04, 1.24, corner_box))
                  * (1.0 - smoothstep(1.32, 1.48, r_raw / sqrt(aspect * aspect + 1.0) * 1.414));
    let bezel_mask = select(raw_bezel, 1.0, zoom >= 0.96);

    // Apply zoom factor so barrel-warped periphery stays within rendered frustum
    let p = raw_p * zoom * 0.5;
    let r2 = dot(p, p);
    let r = sqrt(r2);

    // Kannala-Brandt odd-polynomial angle mapping blended with Brown-Conrady radial/tangential
    let k1 = u_post.lens_distortion.x;
    let k2 = u_post.lens_distortion.y;
    let p1 = u_post.lens_distortion.z;
    let p2 = u_post.lens_distortion.w;

    let theta = atan(r);
    let theta2 = theta * theta;
    let theta4 = theta2 * theta2;
    let theta_d = theta * (1.0 + k1 * theta2 + k2 * theta4);
    let radial_scale = select(1.0, theta_d / r, r > 1e-5);

    // Tangential decentering distortion from multi-element lens tilt
    let tangential = vec2<f32>(
        2.0 * p1 * p.x * p.y + p2 * (r2 + 2.0 * p.x * p.x),
        p1 * (r2 + 2.0 * p.y * p.y) + 2.0 * p2 * p.x * p.y
    );

    let p_distorted = p * radial_scale + tangential;
    let final_uv = center + p_distorted * vec2<f32>(1.0 / aspect, 1.0);

    var out: DistortedLensCoord;
    out.uv = final_uv;
    out.p_aspect = raw_p * 0.5;
    out.radius = r_raw * 0.5;
    out.bezel_mask = bezel_mask;
    return out;
}

// -----------------------------------------------------------------------------
// 2. 180-Degree Shutter Motion Blur + CMOS Rolling Shutter + Chromatic Aberration
// -----------------------------------------------------------------------------
fn sample_lens_with_ca_and_shutter(
    base_uv: vec2<f32>,
    p_aspect: vec2<f32>
) -> vec3<f32> {
    let r2 = dot(p_aspect, p_aspect);
    let r = sqrt(r2);
    let ca_strength = u_post.optics_params.x;

    // Cubic Cauchy wavelength dispersion profile: alpha * (0.35 * r + 0.65 * r^3)
    let dispersion = ca_strength * (0.35 * r + 0.65 * r * r2);
    let radial_dir = select(vec2<f32>(0.0), p_aspect / r, r > 1e-5);

    // CMOS sequential scanline Rolling Shutter skew along vertical axis y
    let angular_vel = u_post.kinematics.xy;
    let rolling_strength = u_post.isp_params.w;
    let scanline_skew = angular_vel * ((base_uv.y - 0.5) * rolling_strength);

    // Per-pixel rotational + translational velocity vector (180-degree shutter = 0.5 frame integration)
    let shutter_vel = angular_vel * 0.5;
    let center_uv = clamp(base_uv - scanline_skew, vec2<f32>(0.001), vec2<f32>(0.999));

    // 5-tap 180-degree shutter velocity integration coupled with spectral R/G/B dispersion
    var accum = vec3<f32>(0.0);
    for (var tap = -2; tap <= 2; tap++) {
        let t = f32(tap) * 0.25;
        let motion_uv = center_uv + shutter_vel * t;

        // Tangential defocus offset for longitudinal chromatic aberration at periphery
        let ortho_dir = vec2<f32>(-radial_dir.y, radial_dir.x) * (dispersion * 0.18 * f32(tap));

        let uv_r = clamp(motion_uv + radial_dir * dispersion * 1.00 + ortho_dir, vec2<f32>(0.001), vec2<f32>(0.999));
        let uv_g = clamp(motion_uv, vec2<f32>(0.001), vec2<f32>(0.999));
        let uv_b = clamp(motion_uv - radial_dir * dispersion * 0.85 - ortho_dir, vec2<f32>(0.001), vec2<f32>(0.999));

        let r_sample = textureSampleLevel(hdr_scene, scene_sampler, uv_r, 0.0).r;
        let g_sample = textureSampleLevel(hdr_scene, scene_sampler, uv_g, 0.0).g;
        let b_sample = textureSampleLevel(hdr_scene, scene_sampler, uv_b, 0.0).b;
        accum += vec3<f32>(r_sample, g_sample, b_sample);
    }
    return accum * 0.2;
}

// -----------------------------------------------------------------------------
// 3. Jorge Jimenez (SIGGRAPH 2014) Karis-Weighted HDR Bloom + Dirty Lens Mask
// -----------------------------------------------------------------------------
fn karis_weight(c: vec3<f32>) -> f32 {
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    return 1.0 / (1.0 + luma);
}

fn sample_hdr_bloom(uv: vec2<f32>) -> vec3<f32> {
    let texel = u_post.resolution.zw;
    var bloom_sum = vec3<f32>(0.0);
    var weight_sum = 0.0;

    // Multi-scale 13-point dual-ring footprint with Karis anti-firefly weighting
    let radii = array<f32, 3>(2.5, 7.5, 18.0);
    let ring_weights = array<f32, 3>(0.50, 0.32, 0.18);

    for (var ring = 0; ring < 3; ring++) {
        let step_uv = texel * radii[ring];
        let rw = ring_weights[ring];

        let c0 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv, vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c1 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>( step_uv.x,  step_uv.y), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c2 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>(-step_uv.x,  step_uv.y), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c3 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>( step_uv.x, -step_uv.y), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c4 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>(-step_uv.x, -step_uv.y), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c5 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>( step_uv.x * 2.0, 0.0), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c6 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>(-step_uv.x * 2.0, 0.0), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c7 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>(0.0,  step_uv.y * 2.0), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
        let c8 = textureSampleLevel(hdr_scene, scene_sampler, clamp(uv + vec2<f32>(0.0, -step_uv.y * 2.0), vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;

        // Extract high-energy bloom above soft knee
        let avg = (c0 * 0.24 + (c1 + c2 + c3 + c4) * 0.12 + (c5 + c6 + c7 + c8) * 0.07);
        let luma = dot(avg, vec3<f32>(0.2126, 0.7152, 0.0722));
        let knee = smoothstep(0.65, 2.2, luma);
        let kw = rw * (0.35 + 0.65 * karis_weight(avg));
        bloom_sum += avg * knee * kw;
        weight_sum += kw;
    }
    return bloom_sum / max(weight_sum, 1e-4);
}

// CCD Vertical Charge Smear (3-CCD / Interline CCD overflow on intense highlights)
fn sample_ccd_vertical_smear(uv: vec2<f32>) -> vec3<f32> {
    var smear = vec3<f32>(0.0);
    let offsets = array<f32, 6>(-0.18, -0.10, -0.04, 0.04, 0.10, 0.18);
    for (var i = 0; i < 6; i++) {
        let tap_uv = clamp(vec2<f32>(uv.x, uv.y + offsets[i]), vec2<f32>(0.001), vec2<f32>(0.999));
        let c = textureSampleLevel(hdr_scene, scene_sampler, tap_uv, 0.0).rgb;
        let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        let hot = smoothstep(1.85, 4.5, luma);
        smear += c * hot;
    }
    return smear * 0.045 * vec3<f32>(0.92, 0.96, 1.12);
}

fn procedural_lens_dirt_mask(p_aspect: vec2<f32>) -> f32 {
    // 1. Dried water spots & dust specks on polycarbonate protective cover
    let dust_fine = smoothstep(0.76, 0.96, noise2d(p_aspect * 28.0 + vec2<f32>(3.1, 7.4)));
    let dust_coarse = smoothstep(0.68, 0.92, noise2d(p_aspect * 9.5 + vec2<f32>(11.3, 2.7)));
    // 2. Diagonal wiping smudge / thumbprint oil streak
    let rotated = vec2<f32>(p_aspect.x * 0.82 - p_aspect.y * 0.57, p_aspect.x * 0.57 + p_aspect.y * 0.82);
    let smear = smoothstep(0.45, 0.88, noise2d(rotated * vec2<f32>(3.2, 18.0)));
    // Dirt accumulates more heavily toward lens periphery
    let edge_weight = 0.35 + 0.65 * smoothstep(0.15, 0.85, length(p_aspect));
    return (dust_fine * 0.55 + dust_coarse * 0.45 + smear * 0.35) * edge_weight;
}

// -----------------------------------------------------------------------------
// 4. CMOS Full-Well Highlight Bleed & AgX Tone Mapping (Sobotka / Wrensch)
// -----------------------------------------------------------------------------
fn sensor_full_well_highlight_bleed(rgb_linear: vec3<f32>, clip_point: f32) -> vec3<f32> {
    let safe_clip = max(clip_point, 1.0);
    let normalized = rgb_linear / safe_clip;
    let max_c = max(normalized.r, max(normalized.g, normalized.b));
    // Charge spilling across adjacent Bayer photodiodes near full-well saturation
    let bleed = smoothstep(0.68, 1.35, max_c);
    let bled = mix(normalized, vec3<f32>(max_c), bleed * 0.62);
    return bled * safe_clip;
}

fn agx_default_contrast_approx(x: vec3<f32>) -> vec3<f32> {
    let x2 = x * x;
    let x4 = x2 * x2;
    return 15.5 * x4 * x2
         - 40.14 * x4 * x
         + 31.96 * x4
         - 6.868 * x2 * x
         + 0.4298 * x2
         + 0.1191 * x
         - 0.00232;
}

fn tonemap_agx_bodycam(color_linear: vec3<f32>) -> vec3<f32> {
    // Rec.709 AgX Inset Matrix (Column-major in WGSL)
    let agx_inset = mat3x3<f32>(
        vec3<f32>(0.842479062253094, 0.0423282422610123, 0.0423756549057051),
        vec3<f32>(0.0784335999999992, 0.878468636469772, 0.0784336),
        vec3<f32>(0.0792237451477643, 0.0791661274605434, 0.879142973793104)
    );
    let agx_outset = mat3x3<f32>(
        vec3<f32>( 1.19687900512017,  -0.0528968517574562, -0.0529716355174438),
        vec3<f32>(-0.0980208811401368, 1.15190312990417,   -0.0980434501171241),
        vec3<f32>(-0.0990297440797205, -0.0989611768448433, 1.1519069348787)
    );

    let min_ev = -12.47393;
    let max_ev = 4.026069;

    var val = agx_inset * max(color_linear, vec3<f32>(1e-6));
    val = clamp(log2(val), vec3<f32>(min_ev), vec3<f32>(max_ev));
    val = (val - vec3<f32>(min_ev)) / (max_ev - min_ev);
    val = agx_default_contrast_approx(val);

    // Bodycam ISP Punchy S-curve & slight shadow desaturation
    let power = vec3<f32>(1.24);
    let sat = 1.06;
    val = pow(clamp(val, vec3<f32>(0.0), vec3<f32>(1.0)), power);
    let luma = dot(val, vec3<f32>(0.2126, 0.7152, 0.0722));
    val = luma + sat * (val - vec3<f32>(luma));

    val = clamp(agx_outset * val, vec3<f32>(0.0), vec3<f32>(1.0));
    // Return in perceptual display space (~gamma 2.2) for ISP sharpening & YCbCr stage
    return val;
}

// -----------------------------------------------------------------------------
// 5. Heteroscedastic Poisson-Gaussian CMOS Noise + Scanline Readout Banding
// -----------------------------------------------------------------------------
fn apply_cmos_sensor_noise(
    rgb_linear: vec3<f32>,
    pixel_coord: vec2<f32>,
    frame_idx: f32,
    iso_gain: f32
) -> vec3<f32> {
    let alpha_shot = 0.00038 * iso_gain;
    let beta_read  = 0.00006 * iso_gain * iso_gain;

    let luma = dot(rgb_linear, vec3<f32>(0.2126, 0.7152, 0.0722));
    let std_dev = sqrt(alpha_shot * max(luma, 0.0) + beta_read);

    // 1x1 crisp photon shot noise in luminance
    let n_luma = gaussian_rand(vec3<f32>(pixel_coord, frame_idx)) * std_dev;

    // 4x4 low-frequency chroma blotches (remaining after hardware ISP chroma denoiser)
    let coarse_px = floor(pixel_coord * 0.25);
    let n_cr = gaussian_rand(vec3<f32>(coarse_px, frame_idx + 7.13)) * std_dev * 0.55;
    let n_cb = gaussian_rand(vec3<f32>(coarse_px, frame_idx + 19.37)) * std_dev * 0.55;

    // Horizontal CMOS ADC row readout banding in deep shadows
    let shadow_weight = 1.0 - smoothstep(0.0, 0.18, luma);
    let row_banding = (hash13(vec3<f32>(0.0, floor(pixel_coord.y), frame_idx)) - 0.5)
                    * sqrt(beta_read) * 0.55 * shadow_weight;

    var noisy = rgb_linear + vec3<f32>(n_luma + row_banding);
    noisy.r += n_cr;
    noisy.b += n_cb;
    return max(noisy, vec3<f32>(0.0));
}

// -----------------------------------------------------------------------------
// 6. BT.709 RGB <-> YCbCr Conversion & 4:2:0 Chroma Subsampling
// -----------------------------------------------------------------------------
fn rgb_to_ycbcr(rgb: vec3<f32>) -> vec3<f32> {
    let y  =  0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
    let cb = -0.1146 * rgb.r - 0.3854 * rgb.g + 0.5000 * rgb.b + 0.5;
    let cr =  0.5000 * rgb.r - 0.4542 * rgb.g - 0.0458 * rgb.b + 0.5;
    return vec3<f32>(y, cb, cr);
}

fn ycbcr_to_rgb(ycbcr: vec3<f32>) -> vec3<f32> {
    let y  = ycbcr.x;
    let cb = ycbcr.y - 0.5;
    let cr = ycbcr.z - 0.5;
    let r = y + 1.5748 * cr;
    let g = y - 0.1873 * cb - 0.4681 * cr;
    let b = y + 1.8556 * cb;
    return clamp(vec3<f32>(r, g, b), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Evaluate tone-mapped display color at an arbitrary distorted UV for the 3x3 ISP kernel
fn eval_tonemapped_tap(uv: vec2<f32>, exposure: f32, clip_point: f32) -> vec3<f32> {
    let clamped_uv = clamp(uv, vec2<f32>(0.001), vec2<f32>(0.999));
    let hdr = textureSampleLevel(hdr_scene, scene_sampler, clamped_uv, 0.0).rgb * exposure;
    let bled = sensor_full_well_highlight_bleed(hdr * u_post.white_balance.xyz, clip_point);
    return tonemap_agx_bodycam(bled);
}

// -----------------------------------------------------------------------------
// 7. Procedural Camcorder Viewfinder OSD Overlay (REC, SMPTE Timecode, Zoom Bar, Framing)
// -----------------------------------------------------------------------------
// 3x5 bitmap font glyphs packed into 15-bit integers (row-major 3 bits per row, 5 rows)
fn glyph_3x5(glyph_id: u32, gx: i32, gy: i32) -> f32 {
    if (gx < 0 || gx >= 3 || gy < 0 || gy >= 5) {
        return 0.0;
    }
    var mask = 0u;
    switch (glyph_id) {
        case 0u:  { mask = 31599u; } // '0'
        case 1u:  { mask = 9362u;  } // '1'
        case 2u:  { mask = 29671u; } // '2'
        case 3u:  { mask = 29391u; } // '3'
        case 4u:  { mask = 23497u; } // '4'
        case 5u:  { mask = 31183u; } // '5'
        case 6u:  { mask = 31215u; } // '6'
        case 7u:  { mask = 29257u; } // '7'
        case 8u:  { mask = 31727u; } // '8'
        case 9u:  { mask = 31695u; } // '9'
        case 10u: { mask = 1040u;  } // ':'
        case 11u: { mask = 27565u; } // 'R'
        case 12u: { mask = 31143u; } // 'E'
        case 13u: { mask = 31015u; } // 'C'
        case 14u: { mask = 29842u; } // 'T'
        case 15u: { mask = 23421u; } // 'W'
        case 16u: { mask = 27502u; } // 'D'
        case 17u: { mask = 23402u; } // 'V'
        case 18u: { mask = 31599u; } // 'O'
        case 19u: { mask = 23213u; } // 'X'
        case 20u: { mask = 23186u; } // 'Y'
        default:  { mask = 0u;     }
    }
    let bit_idx = u32((4 - gy) * 3 + (2 - gx));
    return f32((mask >> bit_idx) & 1u);
}

fn sample_glyph_cell(uv: vec2<f32>, origin: vec2<f32>, cell_size: vec2<f32>, glyph_id: u32) -> f32 {
    let rel = (uv - origin) / cell_size;
    if (rel.x < 0.0 || rel.x >= 1.0 || rel.y < 0.0 || rel.y >= 1.0) {
        return 0.0;
    }
    let gx = i32(floor(rel.x * 3.0));
    let gy = i32(floor(rel.y * 5.0));
    return glyph_3x5(glyph_id, gx, gy);
}

fn render_camcorder_osd(base_rgb: vec3<f32>, uv: vec2<f32>, time_s: f32, zoom_level: f32) -> vec3<f32> {
    let aspect = u_post.resolution.x * u_post.resolution.w;
    var color = base_rgb;

    // 1. Safe-area 90% corner brackets & subtle center autofocus crosshair
    let dx = abs(uv.x - 0.5);
    let dy = abs(uv.y - 0.5);
    let px = u_post.resolution.zw;

    // Corner L-brackets at (0.055..0.945, 0.065..0.935)
    let bx = abs(uv.x - 0.5) - 0.44;
    let by = abs(uv.y - 0.5) - 0.43;
    let on_v_leg = abs(bx) < px.x * 1.4 && by > -0.045 && by <= 0.001;
    let on_h_leg = abs(by) < px.y * 1.4 && bx > -0.032 && bx <= 0.001;

    // Center autofocus bracket + crosshair
    let on_center_cross = (dx < 0.011 && dy < px.y * 1.1 && dx > 0.0025)
                       || (dy < 0.018 && dx < px.x * 1.1 && dy > 0.004);
    let af_box_x = abs(dx - 0.045) < px.x * 1.1 && dy > 0.025 && dy < 0.055;
    let af_box_y = abs(dy - 0.055) < px.y * 1.1 && dx > 0.020 && dx < 0.045;

    if (on_v_leg || on_h_leg || on_center_cross || af_box_x || af_box_y) {
        color = mix(color, vec3<f32>(0.90, 0.94, 0.90), 0.68);
    }

    // 2. Top-Left: Blinking Red Record Dot + "REC" + "DV SP"
    let cell = vec2<f32>(0.0115, 0.0115 * aspect);
    let step_x = cell.x * 1.38;
    let rec_origin = vec2<f32>(0.105, 0.082);

    // Red circle at (0.088, 0.092)
    let dot_d = length((uv - vec2<f32>(0.088, 0.082 + cell.y * 0.5)) * vec2<f32>(aspect, 1.0));
    let blink_on = fract(time_s * 0.95) < 0.72;
    if (dot_d < 0.0095 && blink_on) {
        color = mix(color, vec3<f32>(1.0, 0.08, 0.05), 0.95);
    }

    let rec_mask = sample_glyph_cell(uv, rec_origin + vec2<f32>(0.0 * step_x, 0.0), cell, 11u) // R
                 + sample_glyph_cell(uv, rec_origin + vec2<f32>(1.0 * step_x, 0.0), cell, 12u) // E
                 + sample_glyph_cell(uv, rec_origin + vec2<f32>(2.0 * step_x, 0.0), cell, 13u) // C
                 + sample_glyph_cell(uv, rec_origin + vec2<f32>(4.2 * step_x, 0.0), cell, 16u) // D
                 + sample_glyph_cell(uv, rec_origin + vec2<f32>(5.2 * step_x, 0.0), cell, 17u);// V
    if (rec_mask > 0.5) {
        color = mix(color, vec3<f32>(0.96, 0.98, 0.94), 0.92);
    }

    // 3. Top-Right: Battery Indicator Icon (outline + 3 charge bars)
    let bat_min = vec2<f32>(0.855, 0.080);
    let bat_max = vec2<f32>(0.912, 0.106);
    let in_bat_box = uv.x >= bat_min.x && uv.x <= bat_max.x && uv.y >= bat_min.y && uv.y <= bat_max.y;
    let on_bat_border = in_bat_box && (
        uv.x < bat_min.x + px.x * 2.0 || uv.x > bat_max.x - px.x * 2.0 ||
        uv.y < bat_min.y + px.y * 2.0 || uv.y > bat_max.y - px.y * 2.0
    );
    let bat_nub = uv.x >= bat_max.x && uv.x <= bat_max.x + 0.005 && uv.y >= 0.087 && uv.y <= 0.099;
    let bat_bars = uv.x >= bat_min.x + 0.005 && uv.x <= bat_max.x - 0.011
                && uv.y >= bat_min.y + 0.005 && uv.y <= bat_max.y - 0.005
                && fract((uv.x - bat_min.x) * 65.0) < 0.72;
    if (on_bat_border || bat_nub || bat_bars) {
        color = mix(color, vec3<f32>(0.94, 0.97, 0.94), 0.88);
    }

    // 4. Right Side / Top-Right: Optical Zoom W [===|-----] T Bar
    let zbar_min = vec2<f32>(0.720, 0.086);
    let zbar_max = vec2<f32>(0.815, 0.098);
    let w_glyph = sample_glyph_cell(uv, vec2<f32>(0.698, 0.082), cell * 0.85, 15u); // W
    let t_glyph = sample_glyph_cell(uv, vec2<f32>(0.822, 0.082), cell * 0.85, 14u); // T
    let on_zbar_track = uv.x >= zbar_min.x && uv.x <= zbar_max.x && abs(uv.y - 0.092) < px.y * 1.4;
    let zoom_norm = clamp((zoom_level - 1.0) / 11.0, 0.0, 1.0);
    let marker_x = mix(zbar_min.x, zbar_max.x, zoom_norm);
    let on_zbar_knob = abs(uv.x - marker_x) < 0.004 && uv.y >= 0.082 && uv.y <= 0.102;
    if (w_glyph > 0.5 || t_glyph > 0.5 || on_zbar_track || on_zbar_knob) {
        color = mix(color, vec3<f32>(0.94, 0.97, 0.94), 0.88);
    }

    // 5. Bottom-Right: Live Running SMPTE Timecode "00:14:SS:FF"
    let total_secs = u32(max(time_s, 0.0)) + 862u; // Starts at 00:14:22:00
    let hours = (total_secs / 3600u) % 24u;
    let mins  = (total_secs / 60u) % 60u;
    let secs  = total_secs % 60u;
    let frames = u32(fract(max(time_s, 0.0)) * 30.0) % 30u;

    let tc_origin = vec2<f32>(0.735, 0.885);
    let digits = array<u32, 11>(
        hours / 10u, hours % 10u, 10u,
        mins / 10u,  mins % 10u,  10u,
        secs / 10u,  secs % 10u,  10u,
        frames / 10u, frames % 10u
    );
    var tc_mask = 0.0;
    for (var i = 0; i < 11; i++) {
        tc_mask += sample_glyph_cell(uv, tc_origin + vec2<f32>(f32(i) * step_x, 0.0), cell, digits[i]);
    }
    if (tc_mask > 0.5) {
        // Subtle dark drop-shadow behind OSD text for legibility on bright concrete
        color = mix(color, vec3<f32>(0.96, 0.98, 0.94), 0.92);
    }

    // 6. Bottom-Left: "VOXY" Camcorder Tape Deck Stamp
    let vx_origin = vec2<f32>(0.082, 0.885);
    let vx_mask = sample_glyph_cell(uv, vx_origin + vec2<f32>(0.0 * step_x, 0.0), cell, 17u) // V
                + sample_glyph_cell(uv, vx_origin + vec2<f32>(1.0 * step_x, 0.0), cell, 18u) // O
                + sample_glyph_cell(uv, vx_origin + vec2<f32>(2.0 * step_x, 0.0), cell, 19u) // X
                + sample_glyph_cell(uv, vx_origin + vec2<f32>(3.0 * step_x, 0.0), cell, 20u);// Y
    if (vx_mask > 0.5) {
        color = mix(color, vec3<f32>(0.94, 0.96, 0.88), 0.85);
    }

    return color;
}

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel_coord = in.position.xy;
    let is_camcorder = u_post.white_balance.w < 0.0;
    let camcorder_zoom = abs(u_post.white_balance.w);
    let lens = compute_bodycam_lens_coord(in.uv);

    // Out-of-bounds or occluded by mechanical protective housing bezel
    if (lens.bezel_mask <= 0.001 || any(lens.uv < vec2<f32>(0.0)) || any(lens.uv > vec2<f32>(1.0))) {
        return vec4<f32>(0.005, 0.005, 0.006, 1.0);
    }

    let exposure = u_post.sensor_timing.z;
    let iso_gain = u_post.sensor_timing.w;
    let clip_point = u_post.isp_params.y;

    // 1. Sample HDR scene with 180-deg shutter blur, rolling shutter & lateral/longitudinal CA
    var hdr_color = sample_lens_with_ca_and_shutter(lens.uv, lens.p_aspect);

    // 2. Add Jimenez Karis-weighted HDR bloom + dirty lens diffraction (+ CCD vertical smear for camcorder)
    let bloom = sample_hdr_bloom(lens.uv);
    let dirt_mask = procedural_lens_dirt_mask(lens.p_aspect);
    let bloom_strength = u_post.optics_params.z;
    let dirt_strength = u_post.optics_params.w;
    hdr_color += bloom * (bloom_strength + dirt_strength * dirt_mask);
    if (is_camcorder) {
        hdr_color += sample_ccd_vertical_smear(lens.uv);
    }

    // 3. Radiometric cos^4(theta) natural optical vignetting
    let cos_theta2 = 1.0 / (1.0 + dot(lens.p_aspect, lens.p_aspect) * 0.95);
    let cos4_theta = cos_theta2 * cos_theta2;
    let optical_vignette = mix(1.0, cos4_theta, u_post.optics_params.y);

    // 4. Apply Auto-Exposure, Bradford AWB & Optical Vignette in linear sensor space
    var sensor_linear = hdr_color * exposure * optical_vignette * u_post.white_balance.xyz;

    // 5. Apply Heteroscedastic Poisson-Gaussian CMOS Noise & Row Readout Banding
    sensor_linear = apply_cmos_sensor_noise(sensor_linear, pixel_coord, u_post.sensor_timing.y, iso_gain);

    // 6. Small CMOS Full-Well Capacity Highlight Bleed & AgX Tone Mapping
    let center_bled = sensor_full_well_highlight_bleed(sensor_linear, clip_point);
    let center_display = tonemap_agx_bodycam(center_bled);

    // 7. Hardware ISP 3x3 Unsharp Mask (USM) with Asymmetric Overshoot Halos in Luma (Y)
    let texel = u_post.resolution.zw;
    let n_display = eval_tonemapped_tap(lens.uv + vec2<f32>(0.0, -texel.y * 1.35), exposure * optical_vignette, clip_point);
    let s_display = eval_tonemapped_tap(lens.uv + vec2<f32>(0.0,  texel.y * 1.35), exposure * optical_vignette, clip_point);
    let e_display = eval_tonemapped_tap(lens.uv + vec2<f32>( texel.x * 1.35, 0.0), exposure * optical_vignette, clip_point);
    let w_display = eval_tonemapped_tap(lens.uv + vec2<f32>(-texel.x * 1.35, 0.0), exposure * optical_vignette, clip_point);

    let ycbcr_c = rgb_to_ycbcr(center_display);
    let luma_neighbors = 0.25 * (
        dot(n_display, vec3<f32>(0.2126, 0.7152, 0.0722)) +
        dot(s_display, vec3<f32>(0.2126, 0.7152, 0.0722)) +
        dot(e_display, vec3<f32>(0.2126, 0.7152, 0.0722)) +
        dot(w_display, vec3<f32>(0.2126, 0.7152, 0.0722))
    );
    let luma_detail = ycbcr_c.x - luma_neighbors;
    // Noise coring so fine sensor noise isn't oversharpened, while structural edges get strong halos
    let coring = smoothstep(0.008, 0.022, abs(luma_detail));
    // Asymmetric overshoot halo clamping (bright halo around dark silhouettes against sky)
    let halo = clamp(luma_detail * coring, -0.11, 0.17) * u_post.isp_params.x;
    var y_sharpened = clamp(ycbcr_c.x + halo, 0.0, 1.0);

    // 8. 4:2:0 YCbCr Chroma Subsampling & Velocity-Modulated 8x8 DCT Quantization
    let chroma_Tap = 0.5 * (rgb_to_ycbcr(e_display).yz + ycbcr_c.yz);
    let motion_mag = length(u_post.kinematics.xy) * 120.0;
    let q_luma = max(u_post.isp_params.z / (1.0 + motion_mag * 0.35), 24.0);
    let q_chroma = max(q_luma * 0.55, 16.0);

    y_sharpened = floor(y_sharpened * q_luma + 0.5) / q_luma;
    let cb_quant = floor(chroma_Tap.x * q_chroma + 0.5) / q_chroma;
    let cr_quant = floor(chroma_Tap.y * q_chroma + 0.5) / q_chroma;

    var display_gamma = ycbcr_to_rgb(vec3<f32>(y_sharpened, cb_quant, cr_quant)) * lens.bezel_mask;

    // 9. Camcorder Interlaced Scanlines & Viewfinder OSD Overlay (when Camcorder profile is active)
    if (is_camcorder) {
        let scanline = 1.0 - 0.055 * (0.5 + 0.5 * sin(pixel_coord.y * 3.14159265));
        display_gamma *= scanline;
        display_gamma = render_camcorder_osd(display_gamma, in.uv, u_post.sensor_timing.x, camcorder_zoom);
    }

    // Convert from display gamma (~2.2) back to linear light because wgpu's Rgba8UnormSrgb
    // / Bgra8UnormSrgb swapchain hardware-encodes linear -> sRGB on write!
    let linear_out = pow(max(display_gamma, vec3<f32>(0.0)), vec3<f32>(2.2));
    return vec4<f32>(linear_out, 1.0);
}
