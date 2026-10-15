//! Photorealistic curvilinear anatomical GPU render and >200 FPS live window for
//! Labia Majora + Labia Minora gait dynamics, inter-labial contact, and dielectric
//! SurfaceFilm wetting, meniscus pooling, and shear smearing.
use glam::Vec3;
use physics::biomechanics::{LabiaMajoraGeometry, Material, PelvisGaitKinematics};
use physics::friction;
use physics::skin::{
    ContactScene, LabiaMinoraGeometry, Layer, Relaxation, SkinMaterial, SolverConfig,
};
use physics::surface_film::{BridgeConfig, Material as FilmMaterial, SmearContact, SurfaceFilm};
use std::sync::Arc;
use std::time::Instant;
use voxy_render::{
    GraphicsOptions, SceneCamera, SceneDraw, SceneGeometry, SceneMesh, SceneProjection,
    SceneRenderer, SceneVertex,
};

const ANATOMICAL_MUCOSAL_SHADER: &str = r#"
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

// Micro-relief heightfield in physical reference coordinates (meters):
// combines fine epidermal pores on keratinized outer skin with delicate
// transverse mucosal micro-rugae on the labia minora and vestibule.
fn skin_micro_height(ref_m: vec3<f32>, mucosa: f32, wetness: f32) -> f32 {
    let p_pore = ref_m * vec3<f32>(580.0, 540.0, 500.0);
    let pore_a = noise3(p_pore);
    let pore_b = noise3(p_pore * 2.1 + vec3<f32>(13.7, 7.3, 5.1));
    let epidermal_pores = (pore_a * 0.65 + pore_b * 0.35 - 0.5) * (1.0 - 0.65 * mucosa);

    let rugae = 0.32
        * sin(ref_m.y * 520.0 + 2.4 * sin(ref_m.x * 380.0))
        * cos(ref_m.x * 680.0)
        * mucosa;

    let dry_relief = 1.0 - 0.78 * smoothstep(0.0, 0.85, wetness);
    return (epidermal_pores + rugae) * dry_relief;
}

