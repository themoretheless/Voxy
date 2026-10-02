//! Explicit demo fluid properties, legacy primary source and named independent sources.
use physics::surface_film::Material;
use serde_json::{Value, json};
#[derive(Clone, Debug)]
pub struct FilmSettings {
    pub thickness_view: bool,
    pub self_contact_enabled: bool,
    pub self_contact_max_gap_m: f64,
    pub self_contact_transfer_speed_m_s: f64,
    pub precursor_wetting_enabled: bool,
    pub contact_angle_rad: f64,
    pub precursor_thickness_m: f64,
    pub material: Material,
    pub source_rate_m3_s: f64,
    pub source_center: [f64; 3],
    pub source_radius_m: f64,
    pub sources: Vec<FilmSource>,
    pub refractive_index: f64,
    pub absorption_rgb_per_m: [f64; 3],
}
#[derive(Clone, Debug)]
pub struct FilmSource {
    pub id: String,
    pub center: [f64; 3],
    pub radius_m: f64,
    pub rate_m3_s: f64,
}
impl FilmSource {
    fn to_json(&self) -> Value {
        json!({"id":self.id,"x_m":self.center[0],"y_m":self.center[1],"z_m":self.center[2],"radius_m":self.radius_m,"rate_m3_s":self.rate_m3_s})
    }
    fn from_json(value: &Value) -> Result<Self, &'static str> {
        let object = value.as_object().ok_or("source must be an object")?;
        if object
            .keys()
            .any(|k| !["id", "x_m", "y_m", "z_m", "radius_m", "rate_m3_s"].contains(&k.as_str()))
        {
            return Err("unknown source field");
        }
        let id = value["id"].as_str().ok_or("source requires id")?;
        if id.is_empty()
            || id.len() > 64
            || id == "primary"
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("invalid source id");
        }
        let number = |key: &str| value[key].as_f64().ok_or("missing numeric source field");
        let source = Self {
            id: id.to_string(),
            center: [number("x_m")?, number("y_m")?, number("z_m")?],
            radius_m: number("radius_m")?,
            rate_m3_s: number("rate_m3_s")?,
        };
        if source.center.iter().any(|v| !(-2. ..=2.).contains(v))
            || !(0.001..=0.5).contains(&source.radius_m)
            || !(0. ..=1e-6).contains(&source.rate_m3_s)
        {
            return Err("source outside preview range");
        }
        Ok(source)
    }
}
impl Default for FilmSettings {
    fn default() -> Self {
        Self {
            thickness_view: false,
            self_contact_enabled: false,
            self_contact_max_gap_m: 0.001,
            self_contact_transfer_speed_m_s: 0.0001,
            precursor_wetting_enabled: false,
            contact_angle_rad: 0.1,
            precursor_thickness_m: 1e-7,
            material: Material::default(),
            source_rate_m3_s: 0.,
            source_center: [0., 0.20, 0.13],
            source_radius_m: 0.04,
            sources: Vec::new(),
            refractive_index: 1.333,
            absorption_rgb_per_m: [0.2, 0.08, 0.04],
        }
    }
}
impl FilmSettings {
    pub fn to_json(&self) -> Value {
        json!({"self_contact_enabled":self.self_contact_enabled,"self_contact_max_gap_m":self.self_contact_max_gap_m,"self_contact_transfer_speed_m_s":self.self_contact_transfer_speed_m_s,"precursor_wetting_enabled":self.precursor_wetting_enabled,"contact_angle_rad":self.contact_angle_rad,"precursor_thickness_m":self.precursor_thickness_m,"display_mode":if self.thickness_view {"thickness"} else {"material"},"refractive_index":self.refractive_index,"absorption_r_per_m":self.absorption_rgb_per_m[0],"absorption_g_per_m":self.absorption_rgb_per_m[1],"absorption_b_per_m":self.absorption_rgb_per_m[2],"density":self.material.density,"viscosity":self.material.viscosity,
            "surface_tension":self.material.surface_tension,"wetting":self.material.wetting,
            "source_rate_m3_s":self.source_rate_m3_s,"source_x_m":self.source_center[0],"source_y_m":self.source_center[1],"source_z_m":self.source_center[2],"source_radius_m":self.source_radius_m,"sources":self.sources.iter().map(FilmSource::to_json).collect::<Vec<_>>()})
    }
    pub fn patched(&self, patch: &Value) -> Result<Self, &'static str> {
        let mut next = self.clone();
        for (key, value) in patch.as_object().ok_or("film settings must be an object")? {
            if key == "self_contact_enabled" {
                next.self_contact_enabled = value
                    .as_bool()
                    .ok_or("self_contact_enabled must be boolean")?;
                continue;
            }
            if key == "precursor_wetting_enabled" {
                next.precursor_wetting_enabled = value
                    .as_bool()
                    .ok_or("precursor_wetting_enabled must be boolean")?;
                continue;
            }
            if key == "display_mode" {
                next.thickness_view = match value.as_str() {
                    Some("material") => false,
                    Some("thickness") => true,
                    _ => return Err("invalid film display mode"),
                };
                continue;
            }
            if key == "sources" {
                let array = value.as_array().ok_or("sources must be an array")?;
                if array.len() > 16 {
                    return Err("too many film sources");
                }
                next.sources = array
                    .iter()
                    .map(FilmSource::from_json)
                    .collect::<Result<Vec<_>, _>>()?;
                let mut ids = std::collections::BTreeSet::new();
                if next.sources.iter().any(|s| !ids.insert(&s.id)) {
                    return Err("duplicate film source id");
                }
                continue;
            }
            let value = value.as_f64().ok_or("film setting must be a number")?;
            match key.as_str() {
                "self_contact_max_gap_m" => next.self_contact_max_gap_m = value,
                "self_contact_transfer_speed_m_s" => next.self_contact_transfer_speed_m_s = value,
                "contact_angle_rad" => next.contact_angle_rad = value,
                "precursor_thickness_m" => next.precursor_thickness_m = value,
                "refractive_index" => next.refractive_index = value,
                "absorption_r_per_m" => next.absorption_rgb_per_m[0] = value,
                "absorption_g_per_m" => next.absorption_rgb_per_m[1] = value,
                "absorption_b_per_m" => next.absorption_rgb_per_m[2] = value,
                "density" => next.material.density = value,
                "viscosity" => next.material.viscosity = value,
                "surface_tension" => next.material.surface_tension = value,
                "wetting" => next.material.wetting = value,
                "source_rate_m3_s" => next.source_rate_m3_s = value,
                "source_x_m" => next.source_center[0] = value,
                "source_y_m" => next.source_center[1] = value,
                "source_z_m" => next.source_center[2] = value,
                "source_radius_m" => next.source_radius_m = value,
                _ => return Err("unknown film setting"),
            }
        }
        next.material.validate()?;
        // Engineering bounds for this explicit preview, not physiological ranges.
        if !(1e-6..=0.01).contains(&next.self_contact_max_gap_m)
            || !(0. ..=0.1).contains(&next.self_contact_transfer_speed_m_s)
            || !(0. ..=0.5).contains(&next.contact_angle_rad)
            || !(1e-9..=1e-5).contains(&next.precursor_thickness_m)
            || !(1. ..=2.5).contains(&next.refractive_index)
            || next
                .absorption_rgb_per_m
                .iter()
                .any(|v| !(0. ..=10000.).contains(v))
            || !(100. ..=20000.).contains(&next.material.density)
            || !(0.0001..=100.).contains(&next.material.viscosity)
            || !(0. ..=1.).contains(&next.material.surface_tension)
            || !(0. ..=0.001).contains(&next.material.wetting)
            || !(0. ..=1e-6).contains(&next.source_rate_m3_s)
            || next.source_center.iter().any(|v| !(-2. ..=2.).contains(v))
            || !(0.001..=0.5).contains(&next.source_radius_m)
        {
            return Err("film setting outside preview range");
        }
        if next.sources.len() > 16 {
            return Err("too many film sources");
        }
        let mut ids = std::collections::BTreeSet::new();
        for source in &next.sources {
            FilmSource::from_json(&source.to_json())?;
            if !ids.insert(&source.id) {
                return Err("duplicate film source id");
            }
        }
        Ok(next)
    }
    pub fn read(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(
            Self::default().patched(&serde_json::from_str::<Value>(&std::fs::read_to_string(
                path,
            )?)?)?,
        )
    }
}
pub fn path(body: &std::path::Path) -> std::path::PathBuf {
    body.with_extension("film.json")
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn self_contact_settings_are_bounded_and_preserved() {
        let enabled=FilmSettings::default().patched(&json!({"self_contact_enabled":true,"self_contact_max_gap_m":0.002,"self_contact_transfer_speed_m_s":0.01})).unwrap();
        assert!(enabled.self_contact_enabled);
        assert_eq!(
            enabled
                .patched(&json!({"density":2000}))
                .unwrap()
                .self_contact_max_gap_m,
            0.002
        );
        for patch in [
            json!({"self_contact_enabled":1}),
            json!({"self_contact_max_gap_m":0}),
            json!({"self_contact_transfer_speed_m_s":-1}),
        ] {
            assert!(enabled.patched(&patch).is_err());
        }
        assert!(
            !FilmSettings::default()
                .patched(&json!({"density":1000}))
                .unwrap()
                .self_contact_enabled
        );
        assert_eq!(
            FilmSettings::default()
                .patched(&enabled.to_json())
                .unwrap()
                .to_json(),
            enabled.to_json()
        );
    }
    #[test]
    fn diagnostic_display_roundtrips_and_rejects_unknown_modes() {
        let settings = FilmSettings::default()
            .patched(&json!({"display_mode":"thickness"}))
            .unwrap();
        assert!(settings.thickness_view);
        assert_eq!(settings.to_json()["display_mode"], "thickness");
        assert!(
            FilmSettings::default()
                .patched(&settings.to_json())
                .unwrap()
                .thickness_view
        );
        assert!(
            settings
                .patched(&json!({"display_mode":"pressure"}))
                .is_err()
        );
        assert!(
            !settings
                .patched(&json!({"display_mode":"material"}))
                .unwrap()
                .thickness_view
        );
    }
    #[test]
    fn patch_validates_units_and_rejects_unknown_keys() {
        let s = super::FilmSettings::default();
        let changed = s
            .patched(&serde_json::json!({"density":2000,"source_rate_m3_s":1e-8}))
            .unwrap();
        assert_eq!(changed.material.viscosity, s.material.viscosity);
        assert!(s.patched(&serde_json::json!({"viscosity":0})).is_err());
        assert!(
            s.patched(&serde_json::json!({"source_rate_m3_s":-1}))
                .is_err()
        );
        assert!(s.patched(&serde_json::json!({"anything":1})).is_err());
        assert!(
            s.patched(&serde_json::json!({"refractive_index":0.9}))
                .is_err()
        );
        assert!(
            s.patched(&serde_json::json!({"absorption_r_per_m":-1}))
                .is_err()
        );
        assert_eq!(
            s.patched(&serde_json::json!({"refractive_index":1.4}))
                .unwrap()
                .refractive_index,
            1.4
        );
        let source = serde_json::json!({"id":"one","x_m":0,"y_m":0.2,"z_m":0.13,"radius_m":0.04,"rate_m3_s":1e-9});
        let many = s
            .patched(&serde_json::json!({"sources":[source.clone()]}))
            .unwrap();
        assert_eq!(many.sources.len(), 1);
        assert!(
            s.patched(&serde_json::json!({"sources":[source.clone(),source]}))
                .is_err()
        );
        assert!(many.patched(&serde_json::json!({"viscosity":0})).is_err());
    }
}
