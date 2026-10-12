use super::*;

#[test]
fn split_view_input_keeps_cameras_independent_and_drag_coordinates_local() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.camera.legacy = false;
    app.camera.perspective = false;
    app.camera.distance = 4.;
    let before = app.authoring_document().unwrap();
    app.edit_key(KeyCode::F10).unwrap();
    assert!(app.msaa4);
    app.edit_key(KeyCode::F4).unwrap();
    assert!(app.split_views());
    let (local, size, origin) = app
        .view_pointer(Vec2::new(750., 350.), Vec2::new(1001., 700.))
        .unwrap()
        .unwrap();
    assert_eq!(
        (local, size, origin),
        (
            Vec2::new(250., 350.),
            Vec2::new(501., 700.),
            Vec2::new(500., 0.)
        )
    );
    assert_eq!(app.active_view, 1);
    assert!(
        app.view_pointer(Vec2::ZERO, Vec2::new(1., 700.))
            .unwrap()
            .is_some()
    );
    assert_eq!(app.active_view, 1);
    assert!(app.camera.perspective);
    assert!(!app.secondary_camera.as_ref().unwrap().perspective);
    app.camera.zoom(2.);
    let right_distance = app.camera.distance;
    assert_ne!(right_distance, 4.);
    assert_eq!(app.secondary_camera.as_ref().unwrap().distance, 4.);
    let owner = app.instances[app.selected];
    let original = app.scene.local(owner).unwrap();
    app.begin_drag(local, size).unwrap();
    app.drag.as_mut().unwrap().viewport_origin = origin;
    app.preview_drag(origin + local).unwrap();
    assert_eq!(app.scene.local(owner).unwrap(), original);
    app.preview_drag(origin + local + Vec2::new(10., 0.))
        .unwrap();
    assert_ne!(app.scene.local(owner).unwrap(), original);
    app.view_pointer(Vec2::new(250., 350.), Vec2::new(1001., 700.))
        .unwrap();
    assert_eq!(app.active_view, 0);
    assert!(app.drag.is_none());
    assert_eq!(app.scene.local(owner).unwrap(), original);
    assert!(!app.camera.perspective);
    assert_eq!(app.camera.distance, 4.);
    app.activate_view(1).unwrap();
    assert_eq!(app.camera.distance, right_distance);
    app.edit_key(KeyCode::F4).unwrap();
    assert!(!app.split_views());
    assert_eq!(app.active_view, 0);
    assert_eq!(app.camera.distance, right_distance);
    app.edit_key(KeyCode::F10).unwrap();
    assert!(!app.msaa4);
    assert_eq!(app.authoring_document().unwrap(), before);
    app.stop_workers().unwrap();
}

