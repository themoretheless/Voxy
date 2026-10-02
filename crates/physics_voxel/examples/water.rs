use physics_voxel::{WaterBudget, WaterPlan, WaterStates, step_water};
use std::collections::BTreeMap;
use std::sync::Arc;
use voxy_core::WorldEpoch;
use voxy_world::{
    BlockDef, BlockRegistry, BlockStateId, ChunkData, ChunkPos, CollisionShape, EditSource,
    EditTxn, GeneratedChunk, InterfaceGroupId, MaterialId, Occlusion, PalettedBlocks, RenderKind,
    ResourceKey, Sample, VoxelPos, VoxelView, VoxelWrite, World, WorldLimits,
};
fn fixture() -> (World, Arc<BlockRegistry>, WaterStates, BlockStateId) {
    let mut definitions = vec![BlockDef {
        key: ResourceKey::parse("voxy:air").unwrap(),
        render: RenderKind::Invisible,
        occlusion: Occlusion::None,
        collision: CollisionShape::Empty,
        face_materials: [MaterialId(0); 6],
        translucent_interface_group: None,
        emission: 0,
        blast_resistance: 0,
    }];
    definitions.push(BlockDef {
        key: ResourceKey::parse("voxy:stone").unwrap(),
        render: RenderKind::Opaque,
        occlusion: Occlusion::FullCube,
        collision: CollisionShape::FullCube,
        face_materials: [MaterialId(1); 6],
        translucent_interface_group: None,
        emission: 0,
        blast_resistance: 20,
    });
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
    let registry = Arc::new(BlockRegistry::new(definitions).unwrap());
    let states = WaterStates(std::array::from_fn(|index| {
        registry
            .find(&ResourceKey::parse(format!("voxy:water_{}", index + 1)).unwrap())
            .unwrap()
    }));
    let stone = registry
        .find(&ResourceKey::parse("voxy:stone").unwrap())
        .unwrap();
    let mut world = World::new(
        WorldEpoch::new(1).unwrap(),
        Arc::clone(&registry),
        WorldLimits::default(),
    );
    world
        .insert_generated(GeneratedChunk {
            pos: ChunkPos::default(),
            data: ChunkData {
                blocks: PalettedBlocks::uniform(BlockStateId::AIR),
                block_data: BTreeMap::new(),
            },
        })
        .unwrap();
    (world, registry, states, stone)
}

fn main() {
    let (mut world, registry, states, stone) = fixture();
    let mut writes = Vec::new();
    for x in 4..=16 {
        for z in 4..=16 {
            for y in 4..=14 {
                if y == 4 || x == 4 || x == 16 || z == 4 || z == 16 {
                    writes.push(VoxelWrite {
                        pos: VoxelPos { x, y, z },
                        block: stone,
                    });
                }
            }
        }
    }
    let mut active = Vec::new();
    for x in 7..=10 {
        for y in 9..=12 {
            for z in 7..=10 {
                let pos = VoxelPos { x, y, z };
                active.push(pos);
                writes.push(VoxelWrite {
                    pos,
                    block: states.0[7],
                });
            }
        }
    }
    world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![],
            writes,
        })
        .unwrap();
    for tick in 0..500 {
        match step_water(
            &world,
            &registry,
            states,
            &active,
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap()
        {
            WaterPlan::Settled => {
                println!("Settled at tick {tick}: 512 units");
                break;
            }
            WaterPlan::Transaction { edit, next_active } => {
                world.commit(edit).unwrap();
                active = next_active.into_vec();
            }
        }
        let mut volume = 0;
        for x in 5..16 {
            for y in 5..15 {
                for z in 5..16 {
                    if let Sample::Loaded(block) = world.sample(VoxelPos { x, y, z }) {
                        volume += states
                            .0
                            .iter()
                            .position(|&s| s == block)
                            .map_or(0, |i| i + 1);
                    }
                }
            }
        }
        assert_eq!(volume, 512, "volume changed at tick {tick}");
        if tick % 20 == 0 {
            println!("tick={tick} volume={volume} active={}", active.len());
        }
    }
}
