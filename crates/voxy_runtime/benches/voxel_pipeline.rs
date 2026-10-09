//! Isolated CPU probe of the voxel edit -> rebuild pipeline, not a full game frame.
//!
//! Measures canonical chunk repacking, world commits, greedy meshing, lighting and the
//! combined resident rebuild on the procedural bootstrap scene used by `voxy_app`.
use std::collections::BTreeSet;
use std::{hint::black_box, time::Instant};

use voxy_core::{CancelToken, ChunkPos, LocalPos, VoxelPos};
use voxy_lighting::{LightStamp, LightingBudget, LightingInput, build_light};
use voxy_mesher::{HALO_OFFSETS, MeshStamp, MeshingInput, build_mesh};
use voxy_runtime::{build_procedural_scene, rebuild_bootstrap_chunks};
use voxy_world::{
    BlockStateId, EditSource, EditTxn, PalettedBlocks, ResourceKey, VoxelView, VoxelWrite, World,
};

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

fn report(workload: &str, timings: (u128, u128, u128)) {
    println!("{workload},{},{},{}", timings.0, timings.1, timings.2);
}

fn state(world: &World, name: &str) -> BlockStateId {
    world
        .registry()
        .find(&ResourceKey::parse(format!("voxy:{name}")).unwrap())
        .unwrap()
}

fn main() {
    println!("workload,p50_ns,p95_ns,max_ns");
    let scene = build_procedural_scene(0x56_4f_58_59, 1).expect("bootstrap scene");
    let mut world = scene.world;
    let positions: Vec<ChunkPos> = scene.chunks.iter().map(|chunk| chunk.pos).collect();
    let target = richest_chunk(&world, &positions);
    palette_workloads(&world, target);
    commit_workloads(&mut world, target);
    derived_workloads(&world, &positions);
    let quads: usize = scene
        .chunks
        .iter()
        .map(|chunk| chunk.mesh.quad_count())
        .sum();
    eprintln!("resident chunks {} total quads {quads}", positions.len());
}

/// The resident chunk with the richest palette is the edit target.
fn richest_chunk(world: &World, positions: &[ChunkPos]) -> ChunkPos {
    let target = positions
        .iter()
        .copied()
        .max_by_key(|pos| match &world.chunk(*pos).unwrap().data.blocks {
            PalettedBlocks::Uniform(_) => 0,
            PalettedBlocks::Packed { palette, .. } => palette.len(),
            PalettedBlocks::Direct(_) => 1000,
        })
        .unwrap();
    let representation = match &world.chunk(target).unwrap().data.blocks {
        PalettedBlocks::Uniform(_) => "uniform".to_string(),
        PalettedBlocks::Packed {
            palette,
            bits_per_index,
            ..
        } => format!("packed{}x{}", palette.len(), bits_per_index),
        PalettedBlocks::Direct(_) => "direct".to_string(),
    };
    eprintln!("target chunk {target:?} representation {representation}");
    target
}

fn palette_workloads(world: &World, target: ChunkPos) {
    let blocks = world.chunk(target).unwrap().data.blocks.clone();
    let stone = state(world, "stone");
    let dirt = state(world, "dirt");
    let center = LocalPos::new(16, 16, 16).unwrap().index();
    report(
        "palette_to_dense",
        measure(|| {
            black_box(blocks.to_dense());
        }),
    );
    let dense = blocks.to_dense();
    report(
        "palette_from_dense",
        measure(|| {
            black_box(PalettedBlocks::from_dense(dense.clone()).unwrap());
        }),
    );
    report(
        "palette_with_updates_1",
        measure(|| {
            black_box(blocks.with_updates(&[(center, stone)]).unwrap());
        }),
    );
    let many: Vec<_> = (0..64_u8)
        .map(|i| {
            (
                LocalPos::new(i % 32, 8 + i / 32, 20).unwrap().index(),
                if i % 2 == 0 { stone } else { dirt },
            )
        })
        .collect();
    report(
        "palette_with_updates_64",
        measure(|| {
            black_box(blocks.with_updates(&many).unwrap());
        }),
    );
    let present: BTreeSet<_> = dense.iter().copied().collect();
    let new_block = [
        "water_1", "water_2", "water_3", "water_4", "water_5", "water_6", "water_7", "grass",
        "dirt", "stone",
    ]
    .into_iter()
    .map(|name| state(world, name))
    .find(|candidate| !present.contains(candidate))
    .expect("a registry block absent from the target chunk");
    report(
        "palette_with_updates_new_palette_entry",
        measure(|| {
            black_box(blocks.with_updates(&[(center, new_block)]).unwrap());
        }),
    );
}

