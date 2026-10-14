//! Debris pieces against the voxel world: landing on the chunk's top face,
//! resting, sliding to a stop, and a closed energy ledger.
use std::collections::BTreeMap;
use std::sync::Arc;

use physics::debris::{DebrisBody, DebrisStepReport, DebrisWorldContact, step_debris_in_world};
use physics::plasticity::mesh::{DebrisPiece, convex_hull};
use physics_voxel::VoxelCollisionWorld;
use voxy_core::{ChunkPos, WorldEpoch};
use voxy_world::{
    BlockDef, BlockRegistry, ChunkData, CollisionShape, GeneratedChunk, MaterialId, Occlusion,
    PalettedBlocks, RenderKind, ResourceKey, World, WorldLimits,
};

fn stone_world() -> (World, Arc<BlockRegistry>) {
    let registry = Arc::new(
        BlockRegistry::new(vec![
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
        ])
        .unwrap(),
    );
    let stone = registry
        .find(&ResourceKey::parse("voxy:stone").unwrap())
        .unwrap();
    let air = registry
        .find(&ResourceKey::parse("voxy:air").unwrap())
        .unwrap();
    let mut world = World::new(
        WorldEpoch::new(1).unwrap(),
        registry.clone(),
        WorldLimits::default(),
    );
    // Stone floor chunk at y in [0, 32) and an air chunk above it.
    world
        .insert_generated(GeneratedChunk {
            pos: ChunkPos { x: 0, y: 0, z: 0 },
            data: ChunkData {
                blocks: PalettedBlocks::uniform(stone),
                block_data: BTreeMap::new(),
            },
        })
        .unwrap();
    world
        .insert_generated(GeneratedChunk {
            pos: ChunkPos { x: 0, y: 1, z: 0 },
            data: ChunkData {
                blocks: PalettedBlocks::uniform(air),
                block_data: BTreeMap::new(),
            },
        })
        .unwrap();
    (world, registry)
}

/// Unit cube piece of 1 kg centred at `center`, in voxel units.
fn cube(center: [f64; 3], velocity: [f64; 3]) -> DebrisPiece {
    let mut points = Vec::new();
    for sx in [-0.5, 0.5] {
        for sy in [-0.5, 0.5] {
            for sz in [-0.5, 0.5] {
                points.push([center[0] + sx, center[1] + sy, center[2] + sz]);
            }
        }
    }
    let (hull, hull_faces) = convex_hull(&points).unwrap();
    let inertia = 1. / 6.;
    DebrisPiece {
        nodes: Vec::new(),
        hull,
        hull_faces,
        mass_kg: 1.,
        inertia_kg_m2: [[inertia, 0., 0.], [0., inertia, 0.], [0., 0., inertia]],
        center_m: center,
        velocity_m_s: velocity,
        angular_velocity_rad_s: [0.; 3],
        kinetic_j: 0.5 * velocity.iter().map(|v| v * v).sum::<f64>(),
        deformation_kinetic_j: 0.,
    }
}

fn audit(l: &DebrisStepReport) -> f64 {
    l.kinetic_j + l.potential_j + l.dissipated_j - l.correction_j - l.integration_j
}

#[test]
fn falling_piece_lands_on_the_stone_surface_and_a_sliding_piece_stops() {
    let (world, registry) = stone_world();
    let collision = VoxelCollisionWorld {
        view: &world,
        registry: &registry,
    };
    let gravity = [0., -9.81, 0.];
    let mut bodies = vec![
        DebrisBody::new(cube([10., 36., 10.], [0.; 3])).unwrap(),
        DebrisBody::new(cube([20., 32.5, 10.], [3., 0., 0.])).unwrap(),
    ];
    let mut ledger = DebrisStepReport::default();
    let initial = bodies
        .iter()
        .map(|b| b.kinetic_j() + 9.81 * b.position_m[1])
        .sum::<f64>();
    let mut landed_at = None;
    for step in 0..2000 {
        step_debris_in_world(
            &mut bodies,
            1e-3,
            gravity,
            DebrisWorldContact::default(),
            &collision,
            &mut ledger,
        )
        .unwrap();
        let total = audit(&ledger);
        assert!(
            (total - initial).abs() < 1e-9 * initial,
            "ledger drift {}",
            total - initial
        );
        if landed_at.is_none() && ledger.contacts > 0 && bodies[0].velocity_m_s[1].abs() < 1e-9 {
            landed_at = Some(step);
        }
    }
    // The dropped cube rests on the chunk's top face (y = 32) with its base there.
    let lowest: f64 = bodies[0]
        .world_hull()
        .iter()
        .map(|p| p[1])
        .fold(f64::INFINITY, f64::min);
    assert!((lowest - 32.).abs() < 1e-6, "lowest {lowest}");
    assert!(bodies[0].velocity_m_s.iter().all(|v| v.abs() < 1e-9));
    assert!(landed_at.is_some());
    // The sliding cube decelerates at μ g and stops within 3/(0.6·9.81) ≈ 0.51 s.
    let slid = bodies[1].position_m[0] - 20.;
    let expected = 3.0_f64.powi(2) / (2. * 0.6 * 9.81);
    assert!(
        (slid - expected).abs() < 0.03 * expected,
        "{slid} vs {expected}"
    );
    assert!(bodies[1].velocity_m_s[0].abs() < 1e-6);
    assert!(ledger.dissipated_j > 0.5 * 9.81 * 3. + 4.);
    // A piece inside the stone is rejected rather than tunnelled.
    let mut buried = vec![DebrisBody::new(cube([10., 20., 10.], [0.; 3])).unwrap()];
    let mut scratch = DebrisStepReport::default();
    assert!(
        step_debris_in_world(
            &mut buried,
            1e-3,
            gravity,
            DebrisWorldContact::default(),
            &collision,
            &mut scratch
        )
        .is_err()
    );
}
