//▫~•◦------------------------------------------------------------------------------------◦•~▫//
// █████████████████████████████████████ VOXY ENGINE █████████████████████████████████████ //
//▫~•◦------------------------------------------------------------------------------------◦•~▫//
//
// Photorealistic Linear HDR PBR Voxel & Skinned Shader (`voxel_photoreal.wgsl`)
//
// Scientific & Technical Foundations:
// 1. CIE Standard Overcast Sky (Moon & Spencer 1942 / CIE S 011/E:2003):
//    $L(\theta) = L_z \cdot (1 + 2 \cos\theta) / 3$, paired with warm ground bounce.
// 2. Jimenez et al. (SIGGRAPH 2016) Practical Real-Time Strategies for Accurate Indirect Occlusion:
//    Multi-Bounce Albedo Ambient Occlusion (`gtao_multibounce`), restoring warm/saturated color
//    bleeding inside voxel crevices instead of sterile grey/black AO darkening.
// 3. Lagarde & de Rousiers (SIGGRAPH 2014) Moving Frostbite to PBR:
//    Analytical Specular Occlusion (`compute_specular_occlusion`) preventing glowing specular
//    leaks inside shadowed corners and wet puddles.
// 4. Tokuyoshi & Kaplanyan (JCGT 2021) Stable Geometric Specular Antialiasing:
//    Screen-space normal derivative variance ($\sigma^2 = \|d\vec{n}/dx\|^2 + \|d\vec{n}/dy\|^2$)
//    dilation of GGX roughness $\alpha^2$, eliminating subpixel shimmer on sharp voxel edges.
// 5. Dual-Lobe Clearcoat / Wet Porous Surface BRDF (Lagarde 2012 / Karis 2013):
//    Porous dielectric darkening ($\text{albedo}^{1.35}$), smooth water film clearcoat lobe
//    ($F_0 = 0.02, \alpha_c = 0.035$), and procedural Voronoi aggregate/pore micro-normal relief.
// 6. Tactical Dual-Zone IES Flashlight + Henyey-Greenstein ($g = 0.72$) Volumetric Dust Beam:
//    Central TIR lens hotspot + wide spill + ray-marched forward-scattering atmospheric dust cone.

struct PhotorealCamera {
    view_proj: mat4x4<f32>,
    camera_eye: vec4<f32>,        // xyz: eye position, w: elapsed_seconds
    flashlight_pos: vec4<f32>,    // xyz: flashlight world pos, w: enabled (1.0 or 0.0)
    flashlight_dir: vec4<f32>,    // xyz: flashlight unit dir, w: wetness (0.0 .. 1.0)
};

struct ChunkMeta {
    relative_origin: vec4<i32>,
    light_info: vec4<u32>,
};

struct Object {
    model: mat4x4<f32>,
    material: vec4<u32>,
};

@group(0) @binding(0) var<uniform> camera: PhotorealCamera;
@group(0) @binding(1) var<storage, read> chunk_meta: array<ChunkMeta>;
@group(0) @binding(2) var<storage, read> light_words: array<u32>;
@group(1) @binding(0) var materials: texture_2d_array<f32>;
@group(1) @binding(1) var material_sampler: sampler;
@group(2) @binding(0) var<uniform> object: Object;
@group(2) @binding(1) var<storage, read> joints: array<mat4x4<f32>>;

const PI: f32 = 3.14159265359;

// ============================================================================
// Vertex Inputs & Outputs
// ============================================================================

struct VoxelQuadInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) origin: vec4<u32>,
    @location(1) extent_face: vec4<u32>,
    @location(2) material_layer: vec2<u32>,
    @location(3) ao_diagonal: vec4<u32>,
    @location(4) chunk_slot: vec2<u32>,
};

struct IndirectVoxelInput {
    @location(0) position: vec3<f32>,
    @location(1) face_dir: u32,
    @location(2) material: u32,
    @location(3) ao: u32,
    @location(4) uv: vec2<f32>,
};

struct SkinnedInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) joint: vec4<u32>,
    @location(4) weight: vec4<f32>,
};

struct PhotorealVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) normal: vec3<f32>,
    @location(3) ao: f32,
    @location(4) light: vec2<f32>,
    @location(5) world_pos: vec3<f32>,
    @location(6) @interpolate(flat) chunk_slot: u32,
};

fn sample_light_cell(slot: u32, local_air: vec3<i32>) -> vec2<f32> {
    let voxel = vec3<u32>(clamp(local_air, vec3<i32>(0), vec3<i32>(31)));
    let index = voxel.x + 32u * (voxel.z + 32u * voxel.y);
    let word = light_words[chunk_meta[slot].light_info.x + index / 4u];
    let packed = (word >> ((index & 3u) * 8u)) & 0xffu;
    return vec2<f32>(f32(packed >> 4u), f32(packed & 0x0fu)) / 15.0;
}

