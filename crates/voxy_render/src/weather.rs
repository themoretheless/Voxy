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
    }
}
