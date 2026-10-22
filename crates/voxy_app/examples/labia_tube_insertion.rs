//▫~•◦------------------------------------------------------------------------------------◦•~▫//
// █████████████████████████████████████ VOXY ENGINE █████████████████████████████████████ //
//▫~•◦------------------------------------------------------------------------------------◦•~▫//

//! Photorealistic anatomical vulva, labia minora/majora & vaginal introitus tube insertion (`voxy_app`).
//!
//! Features:
//! - Seamless 360-degree concentric polar-elliptical topology (`128` radial rings × `144` sectors)
//!   centered on the vaginal introitus `(0, -5.5 mm)` — zero `x=0` slit crack or seam artifacts.
//! - True anatomical canal occlusion (`u_rim = 0.14`): the vaginal introitus & fourchette rim stays
//!   at `z ~ -1.6 mm` while the internal vaginal canal plunges to `z = -14.5 mm`, so the penetrating
//!   cylindrical tube shaft disappears genuinely inside the vaginal lumen with its tip occluded by
//!   the posterior fourchette and lateral labia minora lips.
//! - Overhanging, softly apposed petal-shaped Labia Minora (`Nymphae`) with scalloped free edges,
//!   deeply undercut Clitoral Hood (`preputium clitoridis`) and Clitoral Glans (`glans clitoridis`),
//!   deep shadowed interlabial sulci, and full-viewport Labia Majora cushions.
//! - Crisp 2D Voronoi epidermal pores, Fordyce sebaceous micro-papules on the inner labia minora,
//!   and localized mucosal wetness strictly inside the vestibule and lubricant meniscus collar.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use physics::skin::{LabiaTubeInsertion, OrificeDilationKernel};
use voxy_render::GraphicsOptions;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;

/// Radial rings from deep inside the vaginal canal (`r = 0`) to the outer labia majora (`r = NR - 1`).
const NR: usize = 180;
/// Angular sectors around the 360-degree circumference (`s = 0 .. NS - 1`).
const NS: usize = 200;

const TUBE_RADIUS_MM: f32 = 3.80;
const TUBE_HALF_LENGTH_MM: f32 = 11.5;
const INSERTION_PERIOD_SECONDS: f32 = 0.88;

/// Center of the vaginal introitus in world XY coordinates (mm).
const INTROITUS_Y_MM: f32 = -5.5;

const DEFAULT_OUTPUT_PATH: &str = "/private/tmp/voxy-labia-tube-insertion.png";

const SNAPSHOT_FRAMES: [(usize, f32, &str); 5] = [
    (0, 0.05, "approach-clitoris-sphincters-introitus"),
    (1, 0.18, "orifice-contact-lubricant-meniscus"),
    (2, 0.32, "sphincter-dilation-friction-invagination"),
    (3, 0.50, "full-depth-bulbospongiosus-cuff"),
    (4, 0.72, "withdrawal-vaginal-mucosal-rosette-prolapse"),
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    params: [f32; 4], // x: time, y: insertion_phase, z: wetness, w: tube_tip_z
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SkinVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    tissue_props: [f32; 4], // [region_id, roughness, sss_strength, wetness]
    uv_world: [f32; 2],     // anatomical rest coordinates (mm) for 2D Voronoi pores & Fordyce spots
}

const SHADER_SOURCE: &str = r"
struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u_scene: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) tissue_props: vec4<f32>,
    @location(4) uv_world: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) tissue_props: vec4<f32>,
    @location(4) uv_world: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u_scene.view_proj * vec4<f32>(in.position, 1.0);
    out.world_pos = in.position;
    out.normal = normalize(in.normal);
    out.color = in.color;
    out.tissue_props = in.tissue_props;
    out.uv_world = in.uv_world;
    return out;
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = dot(hash22(i + vec2<f32>(0.0, 0.0)) - 0.5, f - vec2<f32>(0.0, 0.0));
    let b = dot(hash22(i + vec2<f32>(1.0, 0.0)) - 0.5, f - vec2<f32>(1.0, 0.0));
    let c = dot(hash22(i + vec2<f32>(0.0, 1.0)) - 0.5, f - vec2<f32>(0.0, 1.0));
    let d = dot(hash22(i + vec2<f32>(1.0, 1.0)) - 0.5, f - vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) + 0.5;
}

fn fbm2(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var pos = p;
    let rot = mat2x2<f32>(0.80, -0.60, 0.60, 0.80);
    for (var i = 0; i < 4; i++) {
        v += a * (noise2d(pos) - 0.5);
        pos = rot * pos * 2.07 + vec2<f32>(13.7, 29.1);
        a *= 0.5;
    }
    return v;
}