fn verify_native_play_audio(app: &mut App, expected: &SceneDocument) {
    if std::env::var_os("VOXY_TEST_NATIVE_AUDIO").is_some() {
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AudioBus { bus: 0, gain: 0.5 },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let with_bus = app.authoring_document().unwrap();
        app.audio.set_output_mode(AudioOutputMode::Native);
        app.toggle_play().unwrap();
        assert!(app.audio.is_pending() && app.play.playing.is_none());
        assert_eq!(app.authoring_document().unwrap(), with_bus);
        let deadline = Instant::now() + Duration::from_mins(3);
        while app.play.playing.is_none() {
            app.poll_audio_device().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        app.advance_game(1.0 / 60.0).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert!((app.audio.play().unwrap().peak - 0.125).abs() < 1e-6);
        let stats = app.audio.play().unwrap().native_stats().unwrap();
        assert!(stats.supplied > 0);
        assert_eq!(stats.errors, 0);
        println!(
            "EDITOR PLAY AUDIO DEVICE PASS supplied={} errors={}",
            stats.supplied, stats.errors
        );
        app.toggle_play().unwrap();
        assert!(app.audio.play().is_none());
        assert_eq!(app.authoring_document().unwrap(), with_bus);
        app.panel_action(panels::Action::AudioBus).unwrap();
        assert_eq!(&app.authoring_document().unwrap(), expected);
    }
}
fn wait_audio_publication(app: &mut App, failed: bool, after: u64) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(5);
    let id = AssetId("tone.wav".into());
    loop {
        app.tick().unwrap();
        let catalog = &app.audio.play().unwrap().catalog;
        let revision = catalog.snapshot_with_revision(&id).unwrap().0;
        let ready = if failed {
            matches!(
                catalog.status(&id),
                Some(voxy_assets::AssetStatus::Failed(_))
            )
        } else {
            matches!(catalog.status(&id), Some(voxy_assets::AssetStatus::Ready)) && revision > after
        };
        if ready {
            return revision;
        }
        assert!(Instant::now() < deadline, "audio watch did not settle");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn verify_play_audio_watch(app: &mut App, root: &std::path::Path) {
    let baseline = wait_audio_publication(app, false, 0);
    app.scene
        .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
        .unwrap()
        .unwrap()
        .looping = false;
    app.advance_game(1.0 / 60.0).unwrap();
    app.audio.play_mut().unwrap().peak = 0.;
    let deadline = Instant::now() + Duration::from_millis(400);
    while Instant::now() < deadline {
        app.tick().unwrap();
        app.advance_game(1.0 / 60.0).unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    let audio = app.audio.play().unwrap();
    assert!(
        audio.peak.abs() < 1e-7,
        "unchanged watch restarted a consumed one-shot"
    );
    assert_eq!(
        audio
            .catalog
            .snapshot_with_revision(&AssetId("tone.wav".into()))
            .unwrap()
            .0,
        baseline
    );
    app.scene
        .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
        .unwrap()
        .unwrap()
        .looping = true;
    let wav_path = root.join("tone.wav");
    let original = std::fs::read(&wav_path).unwrap();
    std::fs::write(&wav_path, b"broken WAV").unwrap();
    assert_eq!(wait_audio_publication(app, true, baseline), baseline);
    app.audio.play_mut().unwrap().peak = 0.;
    app.advance_game(1.0 / 60.0).unwrap();
    assert!((app.audio.play().unwrap().peak - 0.25).abs() < 1e-6);
    let mut repaired = original.clone();
    let end = repaired.len();
    repaired[end - 2..].copy_from_slice(&16384_i16.to_le_bytes());
    std::fs::write(&wav_path, &repaired).unwrap();
    let replacement = wait_audio_publication(app, false, baseline);
    app.audio.play_mut().unwrap().peak = 0.;
    app.advance_game(1.0 / 60.0).unwrap();
    assert!((app.audio.play().unwrap().peak - 0.5).abs() < 1e-6);
    let settings = root.join("tone.import.json");
    let original_settings = std::fs::read(&settings).unwrap();
    std::fs::write(&settings, b"invalid settings").unwrap();
    assert_eq!(wait_audio_publication(app, true, replacement), replacement);
    voxy_gameplay::AudioImportConfig::from_json(&original_settings)
        .unwrap()
        .save_file(&settings)
        .unwrap();
    let recovered = wait_audio_publication(app, false, replacement);
    std::fs::remove_file(&wav_path).unwrap();
    assert_eq!(wait_audio_publication(app, true, recovered), recovered);
    std::fs::write(&wav_path, original).unwrap();
    wait_audio_publication(app, false, recovered);
    println!(
        "EDITOR AUDIO WATCH PASS: invalid WAV/settings and deletion retained publication/PCM; repairs replaced revision"
    );
}
fn verify_audio_manifest_watch(root: &std::path::Path) {
    let manifest = root.join("assets.json");
    let write_manifest = |source: &str| {
        std::fs::write(
            &manifest,
            serde_json::to_vec(&serde_json::json!({
                "version": 1, "assets": [
                    {"asset": "mesh", "source": "quad.obj"},
                    {"asset": "tone.wav", "source": source},
                    {"asset": "tone.import.json", "source": "tone.import.json"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
    };
    write_manifest("tone.wav");
    let mut app = App::from_manifest(&manifest, AssetId("mesh".into()), false).unwrap();
    app.scene
        .insert_component(
            app.instances[0],
            voxy_gameplay::AudioSource {
                asset: "tone.wav".into(),
                import_settings: Some("tone.import.json".into()),
                bus: 0,
                gain: 1.,
                looping: true,
                spatial: false,
                near: 1.,
                far: 5.,
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    let expected = app.authoring_document().unwrap();
    app.toggle_play().unwrap();
    app.advance_game(1.0 / 60.0).unwrap();
    let baseline = wait_audio_publication(&mut app, false, 0);
    write_manifest("missing.wav");
    assert_eq!(wait_audio_publication(&mut app, true, baseline), baseline);
    app.audio.play_mut().unwrap().peak = 0.;
    app.advance_game(1.0 / 60.0).unwrap();
    assert!((app.audio.play().unwrap().peak - 0.25).abs() < 1e-6);
    let mut wav = std::fs::read(root.join("tone.wav")).unwrap();
    let end = wav.len();
    wav[end - 2..].copy_from_slice(&16384_i16.to_le_bytes());
    std::fs::write(root.join("missing.wav"), wav).unwrap();
    wait_audio_publication(&mut app, false, baseline);
    app.audio.play_mut().unwrap().peak = 0.;
    app.advance_game(1.0 / 60.0).unwrap();
    assert!((app.audio.play().unwrap().peak - 0.5).abs() < 1e-6);
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.stop_workers().unwrap();
    println!(
        "EDITOR AUDIO MANIFEST WATCH PASS: remap failure retained PCM; creation of newly observed missing path recovered"
    );
}
fn verify_background_audio_preparation(app: &mut App, expected: &SceneDocument) {
    let mut preparation =
        audio_play::AudioPreparation::new(&app.scene, &app.authoring.authoring_project, 48000)
            .unwrap();
    assert!(preparation.matches(&app.scene).unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !preparation.poll().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(&app.authoring_document().unwrap(), expected);
    app.scene
        .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
        .unwrap()
        .unwrap()
        .import_settings = None;
    assert!(!preparation.matches(&app.scene).unwrap());
    app.scene
        .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
        .unwrap()
        .unwrap()
        .import_settings = Some("tone.import.json".into());
    preparation.close().join().unwrap();
    let mut preparation =
        audio_play::AudioPreparation::new(&app.scene, &app.authoring.authoring_project, 48000)
            .unwrap();
    assert!(!preparation.poll().unwrap());
    app.audio.inject_preparation(preparation);
    app.toggle_play().unwrap();
    assert!(!app.audio.has_preparation() && app.audio.has_retired_workers());
    assert!(app.play.playing.is_none() && app.play.simulation.is_none());
    assert_eq!(&app.authoring_document().unwrap(), expected);
}
#[test]
fn play_imports_scene_audio_advances_pcm_and_stop_releases_session() {
    let root = std::env::temp_dir().join(format!(
        "voxy-play-audio-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let model = root.join("quad.obj");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj"),
        &model,
    )
    .unwrap();
    let mut wav = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0".to_vec();
    for value in [1_u16, 1] {
        wav.extend_from_slice(&value.to_le_bytes());
    }
    for value in [48000_u32, 96000] {
        wav.extend_from_slice(&value.to_le_bytes());
    }
    for value in [2_u16, 16] {
        wav.extend_from_slice(&value.to_le_bytes());
    }
    wav.extend_from_slice(b"data\x02\0\0\0");
    wav.extend_from_slice(&8192_i16.to_le_bytes());
    std::fs::write(root.join("tone.wav"), wav).unwrap();
    let mut app = App::new(&model, false).unwrap();
    let settings_path = root.join("tone.import.json");
    let valid_settings =
        r#"{"version":1,"max_input_bytes":1024,"max_frames":10,"max_filter_evaluations":650}"#;
    std::fs::write(&settings_path, valid_settings).unwrap();
    let descriptor = voxy_gameplay::AudioSource {
        import_settings: Some("tone.import.json".into()),
        asset: "missing.wav".into(),
        bus: 0,
        gain: 1.,
        looping: true,
        spatial: false,
        near: 1.,
        far: 5.,
    };
    app.scene
        .insert_component(app.instances[0], descriptor.clone())
        .unwrap();
    app.commit_authoring().unwrap();
    let before = app.authoring_document().unwrap();
    assert!(app.toggle_play().is_err());
    assert!(app.play.playing.is_none());
    assert_eq!(app.authoring_document().unwrap(), before);
    app.scene
        .insert_component(
            app.instances[0],
            voxy_gameplay::AudioSource {
                asset: "tone.wav".into(),
                ..descriptor
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    let expected = app.authoring_document().unwrap();
    audio_settings::tests::verify_settings_editor(&mut app, &settings_path);
    verify_background_audio_preparation(&mut app, &expected);
    app.toggle_play().unwrap();
    app.advance_game(1.0 / 60.0).unwrap();
    let audio = app.audio.play().unwrap();
    assert_eq!(audio.frames, 800);
    assert!((audio.peak - 0.25).abs() < 1e-6);
    verify_play_audio_watch(&mut app, &root);
    app.toggle_play().unwrap();
    assert!(app.audio.play().is_none());
    assert_eq!(app.authoring_document().unwrap(), expected);
    std::fs::write(
        &settings_path,
        r#"{"version":1,"max_input_bytes":1,"max_frames":10,"max_filter_evaluations":650}"#,
    )
    .unwrap();
    assert!(app.toggle_play().is_err());
    assert!(app.play.playing.is_none());
    assert_eq!(app.authoring_document().unwrap(), expected);
    std::fs::write(&settings_path, valid_settings).unwrap();
    verify_native_play_audio(&mut app, &expected);
    app.stop_workers().unwrap();
    drop(app);
    verify_audio_manifest_watch(&root);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_play_cancel_keeps_authoring_history_and_simulation_stopped() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let before = app.authoring_document().unwrap();
    app.audio.request_play();
    app.advance_game(1.0).unwrap();
    assert_eq!(app.play.simulation_ticks, 0);
    assert!(app.play.playing.is_none() && app.play.simulation.is_none());
    app.toggle_play().unwrap();
    assert!(!app.audio.is_pending());
    assert_eq!(app.authoring_document().unwrap(), before);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &before);
    app.stop_workers().unwrap();
}

#[test]
fn audio_settings_reference_inspector_supports_default_and_history() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.scene
        .insert_component(
            app.instances[0],
            voxy_gameplay::AudioSource {
                asset: "tone.wav".into(),
                import_settings: None,
                bus: 0,
                gain: 1.,
                looping: false,
                spatial: true,
                near: 1.,
                far: 5.,
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    app.inspector = InspectorMode::Audio;
    app.panel_action(panels::Action::Field(7)).unwrap();
    app.field_key(KeyCode::KeyA, Some("audio/tone.import.json"))
        .unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    let configured = app.authoring_document().unwrap();
    assert_eq!(
        configured.objects[0].components["game.audio-source.v1"]["import_settings"],
        "audio/tone.import.json"
    );
    app.panel_action(panels::Action::Field(7)).unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    let defaults = app.authoring_document().unwrap();
    assert!(
        defaults.objects[0].components["game.audio-source.v1"]
            .get("import_settings")
            .is_none()
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), configured);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), defaults);
    app.stop_workers().unwrap();
}

#[test]
fn audio_component_crud_discovers_assets_and_rejects_competing_listener() {
    let root = std::env::temp_dir().join(format!(
        "voxy-audio-crud-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let model = root.join("quad.obj");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj"),
        &model,
    )
    .unwrap();
    let mut app = App::new(&model, false).unwrap();
    let original = app.authoring_document().unwrap();
    assert!(app.panel_action(panels::Action::AudioSource).is_err());
    assert_eq!(app.authoring_document().unwrap(), original);
    std::fs::write(root.join("tone.wav"), []).unwrap();
    app.panel_action(panels::Action::AudioSource).unwrap();
    let source = app.authoring_document().unwrap();
    assert_eq!(
        source.objects[0].components["game.audio-source.v1"]["asset"],
        "tone.wav"
    );
    app.panel_action(panels::Action::AudioSource).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), source);
    app.edit_key(KeyCode::KeyD).unwrap();
    app.selected = 0;
    app.panel_action(panels::Action::AudioListener).unwrap();
    let one_listener = app.authoring_document().unwrap();
    app.selected = 1;
    assert!(app.panel_action(panels::Action::AudioListener).is_err());
    assert_eq!(app.authoring_document().unwrap(), one_listener);
    app.selected = 0;
    app.panel_action(panels::Action::AudioListener).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), one_listener);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(
        app.scene
            .components::<voxy_gameplay::AudioListener>()
            .count(),
        0
    );
    app.stop_workers().unwrap();
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mixer_inspector_round_trip_history_and_duplicate_bus_rollback() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.inspector = InspectorMode::Mixer;
    app.panel_action(panels::Action::AudioBus).unwrap();
    app.panel_action(panels::Action::Field(1)).unwrap();
    app.field_key(KeyCode::Digit2, Some("0.25")).unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    let expected = app.authoring_document().unwrap();
    assert_eq!(
        expected.objects[0].components["game.audio-bus.v1"]["gain"],
        0.25
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(
        app.authoring_document().unwrap().objects[0].components["game.audio-bus.v1"]["gain"],
        1.
    );
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    let serialized = serde_json::to_string(&expected).unwrap();
    let saved: SceneDocument = serde_json::from_str(&serialized).unwrap();
    let loaded = saved.load(&model_registry().unwrap(), 128).unwrap();
    assert!(
        (voxy_gameplay::extract_scene_audio(&loaded.graph, 128)
            .unwrap()
            .buses[0]
            .gain
            - 0.25)
            .abs()
            < 1e-6
    );
    app.panel_action(panels::Action::Field(1)).unwrap();
    app.field_key(KeyCode::Digit2, Some("2")).unwrap();
    assert!(app.field_key(KeyCode::Enter, None).is_err());
    assert_eq!(app.authoring_document().unwrap(), expected);
    assert!(app.edit_key(KeyCode::KeyD).is_err());
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.panel_action(panels::Action::AudioBus).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.toggle_play().unwrap();
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.stop_workers().unwrap();
}
#[test]
fn audio_inspector_edits_asset_and_parameters_with_validated_history() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.scene
        .insert_component(
            app.instances[0],
            voxy_gameplay::AudioSource {
                import_settings: None,
                asset: "tone.wav".into(),
                bus: 0,
                gain: 1.,
                looping: true,
                spatial: false,
                near: 1.,
                far: 5.,
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    app.panel_action(panels::Action::Behavior).unwrap();
    app.panel_action(panels::Action::Behavior).unwrap();
    assert_eq!(app.inspector, InspectorMode::Audio);
    for (index, text) in [
        (0, "0.25"),
        (1, "3"),
        (2, "0"),
        (3, "1"),
        (4, "2"),
        (5, "8"),
        (6, "audio/new.wav"),
    ] {
        app.panel_action(panels::Action::Field(index)).unwrap();
        app.field_key(KeyCode::Digit1, Some(text)).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
    }
    let saved = app.authoring_document().unwrap();
    let data = &saved.objects[0].components["game.audio-source.v1"];
    assert_eq!(data["asset"], "audio/new.wav");
    assert_eq!(data["bus"], 3);
    assert_eq!(data["looping"], false);
    assert_eq!(data["spatial"], true);
    for (index, text) in [(0, "2"), (1, "1.5"), (2, "2"), (4, "9"), (6, "")] {
        app.panel_action(panels::Action::Field(index)).unwrap();
        app.field_key(KeyCode::Digit1, Some(text)).unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.authoring_document().unwrap(), saved);
    }
    app.field_key(KeyCode::Escape, None).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(
        app.authoring_document().unwrap().objects[0].components["game.audio-source.v1"]["asset"],
        "tone.wav"
    );
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), saved);
    app.stop_workers().unwrap();
}

#[test]
fn transform_inspector_accepts_character_rotation_and_rejects_scale_atomically() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.panel_action(panels::Action::Character).unwrap();
    let before = app.authoring_document().unwrap();
    for (index, text) in [(6, "2"), (7, "0.5"), (8, "-1")] {
        app.panel_action(panels::Action::Field(index)).unwrap();
        app.field_key(KeyCode::Digit2, Some(text)).unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.authoring_document().unwrap(), before);
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &before);
    }
    app.panel_action(panels::Action::Field(4)).unwrap();
    app.field_key(KeyCode::Digit9, Some("90")).unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    let rotated = app.authoring_document().unwrap();
    let rotation = glam::Quat::from_array(rotated.objects[0].rotation);
    assert!(rotation.abs_diff_eq(
        glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        1e-6
    ));
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &rotated);
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), before);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), rotated);
    app.panel_action(panels::Action::Field(0)).unwrap();
    app.field_key(KeyCode::Digit2, Some("2")).unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    let moved = app.authoring_document().unwrap();
    assert!((moved.objects[0].translation[0] - 2.).abs() < 1e-6);
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), rotated);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), moved);
    app.stop_workers().unwrap();
}

