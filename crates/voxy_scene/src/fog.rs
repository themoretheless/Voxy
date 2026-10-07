//! Renderer-independent authored optical medium; no GPU or simulation ownership.
use glam::Mat4;

/// Uniform medium in the owner's local box [0, size]. Optical coefficients
/// use world metres, regardless of owner scale. Lighting is supplied separately.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FogVolume {
    pub enabled: bool,
    pub size: [f32; 3],
    pub extinction_m_inverse: f64,
    pub single_scattering_albedo: f32,
    pub asymmetry: f32,
    pub samples: u32,
}
impl<'de> serde::Deserialize<'de> for FogVolume {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Data {
            enabled: bool,
            size: [f32; 3],
            extinction_m_inverse: f64,
            single_scattering_albedo: f32,
            asymmetry: f32,
            samples: u32,
        }
        let data = Data::deserialize(deserializer)?;
        let fog = Self {
            enabled: data.enabled,
            size: data.size,
            extinction_m_inverse: data.extinction_m_inverse,
            single_scattering_albedo: data.single_scattering_albedo,
            asymmetry: data.asymmetry,
            samples: data.samples,
        };
        fog.validate().map_err(serde::de::Error::custom)?;
        Ok(fog)
    }
}
impl Default for FogVolume {
    fn default() -> Self {
        Self {
            enabled: true,
            size: [1.; 3],
            extinction_m_inverse: 0.5,
            single_scattering_albedo: 0.8,
            asymmetry: 0.,
            samples: 32,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogBounds {
    pub origin: [f64; 3],
    pub extent: [f64; 3],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FogError {
    InvalidCalibration,
    UnsupportedTransform,
    UnrepresentableBounds,
}
impl std::fmt::Display for FogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fog volume error: {self:?}")
    }
}
impl std::error::Error for FogError {}
impl FogVolume {
    pub fn validate(&self) -> Result<(), FogError> {
        if self.size.iter().any(|v| !v.is_finite() || *v <= 0.)
            || !self.extinction_m_inverse.is_finite()
            || self.extinction_m_inverse < 0.
            || self.extinction_m_inverse > f64::from(f32::MAX)
            || (self.extinction_m_inverse > 0. && self.extinction_m_inverse as f32 == 0.)
            || !self.single_scattering_albedo.is_finite()
            || !(0. ..=1.).contains(&self.single_scattering_albedo)
            || !self.asymmetry.is_finite()
            || self.asymmetry.abs() >= 1.
            || !(1..=128).contains(&self.samples)
        {
            return Err(FogError::InvalidCalibration);
        }
        Ok(())
    }
    /// Exact axis-aligned box extraction, including negative scales and axis
    /// permutations. Arbitrary rotations/shears are rejected, never inflated
    /// to an AABB that would change the physical medium's occupied region.
    pub fn world_bounds(&self, world: Mat4) -> Result<FogBounds, FogError> {
        self.validate()?;
        if !world.is_finite()
            || world.x_axis.w != 0.
            || world.y_axis.w != 0.
            || world.z_axis.w != 0.
            || world.w_axis.w != 1.
        {
            return Err(FogError::UnsupportedTransform);
        }
        let columns = [
            world.x_axis.to_array(),
            world.y_axis.to_array(),
            world.z_axis.to_array(),
        ];
        if columns
            .iter()
            .any(|column| column[..3].iter().filter(|v| **v != 0.).count() != 1)
            || (0..3).any(|row| columns.iter().filter(|column| column[row] != 0.).count() != 1)
        {
            return Err(FogError::UnsupportedTransform);
        }
        let mut origin = world.w_axis.truncate().to_array().map(f64::from);
        let mut extent = [0.; 3];
        for row in 0..3 {
            for column in 0..3 {
                let span = f64::from(columns[column][row]) * f64::from(self.size[column]);
                origin[row] += span.min(0.);
                extent[row] += span.abs();
            }
        }
        if (0..3).any(|k| {
            let lo = origin[k] as f32;
            let span = extent[k] as f32;
            !lo.is_finite()
                || !span.is_finite()
                || span <= 0.
                || !(lo + span).is_finite()
                || lo + span <= lo
        }) {
            return Err(FogError::UnrepresentableBounds);
        }
        Ok(FogBounds { origin, extent })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_scale_preserves_per_metre_calibration_and_refuses_inflated_boxes() {
        let fog = FogVolume {
            size: [2., 3., 4.],
            ..Default::default()
        };
        let world = Mat4::from_scale_rotation_translation(
            glam::Vec3::new(-2., 3., 0.5),
            glam::Quat::IDENTITY,
            glam::Vec3::new(10., 20., 30.),
        );
        assert_eq!(
            fog.world_bounds(world).unwrap(),
            FogBounds {
                origin: [6., 20., 30.],
                extent: [4., 9., 2.]
            }
        );
        assert_eq!(fog.extinction_m_inverse, 0.5);
        assert_eq!(
            fog.world_bounds(Mat4::from_rotation_y(0.3)),
            Err(FogError::UnsupportedTransform)
        );
        assert_eq!(
            fog.world_bounds(Mat4::ZERO),
            Err(FogError::UnsupportedTransform)
        );
        assert_eq!(
            fog.world_bounds(Mat4::from_translation(glam::Vec3::splat(1e20))),
            Err(FogError::UnrepresentableBounds)
        );
        let permutation =
            Mat4::from_cols(glam::Vec4::Y, glam::Vec4::Z, glam::Vec4::X, glam::Vec4::W);
        assert_eq!(fog.world_bounds(permutation).unwrap().extent, [4., 2., 3.]);
    }
    #[test]
    fn calibration_rejects_invalid_optical_inputs() {
        for fog in [
            FogVolume {
                samples: 0,
                ..Default::default()
            },
            FogVolume {
                samples: 129,
                ..Default::default()
            },
            FogVolume {
                extinction_m_inverse: -1.,
                ..Default::default()
            },
            FogVolume {
                single_scattering_albedo: 1.1,
                ..Default::default()
            },
            FogVolume {
                asymmetry: 1.,
                ..Default::default()
            },
            FogVolume {
                size: [0.; 3],
                ..Default::default()
            },
        ] {
            assert!(fog.validate().is_err());
        }
        assert!(
            FogVolume {
                extinction_m_inverse: 0.,
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
        assert!(serde_json::from_str::<FogVolume>(r#"{"enabled":true,"size":[1,1,1],"extinction_m_inverse":0.5,"single_scattering_albedo":0.8,"asymmetry":0,"samples":32,"extra":0}"#).is_err());
    }
    #[test]
    fn durable_fog_roundtrips_and_invalid_history_edits_preserve_the_scene() {
        use crate::{ComponentRegistry, ObjectId, SceneDocument, SceneHistory, SceneObject};
        let mut registry = ComponentRegistry::default();
        registry.register::<FogVolume>("scene.fog.v1").unwrap();
        let original = SceneDocument {
            version: 1,
            objects: vec![SceneObject {
                id: ObjectId("fog".into()),
                parent: None,
                name: "Fog".into(),
                active: true,
                translation: [3., 4., 5.],
                rotation: [0., 0., 0., 1.],
                scale: [2., 1., 1.],
                components: std::collections::BTreeMap::from([(
                    "scene.fog.v1".into(),
                    serde_json::to_value(FogVolume::default()).unwrap(),
                )]),
            }],
        };
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded = SceneDocument::from_json(&encoded).unwrap();
        let loaded = decoded.load(&registry, 4).unwrap();
        assert_eq!(loaded.capture(&registry).unwrap(), original);
        let mut history = SceneHistory::new(original.clone(), &registry, 4, 8, 100_000).unwrap();
        history
            .edit(&registry, |d| {
                d.objects[0].components.get_mut("scene.fog.v1").unwrap()["extinction_m_inverse"] =
                    serde_json::json!(2.);
                Ok(())
            })
            .unwrap();
        let edited = history.current().clone();
        assert!(history.undo());
        assert_eq!(history.current(), &original);
        assert!(history.redo());
        assert_eq!(history.current(), &edited);
        assert!(
            history
                .edit(&registry, |d| {
                    d.objects[0].components.get_mut("scene.fog.v1").unwrap()["samples"] =
                        serde_json::json!(0);
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(history.current(), &edited);
        assert!(history.undo());
        assert!(history.redo());
        assert_eq!(history.current(), &edited);
    }
}