fn voronoi_skin(p: vec2<f32>) -> vec2<f32> {
    let n = floor(p);
    let f = fract(p);
    var d1 = 8.0;
    var d2 = 8.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let g = vec2<f32>(f32(i), f32(j));
            let o = hash22(n + g);
            let r = g + (0.15 + 0.70 * o) - f;
            let d = dot(r, r);
            if (d < d1) {
                d2 = d1;
                d1 = d;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    let s1 = sqrt(d1);
    let s2 = sqrt(d2);
    return vec2<f32>(s1, s2 - s1);
}

fn vulvar_micro_height(uv_mm: vec2<f32>, region_id: f32) -> f32 {
    // 1. Subtle follicular pore dimples + soft multi-octave dermal micro-relief (no polygonal cracks)
    let v_coarse = voronoi_skin(uv_mm * 3.2);
    let v_fine = voronoi_skin(uv_mm * 6.5 + vec2<f32>(5.7, 14.3));
    let pore_pit = -smoothstep(0.16, 0.0, v_coarse.x) * 0.34
                   -smoothstep(0.14, 0.0, v_fine.x) * 0.16;
    let dermal_grain = fbm2(uv_mm * 1.8) * 0.28 + fbm2(uv_mm * 4.1 + vec2<f32>(3.9, 9.2)) * 0.14;

    // 2. Fine transverse & oblique mucosal micro-pleats on Labia Minora & Vestibule
    let mucosal_pleats = fbm2(vec2<f32>(uv_mm.x * 2.4, uv_mm.y * 1.10)) * 0.38
                       + fbm2(vec2<f32>(uv_mm.x * 4.8, uv_mm.y * 2.15)) * 0.18;

    // 3. Tiny sebaceous Fordyce micro-papules on inner Labia Minora
    let v_fordyce = voronoi_skin(uv_mm * 3.8 + vec2<f32>(19.1, 3.7));
    let fordyce_papule = smoothstep(0.14, 0.0, v_fordyce.x) * 0.18;

    let w_mucosa = clamp(1.0 - region_id, 0.0, 1.0);
    let w_outer = clamp(region_id - 1.0, 0.0, 1.0);
    let w_minora = clamp(1.0 - w_mucosa - w_outer, 0.0, 1.0);

    let h_mucosa = mucosal_pleats * 0.65;
    let h_minora = mucosal_pleats * 0.95 + fordyce_papule + dermal_grain * 0.45;
    let h_outer = dermal_grain + pore_pit * 0.95;

    return h_mucosa * w_mucosa + h_minora * w_minora + h_outer * w_outer;
}

fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let V = normalize(u_scene.camera_pos.xyz - in.world_pos);
    var N = normalize(in.normal);
    let region_id = in.tissue_props.x;
    let base_roughness = in.tissue_props.y;
    let sss_strength = in.tissue_props.z;
    let wetness = clamp(in.tissue_props.w, 0.0, 1.0);

    // 3-point clinical macro photography lighting rig
    let L1 = normalize(vec3<f32>(0.24, -0.42, 0.87));
    let L2 = normalize(vec3<f32>(-0.46, -0.22, 0.86));
    let L3 = normalize(vec3<f32>(0.04, 0.66, 0.75));

    // Medical-grade translucent silicone probe (region_id > 2.5)
    if (region_id > 2.5) {
        let H1 = normalize(L1 + V);
        let H2 = normalize(L2 + V);
        let H3 = normalize(L3 + V);
        let n_dot_v = max(dot(N, V), 0.0);
        let diff1 = max(dot(N, L1), 0.0);
        let diff2 = max(dot(N, L2), 0.0);
        let wrap_probe = max((dot(N, L1) + 0.60) / 1.60, 0.0);
        let spec1 = pow(max(dot(N, H1), 0.0), 190.0) * 1.15;
        let spec2 = pow(max(dot(N, H2), 0.0), 120.0) * 0.55;
        let spec3 = pow(max(dot(N, H3), 0.0), 90.0) * 0.35;
        let fresnel = 0.04 + 0.96 * pow(1.0 - n_dot_v, 4.2);

        let tip_z = u_scene.params.w;
        let axial_dist = max(in.world_pos.z - tip_z, 0.0);
        let wet_sleeve = wetness * exp(-axial_dist * 0.15);
        let clearcoat = pow(max(dot(N, H1), 0.0), 520.0) * wet_sleeve * 1.85;

        let tissue_bleed = exp(-max(in.world_pos.z + 1.5, 0.0) * 0.28) * vec3<f32>(0.24, 0.06, 0.07);
        let base_col = (in.color + tissue_bleed * 0.35) * (0.22 + 0.52 * diff1 + 0.24 * diff2 + 0.18 * wrap_probe)
            + vec3<f32>(0.95, 0.97, 1.0) * (spec1 + spec2 + spec3 + clearcoat + fresnel * 0.16);
        let mapped = pow(aces_tonemap(base_col), vec3<f32>(1.0 / 2.2));
        return vec4<f32>(mapped, 1.0);
    }

    // Epidermal & mucosal micro-normal perturbation
    let uv = in.uv_world;
    let eps = 0.045;
    let h0 = vulvar_micro_height(uv, region_id);
    let hx = vulvar_micro_height(uv + vec2<f32>(eps, 0.0), region_id);
    let hy = vulvar_micro_height(uv + vec2<f32>(0.0, eps), region_id);
    let surface_up = clamp(N.z, 0.0, 1.0);
    let bump_atten = mix(1.0, 0.32, wetness * wetness) * (0.25 + 0.75 * surface_up);
    let micro_nx = -(hx - h0) / eps * 0.048 * bump_atten;
    let micro_ny = -(hy - h0) / eps * 0.048 * bump_atten;
    N = normalize(N + vec3<f32>(micro_nx, micro_ny, 0.0));

    // Subdermal vascular & melanin micro-mottling + Fordyce sebaceous tint
    let mottle_low = fbm2(uv * 0.22);
    let mottle_med = fbm2(uv * 0.85 + vec2<f32>(4.2, 8.7));
    let v_pore = voronoi_skin(uv * 3.2);
    let pore_dark = smoothstep(0.15, 0.0, v_pore.x);
    let v_fordyce = voronoi_skin(uv * 3.8 + vec2<f32>(19.1, 3.7));
    let minora_mask = clamp(1.0 - abs(region_id - 0.85) * 1.6, 0.0, 1.0);
    let fordyce_spot = smoothstep(0.13, 0.0, v_fordyce.x) * minora_mask;

    var albedo = in.color;
    let outer_skin_mask = clamp(region_id - 1.0, 0.0, 1.0);
    albedo *= (1.0 - pore_dark * 0.10 * (0.25 + 0.75 * outer_skin_mask));
    albedo = mix(albedo, vec3<f32>(0.55, 0.30, 0.22), fordyce_spot * 0.18);
    albedo += vec3<f32>(0.022, -0.005, -0.006) * mottle_low
            + vec3<f32>(0.012, 0.003, 0.002) * mottle_med;

    let lube_mask = wetness * wetness;
    albedo = mix(albedo, albedo * vec3<f32>(0.82, 0.70, 0.66), lube_mask * 0.42);
    albedo = clamp(albedo, vec3<f32>(0.015), vec3<f32>(0.92));

    // Dual-lobe subsurface scattering (translucent crimson-pink Labia Minora & warm Labia Majora)
    let n_dot_l1 = max(dot(N, L1), 0.0);
    let n_dot_l2 = max(dot(N, L2), 0.0);
    let n_dot_l3 = max(dot(N, L3), 0.0);
    let wrap1 = max((dot(N, L1) + 0.58) / 1.58, 0.0);
    let wrap2 = max((dot(N, L2) + 0.50) / 1.50, 0.0);
    let sss_blood = vec3<f32>(0.72, 0.12, 0.08);
    let sss_contrib = sss_blood * sss_strength * (
        0.45 * pow(wrap1, 1.48) * (1.0 - 0.50 * n_dot_l1) +
        0.26 * pow(wrap2, 1.65) * (1.0 - 0.50 * n_dot_l2)
    );

    // Self-occlusion in deep vaginal canal lumen & subtle introitus crease occlusion (no outer skin halo)
    let depth_ao = mix(0.008, 1.0, smoothstep(-11.5, -1.4, in.world_pos.z));
    let introitus_dist = length(in.world_pos.xy - vec2<f32>(0.0, -5.5));
    let introitus_ao = mix(0.45, 1.0, smoothstep(0.15, 2.6, introitus_dist));
    let ao = clamp(depth_ao * introitus_ao, 0.008, 1.0);

    let diffuse = albedo * (0.18 * ao + 0.56 * n_dot_l1 + 0.26 * n_dot_l2 + 0.10 * n_dot_l3)
        + sss_contrib * ao;

    // Dual-lobe specular: soft satin epidermal sheen on Labia Majora + sharp wet clearcoat in Vestibule
    let H1 = normalize(L1 + V);
    let H2 = normalize(L2 + V);
    let H3 = normalize(L3 + V);
    let n_dot_v = max(dot(N, V), 0.0);
    let fresnel = 0.028 + 0.972 * pow(1.0 - n_dot_v, 5.0);

    let rough = clamp(mix(base_roughness, 0.04, lube_mask), 0.04, 0.92);
    let spec_exp = mix(400.0, 14.0, rough * rough);
    let pore_spec_mask = 1.0 - pore_dark * 0.55 * outer_skin_mask;

    let dry_lobe = (
        pow(max(dot(N, H1), 0.0), spec_exp) * 0.15 +
        pow(max(dot(N, H2), 0.0), spec_exp * 0.7) * 0.08 +
        pow(max(dot(N, H3), 0.0), 22.0) * 0.04
    ) * pore_spec_mask * ao;

    let wet_clearcoat = (
        pow(max(dot(N, H1), 0.0), 560.0) * 2.05 +
        pow(max(dot(N, H2), 0.0), 330.0) * 0.98 +
        pow(max(dot(N, H3), 0.0), 185.0) * 0.48
    ) * lube_mask * (0.32 + 0.68 * fresnel) * ao;

    let color_linear = diffuse + vec3<f32>(1.0, 0.96, 0.94) * (dry_lobe + wet_clearcoat);
    let mapped = pow(aces_tonemap(color_linear), vec3<f32>(1.0 / 2.2));
    return vec4<f32>(mapped, 1.0);
}
";

#[derive(Clone, Copy, Debug)]
struct Kinematics {
    insertion_phase: f32,
    tip_z_mm: f32,
    velocity_z_mm_s: f32,
}

