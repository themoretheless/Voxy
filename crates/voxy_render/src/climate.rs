//! Calendar-driven sun, exposure and seasonal weather. World +X is east,
//! +Y is up and -Z is north. Times are hours, days are days of the year.
use crate::weather::Weather;
use glam::Vec3;

pub const HOURS_PER_YEAR: f64 = 365. * 24.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}
impl Season {
    /// Northern-hemisphere meteorological seasons.
    pub fn from_day(day: f64) -> Self {
        match (day.rem_euclid(365.) / 365. * 12.) as u32 {
            2..=4 => Self::Spring,
            5..=7 => Self::Summer,
            8..=10 => Self::Autumn,
            _ => Self::Winter,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Spring => "spring",
            Self::Summer => "summer",
            Self::Autumn => "autumn",
            Self::Winter => "winter",
        }
    }
    /// Mid-season day of the year.
    pub fn middle_day(self) -> f64 {
        match self {
            Self::Spring => 105.,
            Self::Summer => 196.,
            Self::Autumn => 288.,
            Self::Winter => 15.,
        }
    }
    /// Relative odds of clear, cloudy, rain and snow spells.
    fn odds(self) -> [f32; 4] {
        match self {
            Self::Spring => [4., 4., 3., 0.3],
            Self::Summer => [7., 3., 1.5, 0.],
            Self::Autumn => [3., 4., 4.5, 0.],
            Self::Winter => [3., 4., 0.8, 3.5],
        }
    }
}

/// Continuous time since 00:00 on 1 January.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calendar {
    pub hours: f64,
    pub latitude_degrees: f32,
}
impl Calendar {
    pub fn new(day: f64, hour: f64) -> Self {
        Self {
            hours: day * 24. + hour,
            latitude_degrees: 48.,
        }
    }
    pub fn advance(&mut self, hours: f64) {
        if hours.is_finite() {
            self.hours = (self.hours + hours).rem_euclid(HOURS_PER_YEAR);
        }
    }
    pub fn day(self) -> f64 {
        (self.hours / 24.).floor()
    }
    pub fn hour(self) -> f64 {
        self.hours.rem_euclid(24.)
    }
    pub fn season(self) -> Season {
        Season::from_day(self.hours / 24.)
    }
    /// Unit vector toward the sun from solar declination and hour angle.
    pub fn sun_direction(self) -> Vec3 {
        let day = self.hours / 24.;
        let declination =
            (-23.44_f64).to_radians() * (std::f64::consts::TAU * (day + 10.) / 365.).cos();
        let hour_angle = ((self.hour() - 12.) * 15.).to_radians();
        let latitude = (self.latitude_degrees as f64).to_radians();
        let up = latitude.sin() * declination.sin()
            + latitude.cos() * declination.cos() * hour_angle.cos();
        let east = -declination.cos() * hour_angle.sin();
        let north = latitude.cos() * declination.sin()
            - latitude.sin() * declination.cos() * hour_angle.cos();
        Vec3::new(east as f32, up as f32, -north as f32).normalize()
    }
}

/// Direct sun as [r, g, b, illuminance] with r/g/b peak-normalised. Rayleigh
/// and Mie extinction along the Kasten-Young air mass redden low sun; cloud
/// optical depth dims it.
pub fn sunlight(direction: Vec3, weather: Weather) -> [f32; 4] {
    let elevation = direction.y.clamp(-1., 1.).asin().to_degrees();
    if elevation < -1. {
        return [1., 0.6, 0.4, 0.];
    }
    let e = elevation.max(0.);
    let air_mass = 1. / (e.to_radians().sin() + 0.50572 * (e + 6.07995).powf(-1.6364));
    let tau = [0.046 + 0.025, 0.108 + 0.025, 0.265 + 0.025];
    let rgb = tau.map(|t| (-t * air_mass).exp());
    let peak = rgb.into_iter().fold(1e-6_f32, f32::max);
    let cloud = (-2.2 * weather.coverage * weather.density).exp();
    let horizon = ((elevation + 1.) / 3.).clamp(0., 1.);
    [
        rgb[0] / peak,
        rgb[1] / peak,
        rgb[2] / peak,
        105_000. * peak * cloud * horizon,
    ]
}

/// EV100 offset from the daylight exposure: 0 by day, about -7 at night
/// when only street lamps remain, with a twilight ramp.
pub fn exposure_offset(direction: Vec3, weather: Weather) -> f32 {
    let elevation = direction.y.clamp(-1., 1.).asin().to_degrees();
    let day = ((elevation + 8.) / 18.).clamp(0., 1.);
    let day = day * day * (3. - 2. * day);
    -7. * (1. - day) - 1.2 * weather.coverage * weather.density.min(1.) * day
}

pub fn lerp_weather(a: Weather, b: Weather, t: f32) -> Weather {
    let t = t.clamp(0., 1.);
    let m = |x: f32, y: f32| x + (y - x) * t;
    Weather {
        coverage: m(a.coverage, b.coverage),
        density: m(a.density, b.density),
        rain: m(a.rain, b.rain),
        snow: m(a.snow, b.snow),
        wind: [m(a.wind[0], b.wind[0]), m(a.wind[1], b.wind[1])],
        wind_gust: m(a.wind_gust, b.wind_gust),
        cloud_base: m(a.cloud_base, b.cloud_base),
        cloud_thickness: m(a.cloud_thickness, b.cloud_thickness),
    }
}