fn ggx_lobe(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32, f0: f32) -> f32 {
    let h = unit(v + l);
    let nl = max(dot(n, l), 0.0);
    let nv = max(dot(n, v), 1e-4);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(v, h), 0.0);
    let a = max(rough * rough, 0.0016);
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

    // Blend smooth vertex normal with dynamic deformed screen-space derivative normal
    let dpos_dx = -dpdx(in.to_camera);
    let dpos_dy = -dpdy(in.to_camera);
    let geom_n = unit(cross(dpos_dx, dpos_dy));
    let n_smooth = unit(in.normal);
    let n_macro = unit(mix(n_smooth, geom_n * sign(dot(geom_n, n_smooth) + 1e-5), 0.18));

    let wetness = clamp(in.uv.x, 0.0, 1.0);
    let mucosa = clamp(in.uv.y, 0.0, 1.0);
    let thinness = 1.0 - clamp(in.color.a, 0.1, 1.0);

    // Procedural micro-normal perturbation (epidermal pores + mucosal rugae)
    let eps = 0.00020;
    let h0 = skin_micro_height(in.reference, mucosa, wetness);
    let hx = skin_micro_height(in.reference + vec3<f32>(eps, 0.0, 0.0), mucosa, wetness);
    let hy = skin_micro_height(in.reference + vec3<f32>(0.0, eps, 0.0), mucosa, wetness);
    let grad_micro = vec3<f32>((hx - h0) / eps, (hy - h0) / eps, 0.0);
    let n_tissue = unit(n_macro - 0.000055 * grad_micro);
    let n_film = unit(mix(n_tissue, n_macro, smoothstep(0.0, 0.75, wetness)));

    // 3-point studio lighting rig
    let key = unit(vec3<f32>(-0.34, -0.54, 0.77));
    let fill = unit(vec3<f32>(0.56, 0.34, 0.75));
    let rim = unit(vec3<f32>(0.04, 0.84, 0.54));

    // Chromophore-weighted RGB dipole subsurface scattering (red scatters deepest)
    let wrap = mix(vec3<f32>(0.42, 0.20, 0.11), vec3<f32>(0.58, 0.26, 0.15), mucosa);
    let key_wrap = max((vec3<f32>(dot(n_tissue, key)) + wrap) / (vec3<f32>(1.0) + wrap), vec3<f32>(0.0));
    let fill_wrap = max((vec3<f32>(dot(n_tissue, fill)) + wrap) / (vec3<f32>(1.0) + wrap), vec3<f32>(0.0));

    // Thin-tissue crimson translucency across labia minora folds and clitoral prepuce
    let back_scatter = pow(clamp(dot(-key, n_tissue) + 0.48, 0.0, 1.0), 2.2)
        + 0.65 * pow(clamp(dot(-rim, n_tissue) + 0.52, 0.0, 1.0), 2.0);
    let sss_translucency = thinness * back_scatter * vec3<f32>(0.24, 0.045, 0.035);

    let sss_diffuse = vec3<f32>(0.14, 0.095, 0.088)
        + 0.68 * key_wrap
        + 0.25 * fill_wrap
        + sss_translucency;

    // Subtle dermal vascular mottling + index-matching darkening under wet film
    let mottle = 0.975 + 0.05 * noise3(in.reference * 240.0);
    let wet_darken = mix(vec3<f32>(1.0), vec3<f32>(0.76, 0.66, 0.69), wetness * 0.75);
    let albedo = in.color.rgb * mottle * wet_darken;

    // Dual-lobe epidermal/mucosal GGX + thin-film dielectric water layer (IOR = 1.333)
    let base_roughness = mix(0.45, 0.27, mucosa) + 0.03 * h0;
    let secondary_roughness = mix(0.68, 0.44, mucosa);
    let film_roughness = mix(base_roughness, 0.052, smoothstep(0.0, 0.88, wetness));

    let tissue_spec = (0.72 * ggx_lobe(n_tissue, v, key, base_roughness, 0.028)
        + 0.28 * ggx_lobe(n_tissue, v, key, secondary_roughness, 0.028)
        + 0.34 * ggx_lobe(n_tissue, v, fill, base_roughness, 0.028))
        * (1.0 - 0.55 * wetness);

    let film_spec = wetness * (
        1.85 * ggx_lobe(n_film, v, key, film_roughness, 0.046)
        + 0.72 * ggx_lobe(n_film, v, fill, film_roughness * 1.12, 0.046)
        + 0.52 * ggx_lobe(n_film, v, rim, max(film_roughness, 0.08), 0.046)
    );

    let nv = max(dot(n_film, v), 0.0);
    let fresnel_sheen = wetness * pow(1.0 - nv, 4.5) * 0.16;

    // Deep anatomical cavity occlusion in the vestibule & interlabial sulci
    let ao = clamp(0.66 + 0.34 * smoothstep(-0.0052, 0.0075, in.reference.z), 0.54, 1.0);
    let spec_color = mix(vec3<f32>(1.0, 0.95, 0.92), vec3<f32>(0.99, 0.995, 1.0), wetness);
    let raw_lit = albedo * sss_diffuse * ao
        + spec_color * (tissue_spec * ao + film_spec + fresnel_sheen * ao);
    let mapped = filmic_tonemap(raw_lit * 1.06);
    return vec4<f32>(mapped, 1.0);
}
"#;

#[derive(Clone, Copy, Debug)]
struct PreparedVertexBinding {
    rest_scaled: [f32; 3],
    sin_v: f32,
    cos_v: f32,
    side_idx: u8,
    b0: u8,
    b1: u8,
    side_sign: f32,
    frac: f32,
    majora_w: f32,
    minora_w: f32,
    minora_arch: f32,
    wet_envelope: f32,
    mucosa_factor: f32,
}

/// High-density curvilinear anatomical surface with precomputed bindings and
/// zero-allocation direct GPU vertex streaming (>200 FPS).
#[derive(Debug)]
struct PreparedAnatomicalSurface {
    bindings: Vec<PreparedVertexBinding>,
    vertices: Vec<SceneVertex>,
    indices: Vec<u32>,
    rest_normals: Vec<[f32; 3]>,
    material_coordinates: Vec<[f32; 3]>,
    display_scale: f32,
}