#[test]
fn duplication_preserves_registered_behavior_and_activity_with_new_identity() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.panel_action(panels::Action::Motion).unwrap();
    app.edit_key(KeyCode::KeyA).unwrap();
    let original = app.authoring_document().unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    let duplicated = app.authoring_document().unwrap();
    assert_eq!(duplicated.objects.len(), 2);
    assert_ne!(duplicated.objects[0].id, duplicated.objects[1].id);
    assert_eq!(
        duplicated.objects[0].components,
        duplicated.objects[1].components
    );
    assert!(!duplicated.objects[1].active);
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), original);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), duplicated);
    app.stop_workers().unwrap();
}

#[test]
fn behavior_inspector_edits_undoes_and_rejects_conflicting_writers() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.panel_action(panels::Action::Motion).unwrap();
    app.panel_action(panels::Action::Behavior).unwrap();
    app.panel_action(panels::Action::Field(3)).unwrap();
    app.field_key(KeyCode::Minus, Some("-2")).unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    let saved = app.authoring_document().unwrap();
    assert_eq!(
        saved.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
        -2.
    );
    app.panel_action(panels::Action::Field(1)).unwrap();
    app.field_key(KeyCode::Digit0, Some("0")).unwrap();
    assert!(app.field_key(KeyCode::Enter, None).is_err());
    assert_eq!(app.authoring_document().unwrap(), saved);
    app.field_key(KeyCode::Escape, None).unwrap();
    assert!(app.panel_action(panels::Action::Collider).is_err());
    assert_eq!(app.authoring_document().unwrap(), saved);
    app.panel_action(panels::Action::Motion).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), saved);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert!(
        !app.authoring_document().unwrap().objects[0]
            .components
            .contains_key("game.angular-motion.v1")
    );
    app.stop_workers().unwrap();
}

