//! Isolated CPU probe of the per-tick voxel queries used by the physics adapter.
//!
//! Measures one water tick on a multi-chunk water body, character-sized AABB sweeps and
//! camera-length raycasts against a sloped stone world with scattered pillars. Every workload
//! folds its results into a checksum printed to stderr so before/after runs can be compared.
use std::collections::BTreeMap;
use std::sync::Arc;
use std::{hint::black_box, time::Instant};

use physics_voxel::{
    AnchoredAabb, RayOrigin, RaycastConfig, RaycastResult, SweepConfig, SweepObstacle, WaterBudget,
    WaterPlan, WaterStates, raycast, step_water, sweep_aabb,
};
use voxy_core::WorldEpoch;
use voxy_world::{
    BlockDef, BlockRegistry, BlockStateId, CHUNK_EDGE, CHUNK_VOLUME, ChunkData, ChunkPos,
    CollisionShape, EditSource, GeneratedChunk, InterfaceGroupId, MaterialId, Occlusion,
    PalettedBlocks, RenderKind, ResourceKey, VoxelPos, World, WorldLimits,
};

const CHUNKS_X: i64 = 4;
const CHUNKS_Y: i64 = 2;
const CHUNKS_Z: i64 = 4;
const SWEEP_CALLS: usize = 10_000;
const RAYCAST_CALLS: usize = 10_000;
const WATER_WARMUP_TICKS: usize = 6;

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

/// FNV-1a style fold so result streams can be compared across runs.
fn fold(checksum: u64, value: u64) -> u64 {
    (checksum ^ value).wrapping_mul(0x0100_0000_01b3)
}

#[allow(clippy::cast_sign_loss)]
fn fold_pos(checksum: u64, pos: VoxelPos) -> u64 {
    fold(
        fold(fold(checksum, pos.x as u64), pos.y as u64),
        pos.z as u64,
    )
}

