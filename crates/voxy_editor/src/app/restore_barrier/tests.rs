use super::*;
fn app() -> App {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/animated-triangle.glb");
    let mut app = App::new(&path, false).unwrap();
    app.tick().unwrap();
    app.begin_model_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    assert!(app.resume_model_checkpoint().unwrap());
    app
}
fn ready(app: &mut App) {
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_restore_barrier(|_| None).unwrap() {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
}
fn release(app: &mut App) {
    let until = Instant::now() + Duration::from_secs(10);
    while !app.resume_restore_barrier().unwrap() {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
}
#[test]
fn coordinated_barrier_freezes_game_draw_and_retains_input_until_resume() {
    let mut app = app();
    app.toggle_play().unwrap();
    assert!(app.play.simulation.is_some());
    app.advance_game(0.05).unwrap();
    app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
    ready(&mut app);
    assert!(app.restore_play_frozen());
    let (ui, audio) = app.restore_barrier_snapshots().unwrap();
    assert!(!ui.to_json(8 * 1024 * 1024).unwrap().is_empty());
    assert!(!audio.to_json(8 * 1024 * 1024).unwrap().is_empty());
    let ticks = app.play.simulation_ticks;
    let animation = app.play.animations.change_token();
    let epoch = app.play.simulation_time;
    app.play
        .player_input
        .event(voxy_gameplay::JUMP, 1.)
        .unwrap();
    app.play
        .player_input
        .event(voxy_gameplay::JUMP, 0.)
        .unwrap();
    let input = app.play.player_input.state("jump").unwrap();
    assert!(input.pressed && input.released);
    app.advance_game(0.05).unwrap();
    app.draw().unwrap();
    assert_eq!(app.play.simulation_ticks, ticks);
    assert_eq!(app.play.animations.change_token(), animation);
    assert_eq!(app.play.simulation_time, epoch);
    assert_eq!(app.play.player_input.state("jump").unwrap(), input);
    assert!(app.toggle_play().is_err());
    let before = Instant::now();
    release(&mut app);
    assert!(app.play.simulation_time >= before);
    assert!(!app.restore_play_frozen());
    assert_eq!(app.play.player_input.state("jump").unwrap(), input);
    app.advance_game(0.02).unwrap();
    assert!(app.play.simulation_ticks > ticks);
    app.stop_workers().unwrap();
}
#[test]
fn acquisition_rejection_preserves_existing_fences_and_native_audio_mode() {
    let mut app = app();
    app.begin_model_checkpoint().unwrap();
    assert!(app.begin_restore_barrier(1024).is_err());
    assert!(app.restore_barrier.is_none());
    assert!(app.resource_checkpoint.is_some());
    app.resume_model_checkpoint().unwrap();
    app.ui_live.begin_checkpoint().unwrap();
    assert!(app.begin_restore_barrier(1024).is_err());
    assert!(app.resource_checkpoint.is_none());
    assert!(app.ui_live.resume_after_checkpoint().unwrap());
    app.audio
        .set_output_mode(audio_session::AudioOutputMode::Native);
    // A selected native policy without actual output is a logical boundary.
    app.begin_restore_barrier(1024).unwrap();
    ready(&mut app);
    assert!(app.restore_audio_frozen());
    release(&mut app);
    assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
    assert_eq!(
        app.audio.output_mode(),
        audio_session::AudioOutputMode::Native
    );
    app.audio
        .set_output_mode(audio_session::AudioOutputMode::Offline);
    app.audio.request_checkpoint().unwrap();
    assert!(app.begin_restore_barrier(1024).is_err());
    assert!(app.audio.request_checkpoint().is_err());
    app.audio.resume_checkpoint().unwrap();
    app.stop_workers().unwrap();
}
#[test]
fn cancel_consumes_issued_watcher_fence_without_pausing_unfrozen_time() {
    let mut app = app();
    let epoch = app.play.simulation_time;
    app.begin_restore_barrier(1024).unwrap();
    app.watcher.as_mut().unwrap().request_checkpoint().unwrap();
    app.resource_checkpoint = Some(crate::app::resource_checkpoint::Barrier::Requested);
    release(&mut app);
    assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
    assert_eq!(app.play.simulation_time, epoch);
    app.ui_live.begin_checkpoint().unwrap();
    app.ui_live.resume_after_checkpoint().unwrap();
    app.begin_model_checkpoint().unwrap();
    app.resume_model_checkpoint().unwrap();
    app.stop_workers().unwrap();
}

#[test]
fn offline_audio_owner_freezes_with_game_and_keeps_exact_snapshot_during_draw() {
    let path = std::env::temp_dir().join(format!("voxy-barrier-audio-{}", std::process::id()));
    std::fs::create_dir(&path).unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&42_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&48000_u32.to_le_bytes());
    bytes.extend_from_slice(&96000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&6_u32.to_le_bytes());
    for sample in [3000_i16, -5000, 7000] {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(path.join("tone.wav"), bytes).unwrap();
    let mut app = app();
    app.toggle_play().unwrap();
    let owner = app.instances[0];
    let identity = app.object_ids[0].clone();
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::AudioSource {
                asset: "tone.wav".into(),
                import_settings: None,
                bus: 0,
                gain: 0.8,
                looping: true,
                spatial: false,
                near: 1.,
                far: 5.,
            },
        )
        .unwrap();
    app.authoring.authoring_project =
        prefab_authoring::AuthoringProject::new(&path, &InputRecipe::Direct).unwrap();
    app.audio.set_play(
        audio_play::AudioPlay::prepare(&app.scene, &app.authoring.authoring_project, None).unwrap(),
    );
    app.advance_game(0.05).unwrap();
    assert!(app.audio.play().unwrap().frames > 0);
    app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !app
        .poll_restore_barrier(|node| (node == owner).then(|| identity.clone()))
        .unwrap()
    {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let frames = app.audio.play().unwrap().frames;
    let before = app
        .restore_barrier_snapshots()
        .unwrap()
        .1
        .to_json(8 * 1024 * 1024)
        .unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&before).unwrap()["play"].is_object());
    app.advance_game(0.05).unwrap();
    app.draw().unwrap();
    assert_eq!(frames, app.audio.play().unwrap().frames);
    let current = app
        .audio
        .poll_checkpoint(
            |node| (node == owner).then(|| identity.clone()),
            8 * 1024 * 1024,
        )
        .unwrap()
        .unwrap();
    assert_eq!(before, current.to_json(8 * 1024 * 1024).unwrap());
    let bindings: Vec<_> = app
        .object_ids
        .iter()
        .cloned()
        .zip(app.instances.iter().copied())
        .collect();
    let ui_limits = crate::ui_live::checkpoint::tests::limits();
    let (_, original_audio) = app
        .coordinated_outer_fingerprint(&bindings, ui_limits)
        .unwrap();
    // A direct native-owner mutation must be observed even though the barrier's
    // cached audio checkpoint still contains the earlier frame.
    app.audio
        .play_mut()
        .unwrap()
        .advance(&app.scene, 1)
        .unwrap();
    let (_, changed_audio) = app
        .coordinated_outer_fingerprint(&bindings, ui_limits)
        .unwrap();
    assert_ne!(original_audio, changed_audio);
    assert_eq!(
        before,
        app.restore_barrier_snapshots()
            .unwrap()
            .1
            .to_json(8 * 1024 * 1024)
            .unwrap()
    );
    release(&mut app);
    app.advance_game(0.02).unwrap();
    assert!(app.audio.play().unwrap().frames > frames);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn snapshot_budget_failure_can_cancel_and_resume_the_original_game() {
    let mut app = app();
    app.toggle_play().unwrap();
    app.advance_game(0.05).unwrap();
    let ticks = app.play.simulation_ticks;
    app.begin_restore_barrier(1).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        match app.poll_restore_barrier(|_| None) {
            Err(error) => {
                assert!(error.to_string().contains("byte budget"));
                break;
            }
            Ok(false) => assert!(Instant::now() < until),
            Ok(true) => panic!("one byte cannot hold the native audio snapshot"),
        }
        std::thread::yield_now();
    }
    assert!(app.restore_play_frozen());
    assert!(app.restore_barrier_snapshots().is_err());
    app.advance_game(0.1).unwrap();
    assert_eq!(app.play.simulation_ticks, ticks);
    release(&mut app);
    assert!(app.resource_checkpoint.is_none());
    app.advance_game(0.02).unwrap();
    assert!(app.play.simulation_ticks > ticks);
    app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
    ready(&mut app);
    release(&mut app);
    app.stop_workers().unwrap();
}

