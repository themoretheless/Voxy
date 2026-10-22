//! Directional wave spectra and the initial Tessendorf amplitudes `h0(k)`.
//!
//! The amplitudes are generated on the CPU with the exact hashing and spectrum of the
//! original web-ocean `SIM_INIT` shader, then uploaded to the GPU. The same table feeds
//! the CPU wave queries, so physics and rendering see the same sea.

use std::f32::consts::{PI, TAU};

/// FFT resolution of one cascade.
pub const N: u32 = 256;
/// Number of cascades in a wave field.
pub const CASCADES: usize = 6;
pub(crate) const GRAV: f32 = 9.81;
/// Surface tension over density (capillary term of the dispersion relation).
const SURF: f32 = 7.4e-5;
/// Spectrum time loop (s); wave frequencies are quantised to it so the field repeats.
pub const LOOP: f32 = 600.0;

/// Spectrum of a wave field. Angles are in degrees, lengths in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveSpectrum {
    /// Patch size of each cascade, coarsest first; neighbouring cascades split the
    /// wavenumber range between them.
    pub cascades: [f32; CASCADES],
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

impl WaveSpectrum {
    /// Fetch-limited open sea with a long swell (the web-ocean defaults).
    pub fn ocean() -> Self {
        Self {
            cascades: [1103.0, 211.3, 39.7, 7.43, 1.393, 0.2617],
            wind_speed: 7.0,
            wind_direction: 118.0,
            fetch_m: 90_000.0,
            swell_height: 1.8,
            swell_direction: 94.0,
            swell_length: 120.0,
            swell_spread: 48.0,
            choppiness: 1.25,
            short_cut: 0.0012,
        }
    }

    /// Gale: steep, choppy, whitecapped sea.
    pub fn storm() -> Self {
        Self {
            wind_speed: 18.0,
            fetch_m: 400_000.0,
            swell_height: 3.5,
            swell_length: 180.0,
            choppiness: 1.45,
            ..Self::ocean()
        }
    }

    /// A lake a couple of kilometres across: short, wind-driven chop, no swell.
    pub fn lake() -> Self {
        Self {
            cascades: [97.0, 31.0, 9.7, 3.1, 0.97, 0.31],
            wind_speed: 6.0,
            wind_direction: 60.0,
            fetch_m: 2_000.0,
            swell_height: 0.0,
            swell_direction: 60.0,
            swell_length: 10.0,
            swell_spread: 8.0,
            choppiness: 1.0,
            short_cut: 0.002,
        }
    }

    /// Sheltered pond: barely moving, catspaw ripples.
    pub fn pond() -> Self {
        Self {
            cascades: [19.0, 7.7, 3.1, 1.2, 0.5, 0.2],
            wind_speed: 2.5,
            fetch_m: 40.0,
            choppiness: 0.5,
            short_cut: 0.004,
            ..Self::lake()
        }
    }

    /// Outdoor swimming pool: wind ripples over a short fetch and a faint slosh.
    pub fn pool() -> Self {
        Self {
            cascades: [9.7, 4.2, 1.8, 0.77, 0.33, 0.14],
            wind_speed: 4.0,
            wind_direction: 70.0,
            fetch_m: 25.0,
            swell_height: 0.012,
            swell_direction: 100.0,
            swell_length: 2.4,
            swell_spread: 6.0,
            choppiness: 0.6,
            short_cut: 0.004,
        }
    }

    /// Wavenumber band `[k_min, k_max)` (rad/m) carried by each cascade.
    pub fn bands(&self) -> [(f32, f32); CASCADES] {
        let l = &self.cascades;
        let mut bands = [(0.0, 0.0); CASCADES];
        for c in 0..CASCADES {
            let k_max = if c < CASCADES - 1 {
                TAU * 6.0 / l[c + 1]
            } else {
                PI * N as f32 / l[c] * 0.98
            };
            let k_min = if c == 0 { PI / l[c] } else { bands[c - 1].1 };
            bands[c] = (k_min, k_max);
        }
        bands
    }