#[test]
fn multiple_authored_bodies_collide_in_play_and_restart_from_document() {
    let root =
        std::env::temp_dir().join(format!("voxy-multiple-bodies-play-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("quad.obj"),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    )
    .unwrap();
    let mut app = App::new(&root.join("quad.obj"), false).unwrap();
    let mut document = app.authoring_document().unwrap();
    document.objects.truncate(1);
    let body = &mut document.objects[0];
    body.translation = [0.; 3];
    body.scale = [1.; 3];
    body.rotation = [0., 0., 0., 1.];
    body.components.clear();
    body.components.insert(
        "game.box.v1".into(),
        serde_json::to_value(BoxCollider {
            half_extents: [0.01; 3],
        })
        .unwrap(),
    );
    body.components.insert(
        "game.liquid-body.v1".into(),
        serde_json::to_value(voxy_gameplay::LiquidBody {
            mass_kg: 1.,
            initial_velocity_m_s: [3., 0., 0.],
        })
        .unwrap(),
    );
    body.components.insert(
        "game.liquid-mass.v1".into(),
        serde_json::to_value(voxy_gameplay::LiquidMassDistribution {
            parts: vec![voxy_gameplay::LiquidMassPart {
                mass_kg: 1.,
                center_m: [0.02, 0., 0.],
                half_edges_m: [[0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
            }],
        })
        .unwrap(),
    );
    let mut second = body.clone();
    second.id = ObjectId("second-body".into());
    second.translation = [0.08, 0., 0.];
    second.components.insert(
        "game.liquid-body.v1".into(),
        serde_json::to_value(voxy_gameplay::LiquidBody {
            mass_kg: 1.,
            initial_velocity_m_s: [0.; 3],
        })
        .unwrap(),
    );
    // First owner's geometry lives solely on its child, exercising persisted compounds.
    let mut child = document.objects[0].clone();
    child.id = ObjectId("body-shape".into());
    child.parent = Some(document.objects[0].id.clone());
    child.translation = [0.; 3];
    child.components.remove("game.liquid-body.v1");
    child.components.remove("game.liquid-mass.v1");
    document.objects[0].components.remove("game.box.v1");
    document.objects.push(second);
    document.objects.push(child);
    app.authoring
        .history
        .as_mut()
        .unwrap()
        .commit(document.clone(), &app.authoring.authoring_project.registry)
        .unwrap();
    app.restore_authoring().unwrap();
    app.toggle_play().unwrap();
    app.advance_game(1. / 60.).unwrap();
    app.advance_game(1. / 60.).unwrap();
    let runtime = app.play.liquid.as_ref().unwrap();
    for (_, properties) in runtime.body_mass_properties() {
        let properties = properties.unwrap();
        assert_eq!(properties.mass, 1.);
        assert_eq!(properties.center, [0.02, 0., 0.]);
        assert!(properties.principal_moments.iter().all(|m| *m > 0.));
    }
    for ((node, frame), (owner, state)) in
        runtime.body_rigid_frames().zip(runtime.body_rigid_states())
    {
        assert_eq!(node, owner);
        let frame = frame.unwrap();
        let prepared = frame.prepare_pose(state, 1., 1e-5).unwrap();
        assert_eq!(
            prepared.pose.translation,
            app.scene.local(node).unwrap().translation
        );
        assert_eq!(prepared.pose.scale, app.scene.local(node).unwrap().scale);
    }
    let states: Vec<_> = runtime.body_states().collect();
    assert_eq!(states.len(), 2);
    for (node, state) in states {
        assert!((state.velocity[0] - 1.5).abs() < 1e-10);
        assert!(
            (state.position[0] - f64::from(app.scene.local(node).unwrap().translation.x) - 0.02)
                .abs()
                < 1e-5
        );
    }
    runtime.validate_bindings(&app.scene).unwrap();
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), document);
    app.toggle_play().unwrap();
    let velocities: Vec<_> = app
        .play
        .liquid
        .as_ref()
        .unwrap()
        .body_states()
        .map(|(_, state)| state.velocity[0])
        .collect();
    assert_eq!(velocities, vec![3., 0.]);
    app.toggle_play().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn authored_liquid_body_moves_in_play_and_stop_restores_both_descriptors() {
    let root = std::env::temp_dir().join(format!("voxy-liquid-body-play-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("quad.obj"),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    )
    .unwrap();
    std::fs::write(
        root.join("water.json"),
        r#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001}"#,
    )
    .unwrap();
    let mut app = App::new(&root.join("quad.obj"), false).unwrap();
    let mut document = app.authoring_document().unwrap();
    let body = &mut document.objects[0];
    body.translation = [0.; 3];
    body.scale = [1.; 3];
    body.rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4).to_array();
    body.components.insert(
        "game.box.v1".into(),
        serde_json::to_value(BoxCollider {
            half_extents: [0.01, 2., 2.],
        })
        .unwrap(),
    );
    body.components.insert(
        "game.liquid-body.v1".into(),
        serde_json::to_value(voxy_gameplay::LiquidBody {
            mass_kg: 3.,
            initial_velocity_m_s: [0.; 3],
        })
        .unwrap(),
    );
    let mut jet = body.clone();
    jet.id = ObjectId("jet-1".into());
    jet.name = "jet".into();
    jet.translation = [-0.1, 0., 0.];
    jet.rotation = [0., 0., 0., 1.];
    jet.components.clear();
    jet.components.insert(
        "game.liquid-source.v1".into(),
        serde_json::to_value(voxy_gameplay::LiquidSource {
            pulses: vec![voxy_gameplay::LiquidPulse {
                start_s: 0.,
                duration_s: 1. / 60.,
                volume_m3: 0.001,
                speed_m_s: 2.,
            }],
            density_kg_m3: 1000.,
            particle_volume_m3: 0.001,
            nozzle_radius_m: 0.,
            direction: [1., 0., 0.],
            material_asset: "water.json".into(),
        })
        .unwrap(),
    );
    document.objects.push(jet);
    app.authoring
        .history
        .as_mut()
        .unwrap()
        .commit(document.clone(), &app.authoring.authoring_project.registry)
        .unwrap();
    app.restore_authoring().unwrap();
    app.toggle_play().unwrap();
    app.advance_game(1. / 60.).unwrap();
    app.advance_game(1. / 60.).unwrap();
    let runtime = app.play.liquid.as_ref().unwrap();
    let (owner, state) = runtime.body_state().unwrap();
    assert!(state.velocity[0] > 0.);
    assert!(app.scene.local(owner).unwrap().translation.x > 0.);
    assert_eq!(
        app.scene.local(owner).unwrap().rotation.to_array(),
        document.objects[0].rotation
    );
    assert!((runtime.liquid().mass() - 1.).abs() < 1e-12);
    runtime.validate_bindings(&app.scene).unwrap();
    app.toggle_play().unwrap();
    assert!(app.play.liquid.is_none());
    assert_eq!(app.authoring_document().unwrap(), document);
    app.toggle_play().unwrap();
    assert_eq!(
        app.play
            .liquid
            .as_ref()
            .unwrap()
            .body_state()
            .unwrap()
            .1
            .velocity,
        [0.; 3]
    );
    app.toggle_play().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn authored_liquid_play_steps_and_stop_restores_document() {
    let root = std::env::temp_dir().join(format!("voxy-liquid-play-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("quad.obj"),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    )
    .unwrap();
    std::fs::write(
        root.join("water.json"),
        r#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[0.2,0.1,0.05,1.333]}"#,
    )
    .unwrap();
    let mut app = App::new(&root.join("quad.obj"), false).unwrap();
    let mut document = app.authoring_document().unwrap();
    document.objects[0].components.insert(
        "game.liquid-source.v1".into(),
        serde_json::to_value(voxy_gameplay::LiquidSource {
            pulses: vec![voxy_gameplay::LiquidPulse {
                start_s: 0.,
                duration_s: 1.,
                volume_m3: 0.001,
                speed_m_s: 2.,
            }],
            density_kg_m3: 1000.,
            particle_volume_m3: 0.001,
            nozzle_radius_m: 0.,
            direction: [1., 0., 0.],
            material_asset: "water.json".into(),
        })
        .unwrap(),
    );
    app.authoring
        .history
        .as_mut()
        .unwrap()
        .commit(document.clone(), &app.authoring.authoring_project.registry)
        .unwrap();
    app.restore_authoring().unwrap();
    app.toggle_play().unwrap();
    assert!(app.play.liquid.is_some());
    assert_eq!(app.play.liquid_optics, vec![Some([0.2, 0.1, 0.05, 1.333])]);
    app.advance_game(1. / 60.).unwrap();
    let runtime = app.play.liquid.as_ref().unwrap();
    assert!((runtime.liquid().mass() - 1. / 60.).abs() < 1e-12);
    assert_eq!(runtime.liquid().particles().len(), 1);
    app.toggle_play().unwrap();
    assert!(app.play.liquid.is_none());
    assert!(app.play.liquid_optics.is_empty());
    assert_eq!(app.authoring_document().unwrap(), document);
    std::fs::remove_file(root.join("water.json")).unwrap();
    assert!(app.toggle_play().is_err());
    assert!(app.play.playing.is_none());
    assert!(app.play.liquid.is_none());
    assert_eq!(app.authoring_document().unwrap(), document);
    std::fs::write(
        root.join("water.json"),
        r#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[0.2,0.1,0.05,1.333]}"#,
    )
    .unwrap();
    app.toggle_play().unwrap();
    assert_eq!(app.play.liquid.as_ref().unwrap().liquid().mass(), 0.);
    app.toggle_play().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn physics_component_inspector_crud_history_and_play_use_authored_descriptors() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.edit_key(KeyCode::KeyC).unwrap();
    app.panel_action(panels::Action::Physics).unwrap();
    app.panel_action(panels::Action::Field(0)).unwrap();
    app.field_key(KeyCode::Digit8, Some("0.08")).unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    let owner = app.instances[0];
    assert!(
        (app.scene
            .component::<CharacterBody>(owner)
            .unwrap()
            .unwrap()
            .half_extents[0]
            - 0.08)
            .abs()
            < 1e-6
    );
    let saved = app.authoring_document().unwrap();
    app.edit_key(KeyCode::KeyC).unwrap();
    assert!(
        app.scene
            .component::<CharacterBody>(app.instances[0])
            .unwrap()
            .is_none()
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), saved);
    app.panel_action(panels::Action::Field(0)).unwrap();
    app.field_key(KeyCode::Minus, Some("-1")).unwrap();
    assert!(app.field_key(KeyCode::Enter, None).is_err());
    assert_eq!(app.authoring_document().unwrap(), saved);
    app.field_key(KeyCode::Escape, None).unwrap();
    app.toggle_play().unwrap();
    assert!(app.play.physics.is_some());
    let playing_owner = app.instances[0];
    assert_ne!(playing_owner, owner);
    app.game_key(KeyCode::ArrowRight, ElementState::Pressed)
        .unwrap();
    for _ in 0..3 {
        let physics = app.play.physics.as_mut().unwrap();
        app.play
            .simulation
            .as_mut()
            .unwrap()
            .advance_with(&mut app.scene, 1.0 / 60.0, |scene, dt| {
                physics.fixed_step(scene, &mut app.play.player_input, dt)
            })
            .unwrap();
    }
    assert!(app.scene.local(playing_owner).unwrap().translation.x > 0.0);
    app.toggle_play().unwrap();
    assert!(app.play.physics.is_none());
    assert_eq!(app.authoring_document().unwrap(), saved);
    assert!(!app.play.player_input.state("move_x").unwrap().held);
    app.edit_key(KeyCode::KeyD).unwrap();
    assert_eq!(app.scene.components::<CharacterBody>().count(), 2);
    app.edit_key(KeyCode::KeyB).unwrap();
    assert!(
        app.scene
            .component::<CharacterBody>(app.instances[1])
            .unwrap()
            .is_none()
    );
    assert!(
        app.scene
            .component::<BoxCollider>(app.instances[1])
            .unwrap()
            .is_some()
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.scene.components::<CharacterBody>().count(), 2);
}
#[test]
fn playable_fixture_persists_physics_and_rejects_invalid_parenting_or_scale() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/game");
    let mut app =
        App::from_manifest(&root.join("assets.json"), AssetId("player".into()), false).unwrap();
    app.configure_scene(&root.join("game.scene.json")).unwrap();
    let expected = app.authoring_document().unwrap();
    assert_eq!(expected.objects.len(), 3);
    assert_eq!(app.scene.components::<BoxCollider>().count(), 2);
    app.restore_authoring().unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.panel_action(panels::Action::Field(6)).unwrap();
    app.field_key(KeyCode::Digit2, Some("2")).unwrap();
    assert!(app.field_key(KeyCode::Enter, None).is_err());
    app.field_key(KeyCode::Escape, None).unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.toggle_play().unwrap();
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
}
#[test]
fn authored_motion_runs_in_fixed_loop_and_stop_restores_descriptor() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.scene
        .insert_component(
            app.instances[0],
            voxy_gameplay::AngularMotion {
                axis: [0., 0., 1.],
                radians_per_second: 2.,
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    let expected = app.authoring_document().unwrap();
    app.toggle_play().unwrap();
    for _ in 0..60 {
        app.advance_game(1.0 / 60.0).unwrap();
    }
    assert!(
        app.scene
            .local(app.instances[0])
            .unwrap()
            .rotation
            .angle_between(glam::Quat::from_rotation_z(2.))
            < 1e-3
    );
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.scene
        .insert_component(
            app.instances[0],
            voxy_gameplay::AngularMotion {
                axis: [0.; 3],
                radians_per_second: 2.,
            },
        )
        .unwrap();
    assert!(app.toggle_play().is_err());
    assert!(app.play.playing.is_none());
}
#[test]
fn authored_character_turn_uses_physics_and_stop_restores_pose() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let owner = app.instances[0];
    app.scene
        .insert_component(
            owner,
            CharacterBody {
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::AngularMotion {
                axis: [0., 1., 0.],
                radians_per_second: 2.,
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    let expected = app.authoring_document().unwrap();
    app.toggle_play().unwrap();
    for _ in 0..60 {
        app.advance_game(1. / 60.).unwrap();
    }
    assert!(
        app.scene
            .local(app.instances[0])
            .unwrap()
            .rotation
            .abs_diff_eq(glam::Quat::from_rotation_y(2.), 1e-5)
    );
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
}
#[test]
fn shutdown_joins_workers_after_behavior_destroy_panics() {
    #[derive(Debug)]
    struct PanickingDestroy;
    impl Behavior for PanickingDestroy {
        fn on_destroy(&mut self, _: &mut SceneGraph, _: NodeId) {
            panic!("test destroy failure");
        }
    }
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let mut simulation = SceneSimulation::new(
        &app.scene,
        SimulationLimits {
            fixed_step: 1.0 / 60.0,
            max_steps: 8,
            max_behaviors: 1,
            max_commands: 1,
        },
    )
    .unwrap();
    simulation
        .attach(&mut app.scene, app.instances[0], PanickingDestroy)
        .unwrap();
    app.play.simulation = Some(simulation);
    assert!(
        app.stop_workers()
            .unwrap_err()
            .to_string()
            .contains("panicked")
    );
    assert!(app.imports.is_none());
    assert!(app.watcher.is_none());
    app.stop_workers().unwrap();
}
#[test]
fn shutdown_joins_workers_even_if_simulation_rejects_scene() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let foreign = SceneGraph::new(1);
    app.play.simulation = Some(
        SceneSimulation::new(
            &foreign,
            SimulationLimits {
                fixed_step: 1.0 / 60.0,
                max_steps: 8,
                max_behaviors: 1,
                max_commands: 1,
            },
        )
        .unwrap(),
    );
    assert!(
        app.stop_workers()
            .unwrap_err()
            .to_string()
            .contains("simulation stop")
    );
    assert!(app.play.simulation.is_none());
    assert!(app.imports.is_none());
    assert!(app.watcher.is_none());
    app.stop_workers().unwrap();
}
#[test]
fn standalone_playable_fixture_quick_jump_lands_without_losing_actor() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/game");
    let mut app = configured_app(
        &ModelSource::Manifest {
            path: root.join("assets.json"),
            asset: AssetId("player".into()),
        },
        Some(&root.join("game.scene.json")),
        false,
    )
    .unwrap();
    app.start_standalone().unwrap();
    let actor = app.scene.components::<CharacterBody>().next().unwrap().0;
    for _ in 0..60 {
        app.advance_game(1. / 60.).unwrap();
    }
    let before = app.scene.local(actor).unwrap().translation;
    assert!(
        app.play
            .physics
            .as_ref()
            .unwrap()
            .state(&app.scene, actor)
            .unwrap()
            .unwrap()
            .grounded
    );
    app.game_key(KeyCode::Space, ElementState::Pressed).unwrap();
    app.game_key(KeyCode::Space, ElementState::Released)
        .unwrap();
    let mut peak = before.y;
    for _ in 0..180 {
        app.advance_game(1. / 60.).unwrap();
        // Exercise the same interpolated extraction used by native presentation.
        app.draw().unwrap();
        let rendered = app
            .extraction
            .instances()
            .iter()
            .find(|instance| instance.owner == actor)
            .expect("jumping actor must remain in the render extraction");
        let projection = app.camera.matrix(Vec2::new(640., 480.)).unwrap();
        for corner in [Vec3::new(-0.04, -0.04, 0.), Vec3::new(0.04, 0.04, 0.)] {
            let clip = projection * rendered.world * corner.extend(1.);
            assert!(clip.is_finite() && clip.w > 0.);
            assert!(clip.x.abs() < clip.w && clip.y.abs() < clip.w);
            assert!(clip.z >= 0. && clip.z <= clip.w);
        }
        let position = app.scene.local(actor).unwrap().translation;
        assert!(position.is_finite());
        peak = peak.max(position.y);
        assert!(app.scene.active_in_hierarchy(actor).unwrap());
        assert!(
            app.scene
                .component::<ModelInstance>(actor)
                .unwrap()
                .is_some()
        );
    }
    let after = app.scene.local(actor).unwrap().translation;
    assert!(peak > before.y + 0.1, "quick tap must jump");
    assert!(
        (after.y - before.y).abs() < 1e-5,
        "must land at starting height"
    );
    assert!(
        app.play
            .physics
            .as_ref()
            .unwrap()
            .state(&app.scene, actor)
            .unwrap()
            .unwrap()
            .grounded
    );
    eprintln!(
        "STANDALONE_JUMP before={before:?} peak_y={peak} after={after:?} ticks={}",
        app.play.simulation_ticks
    );
    app.stop_workers().unwrap();
}
#[test]
fn standalone_uses_fixed_loop_without_preview_spin_or_authoring_controls() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let expected = app.authoring_document().unwrap();
    app.start_standalone().unwrap();
    for _ in 0..120 {
        app.advance_game(1.0 / 60.0).unwrap();
    }
    assert_eq!(app.play.simulation_ticks, 120);
    assert_eq!(app.authoring_document().unwrap(), expected);
    for key in [
        KeyCode::F6,
        KeyCode::Delete,
        KeyCode::KeyD,
        KeyCode::F5,
        KeyCode::F9,
    ] {
        app.edit_key(key).unwrap();
    }
    assert!(app.play.playing.is_some());
    assert_eq!(app.authoring_document().unwrap(), expected);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
    app.stop_workers().unwrap();
}
#[test]
fn registered_ui_callback_uses_scene_barrier_without_creating_input_action() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.toggle_play().unwrap();
    let owner = app.scene.spawn(None, Transform::default()).unwrap();
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::UiElement {
                origin: [0.0; 2],
                size: [1.0; 2],
                color: [1.0; 4],
                layer: 0,
                enabled: true,
                action: Some("voxy.ui.hide".into()),
                text: None,
            },
        )
        .unwrap();
    app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
        .unwrap();
    app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), false)
        .unwrap();
    assert!(app.scene.active_in_hierarchy(owner).unwrap());
    assert!(app.play.player_input.state("voxy.ui.hide").is_none());
    app.advance_game(1.0 / 60.0).unwrap();
    assert!(!app.scene.active_in_hierarchy(owner).unwrap());
    app.toggle_play().unwrap();
    app.stop_workers().unwrap();
}
#[test]
fn ui_activation_reaches_fixed_input_once_and_stop_resets_custom_actions() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.toggle_play().unwrap();
    let owner = app.scene.spawn(None, Transform::default()).unwrap();
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::UiElement {
                origin: [0.0; 2],
                size: [1.0; 2],
                color: [1.0; 4],
                layer: 0,
                enabled: true,
                action: Some("jump".into()),
                text: None,
            },
        )
        .unwrap();
    app.game_ui_key(KeyCode::Tab, ElementState::Pressed, false)
        .unwrap();
    for key in [KeyCode::Enter, KeyCode::Space] {
        app.game_ui_key(key, ElementState::Pressed, false).unwrap();
    }
    app.game_ui_key(KeyCode::Enter, ElementState::Released, false)
        .unwrap();
    assert!(!app.play.player_input.state("jump").unwrap().pressed);
    app.game_ui_key(KeyCode::Space, ElementState::Released, false)
        .unwrap();
    assert!(app.play.player_input.state("jump").unwrap().pressed);
    app.advance_game(0.0).unwrap();
    assert!(app.play.player_input.state("jump").unwrap().pressed);
    app.advance_game(1.0 / 60.0).unwrap();
    assert!(!app.play.player_input.state("jump").unwrap().pressed);
    app.scene
        .component_mut::<voxy_gameplay::UiElement>(owner)
        .unwrap()
        .unwrap()
        .action = Some("menu".into());
    app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
        .unwrap();
    app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), false)
        .unwrap();
    assert!(app.play.player_input.state("menu").unwrap().pressed);
    assert!(!app.play.player_input.state("menu").unwrap().held);
    app.advance_game(1.0 / 60.0).unwrap();
    assert!(!app.play.player_input.state("menu").unwrap().pressed);
    app.toggle_play().unwrap();
    assert!(app.play.player_input.state("menu").is_none());
    app.stop_workers().unwrap();
}
#[test]
fn custom_ui_setup_recreates_callback_state_per_play_and_rejects_failed_start() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let authoring = app.authoring_document().unwrap();
    let setups = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = Arc::clone(&setups);
    app.ui_action_setup = Some(voxy_gameplay::UiActionSetup::new(move |handlers| {
        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut count = 0_u32;
        handlers.register("app.counter".into(), move |_, event, commands| {
            count += 1;
            let mut local = Transform::default();
            local.translation.x = count as f32;
            commands
                .push(voxy_scene::SceneCommand::SetLocal(event.owner, local))
                .map_err(|error| error.to_string())
        })
    }));
    for _ in 0..2 {
        app.toggle_play().unwrap();
        let owner = app.scene.spawn(None, Transform::default()).unwrap();
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::UiElement {
                    origin: [0.0; 2],
                    size: [1.0; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("app.counter".into()),
                    text: None,
                },
            )
            .unwrap();
        app.activate_game_ui(voxy_gameplay::UiActionEvent {
            owner,
            action: "app.counter".into(),
        })
        .unwrap();
        app.advance_game(0.0).unwrap();
        assert_eq!(app.scene.local(owner).unwrap().translation.x, 1.0);
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), authoring);
    }
    assert_eq!(setups.load(std::sync::atomic::Ordering::SeqCst), 2);
    app.ui_action_setup = Some(voxy_gameplay::UiActionSetup::new(|handlers| {
        handlers.register("voxy.ui.hide".into(), |_, _, _| Ok(()))
    }));
    assert!(app.toggle_play().is_err());
    assert!(app.play.playing.is_none());
    assert!(app.play.simulation.is_none());
    assert!(app.play.ui_actions.is_none());
    assert_eq!(app.authoring_document().unwrap(), authoring);
    app.stop_workers().unwrap();
}

