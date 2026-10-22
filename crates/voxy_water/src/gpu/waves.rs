//! Tessendorf FFT wave field on the GPU: six 256² cascades evolved from CPU-generated
//! initial amplitudes, with displacement, slope/Jacobian and slope-moment texture arrays
//! and full mip chains (LEAN mapping).

use super::{bind_group, compute_pipeline, texture_array, view};
use crate::spectrum::{CASCADES, LOOP, N, WaveSpectrum};

const MIPS: u32 = N.ilog2() + 1;

pub const EVOLVE: &str = concat!(
    include_str!("../shaders/sim_common.wgsl"),
    include_str!("../shaders/sim_evolve.wgsl")
);
pub const FFT_ROWS: &str = concat!(
    include_str!("../shaders/fft_core.wgsl"),
    include_str!("../shaders/fft_rows.wgsl")
);
pub const FFT_COLS: &str = concat!(
    include_str!("../shaders/fft_core.wgsl"),
    include_str!("../shaders/fft_cols.wgsl")
);
pub const MIP_CHAIN: &str = include_str!("../shaders/sim_mips.wgsl");

#[derive(Debug)]
pub struct WaveSimulation {
    evolve: wgpu::ComputePipeline,
    rows: wgpu::ComputePipeline,
    cols: wgpu::ComputePipeline,
    mips: wgpu::ComputePipeline,
    evolve_group: wgpu::BindGroup,
    rows_group: wgpu::BindGroup,
    cols_group: wgpu::BindGroup,
    mip_groups: Vec<wgpu::BindGroup>,
    uniforms: wgpu::Buffer,
    h0: wgpu::Buffer,
    /// (Dx, Dy, Dz, ∂Dx/∂z) per cascade.
    pub displacement: wgpu::TextureView,
    /// (∂h/∂x, ∂h/∂z, ∂Dx/∂x, ∂Dz/∂z) per cascade.
    pub derivatives: wgpu::TextureView,
    /// (sx², sz², sx·sz, 0) per cascade; box-filtered mips give slope variance.
    pub moments: wgpu::TextureView,
    spectrum: WaveSpectrum,
    seed: u32,
    need_upload: bool,
}

impl WaveSimulation {
    pub fn new(device: &wgpu::Device, spectrum: WaveSpectrum, seed: u32) -> Self {
        use wgpu::{BindingResource as R, BufferUsages as BU, TextureViewDimension as D};
        let evolve = compute_pipeline(device, "wave evolve", EVOLVE);
        let rows = compute_pipeline(device, "wave fft rows", FFT_ROWS);
        let cols = compute_pipeline(device, "wave fft cols", FFT_COLS);
        let mips = compute_pipeline(device, "wave mips", MIP_CHAIN);
        let spec_bytes = u64::from(N * N) * CASCADES as u64 * 16;
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let uniforms = buffer("wave uniforms", 224, BU::UNIFORM | BU::COPY_DST);
        let h0 = buffer("wave h0", spec_bytes, BU::STORAGE | BU::COPY_DST);
        let spec_a = buffer("wave spectrum a", spec_bytes, BU::STORAGE);
        let spec_b = buffer("wave spectrum b", spec_bytes, BU::STORAGE);
        let tmp_a = buffer("wave fft a", spec_bytes, BU::STORAGE);
        let tmp_b = buffer("wave fft b", spec_bytes, BU::STORAGE);
        let layers = CASCADES as u32;
        let disp = texture_array(device, "wave displacement", N, layers, MIPS);
        let deriv = texture_array(device, "wave derivatives", N, layers, MIPS);
        let moments = texture_array(device, "wave slope moments", N, layers, MIPS);
        let level = |t: &wgpu::Texture, m: u32| view(t, Some(m), D::D2Array);
        let layout = |p: &wgpu::ComputePipeline| p.get_bind_group_layout(0);
        let evolve_group = bind_group(
            device,
            &layout(&evolve),
            "wave evolve",
            &[
                uniforms.as_entire_binding(),
                h0.as_entire_binding(),
                spec_a.as_entire_binding(),
                spec_b.as_entire_binding(),
            ],
        );
        let rows_group = bind_group(
            device,
            &layout(&rows),
            "wave fft rows",
            &[
                spec_a.as_entire_binding(),
                spec_b.as_entire_binding(),
                tmp_a.as_entire_binding(),
                tmp_b.as_entire_binding(),
            ],
        );
        let (d0, v0, m0) = (level(&disp, 0), level(&deriv, 0), level(&moments, 0));
        let cols_group = bind_group(
            device,
            &layout(&cols),
            "wave fft cols",
            &[
                tmp_a.as_entire_binding(),
                tmp_b.as_entire_binding(),
                R::TextureView(&d0),
                R::TextureView(&v0),
                R::TextureView(&m0),
            ],
        );
        let mip_groups = (1..MIPS)
            .map(|m| {
                let views = [
                    level(&disp, m - 1),
                    level(&deriv, m - 1),
                    level(&moments, m - 1),
                    level(&disp, m),
                    level(&deriv, m),
                    level(&moments, m),
                ];
                let resources: Vec<_> = views.iter().map(R::TextureView).collect();
                bind_group(device, &layout(&mips), "wave mip", &resources)
            })
            .collect();
        Self {
            evolve,
            rows,
            cols,
            mips,
            evolve_group,
            rows_group,
            cols_group,
            mip_groups,
            uniforms,
            h0,
            displacement: view(&disp, None, D::D2Array),
            derivatives: view(&deriv, None, D::D2Array),
            moments: view(&moments, None, D::D2Array),
            spectrum,
            seed,
            need_upload: true,
        }
    }