fn commit_workloads(world: &mut World, target: ChunkPos) {
    let stone = state(world, "stone");
    let air = state(world, "air");
    let edit_pos = VoxelPos {
        x: target.x * 32 + 16,
        y: target.y * 32 + 16,
        z: target.z * 32 + 16,
    };
    let mut phase = false;
    report(
        "world_commit_1_write",
        measure(|| {
            phase = !phase;
            black_box(
                world
                    .commit(EditTxn {
                        source: EditSource::Simulation,
                        expected: Vec::new(),
                        writes: vec![VoxelWrite {
                            pos: edit_pos,
                            block: if phase { stone } else { air },
                        }],
                    })
                    .unwrap(),
            );
        }),
    );
    report(
        "world_commit_64_writes",
        measure(|| {
            phase = !phase;
            let writes = (0..64_i64)
                .map(|i| VoxelWrite {
                    pos: VoxelPos {
                        x: target.x * 32 + i % 32,
                        y: target.y * 32 + 8 + i / 32,
                        z: target.z * 32 + 20,
                    },
                    block: if phase { stone } else { air },
                })
                .collect();
            black_box(
                world
                    .commit(EditTxn {
                        source: EditSource::Simulation,
                        expected: Vec::new(),
                        writes,
                    })
                    .unwrap(),
            );
        }),
    );
}

fn derived_workloads(world: &World, positions: &[ChunkPos]) {
    let registry = world.registry_handle();
    let token = CancelToken::new();
    let mesh_inputs: Vec<MeshingInput> = positions
        .iter()
        .map(|&pos| {
            let center = world.chunk(pos).unwrap();
            let neighbors = std::array::from_fn(|index| {
                let offset = HALO_OFFSETS[index];
                world.chunk(ChunkPos {
                    x: pos.x + i64::from(offset.dx),
                    y: pos.y + i64::from(offset.dy),
                    z: pos.z + i64::from(offset.dz),
                })
            });
            MeshingInput {
                stamp: MeshStamp {
                    center: center.revision,
                    halo: [None; 26],
                    registry_epoch: 1,
                    mesher_epoch: 1,
                },
                center,
                neighbors,
                registry: registry.clone(),
            }
        })
        .collect();
    report(
        "mesh_all_resident",
        measure(|| {
            for input in &mesh_inputs {
                black_box(build_mesh(input, &token).unwrap());
            }
        }),
    );
    let light_inputs: Vec<LightingInput> = positions
        .iter()
        .map(|&pos| {
            let center = world.chunk(pos).unwrap();
            let faces = [
                ChunkPos {
                    x: pos.x - 1,
                    ..pos
                },
                ChunkPos {
                    x: pos.x + 1,
                    ..pos
                },
                ChunkPos {
                    y: pos.y - 1,
                    ..pos
                },
                ChunkPos {
                    y: pos.y + 1,
                    ..pos
                },
                ChunkPos {
                    z: pos.z - 1,
                    ..pos
                },
                ChunkPos {
                    z: pos.z + 1,
                    ..pos
                },
            ];
            let neighbors = faces.map(|neighbor| world.chunk(neighbor));
            let stamp_faces = neighbors
                .each_ref()
                .map(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.revision));
            LightingInput {
                stamp: LightStamp {
                    center: center.revision,
                    faces: stamp_faces,
                    registry_epoch: 1,
                    lighting_epoch: 1,
                },
                center,
                neighbors,
                registry: registry.clone(),
                sky_from_above: true,
            }
        })
        .collect();
    report(
        "light_all_resident",
        measure(|| {
            for input in &light_inputs {
                black_box(build_light(input, LightingBudget::default(), &token).unwrap());
            }
        }),
    );
    report(
        "rebuild_all_resident",
        measure(|| {
            black_box(rebuild_bootstrap_chunks(world, positions, 2).unwrap());
        }),
    );
}