fn registry() -> Arc<BlockRegistry> {
    let mut definitions = vec![
        BlockDef {
            key: ResourceKey::parse("voxy:air").unwrap(),
            render: RenderKind::Invisible,
            occlusion: Occlusion::None,
            collision: CollisionShape::Empty,
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 0,
        },
        BlockDef {
            key: ResourceKey::parse("voxy:stone").unwrap(),
            render: RenderKind::Opaque,
            occlusion: Occlusion::FullCube,
            collision: CollisionShape::FullCube,
            face_materials: [MaterialId(1); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 20,
        },
    ];
    for level in 1..=8 {
        definitions.push(BlockDef {
            key: ResourceKey::parse(format!("voxy:water_{level}")).unwrap(),
            render: RenderKind::Translucent,
            occlusion: Occlusion::None,
            collision: CollisionShape::Empty,
            face_materials: [MaterialId(2); 6],
            translucent_interface_group: Some(InterfaceGroupId(1)),
            emission: 0,
            blast_resistance: 0,
        });
    }
    Arc::new(BlockRegistry::new(definitions).unwrap())
}

fn state(registry: &BlockRegistry, name: &str) -> BlockStateId {
    registry
        .find(&ResourceKey::parse(format!("voxy:{name}")).unwrap())
        .unwrap()
}

/// Terrain surface height: a slope rising along +X.
fn terrain_height(x: i64, _z: i64) -> i64 {
    12 + x / 4
}

/// Scattered one-voxel pillars three blocks tall on the terrain surface.
fn pillar(x: i64, z: i64) -> bool {
    (x * 7 + z * 13) % 17 == 0
}

fn reservoir(x: i64, y: i64, z: i64) -> bool {
    let height = terrain_height(x, z);
    (88..112).contains(&x) && (52..76).contains(&z) && (height..height + 8).contains(&y)
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn build_world(registry: &Arc<BlockRegistry>, states: WaterStates) -> (World, Vec<VoxelPos>) {
    let stone = state(registry, "stone");
    let mut world = World::new(
        WorldEpoch::new(1).unwrap(),
        Arc::clone(registry),
        WorldLimits::default(),
    );
    let mut water = Vec::new();
    for cx in 0..CHUNKS_X {
        for cy in 0..CHUNKS_Y {
            for cz in 0..CHUNKS_Z {
                let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
                for ly in 0..CHUNK_EDGE {
                    for lz in 0..CHUNK_EDGE {
                        for lx in 0..CHUNK_EDGE {
                            let (x, y, z) = (
                                cx * CHUNK_EDGE + lx,
                                cy * CHUNK_EDGE + ly,
                                cz * CHUNK_EDGE + lz,
                            );
                            let height = terrain_height(x, z);
                            let index = (lx + CHUNK_EDGE * (lz + CHUNK_EDGE * ly)) as usize;
                            if y < height || (pillar(x, z) && y < height + 3) {
                                dense[index] = stone;
                            } else if reservoir(x, y, z) {
                                dense[index] = states.0[7];
                                water.push(VoxelPos { x, y, z });
                            }
                        }
                    }
                }
                world
                    .insert_generated(GeneratedChunk {
                        pos: ChunkPos {
                            x: cx,
                            y: cy,
                            z: cz,
                        },
                        data: ChunkData {
                            blocks: PalettedBlocks::from_dense(dense).unwrap(),
                            block_data: BTreeMap::new(),
                        },
                    })
                    .unwrap();
            }
        }
    }
    (world, water)
}

fn plan_checksum(plan: &WaterPlan) -> u64 {
    match plan {
        WaterPlan::Settled => fold(0, 1),
        WaterPlan::Transaction { edit, next_active } => {
            let mut checksum = fold(0, 2);
            for (chunk, revision) in &edit.expected {
                checksum = fold_pos(
                    checksum,
                    VoxelPos {
                        x: chunk.x,
                        y: chunk.y,
                        z: chunk.z,
                    },
                );
                checksum = fold(checksum, revision.get());
            }
            for write in &edit.writes {
                checksum = fold(fold_pos(checksum, write.pos), u64::from(write.block.get()));
            }
            for pos in next_active.iter() {
                checksum = fold_pos(checksum, *pos);
            }
            checksum
        }
    }
}

fn water_workload(
    world: &mut World,
    registry: &BlockRegistry,
    states: WaterStates,
    water: Vec<VoxelPos>,
) {
    let mut active = water;
    for _ in 0..WATER_WARMUP_TICKS {
        match step_water(
            world,
            registry,
            states,
            &active,
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap()
        {
            WaterPlan::Settled => break,
            WaterPlan::Transaction { edit, next_active } => {
                world.commit(edit).unwrap();
                active = next_active.into_vec();
            }
        }
    }
    let mut checksum = 0;
    let mut writes = 0;
    report(
        "water_tick",
        measure(|| {
            let plan = step_water(
                black_box(&*world),
                registry,
                states,
                black_box(&active),
                EditSource::Simulation,
                WaterBudget::default(),
            )
            .unwrap();
            if let WaterPlan::Transaction { edit, .. } = &plan {
                writes = edit.writes.len();
            }
            checksum = plan_checksum(&plan);
        }),
    );
    eprintln!(
        "water_tick active {} writes {writes} checksum {checksum:016x}",
        active.len()
    );
}

/// Deterministic LCG so workloads are reproducible across runs.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    #[allow(clippy::cast_possible_wrap)]
    fn range(&mut self, span: u64) -> i64 {
        (self.next() % span) as i64
    }

    #[allow(clippy::cast_precision_loss)]
    fn unit(&mut self) -> f64 {
        (self.next() % 10_007) as f64 / 10_007.0
    }
}

fn sweep_workload(world: &World, registry: &BlockRegistry) {
    let mut lcg = Lcg(0x5eed_0001);
    let queries: Vec<(AnchoredAabb, [f64; 3])> = (0..SWEEP_CALLS)
        .map(|_| {
            let x = 1 + lcg.range(126);
            let z = 1 + lcg.range(126);
            let y = terrain_height(x, z) + lcg.range(3);
            let aabb = AnchoredAabb {
                anchor: VoxelPos { x, y, z },
                min: [
                    0.2 + lcg.unit() * 0.1,
                    0.0 + lcg.unit() * 0.6,
                    0.2 + lcg.unit() * 0.1,
                ],
                max: [
                    0.8 + lcg.unit() * 0.1,
                    1.8 + lcg.unit() * 0.6,
                    0.8 + lcg.unit() * 0.1,
                ],
            };
            let displacement = [
                (lcg.unit() - 0.5) * 0.8,
                (lcg.unit() - 0.7) * 0.6,
                (lcg.unit() - 0.5) * 0.8,
            ];
            (aabb, displacement)
        })
        .collect();
    let mut checksum = 0;
    let mut hits = 0;
    report(
        "sweep_aabb_x10000",
        measure(|| {
            checksum = 0;
            hits = 0;
            for &(aabb, displacement) in &queries {
                let result = sweep_aabb(
                    black_box(world),
                    registry,
                    black_box(aabb),
                    black_box(displacement),
                    SweepConfig::default(),
                )
                .unwrap();
                checksum = fold(checksum, result.fraction.to_bits());
                for component in result.normal {
                    checksum = fold(
                        checksum,
                        u64::from(component.unsigned_abs()) + 2 * u64::from(component < 0),
                    );
                }
                match result.obstacle {
                    None => checksum = fold(checksum, 0),
                    Some(SweepObstacle::Block { pos, block }) => {
                        hits += 1;
                        checksum = fold(fold_pos(checksum, pos), u64::from(block.get()));
                    }
                    Some(SweepObstacle::Unloaded { at, .. }) => checksum = fold_pos(checksum, at),
                    Some(SweepObstacle::Unavailable { at, .. }) => {
                        checksum = fold(fold_pos(checksum, at), 7);
                    }
                }
            }
        }),
    );
    eprintln!("sweep_aabb_x10000 hits {hits} checksum {checksum:016x}");
}

fn raycast_workload(world: &World) {
    let mut lcg = Lcg(0x5eed_0002);
    let queries: Vec<(RayOrigin, [f64; 3])> = (0..RAYCAST_CALLS)
        .map(|_| {
            let x = lcg.range(128);
            let z = lcg.range(128);
            let y = terrain_height(x, z) + 4 + lcg.range(12);
            let origin = RayOrigin {
                voxel: VoxelPos { x, y, z },
                offset: [lcg.unit() * 0.999, lcg.unit() * 0.999, lcg.unit() * 0.999],
            };
            let direction = [
                lcg.unit() * 2.0 - 1.0,
                lcg.unit() * 1.2 - 0.9,
                lcg.unit() * 2.0 - 1.0,
            ];
            (origin, direction)
        })
        .collect();
    let config = RaycastConfig {
        max_distance: 96.0,
        max_steps: 128,
    };
    let mut checksum = 0;
    let mut hits = 0;
    report(
        "raycast_x10000",
        measure(|| {
            checksum = 0;
            hits = 0;
            for &(origin, direction) in &queries {
                let result = raycast(
                    black_box(world),
                    black_box(origin),
                    black_box(direction),
                    config,
                )
                .unwrap();
                match result {
                    RaycastResult::Hit(hit) => {
                        hits += 1;
                        checksum = fold_pos(checksum, hit.pos);
                        checksum = fold(checksum, u64::from(hit.block.get()));
                        checksum = fold(checksum, hit.distance.to_bits());
                        for component in hit.normal {
                            checksum = fold(
                                checksum,
                                u64::from(component.unsigned_abs()) + 2 * u64::from(component < 0),
                            );
                        }
                    }
                    RaycastResult::Miss => checksum = fold(checksum, 11),
                    RaycastResult::StepBudgetExhausted => checksum = fold(checksum, 13),
                    RaycastResult::Unloaded { at, .. } => {
                        checksum = fold(fold_pos(checksum, at), 17)
                    }
                    RaycastResult::Unavailable { at, .. } => {
                        checksum = fold(fold_pos(checksum, at), 19);
                    }
                }
            }
        }),
    );
    eprintln!("raycast_x10000 hits {hits} checksum {checksum:016x}");
}

fn main() {
    println!("workload,p50_ns,p95_ns,max_ns");
    let registry = registry();
    let states = WaterStates(std::array::from_fn(|index| {
        state(&registry, &format!("water_{}", index + 1))
    }));
    let (mut world, water) = build_world(&registry, states);
    eprintln!(
        "world chunks {} reservoir cells {}",
        CHUNKS_X * CHUNKS_Y * CHUNKS_Z,
        water.len()
    );
    water_workload(&mut world, &registry, states, water);
    sweep_workload(&world, &registry);
    raycast_workload(&world);
}
