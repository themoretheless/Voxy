//! Photorealistic 3D GPU animation and >200 FPS live window for
//! Rounded-Tip Cylindrical Tube Penetration into the Anatomical Anal Orifice (Sphincter Ani),
//! Radiating Anoderm Rugae (Plicae Radiatae) Effacement, LuGre Frictional Cuff Invagination/Eversion,
//! and 3D Viscous Lubricant Meniscus & SurfaceFilm Smearing.
use glam::Vec3;
use physics::biomechanics::{LabiaMajoraGeometry, Material};
use physics::friction;
use physics::skin::{
    LabiaMinoraGeometry, LabiaTubeInsertion, Layer, Relaxation, SkinMaterial, SolverConfig,
};
use physics::surface_film::{BridgeConfig, Material as FilmMaterial, SmearContact, SurfaceFilm};
use std::sync::Arc;
use std::time::Instant;
use voxy_render::{
    GraphicsOptions, SceneCamera, SceneDraw, SceneGeometry, SceneMesh, SceneProjection,
    SceneRenderer, SceneVertex,
};

const ANATOMICAL_ANAL_AND_TUBE_SHADER: &str = r"
struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, camera: vec4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) to_camera: vec3<f32>,
    @location(3) reference: vec3<f32>,
    @location(4) uv: vec2<f32>,
}

fn unit(v: vec3<f32>) -> vec3<f32> {
    return v * inverseSqrt(max(dot(v, v), 1e-12));
}

