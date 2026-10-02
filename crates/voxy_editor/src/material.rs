//! Optional authored material overrides and a bounded directional light.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneMaterial {
    pub tint: [f32; 4],
    pub lit: bool,
}
impl Default for SceneMaterial {
    fn default() -> Self {
        Self {
            tint: [1.; 4],
            lit: true,
        }
    }
}
impl SceneMaterial {
    pub(crate) fn valid(self) -> bool {
        self.tint[3].to_bits() == 1_f32.to_bits()
            && self
                .tint
                .into_iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
    }
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    pub intensity: f32,
}
impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: [0.4, 0.8, 1.],
            intensity: 0.8,
        }
    }
}
impl DirectionalLight {
    pub(crate) fn valid(self) -> bool {
        let direction = glam::Vec3::from_array(self.direction);
        direction.is_finite()
            && direction.length_squared() > 1e-8
            && self.intensity.is_finite()
            && (0.0..=10.0).contains(&self.intensity)
    }
}
