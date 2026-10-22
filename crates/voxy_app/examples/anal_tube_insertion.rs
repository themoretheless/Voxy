//▫~•◦------------------------------------------------------------------------------------◦•~▫//
// █████████████████████████████████████ VOXY ENGINE █████████████████████████████████████ //
//▫~•◦------------------------------------------------------------------------------------◦•~▫//

//! Photorealistic anatomical anal orifice & sphincter tube insertion simulation (`voxy_app`).
//!
//! Features:
//! - Seamless 360-degree concentric polar topology (`128` radial rings × `144` angular sectors)
//!   around the anal canal & verge — zero vertical seam or Cartesian slit artifacts.
//! - Tightly closed, puckered resting orifice (`r_rest ~ 0.22 mm`) with 11 plump, convex fleshy
//!   radiating cushions (`plicae radiatae`) crossed by concentric `corrugator cutis ani`
//!   circumferential puckers and narrow, deeply shadowed sulcal creases.
//! - True anatomical canal occlusion (`u_rim = 0.14`): the anal verge collar stays at `z ~ -1.5 mm`
//!   while the internal anal canal plunges to `z = -14.5 mm`, so the penetrating cylindrical tube
//!   shaft disappears genuinely inside the body with its tip occluded by the anterior sphincter rim.
//! - Realistic 3-zone pigmentation (warm peachy-tan gluteal dermis -> hyperpigmented perianal
//!   brownish-rose rosette -> vascular crimson-pink pecten & mucosal verge) with crisp Voronoi pores.

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

/// Radial rings from deep inside the anal canal (`r = 0`) to the outer gluteal skin (`r = NR - 1`).
const NR: usize = 160;
/// Angular sectors around the 360-degree circumference (`s = 0 .. NS - 1`).
const NS: usize = 180;

const TUBE_RADIUS_MM: f32 = 3.80;
const TUBE_HALF_LENGTH_MM: f32 = 11.5;
const INSERTION_PERIOD_SECONDS: f32 = 0.88;

const DEFAULT_OUTPUT_PATH: &str = "/private/tmp/voxy-anal-tube-insertion.png";

