//! Procedural atmosphere controls. Distances are metres and time is seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weather {
    pub coverage: f32,
    pub density: f32,
    pub rain: f32,
    pub snow: f32,
    /// Horizontal velocity in metres/second along world X/Z.
    pub wind: [f32; 2],
    /// Fractional gust amplitude, from steady (0) to strong gusts (1).
    pub wind_gust: f32,
    pub cloud_base: f32,
    pub cloud_thickness: f32,
}
impl Weather {
    pub const CLEAR: Self = Self {
        coverage: 0.,
        density: 0.,
        rain: 0.,
        snow: 0.,
        wind: [8., 3.],
        wind_gust: 0.35,
        cloud_base: 180.,
        cloud_thickness: 240.,
    };
    pub const CLOUDY: Self = Self {
        coverage: 0.55,
        density: 0.8,
        ..Self::CLEAR
    };
    pub const STORM: Self = Self {
        coverage: 0.95,
        density: 2.,
        rain: 0.85,
        ..Self::CLEAR
    };
    pub const SNOW: Self = Self {
        coverage: 0.85,
        density: 1.2,
        snow: 0.8,
        wind: [2., 1.],
        ..Self::CLEAR
    };
    /// Gust displacement is the integral of velocity, avoiding time * gust jumps.
    pub fn wind_displacement(self, seconds: f32) -> [f32; 2] {
        let t = seconds;
        let travel = t + self.wind_gust
            * (0.65 * (1. - (0.7 * t).cos()) / 0.7 + 0.35 * (1. - (1.9 * t).cos()) / 1.9);
        self.wind.map(|v| v * travel)
    }
    pub fn wind_velocity(self, seconds: f32) -> [f32; 2] {
        let strength =
            1. + self.wind_gust * (0.65 * (0.7 * seconds).sin() + 0.35 * (1.9 * seconds).sin());
        self.wind.map(|v| v * strength)
    }
    pub fn is_valid(self) -> bool {
        [self.coverage, self.rain, self.snow]
            .iter()
            .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            && self.density.is_finite()
            && (0. ..=4.).contains(&self.density)
            && self.wind_gust.is_finite()
            && (0. ..=1.).contains(&self.wind_gust)
            && self.wind.iter().all(|v| v.is_finite() && v.abs() <= 100.)
            && self.cloud_base.is_finite()
            && self.cloud_base >= 1.
            && self.cloud_thickness.is_finite()
            && self.cloud_thickness >= 1.
    }
}
impl Default for Weather {
    fn default() -> Self {
        Self::CLEAR
    }
}

/// Evaluates twilight atmospheric spectral transmittance `[r, g, b]` in `[0.0, 1.0]` combining
/// Rayleigh scattering, Mie aerosol extinction, and Chappuis-band stratospheric ozone absorption
/// (which peaks in orange-red $550\text{–}650\text{ nm}$ and produces deep cobalt-blue twilight
/// skies when `sun_elevation_deg` is in `[-12.0, 10.0]`).
#[must_use]
pub fn twilight_ozone_transmittance_rgb(sun_elevation_deg: f32, turbidity: f32) -> [f32; 3] {
    if !sun_elevation_deg.is_finite() || !turbidity.is_finite() {
        return [0.0; 3];
    }
    let elev = sun_elevation_deg.clamp(-18.0, 90.0);
    let t = turbidity.clamp(1.0, 10.0);
    // Relative optical airmass with spherical-shell horizon continuation down to -18 deg:
    let eff_deg = elev.max(-6.0) + 6.5;
    let air_mass = (1.0 / eff_deg.to_radians().sin().max(0.035)).min(38.0);
    // Ozone path length peaks near the horizon (stratospheric layer at ~22 km):
    let ozone_mass = (air_mass * 0.65).min(18.0);
    let beta_rayleigh = [0.0058_f32, 0.0135, 0.0331];
    let beta_mie = 0.0045 * t;
    // Chappuis ozone absorption cross-section is strongest in red/green, weak in blue:
    let beta_ozone = [0.065_f32, 0.048, 0.003];
    let night_fade = ((elev + 14.0) / 14.0).clamp(0.0, 1.0);
    std::array::from_fn(|i| {
        let tau = beta_rayleigh[i] * air_mass + beta_mie * air_mass + beta_ozone[i] * ozone_mass;
        ((-tau).exp() * night_fade).clamp(0.0, 1.0)
    })
}

