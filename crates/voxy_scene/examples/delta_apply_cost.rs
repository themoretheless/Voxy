//! Cost of applying a one-object delta to snapshots of growing size.
//! Run: `cargo run --release -p voxy_scene --example delta_apply_cost`.
use std::collections::BTreeMap;
use std::time::Instant;
use voxy_scene::{ObjectId, SceneDocument, SceneObject, SceneSnapshotDelta, TickSnapshot};

fn document(objects: usize, moved: f32) -> SceneDocument {
    SceneDocument {
        version: 1,
        objects: (0..objects)
            .map(|index| SceneObject {
                precise_translation_bits: None,
                id: ObjectId(format!("object-{index:06}")),
                parent: None,
                name: format!("Object {index}"),
                active: true,
                translation: [
                    f32::from(u16::try_from(index % 1000).unwrap_or(0)),
                    if index == 0 { moved } else { 0. },
                    0.,
                ],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
                components: BTreeMap::new(),
            })
            .collect(),
    }
}

fn main() {
    for objects in [100, 1_000, 10_000, 100_000] {
        let base = TickSnapshot::from_document(1, document(objects, 0.)).unwrap();
        let target = TickSnapshot::from_document(2, document(objects, 1.)).unwrap();
        let delta = SceneSnapshotDelta::diff(&base, &target);
        let rounds = (200_000 / objects).max(3);
        let mut samples: Vec<f64> = (0..rounds)
            .map(|_| {
                let started = Instant::now();
                let applied = delta.apply(&base).unwrap();
                let elapsed = started.elapsed().as_secs_f64() * 1e3;
                assert_eq!(applied.state_crc32c, target.state_crc32c);
                elapsed
            })
            .collect();
        samples.sort_by(f64::total_cmp);
        println!(
            "objects={objects} rounds={rounds} apply_ms p50={:.4} p95={:.4}",
            samples[rounds / 2],
            samples[rounds * 95 / 100]
        );
    }
}