    /// Variance spectrum in wavenumber space, `S(k)`.
    pub fn density(&self, k: [f32; 2]) -> f32 {
        let kl = (k[0] * k[0] + k[1] * k[1]).sqrt();
        if kl < 1e-5 {
            return 0.0;
        }
        let w = omega(kl);
        let theta = k[1].atan2(k[0]);
        let mut s = 0.0;
        let (u, f) = (self.wind_speed, self.fetch_m.max(1.0));
        if u > 0.1 {
            let alpha = 0.076 * (u * u / (f * GRAV)).powf(0.22);
            let wp = 22.0 * (GRAV * GRAV / (u * f)).powf(1.0 / 3.0);
            s = jonswap(w, wp, alpha, 3.3)
                * donelan_banner(wrap_angle(theta - self.wind_direction.to_radians()), w, wp);
        }
        if self.swell_height > 0.001 {
            let ws = (GRAV * TAU / self.swell_length).sqrt();
            let gs = 6.0f32;
            let hs4 = self.swell_height * 0.25;
            let alpha = hs4 * hs4 * 5.0 * ws.powi(4) / (GRAV * GRAV) * (1.0 - 0.287 * gs.ln());
            s += jonswap(w, ws, alpha, gs)
                * cos_spread(
                    wrap_angle(theta - self.swell_direction.to_radians()),
                    self.swell_spread,
                );
        }
        s *= (-kl * kl * self.short_cut * self.short_cut).exp();
        s * d_omega(kl) / kl
    }

    fn amplitude(&self, k: [f32; 2], cascade: usize, bands: &[(f32, f32); CASCADES]) -> f32 {
        let kl = (k[0] * k[0] + k[1] * k[1]).sqrt();
        let (k_min, k_max) = bands[cascade];
        if kl < 1e-5 || kl < k_min || kl >= k_max {
            return 0.0;
        }
        let dk = TAU / self.cascades[cascade];
        // the finest cascades are toned down (they otherwise read as crumpled foil)
        let gain = match cascade {
            5 => 0.6,
            4 => 0.85,
            _ => 1.0,
        };
        self.density(k).max(0.0).sqrt() * dk * 0.5 * gain
    }

    /// Initial amplitudes for every cascade, `(a.re, a.im, conj(b).re, conj(b).im)` per
    /// texel, laid out `[cascade][y][x]` exactly as the GPU evolution shader expects.
    pub fn initial_amplitudes(&self, seed: u32) -> Vec<[f32; 4]> {
        let bands = self.bands();
        let mut h0 = vec![[0.0f32; 4]; CASCADES * (N * N) as usize];
        for c in 0..CASCADES {
            let base = seed
                .wrapping_mul(7919)
                .wrapping_add((c as u32).wrapping_mul(1_000_003));
            for y in 0..N {
                for x in 0..N {
                    let k = wave_vector(x, y, self.cascades[c]);
                    let (mx, my) = ((N - x) % N, (N - y) % N);
                    let a = gauss2(base.wrapping_add(y * N + x));
                    let b = gauss2(base.wrapping_add(my * N + mx));
                    let aa = self.amplitude(k, c, &bands);
                    let ab = self.amplitude([-k[0], -k[1]], c, &bands);
                    h0[(c * (N * N) as usize) + (y * N + x) as usize] =
                        [a[0] * aa, a[1] * aa, b[0] * ab, -b[1] * ab];
                }
            }
        }
        h0
    }

    /// Significant wave height `4 sqrt(variance)` implied by the discrete amplitudes.
    pub fn significant_height(h0: &[[f32; 4]]) -> f32 {
        // height = sum over k of 2|a(k)| cos(...): variance = sum (2|a|)^2 / 2
        let variance: f64 = h0
            .iter()
            .map(|a| 2.0 * f64::from(a[0] * a[0] + a[1] * a[1]))
            .sum();
        4.0 * variance.sqrt() as f32
    }
}

pub(crate) fn wave_vector(x: u32, y: u32, patch: f32) -> [f32; 2] {
    let nx = if x >= N / 2 {
        x as i32 - N as i32
    } else {
        x as i32
    };
    let ny = if y >= N / 2 {
        y as i32 - N as i32
    } else {
        y as i32
    };
    [nx as f32 * TAU / patch, ny as f32 * TAU / patch]
}

/// Capillary-gravity dispersion relation.
pub fn omega(k: f32) -> f32 {
    (GRAV * k + SURF * k * k * k).sqrt()
}

/// The angular frequency the simulation uses: quantised so the field loops.
pub fn looped_omega(k: f32) -> f32 {
    let w0 = TAU / LOOP;
    (omega(k) / w0).floor() * w0
}

fn d_omega(k: f32) -> f32 {
    (GRAV + 3.0 * SURF * k * k) / (2.0 * omega(k))
}