/// Deterministic weather spells of `SPELL_HOURS`, chosen by season odds and
/// blended over the final `BLEND_HOURS` of each spell.
#[derive(Clone, Copy, Debug)]
pub struct WeatherSchedule {
    pub seed: u64,
}
impl WeatherSchedule {
    pub const SPELL_HOURS: f64 = 4.;
    pub const BLEND_HOURS: f64 = 1.5;
    fn spell(self, index: i64) -> Weather {
        let mut h = self.seed ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        h ^= h >> 31;
        h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h ^= h >> 29;
        let unit = (h >> 40) as f32 / (1u64 << 24) as f32;
        let day = index as f64 * Self::SPELL_HOURS / 24.;
        let odds = Season::from_day(day).odds();
        let mut pick = unit * odds.iter().sum::<f32>();
        let mut kind = 0;
        for (i, w) in odds.iter().enumerate() {
            if pick < *w {
                kind = i;
                break;
            }
            pick -= w;
            kind = i;
        }
        let mut weather = [Weather::CLEAR, Weather::CLOUDY, Weather::STORM, Weather::SNOW][kind];
        // Wind direction drifts per spell so gusts do not always share an axis.
        let angle = (h & 0xffff) as f32 / 65536. * std::f32::consts::TAU;
        let speed = weather.wind[0].hypot(weather.wind[1]);
        weather.wind = [angle.cos() * speed, angle.sin() * speed];
        weather
    }
    pub fn at(self, hours: f64) -> Weather {
        let index = (hours / Self::SPELL_HOURS).floor();
        let into = hours - index * Self::SPELL_HOURS;
        let blend = ((into - (Self::SPELL_HOURS - Self::BLEND_HOURS)) / Self::BLEND_HOURS).clamp(0., 1.);
        let blend = blend * blend * (3. - 2. * blend);
        lerp_weather(self.spell(index as i64), self.spell(index as i64 + 1), blend as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sun_follows_day_and_season() {
        let noon_summer = Calendar::new(172., 12.).sun_direction();
        let noon_winter = Calendar::new(355., 12.).sun_direction();
        let midnight = Calendar::new(172., 0.).sun_direction();
        let morning = Calendar::new(80., 8.).sun_direction();
        // 48 degrees north: about 65.4 and 18.6 degrees at the solstices.
        assert!((noon_summer.y.asin().to_degrees() - 65.4).abs() < 0.5);
        assert!((noon_winter.y.asin().to_degrees() - 18.6).abs() < 0.5);
        assert!(noon_summer.z > 0., "noon sun is due south");
        assert!(midnight.y < 0.);
        assert!(morning.x > 0., "morning sun is in the east");
        assert_eq!(sunlight(midnight, Weather::CLEAR)[3], 0.);
        assert!(sunlight(noon_summer, Weather::CLEAR)[3] > 80_000.);
        let low = sunlight(Calendar::new(172., 20.).sun_direction(), Weather::CLEAR);
        assert!(low[2] < low[0], "low sun is reddened");
        assert!(sunlight(noon_summer, Weather::STORM)[3] < 20_000.);
        assert!(exposure_offset(midnight, Weather::CLEAR) < -6.9);
        assert_eq!(exposure_offset(noon_summer, Weather::CLEAR), 0.);
    }
    #[test]
    fn seasons_and_calendar_wrap() {
        assert_eq!(Season::from_day(15.), Season::Winter);
        assert_eq!(Season::from_day(100.), Season::Spring);
        assert_eq!(Season::from_day(200.), Season::Summer);
        assert_eq!(Season::from_day(300.), Season::Autumn);
        assert_eq!(Season::from_day(350.), Season::Winter);
        for s in [Season::Spring, Season::Summer, Season::Autumn, Season::Winter] {
            assert_eq!(Season::from_day(s.middle_day()), s);
        }
        let mut c = Calendar::new(364., 23.);
        c.advance(2.);
        assert_eq!(c.day(), 0.);
        assert!((c.hour() - 1.).abs() < 1e-9);
    }
    #[test]
    fn schedule_is_continuous_valid_and_seasonal() {
        let schedule = WeatherSchedule { seed: 7 };
        let mut snow_summer = 0.;
        let mut snow_winter = 0.;
        for step in 0..(HOURS_PER_YEAR as i64 * 4) {
            let hours = step as f64 * 0.25;
            let a = schedule.at(hours);
            let b = schedule.at(hours + 0.01);
            assert!(a.is_valid());
            assert!((a.coverage - b.coverage).abs() < 0.02, "no jumps at {hours}");
            match Season::from_day(hours / 24.) {
                Season::Summer => snow_summer += a.snow,
                Season::Winter => snow_winter += a.snow,
                _ => {}
            }
        }
        assert_eq!(snow_summer, 0.);
        assert!(snow_winter > 10.);
    }
}
