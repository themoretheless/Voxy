//! Persisted, explicit retarget authoring data; compiled rig bindings stay immutable.
use std::sync::Arc;
use voxy_animation::{RetargetBinding, RetargetJoint, Skeleton};

fn identity_basis() -> [f32; 4] {
    [0., 0., 0., 1.]
}
fn unit_scale() -> f32 {
    1.
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetargetJointProfile {
    pub source: String,
    pub target: String,
    #[serde(default = "identity_basis")]
    pub rotation_basis: [f32; 4],
    #[serde(default = "identity_basis")]
    pub translation_basis: [f32; 4],
    #[serde(default = "unit_scale")]
    pub translation_scale: f32,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRetarget {
    pub source: String,
    pub joints: Vec<RetargetJointProfile>,
}
impl ModelRetarget {
    /// Validates authored data even while asynchronous model imports are pending.
    pub fn validate(&self) -> Result<(), String> {
        let valid_name =
            |name: &str| !name.is_empty() && name.len() <= 1024 && !name.contains('\0');
        if !valid_name(&self.source)
            || self.joints.is_empty()
            || self.joints.len() > voxy_animation::MAX_JOINTS
        {
            return Err("invalid retarget source or joint capacity".into());
        }
        let mut source_names = std::collections::BTreeSet::new();
        let mut target_names = std::collections::BTreeSet::new();
        for joint in &self.joints {
            let rotation = glam::Quat::from_array(joint.rotation_basis);
            let translation = glam::Quat::from_array(joint.translation_basis);
            if !valid_name(&joint.source)
                || !valid_name(&joint.target)
                || !source_names.insert(&joint.source)
                || !target_names.insert(&joint.target)
                || !rotation.is_finite()
                || !rotation.is_normalized()
                || !translation.is_finite()
                || !translation.is_normalized()
                || !joint.translation_scale.is_finite()
                || !(0. ..=1e6).contains(&joint.translation_scale)
            {
                return Err("invalid retarget joint profile".into());
            }
        }
        Ok(())
    }
    /// Resolves public imported bone names before binding canonical skeleton names.
    pub fn compile_models(
        &self,
        source: &voxy_render::ModelAsset,
        target: &voxy_render::ModelAsset,
    ) -> Result<RetargetBinding, String> {
        self.validate()?;
        let mapping = self
            .joints
            .iter()
            .map(|joint| {
                let source_index = source
                    .resolve_joint_name(&joint.source)
                    .map_err(|error| error.to_string())?;
                let target_index = target
                    .resolve_joint_name(&joint.target)
                    .map_err(|error| error.to_string())?;
                Ok(RetargetJoint {
                    source: source.skeleton.joints()[usize::from(source_index)]
                        .name
                        .clone(),
                    target: target.skeleton.joints()[usize::from(target_index)]
                        .name
                        .clone(),
                    rotation_basis: glam::Quat::from_array(joint.rotation_basis),
                    translation_basis: glam::Quat::from_array(joint.translation_basis),
                    translation_scale: joint.translation_scale,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        RetargetBinding::new(&source.skeleton, &target.skeleton, &mapping)
            .map_err(|error| error.to_string())
    }
    pub fn compile(&self, source: &Skeleton, target: &Skeleton) -> Result<RetargetBinding, String> {
        self.validate()?;
        let mapping: Vec<_> = self
            .joints
            .iter()
            .map(|joint| RetargetJoint {
                source: Arc::from(joint.source.as_str()),
                target: Arc::from(joint.target.as_str()),
                rotation_basis: glam::Quat::from_array(joint.rotation_basis),
                translation_basis: glam::Quat::from_array(joint.translation_basis),
                translation_scale: joint.translation_scale,
            })
            .collect();
        RetargetBinding::new(source, target, &mapping).map_err(|error| error.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_round_trip_and_invalid_authoring_never_reaches_compilation() {
        let profile: ModelRetarget = serde_json::from_value(serde_json::json!({
            "source":"walk.glb","joints":[{"source":"hip","target":"pelvis"}]
        }))
        .unwrap();
        profile.validate().unwrap();
        assert_eq!(profile.joints[0].rotation_basis, identity_basis());
        assert_eq!(profile.joints[0].translation_basis, identity_basis());
        assert_eq!(profile.joints[0].translation_scale, 1.);
        let encoded = serde_json::to_string(&profile).unwrap();
        assert_eq!(
            serde_json::from_str::<ModelRetarget>(&encoded).unwrap(),
            profile
        );
        let mut bad = profile.clone();
        bad.joints.push(bad.joints[0].clone());
        assert!(bad.validate().is_err());
        let mut bad = profile.clone();
        bad.joints[0].rotation_basis = [0.; 4];
        assert!(bad.validate().is_err());
        let mut bad = profile.clone();
        bad.joints[0].translation_scale = f32::INFINITY;
        assert!(bad.validate().is_err());
        let mut bad = profile;
        bad.source.clear();
        assert!(bad.validate().is_err());
    }
    #[test]
    fn persisted_names_compile_against_actual_rigs_and_reject_missing_bones() {
        let rig = |name: &str| {
            Skeleton::new(vec![voxy_animation::Joint {
                name: name.into(),
                parent: None,
                bind_local: voxy_animation::Transform::IDENTITY,
                inverse_bind: glam::Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let profile: ModelRetarget = serde_json::from_value(serde_json::json!({
            "source":"walk.glb","joints":[{"source":"hip","target":"pelvis"}]
        }))
        .unwrap();
        let source = rig("hip");
        let target = rig("pelvis");
        let pose = profile
            .compile(&source, &target)
            .unwrap()
            .apply_pose(&source.bind_pose())
            .unwrap();
        assert_eq!(pose, target.bind_pose());
        assert!(profile.compile(&source, &rig("other")).is_err());
    }
    #[test]
    fn editor_profile_history_round_trip_and_invalid_bone_edit_are_atomic() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/foot-contact.glb");
        let mut app = crate::App::new(&fixture, false).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.catalog.snapshot(&app.id).is_none() {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let profile: ModelRetarget = serde_json::from_value(serde_json::json!({
            "source":app.id.0,"joints":[{"source":"hip","target":"hip"}]
        }))
        .unwrap();
        app.scene
            .insert_component(app.instances[0], profile)
            .unwrap();
        app.commit_authoring().unwrap();
        let accepted = app.authoring_document().unwrap();
        let fields = crate::component_fields::fields(&accepted.objects[0]).unwrap();
        let target = fields
            .iter()
            .position(|field| {
                field.schema == "editor.model-retarget.v1" && field.path == "/joints/0/target"
            })
            .unwrap();
        assert!(app.edit_component_field(target, "missing").is_err());
        assert_eq!(app.authoring_document().unwrap(), accepted);
        app.edit_component_field(target, "knee").unwrap();
        let edited = app.authoring_document().unwrap();
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), accepted);
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        let decoded =
            voxy_scene::SceneDocument::from_json(&serde_json::to_string(&edited).unwrap()).unwrap();
        app.validate_authoring_document(&decoded).unwrap();
        assert_eq!(decoded, edited);
        app.stop_workers().unwrap();
    }
}
