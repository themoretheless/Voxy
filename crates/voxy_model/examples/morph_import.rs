//! Explicit CPU/reference inspection; does not launch an editor or claim GPU playback.
use voxy_model::{ModelAsset, ModelLimits, MorphModelAsset};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: morph_import model.glb")?;
    let bytes = std::fs::read(path)?;
    let strict_error = ModelAsset::parse(&bytes, &[], ModelLimits::default())
        .err()
        .map(|e| e.to_string());
    let asset = MorphModelAsset::parse(&bytes, &[], ModelLimits::default())?;
    let mut clips = Vec::new();
    for (index, clip) in asset.animations().iter().enumerate() {
        let mut weights = std::collections::BTreeSet::new();
        let mut geometry = std::collections::BTreeSet::new();
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        let mut vertices = 0;
        for tick in 0..=60 {
            let time = clip.duration() * tick as f32 / 60.;
            let sample = asset.sample_weights(Some(index), time)?;
            weights.insert(serde_json::to_string(&sample)?);
            let meshes = asset.sample_meshes(Some(index), time)?;
            let mut fingerprint = 0xcbf29ce484222325_u64;
            vertices = 0;
            for mesh in meshes {
                vertices += mesh.vertices().len();
                for vertex in mesh.vertices() {
                    for (axis, value) in vertex.position.into_iter().enumerate() {
                        if !value.is_finite() {
                            return Err("nonfinite reference geometry".into());
                        }
                        low[axis] = low[axis].min(value);
                        high[axis] = high[axis].max(value);
                        fingerprint ^= u64::from(value.to_bits());
                        fingerprint = fingerprint.wrapping_mul(0x100000001b3);
                    }
                }
            }
            geometry.insert(fingerprint);
        }
        clips.push(serde_json::json!({"name":clip.name(),"samples":61,"vertices_per_sample":vertices,"distinct_weight_samples":weights.len(),"distinct_geometry_samples":geometry.len(),"bounds_min":low,"bounds_max":high}));
    }
    // Upstream weight channels may be constant. Probe each imported shape
    // explicitly without rewriting the original asset or its animation curves.
    let baseline_weights = asset.sample_weights(None, 0.)?;
    let baseline = asset.sample_meshes(None, 0.)?;
    let target_count = baseline_weights.iter().map(Vec::len).max().unwrap_or(0);
    let mut shape_probes = Vec::new();
    for target in 0..target_count {
        let mut weights = baseline_weights.clone();
        for primitive in &mut weights {
            if let Some(weight) = primitive.get_mut(target) {
                *weight += 1.;
            }
        }
        let meshes = asset.sample_meshes_with_weights(None, 0., &weights)?;
        let mut changed_vertices = 0;
        let mut max_displacement = 0f64;
        for (source, morphed) in baseline.iter().zip(meshes) {
            for (a, b) in source.vertices().iter().zip(morphed.vertices()) {
                let distance = a
                    .position
                    .into_iter()
                    .zip(b.position)
                    .map(|(a, b)| (f64::from(b) - f64::from(a)).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if distance > 0. {
                    changed_vertices += 1;
                }
                max_displacement = max_displacement.max(distance);
            }
        }
        shape_probes.push(serde_json::json!({"target":target,"explicit_additional_weight":1,"changed_vertices":changed_vertices,"max_displacement":max_displacement}));
    }
    // Exercise the original Animator's clocks and interrupted transitions. All
    // samples carry their weight snapshot in the same immutable skeletal frame.
    let mut animator_frames = 0;
    if !asset.animations().is_empty() {
        let mut animator = asset.animator(0)?;
        for clip in asset.animations() {
            animator.transition_to(clip.clone(), 0.5)?;
            for _ in 0..8 {
                let frame = animator.advance_wall(asset.skeleton(), 1. / 60.)?;
                let weights = asset.frame_weights(&frame)?;
                if weights.iter().flatten().any(|w| !w.is_finite()) {
                    return Err("nonfinite Animator morph weights".into());
                }
                let meshes = asset.scene_meshes(&frame)?;
                if meshes.len() != asset.primitive_count() {
                    return Err("Animator frame lost source primitives".into());
                }
                animator_frames += 1;
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"scope":"explicit_cpu_reference_import_and_sampling","mesh_instances":asset.mesh_instance_count(),"primitives":asset.primitive_count(),"morph_primitives":asset.morph_primitive_count(),"joints":asset.skeleton().joints().len(),"clips":clips,"explicit_shape_probes":shape_probes,"animator_property_frames":animator_frames,"default_import_error":strict_error,"accepted_playback_integrated":false,"gpu_morph_integrated":false,"fps_qualified":false})
    );
    Ok(())
}