#[test]
fn play_pointer_path_activates_runtime_ui_and_stop_cancels_capture() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.toggle_play().unwrap();
    let owner = app.scene.spawn(None, Transform::default()).unwrap();
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::UiElement {
                origin: [0.0; 2],
                size: [1.0; 2],
                color: [1.0; 4],
                layer: 0,
                enabled: true,
                action: Some("start".into()),
                text: None,
            },
        )
        .unwrap();
    assert!(
        app.game_ui_key(KeyCode::Tab, ElementState::Pressed, false)
            .unwrap()
    );
    assert!(
        app.game_ui_key(KeyCode::Enter, ElementState::Pressed, false)
            .unwrap()
    );
    let (_, keyboard) = app
        .game_ui_input
        .key(
            &app.scene,
            [1280.0, 720.0],
            KeyCode::Enter,
            ElementState::Released,
            false,
            false,
        )
        .unwrap();
    assert_eq!(keyboard.unwrap().owner, owner);
    app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
        .unwrap();
    let event = app
        .game_ui_input
        .pointer(&app.scene, [100.0; 2], [50.0; 2], false)
        .unwrap()
        .unwrap();
    assert_eq!(event.owner, owner);
    assert_eq!(event.action, "start");
    app.scene
        .component_mut::<voxy_gameplay::UiElement>(owner)
        .unwrap()
        .unwrap()
        .action = Some("voxy.ui.hide".into());
    app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), true)
        .unwrap();
    app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), false)
        .unwrap();
    assert!(app.scene.active_in_hierarchy(owner).unwrap());
    app.advance_game(0.0).unwrap();
    assert!(!app.scene.active_in_hierarchy(owner).unwrap());
    app.scene.set_active(owner, true).unwrap();
    app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
        .unwrap();
    app.toggle_play().unwrap();
    assert!(
        app.game_ui_input
            .pointer(&app.scene, [100.0; 2], [50.0; 2], false)
            .unwrap()
            .is_none()
    );
    app.stop_workers().unwrap();
}
#[test]
fn native_play_simulation_ticks_and_stop_preserve_authoring() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let expected = app.authoring_document().unwrap();
    app.toggle_play().unwrap();
    let mut ticks = 0;
    for _ in 0..100 {
        ticks += app
            .play
            .simulation
            .as_mut()
            .unwrap()
            .advance(&mut app.scene, 0.01)
            .unwrap()
            .time
            .steps;
    }
    assert_eq!(ticks, 60);
    let rotation = app.scene.local(app.instances[0]).unwrap().rotation;
    assert!(rotation.angle_between(glam::Quat::IDENTITY) < 1e-6);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    assert!(app.play.simulation.is_none());
    app.stop_workers().unwrap();
}