const SNAPSHOT_FRAMES: [(usize, f32, &str); 5] = [
    (0, 0.05, "approach-puckered-anal-sphincter"),
    (1, 0.18, "verge-contact-lubricant-meniscus"),
    (2, 0.32, "sphincter-dilation-friction-invagination"),
    (3, 0.50, "full-depth-sphincter-grip-cuff"),
    (4, 0.72, "withdrawal-rectal-mucosal-rosette-prolapse"),
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
    uv_world: [f32; 2],     // anatomical rest coordinates (mm) for 2D Voronoi pores & sulci
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

// Returns (d1_nearest_pore, d2_minus_d1_sulcus_ridge)
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

fn skin_micro_height(uv_mm: vec2<f32>, region_id: f32) -> f32 {
    // 1. Subtle follicular pore dimples + soft multi-octave dermal micro-relief (no polygonal cracks)
    let v_coarse = voronoi_skin(uv_mm * 3.4);
    let v_fine = voronoi_skin(uv_mm * 6.8 + vec2<f32>(7.3, 11.9));
    let pore_pit = -smoothstep(0.16, 0.0, v_coarse.x) * 0.32
                   -smoothstep(0.14, 0.0, v_fine.x) * 0.16;
    let dermal_grain = fbm2(uv_mm * 1.9) * 0.28 + fbm2(uv_mm * 4.3 + vec2<f32>(3.1, 8.4)) * 0.14;

    // 2. Perianal radial + concentric micro-crinkles (corrugator cutis ani)
    let r = max(length(uv_mm), 0.001);
    let dir = uv_mm / r;
    let ortho = vec2<f32>(-dir.y, dir.x);
    let radial_pleats = fbm2(vec2<f32>(dot(uv_mm, ortho) * 3.4, r * 0.70)) * 0.42;
    let concentric_crinkles = sin(r * 3.2 + fbm2(uv_mm * 0.75) * 3.0) * 0.16 * exp(-r * 0.085);

    let w_mucosa = clamp(1.0 - region_id, 0.0, 1.0);
    let w_outer = clamp(region_id - 1.0, 0.0, 1.0);
    let w_perianal = clamp(1.0 - w_mucosa - w_outer, 0.0, 1.0);

    let h_mucosa = radial_pleats * 0.75 + concentric_crinkles * 0.85;
    let h_perianal = radial_pleats * 0.65 + concentric_crinkles + dermal_grain * 0.85 + pore_pit * 0.45;
    let h_outer = dermal_grain + pore_pit * 0.95;

    return h_mucosa * w_mucosa + h_perianal * w_perianal + h_outer * w_outer;
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

        // Subtle warm tissue reflection inside the lower probe tip near the orifice
        let tissue_bleed = exp(-max(in.world_pos.z + 1.5, 0.0) * 0.28) * vec3<f32>(0.22, 0.06, 0.06);
        let base_col = (in.color + tissue_bleed * 0.35) * (0.22 + 0.52 * diff1 + 0.24 * diff2 + 0.18 * wrap_probe)
            + vec3<f32>(0.95, 0.97, 1.0) * (spec1 + spec2 + spec3 + clearcoat + fresnel * 0.16);
        let mapped = pow(aces_tonemap(base_col), vec3<f32>(1.0 / 2.2));
        return vec4<f32>(mapped, 1.0);
    }

    // Epidermal & perianal micro-normal perturbation
    let uv = in.uv_world;
    let eps = 0.045;
    let h0 = skin_micro_height(uv, region_id);
    let hx = skin_micro_height(uv + vec2<f32>(eps, 0.0), region_id);
    let hy = skin_micro_height(uv + vec2<f32>(0.0, eps), region_id);
    let surface_up = clamp(N.z, 0.0, 1.0);
    let bump_atten = mix(1.0, 0.32, wetness * wetness) * (0.25 + 0.75 * surface_up);
    let micro_nx = -(hx - h0) / eps * 0.048 * bump_atten;
    let micro_ny = -(hy - h0) / eps * 0.048 * bump_atten;
    N = normalize(N + vec3<f32>(micro_nx, micro_ny, 0.0));

    // Natural melanin & subdermal hemoglobin micro-mottling
    let mottle_low = fbm2(uv * 0.22);
    let mottle_med = fbm2(uv * 0.85 + vec2<f32>(4.2, 8.7));
    let v_pore = voronoi_skin(uv * 3.4);
    let pore_dark = smoothstep(0.15, 0.0, v_pore.x);

    var albedo = in.color;
    // Gentle pore darkening without any polygonal edge lines
    albedo *= (1.0 - pore_dark * 0.10);
    // Warm subdermal capillary & melanin variation
    albedo += vec3<f32>(0.022, -0.005, -0.006) * mottle_low
            + vec3<f32>(0.012, 0.003, 0.002) * mottle_med;
    // Wetness slightly deepens and saturates tissue pigment where lubricated
    let lube_mask = wetness * wetness;
    albedo = mix(albedo, albedo * vec3<f32>(0.82, 0.70, 0.66), lube_mask * 0.42);
    albedo = clamp(albedo, vec3<f32>(0.015), vec3<f32>(0.92));

    // Dual-lobe subsurface scattering (rich dermal hemoglobin red-orange scatter)
    let n_dot_l1 = max(dot(N, L1), 0.0);
    let n_dot_l2 = max(dot(N, L2), 0.0);
    let n_dot_l3 = max(dot(N, L3), 0.0);
    let wrap1 = max((dot(N, L1) + 0.55) / 1.55, 0.0);
    let wrap2 = max((dot(N, L2) + 0.48) / 1.48, 0.0);
    let sss_blood = vec3<f32>(0.68, 0.11, 0.07);
    let sss_contrib = sss_blood * sss_strength * (
        0.42 * pow(wrap1, 1.55) * (1.0 - 0.55 * n_dot_l1) +
        0.24 * pow(wrap2, 1.70) * (1.0 - 0.55 * n_dot_l2)
    );

    // Crease self-occlusion in deep anal sulci & internal canal entrance
    let r_xy = length(in.world_pos.xy);
    let depth_ao = clamp((in.world_pos.z + 5.2) / 5.2, 0.10, 1.0);
    let center_ao = mix(0.24, 1.0, smoothstep(0.18, 4.0, r_xy));
    let ao = clamp(depth_ao * center_ao, 0.10, 1.0);

    let diffuse = albedo * (0.18 * ao + 0.56 * n_dot_l1 + 0.26 * n_dot_l2 + 0.10 * n_dot_l3)
        + sss_contrib * ao;

    // Dual-lobe specular: soft lipid sheen on dry skin + sharp capillary clearcoat on wet verge
    let H1 = normalize(L1 + V);
    let H2 = normalize(L2 + V);
    let H3 = normalize(L3 + V);
    let n_dot_v = max(dot(N, V), 0.0);
    let fresnel = 0.028 + 0.972 * pow(1.0 - n_dot_v, 5.0);

    let rough = clamp(mix(base_roughness, 0.05, lube_mask), 0.04, 0.92);
    let spec_exp = mix(380.0, 14.0, rough * rough);
    let pore_spec_mask = 1.0 - pore_dark * 0.55;

    let dry_lobe = (
        pow(max(dot(N, H1), 0.0), spec_exp) * 0.14 +
        pow(max(dot(N, H2), 0.0), spec_exp * 0.7) * 0.08 +
        pow(max(dot(N, H3), 0.0), 20.0) * 0.04
    ) * pore_spec_mask * ao;

    let wet_clearcoat = (
        pow(max(dot(N, H1), 0.0), 540.0) * 1.95 +
        pow(max(dot(N, H2), 0.0), 320.0) * 0.95 +
        pow(max(dot(N, H3), 0.0), 180.0) * 0.45
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

struct AnalSimulation {
    dilation_kernel: OrificeDilationKernel,
    sphincter_separation_mm: f32,
    gluteal_separation_mm: f32,
    lubricant_boost: f32,
}

impl AnalSimulation {
    fn new() -> Self {
        Self {
            dilation_kernel: OrificeDilationKernel::anal_sphincter_default(),
            sphincter_separation_mm: 9.0,
            gluteal_separation_mm: 22.0,
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
        // u in [0.0, 0.5]: downward penetration; u in [0.5, 1.0]: upward withdrawal.
        // At frame 3 (t = 0.50 s, u = 0.568), prolapse_t = 0.37 (initial mucosal cuff eversion around shaft);
        // At frame 4 (t = 0.72 s, u = 0.818), prolapse_t = 1.00 (peak rectal mucosal rosette prolapse!).
        let cycle_u = (time_s / INSERTION_PERIOD_SECONDS).rem_euclid(1.0);
        let prolapse_t = smoothstep(0.46, 0.72, cycle_u) * (1.0 - smoothstep(0.88, 0.98, cycle_u));

        // Exact cross-sectional radius of the hemispherical/cylindrical tube at the anal verge rim height
        let z_rim_est = -1.35 - 1.10 * pen + 1.15 * prolapse_t * (1.0 - pen);
        let cap_center_z = kin.tip_z_mm + TUBE_RADIUS_MM;
        let r_tube_at_rim = if kin.tip_z_mm >= z_rim_est {
            0.0
        } else if z_rim_est < cap_center_z {
            let dz = cap_center_z - z_rim_est;
            (TUBE_RADIUS_MM * TUBE_RADIUS_MM - dz * dz).max(0.0).sqrt()
        } else {
            TUBE_RADIUS_MM
        };

        let pre_dilation = 0.45 * smoothstep(3.0, -0.5, kin.tip_z_mm) * (1.0 - prolapse_t);
        // During withdrawal prolapse (frame 4), the everted rectal mucosal rosette stays patulous/gaping
        // (`prolapse_lumen_r ~ 1.75 mm`) completely clear below the departing probe tip!
        let prolapse_lumen_r = 1.75 * prolapse_t;
        let orifice_r_rest = 0.18;
        let orifice_r = (orifice_r_rest + pre_dilation)
            .max(r_tube_at_rim - 0.05)
            .max(prolapse_lumen_r)
            .min(TUBE_RADIUS_MM - 0.04);
        let tube_contact_t =
            ((orifice_r - orifice_r_rest) / (TUBE_RADIUS_MM - orifice_r_rest)).clamp(0.0, 1.0);

        // Anal rugae (plicae radiatae) stretch out as the sphincter dilates
        let rugae_effacement = 1.0 - 0.68 * pen - 0.35 * prolapse_t * (1.0 - pen);

        // Axial shear drag from tube motion (invagination on insertion, cuff eversion on withdrawal)
        let shear_dir = (-kin.velocity_z_mm_s / 42.0).clamp(-1.0, 1.0);

        // Continuous capillary lubricant meniscus strength when the tube contacts the verge
        let meniscus_contact = smoothstep(2.4, -1.2, kin.tip_z_mm).max(0.75 * prolapse_t);

        let mut positions = vec![Vec3::ZERO; NR * NS];
        let mut colors = vec![[0.0f32; 3]; NR * NS];
        let mut props = vec![[0.0f32; 4]; NR * NS];
        let mut uvs = vec![[0.0f32; 2]; NR * NS];

        // u_rim separates the internal vertical anal canal (u <= u_rim) from the external perianal skin (u > u_rim)
        let u_rim = 0.14;

        for r in 0..NR {
            let u_ring = r as f32 / (NR - 1) as f32;

            for s in 0..NS {
                let theta = (s as f32 / NS as f32) * std::f32::consts::TAU;
                let cos_t = theta.cos();
                let sin_t = theta.sin();

                // Subtle organic asymmetry without pinwheel swirl
                let theta_org =
                    theta + 0.055 * (2.0 * theta + 0.6).sin() + 0.035 * (3.0 * theta - 0.9).cos();

                // Plump 6-cushion internal hemorrhoidal/rectal mucosal rosette profile
                // (wide convex lobes separated by narrow sulcal dimples — never pointy spikes!)
                let u_lobe_6 = (0.5 - 0.5 * (6.0 * theta_org - 0.35).cos()).clamp(0.0, 1.0);
                let u_lobe_12 = (0.5 - 0.5 * (12.0 * theta_org + 0.80).cos()).clamp(0.0, 1.0);
                let prolapse_sulcus = 0.72 * u_lobe_6.powf(2.4) + 0.28 * u_lobe_12.powf(2.4);
                let prolapse_cushion_profile = 1.0 - 0.26 * prolapse_sulcus;

                let (r_2d, r_rest_2d, r_from_rim, is_canal) = if u_ring <= u_rim {
                    let t_canal = u_ring / u_rim; // 0 at deep canal bottom, 1 at anal verge rim
                    let r_c = (orifice_r * (0.35 + 0.65 * t_canal)).max(0.06);
                    let r_rest_c = (orifice_r_rest * (0.35 + 0.65 * t_canal)).max(0.06);
                    (r_c, r_rest_c, 0.0, true)
                } else {
                    let u_ext = (u_ring - u_rim) / (1.0 - u_rim);
                    let r_max = 38.0;
                    let r_ext = orifice_r + (r_max - orifice_r) * u_ext.powf(1.24);
                    let r_rest_ext = orifice_r_rest + (r_max - orifice_r_rest) * u_ext.powf(1.24);
                    (r_ext, r_rest_ext, (r_ext - orifice_r).max(0.0), false)
                };

                // Plump, rounded radiating cushions (plicae radiatae) separated by narrow deep sulci
                // that run continuously from the outer sphincter all the way into the central verge!
                let u10 = (0.5 - 0.5 * (10.0 * theta_org).cos()).clamp(0.0, 1.0);
                let u7 = (0.5 - 0.5 * (7.0 * theta_org + 0.85).cos()).clamp(0.0, 1.0);
                let u15 = (0.5 - 0.5 * (15.0 * theta_org + 1.4).cos()).clamp(0.0, 1.0);
                let cos_5 = (5.0 * theta_org - 0.55).cos();

                let s10 = u10.powf(2.35);
                let s7 = u7.powf(2.15);
                let s15 = u15.powf(2.40);

                let cushion_10 = 0.62 - 1.55 * s10;
                let cushion_7 = 0.55 - 1.35 * s7;
                let cushion_15 = 0.45 - 1.15 * s15;

                let asym = 1.0 + 0.22 * cos_5 + 0.14 * (3.0 * theta_org + 0.4).sin();

                // Concentric circumferential puckers from subcutaneous corrugator cutis ani & sphincter bundles
                let concentric_pucker = (r_from_rim * 2.85 + 0.55 * cos_5).sin()
                    * 0.18
                    * smoothstep(0.4, 1.8, r_from_rim)
                    * (-r_from_rim * 0.14).exp();

                // Continuous inner envelope (`0.35 + 0.65 * smoothstep`) so radiating folds converge all the way
                // into the central anal orifice without splitting into two concentric tiers!
                let env_primary =
                    (0.35 + 0.65 * smoothstep(0.0, 1.4, r_from_rim)) * (-r_from_rim * 0.145).exp();
                let env_secondary = smoothstep(0.0, 1.0, r_from_rim) * (-r_from_rim * 0.23).exp();

                let collar_stretch_suppress =
                    1.0 - 0.85 * tube_contact_t * (-r_from_rim * 0.75).exp();

                let rugae_z = (1.15 * cushion_10 * env_primary
                    + 0.54 * cushion_7 * env_primary
                    + 0.22 * cushion_15 * env_secondary)
                    * asym
                    * rugae_effacement
                    * (1.0 - 0.55 * prolapse_t * (-r_from_rim * 0.45).exp())
                    + concentric_pucker * (0.5 + 0.5 * rugae_effacement);

                // Smooth toroidal radial lip fullness of the everted rectal mucosal doughnut during prolapse
                let prolapse_torus_env = (-((r_from_rim - 0.95) / 1.25).powi(2)).exp();
                let prolapse_radial_bulge =
                    0.55 * prolapse_t * prolapse_torus_env * prolapse_cushion_profile;

                let fold_r_mod =
                    0.10 * cushion_10 * env_secondary * rugae_effacement * collar_stretch_suppress
                        + prolapse_radial_bulge;

                let r_final = (r_2d + fold_r_mod).max(0.06);
                let x = r_final * cos_t;
                let y = r_final * sin_t;

                // Anatomical rest UV in mm
                let ux = r_rest_2d * cos_t;
                let uy = r_rest_2d * sin_t;

                // Smooth C^2 gluteal cheeks (+X and -X) and intergluteal cleft valley along Y
                let x_sym = (x * x + 0.5).sqrt();
                let gluteal_mound = 5.6 * (1.0 - (-x_sym * x_sym * 0.0028).exp());
                let cleft_valley = -0.65 * (-x * x * 0.011).exp();
                let ap_curvature = -0.0024 * y * y + 0.020 * y;

                // Perianal rosette dimple + smooth funnel roll-in at the anal verge rim
                let rosette_recess = -0.65 * (-r_from_rim * 0.20).exp() * (1.0 - 0.55 * prolapse_t);
                let verge_roll_in = -0.55 * (-r_from_rim * 0.85).exp() * (1.0 - 0.75 * prolapse_t);

                // ── Prominent Internal & External Anal Sphincter Muscular Architecture ──
                // 1. External Anal Sphincter (`m. sphincter ani externus`, pars subcutanea & superficialis):
                //    Broad, plump muscular ring mound around the anal verge (`r_from_rim ~ 1.5..7.5 mm`)
                //    with subtle concentric muscle bundle ripples and a crisp outer sphincter sulcus.
                let ext_sph_center = 3.6 + 0.55 * prolapse_t;
                let ext_sph_env = (-((r_from_rim - ext_sph_center) / 2.65).powi(2)).exp();
                let sphincter_bundle_ripples = 0.16
                    * ((r_from_rim - 1.8) * 2.35).cos()
                    * smoothstep(1.4, 2.5, r_from_rim)
                    * (1.0 - smoothstep(6.0, 8.2, r_from_rim));
                let external_sphincter_torus = (1.55 * ext_sph_env + sphincter_bundle_ripples)
                    * (1.0 + 0.28 * pen + 0.35 * prolapse_t);

                // 2. Intersphincteric Groove (`sulcus intersphinctericus` at the neck of the prolapsed rosette):
                //    Subtle at rest (-0.18 mm) so radiating folds stay continuous, deepening (-0.68 mm)
                //    when the sphincter constricts around the base of the prolapsed mucosal rosette!
                let intersphincteric_groove = -(0.18 + 0.52 * prolapse_t + 0.18 * pen)
                    * (-((r_from_rim - 2.35) / 0.58).powi(2)).exp();

                // ── True Rectal Mucosal Rosette Prolapse (`пролапс слизистой розетки прямой кишки`) ──
                // During late penetration (frame 3) and peaking on withdrawal (frame 4, prolapse_t = 1.0),
                // the internal crimson rectal mucosa rolls outward through the constricted sphincter collar
                // into a plump, bulbous, rolled toroidal doughnut (+2.20 mm) completely clear of the probe tip!
                let torus_dome = (0.55 * (-r_from_rim * 0.85).exp() + prolapse_torus_env)
                    * prolapse_cushion_profile;
                let prolapse_concentric_rugae =
                    0.14 * (r_from_rim * 6.2 - 0.6).sin() * (-r_from_rim * 0.55).exp();
                let mucosal_prolapse_rosette =
                    prolapse_t * (1.85 * torus_dome + prolapse_concentric_rugae);

                // Dynamic axial response at the anal verge collar (invagination on entry, cuff eversion on exit)
                let verge_collar_env = (-r_from_rim * 0.36).exp();
                let invagination = -1.35
                    * pen
                    * (1.0 - 0.65 * prolapse_t)
                    * verge_collar_env
                    * (0.65 + 0.35 * shear_dir.max(0.0));
                let withdrawal_cuff = 1.45
                    * (-shear_dir).max(0.0)
                    * pen
                    * (-((r_from_rim - 0.75).powi(2)) / 2.2).exp();

                // Seamless capillary lubricant meniscus lip right at the tube-tissue contact rim
                let meniscus_lip = 0.58 * meniscus_contact * (-r_from_rim * 1.9).exp();

                // Unified surface height guarantees 100% C^1 continuity across u_ring == u_rim
                let z_surface = gluteal_mound
                    + cleft_valley
                    + ap_curvature
                    + rosette_recess
                    + verge_roll_in
                    + intersphincteric_groove
                    + external_sphincter_torus
                    + rugae_z
                    + mucosal_prolapse_rosette
                    + invagination
                    + withdrawal_cuff
                    + meniscus_lip;

                let z = if is_canal {
                    let t_canal = u_ring / u_rim; // 0 at bottom, 1 at rim
                    let canal_drop = -12.5 * (1.0 - t_canal).powf(1.38);
                    z_surface + canal_drop
                } else {
                    z_surface
                };

                // Crease factor (1 inside deep narrow sulcal grooves, 0 on plump cushion crowns)
                let crease_factor = (0.68 * s10 + 0.32 * s7).clamp(0.0, 1.0) * env_primary;

                // True linear-space (sRGB^2.2) human skin, sphincter & prolapsed rectal mucosa pigmentation
                let outer_skin = [0.50f32, 0.25, 0.165];
                let perianal_pigment = [0.21f32, 0.078, 0.058];
                let sphincter_muscle_tint = [0.26f32, 0.086, 0.070];
                let sulcus_pigment = [0.09f32, 0.024, 0.019];
                let pecten_mucosa = [0.35f32, 0.066, 0.070];
                let rectal_prolapse_mucosa = [0.44f32, 0.042, 0.052];
                let prolapse_sulcus_crimson = [0.16f32, 0.015, 0.020];
                let deep_canal_col = [0.02f32, 0.003, 0.004];

                let pig_t = smoothstep(3.2, 16.5, r_from_rim);
                let verge_t = smoothstep(0.0, 2.8, r_from_rim);
                // When prolapse_t > 0, the everted internal rectal mucosa rolls out to the intersphincteric groove (r ~ 2.25 mm)
                let prolapse_mucosa_mask = prolapse_t * (1.0 - smoothstep(1.35, 2.35, r_from_rim));
                let prolapse_crease = prolapse_sulcus.clamp(0.0, 1.0);

                let mut col = [0.0f32; 3];
                for c in 0..3 {
                    let perianal = perianal_pigment[c] * (1.0 - crease_factor * 0.65)
                        + sulcus_pigment[c] * (crease_factor * 0.65);
                    let with_sphincter = perianal * (1.0 - ext_sph_env * 0.40)
                        + sphincter_muscle_tint[c] * (ext_sph_env * 0.40);
                    let skin_blend = with_sphincter * (1.0 - pig_t) + outer_skin[c] * pig_t;
                    let mut surf_col = pecten_mucosa[c] * (1.0 - verge_t) + skin_blend * verge_t;
                    let prolapse_col = rectal_prolapse_mucosa[c] * (1.0 - prolapse_crease * 0.70)
                        + prolapse_sulcus_crimson[c] * (prolapse_crease * 0.70);
                    surf_col = surf_col * (1.0 - prolapse_mucosa_mask)
                        + prolapse_col * prolapse_mucosa_mask;
                    if is_canal {
                        let t_c = (u_ring / u_rim).powf(2.2);
                        col[c] = deep_canal_col[c] * (1.0 - t_c) + surf_col * t_c;
                    } else {
                        col[c] = surf_col;
                    }
                }

                // Smooth continuous region_id in 0.0..2.0 (zero discrete triangle jumps)
                let region_id = if is_canal {
                    0.0
                } else {
                    (smoothstep(0.8, 3.8, r_from_rim) + smoothstep(8.5, 18.0, r_from_rim))
                        * (1.0 - 0.85 * prolapse_mucosa_mask)
                };

                // Strict wetness localization: glistening wet on the anal verge, capillary meniscus,
                // and everted rectal mucosal prolapse rosette; natural satin/matte on outer gluteal skin
                let canal_wet_mask = if is_canal {
                    (u_ring / u_rim).powf(2.5)
                } else {
                    1.0
                };
                let wet_zone =
                    ((-r_from_rim * 0.28).exp().max(prolapse_mucosa_mask * 0.98)) * canal_wet_mask;
                let meniscus_wet = meniscus_contact * (-r_from_rim * 1.3).exp() * canal_wet_mask;
                let local_wetness = ((0.88 * wet_zone
                    + 0.35 * pen * (-r_from_rim * 0.38).exp() * canal_wet_mask
                    + 0.48 * meniscus_wet)
                    + self.lubricant_boost * wet_zone)
                    .clamp(0.02, 0.99);
                let roughness = (0.10 * (1.0 - pig_t) + 0.56 * pig_t)
                    * (1.0 - 0.62 * local_wetness * local_wetness);
                let sss = (0.90 * (1.0 - pig_t) + 0.45 * pig_t) * canal_wet_mask;

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
            axis_xy_m: [0.0, 0.0],
            start_tip_z_m: 0.0072,
            max_depth_m: 0.0150,
            frequency_hz: (1.0 / INSERTION_PERIOD_SECONDS) as f64,
        };
        let inner_probe = self
            .dilation_kernel
            .evaluate_point([0.0045, 0.0], 0.0005, &tube, time_s as f64, 0.0)
            .expect("valid inner probe");
        let outer_probe = self
            .dilation_kernel
            .evaluate_point([0.0110, 0.0], 0.0002, &tube, time_s as f64, 0.0)
            .expect("valid outer probe");
        self.sphincter_separation_mm = (9.0
            + 2000.0 * inner_probe.radial_displacement_xy_m[0].abs() as f32)
            .max(2.0 * orifice_r);
        self.gluteal_separation_mm = 22.0 + 2000.0 * outer_probe.radial_displacement_xy_m[0] as f32;

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
    let withdrawal_lift =
        2.45 * smoothstep(1.0, 3.0, kin.tip_z_mm) * smoothstep(0.0, 25.0, kin.velocity_z_mm_s);
    let tip_z = kin.tip_z_mm + withdrawal_lift;
    let r = TUBE_RADIUS_MM;
    let cyl_top_z = tip_z + 2.0 * TUBE_HALF_LENGTH_MM;
    let cap_center_z = tip_z + r;

    let stacks_hemi = 18usize;
    let stacks_cyl = 26usize;
    let slices = 64usize;
    let base_vertex = vertices.len() as u32;
    let ring_stride = (slices + 1) as u32;

    let total_rings = stacks_hemi + stacks_cyl + 1;
    for i in 0..total_rings {
        let (ring_r, z, nz_local) = if i <= stacks_hemi {
            let t = i as f32 / stacks_hemi as f32;
            let phi = t * std::f32::consts::FRAC_PI_2;
            let sin_p = phi.sin();
            let cos_p = phi.cos();
            (r * sin_p, cap_center_z - r * cos_p, -cos_p)
        } else {
            let t = (i - stacks_hemi) as f32 / stacks_cyl as f32;
            let z = cap_center_z + t * (cyl_top_z - cap_center_z);
            (r, z, 0.0)
        };

        for j in 0..=slices {
            let theta = (j as f32 / slices as f32) * std::f32::consts::TAU;
            let cos_t = theta.cos();
            let sin_t = theta.sin();
            let nr_xy = (1.0 - nz_local * nz_local).max(0.0).sqrt();
            let normal = Vec3::new(nr_xy * cos_t, nr_xy * sin_t, nz_local).normalize_or_zero();
            let pos = Vec3::new(ring_r * cos_t, ring_r * sin_t, z);

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
            "Anal Tube Insertion GPU Renderer: {:?}",
            render_device.adapter().get_info()
        );

        let device = render_device.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("anal-tube-insertion-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("anal-tube-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("anal-tube-bgl"),
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
            label: Some("anal-tube-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("anal-tube-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let target_format = render_device
            .surface_config()
            .map(|c| c.format)
            .unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("anal-tube-pipeline"),
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
            skin_indices: AnalSimulation::build_polar_grid_indices(),
        })
    }

    #[allow(deprecated)]
    fn build_scene_buffers(
        &self,
        sim: &mut AnalSimulation,
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

        let base_dist = 42.5 * cam_zoom;
        let eye = Vec3::new(
            base_dist * cam_yaw.sin() * cam_pitch.cos(),
            -base_dist * cam_yaw.cos() * cam_pitch.cos(),
            base_dist * cam_pitch.sin(),
        );
        let target = Vec3::new(0.0, -0.5, -1.2);
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
            label: Some("anal-scene-vbuf"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("anal-scene-ibuf"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vbuf, ibuf, indices.len() as u32, kin)
    }

    fn render_offscreen_frame(
        &self,
        sim: &mut AnalSimulation,
        time_s: f32,
        output_path: &Path,
    ) -> Result<Kinematics, Box<dyn Error>> {
        let (vbuf, ibuf, index_count, kin) =
            self.build_scene_buffers(sim, time_s, WIDTH as f32 / HEIGHT as f32, 0.0, 0.73, 1.0);
        let device = self.render_device.device();
        let queue = self.render_device.queue();

        let color_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("anal-offscreen-color"),
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
            label: Some("anal-offscreen-depth"),
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
            label: Some("anal-readback"),
            size: (bytes_per_row * HEIGHT) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("anal-offscreen-enc"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("anal-offscreen-pass"),
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
    sim: AnalSimulation,
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
            sim: AnalSimulation::new(),
            last_tick: Instant::now(),
            sim_time_s: 0.0,
            paused: false,
            speed_scale: 1.0,
            cam_yaw: 0.0,
            cam_pitch: 0.73,
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
                "Voxy Engine — Photorealistic Anal Orifice & Sphincter Tube Insertion [Space: Pause | Arrows/WASD: Orbit/Speed | L: Lubricate | R: Reset]",
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
                            self.sim = AnalSimulation::new();
                            self.sim_time_s = 0.0;
                            self.paused = false;
                            self.speed_scale = 1.0;
                            self.cam_yaw = 0.0;
                            self.cam_pitch = 0.73;
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
                                    label: Some("anal-win-depth"),
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
                                        label: Some("anal-win-enc"),
                                    },
                                );
                                {
                                    let mut pass =
                                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                            label: Some("anal-win-pass"),
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
    let mut sim = AnalSimulation::new();

    for (frame_idx, time_s, label) in SNAPSHOT_FRAMES {
        let frame_path = format!("/private/tmp/voxy-anal-tube-frame-{frame_idx}.png");
        let kin = gpu.render_offscreen_frame(&mut sim, time_s, Path::new(&frame_path))?;
        println!(
            "frame={frame_idx} ({label}) | t={time_s:.2}s phase={:.2} tip_z={:.2}mm vel_z={:.1}mm/s | sphincter_sep={:.2}mm gluteal_sep={:.2}mm",
            kin.insertion_phase,
            kin.tip_z_mm,
            kin.velocity_z_mm_s,
            sim.sphincter_separation_mm,
            sim.gluteal_separation_mm,
        );
        if frame_idx == 3 {
            std::fs::copy(&frame_path, DEFAULT_OUTPUT_PATH)?;
        }
    }

    println!("ANAL TUBE INSERTION SNAPSHOT PASS: {DEFAULT_OUTPUT_PATH}");
    Ok(())
}
