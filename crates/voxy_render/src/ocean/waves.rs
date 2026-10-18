//! Tessendorf FFT wave field shared by the open ocean and enclosed water bodies.
//! Six 256² cascades produce displacement, slope/Jacobian and slope-moment texture
//! arrays with full mip chains (LEAN mapping).

use super::{CASCADES, HDR, N, SIM_MIPS, bind_group, compute_pipeline, texture, view};

/// Spectrum of a wave field. Angles are in degrees, lengths in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveSpectrum {
    /// Patch size of each cascade, coarsest first; neighbouring cascades split the
    /// wavenumber range between them.
    pub cascades: [f32; CASCADES as usize],
    /// Wind speed at 10 m (m/s).
    pub wind_speed: f32,
    /// Direction the wind waves travel (90 = towards +Z).
    pub wind_direction: f32,
    pub fetch_m: f32,
    /// Swell significant height.
    pub swell_height: f32,
    pub swell_direction: f32,
    /// Swell peak wavelength.
    pub swell_length: f32,
    /// `cos^2s` spreading exponent of the swell.
    pub swell_spread: f32,
    /// Horizontal (choppy) displacement scale.
    pub choppiness: f32,
    /// Waves shorter than about this length are damped.
    pub short_cut: f32,
}

pub(super) const SPECTRUM: &str = concat!(
    include_str!("sim_common.wgsl"),
    include_str!("noise.wgsl"),
    include_str!("sim_init.wgsl")
);
pub(super) const EVOLVE: &str = concat!(include_str!("sim_common.wgsl"), include_str!("sim_evolve.wgsl"));
pub(super) const FFT_ROWS: &str = concat!(include_str!("fft_core.wgsl"), include_str!("fft_rows.wgsl"));
pub(super) const FFT_COLS: &str = concat!(include_str!("fft_core.wgsl"), include_str!("fft_cols.wgsl"));

/// Spectrum time loop (s); wave frequencies are quantised to it.
pub(super) const LOOP: f32 = 600.0;

#[derive(Debug)]
pub struct WaveSimulation {
    init: wgpu::ComputePipeline,
    evolve: wgpu::ComputePipeline,
    rows: wgpu::ComputePipeline,
    cols: wgpu::ComputePipeline,
    mips: wgpu::ComputePipeline,
    init_group: wgpu::BindGroup,
    evolve_group: wgpu::BindGroup,
    rows_group: wgpu::BindGroup,
    cols_group: wgpu::BindGroup,
    mip_groups: Vec<wgpu::BindGroup>,
    uniforms: wgpu::Buffer,
    /// (Dx, Dy, Dz, ∂Dx/∂z) per cascade.
    pub displacement: wgpu::TextureView,
    /// (∂h/∂x, ∂h/∂z, ∂Dx/∂x, ∂Dz/∂z) per cascade.
    pub derivatives: wgpu::TextureView,
    /// (sx², sz², sx·sz, 0) per cascade; box-filtered mips give slope variance.
    pub moments: wgpu::TextureView,
    spectrum: WaveSpectrum,
    need_spectrum: bool,
}

