//! Editor checks for the shared retarget authoring profile.
pub use voxy_model::{ModelRetarget, RetargetJointProfile};

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_animation::Skeleton;
    #[test]
    fn defaults_round_trip_and_invalid_authoring_never_reaches_compilation() {
        let profile: ModelRetarget = serde_json::from_value(serde_json::json!({
            "source":"walk.glb","joints":[{"source":"hip","target":"pelvis"}]
        }))
        .unwrap();
        profile.validate().unwrap();
        assert_eq!(profile.joints[0].rotation_basis, [0., 0., 0., 1.]);
        assert_eq!(profile.joints[0].translation_basis, [0., 0., 0., 1.]);
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
        while app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .is_none()
        {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let profile: ModelRetarget = serde_json::from_value(serde_json::json!({
            "source":app.id.as_ref().expect("model-only fixture").clone().0,"joints":[{"source":"hip","target":"hip"}]
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
    #[test]
    fn source_only_rig_imports_and_survives_package_without_project_files() {
        use voxy_assets::{AssetId, PackageLimits, ResourcePackage};
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "voxy-retarget-package-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let bytes = include_bytes!("../../voxy_render/examples/assets/foot-contact.glb");
        let target_glb = gltf::binary::Glb::from_slice(bytes).unwrap();
        let mut target_json: serde_json::Value = serde_json::from_slice(&target_glb.json).unwrap();
        target_json["animations"] = serde_json::json!([]);
        let target_bytes = gltf::binary::Glb {
            header: target_glb.header,
            json: serde_json::to_vec(&target_json).unwrap().into(),
            bin: target_glb.bin,
        }
        .to_vec()
        .unwrap();
        std::fs::write(root.join("target.glb"), target_bytes).unwrap();
        let glb = gltf::binary::Glb::from_slice(bytes).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
        json["nodes"][0]["name"] = serde_json::json!("sourceHip");
        let source_bytes = gltf::binary::Glb {
            header: glb.header,
            json: serde_json::to_vec(&json).unwrap().into(),
            bin: glb.bin,
        }
        .to_vec()
        .unwrap();
        std::fs::write(root.join("source.glb"), &source_bytes).unwrap();
        let mut app = crate::App::new(&root.join("target.glb"), false).unwrap();
        let profile: ModelRetarget = serde_json::from_value(serde_json::json!({"source":"source.glb","joints":[{"source":"sourceHip","target":"hip"}]})).unwrap();
        app.scene
            .insert_component(app.instances[0], profile)
            .unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                crate::ModelAnimation {
                    clip_name: "move".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let source_id = AssetId("source.glb".into());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.catalog.snapshot(&source_id).is_none()
            || app
                .catalog
                .snapshot(app.id.as_ref().expect("model-only fixture"))
                .is_none()
        {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let imported = app.catalog.snapshot(&source_id).unwrap();
        let model = imported.value().animated.as_ref().unwrap();
        assert!(model.resolve_joint_name("sourceHip").is_ok());
        assert!(model.resolve_joint_name("hip").is_err());
        assert!(app.required_cpu_model_assets().contains(&source_id));
        assert!(!app.required_gpu_assets().contains(&source_id));
        assert!(imported.inputs().observations().contains_key(&source_id));
        app.validate_authoring_document(&app.authoring_document().unwrap())
            .unwrap();
        let scene = root.join("scene.json");
        std::fs::write(
            &scene,
            serde_json::to_vec(&app.authoring_document().unwrap()).unwrap(),
        )
        .unwrap();
        app.stop_workers().unwrap();
        let output = root.join("game.vpak");
        crate::export_game_package(
            &crate::ModelSource::File(root.join("target.glb")),
            &scene,
            &output,
        )
        .unwrap();
        let package = ResourcePackage::from_bytes(
            &std::fs::read(&output).unwrap(),
            PackageLimits {
                max_entries: 4096,
                max_payload_bytes: 64 * 1024 * 1024,
                max_document_bytes: 256 * 1024 * 1024,
            },
        )
        .unwrap();
        assert_eq!(
            package.read(&source_id, 64 * 1024 * 1024).unwrap(),
            source_bytes
        );
        for file in ["target.glb", "source.glb", "scene.json"] {
            std::fs::remove_file(root.join(file)).unwrap();
        }
        crate::run_packaged_game(&output, crate::ViewportMode::GameCheck).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
