//! CPU requirement/associated-data probe; not an end-to-end frame benchmark.
use std::{hint::black_box, time::Instant};
use voxy_scene::{AssociatedData, ComponentTable, SceneGraph, Transform};

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
fn main() {
    println!(
        "slots,requirement_stride,inactive_stride,eligible,workload,median_ns,sample20_ns,max_ns"
    );
    for count in [1_000, 10_000, 100_000] {
        for (requirements, inactive) in [(1, 0), (10, 0), (1, 2), (10, 2)] {
            probe(count, requirements, inactive);
        }
    }
}
fn probe(count: usize, requirement_stride: usize, inactive_stride: usize) {
    let mut scene = SceneGraph::new(count);
    for index in 0..count {
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(owner, index as u64).unwrap();
        if index % requirement_stride == 0 {
            scene.insert_component(owner, true).unwrap();
        }
        // Alternate requirement groups so sparse matches also lose half their owners.
        if inactive_stride != 0 && (index / requirement_stride) % inactive_stride == 1 {
            scene.set_active(owner, false).unwrap();
        }
    }
    let expected = (0..count)
        .filter(|index| {
            index % requirement_stride == 0
                && (inactive_stride == 0 || (index / requirement_stride) % inactive_stride != 1)
        })
        .map(|index| index as u64)
        .sum::<u64>();
    let eligible = scene.active_components_with::<u64, bool>().count();
    let mut table = ComponentTable::<[u64; 8]>::new(&scene);
    table
        .rebuild_active_with::<u64, bool, ()>(&scene, |_, value, _, _| Ok([*value; 8]))
        .unwrap();
    let joined = scene
        .active_components_with::<u64, bool>()
        .map(|(_, value, _)| *value)
        .sum::<u64>();
    let stored = table
        .query_mut(&scene, true)
        .unwrap()
        .map(|(_, value)| value[0])
        .sum::<u64>();
    assert_eq!(joined, expected);
    assert_eq!(stored, expected);
    assert_eq!(table.stored_len(), eligible);
    let report = |workload: &str, timings: (u128, u128, u128)| {
        println!(
            "{count},{requirement_stride},{inactive_stride},{eligible},{workload},{},{},{}",
            timings.0, timings.1, timings.2
        );
    };
    report(
        "active_requirement_scan",
        measure(|| {
            let checksum = scene
                .active_components_with::<u64, bool>()
                .map(|(_, value, _)| *value)
                .sum::<u64>();
            black_box(checksum);
        }),
    );
    report(
        "transactional_rebuild_64byte_rows",
        measure(|| {
            table
                .rebuild_active_with::<u64, bool, ()>(&scene, |_, value, _, previous| {
                    black_box(previous);
                    Ok([*value; 8])
                })
                .unwrap();
            black_box(table.stored_len());
        }),
    );
    report(
        "published_table_scan",
        measure(|| {
            let checksum = table
                .query_mut(&scene, true)
                .unwrap()
                .map(|(_, value)| value[0])
                .sum::<u64>();
            black_box(checksum);
        }),
    );
    associated_probe(&mut scene, &mut table, report);
}

fn associated_probe(
    scene: &mut SceneGraph,
    table: &mut ComponentTable<[u64; 8]>,
    report: impl Fn(&str, (u128, u128, u128)),
) {
    let owners: Vec<_> = scene
        .active_components_with::<u64, bool>()
        .map(|(owner, _, _)| owner)
        .collect();
    let mut associated = AssociatedData::<u64, bool, [u64; 8]>::new(scene);
    associated
        .synchronize::<()>(scene, |_, value, _, _| Ok([*value; 8]))
        .unwrap();
    report(
        "reuse_unchanged_64byte_rows",
        measure(|| {
            associated
                .synchronize::<()>(scene, |_, _, _, _| panic!("unchanged factory"))
                .unwrap();
        }),
    );
    report(
        "validated_associated_scan",
        measure(|| {
            black_box(
                associated
                    .query(scene)
                    .unwrap()
                    .map(|(_, value)| value[0])
                    .sum::<u64>(),
            );
        }),
    );
    report(
        "one_percent_churn_full_rebuild",
        measure(|| {
            for owner in owners.iter().step_by(100) {
                let value = scene.component_mut::<u64>(*owner).unwrap().unwrap();
                *value = value.wrapping_add(1);
            }
            table
                .rebuild_active_with::<u64, bool, ()>(scene, |_, value, _, _| Ok([*value; 8]))
                .unwrap();
        }),
    );
    // Align the reused table to the preceding workload before measuring churn.
    associated
        .synchronize::<()>(scene, |_, value, _, _| Ok([*value; 8]))
        .unwrap();
    report(
        "one_percent_churn_factory_reuse",
        measure(|| {
            for owner in owners.iter().step_by(100) {
                let value = scene.component_mut::<u64>(*owner).unwrap().unwrap();
                *value = value.wrapping_add(1);
            }
            let mut calls = 0;
            associated
                .synchronize::<()>(scene, |_, value, _, _| {
                    calls += 1;
                    Ok([*value; 8])
                })
                .unwrap();
            assert_eq!(calls, owners.len().div_ceil(100));
        }),
    );
    let current = scene
        .active_components_with::<u64, bool>()
        .map(|(_, value, _)| *value)
        .sum::<u64>();
    assert_eq!(
        associated
            .query(scene)
            .unwrap()
            .map(|(_, value)| value[0])
            .sum::<u64>(),
        current
    );
}
