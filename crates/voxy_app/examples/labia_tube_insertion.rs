//! Photorealistic 3D GPU animation and >200 FPS live window for
//! Rounded-Tip Cylindrical Tube Insertion into Labia Minora & Majora,
//! with positive-gap contact parting, mucosal cuff invagination/eversion,
//! and SurfaceFilm lubricant meniscus smearing.
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

const ANATOMICAL_MUCOSAL_AND_TUBE_SHADER: &str = r"
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
    let dpos_dx = -dpdx(in.to_camera);
    let dpos_dy = -dpdy(in.to_camera);
    let geom_n = unit(cross(dpos_dx, dpos_dy));
    let n_smooth = unit(in.normal);

    let key = unit(vec3<f32>(-0.34, -0.54, 0.77));
    let fill = unit(vec3<f32>(0.56, 0.34, 0.75));
    let rim = unit(vec3<f32>(0.04, 0.84, 0.54));

    // Smooth medical-grade silicone/glass tube branch (tagged by uv.y < -0.5)
    if (in.uv.y < -0.5) {
        let n_tube = unit(mix(n_smooth, geom_n * sign(dot(geom_n, n_smooth) + 1e-5), 0.25));
        let tube_wet = clamp(in.uv.x, 0.0, 1.0);
        let rough = mix(0.11, 0.035, tube_wet);
        let spec = 2.1 * ggx_lobe(n_tube, v, key, rough, 0.048)
            + 0.85 * ggx_lobe(n_tube, v, fill, rough * 1.15, 0.048)
            + 0.65 * ggx_lobe(n_tube, v, rim, 0.07, 0.048);
        let nv = max(dot(n_tube, v), 0.0);
        let fresnel = pow(1.0 - nv, 3.6);
        let wrap_diff = 0.28 + 0.52 * max(dot(n_tube, key) + 0.35, 0.0) + 0.25 * max(dot(n_tube, fill) + 0.35, 0.0);
        // Subtle internal lumen refraction & warm tissue bounce near tip
        let tissue_bounce = (1.0 - clamp(in.reference.z * 30.0, 0.0, 1.0)) * vec3<f32>(0.22, 0.07, 0.08);
        let tube_rgb = in.color.rgb * wrap_diff + tissue_bounce + vec3<f32>(0.96, 0.98, 1.0) * (spec + 0.28 * fresnel);
        return vec4<f32>(filmic_tonemap(tube_rgb * 1.05), 1.0);
    }

    let n_macro = unit(mix(n_smooth, geom_n * sign(dot(geom_n, n_smooth) + 1e-5), 0.38));
    let wetness = clamp(in.uv.x, 0.0, 1.0);
    let mucosa = clamp(in.uv.y, 0.0, 1.0);
    let thinness = 1.0 - clamp(in.color.a, 0.1, 1.0);

    let eps = 0.00020;
    let h0 = skin_micro_height(in.reference, mucosa, wetness);
    let hx = skin_micro_height(in.reference + vec3<f32>(eps, 0.0, 0.0), mucosa, wetness);
    let hy = skin_micro_height(in.reference + vec3<f32>(0.0, eps, 0.0), mucosa, wetness);
    let grad_micro = vec3<f32>((hx - h0) / eps, (hy - h0) / eps, 0.0);
    let n_tissue = unit(n_macro - 0.000055 * grad_micro);
    let n_film = unit(mix(n_tissue, n_macro, smoothstep(0.0, 0.75, wetness)));

    let wrap = mix(vec3<f32>(0.42, 0.20, 0.11), vec3<f32>(0.58, 0.26, 0.15), mucosa);
    let key_wrap = max((vec3<f32>(dot(n_tissue, key)) + wrap) / (vec3<f32>(1.0) + wrap), vec3<f32>(0.0));
    let fill_wrap = max((vec3<f32>(dot(n_tissue, fill)) + wrap) / (vec3<f32>(1.0) + wrap), vec3<f32>(0.0));

    let back_scatter = pow(clamp(dot(-key, n_tissue) + 0.48, 0.0, 1.0), 2.2)
        + 0.65 * pow(clamp(dot(-rim, n_tissue) + 0.52, 0.0, 1.0), 2.0);
    let sss_translucency = thinness * back_scatter * vec3<f32>(0.24, 0.045, 0.035);

    let sss_diffuse = vec3<f32>(0.14, 0.095, 0.088)
        + 0.68 * key_wrap
        + 0.25 * fill_wrap
        + sss_translucency;

    let mottle = 0.975 + 0.05 * noise3(in.reference * 240.0);
    let wet_darken = mix(vec3<f32>(1.0), vec3<f32>(0.74, 0.63, 0.66), wetness * 0.78);
    let albedo = in.color.rgb * mottle * wet_darken;

    let base_roughness = mix(0.45, 0.25, mucosa) + 0.03 * h0;
    let secondary_roughness = mix(0.68, 0.42, mucosa);
    let film_roughness = mix(base_roughness, 0.045, smoothstep(0.0, 0.88, wetness));

    let tissue_spec = (0.72 * ggx_lobe(n_tissue, v, key, base_roughness, 0.028)
        + 0.28 * ggx_lobe(n_tissue, v, key, secondary_roughness, 0.028)
        + 0.34 * ggx_lobe(n_tissue, v, fill, base_roughness, 0.028))
        * (1.0 - 0.55 * wetness);

    let film_spec = wetness * (
        1.95 * ggx_lobe(n_film, v, key, film_roughness, 0.046)
        + 0.78 * ggx_lobe(n_film, v, fill, film_roughness * 1.12, 0.046)
        + 0.56 * ggx_lobe(n_film, v, rim, max(film_roughness, 0.075), 0.046)
    );

    let nv = max(dot(n_film, v), 0.0);
    let fresnel_sheen = wetness * pow(1.0 - nv, 4.5) * 0.18;

    let ao = clamp(0.64 + 0.36 * smoothstep(-0.0065, 0.0075, in.reference.z), 0.50, 1.0);
    let spec_color = mix(vec3<f32>(1.0, 0.95, 0.92), vec3<f32>(0.99, 0.995, 1.0), wetness);
    let raw_lit = albedo * sss_diffuse * ao
        + spec_color * (tissue_spec * ao + film_spec + fresnel_sheen * ao);
    let mapped = filmic_tonemap(raw_lit * 1.06);
    return vec4<f32>(mapped, 1.0);
}
";