fn jonswap(w: f32, wp: f32, alpha: f32, gamma: f32) -> f32 {
    let sigma = if w <= wp { 0.07 } else { 0.09 };
    let d = (w - wp) / (sigma * wp);
    let r = (-0.5 * d * d).exp();
    let q = wp / w;
    let q2 = q * q;
    alpha * GRAV * GRAV / w.powi(5) * (-1.25 * q2 * q2).exp() * gamma.powf(r)
}

/// Donelan-Banner directional spreading, normalised over [-pi, pi].
fn donelan_banner(theta: f32, w: f32, wp: f32) -> f32 {
    let ratio = w / wp;
    let beta = if ratio < 0.95 {
        2.61 * ratio.powf(1.3)
    } else if ratio < 1.6 {
        2.28 * ratio.powf(-1.3)
    } else {
        let eps = -0.4 + 0.8393 * (-0.567 * (ratio * ratio).ln()).exp();
        10f32.powf(eps)
    };
    let sh = 1.0 / (beta * theta).clamp(-20.0, 20.0).cosh();
    beta / (2.0 * (beta * PI).tanh()) * sh * sh
}

/// `cos^2s(theta/2)` spreading, normalised.
fn cos_spread(theta: f32, s: f32) -> f32 {
    let q = (s + 0.25).sqrt() / (2.0 * PI.sqrt());
    q * (theta * 0.5).cos().abs().powf(2.0 * s)
}

fn wrap_angle(a: f32) -> f32 {
    a - TAU * ((a + PI) / TAU).floor()
}

pub(crate) fn pcg_hash(v: u32) -> u32 {
    let s = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let w = ((s >> ((s >> 28) + 4)) ^ s).wrapping_mul(277_803_737);
    (w >> 22) ^ w
}

fn rand01(s: u32) -> f32 {
    ((pcg_hash(s) >> 8) as f32 + 0.5) / 16_777_216.0
}

fn gauss2(seed: u32) -> [f32; 2] {
    let u1 = rand01(seed.wrapping_mul(2).wrapping_add(17));
    let u2 = rand01(
        seed.wrapping_mul(2)
            .wrapping_add(18)
            .wrapping_add(pcg_hash(seed)),
    );
    let r = (-2.0 * u1.ln()).sqrt();
    let a = TAU * u2;
    [r * a.cos(), r * a.sin()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_tile_the_wavenumber_axis() {
        let bands = WaveSpectrum::ocean().bands();
        for c in 1..CASCADES {
            assert_eq!(bands[c].0, bands[c - 1].1);
            assert!(bands[c].1 > bands[c].0);
        }
    }

    /// The discrete fetch-limited sea matches the empirical JONSWAP growth law
    /// `Hs = 1.6e-3 U sqrt(F / g)` within the tolerance of the spectral shape.
    #[test]
    fn wind_sea_height_follows_fetch_law() {
        let spectrum = WaveSpectrum {
            swell_height: 0.0,
            ..WaveSpectrum::lake()
        };
        let hs = WaveSpectrum::significant_height(&spectrum.initial_amplitudes(1));
        let expected = 1.6e-3 * spectrum.wind_speed * (spectrum.fetch_m / GRAV).sqrt();
        assert!((hs / expected - 1.0).abs() < 0.35, "Hs {hs} vs {expected}");
    }

    #[test]
    fn amplitudes_are_hermitian_pairs() {
        let h0 = WaveSpectrum::pool().initial_amplitudes(3);
        // conj(b(k)) stored at k equals conj(a(-k)) stored at -k
        for (x, y) in [(3u32, 5u32), (17, 250), (100, 1)] {
            let (mx, my) = ((N - x) % N, (N - y) % N);
            let p = h0[(y * N + x) as usize];
            let m = h0[(my * N + mx) as usize];
            assert!((p[2] - m[0]).abs() < 1e-9 && (p[3] + m[1]).abs() < 1e-9);
        }
    }

    #[test]
    fn storm_is_taller_than_ocean_is_taller_than_pool() {
        let hs = |s: WaveSpectrum| WaveSpectrum::significant_height(&s.initial_amplitudes(1));
        let (storm, ocean, pool) = (
            hs(WaveSpectrum::storm()),
            hs(WaveSpectrum::ocean()),
            hs(WaveSpectrum::pool()),
        );
        assert!(
            storm > ocean && ocean > 1.0 && pool < 0.05,
            "{storm} {ocean} {pool}"
        );
    }
}
