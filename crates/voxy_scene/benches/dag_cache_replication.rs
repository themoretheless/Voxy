//! Isolated CPU micro-benchmarks for CRC-32C, scene snapshot delta replication,
//! topological schedule execution, asset dependency DAG wavefronts, and sharded asset caching.

use std::collections::BTreeMap;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use voxy_assets::{AssetCache, AssetDependencies, AssetId};
use voxy_core::crc32c;
use voxy_scene::{
    ObjectId, SceneDocument, SceneObject, SceneSnapshotDelta, SchedulePlan, SystemAccess,
    SystemSpec, TickSnapshot,
};

fn measure(mut operation: impl FnMut()) -> (u128, u128, u128) {
    for _ in 0..3 {
        operation();
    }
    let mut samples = Vec::with_capacity(21);
    for _ in 0..21 {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort_unstable();
    (samples[10], samples[19], samples[20])
}

fn report(workload: &str, timings: (u128, u128, u128)) {
    println!("{workload},{},{},{}", timings.0, timings.1, timings.2);
}

fn make_document(objects: usize, shifted_stride: usize, moved: f32) -> SceneDocument {
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
                    if shifted_stride > 0 && index.is_multiple_of(shifted_stride) {
                        moved
                    } else {
                        0.
                    },
                    0.,
                ],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
                components: BTreeMap::new(),
            })
            .collect(),
    }
}

fn bench_crc32c() {
    let payload: Vec<u8> = (0..1_048_576_usize)
        .map(|i| ((i.wrapping_mul(131)) ^ (i >> 7)) as u8)
        .collect();
    report(
        "crc32c_1mb",
        measure(|| {
            black_box(crc32c(&payload));
        }),
    );
}

fn bench_replication() {
    let base_1k = TickSnapshot::from_document(1, make_document(1_000, 1_000, 0.)).unwrap();
    let target_1k = TickSnapshot::from_document(2, make_document(1_000, 1_000, 1.)).unwrap();
    let delta_1k = SceneSnapshotDelta::diff(&base_1k, &target_1k);

    let base_10k = TickSnapshot::from_document(1, make_document(10_000, 2_500, 0.)).unwrap();
    let target_10k = TickSnapshot::from_document(2, make_document(10_000, 2_500, 1.)).unwrap();
    let delta_10k = SceneSnapshotDelta::diff(&base_10k, &target_10k);

    report(
        "delta_diff_10k",
        measure(|| {
            let delta = SceneSnapshotDelta::diff(&base_10k, &target_10k);
            assert_eq!(delta.upserted.len(), 4);
            black_box(delta);
        }),
    );
    report(
        "delta_apply_1k",
        measure(|| {
            let applied = delta_1k.apply(&base_1k).unwrap();
            assert_eq!(applied.state_crc32c, target_1k.state_crc32c);
            black_box(applied);
        }),
    );
    report(
        "delta_apply_10k",
        measure(|| {
            let applied = delta_10k.apply(&base_10k).unwrap();
            assert_eq!(applied.state_crc32c, target_10k.state_crc32c);
            black_box(applied);
        }),
    );
}

