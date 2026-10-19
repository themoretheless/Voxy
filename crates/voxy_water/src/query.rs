//! CPU evaluation of a wave field for gameplay and physics: height, normal and orbital
//! velocity at any point and time, without GPU readback.
//!
//! The FFT field is `h(x, t) = sum_k 2 Re(a(k) e^{i(k.x - w t)})` over the same `h0`
//! table the GPU evolves, so a field built from the `count` most energetic components
//! tracks the rendered surface; the omitted short waves only carry centimetre detail.

use crate::spectrum::{CASCADES, LOOP, N, WaveSpectrum, looped_omega, wave_vector};
use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveComponent {
    /// Wave vector (rad/m) in the xz plane.
    pub k: Vec2,
    /// Crest-to-mean amplitude (m).
    pub amplitude: f32,
    pub phase: f32,
    /// Angular frequency (rad/s), quantised to the simulation loop.
    pub omega: f32,
}

/// Surface state at a horizontal position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceSample {
    /// Height of the displaced surface relative to the still-water level.
    pub height: f32,
    pub normal: Vec3,
    /// Orbital velocity of the surface particle (m/s).
    pub velocity: Vec3,
}

impl SurfaceSample {
    pub const FLAT: Self = Self { height: 0.0, normal: Vec3::Y, velocity: Vec3::ZERO };
}

#[derive(Clone, Debug)]
pub struct WaveField {
    components: Vec<WaveComponent>,
    choppiness: f32,
}

impl WaveField {
    /// Keeps the `count` most energetic components of `h0` (see
    /// [`WaveSpectrum::initial_amplitudes`]).
    pub fn from_amplitudes(spectrum: &WaveSpectrum, h0: &[[f32; 4]], count: usize) -> Self {
        let texels = (N * N) as usize;
        let mut components: Vec<WaveComponent> = Vec::with_capacity(h0.len() / 4);
        for c in 0..CASCADES {
            for y in 0..N {
                for x in 0..N {
                    let a = h0[c * texels + (y * N + x) as usize];
                    let magnitude = (a[0] * a[0] + a[1] * a[1]).sqrt();
                    if magnitude <= 0.0 {
                        continue;
                    }
                    let k = Vec2::from_array(wave_vector(x, y, spectrum.cascades[c]));
                    components.push(WaveComponent {
                        k,
                        amplitude: 2.0 * magnitude,
                        phase: a[1].atan2(a[0]),
                        omega: looped_omega(k.length()),
                    });
                }
            }
        }
        components.sort_by(|a, b| b.amplitude.total_cmp(&a.amplitude));
        components.truncate(count);
        Self { components, choppiness: spectrum.choppiness }
    }

    pub fn new(spectrum: &WaveSpectrum, seed: u32, count: usize) -> Self {
        Self::from_amplitudes(spectrum, &spectrum.initial_amplitudes(seed), count)
    }

    /// A still surface.
    pub fn calm() -> Self {
        Self { components: Vec::new(), choppiness: 0.0 }
    }

    pub fn components(&self) -> &[WaveComponent] {
        &self.components
    }

    /// Displacement of the surface particle that rests at `xz` (Lagrangian).
    pub fn displacement(&self, xz: Vec2, time: f32) -> Vec3 {
        let t = time % LOOP;
        let mut d = Vec3::ZERO;
        for w in &self.components {
            let phase = w.k.dot(xz) - w.omega * t + w.phase;
            let (s, c) = phase.sin_cos();
            let dir = w.k / w.k.length();
            d.y += w.amplitude * c;
            d.x -= self.choppiness * dir.x * w.amplitude * s;
            d.z -= self.choppiness * dir.y * w.amplitude * s;
        }
        d
    }

    /// Surface at the fixed world position `xz` (Eulerian): inverts the horizontal
    /// displacement with a few fixed-point steps, like the renderer's height probe.
    pub fn sample(&self, xz: Vec2, time: f32) -> SurfaceSample {
        if self.components.is_empty() {
            return SurfaceSample::FLAT;
        }
        let mut rest = xz;
        for _ in 0..4 {
            let d = self.displacement(rest, time);
            rest = xz - Vec2::new(d.x, d.z);
        }
        let t = time % LOOP;
        let (mut height, mut slope, mut velocity) = (0.0, Vec2::ZERO, Vec3::ZERO);
        let mut jacobian = Vec2::ONE;
        for w in &self.components {
            let phase = w.k.dot(rest) - w.omega * t + w.phase;
            let (s, c) = phase.sin_cos();
            let kl = w.k.length();
            let dir = w.k / kl;
            height += w.amplitude * c;
            slope -= w.k * (w.amplitude * s);
            jacobian -= dir * w.k * (self.choppiness * w.amplitude * c);
            velocity += Vec3::new(
                self.choppiness * dir.x * w.amplitude * w.omega * c,
                w.amplitude * w.omega * s,
                self.choppiness * dir.y * w.amplitude * w.omega * c,
            );
        }
        let slope = slope / jacobian.max(Vec2::splat(0.15));
        SurfaceSample { height, normal: Vec3::new(-slope.x, 1.0, -slope.y).normalize(), velocity }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strongest_components_carry_most_of_the_variance() {
        let spectrum = WaveSpectrum::ocean();
        let h0 = spectrum.initial_amplitudes(1);
        let full = WaveSpectrum::significant_height(&h0);
        let field = WaveField::from_amplitudes(&spectrum, &h0, 512);
        let kept: f32 = field.components.iter().map(|w| w.amplitude * w.amplitude * 0.5).sum();
        let hs = 4.0 * kept.sqrt();
        assert!(hs > 0.8 * full && hs <= full * 1.0001, "{hs} of {full}");
    }

    #[test]
    fn sample_matches_lagrangian_displacement() {
        let field = WaveField::new(&WaveSpectrum::lake(), 7, 128);
        let rest = Vec2::new(12.3, -4.0);
        let d = field.displacement(rest, 3.5);
        let at = field.sample(rest + Vec2::new(d.x, d.z), 3.5);
        assert!((at.height - d.y).abs() < 0.01, "{} vs {}", at.height, d.y);
        assert!(at.normal.y > 0.5);
    }

    #[test]
    fn surface_moves_and_loops() {
        let field = WaveField::new(&WaveSpectrum::ocean(), 1, 64);
        let p = Vec2::new(5.0, 9.0);
        assert!((field.sample(p, 0.0).height - field.sample(p, 2.0).height).abs() > 1e-3);
        assert!((field.sample(p, 1.0).height - field.sample(p, 1.0 + LOOP).height).abs() < 1e-3);
    }
}
