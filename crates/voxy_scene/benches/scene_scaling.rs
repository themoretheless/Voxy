//! Isolated CPU scaling probe, not a complete game-frame benchmark.
use glam::Vec3;
use std::{hint::black_box, time::Instant};
use voxy_scene::{ComponentTable, SceneExtraction, SceneGraph, Transform, extraction_schedule};
fn measure(mut operation: impl FnMut()) -> (u128, u128, u128) {
    for _ in 0..3 {
        operation();
    }
    let mut samples = Vec::new();
    for _ in 0..21 {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort_unstable();
    (samples[10], samples[19], samples[20])
}
fn report(count: usize, workload: &str, timings: (u128, u128, u128)) {
    println!(
        "{count},{workload},{},{},{}",
        timings.0, timings.1, timings.2
    );
}
fn main() {
    println!("objects,workload,p50_ns,p95_ns,p99_ns");
    for count in [1_000, 10_000, 100_000] {
        run_scene(count);
        hierarchy_probe(count, false);
        hierarchy_probe(count, true);
    }
}
fn run_scene(count: usize) {
    let (mut scene, nodes, mut table) = build_scene(count);
    report(
        count,
        "cached_world_reads",
        measure(|| {
            for node in &nodes {
                black_box(scene.world_matrix(*node).unwrap());
            }
        }),
    );
    projection_measure(count, &mut scene);
    let mut phase = false;
    report(
        count,
        "root_edit",
        measure(|| {
            phase = !phase;
            scene
                .set_local(
                    nodes[0],
                    Transform {
                        translation: if phase { Vec3::X } else { Vec3::ZERO },
                        ..Transform::default()
                    },
                )
                .unwrap();
            black_box(scene.world_matrix(*nodes.last().unwrap()).unwrap());
        }),
    );
    report(
        count,
        "one_percent_leaf_edits",
        measure(|| {
            phase = !phase;
            for node in &nodes[count - count / 100..] {
                scene
                    .set_local(
                        *node,
                        Transform {
                            translation: if phase { Vec3::Y } else { Vec3::ZERO },
                            ..Transform::default()
                        },
                    )
                    .unwrap();
            }
            black_box(scene.world_matrix(*nodes.last().unwrap()).unwrap());
        }),
    );
    report(
        count,
        "all_node_edits",
        measure(|| {
            phase = !phase;
            for node in &nodes {
                scene
                    .set_local(
                        *node,
                        Transform {
                            translation: if phase { Vec3::Z } else { Vec3::ZERO },
                            ..Transform::default()
                        },
                    )
                    .unwrap();
            }
            black_box(scene.world_matrix(*nodes.last().unwrap()).unwrap());
        }),
    );
    batch_measure(count, &mut scene, &nodes);
    report(
        count,
        "node_hashmap_updates",
        measure(|| {
            for node in &nodes {
                if scene.active_in_hierarchy(*node).unwrap() {
                    let value = scene.component_mut::<u64>(*node).unwrap().unwrap();
                    *value = value.wrapping_add(1);
                    black_box(*value);
                }
            }
        }),
    );
    report(
        count,
        "typed_table_updates",
        measure(|| {
            for (_, value) in table.query_mut(&scene, true).unwrap() {
                *value = value.wrapping_add(1);
                black_box(*value);
            }
        }),
    );
}

// Alternate first-run order to avoid treating sequential cache/load effects as
// scheduler speedups. Both paths reuse one publication/staging allocation pair.
fn projection_measure(count: usize, scene: &mut SceneGraph) {
    let mut projection = SceneExtraction::<u64>::new(count);
    let plan = extraction_schedule().unwrap();
    let mut serial = Vec::with_capacity(21);
    let mut scoped_samples = Vec::with_capacity(21);
    for round in 0..24 {
        let order = if round % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for scoped in order {
            let start = Instant::now();
            if scoped {
                plan.run_scene(scene, |_, access| {
                    projection.refresh_scoped_with(access, |scene, owner| scene.world_matrix(owner))
                })
                .unwrap();
            } else {
                projection.refresh(scene).unwrap();
            }
            let elapsed = start.elapsed().as_nanos();
            assert_eq!(projection.instances().len(), count);
            black_box(projection.instances());
            if round >= 3 {
                if scoped {
                    scoped_samples.push(elapsed);
                } else {
                    serial.push(elapsed);
                }
            }
        }
    }
    serial.sort_unstable();
    scoped_samples.sort_unstable();
    report(
        count,
        "projection_serial",
        (serial[10], serial[19], serial[20]),
    );
    report(
        count,
        "projection_scoped",
        (scoped_samples[10], scoped_samples[19], scoped_samples[20]),
    );
}

fn build_scene(count: usize) -> (SceneGraph, Vec<voxy_scene::NodeId>, ComponentTable<u64>) {
    let mut scene = SceneGraph::new(count);
    let mut nodes = Vec::with_capacity(count);
    for index in 0..count {
        let parent = if index == 0 {
            None
        } else {
            Some(nodes[(index - 1) / 2])
        };
        let node = scene.spawn(parent, Transform::default()).unwrap();
        scene.insert_component(node, 0_u64).unwrap();
        nodes.push(node);
    }
    let mut table = ComponentTable::new(&scene);
    for node in &nodes {
        table.insert(&scene, *node, 0_u64).unwrap();
    }
    (scene, nodes, table)
}

fn batch_measure(count: usize, scene: &mut SceneGraph, nodes: &[voxy_scene::NodeId]) {
    let mut phase = false;
    let mut edits: Vec<_> = nodes.iter().map(|id| (*id, Transform::default())).collect();
    report(
        count,
        "all_node_batch_edits",
        measure(|| {
            phase = !phase;
            for (_, transform) in &mut edits {
                transform.translation = if phase { Vec3::Z } else { Vec3::ZERO };
            }
            scene.set_locals(&edits).unwrap();
            black_box(scene.world_matrix(*nodes.last().unwrap()).unwrap());
        }),
    );
}

// Deliberately omit eager all-node edits on chains: that workload revisits
// descendants quadratically. Probe the batch barrier and sparse leaf edit.
fn hierarchy_probe(count: usize, chain: bool) {
    let mut scene = SceneGraph::new(count);
    let mut nodes = Vec::with_capacity(count);
    for index in 0..count {
        let parent = if index == 0 {
            None
        } else if chain {
            Some(nodes[index - 1])
        } else {
            Some(nodes[0])
        };
        nodes.push(scene.spawn(parent, Transform::default()).unwrap());
    }
    let prefix = if chain { "chain" } else { "wide" };
    let mut phase = false;
    let leaf = *nodes.last().unwrap();
    report(
        count,
        &format!("{prefix}_single_leaf_batch"),
        measure(|| {
            phase = !phase;
            scene
                .set_locals(&[(
                    leaf,
                    Transform {
                        translation: if phase { Vec3::X } else { Vec3::ZERO },
                        ..Transform::default()
                    },
                )])
                .unwrap();
            black_box(scene.world_matrix(leaf).unwrap());
        }),
    );
    let mut edits: Vec<_> = nodes.iter().map(|id| (*id, Transform::default())).collect();
    report(
        count,
        &format!("{prefix}_all_node_batch"),
        measure(|| {
            phase = !phase;
            for (_, transform) in &mut edits {
                transform.translation = if phase { Vec3::X } else { Vec3::ZERO };
            }
            scene.set_locals(&edits).unwrap();
            black_box(scene.world_matrix(leaf).unwrap());
        }),
    );
}
