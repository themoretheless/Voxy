use crate::{VoxelCollisionWorld, test_support::test_registry};
use physics::liquid::{Config, ContactConfig, Liquid, Material, Particle};
use voxy_world::{BlockStateId, ChunkPos, ChunkSnapshot, Sample, VoxelPos, VoxelView};

#[derive(Debug)]
struct Wall {
    stone: BlockStateId,
    unloaded: bool,
}
impl VoxelView for Wall {
    fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
        if pos.x == 0 {
            if self.unloaded {
                Sample::Unloaded {
                    chunk: ChunkPos::default(),
                }
            } else {
                Sample::Loaded(self.stone)
            }
        } else {
            Sample::Loaded(BlockStateId::AIR)
        }
    }
    fn chunk(&self, _: ChunkPos) -> Option<ChunkSnapshot> {
        None
    }
}
#[test]
fn continuous_liquid_stops_at_voxel_walls_and_unloaded_space() {
    let registry = test_registry();
    for unloaded in [false, true] {
        let wall = Wall {
            unloaded,
            stone: registry
                .find(&voxy_world::ResourceKey::parse("voxy:stone").unwrap())
                .unwrap(),
        };
        let mut liquid = Liquid::new(
            vec![Particle {
                position: [-0.5, 0.5, 0.5],
                velocity: [100.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            }],
            vec![Material::WATER],
            Config {
                smoothing_radius: 1.0,
                particle_radius: 0.05,
                gravity: [0.0; 3],
                ..Config::default()
            },
        )
        .unwrap();
        liquid
            .step_with_world(
                0.01,
                None,
                &VoxelCollisionWorld {
                    view: &wall,
                    registry: &registry,
                },
                ContactConfig::default(),
            )
            .unwrap();
        assert!((liquid.particles()[0].position[0] + 0.05).abs() < 1e-10);
        assert!(liquid.particles()[0].velocity[0].abs() < 1e-12);
    }
}
