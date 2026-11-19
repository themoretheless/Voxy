//! Physics joint authoring: kind switching, new-joint defaults and inspector rows.
//!
//! Adding, removing and retargeting reuse the follow-camera target picker in
//! `camera_authoring`; numeric members are edited through the generic component
//! inspector. Every edit is a whole-document candidate that is validated and then
//! committed to the authoring history.
use crate::component_fields::ComponentField;
use crate::*;
use serde_json::Value;
use voxy_gameplay::{PhysicsJoint, PhysicsJointKind};

pub(crate) const SCHEMA: &str = "game.physics-joint.v1";

/// Binds a presented joint button to the owner and descriptor it was drawn for.
pub(crate) fn request_token(
    object: &voxy_scene::SceneObject,
) -> Result<[u8; 32], serde_json::Error> {
    Ok(*blake3::hash(&serde_json::to_vec(&(
        &object.id,
        SCHEMA,
        object.components.get(SCHEMA),
    ))?)
    .as_bytes())
}

/// Readable inspector name of one joint member path.
pub(crate) fn field_label(path: &str) -> Option<String> {
    let (group, member) = path.rsplit_once('/')?;
    let axis = || {
        ["X", "Y", "Z", "W"]
            .get(member.parse::<usize>().ok()?)
            .copied()
    };
    Some(match (group, member) {
        ("", "target") => "Joint target".into(),
        ("/kind", "type") => "joint".into(),
        ("/kind", "rest_length") => "Rest length (m)".into(),
        ("/kind", "stiffness") => "Spring stiffness".into(),
        ("/kind", "damping") => "Spring damping".into(),
        ("/kind", "min_angle_rad") => "Min angle (rad)".into(),
        ("/kind", "max_angle_rad") => "Max angle (rad)".into(),
        ("/target_anchor", _) => format!("Target anchor {}", axis()?),
        ("/local_anchor", _) => format!("Own anchor {}", axis()?),
        ("/kind/axis", _) => format!("Hinge axis {}", axis()?),
        ("/kind/relative_rotation", _) => format!("Weld rotation {}", axis()?),
        _ => return None,
    })
}

/// Compact row text: joint numbers are single precision, so they are shown without
/// widening noise, and the kind tag reads as a word next to its "joint" label.
pub(crate) fn value_text(member: &ComponentField) -> Option<String> {
    if member.schema != SCHEMA {
        return None;
    }
    if member.path == "/kind/type" {
        return Some(
            match member.value.as_str()? {
                "fixed" => "Fixed",
                "hinge" => "Hinge",
                "distance_spring" => "Spring",
                other => other,
            }
            .into(),
        );
    }
    member
        .value
        .as_f64()
        .map(|value| format!("{}", value as f32))
}

/// Extra button on a joint row: target pick or kind switch.
pub(crate) fn row_button(
    object: &voxy_scene::SceneObject,
    member: &ComponentField,
) -> Result<Option<(&'static str, panels::Action)>, serde_json::Error> {
    if member.schema != SCHEMA {
        return Ok(None);
    }
    Ok(match member.path.as_str() {
        "/target" => Some(("Pick", panels::Action::JointTarget(request_token(object)?))),
        "/kind/type" => Some(("Change", panels::Action::JointKind(request_token(object)?))),
        _ => None,
    })
}

/// A hinge axis and a weld rotation must stay unit length, so an edited component
/// rescales its vector. Zero or non-finite vectors are left for validation to reject.
pub(crate) fn renormalize(object: &mut voxy_scene::SceneObject, schema: &str, path: &str) {
    let Some((vector, _)) = path.rsplit_once('/') else {
        return;
    };
    if schema != SCHEMA || !matches!(vector, "/kind/axis" | "/kind/relative_rotation") {
        return;
    }
    let Some(items) = object
        .components
        .get_mut(SCHEMA)
        .and_then(|joint| joint.pointer_mut(vector))
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    let values: Vec<f64> = items.iter().filter_map(Value::as_f64).collect();
    let length = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    if values.len() != items.len() || !length.is_finite() || length < 1e-6 {
        return;
    }
    for (item, value) in items.iter_mut().zip(values) {
        *item = Value::from((value / length) as f32);
    }
}

impl App {
    pub(crate) fn resolve_object(&self, id: &ObjectId) -> Option<NodeId> {
        self.object_ids
            .iter()
            .zip(&self.instances)
            .find_map(|(candidate, node)| (candidate == id).then_some(*node))
    }