// Continuous 3D trilinear interpolation of the baked voxel light field (sky + block light).
// Eliminates Gouraud vertex banding across large greedy-meshed concrete walls & floors!
fn sample_light_trilinear(slot: u32, local_pos: vec3<f32>) -> vec2<f32> {
    let p = clamp(local_pos - vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(31.0));
    let base = vec3<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let l000 = sample_light_cell(slot, base + vec3<i32>(0, 0, 0));
    let l100 = sample_light_cell(slot, base + vec3<i32>(1, 0, 0));
    let l010 = sample_light_cell(slot, base + vec3<i32>(0, 1, 0));
    let l110 = sample_light_cell(slot, base + vec3<i32>(1, 1, 0));
    let l001 = sample_light_cell(slot, base + vec3<i32>(0, 0, 1));
    let l101 = sample_light_cell(slot, base + vec3<i32>(1, 0, 1));
    let l011 = sample_light_cell(slot, base + vec3<i32>(0, 1, 1));
    let l111 = sample_light_cell(slot, base + vec3<i32>(1, 1, 1));

    let lx00 = mix(l000, l100, u.x);
    let lx10 = mix(l010, l110, u.x);
    let lx01 = mix(l001, l101, u.x);
    let lx11 = mix(l011, l111, u.x);
    let lxy0 = mix(lx00, lx10, u.y);
    let lxy1 = mix(lx01, lx11, u.y);
    return mix(lxy0, lxy1, u.z);
}

// Compute the 3D gradient of the sky-light field (Bent Normal direction toward open windows/skylights)
fn compute_light_field_bent_normal(slot: u32, air_pos: vec3<f32>, geom_n: vec3<f32>) -> vec3<f32> {
    let sx_p = sample_light_trilinear(slot, air_pos + vec3<f32>(1.0, 0.0, 0.0)).x;
    let sx_n = sample_light_trilinear(slot, air_pos - vec3<f32>(1.0, 0.0, 0.0)).x;
    let sy_p = sample_light_trilinear(slot, air_pos + vec3<f32>(0.0, 1.0, 0.0)).x;
    let sy_n = sample_light_trilinear(slot, air_pos - vec3<f32>(0.0, 1.0, 0.0)).x;
    let sz_p = sample_light_trilinear(slot, air_pos + vec3<f32>(0.0, 0.0, 1.0)).x;
    let sz_n = sample_light_trilinear(slot, air_pos - vec3<f32>(0.0, 0.0, 1.0)).x;
    let grad = vec3<f32>(sx_p - sx_n, sy_p - sy_n, sz_p - sz_n);
    return normalize(geom_n + grad * 1.35);
}

// Ray-march the 3D voxel light volume (solid blocks have light == 0) for soft skylight/sun shadows
fn trace_voxel_light_shadow(slot: u32, start_air_pos: vec3<f32>, ray_dir: vec3<f32>, max_steps: i32) -> f32 {
    var shadow = 1.0;
    for (var step = 1; step <= max_steps; step++) {
        let t = f32(step) * 0.95;
        let p = start_air_pos + ray_dir * t;
        if (any(p < vec3<f32>(0.5)) || any(p > vec3<f32>(31.0))) {
            break;
        }
        let s = sample_light_trilinear(slot, p);
        let total_energy = s.x + s.y;
        // Solid occluder cells inside the chunk have zero light energy
        let cell_transmittance = smoothstep(0.005, 0.085, total_energy);
        shadow = min(shadow, mix(0.18, 1.0, cell_transmittance));
    }
    return shadow;
}

fn face_normal(face: u32) -> vec3<f32> {
    switch face {
        case 0u: { return vec3<f32>(-1.0, 0.0, 0.0); }
        case 1u: { return vec3<f32>( 1.0, 0.0, 0.0); }
        case 2u: { return vec3<f32>(0.0, -1.0, 0.0); }
        case 3u: { return vec3<f32>(0.0,  1.0, 0.0); }
        case 4u: { return vec3<f32>(0.0, 0.0, -1.0); }
        default: { return vec3<f32>(0.0, 0.0, 1.0); }
    }
}

fn face_normal_i32(face: u32) -> vec3<i32> {
    switch face {
        case 0u: { return vec3<i32>(-1, 0, 0); }
        case 1u: { return vec3<i32>( 1, 0, 0); }
        case 2u: { return vec3<i32>( 0, -1, 0); }
        case 3u: { return vec3<i32>( 0,  1, 0); }
        case 4u: { return vec3<i32>( 0, 0, -1); }
        default: { return vec3<i32>( 0, 0,  1); }
    }
}

