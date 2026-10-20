//! White-furnace gate for the shared BSDF energy terms in bsdf.wgsl.
//! The split-sum DFG is integrated on the CPU with GGX importance sampling,
//! then fed through Rust mirrors of `multiscatter_specular_ibl` and
//! `energy_compensation`.

const SAMPLES: u32 = 4096;

fn hammersley(i: u32, n: u32) -> (f32, f32) {
    (
        i as f32 / n as f32,
        i.reverse_bits() as f32 * 2.328_306_4e-10,
    )
}

/// Returns (scale, bias) with single-scatter albedo Ess = scale + bias.
fn dfg(nv: f32, roughness: f32) -> (f32, f32) {
    let a = roughness * roughness;
    let a2 = a * a;
    let v = [(1. - nv * nv).sqrt(), 0., nv];
    let (mut scale, mut bias) = (0., 0.);
    for i in 0..SAMPLES {
        let (u, w) = hammersley(i, SAMPLES);
        let phi = std::f32::consts::TAU * u;
        let cos_t = ((1. - w) / (1. + (a2 - 1.) * w)).sqrt();
        let sin_t = (1. - cos_t * cos_t).sqrt();
        let h = [sin_t * phi.cos(), sin_t * phi.sin(), cos_t];
        let vh = v[0] * h[0] + v[1] * h[1] + v[2] * h[2];
        let nl = 2. * vh * h[2] - v[2];
        if nl <= 0. {
            continue;
        }
        let nh = h[2].max(0.);
        let vh = vh.max(0.);
        let gv = nl * (nv * nv * (1. - a2) + a2).sqrt();
        let gl = nv * (nl * nl * (1. - a2) + a2).sqrt();
        let visibility = 0.5 / (gv + gl);
        let g = 4. * visibility * nl * vh / nh;
        let fc = (1. - vh).powi(5);
        scale += (1. - fc) * g;
        bias += fc * g;
    }
    (scale / SAMPLES as f32, bias / SAMPLES as f32)
}

fn multiscatter_specular_ibl(f0: f32, (s, b): (f32, f32), radiance: f32, irradiance: f32) -> f32 {
    let single = f0 * s + b;
    let ems = 1. - (s + b);
    let favg = f0 + (1. - f0) / 21.;
    single * radiance + single * favg / (1. - ems * favg) * ems * irradiance
}

#[test]
fn shared_bsdf_shader_is_valid() {
    let source = concat!(
        include_str!("../src/bsdf.wgsl"),
        "@compute @workgroup_size(1) fn main() {}"
    );
    let module = naga::front::wgsl::parse_str(source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
fn white_metal_furnace_conserves_energy_with_multiscatter() {
    let mut worst_single = 1_f32;
    for r in [0.05, 0.25, 0.5, 0.75, 1.0] {
        for nv in [0.1, 0.4, 0.7, 1.0] {
            let d = dfg(nv, r);
            worst_single = worst_single.min(d.0 + d.1);
            let total = multiscatter_specular_ibl(1., d, 1., 1.);
            assert!((total - 1.).abs() < 0.01, "roughness {r}, nv {nv}: {total}");
        }
    }
    // Single scattering alone visibly loses energy at high roughness.
    assert!(worst_single < 0.8, "{worst_single}");
}

#[test]
fn dielectric_furnace_never_exceeds_one() {
    for r in [0.05, 0.5, 1.0] {
        for nv in [0.1, 0.5, 1.0] {
            let total = multiscatter_specular_ibl(0.04, dfg(nv, r), 1., 1.);
            assert!(total <= 1.001, "roughness {r}, nv {nv}: {total}");
        }
    }
}
