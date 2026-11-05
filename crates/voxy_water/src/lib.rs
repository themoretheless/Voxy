//! Water generation for Voxy.
//!
//! * [`spectrum`]: directional wave spectra (JONSWAP wind sea + swell, capillary
//!   dispersion) with presets from a storm-driven ocean down to a swimming pool, and the
//!   Tessendorf initial amplitudes, generated deterministically on the CPU.
//! * [`query`]: CPU height, normal and orbital velocity of the same field, for
//!   gameplay and physics without GPU readback.
//! * [`bodies`]: basins, lake and sea regions and rivers in one [`WaterWorld`] that
//!   answers where the water is and how it moves.
//! * [`river`]: spline rivers whose speed follows the bed; rapids and waterfalls.
//! * [`shore`]: depth and waterline distance from bathymetry, shallow-water damping and
//!   breaking waves on beaches.
//! * [`buoyancy`]: Archimedes force, drag against currents and waves, and torque.
//! * [`gpu`]: the FFT wave field and an interactive ripple field (splashes, wakes) on
//!   WebGPU-baseline compute.
//! * [`mesh`]: surface meshes for every shape in one vertex format.
//! * [`WATER_COMMON_WGSL`]: shader helpers (breakers, flow advection, foam) matching the
//!   CPU models, for renderers.
//!
//! The wave shaders and spectrum come from web-ocean by Maksymilian Dendura (MIT, see
//! `LICENSE-web-ocean`).

pub mod bodies;
pub mod buoyancy;
pub mod gpu;
pub mod mesh;
pub mod query;
pub mod river;
pub mod shore;
pub mod spectrum;

pub use bodies::{WaterBody, WaterMaterial, WaterSample, WaterShape, WaterWorld};
pub use buoyancy::{BuoyancyForces, Floater, buoyancy};
pub use gpu::{RippleField, WaveSimulation};
pub use mesh::{WaterMesh, WaterVertex, surface_mesh};
pub use query::{SurfaceSample, WaveField};
pub use river::{River, RiverNode};
pub use shore::{
    Bathymetry, FluxLimiter, ShoreField, ShoreWaves, advect_foam_step,
};
pub use spectrum::{CASCADES, WaveSpectrum};

/// Shared WGSL functions: `shore_wave`, `flow_phases`, `foam_pattern`.
pub const WATER_COMMON_WGSL: &str = include_str!("shaders/water_common.wgsl");

/// Computes the angular frequency $\omega(k, h) = \sqrt{\left(g k + \frac{\sigma}{\rho} k^3\right)\tanh(k h)}$
/// (`rad/s`) of gravity-capillary surface waves on water of finite depth `depth_m` for wavenumber
/// `wavenumber_rad_m` ($k$), gravity `gravity_m_s2`, and capillary ratio $\sigma / \rho \approx 7.2\times 10^{-5}\text{ m}^3/\text{s}^2$.
#[must_use]
pub fn finite_depth_wave_omega(
    wavenumber_rad_m: f32,
    depth_m: f32,
    gravity_m_s2: f32,
    capillary_sigma_over_rho: f32,
) -> f32 {
    if !wavenumber_rad_m.is_finite()
        || !depth_m.is_finite()
        || !gravity_m_s2.is_finite()
        || !capillary_sigma_over_rho.is_finite()
        || wavenumber_rad_m <= 0.0
        || depth_m <= 0.0
        || gravity_m_s2 <= 0.0
        || capillary_sigma_over_rho < 0.0
    {
        return 0.0;
    }
    let k = wavenumber_rad_m;
    let kh = (k * depth_m).min(20.0);
    let omega_sq = (gravity_m_s2 * k + capillary_sigma_over_rho * k * k * k) * kh.tanh();
    omega_sq.max(0.0).sqrt()
}

/// Evaluates a normalized wave-breaking crest steepness / whitecap foam trigger in `[0.0, 1.0]`
/// based on the Miche–Michell limiting steepness $H_{\max} / \lambda \approx 0.142 \tanh(k h)$.
#[must_use]
pub fn wave_breaking_crest_factor(
    wave_height_m: f32,
    wavelength_m: f32,
    depth_m: f32,
) -> f32 {
    if !wave_height_m.is_finite()
        || !wavelength_m.is_finite()
        || !depth_m.is_finite()
        || wave_height_m <= 0.0
        || wavelength_m <= 1e-4
        || depth_m <= 1e-4
    {
        return 0.0;
    }
    let k = std::f32::consts::TAU / wavelength_m;
    let miche_limit = 0.142 * (k * depth_m).min(20.0).tanh().max(1e-4);
    let steepness = wave_height_m / wavelength_m;
    // Smooth onset starting at 70% of the Miche breaking limit.
    let u = ((steepness / miche_limit - 0.70) / 0.30).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

#[cfg(test)]
mod shader_tests {
    use wgpu::naga::{
        front::wgsl,
        valid::{Capabilities, ValidationFlags, Validator},
    };

    #[test]
    fn simulation_shaders_validate() {
        let helpers = format!(
            "{}\n@compute @workgroup_size(1) fn main() {{ _ = shore_wave(1., 2., 3., 0.4, 7., 30.); _ = flow_phases(1., 2.); _ = foam_pattern(vec2<f32>(1.), 0.5); }}",
            super::WATER_COMMON_WGSL
        );
        for (label, source) in [
            ("evolve", super::gpu::waves_sources()[0]),
            ("fft rows", super::gpu::waves_sources()[1]),
            ("fft cols", super::gpu::waves_sources()[2]),
            ("mips", super::gpu::waves_sources()[3]),
            ("ripples", super::gpu::ripples_source()),
            ("common", helpers.as_str()),
        ] {
            let module = wgsl::parse_str(source)
                .unwrap_or_else(|e| panic!("{label}: {}", e.emit_to_string(source)));
            Validator::new(ValidationFlags::all(), Capabilities::all())
                .validate(&module)
                .unwrap_or_else(|e| panic!("{label}: {}", e.emit_to_string(source)));
        }
        let shallow = super::finite_depth_wave_omega(1.0, 0.5, 9.81, 7.2e-5);
        let deep = super::finite_depth_wave_omega(1.0, 50.0, 9.81, 7.2e-5);
        assert!(deep > shallow && shallow > 0.0);
        assert_eq!(super::wave_breaking_crest_factor(0.1, 10.0, 5.0), 0.0);
        assert!(super::wave_breaking_crest_factor(2.0, 10.0, 5.0) > 0.8);
    }
}