    pub fn spectrum(&self) -> &WaveSpectrum {
        &self.spectrum
    }

    /// Regenerates and uploads the initial amplitudes on the next encode when changed.
    pub fn set_spectrum(&mut self, spectrum: WaveSpectrum) {
        if spectrum != self.spectrum {
            self.spectrum = spectrum;
            self.need_upload = true;
        }
    }

    /// `1 / patch size` per cascade, as shaders sample the field with world xz.
    pub fn inverse_lengths(&self) -> [f32; CASCADES] {
        self.spectrum.cascades.map(|l| 1.0 / l)
    }

    fn write_uniforms(&self, queue: &wgpu::Queue, time: f32, dt: f32) {
        let s = &self.spectrum;
        let mut f = [0.0f32; 56];
        for (c, (band, l)) in s.bands().iter().zip(s.cascades).enumerate() {
            f[c] = l;
            f[8 + c] = band.0;
            f[16 + c] = band.1;
        }
        f[40..44].copy_from_slice(&[s.wind_speed, s.wind_direction.to_radians(), s.fetch_m, 3.3]);
        f[48..52].copy_from_slice(&[s.choppiness, s.short_cut, LOOP, self.seed as f32]);
        f[52..56].copy_from_slice(&[time % LOOP, dt, 2.2, 1.0]);
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&f));
    }

    /// Evolves the field to `time` (seconds) and rebuilds the mip chains.
    pub fn encode(
        &mut self,
        queue: &wgpu::Queue,
        pass: &mut wgpu::ComputePass,
        time: f32,
        dt: f32,
    ) {
        if self.need_upload {
            queue.write_buffer(
                &self.h0,
                0,
                bytemuck::cast_slice(&self.spectrum.initial_amplitudes(self.seed)),
            );
            self.need_upload = false;
        }
        self.write_uniforms(queue, time, dt);
        pass.set_pipeline(&self.evolve);
        pass.set_bind_group(0, &self.evolve_group, &[]);
        pass.dispatch_workgroups(N / 16, N / 16, CASCADES as u32);
        pass.set_pipeline(&self.rows);
        pass.set_bind_group(0, &self.rows_group, &[]);
        pass.dispatch_workgroups(N, 1, CASCADES as u32);
        pass.set_pipeline(&self.cols);
        pass.set_bind_group(0, &self.cols_group, &[]);
        pass.dispatch_workgroups(N, 1, CASCADES as u32);
        pass.set_pipeline(&self.mips);
        for (m, group) in (1..MIPS).zip(&self.mip_groups) {
            let s = (N >> m).div_ceil(8);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(s, s, CASCADES as u32);
        }
    }
}
