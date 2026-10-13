//! Real CC0 animation-library compatibility: no generated substitute clips.
use voxy_render::{ModelAsset, ModelLimits};

const DOCUMENT: &[u8] = include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf");
const BUFFER: &[u8] = include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin");

#[test]
fn real_library_imports_and_samples_every_clip_with_native_skinning() {
    let asset = ModelAsset::parse(DOCUMENT, &[BUFFER], ModelLimits::default()).unwrap();
    assert_eq!(asset.animations.len(), 46, "do not silently omit unsupported clips");
    for name in ["Idle_Loop", "Walk_Loop", "Jog_Fwd_Loop", "Sprint_Loop", "Jump_Start", "Jump_Loop", "Jump_Land"] {
        assert!(asset.animations.iter().any(|clip| clip.name() == name), "missing {name}");
    }
    for (index, clip) in asset.animations.iter().enumerate() {
        assert!(clip.duration().is_finite() && clip.duration() > 0., "{}", clip.name());
        for frame in 0..=120 {
            let phase = frame as f64 / 120.;
            let pose = asset.sample_pose_phase(Some(index), phase).unwrap();
            let palette = asset.skin_matrices(&pose).unwrap();
            assert!(palette.iter().all(|matrix| matrix.is_finite()), "{} at {phase}", clip.name());
        }
        for phase in [0., 0.25, 0.5, 0.75, 1.] {
            let pose = asset.sample_pose_phase64(Some(index), phase).unwrap();
            let surfaces = asset.scene_surfaces64(&pose).unwrap();
            assert!(!surfaces.is_empty());
            assert!(surfaces.iter().all(|surface| !surface.positions.is_empty()
                && surface.positions.iter().flatten().all(|value| value.is_finite())
                && surface.indices.iter().all(|&i| (i as usize) < surface.positions.len())));
        }
    }
    let walk = asset.animations.iter().position(|clip| clip.name() == "Walk_Loop").unwrap();
    let a = asset.scene_meshes(&asset.sample_pose_phase(Some(walk), 0.).unwrap()).unwrap();
    let b = asset.scene_meshes(&asset.sample_pose_phase(Some(walk), 0.25).unwrap()).unwrap();
    assert!(a.iter().zip(&b).any(|(a,b)|a.vertices().iter().zip(b.vertices()).any(|(a,b)|a.position != b.position)),
        "walk was imported but did not move actual mesh vertices");
    eprintln!("QUATERNIUS LIBRARY clips={} joints={} pose_samples={} wide_surface_samples={}",
        asset.animations.len(), asset.skeleton.joints().len(), 46*121, 46*5);
}

#[test]
fn real_locomotion_clips_preserve_model_rotations_through_identity_retarget() {
    use voxy_animation::{RetargetBinding, RetargetJoint};
    let asset = ModelAsset::parse(DOCUMENT, &[BUFFER], ModelLimits::default()).unwrap();
    let mapping: Vec<_> = asset.skeleton.joints().iter().map(|joint|RetargetJoint {
        source: joint.name.clone(), target: joint.name.clone(), rotation_basis: glam::Quat::IDENTITY,
        translation_basis: glam::Quat::IDENTITY, translation_scale: 1.,
    }).collect();
    let binding = RetargetBinding::new(&asset.skeleton,&asset.skeleton,&mapping).unwrap()
        .compile_model_rotations(glam::Quat::IDENTITY).unwrap();
    for name in ["Idle_Loop","Walk_Loop","Jog_Fwd_Loop","Sprint_Loop"] {
        let index = asset.animations.iter().position(|clip|clip.name()==name).unwrap();
        for frame in 0..=120 {
            let pose = asset.sample_pose_phase(Some(index),frame as f64/120.).unwrap();
            let transported = binding.apply_pose(&pose).unwrap();
            let original = asset.skin_matrices(&pose).unwrap();
            let actual = asset.skin_matrices(&transported).unwrap();
            for (a,b) in original.iter().zip(actual) {
                assert!(a.abs_diff_eq(b,2e-5),"identity model retarget drift: {name} frame {frame}");
            }
        }
    }
}
