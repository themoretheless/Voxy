//! CPU qualification of the complete model resource file, without image/GPU/editor ownership.
use std::{error::Error, fs, io::Write, path::PathBuf, sync::Arc};
use voxy_model::{ModelAsset, ModelAssetCheckpoint, ModelAssetCheckpointLimits, ModelLimits};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let source_path = PathBuf::from(
        args.next()
            .ok_or("usage: model_checkpoint source.glb new-output-directory")?,
    );
    let output = PathBuf::from(args.next().ok_or("missing output directory")?);
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let bytes = fs::read(&source_path)?;
    let source_digest = blake3::hash(&bytes).to_hex().to_string();
    let parsed = gltf::Gltf::from_slice(&bytes)?;
    let blob = parsed
        .blob
        .as_deref()
        .ok_or("example requires embedded GLB buffer")?;
    let model = Arc::new(ModelAsset::parse_animated(
        &bytes,
        &[blob],
        ModelLimits::default(),
    )?);
    let models = vec![model.clone(), Arc::new(model.as_ref().clone()), model];
    let limits = ModelAssetCheckpointLimits::default();
    let record = ModelAssetCheckpoint::capture(&models, limits)?;
    let archive = record.to_bytes(limits)?;
    // create_dir and create_new preserve any previous qualification output.
    fs::create_dir(&output)?;
    let archive_path = output.join("model.voxymdl");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&archive_path)?
        .write_all(&archive)?;
    drop(parsed);
    drop(bytes);
    let file = fs::read(&archive_path)?;
    let restored = ModelAssetCheckpoint::from_bytes(&file, limits)?.prepare(limits)?;
    if ModelAssetCheckpoint::capture(&restored, limits)? != record {
        return Err("resource graph differs".into());
    }
    if !Arc::ptr_eq(&restored[0], &restored[2]) || Arc::ptr_eq(&restored[0], &restored[1]) {
        return Err("outer model identity differs".into());
    }
    let mut checks = 0usize;
    for (source, restored) in models.iter().zip(&restored) {
        let clips: Vec<_> = if source.animations.is_empty() {
            vec![None]
        } else {
            (0..source.animations.len()).map(Some).collect()
        };
        for clip in clips {
            for phase in 0..120 {
                let phase = f64::from(phase) / 119.;
                let a = source.sample_frame_phase(clip, phase)?;
                let b = restored.sample_frame_phase(clip, phase)?;
                if a.pose.checkpoint() != b.pose.checkpoint() {
                    return Err("pose words differ".into());
                }
                let words = |values: Vec<Vec<f32>>| {
                    values
                        .into_iter()
                        .map(|v| v.into_iter().map(f32::to_bits).collect::<Vec<_>>())
                        .collect::<Vec<_>>()
                };
                if words(source.frame_weights(&a)?) != words(restored.frame_weights(&b)?) {
                    return Err("weight words differ".into());
                }
                let a = source.scene_meshes_frame(&a)?;
                let b = restored.scene_meshes_frame(&b)?;
                if a.iter()
                    .map(|m| m.checkpoint())
                    .ne(b.iter().map(|m| m.checkpoint()))
                {
                    return Err("posed mesh words differ".into());
                }
                checks += 1;
            }
        }
    }
    if ModelAssetCheckpoint::capture(&models, limits)?
        != ModelAssetCheckpoint::capture(&restored, limits)?
    {
        return Err("continued resource graph differs".into());
    }
    let receipt = serde_json::json!({
        "scope":"CPU native ModelAsset file; images, LOD, import provenance and GPU/editor publication are outside this owner",
        "source_blake3":source_digest,
        "archive":"model.voxymdl",
        "archive_bytes":file.len(),
        "archive_blake3":blake3::hash(&file).to_hex().to_string(),
        "model_references":restored.len(),
        "animation_clips":restored[0].animations.len(),
        "phases_per_clip":120,
        "exact_pose_weight_mesh_checks":checks,
        "original_source_bytes_dropped_before_prepare":true,
        "original_resource_resolver_supplied_to_prepare":false,
        "canonical_resource_graph_equal_before_and_after_continuation":true,
        "outer_model_aliases_equal":true,
        "gpu_or_fps_qualification":false
    });
    let json = serde_json::to_string_pretty(&receipt)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("receipt.json"))?
        .write_all(json.as_bytes())?;
    println!("{json}");
    Ok(())
}
