//! Immutable scene extraction consumed by a separate owner-thread audio mixer.
//! Offline PCM verification, not an audio-device demonstration.
use glam::Vec3;
use voxy_audio::{Clip, Mixer, VoiceId, spatial_gains};
use voxy_scene::{NodeId, SceneGraph, Transform};
#[derive(Clone, Copy, Debug)]
enum SourceSnapshot {
    Playing { position: [f32; 3] },
    Paused,
    Removed,
}
fn extract(scene: &SceneGraph, node: NodeId) -> Result<SourceSnapshot, Box<dyn std::error::Error>> {
    match scene.active_in_hierarchy(node) {
        Ok(true) => Ok(SourceSnapshot::Playing {
            position: scene
                .world_matrix(node)?
                .transform_point3(Vec3::ZERO)
                .to_array(),
        }),
        Ok(false) => Ok(SourceSnapshot::Paused),
        Err(voxy_scene::SceneGraphError::InvalidNode) => Ok(SourceSnapshot::Removed),
        Err(error) => Err(error.into()),
    }
}
fn apply(
    mixer: &mut Mixer,
    voice: VoiceId,
    snapshot: SourceSnapshot,
) -> Result<(), voxy_audio::AudioError> {
    match snapshot {
        SourceSnapshot::Playing { position } => {
            let gains = spatial_gains([0.0; 3], [1.0, 0.0, 0.0], position, 1.0, 5.0)?;
            mixer.ramp_channel_gains(voice, gains, 0)?;
            mixer.pause(voice, false)
        }
        SourceSnapshot::Paused => mixer.pause(voice, true),
        SourceSnapshot::Removed => mixer.stop(voice),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = SceneGraph::new(3);
    let parent = scene.spawn(None, Transform::default())?;
    let source = scene.spawn(
        Some(parent),
        Transform {
            translation: -Vec3::X,
            ..Transform::default()
        },
    )?;
    let mut mixer = Mixer::new(48_000, 1, 1)?;
    let voice = mixer.play(Clip::new(48_000, vec![[0.5; 2], [0.25; 2]])?, 0, 1.0, true)?;
    let old_snapshot = extract(&scene, source)?;
    scene.set_local(
        parent,
        Transform {
            translation: Vec3::X * 2.0,
            ..Transform::default()
        },
    )?;
    let new_snapshot = extract(&scene, source)?;
    let mut output = [[0.0; 2]; 1];
    apply(&mut mixer, voice, old_snapshot)?;
    mixer.render(&mut output);
    assert!((output[0][0] - 0.5).abs() < f32::EPSILON && output[0][1].abs() < f32::EPSILON);
    apply(&mut mixer, voice, new_snapshot)?;
    mixer.render(&mut output);
    assert!(output[0][0].abs() < f32::EPSILON && (output[0][1] - 0.25).abs() < f32::EPSILON);
    scene.set_active(parent, false)?;
    apply(&mut mixer, voice, extract(&scene, source)?)?;
    mixer.render(&mut output);
    assert!(output[0].iter().all(|x| x.abs() < f32::EPSILON));
    scene.set_active(parent, true)?;
    apply(&mut mixer, voice, extract(&scene, source)?)?;
    mixer.render(&mut output);
    assert!((output[0][1] - 0.5).abs() < f32::EPSILON);
    scene.remove_subtree(parent)?;
    apply(&mut mixer, voice, extract(&scene, source)?)?;
    mixer.render(&mut output);
    assert!(output[0].iter().all(|x| x.abs() < f32::EPSILON));
    let replacement = scene.spawn(None, Transform::default())?;
    assert_ne!(replacement, source);
    assert!(matches!(extract(&scene, source)?, SourceSnapshot::Removed));
    mixer.play(Clip::new(48_000, vec![[0.1; 2]])?, 0, 1.0, false)?;
    println!(
        "SCENE AUDIO PASS: immutable extraction, parent motion pans source, inactive hierarchy pauses cursor, deletion releases voice"
    );
    Ok(())
}