fn sample_kinematics(time_s: f32) -> Kinematics {
    let omega = std::f32::consts::TAU / INSERTION_PERIOD_SECONDS;
    let angle = time_s * omega;
    let raw_sin = angle.sin();
    let raw_cos = angle.cos();
    let phase = 0.5 * (1.0 - raw_cos);
    let tip_z_mm = 7.2 - 15.0 * phase;
    let velocity_z_mm_s = -7.5 * omega * raw_sin;
    Kinematics {
        insertion_phase: phase,
        tip_z_mm,
        velocity_z_mm_s,
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

struct VulvaSimulation {
    dilation_kernel: OrificeDilationKernel,
    minora_separation_mm: f32,
    majora_separation_mm: f32,
    lubricant_boost: f32,
}

impl VulvaSimulation {
    fn new() -> Self {
        Self {
            dilation_kernel: OrificeDilationKernel::vaginal_introitus_default(),
            minora_separation_mm: 10.0,
            majora_separation_mm: 22.0,
            lubricant_boost: 0.0,
        }
    }

    fn build_polar_grid_indices() -> Vec<u32> {
        let mut indices = Vec::with_capacity((NR - 1) * NS * 6);
        for r in 0..(NR - 1) {
            for s in 0..NS {
                let s_next = (s + 1) % NS;
                let i00 = (r * NS + s) as u32;
                let i10 = ((r + 1) * NS + s) as u32;
                let i01 = (r * NS + s_next) as u32;
                let i11 = ((r + 1) * NS + s_next) as u32;
                indices.extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
            }
        }
        indices
    }

    fn build_vertices(&mut self, time_s: f32) -> (Vec<SkinVertex>, Kinematics) {
        let kin = sample_kinematics(time_s);
        let pen = ((2.2 - kin.tip_z_mm) / 9.8).clamp(0.0, 1.0);

        // Normalized insertion-withdrawal cycle [0, 1):
        // At frame 3 (t = 0.50 s, u = 0.568), prolapse_t = 0.37 (initial mucosal cuff eversion around shaft);
        // At frame 4 (t = 0.72 s, u = 0.818), prolapse_t = 1.00 (peak vaginal mucosal rosette prolapse!).
        let cycle_u = (time_s / INSERTION_PERIOD_SECONDS).rem_euclid(1.0);
        let prolapse_t = smoothstep(0.46, 0.72, cycle_u) * (1.0 - smoothstep(0.88, 0.98, cycle_u));

        // 26-degree anatomical vaginal canal axis (tilted toward -Y so the probe enters cleanly
        // from the bottom-center of the frame and never obstructs the Vestibule or Clitoris).
        let sin_alpha = 0.4384f32;
        let cos_alpha = 0.8988f32;
        let tan_alpha = sin_alpha / cos_alpha;

        let z_pivot = -1.35 - 0.95 * pen;
        let s_tip = kin.tip_z_mm - z_pivot;
        let cap_center_s = s_tip + TUBE_RADIUS_MM;
        let r_tube_at_rim = if s_tip >= 0.0 {
            0.0
        } else if cap_center_s > 0.0 {
            let ds = cap_center_s;
            (TUBE_RADIUS_MM * TUBE_RADIUS_MM - ds * ds).max(0.0).sqrt()
        } else {
            TUBE_RADIUS_MM
        };

        // Resting vaginal introitus (`ostium vaginae`) is an almond-shaped mucosal opening
        // (~2.2 mm x 3.7 mm) framed by the Bulbospongiosus sphincter ring & hymenal caruncles.
        // During withdrawal prolapse (frame 4), the everted vaginal mucosal rosette stays gaping
        // (`prolapse_rx ~ 1.68 mm, prolapse_ry ~ 2.35 mm`) inside the Labia Minora!
        let pre_dilation = 0.45 * smoothstep(3.2, -0.3, s_tip) * (1.0 - prolapse_t);
        let prolapse_rx = 1.68 * prolapse_t;
        let prolapse_ry = 2.35 * prolapse_t;
        let orifice_rx_rest = 1.10;
        let orifice_ry_rest = 1.85;
        let orifice_rx = (orifice_rx_rest + pre_dilation)
            .max(r_tube_at_rim - 0.04)
            .max(prolapse_rx)
            .min(TUBE_RADIUS_MM - 0.03);
        let orifice_ry = (orifice_ry_rest + pre_dilation * 0.95)
            .max((r_tube_at_rim / cos_alpha) - 0.04)
            .max(prolapse_ry)
            .min((TUBE_RADIUS_MM / cos_alpha) - 0.03);
        let tube_contact_t =
            ((orifice_rx - orifice_rx_rest) / (TUBE_RADIUS_MM - orifice_rx_rest)).clamp(0.0, 1.0);

        let shear_dir = (-kin.velocity_z_mm_s / 42.0).clamp(-1.0, 1.0);
        let meniscus_contact = smoothstep(2.6, -1.0, s_tip).max(0.75 * prolapse_t);

        let mut positions = vec![Vec3::ZERO; NR * NS];
        let mut colors = vec![[0.0f32; 3]; NR * NS];
        let mut props = vec![[0.0f32; 4]; NR * NS];
        let mut uvs = vec![[0.0f32; 2]; NR * NS];

        // u_rim separates the internal vaginal canal (u <= u_rim) from the external vulvar anatomy (u > u_rim)
        let u_rim = 0.14;

        for r in 0..NR {
            let u_ring = r as f32 / (NR - 1) as f32;

            for s in 0..NS {
                let theta = (s as f32 / NS as f32) * std::f32::consts::TAU;
                let cos_t = theta.cos();
                let sin_t = theta.sin();

                let (rx, ry, rx_rest, ry_rest, is_canal) = if u_ring <= u_rim {
                    let t_canal = u_ring / u_rim;
                    let scale_c = 0.35 + 0.65 * t_canal;
                    (
                        (orifice_rx * scale_c).max(0.10),
                        (orifice_ry * scale_c).max(0.16),
                        (orifice_rx_rest * scale_c).max(0.10),
                        (orifice_ry_rest * scale_c).max(0.16),
                        true,
                    )
                } else {
                    let u_ext = (u_ring - u_rim) / (1.0 - u_rim);
                    let max_rx = 34.0;
                    let max_ry = 42.0;
                    let rx_r = orifice_rx_rest + (max_rx - orifice_rx_rest) * u_ext.powf(1.22);
                    let ry_r = orifice_ry_rest + (max_ry - orifice_ry_rest) * u_ext.powf(1.20);
                    // Exponential radial tissue decay so dilation stretches the introitus and lower
                    // Labia Minora while keeping the Clitoral Glans, Hood, and outer Majora anchored.
                    let dilate_decay = (-u_ext * 4.5).exp();
                    let rx_e = rx_r + (orifice_rx - orifice_rx_rest) * dilate_decay;
                    let ry_e = ry_r + (orifice_ry - orifice_ry_rest) * dilate_decay;
                    (rx_e, ry_e, rx_r, ry_r, false)
                };

                let x0 = rx * cos_t;
                let y_surf = INTROITUS_Y_MM + ry * sin_t;

                let ux = rx_rest * cos_t;
                let uy = INTROITUS_Y_MM + ry_rest * sin_t;

                // Elliptical radial distance from the vaginal introitus rim
                let dx_intro = x0;
                let dy_intro = y_surf - INTROITUS_Y_MM;
                let r_intro = (dx_intro * dx_intro + dy_intro * dy_intro).sqrt();
                let r_orifice_angle =
                    ((orifice_rx * cos_t).powi(2) + (orifice_ry * sin_t).powi(2)).sqrt();
                let r_from_introitus = if is_canal {
                    0.0
                } else {
                    (r_intro - r_orifice_angle).max(0.0)
                };

                // Material-anchored longitudinal coordinate `vy` keeps upper vulvar landmarks fixed
                let vy = uy;

                // Plump 8-cushion vaginal rugae / caruncular profile (wide convex cushions, narrow sulci)
                let u_lobe_8 = (0.5 - 0.5 * (8.0 * theta + 0.25).cos()).clamp(0.0, 1.0);
                let u_lobe_14 = (0.5 - 0.5 * (14.0 * theta - 0.55).cos()).clamp(0.0, 1.0);
                let prolapse_sulcus = 0.72 * u_lobe_8.powf(2.3) + 0.28 * u_lobe_14.powf(2.4);
                let prolapse_cushion_profile = 1.0 - 0.26 * prolapse_sulcus;
                let prolapse_torus_env = (-((r_from_introitus - 0.72) / 0.88).powi(2)).exp();

                // Fusiform (almond/spindle) longitudinal envelope of the vulvar cleft from
                // Posterior Fourchette (y = -9.6 mm) to Anterior Commissure / Clitoral Hood (y = +13.0 mm)
                let spindle_u = ((vy - 0.5) / 10.4).clamp(-1.0, 1.0);
                let spindle_env = (1.0 - spindle_u * spindle_u).max(0.0).sqrt();

                // ── 1. Prominent 3D Clitoral Complex (`glans clitoridis`, `preputium`, `frenulum`) ──
                // Evaluated using concentric elliptical coordinates around the Clitoral Glans center
                // `(0.0, +7.35 mm)` so the Glans Pearl, Horseshoe Sub-Preputial Moat, and Clitoral Hood
                // are 100% smooth and free of any diagonal grid aliasing!
                let glans_dx = x0 / 1.28;
                let glans_dy = (vy - 7.35) / 1.52;
                let glans_r2 = glans_dx * glans_dx + glans_dy * glans_dy;
                let r_glans_ellip = glans_r2.sqrt();
                let glans_mask = (-glans_r2 * 1.35).exp();
                let glans_tip_dimple =
                    -0.25 * (-(x0 / 0.32).powi(2)).exp() * (-((vy - 6.55) / 0.55).powi(2)).exp();
                let clitoral_glans = 3.55 * glans_mask + glans_tip_dimple;

                // Deep Horseshoe Sub-Preputial Moat (`sulcus preputialis`) wrapping around the top and
                // lateral sides of the Clitoral Glans (`r_glans_ellip ~ 1.18`, `vy > 5.5 mm`):
                let moat_horseshoe =
                    (-((r_glans_ellip - 1.18) / 0.46).powi(2)).exp() * smoothstep(5.4, 7.0, vy);
                let glans_moat = -1.45 * moat_horseshoe;

                // Thick Overhanging Horseshoe Clitoral Hood (`preputium clitoridis`) wrapping outside
                // the moat (`r_glans_ellip ~ 1.82`) and flowing seamlessly into the Labia Minora:
                let hood_horseshoe =
                    (-((r_glans_ellip - 1.82) / 0.72).powi(2)).exp() * smoothstep(4.0, 6.2, vy);
                let hood_superior_tail =
                    (-((x0 / 1.70).powi(2) + ((vy - 10.5) / 2.25).powi(2))).exp();
                let clitoral_hood = 3.35 * (hood_horseshoe + 0.60 * hood_superior_tail).min(1.15);

                // Bilateral Clitoral Frenula (`frenulum clitoridis`) gently connecting the lower pole
                // of the Clitoral Glans (`vy = 6.2 mm, x = ±0.45 mm`) into the Labia Minora (`vy = 4.2 mm, x = ±1.65 mm`):
                let fren_t = ((6.3 - vy) / 2.2).clamp(0.0, 1.0);
                let fren_x_center = 0.42 + 1.18 * fren_t;
                let fren_y_env = smoothstep(3.6, 4.8, vy) * (1.0 - smoothstep(6.0, 7.0, vy));
                let fren_r = (-((x0 - fren_x_center) / 0.62).powi(2)).exp() * fren_y_env;
                let fren_l = (-((x0 + fren_x_center) / 0.62).powi(2)).exp() * fren_y_env;
                let clitoral_frenulum = 1.35 * (fren_r + fren_l);

                // Clear the Labia Minora inside the Clitoral Glans & Sub-Preputial Pocket so the Glans stands out!
                let clitoral_pocket_clear =
                    (-((x0 / 1.75).powi(2) + ((vy - 7.45) / 2.00).powi(2))).exp();

                // ── 2. Labia Minora (`Nymphae`) Framing the Open Vulvar Vestibule ──
                // The Labia Minora crests sit at `x = ±2.05..±2.45 mm` along the mid-vestibule so the
                // Urethral Sphincter Rosette (`vy = +1.25 mm`) and Vaginal Introitus (`vy = -5.5 mm`)
                // sit cleanly on the open Vestibular Floor between the two fleshy Labia Minora petals!
                let minora_ant_taper =
                    (1.0 - smoothstep(5.2, 7.4, vy)) * (1.0 - 0.88 * clitoral_pocket_clear);
                let minora_post_taper = smoothstep(-9.6, -6.6, vy);
                let minora_y_env = minora_ant_taper * minora_post_taper;

                let wave_r = 0.12 * (vy * 0.55 + 0.35).sin() + 0.06 * (vy * 1.15 - 0.40).cos();
                let wave_l = 0.12 * (vy * 0.50 - 0.45).sin() + 0.06 * (vy * 1.18 + 0.50).cos();

                let introitus_local = (-((vy - INTROITUS_Y_MM) / 4.4).powi(2)).exp();
                let base_ridge_x = 0.55
                    + 1.52 * spindle_env
                    + 0.58 * introitus_local
                    + 2.15 * tube_contact_t * introitus_local;
                let ridge_x_r = (base_ridge_x + wave_r * minora_y_env).max(0.55);
                let ridge_x_l = (base_ridge_x + wave_l * minora_y_env).max(0.55);

                let w_inner = 0.72 + 0.22 * spindle_env;
                let w_outer = 1.25 + 0.55 * spindle_env;

                let dx_r = x0 - ridge_x_r;
                let dx_l = -x0 - ridge_x_l;
                let w_r = w_inner + (w_outer - w_inner) * smoothstep(-w_inner, w_outer, dx_r);
                let w_l = w_inner + (w_outer - w_inner) * smoothstep(-w_inner, w_outer, dx_l);

                let scallop_r = 1.0
                    + 0.08 * (vy * 0.62 + 0.30).sin()
                    + 0.04 * (vy * 1.28 - 0.25).cos()
                    + 0.03 * (dx_r * 2.0).cos();
                let scallop_l = 1.0
                    + 0.08 * (vy * 0.58 - 0.40).sin()
                    + 0.04 * (vy * 1.24 + 0.45).cos()
                    + 0.03 * (dx_l * 2.0).cos();

                let lip_r = (-(dx_r / w_r).powi(2)).exp();
                let lip_l = (-(dx_l / w_l).powi(2)).exp();

                let minora_skew = (lip_r + lip_l).clamp(0.0, 1.0);
                // Keep the Labia Minora outside the prolapsing inner introital sphincter cuff (`r_from_introitus > 1.35 mm`)
                let collar_taper = smoothstep(0.45, 1.75, r_from_introitus);
                let minora_height =
                    3.45 * (lip_r * scallop_r + lip_l * scallop_l) * minora_y_env * collar_taper;

                // ── 3. Posterior Fourchette (`frenulum labiorum pudendi`) & Fossa Navicularis ──
                let fourchette_bridge = 1.55
                    * (-((vy + 8.9) / 1.40).powi(2)).exp()
                    * (-x0 * x0 / 4.8).exp()
                    * (1.0 - 0.30 * pen)
                    * smoothstep(0.3, 1.4, r_from_introitus);

                // ── 4. Smooth Interlabial Sulci (`sulcus interlabialis`) & Pudendal Cleft ──
                let sulcus_center_x = 0.65 + (base_ridge_x + w_outer * 0.92) * spindle_env;
                let sulc_r = (-((x0 - sulcus_center_x) / 0.88).powi(2)).exp();
                let sulc_l = (-((x0 + sulcus_center_x) / 0.88).powi(2)).exp();
                let sulc_y_env = smoothstep(-10.8, -7.5, vy) * (1.0 - smoothstep(10.2, 13.8, vy));
                let interlabial_sulcus = -1.10 * (sulc_r + sulc_l) * sulc_y_env;

                let ant_cleft = -0.85 * smoothstep(10.8, 14.8, vy) * (-x0 * x0 / 2.2).exp();
                let post_cleft =
                    -0.75 * (1.0 - smoothstep(-15.0, -9.0, vy)) * (-x0 * x0 / 2.2).exp();

                // ── 5. Pure C^infty Bilateral Labia Majora Cushions & Mons Pubis ──
                let majora_center_x = 7.6 + 1.4 * spindle_env;
                let maj_r = (-((x0 - majora_center_x) / 5.2).powi(2)).exp();
                let maj_l = (-((x0 + majora_center_x) / 5.2).powi(2)).exp();
                let majora_cushion = 1.95 * (maj_r + maj_l);

                let x_rad = (x0 * x0 + 1.0).sqrt();
                let outer_thigh_slope = 0.75 * smoothstep(13.0, 34.0, x_rad);
                let mons_pubis = 1.65 * smoothstep(8.0, 21.0, vy) * (-x0 * x0 / 190.0).exp();
                let perineal_body =
                    0.55 * (1.0 - smoothstep(-16.5, -9.2, vy)) * (-x0 * x0 / 110.0).exp();
                let ap_curvature = -0.0010 * vy * vy + 0.008 * vy;

                // ── 6. Vestibular Cleft & External Urethral Sphincter Rosette (`m. sphincter urethrae`) ──
                let vest_y_env = smoothstep(-8.8, -6.0, vy) * (1.0 - smoothstep(4.8, 6.8, vy));
                let vest_width_x = (base_ridge_x * 0.78).max(1.15);
                let vest_floor_env = (-(x0 / vest_width_x).powi(2)).exp() * vest_y_env;

                // Prominent External Urethral Sphincter Rosette (`ostium urethrae externum`) at (0.0, +1.25 mm)
                // sitting cleanly on the concave vestibular floor between the parted Labia Minora:
                let u_dx = x0 / 0.85;
                let u_dy = (vy - 1.25) / 0.98;
                let r_urethra = (u_dx * u_dx + u_dy * u_dy).sqrt();
                let theta_urethra = u_dy.atan2(u_dx);
                let urethral_pit_mask = (-r_urethra * r_urethra * 5.0).exp();
                let urethral_cuff_mask = (-((r_urethra - 0.80) / 0.40).powi(2)).exp();
                let urethral_stellate = 0.82 + 0.18 * (6.0 * theta_urethra).cos();
                let urethral_outer_moat = -0.35 * (-((r_urethra - 1.35) / 0.35).powi(2)).exp();
                let urethral_sphincter_rosette = -1.45 * urethral_pit_mask
                    + 1.18 * urethral_cuff_mask * urethral_stellate
                    + urethral_outer_moat;

                let vestibule_recess = -0.85 * vest_floor_env + urethral_sphincter_rosette;

                // ── 7. Bulbospongiosus Vaginal Introital Sphincter Cuff & Vaginal Mucosal Prolapse ──
                // Strictly localized around the vaginal introitus rim (`r_from_introitus < 2.2 mm`) inside the
                // vestibule so it never creates concentric bullseye rings across the outer Labia Minora/Majora!
                let intro_sphincter_env = (-((r_from_introitus - 0.65) / 0.72).powi(2)).exp();
                let intro_sphincter_outer_groove = -0.45
                    * (0.35 + 0.65 * prolapse_t + 0.25 * pen)
                    * (-((r_from_introitus - 1.55) / 0.45).powi(2)).exp();
                let bulbospongiosus_sphincter =
                    0.95 * intro_sphincter_env * (1.0 + 0.35 * pen + 0.35 * prolapse_t)
                        + intro_sphincter_outer_groove;

                // Fleshy hymenal caruncles & vaginal rugae (`carunculae hymenales` & `columnae rugarum`)
                let u_8 = (0.5 - 0.5 * (8.0 * theta + 0.25).cos()).powf(2.25);
                let u_14 = (0.5 - 0.5 * (14.0 * theta - 0.40).cos()).powf(2.40);
                let rugae_profile = (1.0 - 0.72 * u_8 - 0.28 * u_14) - 0.26;
                let rugae_effacement = 1.0 - 0.80 * pen - 0.30 * prolapse_t * (1.0 - pen);
                let rugae_cushions =
                    0.72 * rugae_profile * rugae_effacement * (-r_from_introitus * 0.62).exp();

                // True Vaginal Mucosal Rosette Prolapse (`пролапс слизистой розетки влагалища`):
                // Compact, plump, rolled toroidal mucosal doughnut (`r_from_introitus = 0..1.55 mm`, `+2.10 mm` high)
                // everting out of the gaping vaginal introitus inside the Labia Minora!
                let torus_dome = (0.55 * (-r_from_introitus * 0.95).exp() + prolapse_torus_env)
                    * prolapse_cushion_profile;
                let prolapse_transverse_rugae =
                    0.14 * (r_from_introitus * 6.4 - 0.6).sin() * (-r_from_introitus * 0.65).exp();
                let vaginal_prolapse_rosette =
                    prolapse_t * (1.95 * torus_dome + prolapse_transverse_rugae);

                // Smooth funnel roll-in at the vaginal introitus rim
                let introitus_roll_in =
                    -0.85 * (-r_from_introitus * 0.85).exp() * (1.0 - 0.75 * prolapse_t);

                // Dynamic axial response at the introitus collar
                let intro_collar_env = (-r_from_introitus * 0.38).exp();
                let invagination = -1.25
                    * pen
                    * (1.0 - 0.65 * prolapse_t)
                    * intro_collar_env
                    * (0.65 + 0.35 * shear_dir.max(0.0));
                let withdrawal_cuff = 1.45
                    * (-shear_dir).max(0.0)
                    * pen
                    * (-((r_from_introitus - 0.65).powi(2)) / 2.0).exp();

                // Seamless capillary lubricant meniscus lip right at the introitus contact collar
                let meniscus_lip = 0.58 * meniscus_contact * (-r_from_introitus * 1.9).exp();

                // Lateral petal fullness & toroidal prolapse lip overhang applied after clean field evaluation
                let mut x = x0;
                if !is_canal {
                    let overhang_r = 0.32 * lip_r * smoothstep(0.0, 1.8, dx_r);
                    let overhang_l = -0.32 * lip_l * smoothstep(0.0, 1.8, dx_l);
                    let prolapse_r_bulge =
                        0.45 * prolapse_t * prolapse_torus_env * prolapse_cushion_profile;
                    x += (overhang_r + overhang_l) * minora_y_env * collar_taper
                        + prolapse_r_bulge * cos_t;
                }

                // Unified surface height guarantees 100% C^1 continuity across u_ring == u_rim
                let z_surface = vestibule_recess
                    + rugae_cushions
                    + introitus_roll_in
                    + bulbospongiosus_sphincter
                    + vaginal_prolapse_rosette
                    + minora_height
                    + fourchette_bridge
                    + clitoral_glans
                    + glans_moat
                    + clitoral_frenulum
                    + clitoral_hood
                    + interlabial_sulcus
                    + ant_cleft
                    + post_cleft
                    + majora_cushion
                    + outer_thigh_slope
                    + mons_pubis
                    + perineal_body
                    + ap_curvature
                    + invagination
                    + withdrawal_cuff
                    + meniscus_lip;

                // Exact tilted-cylinder rim coupling: shifting y by `-tan_alpha * (z_surface - z_pivot)`
                // near the rim ensures that at u_ring == u_rim, every vertex (x, y, z_surface) lies
                // 100% on the tilted probe surface regardless of local cushion/labia height `z_surface`!
                let rim_tilt_couple = (-r_from_introitus * 0.65).exp();
                let y_coupled = y_surf - tan_alpha * (z_surface - z_pivot) * rim_tilt_couple;

                let (y, z) = if is_canal {
                    let t_canal = u_ring / u_rim;
                    let canal_drop = -12.5 * (1.0 - t_canal).powf(1.38);
                    (y_coupled - tan_alpha * canal_drop, z_surface + canal_drop)
                } else {
                    (y_coupled, z_surface)
                };

                // ── 8. True Linear-Space (sRGB^2.2) Multi-Zone Vulvar, Clitoral, Sphincter & Prolapse Pigmentation ──
                let majora_skin = [0.50f32, 0.25, 0.165];
                let sulcus_shadow_col = [0.24f32, 0.095, 0.072];
                let subpreputial_shadow_col = [0.065f32, 0.014, 0.016];
                let minora_edge_pigment = [0.25f32, 0.082, 0.068];
                let minora_inner_pink = [0.39f32, 0.102, 0.100];
                let glans_coral_rose = [0.49f32, 0.118, 0.125];
                let urethral_cuff_rose = [0.42f32, 0.082, 0.090];
                let urethral_lumen_dark = [0.032f32, 0.005, 0.007];
                let vestibule_mucosa = [0.31f32, 0.052, 0.062];
                let vaginal_prolapse_mucosa = [0.44f32, 0.042, 0.054];
                let prolapse_sulcus_crimson = [0.16f32, 0.015, 0.020];
                let deep_canal_col = [0.008f32, 0.001, 0.0015];

                let cleft_x_norm = x0 / (sulcus_center_x + 0.15);
                let cleft_y_norm = (vy - 0.6) / 10.4;
                let cleft_elliptical_r =
                    (cleft_x_norm * cleft_x_norm + cleft_y_norm * cleft_y_norm).sqrt();
                let to_majora = smoothstep(0.58, 1.22, cleft_elliptical_r);

                let in_sulcus =
                    (sulc_r + sulc_l).clamp(0.0, 1.0) * sulc_y_env * (1.0 - to_majora * 0.45);
                let on_minora_crest = minora_skew * minora_y_env * collar_taper;
                let in_vestibule =
                    vest_floor_env.max((-r_from_introitus * 0.65).exp() * (1.0 - to_majora));
                let prolapse_mucosa_mask =
                    prolapse_t * (1.0 - smoothstep(0.95, 1.65, r_from_introitus));
                let prolapse_crease = prolapse_sulcus.clamp(0.0, 1.0);

                let mut col = [0.0f32; 3];
                for c in 0..3 {
                    let mut surf =
                        minora_inner_pink[c] * (1.0 - to_majora) + majora_skin[c] * to_majora;
                    surf = surf * (1.0 - on_minora_crest * 0.62)
                        + minora_edge_pigment[c] * (on_minora_crest * 0.62);
                    surf =
                        surf * (1.0 - in_sulcus * 0.48) + sulcus_shadow_col[c] * (in_sulcus * 0.48);
                    surf = surf * (1.0 - in_vestibule * 0.85)
                        + vestibule_mucosa[c] * (in_vestibule * 0.85);
                    // Urethral sphincter cuff & dark central meatus pit
                    let u_cuff = (urethral_cuff_mask * 0.78).clamp(0.0, 1.0);
                    let u_pit = (urethral_pit_mask * 0.92).clamp(0.0, 1.0);
                    surf = surf * (1.0 - u_cuff) + urethral_cuff_rose[c] * u_cuff;
                    surf = surf * (1.0 - u_pit) + urethral_lumen_dark[c] * u_pit;
                    // Deep horseshoe sub-preputial pocket shadow around the Clitoral Glans
                    let moat_dark = (moat_horseshoe * 0.85).clamp(0.0, 1.0);
                    surf = surf * (1.0 - moat_dark) + subpreputial_shadow_col[c] * moat_dark;
                    // Bright vascular coral-rose Clitoral Glans pearl
                    let g_blend = (glans_mask * 0.92).clamp(0.0, 1.0);
                    surf = surf * (1.0 - g_blend) + glans_coral_rose[c] * g_blend;
                    // Everted crimson-rose vaginal mucosal prolapse rosette
                    let prolapse_col = vaginal_prolapse_mucosa[c] * (1.0 - prolapse_crease * 0.70)
                        + prolapse_sulcus_crimson[c] * (prolapse_crease * 0.70);
                    surf =
                        surf * (1.0 - prolapse_mucosa_mask) + prolapse_col * prolapse_mucosa_mask;

                    if is_canal {
                        let t_c = (u_ring / u_rim).powf(2.3);
                        col[c] = deep_canal_col[c] * (1.0 - t_c) + surf * t_c;
                    } else {
                        col[c] = surf;
                    }
                }

                let region_id = if is_canal {
                    0.0
                } else {
                    ((1.0 - in_vestibule * 0.85) * (1.0 - to_majora) + 2.0 * to_majora)
                        * (1.0 - 0.85 * prolapse_mucosa_mask)
                };

                let canal_wet_mask = if is_canal {
                    (u_ring / u_rim).powf(2.4)
                } else {
                    1.0
                };
                let wet_intro = ((-r_from_introitus * 0.30).exp() * (1.0 - to_majora))
                    .max(prolapse_mucosa_mask * 0.98)
                    * canal_wet_mask;
                let wet_vest =
                    (in_vestibule * 0.90 + glans_mask * 0.85 + urethral_cuff_mask * 0.92)
                        * canal_wet_mask;
                let wet_zone = wet_intro.max(wet_vest);
                let meniscus_wet =
                    meniscus_contact * (-r_from_introitus * 1.3).exp() * canal_wet_mask;
                let local_wetness = ((0.90 * wet_zone
                    + 0.32 * pen * (-r_from_introitus * 0.34).exp() * canal_wet_mask
                    + 0.48 * meniscus_wet)
                    + self.lubricant_boost * wet_zone)
                    .clamp(0.02, 0.99);

                let roughness = (0.09 * (1.0 - to_majora) + 0.56 * to_majora)
                    * (1.0 - 0.60 * local_wetness * local_wetness);
                let sss = (0.90 * (1.0 - to_majora) + 0.45 * to_majora) * canal_wet_mask;

                let idx = r * NS + s;
                positions[idx] = Vec3::new(x, y, z);
                colors[idx] = col;
                props[idx] = [region_id, roughness, sss, local_wetness];
                uvs[idx] = [ux, uy];
            }
        }

        // Preserve exact physics kernel telemetry for snapshot logs
        let tube = LabiaTubeInsertion {
            radius_m: (TUBE_RADIUS_MM * 0.001) as f64,
            length_m: (2.0 * TUBE_HALF_LENGTH_MM * 0.001) as f64,
            axis_xy_m: [0.0, (INTROITUS_Y_MM * 0.001) as f64],
            start_tip_z_m: 0.0072,
            max_depth_m: 0.0150,
            frequency_hz: (1.0 / INSERTION_PERIOD_SECONDS) as f64,
        };
        let minora_probe = self
            .dilation_kernel
            .evaluate_point(
                [0.0050, (INTROITUS_Y_MM * 0.001) as f64],
                0.0005,
                &tube,
                time_s as f64,
                0.0,
            )
            .expect("valid minora probe");
        let majora_probe = self
            .dilation_kernel
            .evaluate_point(
                [0.0110, (INTROITUS_Y_MM * 0.001) as f64],
                0.0002,
                &tube,
                time_s as f64,
                0.0,
            )
            .expect("valid majora probe");
        self.minora_separation_mm = (10.0
            + 2000.0 * minora_probe.radial_displacement_xy_m[0].abs() as f32)
            .max(2.0 * orifice_rx);
        self.majora_separation_mm = 22.0 + 2000.0 * majora_probe.radial_displacement_xy_m[0] as f32;

        // Compute smooth outward-facing vertex normals (`dr.cross(ds)` points +Z on skin and inward toward lumen in canal)
        let mut vertices = Vec::with_capacity(NR * NS);
        for r in 0..NR {
            let r_prev = if r > 0 { r - 1 } else { r };
            let r_next = if r + 1 < NR { r + 1 } else { r };
            for s in 0..NS {
                let s_prev = (s + NS - 1) % NS;
                let s_next = (s + 1) % NS;

                let dr = positions[r_next * NS + s] - positions[r_prev * NS + s];
                let ds = positions[r * NS + s_next] - positions[r * NS + s_prev];
                let mut n = dr.cross(ds);
                if n.length_squared() < 1.0e-8 {
                    n = Vec3::Z;
                } else {
                    n = n.normalize();
                }

                let idx = r * NS + s;
                vertices.push(SkinVertex {
                    position: positions[idx].to_array(),
                    normal: n.to_array(),
                    color: colors[idx],
                    tissue_props: props[idx],
                    uv_world: uvs[idx],
                });
            }
        }

        (vertices, kin)
    }
}

fn append_tube_and_meniscus_mesh(
    vertices: &mut Vec<SkinVertex>,
    indices: &mut Vec<u32>,
    kin: Kinematics,
    wetness: f32,
) {
    let r = TUBE_RADIUS_MM;
    let pen = ((2.2 - kin.tip_z_mm) / 9.8).clamp(0.0, 1.0);
    let z_pivot = -1.35 - 0.95 * pen;
    let s_tip = kin.tip_z_mm - z_pivot;

    // Slight inferior approach/withdrawal offset outside the canal so in Frame 0 the resting Vaginal Introitus
    // and in Frame 4 the prolapsed Vaginal Mucosal Rosette are 100% uncovered above the probe tip!
    let withdrawal_extra = smoothstep(0.0, 25.0, kin.velocity_z_mm_s);
    let approach_y_offset = -(3.4 + 1.8 * withdrawal_extra) * smoothstep(1.0, 5.2, s_tip);
    let pivot = Vec3::new(0.0, INTROITUS_Y_MM + approach_y_offset, z_pivot);

    // 26-degree anatomical vaginal insertion axis (enters cleanly from bottom-center of frame)
    let axis_w = Vec3::new(0.0, -0.4384, 0.8988).normalize();
    let axis_u = Vec3::X;
    let axis_v = axis_w.cross(axis_u).normalize();

    let s_cap_center = s_tip + r;
    // Keep s_top at 56.0 mm so the rear end of the cylinder always extends past the bottom edge of the viewport!
    let s_top = 56.0f32;

    let stacks_hemi = 18usize;
    let stacks_cyl = 26usize;
    let slices = 64usize;
    let base_vertex = vertices.len() as u32;
    let ring_stride = (slices + 1) as u32;

    let total_rings = stacks_hemi + stacks_cyl + 1;
    for i in 0..total_rings {
        let (ring_r, s_axial, nw_local) = if i <= stacks_hemi {
            let t = i as f32 / stacks_hemi as f32;
            let phi = t * std::f32::consts::FRAC_PI_2;
            let sin_p = phi.sin();
            let cos_p = phi.cos();
            (r * sin_p, s_cap_center - r * cos_p, -cos_p)
        } else {
            let t = (i - stacks_hemi) as f32 / stacks_cyl as f32;
            let s_a = s_cap_center + t * (s_top - s_cap_center);
            (r, s_a, 0.0)
        };

        for j in 0..=slices {
            let theta = (j as f32 / slices as f32) * std::f32::consts::TAU;
            let cos_t = theta.cos();
            let sin_t = theta.sin();
            let nr_uv = (1.0 - nw_local * nw_local).max(0.0).sqrt();
            let radial_dir = axis_u * cos_t + axis_v * sin_t;
            let normal = (radial_dir * nr_uv + axis_w * nw_local).normalize_or_zero();
            let pos = pivot + radial_dir * ring_r + axis_w * s_axial;

            vertices.push(SkinVertex {
                position: pos.to_array(),
                normal: normal.to_array(),
                color: [0.60, 0.65, 0.70],
                tissue_props: [3.0, 0.06, 0.20, wetness],
                uv_world: [pos.x, pos.y],
            });
        }
    }

    for i in 0..(total_rings - 1) {
        for j in 0..slices {
            let row0 = base_vertex + (i as u32) * ring_stride + (j as u32);
            let row1 = row0 + ring_stride;
            indices.extend_from_slice(&[row0, row1, row0 + 1, row0 + 1, row1, row1 + 1]);
        }
    }
}

struct RenderDevice {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
}

impl RenderDevice {
    async fn new(window: Option<Arc<Window>>) -> Result<Self, Box<dyn Error>> {
        let instance = GraphicsOptions::default().create_instance();
        let surface = match window.as_ref() {
            Some(w) => Some(instance.create_surface(w.clone())?),
            None => None,
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        let surface_config = if let (Some(surf), Some(w)) = (&surface, &window) {
            let size = w.inner_size();
            let caps = surf.get_capabilities(&adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let mut config = surf
                .get_default_config(&adapter, size.width.max(1), size.height.max(1))
                .ok_or("unsupported surface")?;
            config.format = format;
            surf.configure(&device, &config);
            Some(config)
        } else {
            None
        };
        Ok(Self {
            adapter,
            device,
            queue,
            surface,
            surface_config,
        })
    }

    fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }
    fn device(&self) -> &wgpu::Device {
        &self.device
    }
    fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    fn surface(&self) -> Option<&wgpu::Surface<'static>> {
        self.surface.as_ref()
    }
    fn surface_config(&self) -> Option<&wgpu::SurfaceConfiguration> {
        self.surface_config.as_ref()
    }
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            if let (Some(surf), Some(cfg)) = (&self.surface, &mut self.surface_config) {
                cfg.width = width;
                cfg.height = height;
                surf.configure(&self.device, cfg);
            }
        }
    }
}