#[derive(Clone, Copy, Debug)]
struct PreparedVertexBinding {
    rest_m: [f32; 3],
    rest_scaled: [f32; 3],
    side_idx: u8,
    b0: u8,
    b1: u8,
    frac: f32,
    majora_w: f32,
    minora_w: f32,
    minora_arch: f32,
    wet_envelope: f32,
    mucosa_factor: f32,
}

#[derive(Clone, Copy, Debug)]
struct TubeLocalVertex {
    local_offset_m: [f32; 3],
    color: [f32; 4],
}

/// Combined high-density curvilinear anatomical surface + rounded-tip cylindrical tube mesh.
#[derive(Debug)]
struct PreparedTubeInsertionScene {
    nu: usize,
    nv: usize,
    bindings: Vec<PreparedVertexBinding>,
    tube_local: Vec<TubeLocalVertex>,
    tissue_vertex_count: usize,
    vertices: Vec<SceneVertex>,
    indices: Vec<u32>,
    normals: Vec<[f32; 3]>,
    material_coordinates: Vec<[f32; 3]>,
    display_scale: f32,
}

impl PreparedTubeInsertionScene {
    #[allow(clippy::too_many_lines)]
    fn new(display_scale: f64, tube_radius_m: f64, tube_length_m: f64) -> Self {
        let nu = 128usize;
        let nv = 144usize;
        let half_width_m = 0.034;
        let half_length_m = 0.044;
        let scale = display_scale as f32;

        let tissue_vertex_count = (nu + 1) * (nv + 1);
        let tube_sectors = 36usize;
        let tip_rings = 10usize;
        let shaft_rings = 12usize;
        let cap_rings = 3usize;
        let tube_rings = tip_rings + shaft_rings + cap_rings;
        let tube_vertex_count = (tube_rings + 1) * (tube_sectors + 1);

        let total_vertices = tissue_vertex_count + tube_vertex_count;
        let mut bindings = Vec::with_capacity(tissue_vertex_count);
        let mut vertices = Vec::with_capacity(total_vertices);
        let mut material_coordinates = Vec::with_capacity(total_vertices);

        for iv in 0..=nv {
            let v_lin = (iv as f64 / nv as f64) * 2.0 - 1.0;
            let v = 0.80 * v_lin + 0.20 * v_lin.powi(3);
            let y = v * half_length_m;

            let v_maj = ((v - 0.02) / 0.80).clamp(-1.0, 1.0);
            let majora_env = (1.0 - v_maj * v_maj).max(0.0).powf(0.85);

            let v_min = ((v + 0.02) / 0.58).clamp(-1.0, 1.0);
            let minora_arch = (1.0 - v_min * v_min).max(0.0);
            let minora_env = minora_arch.powf(1.22);
            let minora_ant_bias = (1.0 + 0.22 * (v * 2.0).tanh()).clamp(0.72, 1.24);

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
                let u = 0.42 * u_lin + 0.58 * u_lin.powi(3);
                let row_scale = (1.0 - 0.28 * v * v).sqrt();
                let x_rest = u * half_width_m * row_scale;
                let abs_x = x_rest.abs();
                let side_idx = u8::from(u >= 0.0);
                let side_sign = if u >= 0.0 { 1.0_f32 } else { -1.0_f32 };

                let cleft_half_gap = (0.0014 * minora_arch.powf(1.12)).max(0.00018);

                let majora_center_x = 0.0024 + 0.0092 * majora_env;
                let majora_width = 0.0086 + 0.0038 * majora_env;
                let majora_xi = (abs_x - majora_center_x) / majora_width;
                let majora_profile = (-majora_xi * majora_xi * 1.55).exp() * majora_env;
                let majora_height = 0.0105 * majora_profile;

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

                let minora_center_x = cleft_half_gap + 0.0019 * minora_env;
                let minora_width = 0.00185 + 0.00115 * minora_env;
                let minora_xi = (abs_x - minora_center_x) / minora_width;
                let edge_wave = 1.0
                    + 0.065 * (v * 17.0 + f64::from(side_sign) * 0.5).sin()
                    + 0.035 * (v * 31.0 - f64::from(side_sign) * 0.9).cos();
                let minora_profile =
                    (-minora_xi * minora_xi * 1.95).exp() * minora_env * minora_ant_bias * edge_wave;
                let minora_height = 0.0048 * minora_profile;

                let sulcus_center_x = minora_center_x + 0.0029 * minora_env + 0.0009;
                let sulcus_xi = (abs_x - sulcus_center_x) / 0.0020;
                let sulcus_env = majora_env * (0.25 + 0.75 * minora_arch.sqrt());
                let sulcus_depression = -0.0019 * (-sulcus_xi * sulcus_xi * 1.95).exp() * sulcus_env;

                let groin_xi = (abs_x - 0.0235 * row_scale) / 0.0038;
                let groin_crease = -0.0014 * (-groin_xi * groin_xi * 1.8).exp() * majora_env;

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

                let rest_m = [x_rest as f32, y as f32, z_rest as f32];
                let rest_scaled = [
                    (x_rest * display_scale) as f32,
                    (y * display_scale) as f32,
                    (z_rest * display_scale) as f32,
                ];
                let color = [rgb[0], rgb[1], rgb[2], thickness_proxy];

                bindings.push(PreparedVertexBinding {
                    rest_m,
                    rest_scaled,
                    side_idx,
                    b0: b0 as u8,
                    b1: b1 as u8,
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
                material_coordinates.push(rest_m);
            }
        }

        let stride = nu + 1;
        let mut indices = Vec::with_capacity(nu * nv * 6 + tube_rings * tube_sectors * 6);
        for iv in 0..nv {
            for iu in 0..nu {
                let a = (iv * stride + iu) as u32;
                let b = a + 1;
                let c = ((iv + 1) * stride + iu) as u32;
                let d = c + 1;
                indices.extend([a, b, d, a, d, c]);
            }
        }

        let mut tissue_normals = vec![Vec3::ZERO; tissue_vertex_count];
        for tri in indices.chunks_exact(3) {
            let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let p0 = Vec3::from_array(vertices[i0].position);
            let p1 = Vec3::from_array(vertices[i1].position);
            let p2 = Vec3::from_array(vertices[i2].position);
            let n = (p1 - p0).cross(p2 - p0);
            tissue_normals[i0] += n;
            tissue_normals[i1] += n;
            tissue_normals[i2] += n;
        }
        let mut normals: Vec<[f32; 3]> = tissue_normals
            .into_iter()
            .map(|n| n.try_normalize().unwrap_or(Vec3::Z).to_array())
            .collect();

        // Build smooth rounded-tip cylindrical tube geometry (hemispherical lower tip + cylindrical shaft + beveled lumen cap)
        let mut tube_local = Vec::with_capacity(tube_vertex_count);
        let r_tube = tube_radius_m as f32;
        let shaft_len = (tube_length_m - tube_radius_m).max(0.005) as f32;
        for ir in 0..=tube_rings {
            let (ring_r, z_off, nz, axial_frac, is_lumen) = if ir <= tip_rings {
                // Hemispherical tip from bottom pole (theta = 0) to equator (theta = pi/2)
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
                // Beveled cannula top collar & recessed inner lumen
                let c_frac = (ir - tip_rings - shaft_rings) as f32 / (cap_rings as f32);
                let r_cap = r_tube * (1.0 - 0.96 * c_frac);
                let z_cap = r_tube + shaft_len + 0.0004 * (1.0 - c_frac) - 0.0022 * c_frac * c_frac;
                (r_cap, z_cap, 0.92, 1.0, c_frac > 0.35)
            };

            // Subtle depth graduation rings along the medical silicone/glass tube
            let grad_band = ((axial_frac * 28.0).sin().abs() < 0.14)
                && ir > tip_rings
                && ir <= tip_rings + shaft_rings;
            let base_col = if is_lumen {
                [0.42_f32, 0.50, 0.58, 0.95]
            } else if grad_band {
                [0.62_f32, 0.77, 0.86, 0.92]
            } else {
                [0.84_f32, 0.90, 0.95, 0.88]
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
                    uv: [0.4, -1.0],
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

        Self {
            nu,
            nv,
            bindings,
            tube_local,
            tissue_vertex_count,
            vertices,
            indices,
            normals,
            material_coordinates,
            display_scale: scale,
        }
    }

    fn recompute_tissue_normals(&mut self) {
        let stride = self.nu + 1;
        for iv in 0..=self.nv {
            let iv_prev = iv.saturating_sub(1);
            let iv_next = (iv + 1).min(self.nv);
            for iu in 0..=self.nu {
                let iu_prev = iu.saturating_sub(1);
                let iu_next = (iu + 1).min(self.nu);
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

    fn deform_scene(
        &mut self,
        majora_sim: &physics::biomechanics::LabiaGaitSimulation,
        minora_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
        tube: &LabiaTubeInsertion,
        sim_time_s: f64,
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

        let (tip_pos, tip_vel, _phase) = tube
            .tip_state_at(sim_time_s)
            .unwrap_or(([0.0, -0.0035, 0.0075], [0.0; 3], 0.0));
        let tip_x = tip_pos[0] as f32;
        let tip_y = tip_pos[1] as f32;
        let tip_z = tip_pos[2] as f32;
        let vel_z = tip_vel[2] as f32;
        let r_tube = tube.radius_m as f32;
        let scale = self.display_scale;

        // Global penetration engagement as the rounded tip passes the labial crests (+5.2mm -> -6.8mm)
        let engage_z = ((0.0052 - tip_z) / 0.0115).clamp(0.0, 1.0);
        let smooth_engage = engage_z * engage_z * (3.0 - 2.0 * engage_z);

        // 1. Deform anatomical tissue vertices smoothly (C-infinity) around the advancing/withdrawing tube
        for (vertex, b) in self.vertices[..self.tissue_vertex_count]
            .iter_mut()
            .zip(self.bindings.iter())
        {
            let side = usize::from(b.side_idx);
            let maj = majora_disp[side];
            let min = minora_crest_disp[side];

            let rx = b.rest_m[0] - tip_x;
            let ry = b.rest_m[1] - tip_y;
            let r_xy = rx.hypot(ry);

            // Smooth odd lateral profile (0 at x=0, peaking on the inner labial walls/crests)
            let odd_x = rx / (rx * rx + 0.0014 * 0.0014).sqrt();
            let odd_y = ry / (ry * ry + 0.0022 * 0.0022).sqrt();
            let radial_bell = (-((r_xy / (r_tube * 2.25)).powi(2))).exp();
            let cuff_ring = (-(((r_xy - r_tube * 0.96) / (r_tube * 0.72)).powi(2))).exp();

            // Smooth lateral parting of Labia Minora & Labia Majora around the tube circumference
            let tube_parting_dx = odd_x
                * radial_bell
                * smooth_engage
                * (0.0031 + 0.0035 * b.minora_w + 0.0012 * b.majora_w);
            let tube_parting_dy = odd_y * radial_bell * smooth_engage * 0.0011;

            // Smooth vestibular funneling under the rounded tip + axial mucosal cuff drag (invagination on insertion, eversion on withdrawal)
            let central_funnel =
                -0.0046 * (-((r_xy / (r_tube * 1.22)).powi(2))).exp() * smooth_engage;
            let cuff_lip = 0.0008 * cuff_ring * smooth_engage;
            let velocity_drag_z =
                (vel_z / 0.042).clamp(-1.0, 1.0) * 0.0016 * cuff_ring * smooth_engage;
            let tube_cuff_dz = central_funnel + cuff_lip + velocity_drag_z;

            let dx = (1.5 * b.majora_w * maj[0] + 1.8 * b.minora_w * min[0] + tube_parting_dx)
                .clamp(-0.0075, 0.0075);
            let dy = (1.5 * b.majora_w * maj[1] + 1.6 * b.minora_w * min[1] + tube_parting_dy)
                .clamp(-0.0045, 0.0045);
            let dz = (1.4 * b.majora_w * maj[2]
                + 1.6 * b.minora_w * min[2] * b.minora_arch
                + tube_cuff_dz)
                .clamp(-0.0095, 0.0035);

            vertex.position = [
                b.rest_scaled[0] + dx * scale,
                b.rest_scaled[1] + dy * scale,
                b.rest_scaled[2] + dz * scale,
            ];

            let h_film = (1.0 - b.frac) * film_wetness_profile[side][usize::from(b.b0)]
                + b.frac * film_wetness_profile[side][usize::from(b.b1)];
            let meniscus_boost = 0.54 * cuff_ring * smooth_engage;
            let wetness = (h_film * b.wet_envelope + meniscus_boost).clamp(0.0, 1.0);
            let mucosa_flush = (b.mucosa_factor + 0.22 * cuff_ring * smooth_engage).clamp(0.0, 1.0);
            vertex.uv = [wetness, mucosa_flush];
        }

        // 2. Update 3D tube vertices at current tip position
        for (vertex, tl) in self.vertices[self.tissue_vertex_count..]
            .iter_mut()
            .zip(self.tube_local.iter())
        {
            let wx = tip_x + tl.local_offset_m[0];
            let wy = tip_y + tl.local_offset_m[1];
            let wz = tip_z + tl.local_offset_m[2];
            vertex.position = [wx * scale, wy * scale, wz * scale];
            // Lower portion of the tube becomes coated with mucosal lubricant after entering the cleft
            let wet_coat = ((0.0040 - wz) / 0.0085).clamp(0.15, 0.96);
            vertex.uv = [wet_coat, -1.0];
            vertex.color = tl.color;
        }

        self.recompute_tissue_normals();
    }

    fn build_full_mesh(
        &mut self,
        majora_sim: &physics::biomechanics::LabiaGaitSimulation,
        minora_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
        tube: &LabiaTubeInsertion,
        sim_time_s: f64,
    ) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        self.deform_scene(majora_sim, minora_sim, films, tube, sim_time_s);
        Ok(SceneMesh::new(self.vertices.clone(), self.indices.clone())?
            .with_normals(self.normals.clone())?
            .with_material_coordinates(self.material_coordinates.clone())?)
    }

    fn update_vertices_in_place(
        &mut self,
        queue: &wgpu::Queue,
        geometry: &SceneGeometry,
        majora_sim: &physics::biomechanics::LabiaGaitSimulation,
        minora_sim: &physics::skin::LabiaMinoraGaitSimulation,
        films: &[SurfaceFilm; 2],
        tube: &LabiaTubeInsertion,
        sim_time_s: f64,
    ) {
        self.deform_scene(majora_sim, minora_sim, films, tube, sim_time_s);
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
        .unwrap_or_else(|| "/private/tmp/voxy-labia-tube-insertion.png".into());

    let majora_geom = LabiaMajoraGeometry {
        length_m: 0.052,
        width_m: 0.018,
        height_m: 0.009,
        separation_m: 0.022,
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
        fold_width_m: 0.0040,
        fold_height_m: 0.0055,
        separation_m: 0.010,
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

    let tube = LabiaTubeInsertion {
        radius_m: 0.0038,
        length_m: 0.025,
        axis_xy_m: [0.0, -0.0035],
        start_tip_z_m: 0.0075,
        max_depth_m: 0.0145,
        frequency_hz: 1.0,
    };

    let display_scale = 34.0;
    let mut prepared_scene =
        PreparedTubeInsertionScene::new(display_scale, tube.radius_m, tube.length_m);

    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Labia Tube Insertion GPU Renderer: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut geometry = renderer.reserve_geometry(&device, 24_000, 135_000)?;
    pollster::block_on(renderer.set_geometry_opaque_shader(
        &device,
        &mut geometry,
        ANATOMICAL_MUCOSAL_AND_TUBE_SHADER,
    ))?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let camera_eye = Vec3::new(0.95, -1.75, 1.75);
    let base_projection = SceneCamera {
        eye: camera_eye,
        target: Vec3::new(0.0, -0.06, 0.04),
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
            label: Some("labia tube snapshot"),
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
        label: Some("labia tube pixels"),
        size: 4096 * 768,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let cells = films[0].triangles().len();
    for side in 0..2 {
        films[side].deposit(cells / 3, 2.5e-9)?;
        films[side].deposit(cells / 2, 3.2e-9)?;
    }
    let friction_mat = friction::Material::new(1e6, 1e6, 0.32)?;
    let mut friction_states = [
        vec![friction::State::default(); cells],
        vec![friction::State::default(); cells],
    ];
    let pressures = vec![480.0; cells];

    let stages = [
        (0.05, "approach"),
        (0.18, "labial-contact-parting"),
        (0.32, "vestibular-insertion"),
        (0.50, "full-depth-meniscus"),
        (0.72, "withdrawal-cuff-eversion"),
    ];

    let mut frames = Vec::new();
    let mut sim_t = 0.0_f64;

    for (frame_idx, &(target_t, stage_label)) in stages.iter().enumerate() {
        while sim_t + 0.01 <= target_t {
            let (_, tip_vel, phase) = tube.tip_state_at(sim_t)?;
            majora_sim.step_tube_insertion(phase, tip_vel[2], 0.14, 2e-4, 5e-3)?;
            let mut synced_tube = tube;
            synced_tube.radius_m = 0.0016;
            synced_tube.frequency_hz = 10.0;
            minora_sim.step_tube_insertion(
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
                films[side].update_geometry(minora_sim.folds[side].skin.positions())?;
                films[side].advance_smear(
                    0.001,
                    SmearContact {
                        normal: [if side == 0 { 1.0 } else { -1.0 }, 0.0, 0.0],
                        body_velocity_m_s: [0.0, 0.015, tip_vel[2] * 0.4],
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
                    transfer_speed: 0.002,
                    opposing_cosine: 0.1,
                    max_candidates: 500_000,
                },
            )?;
            sim_t += 0.02;
        }

        let (tip_pos, tip_vel, phase) = tube.tip_state_at(target_t)?;
        let mesh = prepared_scene.build_full_mesh(&majora_sim, &minora_sim, &films, &tube, target_t)?;
        geometry.update(&queue, &mesh)?;

        println!(
            "frame={frame_idx} ({stage_label}) | t={target_t:.2}s phase={phase:.2} tip_z={:.2}mm vel_z={:.1}mm/s | minora_sep={:.2}mm majora_sep={:.2}mm",
            tip_pos[2] * 1000.0,
            tip_vel[2] * 1000.0,
            minora_sim.mean_crest_separation_m() * 1000.0,
            majora_sim.apex_separation_m() * 1000.0,
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
        let frame_vec = pixels.to_vec();
        drop(pixels);
        readback.unmap();

        let single_path = format!("/private/tmp/voxy-labia-tube-frame-{frame_idx}.png");
        image::save_buffer(
            &single_path,
            &frame_vec,
            1024,
            768,
            image::ColorType::Rgba8,
        )?;
        frames.push(frame_vec);
    }

    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }

    let mut strip = Vec::with_capacity(4096 * frames.len() * 768);
    for row in 0..768 {
        for frame in &frames {
            strip.extend_from_slice(&frame[row * 4096..(row + 1) * 4096]);
        }
    }
    image::save_buffer(
        &path,
        &strip,
        1024 * frames.len() as u32,
        768,
        image::ColorType::Rgba8,
    )?;
    println!("LABIA TUBE INSERTION SNAPSHOT PASS: {path}");

    if window_mode {
        run_interactive_window(
            instance,
            adapter,
            device,
            queue,
            majora_sim,
            minora_sim,
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
    majora_sim: physics::biomechanics::LabiaGaitSimulation,
    minora_sim: physics::skin::LabiaMinoraGaitSimulation,
    films: [SurfaceFilm; 2],
    tube: LabiaTubeInsertion,
    pressures: Vec<f64>,
    friction_mat: friction::Material,
    friction_states: [Vec<friction::State>; 2],
    prepared_scene: PreparedTubeInsertionScene,
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
        tube: LabiaTubeInsertion,
        pressures: Vec<f64>,
        friction_mat: friction::Material,
        friction_states: [Vec<friction::State>; 2],
        prepared_scene: PreparedTubeInsertionScene,
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
                .with_title("Voxy FEM | Labia Tube Insertion & Mucosal Meniscus Animation")
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
                .reserve_geometry(&self.device, 24_000, 135_000)
                .expect("geometry");
            pollster::block_on(renderer.set_geometry_opaque_shader(
                &self.device,
                &mut geometry,
                ANATOMICAL_MUCOSAL_AND_TUBE_SHADER,
            ))
            .expect("shader");
            let initial_mesh = self
                .prepared_scene
                .build_full_mesh(&self.majora_sim, &self.minora_sim, &self.films, &self.tube, 0.0)
                .expect("initial mesh");
            geometry.update(&self.queue, &initial_mesh).expect("initial upload");

            let texture = renderer
                .upload_texture(&self.device, &self.queue, 1, 1, &[255; 4])
                .expect("texture");
            let camera_eye = Vec3::new(0.95, -1.75, 1.75);
            let proj = SceneCamera {
                eye: camera_eye,
                target: Vec3::new(0.0, -0.06, 0.04),
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
            let transform = renderer.create_transform(&self.device, proj).expect("transform");
            transform
                .update_view_position(&self.queue, camera_eye)
                .expect("view_pos");
            let depth = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("labia tube window depth"),
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
                        .unwrap_or(([0.0, -0.0035, 0.0075], [0.0; 3], 0.0));

                    if self.frame_count & 1 == 0 {
                        let _ = self
                            .majora_sim
                            .step_tube_insertion(phase, tip_vel[2], 0.015, 1.6e-4, 2e-2);
                    } else {
                        let active_side = ((self.frame_count >> 1) as usize) & 1;
                        let _ = self.films[active_side].advance_smear(
                            0.001,
                            SmearContact {
                                normal: [if active_side == 0 { 1.0 } else { -1.0 }, 0.0, 0.0],
                                body_velocity_m_s: [0.0, 0.01, tip_vel[2] * 0.35],
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
                            &self.majora_sim,
                            &self.minora_sim,
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
                        let last_work_ms =
                            self.step_render_ms_samples.last().copied().unwrap_or(1.0);
                        let compute_fps = 1000.0 / last_work_ms.max(1e-4);
                        if self.frame_count == 1 || self.frame_count % 20 == 0 {
                            window.set_title(&format!(
                                "Voxy FEM | Labia Tube Insertion | phase={:.0}% tip_z={:.2}mm | {:.0} FPS ({:.2}ms)",
                                phase * 100.0,
                                tip_pos[2] * 1000.0,
                                compute_fps,
                                last_work_ms,
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
