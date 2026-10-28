use super::*;
use crate::certified_lod::tests::{POSITIONS, variant};
use crate::{LodArchiveLimits, LodPolicy};
fn limits() -> LodArchiveLimits {
    LodArchiveLimits {
        bytes: 1024 * 1024,
        positions: 100,
        levels: 10,
        indices: 100,
        cells: 1000,
    }
}
fn original(subdivided: bool) -> CertifiedLodIndexSet {
    let mut positions = POSITIONS.to_vec();
    positions[0][0] = -0.;
    positions.push([f32::from_bits(1), 0., 0.]);
    let base = vec![0, 1, 3, 0, 3, 2];
    if subdivided {
        let approx = vec![0, 1, 2];
        let proof = crate::generate_subdivided_lod_witnesses(
            LodSurface {
                positions: &positions,
                indices: &base,
            },
            LodSurface {
                positions: &positions,
                indices: &approx,
            },
            1,
            1000,
        )
        .unwrap();
        CertifiedLodIndexSet::new_subdivided(
            positions,
            base,
            vec![CertifiedLodSubdivisionVariant {
                indices: approx,
                source_to_variant: proof.source_to_approximation,
                variant_to_source: proof.approximation_to_source,
            }],
        )
        .unwrap()
    } else {
        CertifiedLodIndexSet::new(positions, base, vec![variant()]).unwrap()
    }
}
#[test]
fn whole_and_subdivided_file_proofs_restore_exact_error_envelopes_and_policy_results() {
    let limits = limits();
    for subdivided in [false, true] {
        let source = original(subdivided);
        let record = source.checkpoint(limits).unwrap();
        let json = record.to_json(limits.bytes).unwrap();
        let path = std::env::temp_dir().join(format!(
            "voxy-certified-lod-{}-{}-{subdivided}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, &json).unwrap();
        let json = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let restored = CertifiedLodCheckpoint::from_json(&json, limits.bytes)
            .unwrap()
            .prepare(limits)
            .unwrap();
        assert_eq!(restored.checkpoint(limits).unwrap(), record);
        assert_eq!(
            source
                .positions()
                .iter()
                .map(|v| v.map(f32::to_bits))
                .collect::<Vec<_>>(),
            restored
                .positions()
                .iter()
                .map(|v| v.map(f32::to_bits))
                .collect::<Vec<_>>()
        );
        let mut previous_a = None;
        let mut previous_b = None;
        let mut seen = [false; 2];
        for phase in 0..120 {
            let policy = LodPolicy {
                target_pixels: 1.0,
                hysteresis: 0.1,
            };
            let pixels = 0.05 + f64::from(phase % 60) / 5.;
            let a = policy
                .select(source.indices().levels(), 1.0, pixels, previous_a)
                .unwrap();
            let b = policy
                .select(restored.indices().levels(), 1.0, pixels, previous_b)
                .unwrap();
            assert_eq!(a, b);
            seen[a] = true;
            previous_a = Some(a);
            previous_b = Some(b);
        }
        assert!(seen.into_iter().all(|v| v));
    }
}
#[test]
fn forged_errors_invalid_original_witnesses_counts_and_limits_reject_without_changing_live_lod() {
    let limits = limits();
    for subdivided in [false, true] {
        let source = original(subdivided);
        let record = source.checkpoint(limits).unwrap();
        let mut cases = Vec::new();
        let mut r = record.clone();
        r.version = 0;
        cases.push(r);
        let mut r = record.clone();
        r.levels[1].error ^= 1;
        cases.push(r);
        let mut r = record.clone();
        r.levels[0].count ^= 1;
        cases.push(r);
        let mut r = record.clone();
        r.index_bytes ^= 1;
        cases.push(r);
        let mut r = record.clone();
        r.positions[0][0] = f32::NAN.to_bits();
        cases.push(r);
        let mut r = record.clone();
        match &mut r.proofs {
            Proofs::Whole(v) => v[0].0[0].weights[0][0] = 0,
            Proofs::Subdivided(v) => v[0].0[0].cells[0].weights[0][0] = 0,
        }
        cases.push(r);
        let mut r = record.clone();
        match &mut r.proofs {
            Proofs::Whole(v) => {
                v[0].0.pop();
            }
            Proofs::Subdivided(v) => {
                v[0].0[0].cells.pop();
            }
        }
        cases.push(r);
        for record in cases {
            assert!(record.prepare(limits).is_err());
        }
        for limits in [
            LodArchiveLimits {
                positions: 0,
                ..limits
            },
            LodArchiveLimits {
                levels: 1,
                ..limits
            },
            LodArchiveLimits { cells: 0, ..limits },
            LodArchiveLimits {
                indices: 0,
                ..limits
            },
            LodArchiveLimits { bytes: 1, ..limits },
        ] {
            assert!(record.prepare(limits).is_err());
        }
        let json = record.to_json(limits.bytes).unwrap();
        assert!(CertifiedLodCheckpoint::from_json(&json, json.len() - 1).is_err());
        assert!(
            CertifiedLodCheckpoint::from_json(
                &json.replacen("\"version\":1", "\"unexpected\":1,\"version\":1", 1),
                limits.bytes
            )
            .is_err()
        );
        assert_eq!(source.checkpoint(limits).unwrap(), record);
    }
}
#[test]
fn base_only_lods_preserve_verifier_kind_and_exact_zero_error_receipt() {
    let limits = limits();
    let a = CertifiedLodIndexSet::new(POSITIONS.to_vec(), vec![0, 1, 2], vec![]).unwrap();
    let b =
        CertifiedLodIndexSet::new_subdivided(POSITIONS.to_vec(), vec![0, 1, 2], vec![]).unwrap();
    for source in [a, b] {
        let record = source.checkpoint(limits).unwrap();
        assert_eq!(
            record.prepare(limits).unwrap().checkpoint(limits).unwrap(),
            record
        );
    }
}