fn bench_schedule_dag() {
    let mut specs = Vec::with_capacity(128);
    for i in 0..128_usize {
        let phase = u16::try_from(i / 16).unwrap_or(0);
        let mut after = Vec::new();
        if i >= 16 {
            after.push(format!("sys-{}", i - 16));
            if i.is_multiple_of(3) {
                after.push(format!("sys-{}", i - 15));
            }
        }
        let access = vec![
            SystemAccess {
                resource: format!("res-{}", i % 24),
                write: i.is_multiple_of(5),
            },
            SystemAccess {
                resource: "scene".into(),
                write: false,
            },
        ];
        specs.push(SystemSpec {
            name: format!("sys-{i}"),
            phase,
            after,
            access,
        });
    }
    report(
        "schedule_build_128",
        measure(|| {
            let plan = SchedulePlan::build(&specs, 256).unwrap();
            black_box(plan.batches().len());
        }),
    );

    let batch_specs: Vec<SystemSpec> = (0..16)
        .map(|i| SystemSpec {
            name: format!("w-{i:02}"),
            phase: 0,
            after: Vec::new(),
            access: vec![SystemAccess {
                resource: format!("slot-{i}"),
                write: true,
            }],
        })
        .collect();
    let batch_plan = SchedulePlan::build(&batch_specs, 32).unwrap();
    assert_eq!(batch_plan.batches().len(), 1);
    report(
        "schedule_run_parallel_straggler",
        measure(|| {
            let results = batch_plan
                .run_parallel(4, |name, access| -> Result<u64, ()> {
                    let idx: usize = name[2..].parse().unwrap_or(0);
                    access.require_write(&format!("slot-{idx}")).unwrap();
                    let rounds = if idx.is_multiple_of(4) { 25_000 } else { 500 };
                    let mut acc = 0x9E37_79B9_7F4A_7C15_u64 ^ idx as u64;
                    for step in 0..rounds {
                        acc = acc.wrapping_mul(0xBF58_476D_1CE4_E5B9) ^ step as u64;
                    }
                    Ok(black_box(acc))
                })
                .unwrap();
            assert_eq!(results.len(), 16);
            black_box(results);
        }),
    );
}

fn bench_asset_dag() {
    let mut deps = AssetDependencies::new(2_500, 8_000);
    for i in 0..2_000_usize {
        deps.declare(AssetId(format!("asset-{i:04}"))).unwrap();
    }
    for i in 1..2_000_usize {
        let mut parents = vec![AssetId(format!("asset-{:04}", (i - 1) / 2))];
        if i > 10 && i.is_multiple_of(7) {
            parents.push(AssetId(format!("asset-{:04}", i - 7)));
        }
        deps.set(&AssetId(format!("asset-{i:04}")), parents)
            .unwrap();
    }
    let changed = [AssetId("asset-1990".into())];
    report(
        "asset_dag_wavefront_2k_leaf",
        measure(|| {
            let layers = deps.wavefront_layers(changed.clone()).unwrap();
            assert_eq!(layers, vec![vec![AssetId("asset-1990".into())]]);
            black_box(layers);
        }),
    );
}

fn bench_asset_cache() {
    let hit_cache = Arc::new(AssetCache::new());
    for i in 0..32_usize {
        let key = format!("shared-{i}");
        let val = hit_cache
            .load_or_cache(&key, || Ok(vec![i as u8; 1024]))
            .unwrap();
        assert_eq!(val.len(), 1024);
    }
    report(
        "asset_cache_concurrent_hits_8t",
        measure(|| {
            std::thread::scope(|scope| {
                for t in 0..8_usize {
                    let cache = Arc::clone(&hit_cache);
                    scope.spawn(move || {
                        for iter in 0..250_usize {
                            let key = format!("shared-{}", (t * 7 + iter) % 32);
                            let v: Vec<u8> = cache
                                .load_or_cache(&key, || unreachable!("already cached"))
                                .unwrap();
                            black_box(v.len());
                        }
                    });
                }
            });
        }),
    );

    let mut miss_round = 0_usize;
    report(
        "asset_cache_concurrent_misses_8t",
        measure(|| {
            miss_round += 1;
            let cache = Arc::new(AssetCache::new());
            std::thread::scope(|scope| {
                for t in 0..8_usize {
                    let cache = Arc::clone(&cache);
                    let round = miss_round;
                    scope.spawn(move || {
                        let key = format!("miss-{round}-{t}");
                        let val = cache
                            .load_or_cache(&key, || {
                                let mut h = 0x517c_c1b7_2722_0a95_u64 ^ t as u64;
                                for k in 0..50_000_u64 {
                                    h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ k;
                                }
                                Ok(vec![h; 64])
                            })
                            .unwrap();
                        assert_eq!(val.len(), 64);
                        black_box(val);
                    });
                }
            });
        }),
    );
}

fn main() {
    println!("workload,p50_ns,p95_ns,max_ns");
    bench_crc32c();
    bench_replication();
    bench_schedule_dag();
    bench_asset_dag();
    bench_asset_cache();
}