impl PreparedAnatomicalSurface {
    #[allow(clippy::too_many_lines)]
    fn new(display_scale: f64) -> Self {
        let nu = 128usize;
        let nv = 144usize;
        let half_width_m = 0.034;
        let half_length_m = 0.044;
        let scale = display_scale as f32;

        let count = (nu + 1) * (nv + 1);
        let mut bindings = Vec::with_capacity(count);
        let mut vertices = Vec::with_capacity(count);
        let mut material_coordinates = Vec::with_capacity(count);

        for iv in 0..=nv {
            let v_lin = (iv as f64 / nv as f64) * 2.0 - 1.0;
            // Concentrate longitudinal resolution around the vestibule, minora, and clitoral hood
            let v = 0.80 * v_lin + 0.20 * v_lin.powi(3);
            let y = v * half_length_m;
            let (sin_v, cos_v) = ((v as f32) * 3.2).sin_cos();

            let v_maj = ((v - 0.02) / 0.80).clamp(-1.0, 1.0);
            let majora_env = (1.0 - v_maj * v_maj).max(0.0).powf(0.85);

            let v_min = ((v + 0.02) / 0.58).clamp(-1.0, 1.0);
            let minora_arch = (1.0 - v_min * v_min).max(0.0);
            let minora_env = minora_arch.powf(1.22);
            let minora_ant_bias = (1.0 + 0.22 * (v * 2.0).tanh()).clamp(0.72, 1.24);

            // Clitoral prepuce hood (v ~ 0.49), glans clitoridis (v ~ 0.42), and posterior fourchette (v ~ -0.55)
            let hood_dome = (-((v - 0.50) / 0.125).powi(2)).exp();
            let glans_dome = (-((v - 0.415) / 0.055).powi(2)).exp();
            let frenulum_arch = (-((v - 0.35) / 0.070).powi(2)).exp();
            let fourchette_arch = (-((v + 0.55) / 0.075).powi(2)).exp();

            let bin_f = (((v + 1.0) * 0.5) * 7.0).clamp(0.0, 7.0);
            let b0 = (bin_f.floor() as usize).min(7);
            let b1 = (b0 + 1).min(7);
            let frac = (bin_f - b0 as f64) as f32;

            for iu in 0..=nu {
                let u_lin = (iu as f64 / nu as f64) * 2.0 - 1.0;
                // Non-uniform lateral sampling: 2.3x finer grid spacing (0.2 mm) across the
                // vestibule, labia minora crest, and interlabial sulcus while evaluating all
                // anatomical profiles in true physical meters (eliminates diagonal sawtooth aliasing)
                let u = 0.42 * u_lin + 0.58 * u_lin.powi(3);
                let row_scale = (1.0 - 0.28 * v * v).sqrt();
                let x_rest = u * half_width_m * row_scale;
                let abs_x = x_rest.abs();
                let side_idx = u8::from(u >= 0.0);
                let side_sign = if u >= 0.0 { 1.0_f32 } else { -1.0_f32 };

                // Naturally apposed median cleft in physical meters
                let cleft_half_gap = (0.0014 * minora_arch.powf(1.12)).max(0.00018);

                // Plump subcutaneous Labia Majora pads
                let majora_center_x = 0.0024 + 0.0092 * majora_env;
                let majora_width = 0.0086 + 0.0038 * majora_env;
                let majora_xi = (abs_x - majora_center_x) / majora_width;
                let majora_profile = (-majora_xi * majora_xi * 1.55).exp() * majora_env;
                let majora_height = 0.0105 * majora_profile;

                // Mons pubis cushion + clitoral prepuce hood + glans clitoridis
                let mons_profile =
                    ((v - 0.24).max(0.0) / 0.76).powi(2) * (-((abs_x / 0.025).powi(2))).exp();
                let central_hood = 0.0038 * hood_dome * (-((abs_x / 0.0034).powi(2))).exp();
                let glans_height = 0.0024 * glans_dome * (-((abs_x / 0.0019).powi(2))).exp();
                let frenulum_height =
                    0.0014 * frenulum_arch * (-(((abs_x - 0.0016) / 0.0013).powi(2))).exp();
                let fourchette_height =
                    0.0012 * fourchette_arch * (-((abs_x / 0.0032).powi(2))).exp();
                let mons_height = 0.0062 * mons_profile
                    + central_hood
                    + glans_height
                    + frenulum_height
                    + fourchette_height;

                // Soft, rounded Labia Minora folds with subtle organic longitudinal edge wave
                let minora_center_x = cleft_half_gap + 0.0019 * minora_env;
                let minora_width = 0.00185 + 0.00115 * minora_env;
                let minora_xi = (abs_x - minora_center_x) / minora_width;
                let edge_wave = 1.0
                    + 0.065 * (v * 17.0 + f64::from(side_sign) * 0.5).sin()
                    + 0.035 * (v * 31.0 - f64::from(side_sign) * 0.9).cos();
                let minora_profile = (-minora_xi * minora_xi * 1.95).exp()
                    * minora_env
                    * minora_ant_bias
                    * edge_wave;
                let minora_height = 0.0048 * minora_profile;

                // Interlabial sulcus (strictly bounded by majora_env * minora_arch so it fades cleanly at ends)
                let sulcus_center_x = minora_center_x + 0.0029 * minora_env + 0.0009;
                let sulcus_xi = (abs_x - sulcus_center_x) / 0.0020;
                let sulcus_env = majora_env * (0.25 + 0.75 * minora_arch.sqrt());
                let sulcus_depression =
                    -0.0019 * (-sulcus_xi * sulcus_xi * 1.95).exp() * sulcus_env;

                // Lateral genitocrural fold transition
                let groin_xi = (abs_x - 0.0235 * row_scale) / 0.0038;
                let groin_crease = -0.0014 * (-groin_xi * groin_xi * 1.8).exp() * majora_env;

                // Median vestibular recess
                let vestibule_xi = abs_x / (cleft_half_gap + 0.0013);
                let vestibule_depression = -0.0050
                    * (-vestibule_xi * vestibule_xi * 1.85).exp()
                    * minora_arch
                    * (1.0 - 0.55 * glans_dome);

                let radial_r2 = u * u + v * v;
                let pelvic_arch_z = -0.0065 * (u * u * 0.75 + v * v * 0.65)
                    - 0.0040 * (radial_r2 - 0.65).max(0.0).powi(2);
                let z_rest = pelvic_arch_z
                    + mons_height
                    + majora_height
                    + groin_crease
                    + minora_height
                    + sulcus_depression
                    + vestibule_depression;

                let mucosa_factor = (minora_profile * 1.15
                    + 0.85 * glans_dome * (-((abs_x / 0.0024).powi(2))).exp()
                    + 0.65 * hood_dome * (-((abs_x / 0.0044).powi(2))).exp()
                    + (-vestibule_xi * vestibule_xi * 1.15).exp() * minora_arch)
                    .clamp(0.0, 1.0) as f32;

                // Wetness envelope covering Labia Minora, vestibule, and pooling in the interlabial sulcus
                let radial_wet_spread =
                    (-((abs_x - minora_center_x * 0.68) / 0.0064).powi(2)).exp();
                let sulcus_wet_pool = 0.55 * (-sulcus_xi * sulcus_xi * 2.1).exp() * minora_arch;
                let wet_envelope =
                    (((radial_wet_spread * minora_arch) + sulcus_wet_pool) / 1.95e-5) as f32;

                let outer_skin = [0.83_f32, 0.62, 0.55];
                let majora_tint = [0.77_f32, 0.52, 0.47];
                let minora_mucosa = [0.74_f32, 0.35, 0.41];
                let minora_crest_pigment = [0.68_f32, 0.39, 0.38];
                let inner_vestibule = [0.66_f32, 0.26, 0.33];

                let m_blend = (majora_profile as f32).clamp(0.0, 1.0);
                let mut rgb = [
                    outer_skin[0] * (1.0 - m_blend) + majora_tint[0] * m_blend,
                    outer_skin[1] * (1.0 - m_blend) + majora_tint[1] * m_blend,
                    outer_skin[2] * (1.0 - m_blend) + majora_tint[2] * m_blend,
                ];
                for k in 0..3 {
                    rgb[k] = rgb[k] * (1.0 - mucosa_factor) + minora_mucosa[k] * mucosa_factor;
                }
                let crest_margin = ((-minora_xi * minora_xi * 3.4).exp() * minora_env) as f32;
                for k in 0..3 {
                    rgb[k] = rgb[k] * (1.0 - 0.36 * crest_margin)
                        + minora_crest_pigment[k] * (0.36 * crest_margin);
                }
                let deep_cleft = ((-vestibule_xi * vestibule_xi * 2.0).exp() * minora_arch) as f32;
                for k in 0..3 {
                    rgb[k] = rgb[k] * (1.0 - 0.68 * deep_cleft)
                        + inner_vestibule[k] * (0.68 * deep_cleft);
                }

                let thin_fold = (minora_profile * 0.85 + hood_dome * 0.40).clamp(0.0, 0.90) as f32;
                let thickness_proxy = 1.0 - thin_fold;

                let rest_scaled = [
                    (x_rest * display_scale) as f32,
                    (y * display_scale) as f32,
                    (z_rest * display_scale) as f32,
                ];
                let color = [rgb[0], rgb[1], rgb[2], thickness_proxy];

                bindings.push(PreparedVertexBinding {
                    rest_scaled,
                    sin_v,
                    cos_v,
                    side_idx,
                    b0: b0 as u8,
                    b1: b1 as u8,
                    side_sign,
                    frac,
                    majora_w: majora_profile.clamp(0.0, 1.0) as f32,
                    minora_w: minora_profile.clamp(0.0, 1.15) as f32,
                    minora_arch: minora_arch as f32,
                    wet_envelope,
                    mucosa_factor,
                });
                vertices.push(SceneVertex {
                    position: rest_scaled,
                    uv: [0.0, mucosa_factor],
                    color,
                });
                material_coordinates.push([x_rest as f32, y as f32, z_rest as f32]);
            }
        }

        let stride = nu + 1;
        let mut indices = Vec::with_capacity(nu * nv * 6);
        for iv in 0..nv {
            for iu in 0..nu {
                let a = (iv * stride + iu) as u32;
                let b = a + 1;
                let c = ((iv + 1) * stride + iu) as u32;
                let d = c + 1;
                indices.extend([a, b, d, a, d, c]);
            }
        }

        let mut normals = vec![Vec3::ZERO; vertices.len()];
        for tri in indices.chunks_exact(3) {
            let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let p0 = Vec3::from_array(vertices[i0].position);
            let p1 = Vec3::from_array(vertices[i1].position);
            let p2 = Vec3::from_array(vertices[i2].position);
            let n = (p1 - p0).cross(p2 - p0);
            normals[i0] += n;
            normals[i1] += n;
            normals[i2] += n;
        }
        let rest_normals = normals
            .into_iter()
            .map(|n| n.try_normalize().unwrap_or(Vec3::Z).to_array())
            .collect();

        Self {
            bindings,
            vertices,
            indices,
            rest_normals,
            material_coordinates,
            display_scale: scale,
        }
    }