/// Evaluates the spherical-shell Chapman grazing-incidence optical air-mass function `Ch(X, chi)`:
/// `sqrt(0.5 * pi * X) * exp(0.5 * X * cos^2(chi)) * erfc(sqrt(0.5 * X) * |cos(chi)|)` for `cos_zenith >= 0`,
/// where `scale_height_ratio = R_planet / H` (`~799.0` for Earth's Rayleigh scale height).
#[must_use]
pub fn chapman_optical_air_mass(cos_zenith: f32, scale_height_ratio: f32) -> f32 {
    if !cos_zenith.is_finite() || !scale_height_ratio.is_finite() || scale_height_ratio <= 1.0 {
        return 1.0;
    }
    let x = scale_height_ratio.clamp(10.0, 10_000.0);
    let mu = cos_zenith.clamp(-0.2, 1.0);
    if mu >= 0.0 {
        let y = (0.5 * x).sqrt() * mu;
        // Rational approximation to exp(y^2) * erfc(y) * sqrt(pi) (Tichy / Fitzmaurice):
        let ch = (1.060_696_3 + 0.556_438_3 * y) / (1.061_989_6 + 1.724_560_9 * y + y * y);
        ((0.5 * std::f32::consts::PI * x).sqrt() * ch).max(1.0)
    } else {
        let ch_90 = (0.5 * std::f32::consts::PI * x).sqrt();
        (ch_90 * (1.0 - 1.5 * mu)).min(60.0)
    }
}

/// Evaluates barometric atmospheric density decay `(rho_rayleigh, rho_mie) = (exp(-h / 8000), exp(-h / 1200))`
/// at `altitude_m` meters above sea level.
#[must_use]
pub fn atmosphere_scale_height_density(altitude_m: f32) -> (f32, f32) {
    if !altitude_m.is_finite() {
        return (0.0, 0.0);
    }
    let h = altitude_m.max(0.0);
    let rayleigh = (-h / 8000.0).exp().clamp(0.0, 1.0);
    let mie = (-h / 1200.0).exp().clamp(0.0, 1.0);
    (rayleigh, mie)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gust_velocity_matches_integrated_displacement() {
        let weather = Weather::STORM;
        for t in [0.5, 2., 8., 30.] {
            let before = weather.wind_displacement(t - 0.005);
            let after = weather.wind_displacement(t + 0.005);
            let velocity = weather.wind_velocity(t);
            for axis in 0..2 {
                assert!(((after[axis] - before[axis]) / 0.01 - velocity[axis]).abs() < 0.01);
            }
        }
        let calm = Weather {
            wind: [0., 0.],
            ..weather
        };
        assert_eq!(calm.wind_displacement(12.), [0., 0.]);
        assert_eq!(calm.wind_velocity(12.), [0., 0.]);
        assert!(
            !Weather {
                wind_gust: f32::NAN,
                ..weather
            }
            .is_valid()
        );
    }
    #[test]
    fn presets_and_invalid_inputs() {
        for w in [
            Weather::CLEAR,
            Weather::CLOUDY,
            Weather::STORM,
            Weather::SNOW,
        ] {
            assert!(w.is_valid());
        }
        assert!(
            !Weather {
                rain: f32::NAN,
                ..Weather::CLEAR
            }
            .is_valid()
        );
        assert!(
            !Weather {
                cloud_thickness: 0.,
                ..Weather::CLEAR
            }
            .is_valid()
        );
        let civil_twilight = twilight_ozone_transmittance_rgb(-3.0, 2.0);
        let noon = twilight_ozone_transmittance_rgb(60.0, 2.0);
        assert!(noon[0] > civil_twilight[0]);
        assert!(civil_twilight[0] > 0.0 && civil_twilight[2] > 0.0);

        let zenith_am = chapman_optical_air_mass(1.0, 800.0);
        let horizon_am = chapman_optical_air_mass(0.0, 800.0);
        assert!((zenith_am - 1.0).abs() < 0.05);
        assert!(horizon_am > 30.0);

        let (r0, m0) = atmosphere_scale_height_density(0.0);
        let (r_high, m_high) = atmosphere_scale_height_density(2400.0);
        assert_eq!((r0, m0), (1.0, 1.0));
        assert!(r_high > m_high && m_high > 0.0);
    }
}
