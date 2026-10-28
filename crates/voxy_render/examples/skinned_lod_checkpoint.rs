//! Offline skeletal LOD source-file qualification; no device or window.
use glam::{Mat4, Vec3};
use std::{error::Error, fs, io::Write, path::PathBuf, sync::Arc};
use voxy_render::{
    CertifiedLodSubdivisionVariant, CertifiedLodVariant, LodSurface, SkinnedLodCheckpoint,
    SkinnedLodCheckpointLimits, SkinnedLodMesh, SkinnedMesh, SkinnedVertex,
};
fn main() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("supply a new output directory")?,
    );
    fs::create_dir(&path)?;
    let mesh = Arc::new(
        SkinnedMesh::new(
            [[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.], [0., 0., 0.]]
                .into_iter()
                .enumerate()
                .map(|(i, position)| SkinnedVertex {
                    position,
                    normal: [0., 0., 1.],
                    uv: [0.; 2],
                    joints: [u16::from(i == 3), 0, 0, 0],
                    weights: [u16::MAX, 0, 0, 0],
                })
                .collect(),
            vec![0, 1, 2, 0, 1, 3],
            2,
        )
        .map_err(|e| format!("{e:?}"))?,
    );
    let positions: Vec<_> = mesh.vertices().iter().map(|v| v.position).collect();
    let indices = vec![0, 1, 2];
    let source = LodSurface {
        positions: &positions,
        indices: mesh.indices(),
    };
    let target = LodSurface {
        positions: &positions,
        indices: &indices,
    };
    let limits = SkinnedLodCheckpointLimits::default();
    let mut receipts = Vec::new();
    for subdivided in [false, true] {
        let original = if subdivided {
            let p = voxy_render::generate_subdivided_lod_witnesses(source, target, 1, 1000)
                .map_err(|e| format!("{e:?}"))?;
            SkinnedLodMesh::new_subdivided(
                mesh.clone(),
                vec![CertifiedLodSubdivisionVariant {
                    indices: indices.clone(),
                    source_to_variant: p.source_to_approximation,
                    variant_to_source: p.approximation_to_source,
                }],
            )
        } else {
            let p = voxy_render::generate_lod_witnesses(source, target, 1000)
                .map_err(|e| format!("{e:?}"))?;
            SkinnedLodMesh::new(
                mesh.clone(),
                vec![CertifiedLodVariant {
                    indices: indices.clone(),
                    source_to_variant: p.source_to_approximation,
                    variant_to_source: p.approximation_to_source,
                }],
            )
        }
        .map_err(|e| format!("{e:?}"))?;
        let record = original.checkpoint(limits)?;
        let filename = if subdivided {
            "subdivided.json"
        } else {
            "whole.json"
        };
        let json = record.to_json(limits.max_bytes)?;
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path.join(filename))?
            .write_all(json.as_bytes())?;
        let json = fs::read_to_string(path.join(filename))?;
        let decoded = SkinnedLodCheckpoint::from_json(&json, limits.max_bytes)?;
        let restored = decoded.prepare(limits)?;
        let bound = decoded.prepare_with_mesh(mesh.clone(), limits)?;
        if !Arc::ptr_eq(bound.source_mesh(), &mesh) {
            return Err("external source mesh identity differs".into());
        }
        let mut trajectory = blake3::Hasher::new();
        let mut errors = Vec::new();
        let mut checks = 0;
        for phase in 0..120 {
            let t = phase as f32 / 119. * std::f32::consts::TAU;
            let palette = [
                Mat4::IDENTITY,
                Mat4::from_translation(Vec3::new(t.cos() * 0.1, 0., t.sin() * 0.5)),
            ];
            let model = Mat4::from_scale(Vec3::new(1.5, 0.5, 2.));
            let a = original
                .prepare(&palette, model)
                .map_err(|e| format!("{e:?}"))?;
            let b = restored
                .prepare(&palette, model)
                .map_err(|e| format!("{e:?}"))?;
            let words = |p: &[[f32; 3]]| p.iter().map(|v| v.map(f32::to_bits)).collect::<Vec<_>>();
            if words(a.positions()) != words(b.positions()) {
                return Err("posed position words differ".into());
            }
            for (a, b) in a.levels().iter().zip(b.levels()) {
                if a.object_error.to_bits() != b.object_error.to_bits()
                    || a.index_count != b.index_count
                {
                    return Err("pose quality receipts differ".into());
                }
                trajectory.update(&a.object_error.to_bits().to_le_bytes());
                trajectory.update(&a.index_count.to_le_bytes());
            }
            errors.push(a.levels()[1].object_error.to_bits());
            for level in 0..2 {
                if a.posed_scene_mesh(level, [1.; 4])?.checkpoint()
                    != b.posed_scene_mesh(level, [1.; 4])?.checkpoint()
                {
                    return Err("posed geometry differs".into());
                }
                checks += 1;
            }
        }
        if !errors.windows(2).any(|v| v[0] != v[1]) {
            return Err("qualification failed to vary deformed bounds".into());
        }
        if restored.checkpoint(limits)? != record {
            return Err("source data changed during continuation".into());
        }
        receipts.push(serde_json::json!({"file":filename,"bytes":json.len(),"blake3":blake3::hash(json.as_bytes()).to_hex().to_string(),"pose_frames":120,"exact_geometry_checks":checks,"quality_trajectory_blake3":trajectory.finalize().to_hex().to_string(),"quality_varies_with_pose":true,"external_mesh_arc_preserved":true,"source_checkpoint_equal_after_continuation":true}));
    }
    let json = serde_json::to_string_pretty(
        &serde_json::json!({"scope":"native CPU skeletal source and pose-specific quality; no GPU/frame-rate qualification","receipts":receipts}),
    )?;
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path.join("receipt.json"))?
        .write_all(json.as_bytes())?;
    println!("{json}");
    Ok(())
}
