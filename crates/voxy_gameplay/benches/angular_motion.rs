//! Matched, alternating-order CPU motion comparison; not a total frame benchmark.
use std::{hint::black_box, time::Instant};
use voxy_gameplay::{AngularMotion, AngularMotionBatch};
use voxy_scene::{BehaviorRunner, SceneGraph, Transform};
fn build(count: usize) -> (SceneGraph, Vec<voxy_scene::NodeId>) {
    let mut scene = SceneGraph::new(count);
    let mut nodes = Vec::with_capacity(count);
    for index in 0..count {
        let parent = if index == 0 {
            None
        } else {
            Some(nodes[(index - 1) / 2])
        };
        let owner = scene.spawn(parent, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                AngularMotion {
                    axis: [0., 1., 0.],
                    radians_per_second: if index % 2 == 0 { 1. } else { -1. },
                },
            )
            .unwrap();
        nodes.push(owner);
    }
    (scene, nodes)
}
fn main() {
    println!("objects,path,median_ns,sample20_ns,max_ns");
    for count in [1000, 10000, 100000] {
        let (mut legacy, old_nodes) = build(count);
        let (mut scene, nodes) = build(count);
        let mut runner = BehaviorRunner::default();
        for owner in &old_nodes {
            let motion = *legacy.component::<AngularMotion>(*owner).unwrap().unwrap();
            runner.attach(&mut legacy, *owner, motion).unwrap();
        }
        let mut batch = AngularMotionBatch::new(&scene, count).unwrap();
        let mut samples = [Vec::with_capacity(21), Vec::with_capacity(21)];
        for round in 0..24 {
            for path in if round % 2 == 0 { [0, 1] } else { [1, 0] } {
                let start = Instant::now();
                if path == 0 {
                    runner.fixed_update(&mut legacy, 1. / 60.);
                } else {
                    batch.fixed_step(&mut scene, 1. / 60.).unwrap();
                }
                let elapsed = start.elapsed().as_nanos();
                if round >= 3 {
                    samples[path].push(elapsed);
                }
            }
            for index in [0, count / 2, count - 1] {
                let old = legacy.world_matrix(old_nodes[index]).unwrap();
                let new = scene.world_matrix(nodes[index]).unwrap();
                assert!(old.abs_diff_eq(new, 1e-5));
                black_box(new);
            }
        }
        for (path, values) in samples.iter_mut().enumerate() {
            values.sort_unstable();
            println!(
                "{count},{},{},{},{}",
                if path == 0 { "behavior" } else { "batch" },
                values[10],
                values[19],
                values[20]
            );
        }
    }
}