    /// Play preflight: an invalid, unresolved or self-targeting joint stops Play before a tick.
    pub(crate) fn validate_live_joints(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(voxy_gameplay::validate_physics_joints(&self.scene, |id| {
            self.resolve_object(id)
        })?)
    }

    /// Authored world matrices of a joint owner and its target.
    fn joint_poses(
        &self,
        owner: &ObjectId,
        target: &ObjectId,
    ) -> Result<(Mat4, Mat4), Box<dyn std::error::Error>> {
        let owner = self
            .resolve_object(owner)
            .ok_or("physics joint owner is not in the scene")?;
        let target = self
            .resolve_object(target)
            .ok_or("physics joint target is not in the scene")?;
        Ok((
            self.scene.world_matrix(owner)?,
            self.scene.world_matrix(target)?,
        ))
    }

    /// Relative rotation that keeps the owner's authored orientation under a weld.
    fn weld_rotation(owner_world: Mat4, target_world: Mat4) -> [f32; 4] {
        let (_, owner_rotation, _) = owner_world.to_scale_rotation_translation();
        let (_, target_rotation, _) = target_world.to_scale_rotation_translation();
        (target_rotation.inverse() * owner_rotation)
            .normalize()
            .to_array()
    }

    /// Descriptor written by a target pick. A new joint is a weld that holds the
    /// authored relative pose; a retarget keeps anchors and kind.
    pub(crate) fn picked_joint(
        &self,
        owner: &ObjectId,
        expected: Option<Value>,
        target: ObjectId,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        if &target == owner {
            return Err("a physics joint cannot target its own object".into());
        }
        let joint = if let Some(expected) = expected {
            PhysicsJoint {
                target,
                ..serde_json::from_value(expected)?
            }
        } else {
            let (owner_world, target_world) = self.joint_poses(owner, &target)?;
            let anchor = target_world
                .inverse()
                .transform_point3(owner_world.w_axis.truncate());
            PhysicsJoint {
                target,
                target_anchor: anchor.to_array(),
                local_anchor: [0.; 3],
                kind: PhysicsJointKind::Fixed {
                    relative_rotation: Self::weld_rotation(owner_world, target_world),
                },
            }
        };
        if !joint.valid() {
            return Err("physics joint needs finite, invertible object transforms".into());
        }
        Ok(serde_json::to_value(joint)?)
    }

    /// Fixed -> Hinge -> `DistanceSpring` -> Fixed. Anchors and target are kept; the new
    /// kind starts from the authored pose (current anchor distance, current relative rotation).
    pub(crate) fn cycle_joint_kind(
        &mut self,
        token: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop play before editing a physics joint".into());
        }
        let mut document = self.authoring_document()?;
        let object = document
            .objects
            .get_mut(self.selected)
            .ok_or("missing physics joint owner")?;
        if request_token(object)? != token {
            return Err("physics joint changed before activation".into());
        }
        let mut joint: PhysicsJoint = serde_json::from_value(
            object
                .components
                .get(SCHEMA)
                .cloned()
                .ok_or("selected object has no physics joint")?,
        )?;
        let (owner_world, target_world) = self.joint_poses(&object.id, &joint.target)?;
        joint.kind = match joint.kind {
            PhysicsJointKind::Fixed { .. } => PhysicsJointKind::Hinge {
                axis: [0., 1., 0.],
                min_angle_rad: -std::f32::consts::FRAC_PI_2,
                max_angle_rad: std::f32::consts::FRAC_PI_2,
            },
            PhysicsJointKind::Hinge { .. } => PhysicsJointKind::DistanceSpring {
                rest_length: target_world
                    .transform_point3(Vec3::from_array(joint.target_anchor))
                    .distance(owner_world.transform_point3(Vec3::from_array(joint.local_anchor))),
                stiffness: 50.,
                damping: 2.,
            },
            PhysicsJointKind::DistanceSpring { .. } => PhysicsJointKind::Fixed {
                relative_rotation: Self::weld_rotation(owner_world, target_world),
            },
        };
        if !joint.valid() {
            return Err("physics joint needs finite object transforms".into());
        }
        object
            .components
            .insert(SCHEMA.into(), serde_json::to_value(joint)?);
        self.commit_follow_document(document)
    }

    /// Open the component page that holds the selected object's joint members.
    pub(crate) fn show_joint_fields(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let document = self.panel_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing physics joint owner")?;
        if let Some(index) = component_fields::fields(object)?
            .iter()
            .position(|field| field.schema == SCHEMA)
        {
            self.inspector = InspectorMode::Components(index / 6);
        }
        Ok(())
    }
}
