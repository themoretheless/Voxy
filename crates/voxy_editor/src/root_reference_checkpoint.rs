//! Exact root-reference state; no approximate transform replaces an interval.
use super::*;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RootReferenceCheckpoint {
    version: u32,
    scale: voxy_animation::RootUniformScaleCheckpoint,
    body_to_world: voxy_animation::RootRigidEnclosureCheckpoint,
    authored_to_body: voxy_animation::RootRigidEnclosureCheckpoint,
}
#[allow(dead_code)]
impl RootReference {
    pub(super) fn checkpoint(&self) -> RootReferenceCheckpoint {
        RootReferenceCheckpoint {
            version: 1,
            scale: self.scale.checkpoint(),
            body_to_world: self.body_to_world.checkpoint(),
            authored_to_body: self.authored_to_body.checkpoint(),
        }
    }
}
#[allow(dead_code)]
impl RootReferenceCheckpoint {
    pub(super) fn prepare(&self) -> Result<RootReference, EditorError> {
        if self.version != 1 {
            return Err("unsupported root reference checkpoint".into());
        }
        Ok(RootReference {
            scale: self.scale.prepare()?,
            body_to_world: self.body_to_world.prepare()?,
            authored_to_body: self.authored_to_body.prepare()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_json_retains_all_intervals_and_rejects_invalid_unit_box() {
        let transform = voxy_animation::RootRigidTransform {
            translation: glam::DVec3::new(1.2, -0., 0.03),
            rotation: glam::DQuat::from_rotation_y(0.731),
        };
        let body = voxy_animation::RootRigidEnclosure::from_transform(transform).unwrap();
        let original = RootReference {
            scale: voxy_animation::RootUniformScaleEnclosure::from_scale(-1.3)
                .unwrap()
                .multiplied(voxy_animation::RootUniformScaleEnclosure::from_scale(0.731).unwrap())
                .unwrap(),
            body_to_world: body,
            authored_to_body: body
                .inverse()
                .unwrap()
                .compose(&voxy_animation::RootRigidEnclosure::IDENTITY)
                .unwrap(),
        };
        let json = serde_json::to_string(&original.checkpoint()).unwrap();
        let saved: RootReferenceCheckpoint = serde_json::from_str(&json).unwrap();
        let restored = saved.prepare().unwrap();
        assert!(original.matches(&restored));
        assert_eq!(serde_json::to_string(&restored.checkpoint()).unwrap(), json);
        let mut wire: serde_json::Value = serde_json::from_str(&json).unwrap();
        wire["authored_to_body"]["rotation"] = serde_json::json!([[0, 0], [0, 0], [0, 0], [0, 0]]);
        let bad: RootReferenceCheckpoint = serde_json::from_value(wire).unwrap();
        assert!(bad.prepare().is_err());
        assert_eq!(serde_json::to_string(&original.checkpoint()).unwrap(), json);
    }
}