#[test]
fn model_release_failure_keeps_audio_and_game_frozen_until_recovery() {
    let mut app = app();
    app.toggle_play().unwrap();
    app.advance_game(0.05).unwrap();
    app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
    ready(&mut app);
    let ticks = app.play.simulation_ticks;
    let clock = app.play.simulation_time;
    let source = app.scene.change_token();
    // Exercise the real watcher-disappearance failure during cancellation.
    let checkpoint = app
        .resource_checkpoint
        .replace(crate::app::resource_checkpoint::Barrier::Requested);
    let watcher = app.watcher.take();
    let error = app.resume_restore_barrier().unwrap_err();
    assert!(error.to_string().contains("model watcher disappeared"));
    assert!(app.restore_play_frozen());
    assert!(app.restore_barrier.as_ref().unwrap().audio);
    // The original audio owner must still own its save fence after the error.
    assert!(
        app.audio
            .poll_checkpoint(|_| None, 8 * 1024 * 1024)
            .unwrap()
            .is_some()
    );
    app.advance_game(0.05).unwrap();
    assert_eq!(app.play.simulation_ticks, ticks);
    assert_eq!(app.play.simulation_time, clock);
    assert_eq!(app.scene.change_token(), source);
    app.resource_checkpoint = checkpoint;
    app.watcher = watcher;
    let recovered_at = Instant::now();
    release(&mut app);
    assert!(app.play.simulation_time >= recovered_at);
    assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
    assert!(
        app.audio
            .poll_checkpoint(|_| None, 8 * 1024 * 1024)
            .is_err()
    );
    app.advance_game(0.02).unwrap();
    assert!(app.play.simulation_ticks > ticks);
    app.stop_workers().unwrap();
}
