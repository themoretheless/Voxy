//! Physical NVIDIA acceptance of CUDA classification from immutable world snapshots.
use voxy_core::VoxelPos;
use voxy_gpu::{VoxelRegion, VoxelRegionResult, VoxelRegionSnapshot};
use voxy_world::{
    BlockStateId, CollisionShape, EditSource, EditTxn, Sample, VoxelView, VoxelWrite,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--cpu-dense-fixture") {
        return check_dense_batches(None);
    }
    let ordinal = std::env::args()
        .nth(1)
        .map_or(Ok(0), |value| value.parse::<usize>())?;
    let compute = voxy_cuda::CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("CUDA world classification: {:?}", compute.capabilities()?);
    check_character(&compute)?;
    check_boundaries(&compute)?;
    check_dense_batches(Some(&compute))?;
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let query = [VoxelRegion {
        min: [0; 3],
        max: [31; 3],
    }];
    let anchor = VoxelPos { x: 0, y: 0, z: 0 };
    let snapshot =
        VoxelRegionSnapshot::capture(&scene.world, scene.world.registry(), anchor, [32; 3])?;
    let mut expected = VoxelRegionResult {
        solid_count: 0,
        first_solid: None,
        first_fault: None,
    };
    for index in 0..32_768 {
        let pos = snapshot.position(index).ok_or("invalid world index")?;
        let Sample::Loaded(block) = scene.world.sample(pos) else {
            return Err("missing fixture voxel".into());
        };
        if scene
            .world
            .registry()
            .get(block)
            .ok_or("unknown fixture block")?
            .collision
            == CollisionShape::FullCube
        {
            expected.solid_count += 1;
            expected.first_solid.get_or_insert(index);
        }
    }
    assert_eq!(snapshot.classify_cuda(&compute, &query)?, [expected]);
    let first = expected.first_solid.ok_or("fixture empty")?;
    scene.world.commit(EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes: vec![VoxelWrite {
            pos: snapshot.position(first).ok_or("bad candidate")?,
            block: BlockStateId::AIR,
        }],
    })?;
    assert!(!snapshot.is_current(&scene.world));
    assert_eq!(snapshot.classify_cuda(&compute, &query)?, [expected]);
    let fresh =
        VoxelRegionSnapshot::capture(&scene.world, scene.world.registry(), anchor, [32; 3])?;
    let result = fresh.classify_cuda(&compute, &query)?;
    assert_eq!(result[0].solid_count, expected.solid_count - 1);
    assert_ne!(result[0].first_solid, Some(first));
    let far = VoxelRegionSnapshot::capture(
        &scene.world,
        scene.world.registry(),
        VoxelPos {
            x: 9_007_199_254_740_993,
            y: -5,
            z: -7,
        },
        [2; 3],
    )?;
    let result = far.classify_cuda(
        &compute,
        &[VoxelRegion {
            min: [1, 0, 1],
            max: [1; 3],
        }],
    )?;
    assert_eq!(
        result,
        [VoxelRegionResult {
            solid_count: 0,
            first_solid: None,
            first_fault: Some(5)
        }]
    );
    assert_eq!(
        far.position(5),
        Some(VoxelPos {
            x: 9_007_199_254_740_994,
            y: -5,
            z: -6
        })
    );
    println!(
        "PASS: CUDA world snapshot exact CPU classification, stale/fresh recovery and far anchors"
    );
    Ok(())
}

fn check_character(compute: &voxy_cuda::CudaCompute) -> Result<(), Box<dyn std::error::Error>> {
    use physics::{AnchoredAabb, CharacterConfig, CharacterInput, CharacterState, Origin};
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let cuda = voxy_gpu::CudaVoxelCollisionWorld {
        compute,
        view: &scene.world,
        registry: scene.world.registry(),
    };
    let cpu = physics_voxel::VoxelCollisionWorld {
        view: &scene.world,
        registry: scene.world.registry(),
    };
    let mut actual = CharacterState {
        body: AnchoredAabb {
            anchor: Origin {
                x: 10,
                y: 25,
                z: 10,
            },
            min: [0.1; 3],
            max: [0.9; 3],
        },
        velocity: [0.0; 3],
        grounded: false,
    };
    let mut expected = actual;
    for tick in 0..240 {
        let input = CharacterInput {
            planar_velocity: [if tick < 120 { 0.5 } else { -0.5 }, 0.0],
            jump_pressed: tick == 180,
        };
        let cpu_report = physics::step_character(
            &cpu,
            &mut expected,
            input,
            1.0 / 60.0,
            CharacterConfig::default(),
        )?;
        let cuda_report = if tick % 2 == 0 {
            let mut velocity = actual.velocity;
            velocity[0] = input.planar_velocity[0];
            velocity[2] = input.planar_velocity[1];
            if input.jump_pressed && actual.grounded {
                velocity[1] = CharacterConfig::default().jump_speed;
            }
            let mut motion = compute
                .euler_motion(
                    &[voxy_cuda::CudaProjectileInput {
                        velocity,
                        acceleration: [0.0, CharacterConfig::default().gravity, 0.0],
                    }],
                    1.0 / 60.0,
                )?
                .into_iter()
                .next()
                .ok_or("missing CUDA motion")?;
            motion.velocity[1] =
                motion.velocity[1].max(-CharacterConfig::default().terminal_fall_speed);
            motion.displacement[1] = motion.velocity[1] * (1.0 / 60.0);
            physics::step_character_with_motion(
                &cuda,
                &mut actual,
                input,
                1.0 / 60.0,
                CharacterConfig::default(),
                motion.velocity,
                motion.displacement,
            )?
        } else {
            physics::step_character(
                &cuda,
                &mut actual,
                input,
                1.0 / 60.0,
                CharacterConfig::default(),
            )?
        };
        assert_eq!(actual, expected, "CUDA character tick {tick}");
        assert_eq!(cuda_report, cpu_report, "CUDA contacts tick {tick}");
    }
    println!("PASS: CUDA broadphase 240 character ticks exact CPU state/contact parity");
    println!("PASS: combined CUDA motion and CUDA collision broadphase exact CPU parity");
    Ok(())
}