@vertex
fn vs_voxel(input: VoxelQuadInput) -> PhotorealVertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let flipped_corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let corner = select(
        corners[input.vertex_index],
        flipped_corners[input.vertex_index],
        (input.ao_diagonal.w & 0x80u) != 0u,
    );
    let face = input.extent_face.z;
    var axis_u = vec3<f32>(1.0, 0.0, 0.0);
    var axis_v = vec3<f32>(0.0, 1.0, 0.0);
    if (face == 0u || face == 1u) {
        axis_u = vec3<f32>(0.0, 0.0, 1.0);
        axis_v = vec3<f32>(0.0, 1.0, 0.0);
    } else if (face == 2u || face == 3u) {
        axis_u = vec3<f32>(1.0, 0.0, 0.0);
        axis_v = vec3<f32>(0.0, 0.0, 1.0);
    }
    let extent = vec2<f32>(f32(input.extent_face.x), f32(input.extent_face.y));
    let quad_offset = axis_u * corner.x * extent.x + axis_v * corner.y * extent.y;
    let local = vec3<f32>(chunk_meta[input.chunk_slot.x].relative_origin.xyz)
        + vec3<f32>(input.origin.xyz)
        + quad_offset;
    let ao_values = vec4<f32>(input.ao_diagonal & vec4<u32>(0x7fu)) / 3.0;
    let ao = select(
        select(ao_values.x, ao_values.y, corner.x > 0.5),
        select(ao_values.w, ao_values.z, corner.x > 0.5),
        corner.y > 0.5,
    );
    let corner_cell = vec3<i32>(floor(
        vec3<f32>(input.origin.xyz)
        + axis_u * clamp(corner.x * extent.x - 0.01, 0.0, max(extent.x - 1.0, 0.0))
        + axis_v * clamp(corner.y * extent.y - 0.01, 0.0, max(extent.y - 1.0, 0.0))
    )) + face_normal_i32(face);

    var output: PhotorealVertexOutput;
    output.position = camera.view_proj * vec4<f32>(local, 1.0);
    output.uv = corner * extent;
    output.material = input.material_layer.x;
    output.normal = face_normal(face);
    output.ao = ao;
    output.light = sample_light_cell(input.chunk_slot.x, corner_cell);
    output.world_pos = local;
    output.chunk_slot = input.chunk_slot.x;
    return output;
}

@vertex
fn vs_indirect(input: IndirectVoxelInput) -> PhotorealVertexOutput {
    var output: PhotorealVertexOutput;
    output.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    output.material = input.material;
    output.normal = face_normal(input.face_dir);
    output.ao = clamp(f32(input.ao) / 3.0, 0.0, 1.0);
    output.light = vec2<f32>(0.85, 0.0);
    output.world_pos = input.position;
    output.chunk_slot = 0xffffffffu;
    return output;
}

fn transported_normal(world: mat4x4<f32>, original: vec3<f32>) -> vec3<f32> {
    let normal_scale = max(max(abs(original.x), abs(original.y)), abs(original.z));
    if (normal_scale == 0.0) { return vec3<f32>(0.0); }
    var a = world[0].xyz; var b = world[1].xyz; var c = world[2].xyz;
    let scale = max(max(max(abs(a.x), abs(a.y)), abs(a.z)),
        max(max(max(abs(b.x), abs(b.y)), abs(b.z)), max(max(abs(c.x), abs(c.y)), abs(c.z))));
    if (scale == 0.0) { return vec3<f32>(0.0); }
    a /= scale; b /= scale; c /= scale;
    let determinant = dot(a, cross(b, c));
    let cofactors = mat3x3<f32>(cross(b, c), cross(c, a), cross(a, b));
    let transformed = (cofactors * (original / normal_scale)) * sign(determinant);
    let magnitude = max(max(abs(transformed.x), abs(transformed.y)), abs(transformed.z));
    if (magnitude == 0.0) { return vec3<f32>(0.0); }
    return normalize(transformed / magnitude);
}

@vertex
fn vs_skinned(input: SkinnedInput) -> PhotorealVertexOutput {
    let skin = joints[input.joint.x] * input.weight.x
        + joints[input.joint.y] * input.weight.y
        + joints[input.joint.z] * input.weight.z
        + joints[input.joint.w] * input.weight.w;
    let world = object.model * skin;
    let world_pos = (world * vec4<f32>(input.position, 1.0)).xyz;
    var output: PhotorealVertexOutput;
    output.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    output.uv = input.uv;
    output.material = object.material.x;
    output.normal = transported_normal(world, input.normal);
    output.ao = 0.88;
    output.light = vec2<f32>(0.92, 0.0);
    output.world_pos = world_pos;
    output.chunk_slot = 0xffffffffu;
    return output;
}