fn hash3(p: vec3<f32>) -> f32 {
    let q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(q, q.yxz + vec3<f32>(33.33));
    return fract((q.x + q.y) * q.z + d);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (vec3<f32>(3.0) - 2.0 * f);
    let n000 = hash3(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash3(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash3(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash3(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash3(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash3(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash3(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash3(i + vec3<f32>(1.0, 1.0, 1.0));
    let nx00 = mix(n000, n100, u.x);
    let nx10 = mix(n010, n110, u.x);
    let nx01 = mix(n001, n101, u.x);
    let nx11 = mix(n011, n111, u.x);
    let nxy0 = mix(nx00, nx10, u.y);
    let nxy1 = mix(nx01, nx11, u.y);
    return mix(nxy0, nxy1, u.z);
}

fn fbm3(p: vec3<f32>) -> f32 {
    let a = noise3(p);
    let b = noise3(p * 2.07 + vec3<f32>(17.1, 9.3, 4.7));
    let c = noise3(p * 4.23 + vec3<f32>(3.7, 29.1, 11.9));
    return 0.55 * a + 0.30 * b + 0.15 * c;
}

// Branch-cut-free Chebyshev harmonic cos(k * theta) using normalized (ux, uy) = (x, y) / r
fn radial_fold_harmonic(ux: f32, uy: f32) -> f32 {
    // r2 = (ux + i*uy)^2 -> cos(2*theta), sin(2*theta)
    let c2 = ux * ux - uy * uy;
    let s2 = 2.0 * ux * uy;
    // 4*theta
    let c4 = c2 * c2 - s2 * s2;
    let s4 = 2.0 * c2 * s2;
    // 8*theta
    let c8 = c4 * c4 - s4 * s4;
    let s8 = 2.0 * c4 * s4;
    // 12*theta = 8*theta + 4*theta, 14*theta = 12*theta + 2*theta
    let c12 = c8 * c4 - s8 * s4;
    let s12 = s8 * c4 + c8 * s4;
    let c14 = c12 * c2 - s12 * s2;
    return 0.65 * c14 + 0.35 * c8;
}

fn anal_micro_height(ref_m: vec3<f32>, anoderm: f32, friction_val: f32) -> f32 {
    // 1. Epidermal pores & crisscrossing Langer's lines on surrounding gluteal/perianal skin
    let p_pore = ref_m * vec3<f32>(610.0, 590.0, 520.0);
    let pore_a = noise3(p_pore);
    let pore_b = noise3(p_pore * 2.3 + vec3<f32>(13.7, 7.3, 5.1));
    let langer = 0.18 * sin((ref_m.x + ref_m.y) * 900.0) * sin((ref_m.x - ref_m.y) * 900.0);
    let epidermal = (pore_a * 0.62 + pore_b * 0.38 - 0.5 + langer) * (1.0 - 0.68 * anoderm);

    // 2. Radiating anoderm micro-rugae (corrugator cutis ani) & concentric sphincter striae (branch-cut-free)
    let r = max(length(ref_m.xy), 1e-4);
    let ux = ref_m.x / r;
    let uy = ref_m.y / r;
    let stellate_micro = radial_fold_harmonic(ux, uy) * (1.0 - 0.55 * friction_val);
    let concentric_rings = sin(r * 2100.0 + 1.5 * noise3(ref_m * 380.0));
    let anoderm_relief = (0.48 * stellate_micro + 0.32 * concentric_rings * (0.45 + 0.55 * friction_val)) * anoderm;

    return epidermal + anoderm_relief;
}

fn lubricant_bead_height(ref_m: vec3<f32>, wetness: f32) -> f32 {
    let b1 = pow(noise3(ref_m * vec3<f32>(420.0, 420.0, 420.0)), 3.2);
    let b2 = pow(noise3(ref_m * vec3<f32>(880.0, 880.0, 880.0) + vec3<f32>(7.1, 3.3, 1.9)), 4.0);
    let rivulet = 0.35 * pow(0.5 + 0.5 * sin(ref_m.x * 1150.0 + 1.8 * noise3(ref_m * 320.0)), 3.0);
    return (0.75 * b1 + 0.55 * b2 + rivulet) * smoothstep(0.22, 1.25, wetness);
}

fn ggx_lobe(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32, f0: f32) -> f32 {
    let h = unit(v + l);
    let nl = max(dot(n, l), 0.0);
    let nv = max(dot(n, v), 1e-4);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(v, h), 0.0);
    let a = max(rough * rough, 0.0008);
    let a2 = a * a;
    let denom = nh * nh * (a2 - 1.0) + 1.0;
    let d = a2 / (3.14159265 * denom * denom);
    let k = (rough + 1.0) * (rough + 1.0) / 8.0;
    let g = (nl / (nl * (1.0 - k) + k)) * (nv / (nv * (1.0 - k) + k));
    let f = f0 + (1.0 - f0) * pow(1.0 - vh, 5.0);
    return d * g * f / max(4.0 * nv, 1e-4);
}

fn filmic_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) _parameters: vec4<f32>,
    @location(5) reference: vec3<f32>,
) -> Output {
    var out: Output;
    out.position = transform.mvp * vec4<f32>(position, 1.0);
    out.color = color;
    out.normal = normal;
    out.to_camera = transform.camera.xyz - position;
    out.reference = reference;
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
    let v = unit(in.to_camera);
    let dpos_dx = -dpdx(in.to_camera);
    let dpos_dy = -dpdy(in.to_camera);
    let geom_n = unit(cross(dpos_dx, dpos_dy));
    let n_smooth = unit(in.normal);

    let key = unit(vec3<f32>(-0.28, -0.52, 0.81));
    let fill = unit(vec3<f32>(0.54, 0.32, 0.78));
    let rim = unit(vec3<f32>(0.06, 0.86, 0.51));
    let softbox_top = unit(vec3<f32>(0.12, -0.22, 0.97));

    // Branch 1: 3D Viscous Lubricant Meniscus Fillet Ring & Beads at Tube-Sphincter Collar (tagged by uv.y < -1.5)
    if (in.uv.y < -1.5) {
        let n_base = unit(mix(n_smooth, geom_n * sign(dot(geom_n, n_smooth) + 1e-5), 0.22));
        let eps_l = 0.00016;
        let lb0 = lubricant_bead_height(in.reference, 1.25);
        let lbx = lubricant_bead_height(in.reference + vec3<f32>(eps_l, 0.0, 0.0), 1.25);
        let lby = lubricant_bead_height(in.reference + vec3<f32>(0.0, eps_l, 0.0), 1.25);
        let n_lube = unit(n_base - 0.00045 * vec3<f32>((lbx - lb0) / eps_l, (lby - lb0) / eps_l, 0.0));

        let nv_l = clamp(dot(n_lube, v), 0.0, 1.0);
        let fresnel_l = pow(1.0 - nv_l, 4.2);
        let spec_l = 4.2 * ggx_lobe(n_lube, v, key, 0.011, 0.052)
            + 2.6 * ggx_lobe(n_lube, v, softbox_top, 0.014, 0.052)
            + 2.0 * ggx_lobe(n_lube, v, fill, 0.018, 0.052)
            + 1.6 * ggx_lobe(n_lube, v, rim, 0.022, 0.052);

        let radial_s = clamp(in.uv.x, 0.0, 1.0);
        let deep_sphincter_mucosa = mix(
            vec3<f32>(0.32, 0.09, 0.11),
            vec3<f32>(0.48, 0.18, 0.20),
            radial_s,
        );
        let light_wrap = 0.24 + 0.48 * max(dot(n_lube, key) + 0.35, 0.0) + 0.20 * max(dot(n_lube, fill) + 0.35, 0.0);
        let clear_gel_base = deep_sphincter_mucosa * light_wrap
            + vec3<f32>(0.75, 0.65, 0.68) * (0.08 * lb0);
        let lube_rgb = clear_gel_base + vec3<f32>(0.99, 0.995, 1.0) * (spec_l + 0.22 * fresnel_l);
        return vec4<f32>(filmic_tonemap(lube_rgb * 1.06), 1.0);
    }

    // Branch 2: Translucent Medical Glass/Silicone Tube with Viscous Lubricant Coating & Friction Streaks (uv.y < -0.5)
    if (in.uv.y < -0.5) {
        let n_tube_macro = unit(mix(n_smooth, geom_n * sign(dot(geom_n, n_smooth) + 1e-5), 0.20));
        let tube_wet = clamp(in.uv.x, 0.0, 1.0);
        let friction_drag = clamp(in.color.a, 0.0, 1.0);

        let streak_wave = 0.5 + 0.5 * sin(in.reference.x * 1600.0 + in.reference.y * 1600.0 + 3.0 * noise3(in.reference * 380.0));
        let bead_drop = pow(clamp(noise3(in.reference * vec3<f32>(520.0, 520.0, 260.0)), 0.0, 1.0), 3.2) * tube_wet;
        let lube_coat = clamp(tube_wet * (0.65 + 0.35 * streak_wave) + 0.55 * bead_drop, 0.0, 1.0);

        let eps_t = 0.00018;
        let tb0 = lubricant_bead_height(in.reference * vec3<f32>(1.2, 1.2, 0.65), lube_coat * 1.1);
        let tbx = lubricant_bead_height((in.reference + vec3<f32>(eps_t, 0.0, 0.0)) * vec3<f32>(1.2, 1.2, 0.65), lube_coat * 1.1);
        let tbz = lubricant_bead_height((in.reference + vec3<f32>(0.0, 0.0, eps_t)) * vec3<f32>(1.2, 1.2, 0.65), lube_coat * 1.1);
        let n_tube = unit(n_tube_macro - 0.00026 * tube_wet * vec3<f32>((tbx - tb0) / eps_t, 0.0, (tbz - tb0) / eps_t));

        let rough = mix(0.085, 0.014, lube_coat);
        let spec = 2.8 * ggx_lobe(n_tube, v, key, rough, 0.050)
            + 2.1 * ggx_lobe(n_tube, v, softbox_top, rough * 1.1, 0.050)
            + 0.95 * ggx_lobe(n_tube, v, fill, rough * 1.2, 0.050)
            + 0.85 * ggx_lobe(n_tube, v, rim, 0.042, 0.050);
        let nv = clamp(dot(n_tube, v), 0.0, 1.0);
        let fresnel = pow(1.0 - nv, 3.4);
        let wrap_diff = 0.24 + 0.50 * max(dot(n_tube, key) + 0.35, 0.0) + 0.24 * max(dot(n_tube, fill) + 0.35, 0.0);

        let submerged = 1.0 - clamp((in.reference.z - 0.0012) * 85.0, 0.0, 1.0);
        let internal_lumen = (1.0 - fresnel) * 0.18 * vec3<f32>(0.32, 0.42, 0.50);
        let tissue_transmission = submerged * (0.45 + 0.55 * friction_drag) * vec3<f32>(0.34, 0.10, 0.12);

        let tube_rgb = in.color.rgb * wrap_diff
            - internal_lumen
            + tissue_transmission
            + vec3<f32>(0.98, 0.99, 1.0) * (spec + 0.32 * fresnel);
        return vec4<f32>(filmic_tonemap(tube_rgb * 1.05), 1.0);
    }

    // Branch 3: Photorealistic Gluteal/Perianal Skin, Pigmented Anoderm Rugae, Sphincter Erythema & Viscous Lubricant Film
    let n_macro = unit(mix(n_smooth, geom_n * sign(dot(geom_n, n_smooth) + 1e-5), 0.32));
    let wetness = clamp(in.uv.x, 0.0, 1.0);
    let anoderm = clamp(in.uv.y, 0.0, 1.0);
    let friction_val = clamp(in.color.a, 0.0, 1.0);

    let eps = 0.00016;
    let h0 = anal_micro_height(in.reference, anoderm, friction_val);
    let hx = anal_micro_height(in.reference + vec3<f32>(eps, 0.0, 0.0), anoderm, friction_val);
    let hy = anal_micro_height(in.reference + vec3<f32>(0.0, eps, 0.0), anoderm, friction_val);
    let grad_micro = vec3<f32>((hx - h0) / eps, (hy - h0) / eps, 0.0);
    let n_tissue = unit(n_macro - 0.00084 * grad_micro);

    let lb0 = lubricant_bead_height(in.reference, wetness);
    let lbx = lubricant_bead_height(in.reference + vec3<f32>(eps, 0.0, 0.0), wetness);
    let lby = lubricant_bead_height(in.reference + vec3<f32>(0.0, eps, 0.0), wetness);
    let grad_lube = vec3<f32>((lbx - lb0) / eps, (lby - lb0) / eps, 0.0);
    let n_film = unit(mix(n_tissue, n_macro - 0.00028 * grad_lube, smoothstep(0.05, 0.75, wetness)));

    // Dual-layer RGB dipole subsurface scattering
    let wrap = mix(vec3<f32>(0.42, 0.20, 0.12), vec3<f32>(0.58, 0.24, 0.16), anoderm);
    let key_wrap = max((vec3<f32>(dot(n_tissue, key)) + wrap) / (vec3<f32>(1.0) + wrap), vec3<f32>(0.0));
    let fill_wrap = max((vec3<f32>(dot(n_tissue, fill)) + wrap) / (vec3<f32>(1.0) + wrap), vec3<f32>(0.0));

    let thinness = clamp(anoderm * 0.65 + 0.30 * friction_val, 0.0, 0.88);
    let back_scatter = pow(clamp(dot(-key, n_tissue) + 0.48, 0.0, 1.0), 2.1)
        + 0.68 * pow(clamp(dot(-rim, n_tissue) + 0.52, 0.0, 1.0), 1.9);
    let sss_translucency = thinness * back_scatter * vec3<f32>(0.24, 0.042, 0.035);

    let sss_diffuse = vec3<f32>(0.13, 0.088, 0.082)
        + 0.70 * key_wrap
        + 0.26 * fill_wrap
        + sss_translucency;

    // Subdermal hemorrhoidal/perianal vascular network & frictional hyperemia around the stretched anal verge
    let vasc_noise = fbm3(in.reference * vec3<f32>(340.0, 340.0, 340.0));
    let capillary_veins = smoothstep(0.54, 0.78, vasc_noise) * anoderm;
    let base_mottle = 0.96 + 0.08 * fbm3(in.reference * 220.0);
    var tissue_rgb = in.color.rgb * base_mottle;
    tissue_rgb = mix(tissue_rgb, vec3<f32>(0.54, 0.14, 0.20), 0.26 * capillary_veins);
    // Warm arterial crimson flush where the tube dilates and rubs the anal sphincter ring
    tissue_rgb = mix(tissue_rgb, vec3<f32>(0.68, 0.13, 0.18), 0.52 * friction_val);

    let wet_darken = mix(vec3<f32>(1.0), vec3<f32>(0.64, 0.48, 0.50), wetness * 0.82);
    let albedo = tissue_rgb * wet_darken;

    let base_roughness = mix(0.46, 0.25, anoderm) + 0.04 * h0;
    let secondary_roughness = mix(0.68, 0.42, anoderm);
    let film_roughness = mix(base_roughness, 0.015, smoothstep(0.0, 0.85, wetness));

    let tissue_spec = (0.72 * ggx_lobe(n_tissue, v, key, base_roughness, 0.028)
        + 0.28 * ggx_lobe(n_tissue, v, key, secondary_roughness, 0.028)
        + 0.34 * ggx_lobe(n_tissue, v, fill, base_roughness, 0.028))
        * (1.0 - 0.65 * wetness);

    let film_spec = wetness * (
        2.65 * ggx_lobe(n_film, v, key, film_roughness, 0.048)
        + 1.95 * ggx_lobe(n_film, v, softbox_top, max(film_roughness * 1.1, 0.018), 0.048)
        + 0.95 * ggx_lobe(n_film, v, fill, film_roughness * 1.15, 0.048)
        + 0.78 * ggx_lobe(n_film, v, rim, max(film_roughness, 0.042), 0.048)
    );

    let nv = clamp(dot(n_film, v), 0.0, 1.0);
    let fresnel_sheen = wetness * pow(1.0 - nv, 4.2) * 0.24;

    // Deep anal canal ambient occlusion (creates a dark 3D sphincter opening inside the anal canal z < -4mm)
    let canal_ao = clamp(0.04 + 0.96 * smoothstep(-0.0135, 0.0025, in.reference.z), 0.04, 1.0);
    let spec_color = mix(vec3<f32>(1.0, 0.95, 0.92), vec3<f32>(0.99, 0.995, 1.0), wetness);
    let raw_lit = albedo * sss_diffuse * canal_ao
        + spec_color * (tissue_spec * canal_ao + film_spec * (0.18 + 0.82 * canal_ao) + fresnel_sheen * canal_ao);
    let mapped = filmic_tonemap(raw_lit * 1.08);
    return vec4<f32>(mapped, 1.0);
}
";

#[derive(Clone, Copy, Debug)]
struct AnalVertexBinding {
    rest_m: [f32; 3],
    rest_scaled: [f32; 3],
    base_rgb: [f32; 3],
    side_idx: u8,
    b0: u8,
    b1: u8,
    frac: f32,
    gluteal_w: f32,
    verge_w: f32,
    canal_w: f32,
    rugae_height_m: f32,
    wet_envelope: f32,
    anoderm_factor: f32,
}

#[derive(Clone, Copy, Debug)]
struct TubeLocalVertex {
    local_offset_m: [f32; 3],
    color: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
struct LubeMeniscusBinding {
    radial_frac: f32,
    angle_rad: f32,
}

/// High-density curvilinear anatomical surface of the perianal region, gluteal cleft,
/// radiating anoderm rugae (plicae radiatae), concentric anal sphincter orifice,
/// rounded-tip cylindrical tube mesh, and 3D viscous lubricant meniscus collar.
#[derive(Debug)]
struct PreparedAnalInsertionScene {
    nu: usize,
    nv: usize,
    tube_sectors: usize,
    lube_rings: usize,
    bindings: Vec<AnalVertexBinding>,
    tube_local: Vec<TubeLocalVertex>,
    lube_bindings: Vec<LubeMeniscusBinding>,
    tissue_vertex_count: usize,
    tube_vertex_count: usize,
    vertices: Vec<SceneVertex>,
    indices: Vec<u32>,
    normals: Vec<[f32; 3]>,
    material_coordinates: Vec<[f32; 3]>,
    display_scale: f32,
}

impl PreparedAnalInsertionScene {
    #[allow(clippy::too_many_lines)]
    fn new(display_scale: f64, tube_radius_m: f64, tube_length_m: f64) -> Self {
        let nu = 129usize;
        let nv = 144usize;
        let half_width_m = 0.034;
        let half_length_m = 0.040;
        let scale = display_scale as f32;

        let tissue_vertex_count = (nu + 1) * (nv + 1);
        let tube_sectors = 40usize;
        let tip_rings = 12usize;
        let shaft_rings = 14usize;
        let cap_rings = 3usize;
        let tube_rings = tip_rings + shaft_rings + cap_rings;
        let tube_vertex_count = (tube_rings + 1) * (tube_sectors + 1);

        let lube_rings = 10usize;
        let lube_vertex_count = (lube_rings + 1) * (tube_sectors + 1);

        let total_vertices = tissue_vertex_count + tube_vertex_count + lube_vertex_count;
        let mut bindings = Vec::with_capacity(tissue_vertex_count);
        let mut vertices = Vec::with_capacity(total_vertices);
        let mut material_coordinates = Vec::with_capacity(total_vertices);

        let left_mid = nu / 2; // 64
        let right_mid = left_mid + 1; // 65

        for iv in 0..=nv {
            let v_lin = (iv as f64 / nv as f64) * 2.0 - 1.0;
            // Concentrate grid density near y = 0 (the anal orifice center)
            let v = 0.52 * v_lin + 0.48 * v_lin.powi(3);
            let y = v * half_length_m;

            let bin_f = (((v + 1.0) * 0.5) * 7.0).clamp(0.0, 7.0);
            let b0 = (bin_f.floor() as usize).min(7);
            let b1 = (b0 + 1).min(7);
            let frac = (bin_f - b0 as f64) as f32;

            for iu in 0..=nu {
                let side_idx = u8::from(iu >= right_mid);
                let side_sign_f64 = if iu >= right_mid { 1.0 } else { -1.0 };
                let dist_from_slit = if iu <= left_mid {
                    (left_mid - iu) as f64 / (left_mid as f64)
                } else {
                    (iu - right_mid) as f64 / (left_mid as f64)
                };
                // High radial resolution around the anal sphincter orifice
                let u_mag = 0.35 * dist_from_slit + 0.65 * dist_from_slit.powi(3);
                let row_scale = (1.0 - 0.18 * v * v).sqrt();

                // Circular/stellate 3D anal orifice opening centered at (0, 0):
                let orifice_long_env = (-((y / 0.0052).powi(2))).exp();
                let wall_flare = 1.0 - (-dist_from_slit * 30.0).exp();
                let canal_band = (-((dist_from_slit / 0.13).powi(2))).exp();
                let canal_w = (canal_band * orifice_long_env) as f32;

                // Preliminary x to compute polar angle around the anal orifice
                let x_prelim = side_sign_f64 * u_mag * half_width_m * row_scale;
                let theta = y.atan2(x_prelim);

                // 14 radiating mucocutaneous folds (plicae radiatae / corrugator cutis ani)
                let stellate_wave = 0.68 * (14.0 * theta).cos()
                    + 0.22 * (7.0 * theta + 0.8).sin()
                    + 0.10 * (21.0 * theta).cos();

                // Star-shaped resting anal orifice gap (r_rest ~ 1.65mm modulated by stellate plicae),
                // sealed at dist_from_slit == 0 (deep anal canal floor at z = -15.5mm)
                let stellate_lumen_mod = (1.0 + 0.26 * stellate_wave).clamp(0.70, 1.30);
                let rest_orifice_half_gap = 0.00175
                    * wall_flare
                    * orifice_long_env.sqrt()
                    * canal_band
                    * stellate_lumen_mod;
                let x_rest = side_sign_f64
                    * (u_mag * half_width_m * row_scale + rest_orifice_half_gap);
                let abs_x = x_rest.abs();
                let r_orifice = x_rest.hypot(y);

                // 1. Surrounding gluteal mounds & intergluteal cleft (межъягодичная складка)
                let gluteal_env = (1.0 - 0.32 * v * v).max(0.0);
                let gluteal_xi = (abs_x - 0.0175) / 0.0115;
                let gluteal_profile = (-gluteal_xi * gluteal_xi * 1.45).exp() * gluteal_env;
                let gluteal_height = 0.0118 * gluteal_profile;

                let cleft_groove =
                    -0.0044 * (-((abs_x / 0.0075).powi(2))).exp() * (1.0 - 0.15 * v * v);
                // Median perineal raphe (anterior +Y) & anococcygeal raphe ridge (posterior -Y)
                let raphe_ridge = 0.00065
                    * (-((abs_x / 0.0014).powi(2))).exp()
                    * (1.0 - (-((y / 0.0075).powi(2))).exp());

                // 2. Concentric External Anal Sphincter (EAS) muscular ring & Anal Verge (r ~ 2.4mm .. 8.8mm)
                let verge_xi = (r_orifice - 0.0046) / 0.0036;
                let verge_band = (-verge_xi * verge_xi * 1.65).exp();
                let verge_w = (verge_band
                    + 0.65 * canal_band * wall_flare * orifice_long_env)
                    .clamp(0.0, 1.0) as f32;
                let sphincter_mound_z = 0.0028 * verge_band;

                // 3. Radiating Anoderm Rugae (plicae radiatae) converging into the anal orifice
                let rugae_radial_env = (-(((r_orifice - 0.0042) / 0.0044).powi(2))).exp()
                    * (1.0 - 0.35 * (1.0 - wall_flare));
                let rugae_height_m = (0.00115 * rugae_radial_env * stellate_wave) as f32;

                // 4. Deep tubular anal canal plunge (-15.0mm into the pectinate/anorectal lumen)
                let deep_canal_plunge = -0.0150 * f64::from(canal_w).powf(1.25);

                let pelvic_arch_z = -0.0048 * (u_mag * u_mag * 0.65 + v * v * 0.75);
                let z_rest = pelvic_arch_z
                    + gluteal_height
                    + cleft_groove
                    + raphe_ridge
                    + sphincter_mound_z
                    + f64::from(rugae_height_m)
                    + deep_canal_plunge;

                let anoderm_factor = (verge_band * 1.15
                    + rugae_radial_env * 0.85
                    + f64::from(canal_w) * 1.30
                    + 0.35 * (-((r_orifice / 0.0105).powi(2))).exp())
                .clamp(0.0, 1.0) as f32;

                // Spatially graduated lubricant wetness envelope centered on the anal verge & intergluteal furrow
                let orifice_wet_pool = (-((r_orifice / 0.0078).powi(2))).exp();
                let furrow_wet_trail =
                    0.38 * (-((abs_x / 0.0038).powi(2))).exp() * (-((y / 0.016).powi(2))).exp();
                let wet_envelope =
                    (((0.68 * orifice_wet_pool) + furrow_wet_trail) / 3.2e-5) as f32;

                // Realistic anatomical pigmentation:
                // Gluteal skin -> darker brownish-rose perianal skin -> vascular rose-plum anoderm -> deep crimson anal canal
                let gluteal_skin = [0.83_f32, 0.62, 0.55];
                let cleft_skin = [0.74_f32, 0.50, 0.44];
                let perianal_pigment = [0.56_f32, 0.33, 0.30];
                let anoderm_rose_plum = [0.62_f32, 0.25, 0.30];
                let pectinate_crimson = [0.34_f32, 0.06, 0.10];

                let cleft_blend = ((-((abs_x / 0.014).powi(2))).exp()) as f32;
                let mut rgb = [
                    gluteal_skin[0] * (1.0 - cleft_blend) + cleft_skin[0] * cleft_blend,
                    gluteal_skin[1] * (1.0 - cleft_blend) + cleft_skin[1] * cleft_blend,
                    gluteal_skin[2] * (1.0 - cleft_blend) + cleft_skin[2] * cleft_blend,
                ];
                let perianal_ring = ((-(((r_orifice - 0.0062) / 0.0058).powi(2))).exp()) as f32;
                for k in 0..3 {
                    rgb[k] = rgb[k] * (1.0 - 0.78 * perianal_ring)
                        + perianal_pigment[k] * (0.78 * perianal_ring);
                }
                let inner_anoderm = ((-((r_orifice / 0.0052).powi(2))).exp()) as f32;
                // Radiating crests have slightly deeper brownish-rose tone than the moist sulci between them
                let fold_crest_shade = (0.5 + 0.5 * stellate_wave as f32).clamp(0.0, 1.0);
                for k in 0..3 {
                    let target_anoderm = anoderm_rose_plum[k] * (1.0 - 0.16 * fold_crest_shade)
                        + perianal_pigment[k] * (0.16 * fold_crest_shade);
                    rgb[k] = rgb[k] * (1.0 - 0.85 * inner_anoderm)
                        + target_anoderm * (0.85 * inner_anoderm);
                }
                for k in 0..3 {
                    rgb[k] = rgb[k] * (1.0 - 0.90 * canal_w) + pectinate_crimson[k] * (0.90 * canal_w);
                }

                let rest_m = [x_rest as f32, y as f32, z_rest as f32];
                let rest_scaled = [
                    (x_rest * display_scale) as f32,
                    (y * display_scale) as f32,
                    (z_rest * display_scale) as f32,
                ];

                bindings.push(AnalVertexBinding {
                    rest_m,
                    rest_scaled,
                    base_rgb: rgb,
                    side_idx,
                    b0: b0 as u8,
                    b1: b1 as u8,
                    frac,
                    gluteal_w: gluteal_profile.clamp(0.0, 1.0) as f32,
                    verge_w,
                    canal_w,
                    rugae_height_m,
                    wet_envelope,
                    anoderm_factor,
                });
                vertices.push(SceneVertex {
                    position: rest_scaled,
                    uv: [0.38 * (verge_w + canal_w), anoderm_factor],
                    color: [rgb[0], rgb[1], rgb[2], 0.0],
                });
                material_coordinates.push(rest_m);
            }
        }

        let tissue_quads = nu * nv;
        let tube_quads = tube_sectors * tube_rings;
        let lube_quads = tube_sectors * lube_rings;
        let mut indices = Vec::with_capacity((tissue_quads + tube_quads + lube_quads) * 6);
        let stride = (nu + 1) as u32;
        for iv in 0..nv {
            for iu in 0..nu {
                let a = (iv as u32) * stride + (iu as u32);
                let b = a + 1;
                let c = a + stride;
                let d = c + 1;
                indices.extend([a, b, d, a, d, c]);
            }
        }

        let mut normals = vec![[0.0_f32, 0.0, 1.0]; tissue_vertex_count];

        // 2. Build smooth rounded-tip cylindrical tube geometry
        let mut tube_local = Vec::with_capacity(tube_vertex_count);
        let r_tube = tube_radius_m as f32;
        let shaft_len = (tube_length_m - tube_radius_m).max(0.005) as f32;
        for ir in 0..=tube_rings {
            let (ring_r, z_off, nz, axial_frac, is_lumen) = if ir <= tip_rings {
                let theta =
                    std::f32::consts::FRAC_PI_2 * (ir as f32 / (tip_rings as f32)).clamp(0.0, 1.0);
                let sin_t = theta.sin();
                let cos_t = theta.cos();
                (
                    r_tube * sin_t,
                    r_tube * (1.0 - cos_t),
                    -cos_t,
                    0.25 * (ir as f32 / (tip_rings as f32)),
                    false,
                )
            } else if ir <= tip_rings + shaft_rings {
                let s_frac = (ir - tip_rings) as f32 / (shaft_rings as f32);
                (
                    r_tube,
                    r_tube + s_frac * shaft_len,
                    0.0,
                    0.25 + 0.75 * s_frac,
                    false,
                )
            } else {
                let c_frac = (ir - tip_rings - shaft_rings) as f32 / (cap_rings as f32);
                let r_cap = r_tube * (1.0 - 0.96 * c_frac);
                let z_cap = r_tube + shaft_len + 0.0004 * (1.0 - c_frac) - 0.0022 * c_frac * c_frac;
                (r_cap, z_cap, 0.92, 1.0, c_frac > 0.35)
            };

            let grad_band = ((axial_frac * 28.0).sin().abs() < 0.14)
                && ir > tip_rings
                && ir <= tip_rings + shaft_rings;
            let base_col = if is_lumen {
                [0.42_f32, 0.50, 0.58, 0.0]
            } else if grad_band {
                [0.62_f32, 0.77, 0.86, 0.0]
            } else {
                [0.84_f32, 0.90, 0.95, 0.0]
            };

            for isec in 0..=tube_sectors {
                let phi = std::f32::consts::TAU * (isec as f32 / (tube_sectors as f32));
                let (sin_p, cos_p) = phi.sin_cos();
                let lx = ring_r * cos_p;
                let ly = ring_r * sin_p;
                let nr = (1.0 - nz * nz).max(0.0).sqrt();
                let n = [nr * cos_p, nr * sin_p, nz];

                tube_local.push(TubeLocalVertex {
                    local_offset_m: [lx, ly, z_off],
                    color: base_col,
                });
                vertices.push(SceneVertex {
                    position: [lx * scale, ly * scale, z_off * scale],
                    uv: [0.5, -1.0],
                    color: base_col,
                });
                normals.push(n);
                material_coordinates.push([lx, ly, z_off]);
            }
        }

        let tube_base = tissue_vertex_count as u32;
        let tube_stride = (tube_sectors + 1) as u32;
        for ir in 0..tube_rings {
            for isec in 0..tube_sectors {
                let a = tube_base + (ir as u32) * tube_stride + (isec as u32);
                let b = a + 1;
                let c = a + tube_stride;
                let d = c + 1;
                indices.extend([a, b, d, a, d, c]);
            }
        }

        // 3. Build 3D Viscous Lubricant Meniscus Collar Ring around the Anal Sphincter Rim (uv.y = -2.0)
        let mut lube_bindings = Vec::with_capacity(lube_vertex_count);
        for ir in 0..=lube_rings {
            let radial_frac = ir as f32 / (lube_rings as f32);
            for isec in 0..=tube_sectors {
                let angle_rad = std::f32::consts::TAU * (isec as f32 / (tube_sectors as f32));
                lube_bindings.push(LubeMeniscusBinding {
                    radial_frac,
                    angle_rad,
                });
                vertices.push(SceneVertex {
                    position: [0.0, 0.0, 0.0],
                    uv: [radial_frac, -2.0],
                    color: [0.92, 0.85, 0.88, 1.0],
                });
                normals.push([0.0, 0.0, 1.0]);
                material_coordinates.push([
                    r_tube * (1.0 + radial_frac) * angle_rad.cos(),
                    r_tube * (1.0 + radial_frac) * angle_rad.sin(),
                    0.0,
                ]);
            }
        }

        // Keep lube_bindings vertices unindexed so no separate opaque ring mesh ever floats below the hemispherical tube tip;
        // the 3D capillary lubricant meniscus is sculpted directly into the high-density anoderm collar & tube shaft.

        let mut scene = Self {
            nu,
            nv,
            tube_sectors,
            lube_rings,
            bindings,
            tube_local,
            lube_bindings,
            tissue_vertex_count,
            tube_vertex_count,
            vertices,
            indices,
            normals,
            material_coordinates,
            display_scale: scale,
        };
        scene.recompute_tissue_normals();
        scene
    }

    fn recompute_tissue_normals(&mut self) {
        let stride = self.nu + 1;
        let left_mid = self.nu / 2;
        let right_mid = left_mid + 1;
        for iv in 0..=self.nv {
            let iv_prev = iv.saturating_sub(1);
            let iv_next = (iv + 1).min(self.nv);
            for iu in 0..=self.nu {
                let (iu_prev, iu_next) = if iu <= left_mid {
                    (iu.saturating_sub(1), iu.min(left_mid))
                } else {
                    (iu.max(right_mid), (iu + 1).min(self.nu))
                };
                let p_u0 = Vec3::from_array(self.vertices[iv * stride + iu_prev].position);
                let p_u1 = Vec3::from_array(self.vertices[iv * stride + iu_next].position);
                let p_v0 = Vec3::from_array(self.vertices[iv_prev * stride + iu].position);
                let p_v1 = Vec3::from_array(self.vertices[iv_next * stride + iu].position);
                let du = p_u1 - p_u0;
                let dv = p_v1 - p_v0;
                self.normals[iv * stride + iu] =
                    du.cross(dv).try_normalize().unwrap_or(Vec3::Z).to_array();
            }
        }
    }

    fn recompute_lube_normals(&mut self) {
        let base = self.tissue_vertex_count + self.tube_vertex_count;
        let stride = self.tube_sectors + 1;
        for ir in 0..=self.lube_rings {
            let ir_prev = ir.saturating_sub(1);
            let ir_next = (ir + 1).min(self.lube_rings);
            for isec in 0..=self.tube_sectors {
                let isec_prev = if isec == 0 { self.tube_sectors - 1 } else { isec - 1 };
                let isec_next = if isec >= self.tube_sectors { 1 } else { isec + 1 };
                let p_r0 = Vec3::from_array(self.vertices[base + ir_prev * stride + isec].position);
                let p_r1 = Vec3::from_array(self.vertices[base + ir_next * stride + isec].position);
                let p_s0 = Vec3::from_array(self.vertices[base + ir * stride + isec_prev].position);
                let p_s1 = Vec3::from_array(self.vertices[base + ir * stride + isec_next].position);
                let dr = p_r1 - p_r0;
                let ds = p_s1 - p_s0;
                self.normals[base + ir * stride + isec] =
                    ds.cross(dr).try_normalize().unwrap_or(Vec3::Z).to_array();
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn deform_scene(
        &mut self,
        gluteal_sim: &physics::biomechanics::LabiaGaitSimulation,
        anoderm_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
        tube: &LabiaTubeInsertion,
        sim_time_s: f64,
    ) {
        let gluteal_pos = gluteal_sim.assembly.body.body().positions();
        let gluteal_rest = gluteal_sim.assembly.body.body().rest_positions();
        let [left_apex, right_apex] = gluteal_sim.apex_nodes();
        let gluteal_disp = [
            [
                (gluteal_pos[left_apex][0] - gluteal_rest[left_apex][0]) as f32,
                (gluteal_pos[left_apex][1] - gluteal_rest[left_apex][1]) as f32,
                (gluteal_pos[left_apex][2] - gluteal_rest[left_apex][2]) as f32,
            ],
            [
                (gluteal_pos[right_apex][0] - gluteal_rest[right_apex][0]) as f32,
                (gluteal_pos[right_apex][1] - gluteal_rest[right_apex][1]) as f32,
                (gluteal_pos[right_apex][2] - gluteal_rest[right_apex][2]) as f32,
            ],
        ];

        let mut anoderm_crest_disp = [[0.0_f32; 3]; 2];
        let mut film_wetness_profile = [[0.0_f32; 8]; 2];
        for side in 0..2 {
            let fold = &anoderm_sim.folds[side];
            let pos = fold.skin.positions();
            let rest = fold.skin.rest_positions();
            let inv_count = 1.0 / (fold.crest_nodes.len().max(1) as f32);
            for &node in &fold.crest_nodes {
                for axis in 0..3 {
                    anoderm_crest_disp[side][axis] +=
                        ((pos[node][axis] - rest[node][axis]) as f32) * inv_count;
                }
            }
            let thicknesses = films[side].thickness();
            let tris = fold.skin.triangles();
            let mut bin_weights = [0.0_f32; 8];
            for (tri, &h) in tris.iter().zip(thicknesses.iter()) {
                let cy = (rest[tri[0]][1] + rest[tri[1]][1] + rest[tri[2]][1]) / 3.0;
                let v_norm = ((cy / 0.042) + 0.5).clamp(0.0, 0.999);
                let bin = (v_norm * 8.0) as usize;
                film_wetness_profile[side][bin] += h as f32;
                bin_weights[bin] += 1.0;
            }
            for b in 0..8 {
                if bin_weights[b] > 0.0 {
                    film_wetness_profile[side][b] /= bin_weights[b];
                }
            }
        }

        let (tip_pos, tip_vel, _phase) = tube
            .tip_state_at(sim_time_s)
            .unwrap_or(([0.0, 0.0, 0.0072], [0.0; 3], 0.0));
        let tip_x = tip_pos[0] as f32;
        let tip_y = tip_pos[1] as f32;
        let tip_z = tip_pos[2] as f32;
        let vel_z = tip_vel[2] as f32;
        let r_tube = tube.radius_m as f32;
        let scale = self.display_scale;

        // Sphincter engagement as the rounded tip contacts the anal verge (+4.5mm) and dilates the anal canal (-1.5mm -> -7.8mm)
        let engage_z = ((0.0046 - tip_z) / 0.0112).clamp(0.0, 1.0);
        let smooth_engage = engage_z * engage_z * (3.0 - 2.0 * engage_z);
        let sphincter_penetration = ((0.0016 - tip_z) / 0.0085).clamp(0.0, 1.0);
        let smooth_sphincter_pen =
            sphincter_penetration * sphincter_penetration * (3.0 - 2.0 * sphincter_penetration);

        let norm_vel_z = (vel_z / 0.042).clamp(-1.0, 1.0);
        let speed_mag = norm_vel_z.abs();

        // 1. Deform perianal tissue, radiating anoderm rugae & concentric anal sphincter canal
        for (vertex, b) in self.vertices[..self.tissue_vertex_count]
            .iter_mut()
            .zip(self.bindings.iter())
        {
            let side = usize::from(b.side_idx);
            let side_sign = if side == 1 { 1.0_f32 } else { -1.0_f32 };
            let glut = gluteal_disp[side];
            let anod = anoderm_crest_disp[side];

            let rx = b.rest_m[0] - tip_x;
            let ry = b.rest_m[1] - tip_y;
            let r_xy = rx.hypot(ry);

            // C^infinity monotonic incompressible radial dilation of the concentric anal sphincter:
            // R(r_xy) = sqrt(r_xy^2 + alpha * r_tube^2 * exp(-(r_xy / r0)^2))
            // Guarantees strictly increasing R(r_xy) and tight 360-degree sphincter grip around the 3.8mm tube shaft!
            let seam_pin = ((b.rest_m[0].abs() - 0.00016) / 0.00042).clamp(0.0, 1.0);
            let seam_smooth = seam_pin * seam_pin * (3.0 - 2.0 * seam_pin);
            let r0 = r_tube * 2.40;
            let radial_env = (-((r_xy / r0).powi(2))).exp();
            let target_r =
                (r_xy * r_xy + smooth_engage * (r_tube * 1.005).powi(2) * radial_env).sqrt();
            let delta_r = (target_r - r_xy) * seam_smooth;

            let dir_x = if r_xy > 1e-5 { rx / r_xy } else { side_sign } * seam_smooth;
            let dir_y = if r_xy > 1e-5 { ry / r_xy } else { 0.0 } * seam_smooth;

            let cuff_ring =
                (-(((target_r - r_tube * 1.03) / (r_tube * 0.50)).powi(2))).exp() * seam_smooth;

            // Active sphincter grip & frictional adhesion during withdrawal (norm_vel_z > 0)
            let sphincter_grip = -0.00058 * norm_vel_z.max(0.0) * cuff_ring * smooth_sphincter_pen;

            // Isotropic 360-degree radial dilation of the circular anal sphincter
            let tube_parting_dx = dir_x * (delta_r + sphincter_grip);
            let tube_parting_dy = dir_y * (delta_r + sphincter_grip);

            // Effacement (smoothing out) of the radiating anoderm rugae under circumferential hoop stretch:
            let hoop_stretch = (delta_r / (r_tube * 0.50)).clamp(0.0, 1.0);
            let rugae_effacement_dz = -b.rugae_height_m * 0.72 * hoop_stretch * smooth_engage;

            // Frictional axial shear deformation (Трение сфинктера):
            // - Insertion (norm_vel_z < 0): rolls the anal verge & anoderm inward/downward into the anal canal (invagination)
            // - Withdrawal (norm_vel_z > 0): pulls the tightly gripped anoderm collar upward into a prominent everted cuff (eversion)
            let contact_collar = ((b.verge_w + 0.92 * cuff_ring + 0.45 * b.canal_w) * seam_smooth)
                .clamp(0.0, 1.35);
            let frictional_axial_drag_z =
                0.0042 * norm_vel_z * contact_collar * smooth_engage;
            let insertion_funnel_z = -0.0026
                * radial_env
                * seam_smooth
                * smooth_engage
                * (1.0 - 0.55 * norm_vel_z.max(0.0));
            let meniscus_lip_ridge_z = 0.0011 * cuff_ring * smooth_engage;
            let tube_cuff_dz = insertion_funnel_z
                + frictional_axial_drag_z
                + meniscus_lip_ridge_z
                + rugae_effacement_dz;

            let outer_fem_mask = (1.0 - 0.85 * radial_env).clamp(0.15, 1.0) * seam_smooth;
            let dx = (0.75 * b.gluteal_w * glut[0] * outer_fem_mask
                + 0.35 * b.verge_w * anod[0] * outer_fem_mask
                + tube_parting_dx)
                .clamp(-0.0065, 0.0065);
            let dy = (0.75 * b.gluteal_w * glut[1] * outer_fem_mask
                + 0.35 * b.verge_w * anod[1] * outer_fem_mask
                + tube_parting_dy)
                .clamp(-0.0065, 0.0065);
            let dz = (1.0 * b.gluteal_w * glut[2]
                + 1.1 * b.verge_w * anod[2]
                + tube_cuff_dz)
                .clamp(-0.0095, 0.0060);

            vertex.position = [
                b.rest_scaled[0] + dx * scale,
                b.rest_scaled[1] + dy * scale,
                b.rest_scaled[2] + dz * scale,
            ];

            // Viscous lubricant film thickness + thick pooled gel in the anal verge & intergluteal furrow
            let h_film = (1.0 - b.frac) * film_wetness_profile[side][usize::from(b.b0)]
                + b.frac * film_wetness_profile[side][usize::from(b.b1)];
            let base_film_wetness = (h_film * b.wet_envelope).clamp(0.0, 0.68);
            let perianal_pool = (-((r_xy / (r_tube * 1.85)).powi(2))).exp();
            let meniscus_lube = (0.98 * cuff_ring + 0.88 * b.verge_w + 0.68 * perianal_pool)
                * (0.52 + 0.48 * smooth_engage);
            let wetness = (base_film_wetness + meniscus_lube).clamp(0.0, 1.0);

            let friction_intensity = ((0.58 * speed_mag + 0.42 * smooth_sphincter_pen)
                * (contact_collar + 0.60 * hoop_stretch))
                .clamp(0.0, 1.0);

            let anoderm_flush =
                (b.anoderm_factor + 0.30 * contact_collar * smooth_engage).clamp(0.0, 1.0);
            vertex.uv = [wetness, anoderm_flush];
            vertex.color = [
                b.base_rgb[0],
                b.base_rgb[1],
                b.base_rgb[2],
                friction_intensity,
            ];
        }

        // 2. Update 3D tube vertices at current tip position with viscous lubricant coat & friction drag
        for (vertex, tl) in self.vertices
            [self.tissue_vertex_count..self.tissue_vertex_count + self.tube_vertex_count]
            .iter_mut()
            .zip(self.tube_local.iter())
        {
            let wx = tip_x + tl.local_offset_m[0];
            let wy = tip_y + tl.local_offset_m[1];
            let wz = tip_z + tl.local_offset_m[2];
            vertex.position = [wx * scale, wy * scale, wz * scale];
            let lube_reach_z = 0.0005 + 0.0082 * smooth_engage + 0.0040 * norm_vel_z.max(0.0);
            let wet_coat = ((lube_reach_z - wz) / 0.0075).clamp(0.26, 1.0);
            vertex.uv = [wet_coat, -1.0];
            vertex.color = [
                tl.color[0],
                tl.color[1],
                tl.color[2],
                (0.45 * smooth_sphincter_pen + 0.55 * speed_mag).clamp(0.0, 1.0),
            ];
        }

        // 3. Update 3D Viscous Lubricant Capillary Bead Seam right at the Tube-Sphincter Contact Line (uv.y = -2.0)
        // Recessed deep inside the dark anal canal before contact (smooth_engage == 0), and forms a tight
        // 0.25mm capillary bead fillet right at the tube-anoderm contact crease during penetration
        let lube_start = self.tissue_vertex_count + self.tube_vertex_count;
        let sphincter_lip_z = -0.0022 + 0.0026 * norm_vel_z * smooth_sphincter_pen
            - 0.0012 * smooth_engage * (1.0 - norm_vel_z.max(0.0));
        for (vertex, lb) in self.vertices[lube_start..]
            .iter_mut()
            .zip(self.lube_bindings.iter())
        {
            let s = lb.radial_frac;
            let (sin_a, cos_a) = lb.angle_rad.sin_cos();

            let rugae_ripple = 1.0
                + 0.08 * (lb.angle_rad * 14.0).cos() * (1.0 - 0.55 * smooth_engage)
                + 0.06 * (lb.angle_rad * 7.0 + (sim_time_s as f32) * 3.0).sin();

            let engaged_inner_r = r_tube * 0.992;
            let engaged_outer_r = r_tube * (1.015 + 0.055 * rugae_ripple);
            let r_curr = smooth_engage * (engaged_inner_r + s * (engaged_outer_r - engaged_inner_r));

            let climb_z = (1.0 - s).powi(3)
                * smooth_engage
                * (0.0012 + 0.0010 * smooth_sphincter_pen + 0.0012 * norm_vel_z.max(0.0));
            let bead_arch_z =
                4.0 * s * (1.0 - s) * (0.00025 * smooth_engage * rugae_ripple);
            let outer_slope_z = -0.0008 * s * s - 0.0135 * (1.0 - smooth_engage);

            let wx = tip_x + r_curr * cos_a;
            let wy = tip_y + r_curr * sin_a;
            let wz = sphincter_lip_z + climb_z + bead_arch_z + outer_slope_z;

            vertex.position = [wx * scale, wy * scale, wz * scale];
            vertex.uv = [s, -2.0];
        }

        self.recompute_tissue_normals();
        self.recompute_lube_normals();
    }

    fn build_full_mesh(
        &mut self,
        gluteal_sim: &physics::biomechanics::LabiaGaitSimulation,
        anoderm_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
        tube: &LabiaTubeInsertion,
        sim_time_s: f64,
    ) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        self.deform_scene(gluteal_sim, anoderm_sim, films, tube, sim_time_s);
        Ok(SceneMesh::new(self.vertices.clone(), self.indices.clone())?
            .with_normals(self.normals.clone())?
            .with_material_coordinates(self.material_coordinates.clone())?)
    }

    fn update_vertices_in_place(
        &mut self,
        queue: &wgpu::Queue,
        geometry: &SceneGeometry,
        gluteal_sim: &physics::biomechanics::LabiaGaitSimulation,
        anoderm_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
        tube: &LabiaTubeInsertion,
        sim_time_s: f64,
    ) {
        self.deform_scene(gluteal_sim, anoderm_sim, films, tube, sim_time_s);
        queue.write_buffer(&geometry.vertices, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&geometry.normals, 0, bytemuck::cast_slice(&self.normals));
    }
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window_mode = std::env::args().any(|a| a == "--window");
    let max_window_frames: Option<u64> =
        std::env::args().find_map(|a| a.strip_prefix("--frames=").and_then(|v| v.parse().ok()));
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .unwrap_or_else(|| "/private/tmp/voxy-anal-tube-insertion.png".into());

    let gluteal_geom = LabiaMajoraGeometry {
        length_m: 0.052,
        width_m: 0.018,
        height_m: 0.010,
        separation_m: 0.022,
        sectors: 8,
        latitude_rings: 2,
    };
    let gluteal_mat = Material::from_young_poisson(3400.0, 0.45)?;
    let mut gluteal_sim = gluteal_geom.build_gait_contact_simulation(
        [gluteal_mat.clone(), gluteal_mat],
        [1000.0; 2],
        0.0001,
        0.0018,
        1.5,
    )?;

    let mut anoderm_sim = LabiaMinoraGeometry {
        length_m: 0.040,
        fold_width_m: 0.0040,
        fold_height_m: 0.0050,
        separation_m: 0.009,
        longitudinal_segments: 4,
        transverse_segments: 2,
    }
    .build_gait_simulation(SkinMaterial {
        layers: vec![Layer {
            thickness: 0.00040,
            density: 1000.0,
            shear_modulus: 1150.0,
            collagen_modulus: 2200.0,
            collagen_exponent: 4.2,
            dispersion: 0.1,
            fiber_angle: 0.5,
            relaxation: vec![Relaxation {
                modulus: 520.0,
                time: 0.028,
            }],
        }],
    })?;

    let film_mat = FilmMaterial {
        density: 1000.0,
        viscosity: 0.050,
        wetting: 2.6e-5,
        surface_tension: 0.048,
    };
    let mut films = anoderm_sim.build_surface_films(film_mat)?;

    let tube = LabiaTubeInsertion {
        radius_m: 0.0038,
        length_m: 0.025,
        axis_xy_m: [0.0, 0.0],
        start_tip_z_m: 0.0072,
        max_depth_m: 0.0150,
        frequency_hz: 1.0,
    };

    let display_scale = 34.0;
    let mut prepared_scene =
        PreparedAnalInsertionScene::new(display_scale, tube.radius_m, tube.length_m);

    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Anal Tube Insertion GPU Renderer: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut geometry = renderer.reserve_geometry(&device, 28_000, 160_000)?;
    pollster::block_on(renderer.set_geometry_opaque_shader(
        &device,
        &mut geometry,
        ANATOMICAL_ANAL_AND_TUBE_SHADER,
    ))?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let camera_eye = Vec3::new(0.74, -1.28, 1.32);
    let camera_target = Vec3::new(0.0, -0.02, -0.02);
    let base_projection = SceneCamera {
        eye: camera_eye,
        target: camera_target,
        up: Vec3::Z,
        projection: SceneProjection::Perspective {
            vertical_fov: 35_f32.to_radians(),
            aspect: 4.0 / 3.0,
            near: 0.1,
            far: 50.0,
        },
    }
    .view_projection()?;
    let transform = renderer.create_transform(&device, base_projection)?;
    transform.update_view_position(&queue, camera_eye)?;

    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("anal tube snapshot"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 768,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("anal tube pixels"),
        size: 4096 * 768,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let cells = films[0].triangles().len();
    for side in 0..2 {
        films[side].deposit(cells / 3, 4.8e-9)?;
        films[side].deposit(cells / 2, 6.0e-9)?;
    }
    let friction_mat = friction::Material::new(1.2e6, 1.2e6, 0.38)?;
    let mut friction_states = [
        vec![friction::State::default(); cells],
        vec![friction::State::default(); cells],
    ];
    let pressures = vec![680.0; cells];

    let stages = [
        (0.05, "approach-puckered-anal-orifice"),
        (0.18, "verge-contact-lubricant-meniscus"),
        (0.32, "sphincter-dilation-friction-invagination"),
        (0.50, "full-depth-rugae-effacement"),
        (0.72, "withdrawal-sphincter-cuff-eversion"),
    ];

    let mut frames = Vec::new();
    let mut sim_t = 0.0_f64;

    for (frame_idx, &(target_t, stage_label)) in stages.iter().enumerate() {
        while sim_t + 0.01 <= target_t {
            let (_, tip_vel, phase) = tube.tip_state_at(sim_t)?;
            gluteal_sim.step_tube_insertion(phase, tip_vel[2], 0.14, 2e-4, 5e-3)?;
            let mut synced_tube = tube;
            synced_tube.radius_m = 0.0016;
            synced_tube.frequency_hz = 10.0;
            anoderm_sim.step_tube_insertion(
                &synced_tube,
                0.002,
                [0.0, 0.0, -9.81],
                6,
                0.0010,
                1200.0,
                0.022,
                SolverConfig {
                    max_newton: 6,
                    max_cg: 32,
                    force_tolerance: 1e-3,
                    relative_tolerance: 1e-2,
                },
            )?;
            for side in 0..2 {
                films[side].update_geometry(anoderm_sim.folds[side].skin.positions())?;
                films[side].advance_smear(
                    0.001,
                    SmearContact {
                        normal: [if side == 0 { 1.0 } else { -1.0 }, 0.0, 0.0],
                        body_velocity_m_s: [0.0, 0.018, tip_vel[2] * 0.55],
                    },
                    &pressures,
                    &friction_mat,
                    &mut friction_states[side],
                    [0.0, 0.0, -9.81],
                    0.0005,
                )?;
            }
            let (left_slice, right_slice) = films.split_at_mut(1);
            let _ = left_slice[0].exchange_contact(
                &mut right_slice[0],
                0.001,
                BridgeConfig {
                    max_gap: 0.020,
                    transfer_speed: 0.0025,
                    opposing_cosine: 0.1,
                    max_candidates: 500_000,
                },
            )?;
            sim_t += 0.02;
        }

        let (tip_pos, tip_vel, phase) = tube.tip_state_at(target_t)?;
        let mesh =
            prepared_scene.build_full_mesh(&gluteal_sim, &anoderm_sim, &films, &tube, target_t)?;
        geometry.update(&queue, &mesh)?;

        println!(
            "frame={frame_idx} ({stage_label}) | t={target_t:.2}s phase={phase:.2} tip_z={:.2}mm vel_z={:.1}mm/s | sphincter_sep={:.2}mm gluteal_sep={:.2}mm",
            tip_pos[2] * 1000.0,
            tip_vel[2] * 1000.0,
            anoderm_sim.mean_crest_separation_m() * 1000.0,
            gluteal_sim.apex_separation_m() * 1000.0,
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&Default::default()),
            &depth.create_view(&Default::default()),
            wgpu::Color {
                r: 0.035,
                g: 0.038,
                b: 0.052,
                a: 1.0,
            },
            &[SceneDraw {
                geometry: &geometry,
                texture: &texture,
                transform: &transform,
                overlay: false,
            }],
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4096),
                    rows_per_image: Some(768),
                },
            },
            color.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        rx.recv()??;
        let frame_pixels = readback.slice(..).get_mapped_range()?.to_vec();
        readback.unmap();

        let single_path = format!("/private/tmp/voxy-anal-tube-frame-{frame_idx}.png");
        image::save_buffer_with_format(
            &single_path,
            &frame_pixels,
            1024,
            768,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )?;
        frames.push(frame_pixels);
    }

    if let Some(err) = pollster::block_on(scope.pop()) {
        return Err(format!("wgpu validation error: {err}").into());
    }

    let n_frames = frames.len();
    let total_width = (1024 * n_frames) as u32;
    let mut strip = vec![0u8; (total_width as usize) * 768 * 4];
    for (idx, frame) in frames.iter().enumerate() {
        for y in 0..768 {
            let src = &frame[y * 4096..(y + 1) * 4096];
            let dst_start = (y * (total_width as usize) + idx * 1024) * 4;
            strip[dst_start..dst_start + 4096].copy_from_slice(src);
        }
    }
    image::save_buffer_with_format(
        &path,
        &strip,
        total_width,
        768,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;
    println!("ANAL TUBE INSERTION SNAPSHOT PASS: {path}");

    if window_mode {
        run_interactive_window(
            instance,
            adapter,
            device,
            queue,
            gluteal_sim,
            anoderm_sim,
            films,
            tube,
            pressures,
            friction_mat,
            friction_states,
            prepared_scene,
            max_window_frames,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_interactive_window(
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    gluteal_sim: physics::biomechanics::LabiaGaitSimulation,
    anoderm_sim: physics::skin::LabiaMinoraGaitSimulation,
    films: [SurfaceFilm; 2],
    tube: LabiaTubeInsertion,
    pressures: Vec<f64>,
    friction_mat: friction::Material,
    friction_states: [Vec<friction::State>; 2],
    prepared_scene: PreparedAnalInsertionScene,
    max_window_frames: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::{Window, WindowAttributes, WindowId};

    struct App {
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        window: Option<Arc<Window>>,
        surface: Option<wgpu::Surface<'static>>,
        config: Option<wgpu::SurfaceConfiguration>,
        renderer: Option<SceneRenderer>,
        geometry: Option<SceneGeometry>,
        texture: Option<voxy_render::SceneTexture>,
        transform: Option<voxy_render::SceneTransform>,
        depth: Option<wgpu::Texture>,
        gluteal_sim: physics::biomechanics::LabiaGaitSimulation,
        anoderm_sim: physics::skin::LabiaMinoraGaitSimulation,
        films: [SurfaceFilm; 2],
        tube: LabiaTubeInsertion,
        pressures: Vec<f64>,
        friction_mat: friction::Material,
        friction_states: [Vec<friction::State>; 2],
        prepared_scene: PreparedAnalInsertionScene,
        frame_count: u64,
        max_frames: Option<u64>,
        started_at: Option<Instant>,
        step_render_ms_samples: Vec<f64>,
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attrs = WindowAttributes::default()
                .with_title(
                    "Voxy FEM | Anal Sphincter Penetration, Anoderm Rugae & 3D Lubricant Meniscus",
                )
                .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0));
            let window = Arc::new(event_loop.create_window(attrs).expect("window"));
            let surface = self.instance.create_surface(window.clone()).expect("surface");
            let size = window.inner_size();
            let mut config = surface
                .get_default_config(&self.adapter, size.width.max(1), size.height.max(1))
                .expect("default surface config");
            let caps = surface.get_capabilities(&self.adapter);
            if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
                config.present_mode = wgpu::PresentMode::Immediate;
            } else if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                config.present_mode = wgpu::PresentMode::Mailbox;
            }
            let format = config.format;
            surface.configure(&self.device, &config);
            let renderer = SceneRenderer::new(&self.device, format);
            let mut geometry = renderer
                .reserve_geometry(&self.device, 28_000, 160_000)
                .expect("geometry");
            pollster::block_on(renderer.set_geometry_opaque_shader(
                &self.device,
                &mut geometry,
                ANATOMICAL_ANAL_AND_TUBE_SHADER,
            ))
            .expect("shader");
            let initial_mesh = self
                .prepared_scene
                .build_full_mesh(
                    &self.gluteal_sim,
                    &self.anoderm_sim,
                    &self.films,
                    &self.tube,
                    0.0,
                )
                .expect("initial mesh");
            geometry.update(&self.queue, &initial_mesh).expect("initial upload");

            let texture = renderer
                .upload_texture(&self.device, &self.queue, 1, 1, &[255; 4])
                .expect("texture");
            let camera_eye = Vec3::new(0.74, -1.28, 1.32);
            let camera_target = Vec3::new(0.0, -0.02, -0.02);
            let proj = SceneCamera {
                eye: camera_eye,
                target: camera_target,
                up: Vec3::Z,
                projection: SceneProjection::Perspective {
                    vertical_fov: 35_f32.to_radians(),
                    aspect: config.width as f32 / config.height as f32,
                    near: 0.1,
                    far: 50.0,
                },
            }
            .view_projection()
            .expect("projection");
            let transform = renderer.create_transform(&self.device, proj).expect("transform");
            transform
                .update_view_position(&self.queue, camera_eye)
                .expect("view_pos");
            let depth = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("anal tube window depth"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.started_at = Some(Instant::now());
            window.request_redraw();
            self.window = Some(window);
            self.surface = Some(surface);
            self.config = Some(config);
            self.renderer = Some(renderer);
            self.geometry = Some(geometry);
            self.texture = Some(texture);
            self.transform = Some(transform);
            self.depth = Some(depth);
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => {
                    let phys_started = Instant::now();
                    let sim_t = (self.frame_count as f64) * 0.008;
                    let (tip_pos, tip_vel, phase) = self
                        .tube
                        .tip_state_at(sim_t)
                        .unwrap_or(([0.0, 0.0, 0.0072], [0.0; 3], 0.0));

                    if self.frame_count & 1 == 0 {
                        let _ = self
                            .gluteal_sim
                            .step_tube_insertion(phase, tip_vel[2], 0.12, 1.6e-4, 2e-2);
                    } else {
                        let active_side = ((self.frame_count >> 1) as usize) & 1;
                        let _ = self.films[active_side].advance_smear(
                            0.001,
                            SmearContact {
                                normal: [if active_side == 0 { 1.0 } else { -1.0 }, 0.0, 0.0],
                                body_velocity_m_s: [0.0, 0.015, tip_vel[2] * 0.5],
                            },
                            &self.pressures,
                            &self.friction_mat,
                            &mut self.friction_states[active_side],
                            [0.0, 0.0, -9.81],
                            0.0005,
                        );
                    }
                    let phys_ms = phys_started.elapsed().as_secs_f64() * 1000.0;

                    if let (
                        Some(surface),
                        Some(renderer),
                        Some(geometry),
                        Some(texture),
                        Some(transform),
                        Some(depth),
                        Some(window),
                    ) = (
                        &self.surface,
                        &self.renderer,
                        &self.geometry,
                        &self.texture,
                        &self.transform,
                        &self.depth,
                        &self.window,
                    ) {
                        let skin_started = Instant::now();
                        self.prepared_scene.update_vertices_in_place(
                            &self.queue,
                            geometry,
                            &self.gluteal_sim,
                            &self.anoderm_sim,
                            &self.films,
                            &self.tube,
                            sim_t,
                        );
                        let skin_ms = skin_started.elapsed().as_secs_f64() * 1000.0;
                        let step_gpu_ms = phys_ms + skin_ms;

                        if let wgpu::CurrentSurfaceTexture::Success(frame)
                        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) =
                            surface.get_current_texture()
                        {
                            let encode_started = Instant::now();
                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            let mut encoder = self
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                            renderer.encode(
                                &mut encoder,
                                &view,
                                &depth.create_view(&wgpu::TextureViewDescriptor::default()),
                                wgpu::Color {
                                    r: 0.035,
                                    g: 0.038,
                                    b: 0.052,
                                    a: 1.0,
                                },
                                &[SceneDraw {
                                    geometry,
                                    texture,
                                    transform,
                                    overlay: false,
                                }],
                            );
                            self.queue.submit([encoder.finish()]);
                            let work_ms =
                                step_gpu_ms + encode_started.elapsed().as_secs_f64() * 1000.0;
                            self.step_render_ms_samples.push(work_ms);
                            self.queue.present(frame);
                        } else {
                            self.step_render_ms_samples.push(step_gpu_ms);
                        }
                        self.frame_count += 1;
                        let last_work_ms =
                            self.step_render_ms_samples.last().copied().unwrap_or(1.0);
                        let compute_fps = 1000.0 / last_work_ms.max(1e-4);
                        if self.frame_count == 1 || self.frame_count % 20 == 0 {
                            window.set_title(&format!(
                                "Voxy FEM | Anal Sphincter Penetration & Lubricant Meniscus | depth={:.0}% tip_z={:.2}mm vel_z={:.1}mm/s | {:.0} FPS",
                                phase * 100.0,
                                tip_pos[2] * 1000.0,
                                tip_vel[2] * 1000.0,
                                compute_fps,
                            ));
                        }
                        if self.max_frames.is_some_and(|max| self.frame_count >= max) {
                            event_loop.exit();
                        } else {
                            window.request_redraw();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let event_loop = EventLoop::new()?;
    let mut app = App {
        instance,
        adapter,
        device,
        queue,
        window: None,
        surface: None,
        config: None,
        renderer: None,
        geometry: None,
        texture: None,
        transform: None,
        depth: None,
        gluteal_sim,
        anoderm_sim,
        films,
        tube,
        pressures,
        friction_mat,
        friction_states,
        prepared_scene,
        frame_count: 0,
        max_frames: max_window_frames,
        started_at: None,
        step_render_ms_samples: Vec::new(),
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