struct GpuPipeline {
    render_device: RenderDevice,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    skin_indices: Vec<u32>,
}

impl GpuPipeline {
    fn new(window: Option<Arc<Window>>) -> Result<Self, Box<dyn Error>> {
        let render_device = pollster::block_on(RenderDevice::new(window))?;
        println!(
            "Labia Tube Insertion GPU Renderer: {:?}",
            render_device.adapter().get_info()
        );

        let device = render_device.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("labia-tube-insertion-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("labia-tube-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("labia-tube-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("labia-tube-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("labia-tube-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let target_format = render_device
            .surface_config()
            .map(|c| c.format)
            .unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("labia-tube-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SkinVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 24,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 36,
                            shader_location: 3,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 52,
                            shader_location: 4,
                        },
                    ],
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            render_device,
            pipeline,
            uniform_buffer,
            bind_group,
            skin_indices: VulvaSimulation::build_polar_grid_indices(),
        })
    }

    #[allow(deprecated)]
    fn build_scene_buffers(
        &self,
        sim: &mut VulvaSimulation,
        time_s: f32,
        aspect: f32,
        cam_yaw: f32,
        cam_pitch: f32,
        cam_zoom: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32, Kinematics) {
        let (mut vertices, kin) = sim.build_vertices(time_s);
        let mut indices = self.skin_indices.clone();
        let wetness = (0.85 + sim.lubricant_boost).clamp(0.0, 1.0);
        append_tube_and_meniscus_mesh(&mut vertices, &mut indices, kin, wetness);

        let base_dist = 47.0 * cam_zoom;
        let target = Vec3::new(0.0, 0.0, 0.0);
        let eye = target
            + Vec3::new(
                base_dist * cam_yaw.sin() * cam_pitch.cos(),
                -base_dist * cam_yaw.cos() * cam_pitch.cos(),
                base_dist * cam_pitch.sin(),
            );
        let view = Mat4::look_at_rh(eye, target, Vec3::Z);
        let proj = Mat4::perspective_rh(29.0f32.to_radians(), aspect, 1.0, 200.0);
        let view_proj = (proj * view).to_cols_array_2d();

        let uniforms = Uniforms {
            view_proj,
            camera_pos: [eye.x, eye.y, eye.z, 1.0],
            params: [time_s, kin.insertion_phase, wetness, kin.tip_z_mm],
        };
        self.render_device.queue().write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        let device = self.render_device.device();
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("labia-scene-vbuf"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("labia-scene-ibuf"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vbuf, ibuf, indices.len() as u32, kin)
    }

    fn render_offscreen_frame(
        &self,
        sim: &mut VulvaSimulation,
        time_s: f32,
        output_path: &Path,
    ) -> Result<Kinematics, Box<dyn Error>> {
        let (vbuf, ibuf, index_count, kin) =
            self.build_scene_buffers(sim, time_s, WIDTH as f32 / HEIGHT as f32, 0.05, 1.18, 1.0);
        let device = self.render_device.device();
        let queue = self.render_device.queue();

        let color_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("labia-offscreen-color"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("labia-offscreen-depth"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let bytes_per_row = (WIDTH * 4).div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("labia-readback"),
            size: (bytes_per_row * HEIGHT) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("labia-offscreen-enc"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("labia-offscreen-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.16,
                            g: 0.08,
                            b: 0.07,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..index_count, 0, 0..1);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()??;

        let mapped = slice.get_mapped_range()?;
        let mut rgba = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
        for row in 0..HEIGHT {
            let start = (row * bytes_per_row) as usize;
            let end = start + (WIDTH * 4) as usize;
            rgba.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        readback.unmap();

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        image::save_buffer(output_path, &rgba, WIDTH, HEIGHT, image::ColorType::Rgba8)?;
        Ok(kin)
    }
}

struct InteractiveApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuPipeline>,
    sim: VulvaSimulation,
    last_tick: Instant,
    sim_time_s: f32,
    paused: bool,
    speed_scale: f32,
    cam_yaw: f32,
    cam_pitch: f32,
    cam_zoom: f32,
}

impl InteractiveApp {
    fn new() -> Self {
        Self {
            window: None,
            gpu: None,
            sim: VulvaSimulation::new(),
            last_tick: Instant::now(),
            sim_time_s: 0.0,
            paused: false,
            speed_scale: 1.0,
            cam_yaw: 0.05,
            cam_pitch: 1.18,
            cam_zoom: 1.0,
        }
    }
}

impl ApplicationHandler for InteractiveApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(
                "Voxy Engine — Photorealistic Vulva & Vaginal Introitus Tube Insertion [Space: Pause | Arrows/WASD: Orbit/Speed | L: Lubricate | R: Reset]",
            )
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let gpu = GpuPipeline::new(Some(window.clone())).expect("init GPU pipeline");
        self.window = Some(window);
        self.gpu = Some(gpu);
        self.last_tick = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                        PhysicalKey::Code(KeyCode::Space) => {
                            self.paused = !self.paused;
                        }
                        PhysicalKey::Code(KeyCode::ArrowUp) => {
                            if self.paused {
                                self.sim_time_s += 0.025;
                            } else {
                                self.speed_scale = (self.speed_scale + 0.25).min(3.5);
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown) => {
                            if self.paused {
                                self.sim_time_s = (self.sim_time_s - 0.025).max(0.0);
                            } else {
                                self.speed_scale = (self.speed_scale - 0.25).max(0.15);
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::KeyA) => {
                            self.cam_yaw -= 0.08;
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::KeyD) => {
                            self.cam_yaw += 0.08;
                        }
                        PhysicalKey::Code(KeyCode::KeyW) => {
                            self.cam_pitch = (self.cam_pitch + 0.06).min(1.42);
                        }
                        PhysicalKey::Code(KeyCode::KeyS) => {
                            self.cam_pitch = (self.cam_pitch - 0.06).max(0.20);
                        }
                        PhysicalKey::Code(KeyCode::KeyQ) => {
                            self.cam_zoom = (self.cam_zoom * 0.92).max(0.50);
                        }
                        PhysicalKey::Code(KeyCode::KeyE) => {
                            self.cam_zoom = (self.cam_zoom * 1.08).min(1.85);
                        }
                        PhysicalKey::Code(KeyCode::KeyL) => {
                            self.sim.lubricant_boost = (self.sim.lubricant_boost + 0.15).min(0.45);
                        }
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            self.sim = VulvaSimulation::new();
                            self.sim_time_s = 0.0;
                            self.paused = false;
                            self.speed_scale = 1.0;
                            self.cam_yaw = 0.05;
                            self.cam_pitch = 1.18;
                            self.cam_zoom = 1.0;
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.render_device.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_tick).as_secs_f32().min(0.05);
                self.last_tick = now;
                if !self.paused {
                    self.sim_time_s += dt * self.speed_scale;
                }

                if let (Some(gpu), Some(window)) = (&mut self.gpu, &self.window) {
                    let size = window.inner_size();
                    if size.width > 0 && size.height > 0 {
                        let aspect = size.width as f32 / size.height as f32;
                        let (vbuf, ibuf, index_count, _) = gpu.build_scene_buffers(
                            &mut self.sim,
                            self.sim_time_s,
                            aspect,
                            self.cam_yaw,
                            self.cam_pitch,
                            self.cam_zoom,
                        );
                        if let Some(surface) = gpu.render_device.surface() {
                            if let wgpu::CurrentSurfaceTexture::Success(frame)
                            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) =
                                surface.get_current_texture()
                            {
                                let view = frame
                                    .texture
                                    .create_view(&wgpu::TextureViewDescriptor::default());
                                let device = gpu.render_device.device();
                                let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
                                    label: Some("labia-win-depth"),
                                    size: wgpu::Extent3d {
                                        width: size.width,
                                        height: size.height,
                                        depth_or_array_layers: 1,
                                    },
                                    mip_level_count: 1,
                                    sample_count: 1,
                                    dimension: wgpu::TextureDimension::D2,
                                    format: wgpu::TextureFormat::Depth32Float,
                                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                    view_formats: &[],
                                });
                                let depth_view =
                                    depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
                                let mut encoder = device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor {
                                        label: Some("labia-win-enc"),
                                    },
                                );
                                {
                                    let mut pass =
                                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                            label: Some("labia-win-pass"),
                                            color_attachments: &[Some(
                                                wgpu::RenderPassColorAttachment {
                                                    view: &view,
                                                    resolve_target: None,
                                                    ops: wgpu::Operations {
                                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                                            r: 0.16,
                                                            g: 0.08,
                                                            b: 0.07,
                                                            a: 1.0,
                                                        }),
                                                        store: wgpu::StoreOp::Store,
                                                    },
                                                    depth_slice: None,
                                                },
                                            )],
                                            depth_stencil_attachment: Some(
                                                wgpu::RenderPassDepthStencilAttachment {
                                                    view: &depth_view,
                                                    depth_ops: Some(wgpu::Operations {
                                                        load: wgpu::LoadOp::Clear(1.0),
                                                        store: wgpu::StoreOp::Store,
                                                    }),
                                                    stencil_ops: None,
                                                },
                                            ),
                                            timestamp_writes: None,
                                            occlusion_query_set: None,
                                            multiview_mask: None,
                                        });
                                    pass.set_pipeline(&gpu.pipeline);
                                    pass.set_bind_group(0, &gpu.bind_group, &[]);
                                    pass.set_vertex_buffer(0, vbuf.slice(..));
                                    pass.set_index_buffer(
                                        ibuf.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    pass.draw_indexed(0..index_count, 0, 0..1);
                                }
                                gpu.render_device.queue().submit(Some(encoder.finish()));
                                gpu.render_device.queue().present(frame);
                            }
                        }
                    }
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--window") {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        let mut app = InteractiveApp::new();
        event_loop.run_app(&mut app)?;
        return Ok(());
    }

    let gpu = GpuPipeline::new(None)?;
    let mut sim = VulvaSimulation::new();

    for (frame_idx, time_s, label) in SNAPSHOT_FRAMES {
        let frame_path = format!("/private/tmp/voxy-labia-tube-frame-{frame_idx}.png");
        let kin = gpu.render_offscreen_frame(&mut sim, time_s, Path::new(&frame_path))?;
        println!(
            "frame={frame_idx} ({label}) | t={time_s:.2}s phase={:.2} tip_z={:.2}mm vel_z={:.1}mm/s | minora_sep={:.2}mm majora_sep={:.2}mm",
            kin.insertion_phase,
            kin.tip_z_mm,
            kin.velocity_z_mm_s,
            sim.minora_separation_mm,
            sim.majora_separation_mm,
        );
        if frame_idx == 3 {
            std::fs::copy(&frame_path, DEFAULT_OUTPUT_PATH)?;
        }
    }

    println!("LABIA TUBE INSERTION SNAPSHOT PASS: {DEFAULT_OUTPUT_PATH}");
    Ok(())
}
