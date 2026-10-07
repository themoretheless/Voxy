//! Durable, explicitly opaque shadow inputs; renderer resources stay session-owned.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionalShadowFilter {
    #[default]
    Hard,
    Pcf3x3,
    Pcf5x5,
}
/// World-space light camera box: X/Y are image width/height, Z is depth along
/// photon travel. The directional light's vector points toward the source.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct DirectionalShadow {
    pub enabled: bool,
    pub center: [f32; 3],
    pub extent: [f32; 3],
    pub up: [f32; 3],
    pub resolution: u32,
    pub bias: f32,
    pub filter: DirectionalShadowFilter,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShadowData {
    enabled: bool,
    center: [f32; 3],
    extent: [f32; 3],
    up: [f32; 3],
    resolution: u32,
    bias: f32,
    filter: DirectionalShadowFilter,
}
impl<'de> Deserialize<'de> for DirectionalShadow {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let d = ShadowData::deserialize(deserializer)?;
        let result = Self {
            enabled: d.enabled,
            center: d.center,
            extent: d.extent,
            up: d.up,
            resolution: d.resolution,
            bias: d.bias,
            filter: d.filter,
        };
        result.validate().map_err(serde::de::Error::custom)?;
        Ok(result)
    }
}
impl Default for DirectionalShadow {
    fn default() -> Self {
        Self {
            enabled: true,
            center: [0.; 3],
            extent: [10.; 3],
            up: [0., 1., 0.],
            resolution: 1024,
            bias: 0.0001,
            filter: DirectionalShadowFilter::Pcf3x3,
        }
    }
}
impl DirectionalShadow {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.center.into_iter().all(f32::is_finite)
            || !self.extent.into_iter().all(|v| v.is_finite() && v > 0.)
            || !self.up.into_iter().all(f32::is_finite)
            || self.up.into_iter().all(|v| v == 0.)
            || !(1..=4096).contains(&self.resolution)
            || !self.bias.is_finite()
            || !(0. ..=1.).contains(&self.bias)
        {
            return Err("invalid directional shadow calibration");
        }
        Ok(())
    }
    pub fn settings(
        self,
        light: crate::DirectionalLight,
    ) -> Result<voxy_render::ShadowSettings, String> {
        self.validate()?;
        if !light.valid() {
            return Err("invalid shadow light".into());
        }
        let d = glam::Vec3::from_array(light.direction);
        let direction = (d / d.abs().max_element()).normalize();
        let up = glam::Vec3::from_array(self.up);
        let up = (up / up.abs().max_element()).normalize();
        if direction.cross(up).length_squared() < 1e-8 {
            return Err("shadow camera up is parallel to light direction".into());
        }
        let center = glam::Vec3::from_array(self.center);
        let camera = voxy_render::SceneCamera {
            eye: center + direction * (self.extent[2] * 0.5),
            target: center,
            up,
            projection: voxy_render::SceneProjection::Orthographic {
                left: -self.extent[0] * 0.5,
                right: self.extent[0] * 0.5,
                bottom: -self.extent[1] * 0.5,
                top: self.extent[1] * 0.5,
                near: 0.,
                far: self.extent[2],
            },
        };
        let light_from_world = camera.view_projection().map_err(|e| e.to_string())?;
        let det = light_from_world.determinant();
        if !det.is_finite() || det == 0. || !light_from_world.inverse().is_finite() {
            return Err("unrepresentable shadow projection".into());
        }
        Ok(voxy_render::ShadowSettings {
            light_from_world,
            bias: self.bias,
            enabled: self.enabled,
            filter: match self.filter {
                DirectionalShadowFilter::Hard => voxy_render::ShadowFilter::Hard,
                DirectionalShadowFilter::Pcf3x3 => voxy_render::ShadowFilter::Pcf3x3,
                DirectionalShadowFilter::Pcf5x5 => voxy_render::ShadowFilter::Pcf5x5,
            },
        })
    }
}
/// Explicit opt-in to opaque triangle shadows. Texture alpha/transmission are
/// ignored; do not attach to transparent, cutout or liquid materials.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpaqueShadowCaster {
    pub enabled: bool,
}
impl Default for OpaqueShadowCaster {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub(crate) fn validate_scene(scene: &voxy_scene::SceneGraph) -> Result<(), String> {
    for (owner, shadow) in scene.components::<DirectionalShadow>() {
        shadow.validate()?;
        let light = scene
            .component::<crate::DirectionalLight>(owner)
            .map_err(|e| e.to_string())?
            .copied()
            .ok_or("directional shadow requires a directional light on its owner")?;
        shadow.settings(light)?;
    }
    for (owner, _) in scene.components::<OpaqueShadowCaster>() {
        if scene
            .component::<crate::ModelInstance>(owner)
            .map_err(|e| e.to_string())?
            .is_none()
            && scene
                .component::<String>(owner)
                .map_err(|e| e.to_string())?
                .is_none()
        {
            return Err("opaque shadow caster requires a model on its owner".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibrated_light_camera_and_invalid_authored_settings() {
        let shadow = DirectionalShadow {
            center: [0.5; 3],
            extent: [1., 1., 3.],
            ..Default::default()
        };
        let light = crate::DirectionalLight {
            direction: [0., 0., 1.],
            intensity: 4.,
        };
        let settings = shadow.settings(light).unwrap();
        let m = settings.light_from_world;
        assert!((m.project_point3(glam::Vec3::new(0.5, 0.5, 2.)).z).abs() < 1e-6);
        assert!((m.project_point3(glam::Vec3::new(0.5, 0.5, -1.)).z - 1.).abs() < 1e-6);
        assert!(
            shadow
                .settings(crate::DirectionalLight {
                    direction: [0., 1., 0.],
                    ..light
                })
                .is_err()
        );
        for (field, value) in [
            ("resolution", serde_json::json!(0)),
            ("resolution", serde_json::json!(4097)),
            ("extent", serde_json::json!([1, 0, 1])),
            ("bias", serde_json::json!(-0.1)),
            ("up", serde_json::json!([0, 0, 0])),
            ("extra", serde_json::json!(true)),
        ] {
            let mut json = serde_json::to_value(shadow).unwrap();
            json[field] = value;
            assert!(serde_json::from_value::<DirectionalShadow>(json).is_err());
        }
    }
    #[test]
    fn durable_shadow_components_history_and_owner_validation() {
        use voxy_scene::{ObjectId, SceneDocument, SceneHistory, SceneObject};
        let registry = crate::model_registry().unwrap();
        let object = |id: &str, components| SceneObject {
            id: ObjectId(id.into()),
            parent: None,
            name: id.into(),
            active: true,
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
            components,
        };
        let original = SceneDocument {
            version: 1,
            objects: vec![
                object(
                    "light",
                    std::collections::BTreeMap::from([
                        (
                            "editor.light.v1".into(),
                            serde_json::to_value(crate::DirectionalLight::default()).unwrap(),
                        ),
                        (
                            "editor.directional-shadow.v1".into(),
                            serde_json::to_value(DirectionalShadow::default()).unwrap(),
                        ),
                    ]),
                ),
                object(
                    "caster",
                    std::collections::BTreeMap::from([
                        ("editor.model.v1".into(), serde_json::json!("fixture")),
                        (
                            "editor.opaque-shadow-caster.v1".into(),
                            serde_json::to_value(OpaqueShadowCaster::default()).unwrap(),
                        ),
                    ]),
                ),
            ],
        };
        let loaded = original.load(&registry, 4).unwrap();
        validate_scene(&loaded.graph).unwrap();
        let mut canonical = original.clone();
        canonical.objects.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        assert_eq!(loaded.capture(&registry).unwrap(), canonical);
        let mut history = SceneHistory::new(original.clone(), &registry, 4, 8, 100_000).unwrap();
        history
            .edit(&registry, |doc| {
                doc.objects[0]
                    .components
                    .get_mut("editor.directional-shadow.v1")
                    .unwrap()["resolution"] = serde_json::json!(512);
                Ok(())
            })
            .unwrap();
        assert!(history.undo());
        assert_eq!(history.current(), &original);
        assert!(history.redo());
        let edited = history.current().clone();
        assert!(
            history
                .edit(&registry, |doc| {
                    doc.objects[0]
                        .components
                        .get_mut("editor.directional-shadow.v1")
                        .unwrap()["bias"] = serde_json::json!(2);
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(history.current(), &edited);
        let mut orphan = original.clone();
        orphan.objects[0].components.remove("editor.light.v1");
        assert!(validate_scene(&orphan.load(&registry, 4).unwrap().graph).is_err());
        let mut orphan = original.clone();
        orphan.objects[1].components.remove("editor.model.v1");
        assert!(validate_scene(&orphan.load(&registry, 4).unwrap().graph).is_err());
    }
}