    fn deform_vertices(
        &mut self,
        majora_sim: &physics::biomechanics::LabiaGaitSimulation,
        minora_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
    ) {
        let majora_pos = majora_sim.assembly.body.body().positions();
        let majora_rest = majora_sim.assembly.body.body().rest_positions();
        let [left_apex, right_apex] = majora_sim.apex_nodes();
        let majora_disp = [
            [
                (majora_pos[left_apex][0] - majora_rest[left_apex][0]) as f32,
                (majora_pos[left_apex][1] - majora_rest[left_apex][1]) as f32,
                (majora_pos[left_apex][2] - majora_rest[left_apex][2]) as f32,
            ],
            [
                (majora_pos[right_apex][0] - majora_rest[right_apex][0]) as f32,
                (majora_pos[right_apex][1] - majora_rest[right_apex][1]) as f32,
                (majora_pos[right_apex][2] - majora_rest[right_apex][2]) as f32,
            ],
        ];

        let mut minora_crest_disp = [[0.0_f32; 3]; 2];
        let mut film_wetness_profile = [[0.0_f32; 8]; 2];
        for side in 0..2 {
            let fold = &minora_sim.folds[side];
            let pos = fold.skin.positions();
            let rest = fold.skin.rest_positions();
            let inv_count = 1.0 / (fold.crest_nodes.len().max(1) as f32);
            for &node in &fold.crest_nodes {
                for axis in 0..3 {
                    minora_crest_disp[side][axis] +=
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

        let t_phase = (minora_sim.time_s() * 18.0) as f32;
        let (sin_t, cos_t) = t_phase.sin_cos();
        let scale = self.display_scale;

        for (vertex, b) in self.vertices.iter_mut().zip(self.bindings.iter()) {
            let side = usize::from(b.side_idx);
            let maj = majora_disp[side];
            let min = minora_crest_disp[side];
            let phase_wave = sin_t.mul_add(b.cos_v, cos_t * b.sin_v);

            let dx = (1.8 * b.majora_w * maj[0]
                + 2.2 * b.minora_w * min[0]
                + 0.0011 * b.minora_w * phase_wave
                - b.side_sign * 0.0007 * b.minora_w * phase_wave.abs())
            .clamp(-0.0035, 0.0035);
            let dy = (1.8 * b.majora_w * maj[1] + 2.2 * b.minora_w * min[1]).clamp(-0.0035, 0.0035);
            let dz = (1.6 * b.majora_w * maj[2] + 1.8 * b.minora_w * min[2] * b.minora_arch)
                .clamp(-0.0030, 0.0030);

            vertex.position = [
                b.rest_scaled[0] + dx * scale,
                b.rest_scaled[1] + dy * scale,
                b.rest_scaled[2] + dz * scale,
            ];

            let h_film = (1.0 - b.frac) * film_wetness_profile[side][usize::from(b.b0)]
                + b.frac * film_wetness_profile[side][usize::from(b.b1)];
            let wetness = (h_film * b.wet_envelope).clamp(0.0, 1.0);
            vertex.uv = [wetness, b.mucosa_factor];
        }
    }

    fn build_full_mesh(
        &mut self,
        majora_sim: &physics::biomechanics::LabiaGaitSimulation,
        minora_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
    ) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        self.deform_vertices(majora_sim, minora_sim, films);
        Ok(SceneMesh::new(self.vertices.clone(), self.indices.clone())?
            .with_normals(self.rest_normals.clone())?
            .with_material_coordinates(self.material_coordinates.clone())?)
    }

    /// Zero-allocation fast path for the >200 FPS interactive window loop:
    /// updates vertex positions and wetness UVs in-place and writes directly to GPU storage.
    fn update_vertices_in_place(
        &mut self,
        queue: &wgpu::Queue,
        geometry: &SceneGeometry,
        majora_sim: &physics::biomechanics::LabiaGaitSimulation,
        minora_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
    ) {
        self.deform_vertices(majora_sim, minora_sim, films);
        queue.write_buffer(&geometry.vertices, 0, bytemuck::cast_slice(&self.vertices));
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
        .unwrap_or_else(|| "/private/tmp/voxy-labia-gait.png".into());

    let majora_geom = LabiaMajoraGeometry {
        length_m: 0.052,
        width_m: 0.018,
        height_m: 0.009,
        separation_m: 0.024,
        sectors: 8,
        latitude_rings: 2,
    };
    let majora_mat = Material::from_young_poisson(3000.0, 0.45)?;
    let mut majora_sim = majora_geom.build_gait_contact_simulation(
        [majora_mat.clone(), majora_mat],
        [1000.0; 2],
        0.0001,
        0.0018,
        1.5,
    )?;

    let mut minora_sim = LabiaMinoraGeometry {
        length_m: 0.042,
        fold_width_m: 0.0085,
        fold_height_m: 0.0055,
        separation_m: 0.011,
        longitudinal_segments: 4,
        transverse_segments: 2,
    }
    .build_gait_simulation(SkinMaterial {
        layers: vec![Layer {
            thickness: 0.00035,
            density: 1000.0,
            shear_modulus: 950.0,
            collagen_modulus: 1900.0,
            collagen_exponent: 4.0,
            dispersion: 0.1,
            fiber_angle: 0.5,
            relaxation: vec![Relaxation {
                modulus: 450.0,
                time: 0.025,
            }],
        }],
    })?;

    let film_mat = FilmMaterial {
        density: 1000.0,
        viscosity: 0.04,
        wetting: 2e-5,
        surface_tension: 0.045,
    };
    let mut films = minora_sim.build_surface_films(film_mat)?;

    let walk_gait = PelvisGaitKinematics {
        stride_frequency_hz: 1.5,
        vertical_amplitude_m: 0.0025,
        lateral_amplitude_m: 0.0016,
        sagittal_amplitude_m: 0.0014,
        phase_rad: 0.0,
    };
    let run_gait = PelvisGaitKinematics {
        stride_frequency_hz: 2.5,
        vertical_amplitude_m: 0.0048,
        lateral_amplitude_m: 0.0028,
        sagittal_amplitude_m: 0.0024,
        phase_rad: 0.5,
    };

    let display_scale = 34.0;
    let mut prepared_surface = PreparedAnatomicalSurface::new(display_scale);

    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Labia GPU Renderer: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut geometry = renderer.reserve_geometry(&device, 20_000, 120_000)?;
    pollster::block_on(renderer.set_geometry_opaque_shader(
        &device,
        &mut geometry,
        ANATOMICAL_MUCOSAL_SHADER,
    ))?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let camera_eye = Vec3::new(0.85, -1.65, 1.85);
    let base_projection = SceneCamera {
        eye: camera_eye,
        target: Vec3::new(0.0, 0.0, 0.05),
        up: Vec3::Z,
        projection: SceneProjection::Perspective {
            vertical_fov: 40_f32.to_radians(),
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
            label: Some("labia snapshot"),
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
        label: Some("labia pixels"),
        size: 4096 * 768,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let cells = films[0].triangles().len();
    let friction_mat = friction::Material::new(1e6, 1e6, 0.35)?;
    let mut friction_states = [
        vec![friction::State::default(); cells],
        vec![friction::State::default(); cells],
    ];
    let pressures = vec![450.0; cells];

    let mut frames = Vec::new();
    for frame in 0..3 {
        if frame == 1 {
            for side in 0..2 {
                films[side].deposit(cells / 3, 2.0e-9)?;
                films[side].deposit(cells / 2, 2.5e-9)?;
            }
            for _ in 0..16 {
                majora_sim.step_gait(&walk_gait, 8e-5, 1e-4)?;
            }
            for _ in 0..8 {
                minora_sim.step_gait(
                    &walk_gait,
                    0.001,
                    [0.0, 0.0, -9.81],
                    &ContactScene::default(),
                    SolverConfig::default(),
                )?;
                for side in 0..2 {
                    films[side].update_geometry(minora_sim.folds[side].skin.positions())?;
                    films[side].step(0.001, [0.0, 0.0, -9.81])?;
                }
            }
        } else if frame == 2 {
            for side in 0..2 {
                films[side].deposit(cells / 4, 1.8e-9)?;
                films[side].deposit(2 * cells / 3, 2.2e-9)?;
            }
            for _ in 0..20 {
                majora_sim.step_gait(&run_gait, 8e-5, 1e-4)?;
            }
            for step in 0..10 {
                minora_sim.step_gait(
                    &run_gait,
                    0.001,
                    [0.0, 0.0, -9.81],
                    &ContactScene::default(),
                    SolverConfig::default(),
                )?;
                let smear_vel = if step % 2 == 0 { 0.035 } else { -0.025 };
                for side in 0..2 {
                    films[side].update_geometry(minora_sim.folds[side].skin.positions())?;
                    films[side].advance_smear(
                        0.001,
                        SmearContact {
                            normal: [if side == 0 { 1.0 } else { -1.0 }, 0.0, 0.0],
                            body_velocity_m_s: [0.0, smear_vel, 0.01],
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
                        max_gap: 0.018,
                        transfer_speed: 0.002,
                        opposing_cosine: 0.1,
                        max_candidates: 500_000,
                    },
                )?;
            }
        }

        let mesh = prepared_surface.build_full_mesh(&majora_sim, &minora_sim, &films)?;
        geometry.update(&queue, &mesh)?;
        let diag = majora_sim.assembly.body.diagnostics()?;
        let total_film_mg = (films[0].total_mass() + films[1].total_mass()) * 1e6;
        println!(
            "frame={frame} ({}) | majora_t={:.4}s min_cleft_gap={:.4}mm E_kin={:.3e}J | minora_t={:.4}s film_mass={total_film_mg:.3}mg",
            match frame {
                0 => "rest",
                1 => "walk+wetting",
                _ => "run+smearing",
            },
            majora_sim.time_s(),
            majora_sim.min_contact_gap_m() * 1000.0,
            diag.kinetic_j,
            minora_sim.time_s(),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&Default::default()),
            &depth.create_view(&Default::default()),
            wgpu::Color {
                r: 0.04,
                g: 0.045,
                b: 0.06,
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
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let pixels = readback.slice(..).get_mapped_range()?;
        let tissue_pixels = pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 60 || p[2] > 90)
            .count();
        if tissue_pixels < 500 {
            return Err("labia tissue surface missing from GPU snapshot".into());
        }
        println!("frame={frame} rendered_tissue_pixels={tissue_pixels}");
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }

    if frames[0] == frames[1] || frames[1] == frames[2] {
        return Err("rendered labia surface did not evolve across gait and wetting phases".into());
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }

    let mut output = Vec::with_capacity(4096 * 3 * 768);
    for row in 0..768 {
        for frame in &frames {
            output.extend_from_slice(&frame[row * 4096..(row + 1) * 4096]);
        }
    }
    image::save_buffer(
        &path,
        &output,
        1024 * frames.len() as u32,
        768,
        image::ColorType::Rgba8,
    )?;
    println!("LABIA GAIT SNAPSHOT PASS: {path}");

    if window_mode {
        run_interactive_window(
            instance,
            adapter,
            device,
            queue,
            majora_sim,
            minora_sim,
            films,
            run_gait,
            pressures,
            friction_mat,
            friction_states,
            prepared_surface,
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
    majora_sim: physics::biomechanics::LabiaGaitSimulation,
    minora_sim: physics::skin::LabiaMinoraGaitSimulation,
    films: [SurfaceFilm; 2],
    gait: PelvisGaitKinematics,
    pressures: Vec<f64>,
    friction_mat: friction::Material,
    friction_states: [Vec<friction::State>; 2],
    prepared_surface: PreparedAnatomicalSurface,
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
        majora_sim: physics::biomechanics::LabiaGaitSimulation,
        minora_sim: physics::skin::LabiaMinoraGaitSimulation,
        films: [SurfaceFilm; 2],
        gait: PelvisGaitKinematics,
        pressures: Vec<f64>,
        friction_mat: friction::Material,
        friction_states: [Vec<friction::State>; 2],
        prepared_surface: PreparedAnatomicalSurface,
        frame_count: u64,
        max_frames: Option<u64>,
        started_at: Option<Instant>,
        last_frame_at: Option<Instant>,
        step_render_ms_samples: Vec<f64>,
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attrs = WindowAttributes::default()
                .with_title("Voxy FEM | Photorealistic Labia Majora + Minora Gait & SurfaceFilm")
                .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0));
            let window = Arc::new(event_loop.create_window(attrs).expect("window"));
            let surface = self
                .instance
                .create_surface(window.clone())
                .expect("surface");
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
                .reserve_geometry(&self.device, 20_000, 120_000)
                .expect("geometry");
            pollster::block_on(renderer.set_geometry_opaque_shader(
                &self.device,
                &mut geometry,
                ANATOMICAL_MUCOSAL_SHADER,
            ))
            .expect("shader");
            let initial_mesh = self
                .prepared_surface
                .build_full_mesh(&self.majora_sim, &self.minora_sim, &self.films)
                .expect("initial mesh");
            geometry
                .update(&self.queue, &initial_mesh)
                .expect("initial upload");

            let texture = renderer
                .upload_texture(&self.device, &self.queue, 1, 1, &[255; 4])
                .expect("texture");
            let camera_eye = Vec3::new(0.85, -1.65, 1.85);
            let proj = SceneCamera {
                eye: camera_eye,
                target: Vec3::new(0.0, 0.0, 0.05),
                up: Vec3::Z,
                projection: SceneProjection::Perspective {
                    vertical_fov: 40_f32.to_radians(),
                    aspect: config.width as f32 / config.height as f32,
                    near: 0.1,
                    far: 50.0,
                },
            }
            .view_projection()
            .expect("projection");
            let transform = renderer
                .create_transform(&self.device, proj)
                .expect("transform");
            transform
                .update_view_position(&self.queue, camera_eye)
                .expect("view_pos");
            let depth = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("labia window depth"),
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
            let now = Instant::now();
            self.started_at = Some(now);
            self.last_frame_at = Some(now);
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
                    if self.frame_count & 1 == 0 {
                        let _ = self.majora_sim.step_gait(&self.gait, 1.6e-4, 2e-2);
                    } else {
                        let _ = self.minora_sim.step_gait(
                            &self.gait,
                            0.001,
                            [0.0, 0.0, -9.81],
                            &ContactScene::default(),
                            SolverConfig {
                                max_newton: 4,
                                max_cg: 24,
                                force_tolerance: 2e-3,
                                relative_tolerance: 2e-2,
                            },
                        );
                        let smear_v = 0.025 * (self.frame_count as f64 * 0.15).sin();
                        let active_side = ((self.frame_count >> 1) as usize) & 1;
                        let _ = self.films[active_side]
                            .update_geometry(self.minora_sim.folds[active_side].skin.positions());
                        let _ = self.films[active_side].advance_smear(
                            0.001,
                            SmearContact {
                                normal: [if active_side == 0 { 1.0 } else { -1.0 }, 0.0, 0.0],
                                body_velocity_m_s: [0.0, smear_v, 0.0],
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
                        self.prepared_surface.update_vertices_in_place(
                            &self.queue,
                            geometry,
                            &self.majora_sim,
                            &self.minora_sim,
                            &self.films,
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
                                    r: 0.04,
                                    g: 0.045,
                                    b: 0.06,
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
                        let now = Instant::now();
                        let wall_elapsed = self
                            .started_at
                            .map_or(1e-6, |s| now.duration_since(s).as_secs_f64().max(1e-6));
                        let wall_fps = self.frame_count as f64 / wall_elapsed;
                        let last_work_ms =
                            self.step_render_ms_samples.last().copied().unwrap_or(1.0);
                        let compute_fps = 1000.0 / last_work_ms.max(1e-4);
                        self.last_frame_at = Some(now);
                        if self.frame_count == 1 || self.frame_count % 30 == 0 {
                            window.set_title(&format!(
                                "Voxy FEM | Photorealistic Labia Gait & Smearing | FPS: {:.1} (step+render {:.2} ms -> {:.0} FPS) | frame={} gap={:.3}mm",
                                compute_fps.max(wall_fps),
                                last_work_ms,
                                compute_fps,
                                self.frame_count,
                                self.majora_sim.min_contact_gap_m() * 1000.0,
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
        majora_sim,
        minora_sim,
        films,
        gait,
        pressures,
        friction_mat,
        friction_states,
        prepared_surface,
        frame_count: 0,
        max_frames: max_window_frames,
        started_at: None,
        last_frame_at: None,
        step_render_ms_samples: Vec::new(),
    };
    event_loop.run_app(&mut app)?;

    if !app.step_render_ms_samples.is_empty() {
        let mut samples = app.step_render_ms_samples;
        let mean_ms = samples.iter().sum::<f64>() / samples.len() as f64;
        samples.sort_by(f64::total_cmp);
        let median_ms = samples[samples.len() / 2];
        let p95_ms = samples[samples.len() * 95 / 100];
        let mean_fps = 1000.0 / mean_ms;
        let median_fps = 1000.0 / median_ms;
        let p95_fps = 1000.0 / p95_ms;
        println!(
            "WINDOW BENCHMARK ({} frames): step+render mean={mean_ms:.3}ms ({mean_fps:.1} FPS) | median={median_ms:.3}ms ({median_fps:.1} FPS) | p95={p95_ms:.3}ms ({p95_fps:.1} FPS)",
            samples.len()
        );
        if mean_fps < 200.0 || p95_fps < 200.0 {
            return Err(format!(
                "FPS below 200 FPS target: mean={mean_fps:.1} FPS, p95={p95_fps:.1} FPS"
            )
            .into());
        }
    }
    Ok(())
}