fn check_boundaries(compute: &voxy_cuda::CudaCompute) -> Result<(), Box<dyn std::error::Error>> {
    use physics::{AnchoredAabb, CollisionWorld, Origin};
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let cuda = voxy_gpu::CudaVoxelCollisionWorld {
        compute,
        view: &scene.world,
        registry: scene.world.registry(),
    };
    let cpu = physics_voxel::VoxelCollisionWorld {
        view: &scene.world,
        registry: scene.world.registry(),
    };
    for (anchor, displacement) in [
        (
            Origin {
                x: 10,
                y: 25,
                z: 10,
            },
            [1.0, 0.0, 0.0],
        ),
        (
            Origin {
                x: 10,
                y: 25,
                z: 10,
            },
            [0.0, -24.0, 0.0],
        ),
        (
            Origin {
                x: 31,
                y: 25,
                z: 10,
            },
            [3.0, 0.0, 0.0],
        ),
        (
            Origin {
                x: 9_007_199_254_740_993,
                y: 0,
                z: 0,
            },
            [1.0, 0.0, 0.0],
        ),
    ] {
        let body = AnchoredAabb {
            anchor,
            min: [0.1; 3],
            max: [0.9; 3],
        };
        assert_eq!(
            cuda.sweep_aabb(body, displacement, 16_384)?,
            cpu.sweep_aabb(body, displacement, 16_384)?
        );
    }
    println!("PASS: CUDA exact sweeps including far anchors and unloaded boundaries");
    Ok(())
}

fn check_dense_batches(
    compute: Option<&voxy_cuda::CudaCompute>,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics::{AnchoredAabb, CollisionWorld, Origin};
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let stone = scene
        .world
        .registry()
        .find(&voxy_world::ResourceKey::parse("voxy:stone")?)
        .ok_or("missing stone")?;
    let mut writes = Vec::new();
    for x in 0..18 {
        for y in 0..17 {
            for z in 0..17 {
                writes.push(VoxelWrite {
                    pos: VoxelPos { x, y, z },
                    block: if x == 17 { BlockStateId::AIR } else { stone },
                });
            }
        }
    }
    scene.world.commit(EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes,
    })?;
    let cpu = physics_voxel::VoxelCollisionWorld {
        view: &scene.world,
        registry: scene.world.registry(),
    };
    for (min, max, displacement, expected_x) in [
        ([17.1, 0.1, 0.1], [17.9, 16.9, 16.9], [-17.0, 0.0, 0.0], 16),
        ([0.1; 3], [16.9; 3], [0.25; 3], 0),
    ] {
        let body = AnchoredAabb {
            anchor: Origin::default(),
            min,
            max,
        };
        let expected = cpu.sweep_aabb(body, displacement, 16_384)?;
        let actual = if let Some(compute) = compute {
            let cuda = voxy_gpu::CudaVoxelCollisionWorld {
                compute,
                view: &scene.world,
                registry: scene.world.registry(),
            };
            cuda.sweep_aabb(body, displacement, 16_384)?
        } else {
            expected
        };
        assert_eq!(actual, expected);
        let Some(physics_voxel::SweepObstacle::Block { pos, .. }) = actual.obstacle else {
            return Err("dense fixture missed contact".into());
        };
        assert_eq!(pos.x, expected_x);
        assert_eq!((pos.y, pos.z), (0, 0));
    }
    if compute.is_some() {
        println!(
            "PASS: CUDA 4913 obstacles across batches, late nearest contact and canonical overlap tie"
        );
    } else {
        println!(
            "PASS: CPU dense fixture, 4913 obstacles with late nearest contact and canonical overlap tie"
        );
    }
    Ok(())
}
