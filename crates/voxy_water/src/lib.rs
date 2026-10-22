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
pub use shore::{Bathymetry, ShoreField, ShoreWaves};
pub use spectrum::{CASCADES, WaveSpectrum};

/// Shared WGSL functions: `shore_wave`, `flow_phases`, `foam_pattern`.
pub const WATER_COMMON_WGSL: &str = include_str!("shaders/water_common.wgsl");

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
    }
}
