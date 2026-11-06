//! Day/night, season and weather progression for the street walk.
//! The sun shadow map and temporal history are rebuilt on every sun,
//! exposure or weather change, so updates are applied in small steps
//! instead of every frame.
#![allow(dead_code)]
use glam::Vec3;
use voxy_render::archive_scene::ArchiveSceneRenderer;
use voxy_render::climate::{self, Calendar, Season, WeatherSchedule};
use voxy_render::weather::Weather;
use winit::keyboard::KeyCode;

/// Canonical time-of-day presets `(label, target_hour)` shared by `F9..F12` hotkeys and the `F1` settings panel.
pub const TIME_OF_DAY_PRESETS: [(&str, f64); 4] = [
    ("Утро (F9)", 7.5),
    ("Полдень (F10)", 12.5),
    ("Закат (F11)", 17.75),
    ("Ночь (F12)", 22.0),
];

/// Applies a weather preset while preserving the active wind vector and gust strength.
pub const fn apply_weather_preset_preserving_wind(current: Weather, preset: Weather) -> Weather {
    Weather {
        wind: current.wind,
        wind_gust: current.wind_gust,
        ..preset
    }
}

/// Preset combinations of season, time of day, and weather with smooth interpolation support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayWeatherPreset {
    ClearMorning,
    GoldenHour,
    RainyDusk,
    WinterBlizzard,
}

impl DayWeatherPreset {
    /// Returns the canonical `(day_of_year, hour_of_day, Weather)` target for this preset.
    pub fn parameters(self) -> (f64, f64, Weather) {
        match self {
            Self::ClearMorning => (Season::Summer.middle_day(), 8.5, Weather::CLEAR),
            Self::GoldenHour => (
                Season::Autumn.middle_day(),
                17.75,
                Weather {
                    coverage: 0.25,
                    density: 0.20,
                    rain: 0.0,
                    snow: 0.0,
                    wind: [1.5, 0.5],
                    ..Weather::CLEAR
                },
            ),
            Self::RainyDusk => (
                Season::Autumn.middle_day(),
                19.0,
                Weather::STORM,
            ),
            Self::WinterBlizzard => (
                Season::Winter.middle_day(),
                15.0,
                Weather::SNOW,
            ),
        }
    }

