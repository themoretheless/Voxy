//! Actual GPU broadphase regions compared with an independent CPU enumeration.
#[path = "support/native_backend.rs"]
mod native_backend;
use voxy_gpu::{VoxelClass, VoxelRegion, VoxelRegionProgram, VoxelRegionResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = native_backend::parse(std::env::args().skip(1))?;
    let instance = options.create_instance();
    let adapter = pollster::block_on(options.adapter(&instance))?;
    println!("Voxel regions GPU: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let program = pollster::block_on(VoxelRegionProgram::new(&device))?;
    check_world_snapshot(&program, &queue)?;
    check_sweeps(&program, &device, &queue)?;
    check_sweep_validation(&program, &queue)?;
    check_controller(&program, &queue)?;
    check_vehicle(&program, &queue)?;
    check_vehicle_recovery(&program, &queue)?;
    check_character_stale(&program, &queue)?;
    check_dense_sweeps(&program, &queue)?;
    check_far_world(&program, &device, &queue)?;
    let dimensions = [7, 9, 11];
    let cells: Vec<_> = (0..693)
        .map(|index| match index % 17 {
            0 => VoxelClass::Unknown,
            1 => VoxelClass::Unavailable,
            2..=8 => VoxelClass::Solid,
            _ => VoxelClass::Empty,
        })
        .collect();
    let regions: Vec<_> = (0..257_u32)
        .map(|index| {
            let min = [index % 7, (index * 3) % 9, (index * 5) % 11];
            let max =
                std::array::from_fn(|axis| min[axis] + index % (dimensions[axis] - min[axis]));
            VoxelRegion { min, max }
        })
        .collect();
    let results = program.classify(&queue, dimensions, &cells, &regions)?;
    check_pending(&program, &queue, dimensions, &cells, &regions, &results)?;
    for (region, actual) in regions.iter().zip(results) {
        let expected = reference(&cells, region)?;
        assert_eq!(actual, expected);
    }
    assert!(
        program
            .classify(&queue, [0, 9, 11], &cells, &regions)
            .is_err()
    );
    assert!(
        program
            .classify(&queue, dimensions, &cells[..692], &regions)
            .is_err()
    );
    assert!(
        program
            .classify(
                &queue,
                dimensions,
                &cells,
                &[VoxelRegion {
                    min: [0; 3],
                    max: [7, 8, 10]
                }]
            )
            .is_err()
    );
    let excessive = vec![
        VoxelRegion {
            min: [0; 3],
            max: [6, 8, 10]
        };
        16_384
    ];
    assert!(
        program
            .classify(&queue, dimensions, &cells, &excessive)
            .is_err()
    );
    assert_eq!(
        program.classify(
            &queue,
            [1; 3],
            &[VoxelClass::Empty],
            &[VoxelRegion {
                min: [0; 3],
                max: [0; 3]
            }]
        )?,
        [VoxelRegionResult {
            solid_count: 0,
            first_solid: None,
            first_fault: None
        }]
    );
    println!(
        "PASS: 257 parallel voxel regions, exact CPU counts/candidates/faults, invalid-input recovery"
    );
    Ok(())
}

fn reference(
    cells: &[VoxelClass],
    region: &VoxelRegion,
) -> Result<VoxelRegionResult, Box<dyn std::error::Error>> {
    let mut expected = VoxelRegionResult {
        solid_count: 0,
        first_solid: None,
        first_fault: None,
    };
    for (index, cell) in cells.iter().enumerate() {
        let index = u32::try_from(index)?;
        let position = [index / 99, (index / 11) % 9, index % 11];
        if (0..3).any(|axis| position[axis] < region.min[axis] || position[axis] > region.max[axis])
        {
            continue;
        }
        match cell {
            VoxelClass::Solid => {
                expected.solid_count += 1;
                expected.first_solid.get_or_insert(index);
            }
            VoxelClass::Unavailable | VoxelClass::Unknown => {
                expected.first_fault.get_or_insert(index);
            }
            VoxelClass::Empty => {}
        }
    }
    Ok(expected)
}

fn check_pending(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
    dimensions: [u32; 3],
    cells: &[VoxelClass],
    regions: &[VoxelRegion],
    expected: &[VoxelRegionResult],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut first = program.begin_classify(queue, dimensions, cells, regions)?;
    let mut second = program.begin_classify(queue, dimensions, cells, regions)?;
    // Dropping an in-flight readback must not poison other submissions.
    drop(program.begin_classify(queue, dimensions, cells, regions)?);
    for pending in [&mut first, &mut second] {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            program.poll_native()?;
            if let Some(actual) = pending.try_result()? {
                assert_eq!(actual, expected);
                break;
            }
            if std::time::Instant::now() >= deadline {
                return Err("voxel region readback timeout".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(matches!(
            pending.try_result(),
            Err(voxy_render::ComputeError::Consumed)
        ));
    }
    println!("PASS: concurrent nonblocking voxel regions, consumed and dropped readback recovery");
    Ok(())
}

fn check_world_snapshot(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_core::VoxelPos;
    use voxy_gpu::VoxelRegionSnapshot;
    use voxy_world::{
        BlockStateId, CollisionShape, EditSource, EditTxn, Sample, VoxelView, VoxelWrite,
    };
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let anchor = VoxelPos { x: 0, y: 0, z: 0 };
    let query = [VoxelRegion {
        min: [0; 3],
        max: [31; 3],
    }];
    let snapshot =
        VoxelRegionSnapshot::capture(&scene.world, scene.world.registry(), anchor, [32; 3])?;
    let mut expected = VoxelRegionResult {
        solid_count: 0,
        first_solid: None,
        first_fault: None,
    };
    for index in 0..32_768 {
        let pos = snapshot.position(index).ok_or("bad world index")?;
        let Sample::Loaded(block) = scene.world.sample(pos) else {
            return Err("fixture chunk missing".into());
        };
        if scene
            .world
            .registry()
            .get(block)
            .ok_or("fixture block unknown")?
            .collision
            == CollisionShape::FullCube
        {
            expected.solid_count += 1;
            expected.first_solid.get_or_insert(index);
        }
    }
    let first = expected.first_solid.ok_or("fixture has no solids")?;
    let removed = snapshot.position(first).ok_or("missing candidate")?;
    let mut pending = snapshot.begin_classify(program, queue, &query)?;
    scene.world.commit(EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes: vec![VoxelWrite {
            pos: removed,
            block: BlockStateId::AIR,
        }],
    })?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let actual = loop {
        program.poll_native()?;
        if let Some(result) = pending.try_result()? {
            break result;
        }
        if std::time::Instant::now() >= deadline {
            return Err("world classification timeout".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert_eq!(actual, [expected]);
    assert!(!snapshot.is_current(&scene.world));
    let fresh =
        VoxelRegionSnapshot::capture(&scene.world, scene.world.registry(), anchor, [32; 3])?;
    assert!(fresh.is_current(&scene.world));
    let mut fresh_pending = fresh.begin_classify(program, queue, &query)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        program.poll_native()?;
        if let Some(result) = fresh_pending.try_result()? {
            assert_eq!(result[0].solid_count, expected.solid_count - 1);
            assert_ne!(result[0].first_solid, Some(first));
            assert_eq!(result[0].first_fault, None);
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err("fresh classification timeout".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("PASS: frozen world GPU counts/candidates, stale edit rejection and fresh recovery");
    Ok(())
}

fn check_far_world(
    program: &VoxelRegionProgram,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_core::VoxelPos;
    use voxy_gpu::VoxelRegionSnapshot;
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let anchor = VoxelPos {
        x: 9_007_199_254_740_993,
        y: -5,
        z: -7,
    };
    let snapshot =
        VoxelRegionSnapshot::capture(&scene.world, scene.world.registry(), anchor, [2; 3])?;
    let query = [VoxelRegion {
        min: [1, 0, 1],
        max: [1; 3],
    }];
    let mut pending = snapshot.begin_classify(program, queue, &query)?;
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let result = pending.try_result()?.ok_or("far world result pending")?;
    assert_eq!(
        result,
        [VoxelRegionResult {
            solid_count: 0,
            first_solid: None,
            first_fault: Some(5)
        }]
    );
    assert_eq!(
        snapshot.position(result[0].first_fault.ok_or("missing fault")?),
        Some(VoxelPos {
            x: 9_007_199_254_740_994,
            y: -5,
            z: -6
        })
    );
    assert!(snapshot.is_current(&scene.world));
    println!("PASS: far integer world GPU fault candidate retains exact i64 coordinates");
    Ok(())
}

fn check_sweeps(
    program: &VoxelRegionProgram,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics_voxel::{AnchoredAabb, SweepConfig, sweep_aabb};
    use voxy_core::VoxelPos;
    use voxy_world::{BlockStateId, EditSource, EditTxn, VoxelWrite};
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let config = SweepConfig::default();
    for (anchor, displacement) in [
        (
            VoxelPos {
                x: 10,
                y: 25,
                z: 10,
            },
            [1.0, 0.0, 0.0],
        ),
        (
            VoxelPos {
                x: 10,
                y: 25,
                z: 10,
            },
            [0.0, -24.0, 0.0],
        ),
        (
            VoxelPos {
                x: 31,
                y: 25,
                z: 10,
            },
            [3.0, 0.0, 0.0],
        ),
        (
            VoxelPos {
                x: 9_007_199_254_740_993,
                y: 0,
                z: 0,
            },
            [1.0, 0.0, 0.0],
        ),
    ] {
        let aabb = AnchoredAabb {
            anchor,
            min: [0.1; 3],
            max: [0.9; 3],
        };
        let expected = sweep_aabb(
            &scene.world,
            scene.world.registry(),
            aabb,
            displacement,
            config,
        )?;
        let mut pending = program.begin_sweep(
            queue,
            &scene.world,
            scene.world.registry(),
            aabb,
            displacement,
            config,
        )?;
        device.poll(wgpu::PollType::wait_indefinitely())?;
        assert_eq!(
            pending.try_sweep(&scene.world)?.ok_or("sweep pending")?,
            expected
        );
        assert!(pending.try_sweep(&scene.world).is_err());
    }
    let aabb = AnchoredAabb {
        anchor: VoxelPos {
            x: 10,
            y: 25,
            z: 10,
        },
        min: [0.1; 3],
        max: [0.9; 3],
    };
    let mut pending = program.begin_sweep(
        queue,
        &scene.world,
        scene.world.registry(),
        aabb,
        [1.0, 0.0, 0.0],
        config,
    )?;
    scene.world.commit(EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes: vec![VoxelWrite {
            pos: VoxelPos { x: 0, y: 0, z: 0 },
            block: BlockStateId::AIR,
        }],
    })?;
    device.poll(wgpu::PollType::wait_indefinitely())?;
    assert!(matches!(
        pending.try_sweep(&scene.world),
        Err(voxy_gpu::GpuSweepError::StaleWorld)
    ));
    println!(
        "PASS: GPU broadphase exact sweep parity, far/unloaded boundaries and stale rejection"
    );
    Ok(())
}

fn check_sweep_validation(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics_voxel::{AnchoredAabb, SweepConfig, SweepError};
    use voxy_core::{ChunkPos, VoxelPos};
    use voxy_world::{BlockStateId, ChunkSnapshot, Sample, VoxelView};
    struct Unreadable;
    impl VoxelView for Unreadable {
        fn sample(&self, _: VoxelPos) -> Sample<BlockStateId> {
            panic!("invalid sweep sampled the world")
        }
        fn chunk(&self, _: ChunkPos) -> Option<ChunkSnapshot> {
            panic!("invalid sweep captured the world")
        }
    }
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let aabb = AnchoredAabb {
        anchor: VoxelPos { x: 0, y: 0, z: 0 },
        min: [0.1; 3],
        max: [0.9; 3],
    };
    assert!(matches!(
        program.begin_sweep(
            queue,
            &Unreadable,
            scene.world.registry(),
            aabb,
            [f64::NAN, 0.0, 0.0],
            SweepConfig::default()
        ),
        Err(voxy_gpu::GpuSweepError::Sweep(
            SweepError::InvalidDisplacement
        ))
    ));
    assert!(matches!(
        program.begin_sweep(
            queue,
            &Unreadable,
            scene.world.registry(),
            aabb,
            [0.0; 3],
            SweepConfig {
                max_candidate_voxels: 0
            }
        ),
        Err(voxy_gpu::GpuSweepError::Sweep(
            SweepError::InvalidCandidateBudget
        ))
    ));
    assert!(matches!(
        program.begin_sweep(
            queue,
            &Unreadable,
            scene.world.registry(),
            aabb,
            [10.0; 3],
            SweepConfig {
                max_candidate_voxels: 1
            }
        ),
        Err(voxy_gpu::GpuSweepError::Sweep(
            SweepError::CandidateBudgetExceeded { .. }
        ))
    ));
    let overflow = AnchoredAabb {
        anchor: VoxelPos {
            x: i64::MAX,
            y: 0,
            z: 0,
        },
        ..aabb
    };
    assert!(matches!(
        program.begin_sweep(
            queue,
            &Unreadable,
            scene.world.registry(),
            overflow,
            [1.0, 0.0, 0.0],
            SweepConfig::default()
        ),
        Err(voxy_gpu::GpuSweepError::Sweep(
            SweepError::CoordinateOverflow
        ))
    ));
    assert!(matches!(
        program.begin_sweep(
            queue,
            &Unreadable,
            scene.world.registry(),
            aabb,
            [512.0, 0.0, 0.0],
            SweepConfig::default()
        ),
        Err(voxy_gpu::GpuSweepError::Compute(
            voxy_render::ComputeError::InvalidBuffer
        ))
    ));
    println!("PASS: invalid GPU sweeps reject before world reads or dispatch");
    Ok(())
}

fn check_controller(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics::{AnchoredAabb, CharacterConfig, CharacterInput, CharacterState, Origin};
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let gpu = voxy_gpu::GpuVoxelCollisionWorld {
        view: &scene.world,
        registry: scene.world.registry(),
        program,
        queue,
    };
    let cpu = physics_voxel::VoxelCollisionWorld {
        view: &scene.world,
        registry: scene.world.registry(),
    };
    let initial = CharacterState {
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
    let mut actual = initial;
    let mut deferred = initial;
    let mut expected = initial;
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
        let gpu_report = physics::step_character(
            &gpu,
            &mut actual,
            input,
            1.0 / 60.0,
            CharacterConfig::default(),
        )?;
        let mut task = voxy_gpu::PendingGpuCharacter::new(
            deferred,
            input,
            1.0 / 60.0,
            CharacterConfig::default(),
            scene.world.registry(),
        );
        if tick % 2 == 0 {
            let mut velocity = deferred.velocity;
            velocity[0] = input.planar_velocity[0];
            velocity[2] = input.planar_velocity[1];
            if input.jump_pressed && deferred.grounded {
                velocity[1] = CharacterConfig::default().jump_speed;
            }
            velocity[1] = (velocity[1] + CharacterConfig::default().gravity * (1.0 / 60.0))
                .max(-CharacterConfig::default().terminal_fall_speed);
            task = task.with_motion(velocity, velocity.map(|v| v * (1.0 / 60.0)));
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let (next, report) = loop {
            program.poll_native()?;
            if let Some(result) = task.try_step(program, queue, &scene.world)? {
                break result;
            }
            if std::time::Instant::now() >= deadline {
                return Err("character task timeout".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        deferred = next;
        assert_eq!(deferred, expected, "deferred character tick {tick}");
        assert_eq!(report, cpu_report, "deferred contacts tick {tick}");
        assert!(task.try_step(program, queue, &scene.world).is_err());
        assert_eq!(actual, expected, "controller tick {tick}");
        assert_eq!(gpu_report, cpu_report, "contacts tick {tick}");
    }
    println!("PASS: 240 GPU-assisted character ticks, exact CPU states and contacts");
    println!(
        "PASS: 240 nonblocking character tasks, exact CPU states/contacts and consumed rejection"
    );
    Ok(())
}

fn check_character_stale(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics::{
        AnchoredAabb, CharacterConfig, CharacterError, CharacterInput, CharacterState, Origin,
    };
    use voxy_gpu::{CharacterGpuQueryError, GpuSweepError, PendingGpuCharacter};
    use voxy_world::{BlockStateId, EditSource, EditTxn, VoxelPos, VoxelWrite};
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    add_character_floor(&mut scene.world)?;
    let initial = CharacterState {
        body: AnchoredAabb {
            anchor: Origin {
                x: 10,
                y: 25,
                z: 10,
            },
            min: [0.1; 3],
            max: [0.9; 3],
        },
        velocity: [0.0, -120.0, 0.0],
        grounded: false,
    };
    let input = CharacterInput {
        planar_velocity: [5.0, 0.0],
        jump_pressed: false,
    };
    let mut stale = PendingGpuCharacter::new(
        initial,
        input,
        0.2,
        CharacterConfig::default(),
        scene.world.registry(),
    );
    assert!(stale.try_step(program, queue, &scene.world)?.is_none());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while stale.completed_sweeps() == 0 {
        program.poll_native()?;
        assert!(
            stale.try_step(program, queue, &scene.world)?.is_none(),
            "fixture must retain a second pending query"
        );
        if std::time::Instant::now() >= deadline {
            return Err("first character contact timeout".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(stale.completed_sweeps(), 1);
    scene.world.commit(EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes: vec![VoxelWrite {
            pos: VoxelPos { x: 0, y: 0, z: 0 },
            block: BlockStateId::AIR,
        }],
    })?;
    assert!(matches!(
        stale.try_step(program, queue, &scene.world),
        Err(CharacterError::Sweep(CharacterGpuQueryError::Gpu(
            GpuSweepError::StaleWorld
        )))
    ));
    assert!(matches!(
        stale.try_step(program, queue, &scene.world),
        Err(CharacterError::Sweep(CharacterGpuQueryError::Gpu(
            GpuSweepError::Compute(voxy_render::ComputeError::Consumed)
        )))
    ));
    drop(stale);
    let mut expected = initial;
    let cpu = physics_voxel::VoxelCollisionWorld {
        view: &scene.world,
        registry: scene.world.registry(),
    };
    let expected_report =
        physics::step_character(&cpu, &mut expected, input, 0.2, CharacterConfig::default())?;
    let mut fresh = PendingGpuCharacter::new(
        initial,
        input,
        0.2,
        CharacterConfig::default(),
        scene.world.registry(),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        program.poll_native()?;
        if let Some((state, report)) = fresh.try_step(program, queue, &scene.world)? {
            assert_eq!(state, expected);
            assert_eq!(report, expected_report);
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err("fresh character task timeout".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("PASS: pending character stale rejection, consumed errors and fresh recovery");
    Ok(())
}

fn add_character_floor(world: &mut voxy_world::World) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_world::{EditSource, EditTxn, VoxelPos, VoxelWrite};
    let stone = world
        .registry()
        .find(&voxy_world::ResourceKey::parse("voxy:stone")?)
        .ok_or("missing stone")?;
    world.commit(EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes: vec![
            VoxelWrite {
                pos: VoxelPos {
                    x: 10,
                    y: 20,
                    z: 10,
                },
                block: stone,
            },
            VoxelWrite {
                pos: VoxelPos {
                    x: 11,
                    y: 20,
                    z: 10,
                },
                block: stone,
            },
        ],
    })?;
    Ok(())
}

fn check_dense_sweeps(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics::{AnchoredAabb, CollisionWorld, Origin};
    use voxy_core::VoxelPos;
    use voxy_world::{BlockStateId, EditSource, EditTxn, VoxelWrite};
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
    let captured_only = LoadedChunkOnlyView(&scene.world);
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
        let gpu = voxy_gpu::GpuVoxelCollisionWorld {
            program,
            queue,
            view: &captured_only,
            registry: scene.world.registry(),
        };
        let actual = gpu.sweep_aabb(body, displacement, 16_384)?;
        assert_eq!(actual, expected);
        let Some(physics_voxel::SweepObstacle::Block { pos, .. }) = actual.obstacle else {
            return Err("dense fixture missed contact".into());
        };
        assert_eq!(pos.x, expected_x);
        assert_eq!((pos.y, pos.z), (0, 0));
    }
    println!("PASS: GPU 4913 obstacles, late nearest contact and canonical overlap tie");
    Ok(())
}

/// Dense fixture is fully loaded: resolving contacts must not resample the world.
struct LoadedChunkOnlyView<'a>(&'a voxy_world::World);
impl voxy_world::VoxelView for LoadedChunkOnlyView<'_> {
    fn sample(&self, _: voxy_core::VoxelPos) -> voxy_world::Sample<voxy_world::BlockStateId> {
        panic!("loaded GPU contacts must use captured chunks")
    }
    fn chunk(&self, pos: voxy_core::ChunkPos) -> Option<voxy_world::ChunkSnapshot> {
        voxy_world::VoxelView::chunk(self.0, pos)
    }
}

fn check_vehicle(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics_voxel::{AnchoredAabb, CharacterState, VehicleConfig, VehicleInput, VehicleState};
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let mut actual = VehicleState {
        chassis: CharacterState {
            body: AnchoredAabb {
                anchor: voxy_core::VoxelPos { x: 8, y: 20, z: 8 },
                min: [-0.6, 0.0, -0.8],
                max: [0.6, 1.2, 0.8],
            },
            velocity: [0.0; 3],
            grounded: false,
        },
        heading: 0.0,
        longitudinal_speed: 0.0,
    };
    let mut expected = actual;
    let config = VehicleConfig {
        max_forward_speed: 4.0,
        max_reverse_speed: 2.0,
        ..VehicleConfig::default()
    };
    let mut forward = false;
    let mut reverse = false;
    let mut braked = false;
    for tick in 0..240 {
        let input = VehicleInput {
            throttle: if (80..120).contains(&tick) {
                0.0
            } else if (120..180).contains(&tick) {
                -1.0
            } else {
                1.0
            },
            steering: if (40..180).contains(&tick) { 0.25 } else { 0.0 },
            brake: (80..120).contains(&tick),
        };
        let report = physics_voxel::step_vehicle(
            &scene.world,
            scene.world.registry(),
            &mut expected,
            input,
            1.0 / 60.0,
            config,
        )?;
        let mut pending = voxy_gpu::PendingGpuVehicle::new(
            actual,
            input,
            1.0 / 60.0,
            config,
            scene.world.registry(),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut integrations = 0;
        let (state, gpu_report) = loop {
            program.poll_native()?;
            if let Some(result) = pending.try_step_with_integrator(
                program,
                queue,
                &scene.world,
                |state, input, dt, config| {
                    integrations += 1;
                    if tick % 2 != 0 {
                        return Ok(None);
                    }
                    let mut velocity = state.velocity;
                    velocity[0] = input.planar_velocity[0];
                    velocity[2] = input.planar_velocity[1];
                    velocity[1] =
                        (velocity[1] + config.gravity * dt).max(-config.terminal_fall_speed);
                    Ok(Some((velocity, velocity.map(|v| v * dt))))
                },
            )? {
                break result;
            }
            if std::time::Instant::now() >= deadline {
                return Err("GPU vehicle timeout".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        assert_eq!(integrations, 1, "integrator repeated across pending polls");
        assert_eq!(state, expected, "vehicle state tick {tick}");
        assert_eq!(gpu_report, report, "vehicle contacts tick {tick}");
        assert!(matches!(
            pending.try_step(program, queue, &scene.world),
            Err(voxy_gpu::VehicleGpuError::Consumed)
        ));
        actual = state;
        forward |= state.longitudinal_speed > 0.1;
        reverse |= state.longitudinal_speed < -0.1;
        braked |= input.brake && state.longitudinal_speed == 0.0;
    }
    assert!(forward && reverse && braked);
    println!(
        "PASS: 240 nonblocking GPU vehicle ticks, exact CPU state/contacts, forward/reverse/brake and consumed rejection"
    );
    Ok(())
}

fn check_vehicle_recovery(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use physics_voxel::{AnchoredAabb, CharacterState, VehicleConfig, VehicleInput, VehicleState};
    use voxy_gpu::{CharacterGpuQueryError, GpuSweepError, PendingGpuVehicle, VehicleGpuError};
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    add_character_floor(&mut scene.world)?;
    let initial = VehicleState {
        chassis: CharacterState {
            body: AnchoredAabb {
                anchor: voxy_core::VoxelPos {
                    x: 10,
                    y: 25,
                    z: 10,
                },
                min: [0.1; 3],
                max: [0.9; 3],
            },
            velocity: [0.0, -120.0, 0.0],
            grounded: false,
        },
        heading: 0.0,
        longitudinal_speed: 0.0,
    };
    let input = VehicleInput {
        throttle: 1.0,
        steering: 0.25,
        brake: false,
    };
    let config = VehicleConfig::default();
    let mut failed = PendingGpuVehicle::new(initial, input, 0.2, config, scene.world.registry());
    assert!(matches!(
        failed.try_step_with_integrator(program, queue, &scene.world, |_, _, _, _| {
            Err("fixture integration failure".into())
        }),
        Err(VehicleGpuError::Integration(_))
    ));
    assert!(matches!(
        failed.try_step(program, queue, &scene.world),
        Err(VehicleGpuError::Consumed)
    ));
    let mut invalid = PendingGpuVehicle::new(initial, input, 0.2, config, scene.world.registry());
    assert!(matches!(
        invalid.try_step_with_integrator(program, queue, &scene.world, |_, _, _, _| {
            Ok(Some(([f64::NAN, 0.0, 0.0], [0.0; 3])))
        }),
        Err(VehicleGpuError::Character(_))
    ));
    assert!(matches!(
        invalid.try_step(program, queue, &scene.world),
        Err(VehicleGpuError::Consumed)
    ));
    let mut stale = PendingGpuVehicle::new(initial, input, 0.2, config, scene.world.registry());
    assert!(stale.try_step(program, queue, &scene.world)?.is_none());
    scene.world.commit(voxy_world::EditTxn {
        source: voxy_world::EditSource::Simulation,
        expected: vec![],
        writes: vec![voxy_world::VoxelWrite {
            pos: voxy_core::VoxelPos { x: 0, y: 0, z: 0 },
            block: voxy_world::BlockStateId::AIR,
        }],
    })?;
    assert!(matches!(
        stale.try_step(program, queue, &scene.world),
        Err(VehicleGpuError::Character(physics::CharacterError::Sweep(
            CharacterGpuQueryError::Gpu(GpuSweepError::StaleWorld)
        )))
    ));
    assert!(matches!(
        stale.try_step(program, queue, &scene.world),
        Err(VehicleGpuError::Consumed)
    ));
    drop(stale);
    let mut expected = initial;
    let expected_report = physics_voxel::step_vehicle(
        &scene.world,
        scene.world.registry(),
        &mut expected,
        input,
        0.2,
        config,
    )?;
    let mut fresh = PendingGpuVehicle::new(initial, input, 0.2, config, scene.world.registry());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        program.poll_native()?;
        if let Some((state, report)) = fresh.try_step(program, queue, &scene.world)? {
            assert_eq!(state, expected);
            assert_eq!(report, expected_report);
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err("GPU vehicle recovery timeout".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("PASS: GPU vehicle stale rejection, consumed error and exact fresh recovery");
    Ok(())
}
