// Anatomically detailed 3D female pelvic floor, vulvar/vaginal, and anal
// surface-bound manifold for the full-body adult female model (`FemaleDemo`).
//
// Bound via barycentric coordinates + smoothed surface frames to the pelvic floor
// triangles of `assets/characters/blender-female/prepared/body-forehead-refined.obj`
// (`60,866` vertices), deforming seamlessly with skeletal poses, jump cycles,
// soft-tissue volume cages, nonlinear FEM skin deformation, and body parameter morphs.
use glam::{Vec2, Vec3};
use voxy_render::SceneVertex;

const FEMALE_BODY_VERTICES: usize = 60_866;

// High-density curvilinear anatomical grid:
// 129 lateral samples x 193 longitudinal samples = 24,897 vertices, 49,152 triangles.
const GRID_U: usize = 129;
const GRID_V: usize = 193;

// Longitudinal bind-space span (metres):
// Covers posterior perianal/anal complex (z = -0.052) across the perineal body
// (z ~ -0.004), vaginal introitus & prolapse (z = +0.0220), urethral meatus (z = +0.0365),
// labia minora/majora, clitoral complex (z = +0.0455), to anterior mons pubis (z = +0.066).
const Z_MIN: f32 = -0.052;
const Z_MAX: f32 = 0.066;

#[derive(Clone, Copy, Debug)]
struct PelvicSampleBind {
    tri: [u32; 3],
    bary: [f32; 3],
    perimeter: f32,
    base_disp_n: f32,
    pulse_disp_n: f32,
    squeeze_disp_n: f32,
    squeeze_pos_disp_n: f32,
    tone_disp_n: f32,
    base_disp_u: f32,
    pulse_disp_u: f32,
    base_disp_v: f32,
    pulse_disp_v: f32,
    color_base_weight: f32,
    color_illum_rgb: [f32; 3],
    lumen_darkness: f32,
    uv: [f32; 2],
}

/// Dynamic physiological & biomechanical parameters driving the pelvic anatomy
/// during jumps, locomotion, and secondary soft-tissue oscillation.
#[derive(Clone, Copy, Debug)]
pub struct PelvicMotionState {
    /// Simulation or preview time in seconds.
    pub time: f32,
    /// Dynamic gluteal/pelvic compression in `[-1.0, 1.0]` from jump landings & hip motion.
    pub gluteal_compression: f32,
    /// Dynamic pelvic floor downward pressure pulsing vaginal & rectal mucosal prolapse (`[-1.0, 1.0]`).
    pub prolapse_pulse: f32,
    /// Dynamic sphincter contraction tone (`[-1.0, 1.0]`).
    pub sphincter_tone: f32,
}

impl Default for PelvicMotionState {
    fn default() -> Self {
        Self {
            time: 0.0,
            gluteal_compression: 0.0,
            prolapse_pulse: 0.0,
            sphincter_tone: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PelvicAnatomy {
    samples: Vec<PelvicSampleBind>,
    indices: Vec<u32>,
    covered_body_vertices: Vec<(usize, f32)>,
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let s = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * s,
        a[1] + (b[1] - a[1]) * s,
        a[2] + (b[2] - a[2]) * s,
    ]
}

#[inline]
fn scale3(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

#[inline]
fn pelvic_half_width(z: f32) -> f32 {
    // Matches the flat pelvic bridge between the medial thigh creases on
    // `body-forehead-refined.obj` (`|x| <= 0.0138 m` in the mid-crotch,
    // flaring slightly toward the anterior mons and posterior perianal cleft).
    let ant = smoothstep(0.036, 0.066, z);
    let post = 1.0 - smoothstep(-0.052, -0.022, z);
    0.0136 + 0.0028 * ant + 0.0026 * post
}

#[inline]
fn closest_point_on_triangle_xz(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> ([f32; 3], f32) {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return ([1.0, 0.0, 0.0], (p - a).length_squared());
    }

    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return ([0.0, 1.0, 0.0], (p - b).length_squared());
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3).max(1e-12);
        let proj = a + ab * v;
        return ([1.0 - v, v, 0.0], (p - proj).length_squared());
    }

    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return ([0.0, 0.0, 1.0], (p - c).length_squared());
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6).max(1e-12);
        let proj = a + ac * w;
        return ([1.0 - w, 0.0, w], (p - proj).length_squared());
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let bc = c - b;
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6)).max(1e-12);
        let proj = b + bc * w;
        return ([0.0, 1.0 - w, w], (p - proj).length_squared());
    }

    let denom = 1.0 / (va + vb + vc).max(1e-12);
    let v = vb * denom;
    let w = vc * denom;
    let u = 1.0 - v - w;
    let proj = a * u + b * v + c * w;
    ([u, v, w], (p - proj).length_squared())
}