impl WaveSimulation {
    pub fn new(device: &wgpu::Device, spectrum: WaveSpectrum) -> Self {
        use wgpu::{BindingResource as R, BufferUsages as BU, TextureUsages as TU, TextureViewDimension as D};
        let init = compute_pipeline(device, "wave spectrum", SPECTRUM);
        let evolve = compute_pipeline(device, "wave evolve", EVOLVE);
        let rows = compute_pipeline(device, "wave fft rows", FFT_ROWS);
        let cols = compute_pipeline(device, "wave fft cols", FFT_COLS);
        let mips = compute_pipeline(device, "wave mips", include_str!("sim_mips.wgsl"));
        let spec_bytes = u64::from(N * N * CASCADES * 16);
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let uniforms = buffer("wave uniforms", 224, BU::UNIFORM | BU::COPY_DST);
        let h0 = buffer("wave h0", spec_bytes, BU::STORAGE);
        let spec_a = buffer("wave spectrum a", spec_bytes, BU::STORAGE);
        let spec_b = buffer("wave spectrum b", spec_bytes, BU::STORAGE);
        let tmp_a = buffer("wave fft a", spec_bytes, BU::STORAGE);
        let tmp_b = buffer("wave fft b", spec_bytes, BU::STORAGE);
        let sim_texture = |label| {
            texture(device, label, [N, N, CASCADES], HDR, SIM_MIPS, 1, TU::TEXTURE_BINDING | TU::STORAGE_BINDING)
        };
        let disp = sim_texture("wave displacement");
        let deriv = sim_texture("wave derivatives");
        let moments = sim_texture("wave slope moments");
        let level = |t: &wgpu::Texture, m: u32| view(t, Some(m), D::D2Array);
        let layout = |p: &wgpu::ComputePipeline| p.get_bind_group_layout(0);
        let init_group = bind_group(
            device,
            &layout(&init),
            "wave spectrum",
            &[uniforms.as_entire_binding(), h0.as_entire_binding()],
        );
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
        let mip_groups = (1..SIM_MIPS)
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
            init,
            evolve,
            rows,
            cols,
            mips,
            init_group,
            evolve_group,
            rows_group,
            cols_group,
            mip_groups,
            uniforms,
            displacement: view(&disp, None, D::D2Array),
            derivatives: view(&deriv, None, D::D2Array),
            moments: view(&moments, None, D::D2Array),
            spectrum,
            need_spectrum: true,
        }
    }

    pub fn spectrum_lengths(&self) -> [f32; CASCADES as usize] {
        self.spectrum.cascades
    }

    /// Regenerates the initial spectrum on the next encode when it changed.
    pub fn set_spectrum(&mut self, spectrum: WaveSpectrum) {
        if spectrum != self.spectrum {
            self.spectrum = spectrum;
            self.need_spectrum = true;
        }
    }

    fn write_uniforms(&self, queue: &wgpu::Queue, time: f32, dt: f32) {
        let s = &self.spectrum;
        let lengths = &s.cascades;
        let c = CASCADES as usize;
        let mut f = [0.0f32; 56];
        for i in 0..c {
            let l = lengths[i];
            f[i] = l;
            // kMax; cascade c starts where c-1 ends
            f[16 + i] = if i < c - 1 {
                std::f32::consts::TAU * 6.0 / lengths[i + 1]
            } else {
                std::f32::consts::PI * N as f32 / l * 0.98
            };
            f[8 + i] = if i == 0 { std::f32::consts::PI / l } else { f[16 + i - 1] };
        }
        f[40..44].copy_from_slice(&[s.wind_speed, s.wind_direction.to_radians(), s.fetch_m, 3.3]);
        let ks = std::f32::consts::TAU / s.swell_length;
        f[44..48].copy_from_slice(&[
            s.swell_height,
            s.swell_direction.to_radians(),
            (9.81 * ks).sqrt(),
            s.swell_spread,
        ]);
        f[48..52].copy_from_slice(&[s.choppiness, s.short_cut, LOOP, 1.0]);
        f[52..56].copy_from_slice(&[time, dt, 2.2, 1.0]);
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&f));
    }

    /// Evolves the field to `time` (seconds, looped by the caller) and rebuilds mips.
    pub fn encode(&mut self, queue: &wgpu::Queue, pass: &mut wgpu::ComputePass, time: f32, dt: f32) {
        self.write_uniforms(queue, time, dt);
        if self.need_spectrum {
            pass.set_pipeline(&self.init);
            pass.set_bind_group(0, &self.init_group, &[]);
            pass.dispatch_workgroups(N / 16, N / 16, CASCADES);
            self.need_spectrum = false;
        }
        pass.set_pipeline(&self.evolve);
        pass.set_bind_group(0, &self.evolve_group, &[]);
        pass.dispatch_workgroups(N / 16, N / 16, CASCADES);
        pass.set_pipeline(&self.rows);
        pass.set_bind_group(0, &self.rows_group, &[]);
        pass.dispatch_workgroups(N, 1, CASCADES);
        pass.set_pipeline(&self.cols);
        pass.set_bind_group(0, &self.cols_group, &[]);
        pass.dispatch_workgroups(N, 1, CASCADES);
        pass.set_pipeline(&self.mips);
        for (m, group) in (1..SIM_MIPS).zip(&self.mip_groups) {
            let s = (N >> m).div_ceil(8);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(s, s, CASCADES);
        }
    }
}
