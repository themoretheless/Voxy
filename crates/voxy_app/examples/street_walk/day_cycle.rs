//! Day/night, season and weather progression for the street walk.
//! The sun shadow map and temporal history are rebuilt on every sun,
//! exposure or weather change, so updates are applied in small steps
//! instead of every frame.
use glam::Vec3;
use voxy_render::archive_scene::ArchiveSceneRenderer;
use voxy_render::climate::{self, Calendar, Season, WeatherSchedule};
use voxy_render::weather::Weather;
use winit::keyboard::KeyCode;

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
    pub fn from_args(value: impl Fn(&str) -> Option<String>, seed: u64, weather_given: bool) -> Option<Self> {
        let number = |name: &str| value(name).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite());
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
        let base_ev = *self.base_ev.get_or_insert_with(|| renderer.exposure_ev100());
        let hours = if self.paused || frozen { 0. } else { real_seconds.min(0.25) * self.time_scale };
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