    /// Blends `calendar` and `weather` toward this preset by `alpha` in `[0.0, 1.0]`.
    pub fn apply_interpolated(self, calendar: &mut Calendar, weather: &mut Weather, alpha: f32) {
        let t = if alpha.is_finite() {
            alpha.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (target_day, target_hour, target_w) = self.parameters();
        let target_hours = target_day * 24.0 + target_hour;
        calendar.hours = calendar.hours + (target_hours - calendar.hours) * f64::from(t);
        weather.coverage += (target_w.coverage - weather.coverage) * t;
        weather.density += (target_w.density - weather.density) * t;
        weather.rain += (target_w.rain - weather.rain) * t;
        weather.snow += (target_w.snow - weather.snow) * t;
        weather.wind[0] += (target_w.wind[0] - weather.wind[0]) * t;
        weather.wind[1] += (target_w.wind[1] - weather.wind[1]) * t;
    }
}

pub struct DayCycle {
    pub calendar: Calendar,
    /// Simulated hours per real second.
    pub time_scale: f64,
    pub paused: bool,
    /// Weather follows the seasonal schedule until a preset key is pressed.
    pub auto_weather: bool,
    schedule: WeatherSchedule,
    wetness: f32,
    base_ev: Option<f32>,
    applied: Option<(Vec3, f32, Weather)>,
    reported: f64,
}
impl DayCycle {
    pub fn new(seed: u64) -> Self {
        Self {
            calendar: Calendar::new(Season::Summer.middle_day(), 9.),
            time_scale: 1. / 30.,
            paused: false,
            auto_weather: true,
            schedule: WeatherSchedule { seed },
            wetness: 0.,
            base_ev: None,
            applied: None,
            reported: f64::NEG_INFINITY,
        }
    }
    /// Base daylight exposure, restored when the cycle is switched off.
    pub fn base_ev(&self) -> Option<f32> {
        self.base_ev
    }
    /// Sets the hour of day on the current calendar day.
    pub fn set_hour_of_day(&mut self, hour: f64) {
        self.calendar.hours = self.calendar.day() * 24. + hour;
    }
    pub fn from_args(
        value: impl Fn(&str) -> Option<String>,
        seed: u64,
        weather_given: bool,
    ) -> Option<Self> {
        let number = |name: &str| {
            value(name)
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite())
        };
        let season = value("--season").and_then(|s| match s.as_str() {
            "spring" => Some(Season::Spring),
            "summer" => Some(Season::Summer),
            "autumn" | "fall" => Some(Season::Autumn),
            "winter" => Some(Season::Winter),
            _ => None,
        });
        let hour = number("--hour");
        let day = number("--day");
        let scale = number("--time-scale");
        if hour.is_none() && day.is_none() && season.is_none() && scale.is_none() {
            return None;
        }
        let day = day.unwrap_or_else(|| season.unwrap_or(Season::Summer).middle_day());
        let mut calendar = Calendar::new(day.rem_euclid(365.), hour.unwrap_or(9.).rem_euclid(24.));
        if let Some(latitude) = number("--latitude") {
            calendar.latitude_degrees = latitude.clamp(-89., 89.) as f32;
        }
        Some(Self {
            calendar,
            time_scale: scale.unwrap_or(1. / 30.).clamp(0., 24.),
            paused: false,
            auto_weather: !weather_given,
            schedule: WeatherSchedule { seed },
            wetness: 0.,
            base_ev: None,
            applied: None,
            reported: f64::NEG_INFINITY,
        })
    }
    /// Returns true when the key was consumed.
    pub fn key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::KeyT => self.paused = !self.paused,
            KeyCode::BracketLeft => self.calendar.advance(-1.),
            KeyCode::BracketRight => self.calendar.advance(1.),
            KeyCode::KeyY => self.calendar.advance(91.25 * 24.),
            KeyCode::F9 => self.set_hour_of_day(TIME_OF_DAY_PRESETS[0].1),
            KeyCode::F10 => self.set_hour_of_day(TIME_OF_DAY_PRESETS[1].1),
            KeyCode::F11 => self.set_hour_of_day(TIME_OF_DAY_PRESETS[2].1),
            KeyCode::F12 => self.set_hour_of_day(TIME_OF_DAY_PRESETS[3].1),
            KeyCode::Minus => self.time_scale = (self.time_scale * 0.5).max(1. / 3600.),
            KeyCode::Equal => self.time_scale = (self.time_scale * 2.).min(24.),
            KeyCode::Digit0 => self.auto_weather = true,
            KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3 | KeyCode::Digit4 => {
                self.auto_weather = false;
                return false;
            }
            _ => return false,
        }
        self.reported = f64::NEG_INFINITY;
        true
    }
    pub fn update(
        &mut self,
        renderer: &mut ArchiveSceneRenderer,
        weather: &mut Weather,
        real_seconds: f64,
        frozen: bool,
        wetness_override: bool,
    ) -> Result<(), voxy_render::ComputeError> {
        let base_ev = *self
            .base_ev
            .get_or_insert_with(|| renderer.exposure_ev100());
        let hours = if self.paused || frozen {
            0.
        } else {
            real_seconds.min(0.25) * self.time_scale
        };
        self.calendar.advance(hours);
        if self.auto_weather {
            let target = self.schedule.at(self.calendar.hours);
            let step = [
                target.coverage - weather.coverage,
                target.density - weather.density,
                target.rain - weather.rain,
                target.snow - weather.snow,
            ]
            .iter()
            .fold(0_f32, |m, v| m.max(v.abs()));
            let gust = (target.wind[0] - weather.wind[0]).hypot(target.wind[1] - weather.wind[1]);
            if step > 0.02 || gust > 0.5 {
                *weather = target;
            }
        }
        // Rain soaks the street quickly; it dries over about three hours.
        self.wetness = if weather.rain > self.wetness {
            weather.rain
        } else {
            (self.wetness - hours as f32 / 3.).max(weather.rain)
        };
        if !wetness_override {
            renderer.set_surface_wetness(Some(self.wetness.clamp(0., 1.)))?;
        }
        let sun = self.calendar.sun_direction();
        let ev = base_ev + climate::exposure_offset(sun, *weather);
        let stale = self.applied.is_none_or(|(s, e, w)| {
            s.dot(sun) < 0.15_f32.to_radians().cos() || (e - ev).abs() > 0.05 || w != *weather
        });
        if stale {
            renderer.set_sun(sun, climate::sunlight(sun, *weather))?;
            renderer.set_exposure_ev100(ev)?;
            self.applied = Some((sun, ev, *weather));
        }
        if self.calendar.hours - self.reported > 1. || self.calendar.hours < self.reported {
            self.reported = self.calendar.hours;
            let hour = self.calendar.hour();
            eprintln!(
                "STREET CLIMATE day {} {:02}:{:02} {} sun_elevation={:.1} weather={} rain={:.2} snow={:.2} wet={:.2} ev100={:.2}{}",
                self.calendar.day() as u32 + 1,
                hour as u32,
                (hour.fract() * 60.) as u32,
                self.calendar.season().name(),
                sun.y.asin().to_degrees(),
                if self.auto_weather { "auto" } else { "manual" },
                weather.rain,
                weather.snow,
                self.wetness,
                ev,
                if self.paused { " paused" } else { "" },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_cycle_keys_scrub_time_season_and_speed() {
        let mut cycle = DayCycle::new(0x564f5859);
        assert!(!cycle.paused);
        assert!((cycle.calendar.hour() - 9.0).abs() < 1e-4);
        assert_eq!(cycle.calendar.season(), Season::Summer);

        assert!(cycle.key(KeyCode::KeyT));
        assert!(cycle.paused);
        assert!(cycle.key(KeyCode::BracketRight));
        assert!((cycle.calendar.hour() - 10.0).abs() < 1e-4);
        assert!(cycle.key(KeyCode::BracketLeft));
        assert!((cycle.calendar.hour() - 9.0).abs() < 1e-4);
        assert!(cycle.key(KeyCode::KeyY));
        assert_eq!(cycle.calendar.season(), Season::Autumn);

        let initial_scale = cycle.time_scale;
        assert!(cycle.key(KeyCode::Equal));
        assert!((cycle.time_scale - initial_scale * 2.0).abs() < 1e-6);
        assert!(cycle.key(KeyCode::Minus));
        assert!((cycle.time_scale - initial_scale).abs() < 1e-6);

        // Weather preset keys disable auto_weather and pass through so main.rs applies the preset.
        assert!(!cycle.key(KeyCode::Digit3));
        assert!(!cycle.auto_weather);
        assert!(cycle.key(KeyCode::Digit0));
        assert!(cycle.auto_weather);

        // Time-of-day presets F9..F12 jump directly to Morning, Noon, Golden Hour, and Night on the current day.
        let day = cycle.calendar.day();
        assert!(cycle.key(KeyCode::F9));
        assert_eq!(cycle.calendar.day(), day);
        assert!((cycle.calendar.hour() - 7.5).abs() < 1e-4);
        assert!(cycle.key(KeyCode::F10));
        assert!((cycle.calendar.hour() - 12.5).abs() < 1e-4);
        assert!(cycle.key(KeyCode::F11));
        assert!((cycle.calendar.hour() - 17.75).abs() < 1e-4);
        assert!(cycle.key(KeyCode::F12));
        assert!((cycle.calendar.hour() - 22.0).abs() < 1e-4);
    }

    #[test]
    fn day_weather_preset_interpolates_smoothly() {
        let mut cal = Calendar::new(Season::Summer.middle_day(), 12.0);
        let mut weather = Weather::CLEAR;
        DayWeatherPreset::WinterBlizzard.apply_interpolated(&mut cal, &mut weather, 0.5);
        assert!(weather.snow > 0.0 && weather.snow < Weather::SNOW.snow);
        DayWeatherPreset::WinterBlizzard.apply_interpolated(&mut cal, &mut weather, 1.0);
        assert!((weather.snow - Weather::SNOW.snow).abs() < 1e-5);
        assert_eq!(cal.season(), Season::Winter);
    }
}