#[test]
#[allow(clippy::float_cmp)]
fn hierarchy_inspector_and_play_preserve_authoring_history() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    app.edit_key(KeyCode::KeyP).unwrap();
    app.panel_action(panels::Action::Select(0)).unwrap();
    let parent = app.instances[0];
    let child = app.instances[1];
    assert_eq!(app.scene.parent(child).unwrap(), Some(parent));
    let document = app.authoring_document().unwrap();
    assert_eq!(
        document.objects[1].parent,
        Some(document.objects[0].id.clone())
    );
    app.restore_authoring().unwrap();
    assert_eq!(app.authoring_document().unwrap(), document);
    app.panel_action(panels::Action::Field(0)).unwrap();
    app.field_key(KeyCode::Digit2, Some("2")).unwrap();
    app.field_key(KeyCode::Enter, None).unwrap();
    assert_eq!(
        app.scene.local(app.instances[1]).unwrap().translation.x,
        2.0
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), document);
    app.panel_action(panels::Action::Field(8)).unwrap();
    app.field = Some((8, "NaN".into()));
    assert!(app.field_key(KeyCode::Enter, None).is_err());
    assert_eq!(app.authoring_document().unwrap(), document);
    app.field_key(KeyCode::Escape, None).unwrap();
    // World-X drag must convert through a rotated/scaled parent.
    let parent = app.instances[0];
    let child = app.instances[1];
    let mut local = app.scene.local(parent).unwrap();
    local.rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    local.scale = Vec3::splat(2.0);
    app.scene.set_local(parent, local).unwrap();
    let document = app.authoring_document().unwrap();
    app.authoring
        .history
        .as_mut()
        .unwrap()
        .commit(document, &model_registry().unwrap())
        .unwrap();
    let before_world = app.scene.world_matrix(child).unwrap().w_axis.truncate();
    app.begin_drag(Vec2::ZERO, Vec2::splat(100.0)).unwrap();
    app.drag.as_mut().unwrap().axis = DragAxis::X;
    app.preview_drag(Vec2::new(10.0, 0.0)).unwrap();
    let after_world = app.scene.world_matrix(child).unwrap().w_axis.truncate();
    assert!((after_world - before_world - Vec3::new(0.2, 0.0, 0.0)).length() < 1e-5);
    app.finish_drag(false).unwrap();
    let authored = app.authoring_document().unwrap();
    let history = app.authoring.history.as_ref().unwrap().current().clone();
    app.toggle_play().unwrap();
    let runtime = app.instances[0];
    let mut local = app.scene.local(runtime).unwrap();
    local.translation = Vec3::splat(9.0);
    app.scene.set_local(runtime, local).unwrap();
    app.edit_key(KeyCode::Delete).unwrap();
    assert_eq!(app.instances.len(), 2);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &history);
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), authored);
    app.selected = 0;
    app.edit_key(KeyCode::Delete).unwrap();
    assert!(app.instances.is_empty());
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), authored);
    app.stop_workers().unwrap();
}

