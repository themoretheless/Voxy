//! Offline CPU proof-carrying LOD file qualification, with no device/window.
use std::{error::Error, fs, io::Write, path::PathBuf};
use voxy_render::{
    CertifiedLodCheckpoint, CertifiedLodIndexSet, CertifiedLodSubdivisionVariant,
    CertifiedLodVariant, LodArchiveLimits, LodPolicy, LodSurface,
};
fn main() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("supply a new output directory")?,
    );
    fs::create_dir(&path)?;
    let positions = vec![
        [-0., 0., 0.],
        [1., 0., 0.],
        [0., 1., 0.],
        [1., 1., 0.],
        [f32::from_bits(1), 0., 0.],
    ];
    let base = vec![0, 1, 3, 0, 3, 2];
    let approximation = vec![0, 1, 2];
    let source = LodSurface {
        positions: &positions,
        indices: &base,
    };
    let target = LodSurface {
        positions: &positions,
        indices: &approximation,
    };
    let limits = LodArchiveLimits {
        bytes: 1024 * 1024,
        positions: 10,
        levels: 3,
        indices: 100,
        cells: 1000,
    };
    let mut receipts = Vec::new();
    for subdivided in [false, true] {
        let original = if subdivided {
            let p = voxy_render::generate_subdivided_lod_witnesses(source, target, 1, 1000)
                .map_err(|e| format!("{e:?}"))?;
            CertifiedLodIndexSet::new_subdivided(
                positions.clone(),
                base.clone(),
                vec![CertifiedLodSubdivisionVariant {
                    indices: approximation.clone(),
                    source_to_variant: p.source_to_approximation,
                    variant_to_source: p.approximation_to_source,
                }],
            )
        } else {
            let p = voxy_render::generate_lod_witnesses(source, target, 1000)
                .map_err(|e| format!("{e:?}"))?;
            CertifiedLodIndexSet::new(
                positions.clone(),
                base.clone(),
                vec![CertifiedLodVariant {
                    indices: approximation.clone(),
                    source_to_variant: p.source_to_approximation,
                    variant_to_source: p.approximation_to_source,
                }],
            )
        }
        .map_err(|e| format!("{e:?}"))?;
        let checkpoint = original.checkpoint(limits)?;
        let filename = if subdivided {
            "subdivided.json"
        } else {
            "whole.json"
        };
        let json = checkpoint.to_json(limits.bytes)?;
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path.join(filename))?
            .write_all(json.as_bytes())?;
        let json = fs::read_to_string(path.join(filename))?;
        let restored = CertifiedLodCheckpoint::from_json(&json, limits.bytes)?.prepare(limits)?;
        if checkpoint != restored.checkpoint(limits)? {
            return Err("native LOD receipt differs".into());
        }
        let policy = LodPolicy {
            target_pixels: 1.,
            hysteresis: 0.1,
        };
        let mut previous_a = None;
        let mut previous_b = None;
        let mut seen = [false; 2];
        for frame in 0..120 {
            let pixels = 0.05 + f64::from(frame % 60) / 5.;
            let a = policy
                .select(original.indices().levels(), 1., pixels, previous_a)
                .map_err(|e| format!("{e:?}"))?;
            let b = policy
                .select(restored.indices().levels(), 1., pixels, previous_b)
                .map_err(|e| format!("{e:?}"))?;
            if a != b {
                return Err("LOD policy continuation differs".into());
            }
            seen[a] = true;
            previous_a = Some(a);
            previous_b = Some(b);
        }
        if !seen.into_iter().all(|v| v) {
            return Err("qualification did not exercise both levels".into());
        }
        receipts.push(serde_json::json!({"file":filename,"bytes":json.len(),"blake3":blake3::hash(json.as_bytes()).to_hex().to_string(),"original_error_bits":original.indices().levels()[1].object_error.to_bits(),"restored_error_bits":restored.indices().levels()[1].object_error.to_bits(),"exact_native_checkpoint_equal":true,"policy_phases":120,"both_levels_exercised":true}));
    }
    let receipt = serde_json::json!({"scope":"native static geometric LOD proof restoration; no GPU, material/normal/UV/deformation or frame-rate qualification","receipts":receipts});
    let json = serde_json::to_string_pretty(&receipt)?;
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path.join("receipt.json"))?
        .write_all(json.as_bytes())?;
    println!("{json}");
    Ok(())
}
