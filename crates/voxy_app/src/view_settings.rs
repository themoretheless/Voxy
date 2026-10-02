//! Persisted surface diagnostic mode; does not change simulation state.
use serde_json::{Value, json};
#[derive(Clone, Debug)]
pub struct ViewSettings {
    pub mode: String,
}
impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            mode: "material".into(),
        }
    }
}
impl ViewSettings {
    pub fn to_json(&self) -> Value {
        json!({"mode":self.mode})
    }
    pub fn patched(&self, patch: &Value) -> Result<Self, &'static str> {
        let object = patch.as_object().ok_or("view settings must be an object")?;
        if object.keys().any(|k| k != "mode") {
            return Err("unknown view setting");
        }
        let mode = match object.get("mode") {
            Some(value) => value.as_str().ok_or("mode must be a string")?,
            None => &self.mode,
        };
        if !["material", "displacement", "strain"].contains(&mode) {
            return Err("invalid view mode");
        }
        Ok(Self { mode: mode.into() })
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
    body.with_extension("view.json")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mode_validation_and_roundtrip() {
        let v = ViewSettings::default()
            .patched(&json!({"mode":"strain"}))
            .unwrap();
        assert_eq!(v.to_json()["mode"], "strain");
        assert!(v.patched(&json!({"mode":"pressure"})).is_err());
        assert!(v.patched(&json!({"other":1})).is_err());
    }
}