#[test]
#[allow(clippy::float_cmp)]
fn multiple_resources_reload_independently_and_round_trip() {
    let directory = std::env::temp_dir().join(format!(
        "voxy-editor-multiple-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let triangle = "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv -0.5 0.5 0\nf 1 2 3\n";
    let small = "v 0 0 0\nv 0.2 0 0\nv 0 0.2 0\nf 1 2 3\n";
    std::fs::write(directory.join("a.obj"), triangle).unwrap();
    std::fs::write(directory.join("b.obj"), small).unwrap();
    std::fs::write(
        directory.join("assets.json"),
        r#"{"version":1,"assets":[{"asset":"a","source":"a.obj"},{"asset":"b","source":"b.obj"}]}"#,
    )
    .unwrap();
    let mut app =
        App::from_manifest(&directory.join("assets.json"), AssetId("a".into()), false).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    app.edit_key(KeyCode::KeyR).unwrap();
    let a = AssetId("a".into());
    let b = AssetId("b".into());
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.catalog.snapshot(&a).is_none() || app.catalog.snapshot(&b).is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    // Initial observations deliberately invalidate once to close the baseline race.
    let settle = Instant::now() + Duration::from_millis(650);
    while Instant::now() < settle {
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let original_a = app.catalog.snapshot(&a).unwrap();
    let original_b = app.catalog.snapshot(&b).unwrap();
    assert_ne!(original_a.value().mesh.vertices().len(), 0);
    assert_ne!(
        original_a.value().mesh.vertices()[0].position,
        original_b.value().mesh.vertices()[0].position
    );
    app.selected = 1;
    app.panel_action(panels::Action::Parent).unwrap();
    app.panel_action(panels::Action::Select(0)).unwrap();
    let expected = app.authoring_document().unwrap();
    assert_eq!(
        expected.objects[1].parent,
        Some(expected.objects[0].id.clone())
    );
    assert_eq!(expected.objects[0].components["editor.model.v1"], "a");
    assert_eq!(expected.objects[1].components["editor.model.v1"], "b");
    assert!(
        app.pick_model(Vec2::new(200.0, 300.0), Vec2::new(640.0, 480.0))
            .unwrap()
    );
    assert_eq!(app.selected, 0);
    app.authoring.scene_path = Some(directory.join("scene.json"));
    app.save_authoring().unwrap();
    let mut fresh = App::from_manifest(&directory.join("assets.json"), a.clone(), false).unwrap();
    fresh
        .configure_scene(&directory.join("scene.json"))
        .unwrap();
    assert_eq!(fresh.authoring_document().unwrap(), expected);
    app.edit_key(KeyCode::KeyZ).unwrap(); // detach undo
    app.edit_key(KeyCode::KeyZ).unwrap(); // resource undo
    assert_eq!(
        app.authoring_document().unwrap().objects[1].components["editor.model.v1"],
        "a"
    );
    app.edit_key(KeyCode::KeyY).unwrap();
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), expected);
    std::fs::write(directory.join("b.obj"), "broken").unwrap();
    while app.failed_at.is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(&original_a, &app.catalog.snapshot(&a).unwrap()));
    assert!(Arc::ptr_eq(&original_b, &app.catalog.snapshot(&b).unwrap()));
    std::fs::write(directory.join("b.obj"), triangle).unwrap();
    while Arc::ptr_eq(&original_b, &app.catalog.snapshot(&b).unwrap()) {
        app.tick().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(&original_a, &app.catalog.snapshot(&a).unwrap()));
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.stop_workers().unwrap();
    fresh.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn gizmo_pointer_gestures_constrain_axes_and_undo_without_changing_resource() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let asset = ObjAsset::parse(
        &std::fs::read_to_string(&fixture).unwrap(),
        ObjLimits {
            source_bytes: 4096,
            attributes: 64,
            vertices: 64,
            triangles: 64,
        },
    )
    .unwrap();
    let imported = voxy_assets::ImportInputs::new(0, 0)
        .finish(import::EditorAsset::from(asset), |_, _| {
            panic!("unexpected input")
        })
        .unwrap();
    let ticket = app.catalog.request(app.id.clone()).unwrap();
    app.catalog.complete(&ticket, Ok(imported)).unwrap();
    let resource = app.catalog.snapshot(&app.id).unwrap();
    let before = app.authoring_document().unwrap();
    for (origin, expected_axis) in [
        (Vec2::new(360.0, 240.0), DragAxis::X),
        (Vec2::new(320.0, 200.0), DragAxis::Y),
        (Vec2::new(320.0, 240.0), DragAxis::Z),
    ] {
        app.pointer_press(origin, Vec2::new(640.0, 480.0)).unwrap();
        assert_eq!(app.drag.as_ref().unwrap().axis, expected_axis);
        app.preview_drag(origin + Vec2::new(20.0, -20.0)).unwrap();
        app.finish_drag(true).unwrap();
        let moved = app.scene.local(app.instances[0]).unwrap().translation;
        match expected_axis {
            DragAxis::X => {
                assert!(moved.x > 0.0);
                assert_eq!(moved.y.to_bits(), 0.0_f32.to_bits());
                assert_eq!(moved.z.to_bits(), 0.5_f32.to_bits());
            }
            DragAxis::Y => {
                assert_eq!(moved.x.to_bits(), 0.0_f32.to_bits());
                assert!(moved.y > 0.0);
                assert_eq!(moved.z.to_bits(), 0.5_f32.to_bits());
            }
            DragAxis::Z => {
                assert_eq!(moved.x.to_bits(), 0.0_f32.to_bits());
                assert_eq!(moved.y.to_bits(), 0.0_f32.to_bits());
                assert!(moved.z > 0.5);
            }
            DragAxis::Plane => panic!("expected axis"),
        }
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        assert!(Arc::ptr_eq(
            &resource,
            &app.catalog.snapshot(&app.id).unwrap()
        ));
    }
    app.stop_workers().unwrap();
}