// ============================================================================
// Procedural Micro-Surface Detail & Hashes
// ============================================================================

fn hash31(p3_in: vec3<f32>) -> f32 {
    var p3 = fract(p3_in * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash31(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash31(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash31(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash31(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash31(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash31(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash31(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash31(i + vec3<f32>(1.0, 1.0, 1.0));
    let nx00 = mix(n000, n100, u.x);
    let nx10 = mix(n010, n110, u.x);
    let nx01 = mix(n001, n101, u.x);
    let nx11 = mix(n011, n111, u.x);
    let nxy0 = mix(nx00, nx10, u.y);
    let nxy1 = mix(nx01, nx11, u.y);
    return mix(nxy0, nxy1, u.z);
}

fn fbm3(p_in: vec3<f32>) -> f32 {
    var p = p_in;
    var v = 0.0;
    var amp = 0.5;
    for (var i: i32 = 0; i < 4; i = i + 1) {
        v += amp * value_noise_3d(p);
        p = p * 2.07 + vec3<f32>(13.1, 7.7, 19.3);
        amp *= 0.5;
    }
    return v;
}

// ============================================================================
// Physical BRDF, Multi-Bounce AO & Specular Anti-Aliasing
// ============================================================================

// Jimenez et al. (SIGGRAPH 2016) Ground Truth Ambient Occlusion Multi-Bounce
fn gtao_multibounce(visibility: f32, albedo: vec3<f32>) -> vec3<f32> {
    let a =  2.0404 * albedo - 0.3324;
    let b = -4.7951 * albedo + 0.6417;
    let c =  2.7552 * albedo + 0.6903;
    let x = vec3<f32>(visibility);
    return max(x, ((x * a + b) * x + c) * x);
}

// Lagarde & de Rousiers (SIGGRAPH 2014) Specular Occlusion
fn compute_specular_occlusion(n_dot_v: f32, ao: f32, roughness: f32) -> f32 {
    return clamp(pow(n_dot_v + ao, exp2(-16.0 * roughness - 1.0)) - 1.0 + ao, 0.0, 1.0);
}

// Tokuyoshi & Kaplanyan (JCGT 2021) Screen-Space Derivative Specular AA
fn geometric_specular_aa(normal: vec3<f32>, base_roughness: f32) -> f32 {
    let dndx = dpdx(normal);
    let dndy = dpdy(normal);
    let variance = 0.25 * (dot(dndx, dndx) + dot(dndy, dndy));
    let kernel_roughness2 = min(2.0 * variance, 0.18);
    let alpha = base_roughness * base_roughness;
    let filtered_alpha2 = clamp(alpha * alpha + kernel_roughness2, 0.0016, 1.0);
    return sqrt(sqrt(filtered_alpha2));
}

fn d_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / max(PI * denom * denom, 1e-6);
}

fn v_smith_ggx_correlated(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let a2 = roughness * roughness * roughness * roughness;
    let gv = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let gl = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    return 0.5 / max(gv + gl, 1e-5);
}

fn f_schlick(v_dot_h: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - v_dot_h, 0.0, 1.0), 5.0);
}

// ============================================================================
// Dual-Zone IES Tactical Flashlight + Henyey-Greenstein Volumetric Cone
// ============================================================================

fn evaluate_tactical_flashlight(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    albedo: vec3<f32>,
    roughness: f32,
    f0: vec3<f32>,
    clearcoat_weight: f32,
) -> vec3<f32> {
    if (camera.flashlight_pos.w < 0.5) {
        return vec3<f32>(0.0);
    }
    let to_light = camera.flashlight_pos.xyz - world_pos;
    let dist_sq = dot(to_light, to_light);
    let dist = sqrt(dist_sq);
    let l = to_light / max(dist, 1e-4);

    let spot_cos = dot(-l, normalize(camera.flashlight_dir.xyz));
    // Central TIR reflector throw hotspot (~11 deg)
    let hotspot = pow( smoothstep(0.955, 0.996, spot_cos), 2.2 ) * 3.8;
    // Wide peripheral tactical spill (~54 deg)
    let spill = pow( smoothstep(0.58, 0.94, spot_cos), 1.35 ) * 0.48;
    // Subtle outer lens bezel ring artifact
    let ring = smoothstep(0.62, 0.66, spot_cos) * (1.0 - smoothstep(0.66, 0.71, spot_cos)) * 0.14;

    let range = 38.0;
    let window = pow(max(1.0 - pow(dist / range, 4.0), 0.0), 2.0);
    let attenuation = window / (dist_sq + 0.35);

    // Cool-white 5700K Cree XP-L LED radiant intensity
    let led_radiance = vec3<f32>(0.94, 0.97, 1.05) * 26.0 * (hotspot + spill + ring) * attenuation;

    let n_dot_l = max(dot(normal, l), 0.0);
    if (n_dot_l <= 0.0) {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view_dir), 1e-4);
    let h = normalize(view_dir + l);
    let n_dot_h = max(dot(normal, h), 0.0);
    let v_dot_h = max(dot(view_dir, h), 0.0);

    let d = d_ggx(n_dot_h, roughness);
    let v = v_smith_ggx_correlated(n_dot_v, n_dot_l, roughness);
    let f = f_schlick(v_dot_h, f0);
    let spec = d * v * f;

    // Clearcoat water film specular lobe
    let dc = d_ggx(n_dot_h, 0.045);
    let vc = v_smith_ggx_correlated(n_dot_v, n_dot_l, 0.045);
    let fc = f_schlick(v_dot_h, vec3<f32>(0.02)) * clearcoat_weight;
    let clearcoat_spec = dc * vc * fc;

    let kd = (vec3<f32>(1.0) - f) * (1.0 - clearcoat_weight * 0.25);
    let diffuse = kd * albedo / PI;

    return (diffuse + spec + clearcoat_spec) * led_radiance * n_dot_l;
}

fn hg_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = pow(max(1.0 + g2 - 2.0 * g * cos_theta, 1e-4), 1.5);
    return (1.0 - g2) / (4.0 * PI * denom);
}

// Ray-marched volumetric dust cone in the flashlight beam
fn flashlight_volumetric_inscatter(ray_origin: vec3<f32>, surface_pos: vec3<f32>) -> vec3<f32> {
    if (camera.flashlight_pos.w < 0.5) {
        return vec3<f32>(0.0);
    }
    let to_surface = surface_pos - ray_origin;
    let max_dist = min(length(to_surface), 16.0);
    if (max_dist <= 0.65) {
        return vec3<f32>(0.0);
    }
    let ray_dir = to_surface / max(length(to_surface), 1e-4);
    let light_dir = normalize(camera.flashlight_dir.xyz);

    var accum = vec3<f32>(0.0);
    let steps = 8;
    let start_dist = 0.65;
    let step_len = (max_dist - start_dist) / f32(steps);
    for (var i: i32 = 0; i < steps; i = i + 1) {
        let t = start_dist + (f32(i) + 0.5) * step_len;
        let sample_p = ray_origin + ray_dir * t;
        let l_vec = camera.flashlight_pos.xyz - sample_p;
        let d2 = dot(l_vec, l_vec);
        let d = sqrt(d2);
        let l = l_vec / max(d, 1e-4);
        let spot_cos = dot(-l, light_dir);
        let cone = pow(smoothstep(0.84, 0.992, spot_cos), 2.4);
        if (cone > 0.001) {
            let phase = hg_phase(dot(-ray_dir, l), 0.72);
            let dust = 0.45 + 0.85 * value_noise_3d(sample_p * 2.8 + vec3<f32>(0.0, camera.camera_eye.w * 0.15, 0.0));
            let atten = cone / (d2 + 1.85);
            accum += vec3<f32>(0.90, 0.95, 1.05) * atten * phase * dust * step_len;
        }
    }
    return accum * 0.11;
}

// ============================================================================
// Fragment Entry Point (Photorealistic Linear HDR Output)
// ============================================================================

@fragment
fn fs_main(input: PhotorealVertexOutput) -> @location(0) vec4<f32> {
    let tex_sample = textureSample(materials, material_sampler, input.uv, i32(input.material));

    // Resolve geometric normal (supports flat-shaded skinned triangles)
    var geom_n = input.normal;
    var n_mag = max(max(abs(geom_n.x), abs(geom_n.y)), abs(geom_n.z));
    if (n_mag == 0.0) {
        geom_n = cross(dpdy(input.world_pos), dpdx(input.world_pos));
        n_mag = max(max(abs(geom_n.x), abs(geom_n.y)), abs(geom_n.z));
    }
    if (n_mag > 0.0) {
        geom_n = normalize(geom_n / n_mag);
    } else {
        geom_n = vec3<f32>(0.0, 1.0, 0.0);
    }

    // World-space multi-scale FBM detail (concrete aggregate pores, weathered patches, water stains)
    let p_fine = input.world_pos * 6.5;
    let detail_fine = fbm3(p_fine);
    let detail_micro = value_noise_3d(input.world_pos * 26.0);
    let detail_macro = fbm3(input.world_pos * 0.42);
    let detail_stain = fbm3(input.world_pos * vec3<f32>(0.85, 2.2, 0.85));

    // Analytical 3D gradient of fine + micro FBM noise (artifact-free across greedy-meshed quad boundaries!)
    let eps = 0.035;
    let df_dx = (fbm3(p_fine + vec3<f32>(eps, 0.0, 0.0)) - detail_fine)
              + 0.32 * (value_noise_3d((input.world_pos + vec3<f32>(eps * 0.25, 0.0, 0.0)) * 26.0) - detail_micro);
    let df_dy = (fbm3(p_fine + vec3<f32>(eps, 0.0, 0.0).yxz) - detail_fine)
              + 0.32 * (value_noise_3d((input.world_pos + vec3<f32>(0.0, eps * 0.25, 0.0)) * 26.0) - detail_micro);
    let df_dz = (fbm3(p_fine + vec3<f32>(0.0, 0.0, eps)) - detail_fine)
              + 0.32 * (value_noise_3d((input.world_pos + vec3<f32>(0.0, 0.0, eps * 0.25)) * 26.0) - detail_micro);
    var fbm_grad = vec3<f32>(df_dx, df_dy, df_dz) / eps;

    // Subtle 1m architectural formwork/masonry seam + sparse tie-rod divot holes on concrete panels
    let tile_uv = fract(input.uv);
    let seam_dist = min(min(tile_uv.x, 1.0 - tile_uv.x), min(tile_uv.y, 1.0 - tile_uv.y));
    var mortar_groove = mix(0.84, 1.0, smoothstep(0.006, 0.042, seam_dist + detail_fine * 0.018));

    // Formwork tie-rod pinholes on every 2nd vertical concrete slab (subtle architectural detail)
    let slab_id = floor(input.world_pos + vec3<f32>(0.01));
    let has_tie_rod = (i32(slab_id.x + slab_id.z) & 1) == 0 && (i32(slab_id.y) & 1) == 0;
    if (input.material == 1u && abs(geom_n.y) < 0.25 && has_tie_rod) {
        let hole_d = length(tile_uv - vec2<f32>(0.50, 0.50));
        let tie_rod_ao = smoothstep(0.012, 0.032, hole_d);
        mortar_groove *= mix(0.72, 1.0, tie_rod_ao);
    }

    // Material-specific physical palette & roughness calibration
    let tex_luma = dot(tex_sample.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let tex_mod = clamp(0.78 + 0.35 * (tex_luma - 0.5), 0.55, 1.25);

    var base_albedo = vec3<f32>(0.32, 0.31, 0.30);
    var base_roughness = 0.78;
    var metallic = 0.0;
    var wetness = clamp(camera.flashlight_dir.w, 0.0, 1.0);

    if (input.material == 1u) {
        // Weathered brutalist concrete with warm/cool aggregate, spalling pores & vertical rainwater streaks
        let concrete_cold = vec3<f32>(0.24, 0.26, 0.28);
        let concrete_warm = vec3<f32>(0.43, 0.39, 0.35);
        let concrete_rust = vec3<f32>(0.35, 0.25, 0.18);
        var tint = mix(concrete_cold, concrete_warm, detail_macro);
        // Vertical damp rainwater run-off streaks along walls
        let vertical_streak = fbm3(input.world_pos * vec3<f32>(2.4, 0.45, 2.4));
        let efflorescence = smoothstep(0.66, 0.88, vertical_streak) * (1.0 - abs(geom_n.y));
        tint = mix(tint, concrete_rust, smoothstep(0.60, 0.86, detail_stain) * 0.48);
        tint = mix(tint, tint * 0.68, efflorescence * 0.45);
        let micro_speckle = 0.88 + 0.24 * detail_micro;
        base_albedo = tint * tex_mod * (0.76 + 0.42 * detail_fine) * micro_speckle * mortar_groove;
        base_roughness = clamp(
            0.84 - 0.24 * detail_fine - 0.18 * efflorescence + (1.0 - mortar_groove) * 0.12,
            0.28,
            0.95
        );
    } else if (input.material == 2u) {
        // Natural damp moss / lichen / olive tactical foliage
        let moss_dark = vec3<f32>(0.09, 0.15, 0.07);
        let moss_olive = vec3<f32>(0.25, 0.29, 0.14);
        base_albedo = mix(moss_dark, moss_olive, detail_macro) * tex_mod * (0.72 + 0.48 * detail_fine);
        base_roughness = clamp(0.86 - 0.15 * detail_fine, 0.45, 0.96);
    } else if (input.material == 3u) {
        // Weathered red-brown industrial brick / rusted rebar / damp earth
        let brick_dark = vec3<f32>(0.21, 0.12, 0.08);
        let brick_terracotta = vec3<f32>(0.45, 0.24, 0.15);
        base_albedo = mix(brick_dark, brick_terracotta, detail_macro)
            * tex_mod
            * (0.74 + 0.46 * detail_fine)
            * (0.88 + 0.24 * detail_micro)
            * mortar_groove;
        base_roughness = clamp(0.75 - 0.22 * detail_fine, 0.30, 0.92);
    } else if (input.material == 4u) {
        // Standing rainwater puddle / wet mud mirror
        base_albedo = vec3<f32>(0.05, 0.07, 0.08) * (0.85 + 0.3 * detail_fine);
        base_roughness = 0.03;
        wetness = 1.0;
    } else {
        base_albedo = vec3<f32>(0.30, 0.29, 0.28) * tex_mod * (0.80 + 0.36 * detail_fine);
        base_roughness = 0.72;
    }

    // Upward-facing horizontal surfaces accumulate rain puddles in low macro-noise pockets
    let up_facing = clamp(geom_n.y, 0.0, 1.0);
    let puddle_mask = up_facing * smoothstep(0.40, 0.64, 1.0 - detail_macro) * wetness;
    let surface_wetness = clamp(wetness * 0.40 + puddle_mask * 0.90, 0.0, 1.0);

    // Subtle concentric capillary raindrop ripples on standing water puddles
    if (puddle_mask > 0.15 || input.material == 4u) {
        let ripple_cell = floor(input.world_pos.xz * 1.8);
        let ripple_center = (ripple_cell + vec2<f32>(0.5, 0.5)) / 1.8;
        let ripple_dist = length(input.world_pos.xz - ripple_center);
        let ripple_phase = fract(camera.camera_eye.w * 0.85 + hash31(vec3<f32>(ripple_cell, 3.7)));
        let ring_wave = sin((ripple_dist - ripple_phase * 0.45) * 48.0)
                      * exp(-ripple_dist * 9.0)
                      * (1.0 - ripple_phase);
        let dir_xz = normalize(input.world_pos.xz - ripple_center + vec2<f32>(1e-4));
        fbm_grad += vec3<f32>(dir_xz.x, 0.0, dir_xz.y) * ring_wave * 0.35;
    }

    // Project 3D FBM gradient onto the tangent plane of geom_n (flattened over puddles)
    let tangent_grad = fbm_grad - geom_n * dot(fbm_grad, geom_n);
    let relief_scale = (1.0 - puddle_mask * 0.88) * 0.105;
    let n = normalize(geom_n - tangent_grad * relief_scale);

    // Porous dielectric darkening when wet (internal total reflection in water film)
    let porous_albedo = pow(max(base_albedo, vec3<f32>(0.001)), vec3<f32>(1.38)) * 0.54;
    let albedo = mix(base_albedo, porous_albedo, surface_wetness * (1.0 - metallic));
    let raw_roughness = mix(base_roughness, 0.030, puddle_mask);

    // Tokuyoshi & Kaplanyan 2021 Specular Anti-Aliasing
    let roughness = geometric_specular_aa(n, raw_roughness);
    let clearcoat_weight = smoothstep(0.20, 0.80, surface_wetness);

    let to_cam = camera.camera_eye.xyz - input.world_pos;
    let cam_dist = length(to_cam);
    let view_dir = to_cam / max(cam_dist, 1e-4);
    let n_dot_v = max(dot(n, view_dir), 1e-4);
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);

    // ------------------------------------------------------------------------
    // 1. Per-Pixel 3D Trilinear Voxel Light Field + Bent-Normal GI + Soft Shadows
    // ------------------------------------------------------------------------
    let sun_dir = normalize(vec3<f32>(0.42, 0.82, 0.38));
    var light_sample = input.light;
    var bent_normal = n;
    var ray_shadow = 1.0;
    var puddle_env_occlusion = 1.0;

    if (input.chunk_slot != 0xffffffffu) {
        let chunk_origin = vec3<f32>(chunk_meta[input.chunk_slot].relative_origin.xyz);
        let air_pos = (input.world_pos - chunk_origin) + geom_n * 0.65;
        // Blend 75% continuous 3D trilinear voxel light field with 25% vertex light for smooth cross-chunk seams
        let tri_light = sample_light_trilinear(input.chunk_slot, air_pos);
        light_sample = mix(input.light, tri_light, 0.75);
        bent_normal = compute_light_field_bent_normal(input.chunk_slot, air_pos, n);
        ray_shadow = trace_voxel_light_shadow(input.chunk_slot, air_pos + sun_dir * 0.35, sun_dir, 5);

        // Ray-march reflected view ray in the 3D voxel light field starting 1.2m above the floor
        // so grazing reflections never self-intersect the floor slab while still catching pillars/walls!
        if (clearcoat_weight > 0.15) {
            let refl_ray = normalize(reflect(-view_dir, n) + vec3<f32>(0.0, 0.35, 0.0));
            let occ = trace_voxel_light_shadow(input.chunk_slot, air_pos + geom_n * 0.55, refl_ray, 4);
            puddle_env_occlusion = mix(0.55, 1.0, occ);
        }
    }

    let sky_exposure = clamp(light_sample.x, 0.05, 1.0);
    // Soften vertex AO curve so corners have realistic contact shadows without black grid stripes
    let smooth_vertex_ao = mix(0.20, 1.0, pow(clamp(input.ao, 0.0, 1.0), 0.78));
    let raw_ao = clamp(smooth_vertex_ao * (0.84 + 0.16 * detail_fine) * mortar_groove, 0.07, 1.0);
    let multibounce_ao = gtao_multibounce(raw_ao, albedo);

    // CIE Overcast Sky evaluated along the 3D Bent Normal direction (window/skylight light-shaft directionality!)
    let zenith_radiance = vec3<f32>(1.52, 1.70, 1.96) * (0.14 + 0.86 * sky_exposure);
    let ground_bounce = vec3<f32>(0.25, 0.21, 0.17) * (0.16 + 0.84 * sky_exposure);
    let cie_factor = (1.0 + 2.0 * max(bent_normal.y, 0.0)) / 3.0;
    let directional_window_wrap = 0.65 + 0.35 * max(dot(n, bent_normal), 0.0);
    let sky_irradiance = mix(ground_bounce, zenith_radiance, clamp(0.30 + 0.70 * cie_factor, 0.0, 1.0))
                       * directional_window_wrap;

    // Directional overcast cloud-break skylight with 3D Voxel Ray-Marched Soft Shadows
    let sun_visibility = smoothstep(0.34, 0.86, sky_exposure) * raw_ao * ray_shadow;
    let sun_radiance = vec3<f32>(2.95, 2.76, 2.48) * sun_visibility;
    let n_dot_l = max(dot(n, sun_dir), 0.0);
    let h_sun = normalize(view_dir + sun_dir);
    let sun_spec = d_ggx(max(dot(n, h_sun), 0.0), roughness)
        * v_smith_ggx_correlated(n_dot_v, max(n_dot_l, 1e-4), roughness)
        * f_schlick(max(dot(view_dir, h_sun), 0.0), f0)
        * n_dot_l;
    let sun_direct = (albedo / PI * n_dot_l + sun_spec) * sun_radiance;

    // Warm sodium-vapor / tungsten block-light emission from voxel light volume (`light_sample.y`)
    let block_light = pow(clamp(light_sample.y, 0.0, 1.0), 1.45) * vec3<f32>(3.35, 2.15, 1.08);

    // Specular sky reflection with Lagarde Specular Occlusion + 3D Puddle Ray-Marched Wall Occlusion
    let refl_dir = reflect(-view_dir, n);
    let spec_ao = compute_specular_occlusion(n_dot_v, raw_ao, roughness) * puddle_env_occlusion;
    let sky_refl_luma = (1.0 + 2.0 * max(refl_dir.y, 0.0)) / 3.0;
    let env_radiance = mix(ground_bounce * 0.70, zenith_radiance * 1.72, sky_refl_luma);
    let env_fresnel = f_schlick(n_dot_v, mix(f0, vec3<f32>(0.025), clearcoat_weight * 0.68));
    let env_specular = env_radiance * env_fresnel * spec_ao * (1.0 - roughness * 0.68);

    // ------------------------------------------------------------------------
    // 2. Tactical Dual-Zone IES Flashlight + Volumetric Dust Beam
    // ------------------------------------------------------------------------
    let flashlight_direct = evaluate_tactical_flashlight(
        input.world_pos,
        n,
        view_dir,
        albedo,
        roughness,
        f0,
        clearcoat_weight,
    ) * mix(0.42, 1.0, raw_ao);

    let volumetric_beam = flashlight_volumetric_inscatter(camera.camera_eye.xyz, input.world_pos);

    var hdr_color = (albedo * (sky_irradiance + block_light) * multibounce_ao)
        + sun_direct
        + env_specular
        + flashlight_direct
        + volumetric_beam;

    // ------------------------------------------------------------------------
    // 3. Atmospheric Overcast Aerial Perspective (Subtle Depth Mist)
    // ------------------------------------------------------------------------
    let fog_density = 0.0055;
    let fog_factor = 1.0 - exp(-max(cam_dist - 4.0, 0.0) * fog_density);
    let fog_radiance = vec3<f32>(1.18, 1.28, 1.44);
    hdr_color = mix(hdr_color, fog_radiance, clamp(fog_factor, 0.0, 0.75));

    let alpha = select(tex_sample.a, 0.85, input.material == 4u);
    return vec4<f32>(max(hdr_color, vec3<f32>(0.0)), alpha);
}