impl PelvicAnatomy {
    /// Builds the surface-bound 3D vulvar, clitoral, urethral, vaginal sphincter/prolapse,
    /// and anal sphincter/prolapse manifold for the adult female body mesh.
    /// Returns an inert empty instance if `rest_vertices` is not the adult female mesh.
    #[must_use]
    pub fn new(rest_vertices: &[SceneVertex], body_indices: &[u32]) -> Self {
        if rest_vertices.len() != FEMALE_BODY_VERTICES {
            return Self {
                samples: Vec::new(),
                indices: Vec::new(),
                covered_body_vertices: Vec::new(),
            };
        }

        // Collect downward-facing pelvic floor & perineal triangles between the medial thigh creases.
        let mut pelvic_faces: Vec<[u32; 3]> = Vec::new();
        for tri in body_indices.chunks_exact(3) {
            let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            if i0 >= rest_vertices.len() || i1 >= rest_vertices.len() || i2 >= rest_vertices.len() {
                continue;
            }
            let p0 = Vec3::from_array(rest_vertices[i0].position);
            let p1 = Vec3::from_array(rest_vertices[i1].position);
            let p2 = Vec3::from_array(rest_vertices[i2].position);
            let center = (p0 + p1 + p2) / 3.0;
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
            if center.x.abs() < 0.025
                && center.y > -0.068
                && center.y < -0.042
                && center.z > -0.062
                && center.z < 0.075
                && normal.y < -0.25
            {
                pelvic_faces.push([tri[0], tri[1], tri[2]]);
            }
        }

        if pelvic_faces.is_empty() {
            return Self {
                samples: Vec::new(),
                indices: Vec::new(),
                covered_body_vertices: Vec::new(),
            };
        }

        // Identify interior base-mesh vertices covered by the pelvic manifold so they
        // can be recessed inward (+Y / -normal) after reading the carrier frame,
        // preventing coarse base-mesh chords from ever z-fighting with anatomical sulci or lumens.
        let mut covered_body_vertices = Vec::new();
        for (idx, v) in rest_vertices.iter().enumerate() {
            let p = v.position;
            if p[1] > -0.068
                && p[1] < -0.044
                && p[2] > -0.042
                && p[2] < 0.044
                && p[0].abs() < 0.0065
            {
                let u_weight = 1.0 - smootherstep(0.0038, 0.0065, p[0].abs());
                let v_weight =
                    smootherstep(-0.042, -0.030, p[2]) * (1.0 - smootherstep(0.032, 0.042, p[2]));
                let recess = 0.0062 * u_weight * v_weight;
                if recess > 1e-5 {
                    covered_body_vertices.push((idx, recess));
                }
            }
        }

        // Tissue palette anchored to the model's skin complexions:
        let skin_base = [0.84_f32, 0.69, 0.62];
        let labia_majora_tone = [
            (skin_base[0] * 0.91).clamp(0.0, 1.0),
            (skin_base[1] * 0.76).clamp(0.0, 1.0),
            (skin_base[2] * 0.71).clamp(0.0, 1.0),
        ];
        let labia_minora_edge_tone = [0.64, 0.36, 0.39];
        let labia_minora_inner_tone = [0.82, 0.45, 0.50];
        let clitoris_glans_tone = [0.89, 0.44, 0.50];
        let vestibule_mucosa_tone = [0.84, 0.43, 0.49];
        let urethral_cuff_tone = [0.86, 0.42, 0.48];
        let vaginal_sphincter_tone = [0.78, 0.36, 0.43];
        let vaginal_prolapse_crimson = [0.85, 0.25, 0.34];
        let vaginal_rugae_crest = [0.92, 0.42, 0.49];
        let perianal_pigment = [0.60, 0.41, 0.37];
        let anal_external_sphincter_tone = [0.55, 0.35, 0.33];
        let anal_intersphincteric_tone = [0.68, 0.40, 0.42];
        let rectal_prolapse_crimson = [0.83, 0.23, 0.31];
        let rectal_column_crest = [0.90, 0.38, 0.45];

        let mut samples = Vec::with_capacity(GRID_U * GRID_V);
        for iv in 0..GRID_V {
            let v_t = iv as f32 / (GRID_V - 1) as f32;
            let rest_z = Z_MIN + (Z_MAX - Z_MIN) * v_t;
            let half_w = pelvic_half_width(rest_z);
            let v_fade = smootherstep(0.0, 0.16, v_t) * (1.0 - smootherstep(0.84, 1.0, v_t));

            for iu in 0..GRID_U {
                let u_norm = (iu as f32 / (GRID_U - 1) as f32) * 2.0 - 1.0;
                let rest_x = u_norm * half_w;
                let u_fade = 1.0 - smootherstep(0.70, 1.0, u_norm.abs());
                let perimeter = (u_fade * v_fade).clamp(0.0, 1.0);

                let target_xz = Vec2::new(rest_x, rest_z);
                let mut best_tri = pelvic_faces[0];
                let mut best_bary = [1.0, 0.0, 0.0];
                let mut best_score = f32::INFINITY;

                for &tri in &pelvic_faces {
                    let a3 = rest_vertices[tri[0] as usize].position;
                    let b3 = rest_vertices[tri[1] as usize].position;
                    let c3 = rest_vertices[tri[2] as usize].position;
                    let a = Vec2::new(a3[0], a3[2]);
                    let b = Vec2::new(b3[0], b3[2]);
                    let c = Vec2::new(c3[0], c3[2]);
                    let (bary, dist2) = closest_point_on_triangle_xz(target_xz, a, b, c);
                    let y_interp = bary[0] * a3[1] + bary[1] * b3[1] + bary[2] * c3[1];
                    let thigh_penalty = (-0.0635 - y_interp).max(0.0) * 0.05;
                    let score = dist2 + thigh_penalty * thigh_penalty;
                    if score < best_score {
                        best_score = score;
                        best_tri = tri;
                        best_bary = bary;
                    }
                }

                let x = rest_x;
                let z = rest_z;
                let abs_x = x.abs();

                // 1. LABIA MAJORA & INTERLABIAL SULCUS (z in [+0.002, +0.058])
                let vulva_v = smoothstep(0.000, 0.012, z) * (1.0 - smootherstep(0.044, 0.056, z));
                let vulva_phase = ((z - 0.002) / 0.052).clamp(0.0, 1.0);
                let majora_arch = (std::f32::consts::PI * vulva_phase).sin().max(0.0);
                let majora_center_x = 0.0062 + 0.0024 * majora_arch;
                let majora_sigma = 0.0024 + 0.0005 * majora_arch;
                let majora_profile =
                    (-((abs_x - majora_center_x) / majora_sigma).powi(2)).exp() * vulva_v;
                let majora_wrinkle = 1.0
                    + 0.040 * (z * 620.0 + abs_x * 280.0).sin()
                    + 0.022 * (z * 1150.0 - abs_x * 410.0).cos();
                let labia_majora_base_h = 0.0050 * majora_profile * majora_wrinkle;

                let mons_pad = 0.0008
                    * smoothstep(0.032, 0.045, z)
                    * (1.0 - smootherstep(0.045, 0.055, z))
                    * (-((x / 0.0085).powi(2))).exp();

                let sulcus_center_x = 0.0040 + 0.0019 * majora_arch;
                let interlabial_sulcus_profile = (-((abs_x - sulcus_center_x) / 0.00105).powi(2))
                    .exp()
                    * smoothstep(0.006, 0.014, z)
                    * (1.0 - smoothstep(0.046, 0.053, z));
                let interlabial_sulcus_h = -0.0020 * interlabial_sulcus_profile;

                // 2. LABIA MINORA (`Nymphae`), FOURCHETTE & VESTIBULE (z in [+0.008, +0.043])
                let minora_v = smoothstep(0.008, 0.014, z) * (1.0 - smoothstep(0.0392, 0.0430, z));
                let minora_phase = ((z - 0.008) / 0.035).clamp(0.0, 1.0);
                let minora_arch = (std::f32::consts::PI * minora_phase)
                    .sin()
                    .max(0.0)
                    .powf(1.05);
                let minora_center_x = 0.0013 + 0.0032 * minora_arch;
                let minora_sigma = 0.00095 + 0.00028 * minora_arch;
                let minora_dist = (abs_x - minora_center_x) / minora_sigma;
                let minora_profile = (-minora_dist * minora_dist).exp() * minora_v;

                let inner_nympho_groove_x = (minora_center_x - 0.00125).max(0.0007);
                let inner_nympho_groove = (-((abs_x - inner_nympho_groove_x) / 0.00065).powi(2))
                    .exp()
                    * smoothstep(0.012, 0.017, z)
                    * (1.0 - smoothstep(0.038, 0.042, z));

                let smooth_sign_x = (x / 0.0010).tanh();
                let minora_crenulation = 1.0
                    + 0.12 * (z * 540.0 + smooth_sign_x * 0.5).sin()
                    + 0.08 * (z * 980.0 - abs_x * 620.0).cos()
                    + 0.04 * (z * 1550.0).sin();
                let labia_minora_h =
                    0.0065 * minora_profile * minora_crenulation - 0.0015 * inner_nympho_groove;
                let labia_minora_flare_u =
                    0.00075 * smooth_sign_x * minora_profile * (0.68 + 0.32 * (z * 480.0).sin());

                let fourchette_arch =
                    (-((z - 0.0095) / 0.0022).powi(2)).exp() * (-((x / 0.0028).powi(2))).exp();
                let fourchette_h = 0.0026 * fourchette_arch;
                let fossa_navicularis_h = -0.0015
                    * (-((z - 0.0132) / 0.0022).powi(2)).exp()
                    * (-((x / 0.0022).powi(2))).exp();

                let inside_vestibule = (1.0
                    - smoothstep(minora_center_x - 0.0006, minora_center_x + 0.0006, abs_x))
                    * smoothstep(0.010, 0.015, z)
                    * (1.0 - smoothstep(0.041, 0.0465, z));
                let vestibule_cleft_h = -0.0012 * inside_vestibule;

                // 3. 3D CLITORAL COMPLEX
                let clit_z = 0.0442;
                let clit_rx = 0.0021;
                let clit_rz = 0.0026;
                let clit_dx = x / clit_rx;
                let clit_dz = (z - clit_z) / clit_rz;
                let clit_r2 = clit_dx * clit_dx + clit_dz * clit_dz;
                let clit_r = clit_r2.sqrt();

                let clit_dome = (-1.18 * clit_r2).exp() * (1.0 - smoothstep(0.84, 1.12, clit_r));
                let clit_corona =
                    (-((clit_r - 0.68) / 0.18).powi(2)).exp() * smoothstep(-0.30, 0.50, clit_dz);
                let clit_cleft = -0.00028
                    * (-((x / 0.00042).powi(2))).exp()
                    * (-((z - (clit_z - 0.0008)) / 0.00090).powi(2)).exp();
                let clit_glans_mask = 1.0 - smoothstep(0.72, 1.02, clit_r);
                let clit_glans_base = 0.0068 * clit_dome + 0.0014 * clit_corona;
                let clit_flare_u = 0.00028 * clit_dx * (-((clit_r - 0.65) / 0.24).powi(2)).exp();
                let clit_flare_v = -0.00028 * (-((clit_r - 0.58) / 0.28).powi(2)).exp();

                let clit_moat_profile =
                    (-((clit_r - 1.05) / 0.18).powi(2)).exp() * smoothstep(-0.45, 0.30, clit_dz);
                let clit_moat_h = -0.0019 * clit_moat_profile;

                let hood_rx = 0.0024;
                let hood_rz = 0.0028;
                let hood_dx = x / hood_rx;
                let hood_dz = (z - (clit_z + 0.0004)) / hood_rz;
                let hood_r = (hood_dx * hood_dx + hood_dz * hood_dz).sqrt();
                let hood_ring = (-((hood_r - 1.36) / 0.38).powi(2)).exp();
                let hood_arch_mask =
                    smoothstep(-0.75, -0.10, hood_dz) * (1.0 - smootherstep(0.0490, 0.0545, z));
                let hood_profile = hood_ring * hood_arch_mask * (1.0 - 0.95 * clit_glans_mask);
                let hood_flank_sulcus = -0.0012
                    * (-((abs_x - 0.0044) / 0.00110).powi(2)).exp()
                    * smoothstep(0.040, 0.043, z)
                    * (1.0 - smoothstep(0.049, 0.053, z));
                let clit_hood_h = 0.0056 * hood_profile + hood_flank_sulcus;

                let fren_t = ((clit_z - 0.0010 - z) / 0.0050).clamp(0.0, 1.0);
                let fren_axis_x = 0.00072 + 0.00160 * fren_t;
                let fren_profile = (-((abs_x - fren_axis_x) / 0.00062).powi(2)).exp()
                    * smoothstep(0.0382, 0.0398, z)
                    * (1.0 - smoothstep(0.0426, 0.0438, z));
                let fren_median_groove = -0.00085
                    * (-((x / 0.00058).powi(2))).exp()
                    * smoothstep(0.0388, 0.0400, z)
                    * (1.0 - smoothstep(0.0424, 0.0435, z));
                let clit_frenulum_h = 0.0034 * fren_profile + fren_median_groove;

                // 4. EXTERNAL URETHRAL SPHINCTER ROSETTE
                let ure_z = 0.0365;
                let ure_rx = 0.0020;
                let ure_rz = 0.0023;
                let ure_dx = x / ure_rx;
                let ure_dz = (z - ure_z) / ure_rz;
                let ure_r = (ure_dx * ure_dx + ure_dz * ure_dz).sqrt();
                let ure_theta = x.atan2(z - ure_z);

                let ure_cuff_ring = (-((ure_r - 0.72) / 0.28).powi(2)).exp();
                let ure_stellate = 0.5 + 0.5 * (8.0 * ure_theta).cos();
                let ure_outer_moat = -0.0006 * (-((ure_r - 1.15) / 0.22).powi(2)).exp();
                let ure_cuff_unit = 0.0030 * ure_cuff_ring * (0.72 + 0.28 * ure_stellate);
                let ure_slit = (-((x / 0.00048).powi(2) + ((z - ure_z) / 0.00105).powi(2))).exp();
                let ure_lumen_h = -0.0032 * ure_slit;

                // 5. BULBOSPONGIOSUS VAGINAL SPHINCTER & VAGINAL MUCOSAL ROSETTE PROLAPSE
                let vag_z = 0.0220;
                let vag_rx = 0.0041;
                let vag_rz = 0.0065;
                let vag_dx = x / vag_rx;
                let vag_dz = (z - vag_z) / vag_rz;
                let vag_r = (vag_dx * vag_dx + vag_dz * vag_dz).sqrt();
                let vag_theta = vag_dx.atan2(vag_dz);

                let vag_sphincter_ring = (-((vag_r - 1.02) / 0.20).powi(2)).exp();
                let vag_sphincter_fibers =
                    1.0 + 0.14 * (16.0 * vag_theta).cos() + 0.10 * (vag_r * 38.0).sin();
                let vag_sphincter_unit = 0.0032 * vag_sphincter_ring * vag_sphincter_fibers;

                let vag_introital_sulcus = (-((vag_r - 0.80) / 0.10).powi(2)).exp();
                let vag_sulcus_h = -0.0014 * vag_introital_sulcus;

                let hymen_ring = (-((vag_r - 0.74) / 0.11).powi(2)).exp();
                let hymen_lobes = (0.5 + 0.5 * (8.0 * vag_theta + 0.3).cos()).powi(2);
                let vag_hymen_h = 0.0018 * hymen_ring * (0.45 + 0.55 * hymen_lobes);

                let vag_prolapse_torus = (-((vag_r - 0.48) / 0.23).powi(2)).exp();
                let vag_lobe_wave =
                    0.5 + 0.5 * (12.0 * vag_theta + 0.25 * (vag_r * 10.0).sin()).cos();
                let vag_lobe_valley = 1.0 - vag_lobe_wave;
                let columna_weight = (vag_dz.abs()).powf(1.3) * (-((x / 0.0022).powi(2))).exp();
                let rugae_wave = 0.5 + 0.5 * (vag_r * 52.0 + 2.0 * (6.0 * vag_theta).cos()).cos();
                let vag_prolapse_shape =
                    vag_prolapse_torus * (0.62 + 0.38 * vag_lobe_wave) * (0.78 + 0.22 * rugae_wave);
                let vag_prolapse_base_h = 0.0056 * vag_prolapse_shape
                    + 0.0014 * columna_weight * vag_prolapse_torus * rugae_wave;

                let vag_flare_env = (-((vag_r - 0.52) / 0.22).powi(2)).exp();
                let vag_flare_base_u = 0.00042 * vag_dx * vag_flare_env;
                let vag_flare_base_v = 0.00052 * vag_dz * vag_flare_env;

                let vag_lumen_pit = (-((vag_r / 0.21).powi(2))).exp();
                let vag_lumen_h = -0.0044 * vag_lumen_pit;

                // 6. PERINEAL BODY & MEDIAN RAPHE
                let perineum_span =
                    smoothstep(-0.017, -0.007, z) * (1.0 - smoothstep(0.002, 0.010, z));
                let perineal_crown = 0.0014 * perineum_span * (-((x / 0.0075).powi(2))).exp();
                let perineal_raphe = 0.00045
                    * perineum_span
                    * (-((x / 0.00085).powi(2))).exp()
                    * (1.0 + 0.15 * (z * 700.0).sin());

                // 7. ANAL SPHINCTERS & RECTAL MUCOSAL PROLAPSE
                let anal_z = -0.0315;
                let anal_rx = 0.0066;
                let anal_rz = 0.0074;
                let anal_dx = x / anal_rx;
                let anal_dz = (z - anal_z) / anal_rz;
                let anal_r = (anal_dx * anal_dx + anal_dz * anal_dz).sqrt();
                let anal_theta = anal_dx.atan2(anal_dz);

                let anal_ext_ring = (-((anal_r - 1.00) / 0.24).powi(2)).exp();
                let anal_concentric = 0.5 + 0.5 * (anal_r * 36.0).cos();
                let plicae_env =
                    smoothstep(0.45, 0.72, anal_r) * (1.0 - smoothstep(1.18, 1.62, anal_r));
                let plicae_primary =
                    0.5 + 0.5 * (16.0 * anal_theta + 0.18 * (anal_r * 16.0).sin()).cos();
                let plicae_secondary = 0.5 + 0.5 * (32.0 * anal_theta - 0.4).cos();
                let plicae_wave = 0.72 * plicae_primary + 0.28 * plicae_secondary;
                let anal_ext_unit = 0.0036 * anal_ext_ring * (0.82 + 0.18 * anal_concentric)
                    + 0.0022 * plicae_env * (plicae_wave - 0.35);

                let anal_intersphincteric = (-((anal_r - 0.72) / 0.095).powi(2)).exp();
                let anal_groove_h = -0.0014 * anal_intersphincteric;

                let anal_prolapse_torus = (-((anal_r - 0.44) / 0.20).powi(2)).exp();
                let morgagni_wave =
                    0.5 + 0.5 * (10.0 * anal_theta + 0.20 * (anal_r * 12.0).cos()).cos();
                let morgagni_cleft = 1.0 - morgagni_wave;
                let rectal_valves = 0.5 + 0.5 * (anal_r * 48.0 + 1.5 * morgagni_wave).cos();
                let anal_prolapse_shape = anal_prolapse_torus
                    * (0.58 + 0.42 * morgagni_wave)
                    * (0.80 + 0.20 * rectal_valves);
                let anal_prolapse_base_h = 0.0054 * anal_prolapse_shape;

                let anal_flare_env = (-((anal_r - 0.46) / 0.20).powi(2)).exp();
                let anal_flare_base_u = 0.00038 * anal_dx * anal_flare_env;
                let anal_flare_base_v = 0.00042 * anal_dz * anal_flare_env;

                let anal_lumen_pit = (-((anal_r / 0.19).powi(2))).exp();
                let anal_lumen_h = -0.0044 * anal_lumen_pit;

                // 8. PRECOMPUTED DISPLACEMENT COEFFICIENTS (tapered by `perimeter`)
                let static_n = 0.00085
                    + mons_pad
                    + labia_majora_base_h
                    + interlabial_sulcus_h
                    + labia_minora_h
                    + fourchette_h
                    + fossa_navicularis_h
                    + vestibule_cleft_h
                    + clit_glans_base
                    + clit_cleft
                    + clit_moat_h
                    + clit_hood_h
                    + clit_frenulum_h
                    + 0.92 * ure_cuff_unit
                    + ure_outer_moat
                    + ure_lumen_h
                    + 0.90 * vag_sphincter_unit
                    + vag_sulcus_h
                    + vag_hymen_h
                    + vag_prolapse_base_h
                    + vag_lumen_h
                    + perineal_crown
                    + perineal_raphe
                    + 0.90 * anal_ext_unit
                    + anal_groove_h
                    + anal_prolapse_base_h
                    + anal_lumen_h;

                let base_disp_n = static_n * perimeter;
                let pulse_disp_n = (0.08 * clit_glans_base
                    + 0.0012 * vag_prolapse_shape
                    + 0.0012 * anal_prolapse_shape)
                    * perimeter;
                let squeeze_disp_n = (0.06 * labia_majora_base_h) * perimeter;
                let squeeze_pos_disp_n = (0.0006 * anal_prolapse_shape) * perimeter;
                let tone_disp_n =
                    (0.16 * ure_cuff_unit + 0.20 * vag_sphincter_unit + 0.20 * anal_ext_unit)
                        * perimeter;

                let base_disp_u =
                    (labia_minora_flare_u + clit_flare_u + vag_flare_base_u + anal_flare_base_u)
                        * perimeter;
                let pulse_disp_u = (0.15 * (vag_flare_base_u + anal_flare_base_u)) * perimeter;
                let base_disp_v = (clit_flare_v + vag_flare_base_v + anal_flare_base_v) * perimeter;
                let pulse_disp_v = (0.15 * (vag_flare_base_v + anal_flare_base_v)) * perimeter;

                // 9. PRECOMPUTED TISSUE PIGMENTATION & WETNESS COEFFICIENTS
                let mut color_base_weight = 1.0_f32;
                let mut color_illum_rgb = [0.0_f32; 3];
                let mut wetness = 0.02_f32;
                let mut roughness = 0.50_f32;

                let mut blend_tint = |tint_rgb: [f32; 3], illum_scale: f32, alpha: f32| {
                    let s = alpha.clamp(0.0, 1.0);
                    color_base_weight *= 1.0 - s;
                    color_illum_rgb = mix3(color_illum_rgb, scale3(tint_rgb, illum_scale), s);
                };

                let majora_tint = (majora_profile * 0.65).clamp(0.0, 1.0);
                blend_tint(labia_majora_tone, 1.0, majora_tint);
                roughness = roughness * (1.0 - 0.12 * majora_tint) + 0.42 * 0.12 * majora_tint;

                blend_tint(vestibule_mucosa_tone, 1.15, inside_vestibule * 0.92);
                wetness = wetness.max(0.65 * inside_vestibule);
                roughness = roughness * (1.0 - inside_vestibule) + 0.20 * inside_vestibule;

                let minora_color_mask = (minora_profile * 1.15).clamp(0.0, 1.0);
                let minora_outer_edge = smoothstep(-0.2, 0.65, minora_dist);
                let minora_tone = mix3(
                    labia_minora_inner_tone,
                    labia_minora_edge_tone,
                    minora_outer_edge,
                );
                blend_tint(minora_tone, 1.12, minora_color_mask);
                wetness = wetness.max(0.58 * minora_color_mask * (1.0 - 0.35 * minora_outer_edge));
                roughness = roughness * (1.0 - minora_color_mask) + 0.24 * minora_color_mask;

                let hood_mask = (hood_profile * 1.1).clamp(0.0, 1.0);
                blend_tint(labia_minora_inner_tone, 1.12, hood_mask * 0.88);
                wetness = wetness.max(0.56 * hood_mask);

                let fren_mask = (fren_profile * 1.2).clamp(0.0, 1.0);
                blend_tint(clitoris_glans_tone, 1.16, fren_mask * 0.88);
                wetness = wetness.max(0.76 * fren_mask);

                blend_tint(clitoris_glans_tone, 1.18, clit_glans_mask);
                wetness = wetness.max(0.88 * clit_glans_mask);
                roughness = roughness * (1.0 - clit_glans_mask) + 0.15 * clit_glans_mask;

                let ure_mask = (1.0 - smoothstep(0.65, 1.15, ure_r)).clamp(0.0, 1.0);
                blend_tint(urethral_cuff_tone, 1.15, ure_mask * 0.90);
                wetness = wetness.max(0.86 * ure_mask);
                roughness = roughness * (1.0 - ure_mask) + 0.16 * ure_mask;

                let vag_sphincter_mask = (vag_sphincter_ring * 1.1).clamp(0.0, 1.0);
                blend_tint(vaginal_sphincter_tone, 1.14, vag_sphincter_mask * 0.85);
                wetness = wetness.max(0.75 * vag_sphincter_mask);

                let vag_prolapse_mask = (smoothstep(0.14, 0.26, vag_r)
                    * (1.0 - smoothstep(0.70, 0.84, vag_r)))
                .clamp(0.0, 1.0);
                let vag_prolapse_tint = mix3(
                    vaginal_prolapse_crimson,
                    vaginal_rugae_crest,
                    0.48 * vag_lobe_wave * rugae_wave,
                );
                blend_tint(vag_prolapse_tint, 1.18, vag_prolapse_mask);
                wetness = wetness.max(0.96 * vag_prolapse_mask);
                roughness = roughness * (1.0 - vag_prolapse_mask) + 0.13 * vag_prolapse_mask;

                let perianal_mask = (1.0 - smoothstep(0.65, 1.65, anal_r)).clamp(0.0, 1.0);
                blend_tint(perianal_pigment, 1.08, perianal_mask * 0.78);
                wetness = wetness.max(0.24 * perianal_mask);
                roughness = roughness * (1.0 - perianal_mask) + 0.34 * perianal_mask;

                let anal_ext_mask = (anal_ext_ring * 1.1 + 0.5 * plicae_env).clamp(0.0, 1.0);
                blend_tint(
                    anal_external_sphincter_tone,
                    1.10,
                    anal_ext_mask * (0.65 + 0.35 * (1.0 - plicae_wave)),
                );
                wetness = wetness.max(0.42 * anal_ext_mask);
                roughness = roughness * (1.0 - anal_ext_mask) + 0.26 * anal_ext_mask;

                let intersphincteric_mask = (anal_intersphincteric * 1.15).clamp(0.0, 1.0);
                blend_tint(anal_intersphincteric_tone, 1.12, intersphincteric_mask);
                wetness = wetness.max(0.68 * intersphincteric_mask);

                let anal_prolapse_mask = (smoothstep(0.14, 0.24, anal_r)
                    * (1.0 - smoothstep(0.64, 0.75, anal_r)))
                .clamp(0.0, 1.0);
                let anal_prolapse_tint = mix3(
                    rectal_prolapse_crimson,
                    rectal_column_crest,
                    0.46 * morgagni_wave * rectal_valves,
                );
                blend_tint(anal_prolapse_tint, 1.18, anal_prolapse_mask);
                wetness = wetness.max(0.95 * anal_prolapse_mask);
                roughness = roughness * (1.0 - anal_prolapse_mask) + 0.13 * anal_prolapse_mask;

                let sulcus_ao = (0.38 * interlabial_sulcus_profile
                    + 0.36 * inner_nympho_groove
                    + 0.56 * clit_moat_profile
                    + 0.34 * (-hood_flank_sulcus / 0.0013).clamp(0.0, 1.0)
                    + 0.42 * vag_introital_sulcus
                    + 0.36 * vag_prolapse_mask * vag_lobe_valley
                    + 0.44 * anal_intersphincteric
                    + 0.35 * plicae_env * (1.0 - plicae_wave)
                    + 0.38 * anal_prolapse_mask * morgagni_cleft)
                    .clamp(0.0, 0.70);
                color_base_weight *= 1.0 - sulcus_ao;
                color_illum_rgb = scale3(color_illum_rgb, 1.0 - sulcus_ao);

                let lumen_darkness = (0.85 * ure_slit
                    + 0.96 * (1.0 - smoothstep(0.06, 0.23, vag_r))
                    + 0.96 * (1.0 - smoothstep(0.06, 0.21, anal_r)))
                .clamp(0.0, 0.97);

                let static_wetness = (wetness * perimeter).clamp(0.0, 1.0);
                let final_roughness = (0.50 + (roughness - 0.50) * perimeter).clamp(0.13, 0.65);

                samples.push(PelvicSampleBind {
                    tri: best_tri,
                    bary: best_bary,
                    perimeter,
                    base_disp_n,
                    pulse_disp_n,
                    squeeze_disp_n,
                    squeeze_pos_disp_n,
                    tone_disp_n,
                    base_disp_u,
                    pulse_disp_u,
                    base_disp_v,
                    pulse_disp_v,
                    color_base_weight,
                    color_illum_rgb,
                    lumen_darkness,
                    uv: [-1.04 - 0.10 * static_wetness, final_roughness],
                });
            }
        }

        let mut indices = Vec::with_capacity((GRID_U - 1) * (GRID_V - 1) * 6);
        for iv in 0..(GRID_V - 1) {
            for iu in 0..(GRID_U - 1) {
                let r0 = (iv * GRID_U + iu) as u32;
                let r1 = r0 + 1;
                let r2 = ((iv + 1) * GRID_U + iu) as u32;
                let r3 = r2 + 1;
                // Winding order chosen so (p1 - p0).cross(p2 - p0) points outward (-Y).
                indices.extend_from_slice(&[r0, r2, r1, r1, r2, r3]);
            }
        }

        Self {
            samples,
            indices,
            covered_body_vertices,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.samples.len()
    }

    /// Appends the deformed 3D pelvic/vulvar/anal manifold vertices and triangle indices
    /// onto the character render streams.
    pub fn append(
        &self,
        deformed_vertices: &[SceneVertex],
        deformed_normals: &[Vec3],
        vertices: &mut Vec<SceneVertex>,
        indices: &mut Vec<u32>,
        motion: PelvicMotionState,
    ) {
        self.append_with_normals(
            deformed_vertices,
            deformed_normals,
            vertices,
            indices,
            None,
            motion,
        );
    }

    /// Appends the deformed 3D pelvic/vulvar/anal manifold vertices, triangle indices,
    /// and optionally smooth grid-evaluated surface normals onto the character render streams.
    pub fn append_with_normals(
        &self,
        deformed_vertices: &[SceneVertex],
        deformed_normals: &[Vec3],
        vertices: &mut Vec<SceneVertex>,
        indices: &mut Vec<u32>,
        mut out_normals: Option<&mut Vec<[f32; 3]>>,
        motion: PelvicMotionState,
    ) {
        if self.samples.is_empty() || deformed_vertices.len() < FEMALE_BODY_VERTICES {
            return;
        }

        let base_vertex = u32::try_from(vertices.len()).expect("pelvic vertex index fits u32");
        let n_samples = self.samples.len();

        let mut carrier_pos = Vec::with_capacity(n_samples);
        let mut carrier_norm = Vec::with_capacity(n_samples);
        let mut carrier_color = Vec::with_capacity(n_samples);

        for sample in &self.samples {
            let [i0, i1, i2] = sample.tri.map(|i| i as usize);
            let [b0, b1, b2] = sample.bary;

            let p0 = Vec3::from_array(deformed_vertices[i0].position);
            let p1 = Vec3::from_array(deformed_vertices[i1].position);
            let p2 = Vec3::from_array(deformed_vertices[i2].position);
            carrier_pos.push(p0 * b0 + p1 * b1 + p2 * b2);

            let n0 = deformed_normals[i0];
            let n1 = deformed_normals[i1];
            let n2 = deformed_normals[i2];
            let mut b_n = (n0 * b0 + n1 * b1 + n2 * b2).normalize_or_zero();
            if b_n.y > -0.15 {
                b_n = Vec3::new(b_n.x * 0.35, -0.92, b_n.z * 0.35).normalize_or_zero();
            }
            carrier_norm.push(b_n);

            let c0 = Vec3::from_array([
                vertices[i0].color[0],
                vertices[i0].color[1],
                vertices[i0].color[2],
            ]);
            let c1 = Vec3::from_array([
                vertices[i1].color[0],
                vertices[i1].color[1],
                vertices[i1].color[2],
            ]);
            let c2 = Vec3::from_array([
                vertices[i2].color[0],
                vertices[i2].color[1],
                vertices[i2].color[2],
            ]);
            carrier_color.push(c0 * b0 + c1 * b1 + c2 * b2);
        }

        // Smooth the coarse polyhedral carrier surface across the interior of the
        // 129 x 193 grid (weighted by `sample.perimeter` so the boundary remains
        // 100% locked to the exact base-mesh triangles).
        let mut scratch_pos = carrier_pos.clone();
        let mut scratch_norm = carrier_norm.clone();
        for _pass in 0..4 {
            for iv in 1..(GRID_V - 1) {
                let row = iv * GRID_U;
                for iu in 1..(GRID_U - 1) {
                    let idx = row + iu;
                    let w = self.samples[idx].perimeter;
                    let avg_p =
                        (carrier_pos[idx - 1] + carrier_pos[idx] * 2.0 + carrier_pos[idx + 1])
                            * 0.25;
                    scratch_pos[idx] = carrier_pos[idx].lerp(avg_p, 0.85 * w);
                    let avg_n =
                        (carrier_norm[idx - 1] + carrier_norm[idx] * 2.0 + carrier_norm[idx + 1])
                            .normalize_or_zero();
                    scratch_norm[idx] = carrier_norm[idx]
                        .lerp(avg_n, 0.92 * w.sqrt())
                        .normalize_or_zero();
                }
            }
            for iv in 1..(GRID_V - 1) {
                let row = iv * GRID_U;
                for iu in 1..(GRID_U - 1) {
                    let idx = row + iu;
                    let w = self.samples[idx].perimeter;
                    let avg_p = (scratch_pos[idx - GRID_U]
                        + scratch_pos[idx] * 2.0
                        + scratch_pos[idx + GRID_U])
                        * 0.25;
                    carrier_pos[idx] = scratch_pos[idx].lerp(avg_p, 0.85 * w);
                    let avg_n = (scratch_norm[idx - GRID_U]
                        + scratch_norm[idx] * 2.0
                        + scratch_norm[idx + GRID_U])
                        .normalize_or_zero();
                    carrier_norm[idx] = scratch_norm[idx]
                        .lerp(avg_n, 0.92 * w.sqrt())
                        .normalize_or_zero();
                }
            }
        }

        let deep_lumen_dark = [0.05_f32, 0.010, 0.015];
        let pulse = 0.5
            + 0.5 * (motion.time * 2.6).sin() * 0.30
            + 0.35 * motion.prolapse_pulse.clamp(-1.0, 1.0);
        let glute_squeeze = motion.gluteal_compression.clamp(-1.0, 1.0);
        let glute_squeeze_pos = glute_squeeze.max(0.0);
        let tone = (0.5 + 0.5 * motion.sphincter_tone).clamp(0.0, 1.0);

        let start_vertex_idx = vertices.len();
        vertices.reserve(n_samples);

        for (idx, sample) in self.samples.iter().enumerate() {
            let b_pos = carrier_pos[idx];
            let b_n = carrier_norm[idx];
            let base_lit_color = carrier_color[idx].to_array();

            let mut b_u = Vec3::X - b_n * b_n.x;
            if b_u.length_squared() < 1e-6 {
                b_u = Vec3::X;
            } else {
                b_u = b_u.normalize();
            }
            let b_v = b_n.cross(b_u).normalize_or_zero();

            let disp_n = sample.base_disp_n
                + pulse * sample.pulse_disp_n
                + glute_squeeze * sample.squeeze_disp_n
                + glute_squeeze_pos * sample.squeeze_pos_disp_n
                + tone * sample.tone_disp_n;
            let disp_u = sample.base_disp_u + pulse * sample.pulse_disp_u;
            let disp_v = sample.base_disp_v + pulse * sample.pulse_disp_v;

            let pos = b_pos + b_n * disp_n + b_u * disp_u + b_v * disp_v;

            let local_illum = (base_lit_color[0] / 0.84).clamp(0.46, 1.0);
            let pre_lumen = [
                base_lit_color[0] * sample.color_base_weight
                    + sample.color_illum_rgb[0] * local_illum,
                base_lit_color[1] * sample.color_base_weight
                    + sample.color_illum_rgb[1] * local_illum,
                base_lit_color[2] * sample.color_base_weight
                    + sample.color_illum_rgb[2] * local_illum,
            ];
            let color = mix3(pre_lumen, deep_lumen_dark, sample.lumen_darkness);
            let final_color = mix3(base_lit_color, color, sample.perimeter);

            vertices.push(SceneVertex {
                position: pos.to_array(),
                uv: sample.uv,
                color: [final_color[0], final_color[1], final_color[2], 1.0],
            });
        }

        if let Some(normals_out) = &mut out_normals {
            normals_out.reserve(n_samples);
            let slice = &vertices[start_vertex_idx..start_vertex_idx + n_samples];
            for iv in 0..GRID_V {
                let iv_prev = iv.saturating_sub(1);
                let iv_next = (iv + 1).min(GRID_V - 1);
                let row = iv * GRID_U;
                for iu in 0..GRID_U {
                    let iu_prev = iu.saturating_sub(1);
                    let iu_next = (iu + 1).min(GRID_U - 1);
                    let idx = row + iu;
                    let p_u0 = Vec3::from_array(slice[row + iu_prev].position);
                    let p_u1 = Vec3::from_array(slice[row + iu_next].position);
                    let p_v0 = Vec3::from_array(slice[iv_prev * GRID_U + iu].position);
                    let p_v1 = Vec3::from_array(slice[iv_next * GRID_U + iu].position);
                    let du = p_u1 - p_u0;
                    let dv = p_v1 - p_v0;
                    let grid_n = dv.cross(du).try_normalize().unwrap_or(carrier_norm[idx]);
                    let w = self.samples[idx].perimeter;
                    let n = carrier_norm[idx].lerp(grid_n, w).normalize_or_zero();
                    normals_out.push(n.to_array());
                }
            }
        }

        indices.extend(self.indices.iter().map(|&i| base_vertex + i));
    }

    /// Appends barycentrically interpolated 4-joint skeletal skinning weights for all
    /// `24,897` pelvic manifold vertices so GPU dual-quaternion skinning (`SurfaceRigidSkinning`)
    /// deforms the pelvic anatomy with the exact pelvis & hip joints.
    pub fn append_gpu_jump_weights(
        &self,
        base_weights: &[voxy_render::SurfaceRigidSkinWeight],
        out: &mut Vec<voxy_render::SurfaceRigidSkinWeight>,
    ) {
        out.reserve(self.samples.len());
        for sample in &self.samples {
            let [i0, i1, i2] = sample.tri.map(|i| i as usize);
            let [b0, b1, b2] = sample.bary;
            let corners = [(i0, b0), (i1, b1), (i2, b2)];

            let mut accum: [(u32, f32); 12] = [(0, 0.0); 12];
            let mut count = 0usize;
            for (corner_idx, bary_w) in corners {
                if bary_w <= 0.0 {
                    continue;
                }
                let Some(row) = base_weights.get(corner_idx) else {
                    continue;
                };
                for k in 0..4 {
                    let w = row.weights[k] * bary_w;
                    if w <= 0.0 {
                        continue;
                    }
                    let joint = row.joints[k];
                    if let Some(existing) = accum[..count].iter_mut().find(|(j, _)| *j == joint) {
                        existing.1 += w;
                    } else if count < 12 {
                        accum[count] = (joint, w);
                        count += 1;
                    }
                }
            }
            accum[..count].sort_by(|a, b| b.1.total_cmp(&a.1));
            let top_count = count.min(4);
            let sum: f32 = accum[..top_count].iter().map(|(_, w)| *w).sum();
            let mut joints = [0u32; 4];
            let mut weights = [0.0f32; 4];
            if sum > 1e-6 {
                for k in 0..top_count {
                    joints[k] = accum[k].0;
                    weights[k] = accum[k].1 / sum;
                }
                // Guarantee exact partition of unity within 1e-6 for GPU validation.
                let tail_sum: f32 = weights[1..top_count].iter().sum();
                weights[0] = (1.0 - tail_sum).max(0.0);
            } else {
                joints[0] = 0;
                weights[0] = 1.0;
            }
            out.push(voxy_render::SurfaceRigidSkinWeight { joints, weights });
        }
    }

    /// Appends barycentrically interpolated soft-tissue cage weights plus dynamic
    /// prolapse/sphincter pulse weights for the `24,897` pelvic manifold vertices.
    pub fn append_gpu_secondary_weights(
        &self,
        pelvic_start: usize,
        pelvic_pulse_control: u32,
        pelvic_squeeze_control: u32,
        rows: &mut [Vec<voxy_render::SurfaceDeformationWeight>],
    ) {
        for (idx, sample) in self.samples.iter().enumerate() {
            let target_vertex = pelvic_start + idx;
            if target_vertex >= rows.len() {
                break;
            }
            let [i0, i1, i2] = sample.tri.map(|i| i as usize);
            let [b0, b1, b2] = sample.bary;
            let corners = [(i0, b0), (i1, b1), (i2, b2)];

            let mut accum: Vec<(u32, f32)> = Vec::with_capacity(8);
            for (corner_idx, bary_w) in corners {
                if bary_w <= 1e-5 || corner_idx >= pelvic_start {
                    continue;
                }
                for entry in &rows[corner_idx] {
                    if entry.control == 0 {
                        continue;
                    }
                    let w = entry.weight * bary_w;
                    if w.abs() <= 1e-6 {
                        continue;
                    }
                    if let Some(existing) = accum.iter_mut().find(|(c, _)| *c == entry.control) {
                        existing.1 += w;
                    } else {
                        accum.push((entry.control, w));
                    }
                }
            }
            let row = &mut rows[target_vertex];
            for (control, weight) in accum {
                row.push(voxy_render::SurfaceDeformationWeight { control, weight });
            }
            let pulse_w = sample.pulse_disp_n + 0.5 * sample.tone_disp_n;
            if pulse_w.abs() > 1e-7 {
                row.push(voxy_render::SurfaceDeformationWeight {
                    control: pelvic_pulse_control,
                    weight: pulse_w,
                });
            }
            let squeeze_w = sample.squeeze_disp_n + sample.squeeze_pos_disp_n;
            if squeeze_w.abs() > 1e-7 {
                row.push(voxy_render::SurfaceDeformationWeight {
                    control: pelvic_squeeze_control,
                    weight: squeeze_w,
                });
            }
        }
    }

    /// Gently recesses the interior base-mesh vertices covered by the pelvic patch
    /// after `finish_surface_vertices` so coarse base-mesh triangles cannot poke
    /// through deep anatomical sulci or lumens.
    pub fn recess_covered_body_vertices(
        &self,
        deformed_normals: &[Vec3],
        vertices: &mut [SceneVertex],
    ) {
        for &(body_idx, recess) in &self.covered_body_vertices {
            if let (Some(n), Some(v)) = (deformed_normals.get(body_idx), vertices.get_mut(body_idx))
            {
                let inward = (-n.normalize_or_zero() * 0.55 + Vec3::Y * 0.45).normalize_or_zero();
                let p = Vec3::from_array(v.position) + inward * recess;
                v.position = p.to_array();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pelvic_anatomy_binds_and_sculpts_clitoris_sphincters_and_prolapse_on_female_model() {
        let demo = crate::female_demo::FemaleDemo::new().expect("load female demo");
        assert!(
            !demo.pelvic.is_empty(),
            "female model must bind high-resolution 3D pelvic/perineal anatomy"
        );
        assert_eq!(demo.pelvic.vertex_count(), GRID_U * GRID_V);

        let mesh = demo.mesh().expect("build female mesh with pelvic anatomy");
        let pelvic_vertices: Vec<_> = mesh
            .vertices()
            .iter()
            .filter(|v| v.uv[0] <= -1.035 && v.uv[0] >= -1.145)
            .collect();
        assert_eq!(pelvic_vertices.len(), GRID_U * GRID_V);

        let min_y = pelvic_vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = pelvic_vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_y - min_y > 0.008,
            "pelvic anatomy must have > 8 mm vertical 3D relief (got {} m)",
            max_y - min_y
        );
    }
}