#[test]
fn authoring_file_round_trip_preserves_ids_transforms_and_resource_references() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let directory = std::env::temp_dir().join(format!(
        "voxy-editor-scene-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("scene.json");
    let mut first = App::new(&fixture, false).unwrap();
    first.authoring.scene_path = Some(path.clone());
    first.edit_key(KeyCode::KeyD).unwrap();
    first.edit_key(KeyCode::ArrowUp).unwrap();
    first
        .scene
        .set_name(first.instances[1], "Saved duplicate")
        .unwrap();
    first.scene.set_active(first.instances[1], false).unwrap();
    let mut local = first.scene.local(first.instances[1]).unwrap();
    local.scale = Vec3::new(1.5, 0.75, 1.0);
    local.rotation = glam::Quat::from_rotation_z(0.3);
    first.scene.set_local(first.instances[1], local).unwrap();
    first.save_authoring().unwrap();
    let saved = first.authoring_document().unwrap();
    let mut reopened = App::new(&fixture, false).unwrap();
    reopened.authoring.scene_path = Some(path.clone());
    reopened.load_authoring().unwrap();
    assert_eq!(reopened.authoring_document().unwrap(), saved);
    assert!(reopened.authoring.authoring_source.is_none());
    assert!(reopened.authoring.scene_revision.is_some());
    reopened.save_authoring().unwrap();
    reopened.save_authoring().unwrap();
    let observed_bytes = std::fs::read(&path).unwrap();
    // Even a formatting-only external write changes the observed revision.
    let mut external_bytes = observed_bytes.clone();
    external_bytes.push(b'\n');
    std::fs::write(&path, &external_bytes).unwrap();
    assert!(reopened.save_authoring().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), external_bytes);
    assert_eq!(reopened.authoring_document().unwrap(), saved);
    std::fs::write(&path, &observed_bytes).unwrap();
    reopened.save_authoring().unwrap();
    reopened.edit_key(KeyCode::KeyD).unwrap();
    assert_eq!(reopened.object_ids[2], ObjectId("model-2".into()));
    let before_invalid = reopened.authoring_document().unwrap();
    std::fs::write(&path, "invalid scene").unwrap();
    assert!(reopened.load_authoring().is_err());
    assert!(reopened.save_authoring().is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid scene");
    assert_eq!(reopened.authoring_document().unwrap(), before_invalid);
    let mut foreign = saved.clone();
    foreign.objects[0].components.insert(
        "editor.model.v1".into(),
        serde_json::Value::String("other-model".into()),
    );
    std::fs::write(&path, foreign.to_json().unwrap()).unwrap();
    assert!(reopened.load_authoring().is_err());
    assert_eq!(reopened.authoring_document().unwrap(), before_invalid);
    reopened.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(reopened.authoring_document().unwrap(), saved);
    first.stop_workers().unwrap();
    reopened.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn drag_is_one_undo_step_and_cancel_preserves_redo() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let before = app.authoring_document().unwrap();
    app.begin_drag(Vec2::splat(50.0), Vec2::splat(100.0))
        .unwrap();
    for x in 51_u16..=70 {
        app.preview_drag(Vec2::new(f32::from(x), 40.0)).unwrap();
    }
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &before);
    app.finish_drag(true).unwrap();
    let moved = app.authoring_document().unwrap();
    assert_ne!(before, moved);
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), before);
    app.begin_drag(Vec2::splat(50.0), Vec2::splat(100.0))
        .unwrap();
    app.preview_drag(Vec2::splat(80.0)).unwrap();
    app.edit_key(KeyCode::Escape).unwrap();
    assert_eq!(app.authoring_document().unwrap(), before);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), moved);
    app.begin_drag(Vec2::splat(50.0), Vec2::splat(100.0))
        .unwrap();
    app.preview_drag(Vec2::splat(60.0)).unwrap();
    app.finish_drag(false).unwrap();
    assert_eq!(app.authoring_document().unwrap(), moved);
    app.stop_workers().unwrap();
}

#[test]
fn deletion_restores_empty_scene_and_preserves_object_identity() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    let duplicate_id = app.object_ids[1].clone();
    app.edit_key(KeyCode::Delete).unwrap();
    assert_eq!(app.object_ids, vec![ObjectId("model-0".into())]);
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.object_ids[1], duplicate_id);
    app.edit_key(KeyCode::KeyY).unwrap();
    app.edit_key(KeyCode::Backspace).unwrap();
    assert!(app.instances.is_empty());
    app.edit_key(KeyCode::Tab).unwrap();
    app.edit_key(KeyCode::ArrowRight).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.object_ids, vec![ObjectId("model-0".into())]);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert!(app.instances.is_empty());
    app.edit_key(KeyCode::KeyD).unwrap();
    assert_eq!(app.instances.len(), 1);
    assert_ne!(app.object_ids[0], duplicate_id);
    assert_ne!(app.object_ids[0], ObjectId("model-0".into()));
    app.stop_workers().unwrap();
}

#[test]
fn mouse_selection_uses_transforms_depth_activity_and_restored_handles() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    let bounds = PickBounds {
        min: Vec3::new(-0.1, -0.1, 0.0),
        max: Vec3::new(0.1, 0.1, 0.0),
        layers: 1,
    };
    let size = Vec2::splat(100.0);
    assert!(
        app.pick_bounds(Vec2::new(57.5, 50.0), size, bounds)
            .unwrap()
    );
    assert_eq!(app.selected, 1);
    assert!(app.pick_bounds(Vec2::splat(50.0), size, bounds).unwrap());
    assert_eq!(app.selected, 0);
    assert!(!app.pick_bounds(Vec2::ZERO, size, bounds).unwrap());
    let duplicate = app.instances[1];
    app.scene
        .set_local(
            duplicate,
            Transform {
                translation: Vec3::new(0.0, 0.0, 0.25),
                ..Transform::default()
            },
        )
        .unwrap();
    assert!(app.pick_bounds(Vec2::splat(50.0), size, bounds).unwrap());
    assert_eq!(app.selected, 1);
    app.scene.set_active(duplicate, false).unwrap();
    app.pick_bounds(Vec2::splat(50.0), size, bounds).unwrap();
    assert_eq!(app.selected, 0);
    app.edit_key(KeyCode::KeyZ).unwrap();
    app.edit_key(KeyCode::KeyY).unwrap();
    assert!(
        app.pick_bounds(Vec2::new(57.5, 50.0), size, bounds)
            .unwrap()
    );
    assert_eq!(app.selected, 1);
    app.stop_workers().unwrap();
}

#[test]
fn reload_preserves_scene_instance_and_edited_transform() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let node = app.instances[0];
    let edited = Transform {
        translation: Vec3::new(0.2, -0.1, 0.5),
        scale: Vec3::splat(0.75),
        ..Transform::default()
    };
    app.scene.set_local(node, edited).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    let duplicate = app.instances[1];
    app.edit_key(KeyCode::ArrowRight).unwrap();
    let duplicate_matrix = app.scene.world_matrix(duplicate).unwrap();
    assert_ne!(duplicate_matrix, edited.matrix().unwrap());
    app.edit_key(KeyCode::Tab).unwrap();
    assert_eq!(app.instances[app.selected], node);
    let old_ticket = app.catalog.request(app.id.clone()).unwrap();
    app.catalog
        .invalidate(std::slice::from_ref(&app.id))
        .unwrap();
    assert!(matches!(
        app.catalog.complete(&old_ticket, Err("obsolete".into())),
        Err(AssetError::StaleTicket)
    ));
    let new_ticket = app.catalog.request(app.id.clone()).unwrap();
    app.catalog
        .complete(&new_ticket, Err("invalid OBJ".into()))
        .unwrap();
    assert_eq!(app.instances[0], node);
    assert_eq!(app.instances.len(), 2);
    assert_eq!(app.scene.world_matrix(duplicate).unwrap(), duplicate_matrix);
    assert_eq!(
        app.scene
            .component::<ModelInstance>(duplicate)
            .unwrap()
            .unwrap()
            .asset,
        app.id
    );
    assert_eq!(
        app.scene.world_matrix(node).unwrap(),
        edited.matrix().unwrap()
    );
    assert_eq!(
        app.scene
            .component::<ModelInstance>(node)
            .unwrap()
            .unwrap()
            .asset,
        app.id
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_ne!(
        app.scene.world_matrix(app.instances[1]).unwrap(),
        duplicate_matrix
    );
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(
        app.scene.world_matrix(app.instances[1]).unwrap(),
        duplicate_matrix
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.instances.len(), 1);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.instances.len(), 2);
    app.edit_key(KeyCode::ArrowUp).unwrap();
    let after_new_edit = app.authoring_document().unwrap();
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), after_new_edit);
    app.stop_workers().unwrap();
}
