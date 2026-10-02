//! Actual integer GPU transfers compared to the existing CPU world transaction.
#[path = "support/native_backend.rs"]
mod native_backend;
use physics_voxel::{WaterBudget, WaterPlan, WaterStates};
use std::collections::BTreeSet;
use voxy_gpu::{WaterComputeError, WaterNode, WaterTransferProgram, WaterWorldSnapshot};
use voxy_world::{BlockStateId, EditSource, ResourceKey, Sample, VoxelPos, VoxelView};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = native_backend::parse(std::env::args().skip(1))?;
    let instance = options.create_instance();
    let adapter = pollster::block_on(options.adapter(&instance))?;
    println!("Water GPU: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let program = pollster::block_on(WaterTransferProgram::new(&device))?;
    verify_failures(&program, &queue)?;
    verify_pending(&program, &device, &queue)?;
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let mut levels = [BlockStateId::AIR; 8];
    for (i, level) in levels.iter_mut().enumerate() {
        *level = scene
            .world
            .registry()
            .find(&ResourceKey::parse(format!("voxy:water_{}", i + 1))?)
            .ok_or("missing water level")?;
    }
    let states = WaterStates(levels);
    verify_pending_world(&program, &device, &queue, states)?;
    verify_unavailable_world(&program, &queue, &scene.world, states)?;
    verify_loading_recovery(&program, &queue, &scene.world, states)?;
    let positions: Vec<_> = (0..32)
        .flat_map(|x| (0..32).flat_map(move |y| (0..32).map(move |z| VoxelPos { x, y, z })))
        .collect();
    let mut active = Vec::new();
    for &pos in &positions {
        if let Sample::Loaded(block) = scene.world.sample(pos)
            && amount(block, &levels).is_some_and(|n| n > 0)
        {
            active.push(pos);
        }
    }
    let mut changed_ticks = 0;
    for _ in 0..16 {
        let snapshot =
            WaterWorldSnapshot::capture(&scene.world, scene.world.registry(), states, &positions)?;
        let active_indices = snapshot.active_indices(&active)?;
        let result = program.step_detailed(
            &queue,
            snapshot.nodes(),
            &active_indices,
            8,
            4,
            131_072,
            65_536,
        )?;
        let (plan, cpu_reads) = cpu_plan(&scene.world, states, &active)?;
        let gpu_reads: BTreeSet<_> = positions
            .iter()
            .zip(&result.sampled)
            .filter_map(|(&pos, &read)| read.then_some(pos))
            .collect();
        if gpu_reads != cpu_reads {
            return Err("GPU/CPU exact water read sets differ".into());
        }
        let gpu_plan = snapshot.plan(&result, EditSource::Simulation, WaterBudget::default())?;
        if !active.is_empty() {
            verify_sparse(&program, &queue, &scene.world, states, &active, &gpu_plan)?;
        }
        if gpu_plan != plan {
            return Err("GPU/CPU water transactions differ".into());
        }
        match gpu_plan {
            WaterPlan::Settled => active.clear(),
            WaterPlan::Transaction { edit, next_active } => {
                verify_provenance(&positions, &result.sampled, &edit)?;
                scene.world.commit(edit)?;
                active = next_active.into_vec();
                changed_ticks += 1;
            }
        }
        for (&pos, actual) in positions.iter().zip(result.amounts) {
            let Sample::Loaded(block) = scene.world.sample(pos) else {
                return Err("CPU fixture became unavailable".into());
            };
            if actual != amount(block, &levels) {
                return Err(format!(
                    "water GPU/CPU mismatch at {pos:?}: GPU {actual:?}, CPU {:?}",
                    amount(block, &levels)
                )
                .into());
            }
        }
    }
    if changed_ticks == 0 {
        return Err("fixture never moved water".into());
    }
    println!(
        "PASS: 16 GPU water ticks, 524288 exact CPU cell comparisons, conserved volume and canonical transfers"
    );
    Ok(())
}

fn amount(block: BlockStateId, levels: &[BlockStateId; 8]) -> Option<u8> {
    if block == BlockStateId::AIR {
        Some(0)
    } else {
        levels
            .iter()
            .position(|&state| state == block)
            .and_then(|i| u8::try_from(i + 1).ok())
    }
}

fn verify_failures(
    program: &WaterTransferProgram,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let empty = WaterNode {
        amount: Some(0),
        neighbors: [None; 5],
    };
    assert_eq!(program.step(queue, &[empty], &[0], 8, 4, 1, 1)?, [Some(0)]);
    let detail = program.step_detailed(queue, &[empty, empty], &[0], 8, 4, 1, 1)?;
    assert_eq!(detail.sampled, [true, false]);
    assert_eq!(detail.amounts, [Some(0), Some(0)]);
    verify_lazy_errors(program, queue, empty)?;
    verify_unknown_errors(program, queue, empty)?;
    let source = WaterNode {
        amount: Some(8),
        neighbors: [None; 5],
    };
    assert!(matches!(
        program.step(queue, &[source], &[0], 8, 4, 2, 2),
        Err(WaterComputeError::MissingNeighbor)
    ));
    // Read budget takes precedence over an unavailable required neighbor.
    assert!(matches!(
        program.step(queue, &[source], &[0], 8, 4, 1, 2),
        Err(WaterComputeError::SampleBudget)
    ));
    let source = WaterNode {
        neighbors: [Some(1), None, None, None, None],
        ..source
    };
    assert!(matches!(
        program.step(queue, &[source, empty], &[0], 8, 4, 1, 2),
        Err(WaterComputeError::SampleBudget)
    ));
    assert!(matches!(
        program.step(queue, &[source, empty], &[0], 8, 4, 2, 1),
        Err(WaterComputeError::WriteBudget)
    ));
    assert_eq!(
        program.step(queue, &[source, empty], &[0], 8, 4, 2, 2)?,
        [Some(0), Some(8)]
    );
    assert!(matches!(
        program.step(queue, &[source, empty], &[1, 0], 8, 4, 2, 2),
        Err(WaterComputeError::InvalidInput)
    ));
    println!(
        "PASS: GPU water lazy reads, missing-neighbor/sample/write errors, independent recovery"
    );
    Ok(())
}

fn verify_provenance(
    positions: &[VoxelPos],
    sampled: &[bool],
    edit: &voxy_world::EditTxn,
) -> Result<(), Box<dyn std::error::Error>> {
    let gpu_chunks: BTreeSet<_> = positions
        .iter()
        .zip(sampled)
        .filter_map(|(&pos, &read)| read.then_some(voxy_core::split_voxel(pos).0))
        .collect();
    let cpu_chunks: BTreeSet<_> = edit.expected.iter().map(|&(pos, _)| pos).collect();
    if gpu_chunks != cpu_chunks {
        return Err("GPU water read provenance differs from CPU transaction".into());
    }
    Ok(())
}

struct ReadTrace<'a, V> {
    world: &'a V,
    reads: std::cell::RefCell<BTreeSet<VoxelPos>>,
}
impl<V: VoxelView> VoxelView for ReadTrace<'_, V> {
    fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
        self.reads.borrow_mut().insert(pos);
        self.world.sample(pos)
    }
    fn chunk(&self, pos: voxy_world::ChunkPos) -> Option<voxy_world::ChunkSnapshot> {
        self.world.chunk(pos)
    }
}
fn cpu_plan(
    world: &voxy_world::World,
    states: WaterStates,
    active: &[VoxelPos],
) -> Result<(WaterPlan, BTreeSet<VoxelPos>), Box<dyn std::error::Error>> {
    let trace = ReadTrace {
        world,
        reads: std::cell::RefCell::default(),
    };
    let plan = physics_voxel::step_water(
        &trace,
        world.registry(),
        states,
        active,
        EditSource::Simulation,
        WaterBudget::default(),
    )?;
    Ok((plan, trace.reads.into_inner()))
}

fn verify_sparse(
    program: &WaterTransferProgram,
    queue: &wgpu::Queue,
    world: &voxy_world::World,
    states: WaterStates,
    active: &[VoxelPos],
    expected: &WaterPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    let budget = WaterBudget::default();
    assert_eq!(
        program.plan_world(
            queue,
            world,
            world.registry(),
            states,
            &[],
            EditSource::Simulation,
            budget
        )?,
        WaterPlan::Settled
    );
    if program.plan_world(
        queue,
        world,
        world.registry(),
        states,
        active,
        EditSource::Simulation,
        budget,
    )? != *expected
    {
        return Err("world GPU adapter differs from full GPU/CPU transaction".into());
    }
    let snapshot =
        WaterWorldSnapshot::capture_active(world, world.registry(), states, active, budget)?;
    let indices = snapshot.active_indices(active)?;
    let result = program.step_detailed(queue, snapshot.nodes(), &indices, 8, 4, 131_072, 65_536)?;
    if snapshot.plan(&result, EditSource::Simulation, budget)? != *expected {
        return Err("sparse GPU graph differs from full GPU/CPU transaction".into());
    }
    Ok(())
}

fn verify_loading_recovery(
    program: &WaterTransferProgram,
    queue: &wgpu::Queue,
    original: &voxy_world::World,
    states: WaterStates,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = original.clone();
    let source = VoxelPos {
        x: 63,
        y: 17,
        z: 16,
    };
    let missing = VoxelPos { x: 64, ..source };
    let stone = world
        .registry()
        .find(&ResourceKey::parse("voxy:stone")?)
        .ok_or("stone missing")?;
    world.commit(voxy_world::EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes: vec![
            voxy_world::VoxelWrite {
                pos: source,
                block: states.0[7],
            },
            voxy_world::VoxelWrite {
                pos: VoxelPos { y: 16, ..source },
                block: stone,
            },
            voxy_world::VoxelWrite {
                pos: VoxelPos { x: 62, ..source },
                block: BlockStateId::AIR,
            },
        ],
    })?;
    let revision = world
        .chunk(voxy_core::split_voxel(source).0)
        .ok_or("source missing")?
        .revision;
    assert!(
        matches!(program.plan_world(queue, &world, world.registry(), states, &[source], EditSource::Simulation, WaterBudget::default()),
        Err(voxy_gpu::WaterSnapshotError::MissingSample(pos)) if pos == missing)
    );
    assert_eq!(world.sample(source), Sample::Loaded(states.0[7]));
    assert_eq!(
        world.sample(VoxelPos { x: 62, ..source }),
        Sample::Loaded(BlockStateId::AIR)
    );
    assert_eq!(
        world
            .chunk(voxy_core::split_voxel(source).0)
            .ok_or("source missing")?
            .revision,
        revision
    );
    world.insert_generated(voxy_world::GeneratedChunk {
        pos: voxy_core::split_voxel(missing).0,
        data: voxy_world::ChunkData::uniform(BlockStateId::AIR),
    })?;
    let (expected, _) = cpu_plan(&world, states, &[source])?;
    let plan = program.plan_world(
        queue,
        &world,
        world.registry(),
        states,
        &[source],
        EditSource::Simulation,
        WaterBudget::default(),
    )?;
    assert_eq!(plan, expected);
    let WaterPlan::Transaction { edit, .. } = plan else {
        return Err("loaded retry unexpectedly settled".into());
    };
    world.commit(edit)?;
    assert_eq!(world.sample(missing), Sample::Loaded(states.0[0]));
    println!(
        "PASS: missing chunk load and fresh GPU retry match CPU without partial failed transfers"
    );
    Ok(())
}

fn verify_missing_sample(
    program: &WaterTransferProgram,
    queue: &wgpu::Queue,
    world: &voxy_world::World,
    states: WaterStates,
    position: VoxelPos,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = program.begin_plan_world(
        queue,
        world,
        world.registry(),
        states,
        &[position],
        EditSource::Simulation,
        WaterBudget::default(),
    )?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        program.poll_native()?;
        match pending.try_plan() {
            Err(voxy_gpu::WaterSnapshotError::MissingSample(pos)) => {
                assert_eq!(pos, position);
                break;
            }
            Ok(None) if std::time::Instant::now() < deadline => std::thread::yield_now(),
            result => return Err(format!("missing-cell provenance mismatch: {result:?}").into()),
        }
    }
    assert!(pending.try_plan().is_err());
    println!("PASS: GPU missing sample identifies exact world position and consumes failed plan");
    Ok(())
}

fn verify_unavailable_world(
    program: &WaterTransferProgram,
    queue: &wgpu::Queue,
    world: &voxy_world::World,
    states: WaterStates,
) -> Result<(), Box<dyn std::error::Error>> {
    let positions = [
        VoxelPos {
            x: 16,
            y: 17,
            z: 16,
        },
        VoxelPos {
            x: 64,
            y: 17,
            z: 16,
        },
    ];
    let snapshot =
        WaterWorldSnapshot::capture_available(world, world.registry(), states, &positions)?;
    assert_eq!(snapshot.availability(), [true, false]);
    let result = program.step_detailed_available(
        queue,
        snapshot.nodes(),
        &[0],
        snapshot.availability(),
        8,
        4,
        1,
        1,
    )?;
    assert_eq!(
        snapshot.plan(&result, EditSource::Simulation, WaterBudget::default())?,
        WaterPlan::Settled
    );
    assert!(matches!(
        program.step_detailed_available(
            queue,
            snapshot.nodes(),
            &[1],
            snapshot.availability(),
            8,
            4,
            1,
            1
        ),
        Err(WaterComputeError::UnavailableNode(1))
    ));
    verify_missing_sample(program, queue, world, states, positions[1])?;
    let sparse = WaterWorldSnapshot::capture_active_available(
        world,
        world.registry(),
        states,
        &positions,
        WaterBudget::default(),
    )?;
    let loaded_active = sparse.active_indices(&positions[..1])?;
    let sparse_result = program.step_detailed_available(
        queue,
        sparse.nodes(),
        &loaded_active,
        sparse.availability(),
        8,
        4,
        1,
        1,
    )?;
    assert_eq!(
        sparse.plan(
            &sparse_result,
            EditSource::Simulation,
            WaterBudget::default()
        )?,
        WaterPlan::Settled
    );
    let forged = voxy_gpu::WaterTransferResult {
        sampled: vec![false, true],
        ..result
    };
    assert!(
        snapshot
            .plan(&forged, EditSource::Simulation, WaterBudget::default())
            .is_err()
    );
    println!(
        "PASS: captured unavailable world chunks remain lazy and cannot enter successful transactions"
    );
    Ok(())
}

fn verify_lazy_errors(
    program: &WaterTransferProgram,
    queue: &wgpu::Queue,
    empty: WaterNode,
) -> Result<(), Box<dyn std::error::Error>> {
    let overflow_empty = WaterNode {
        neighbors: [Some(u32::MAX - 1); 5],
        ..empty
    };
    assert_eq!(
        program.step(queue, &[overflow_empty], &[0], 8, 4, 1, 1)?,
        [Some(0)]
    );
    let overflow_water = WaterNode {
        amount: Some(8),
        ..overflow_empty
    };
    assert!(matches!(
        program.step(queue, &[overflow_water], &[0], 8, 4, 1, 1),
        Err(WaterComputeError::CoordinateOverflow)
    ));
    let unavailable = WaterNode {
        amount: None,
        neighbors: [None; 5],
    };
    let lazy = program.step_detailed_available(
        queue,
        &[empty, unavailable],
        &[0],
        &[true, false],
        8,
        4,
        1,
        1,
    )?;
    assert_eq!(lazy.sampled, [true, false]);
    assert!(matches!(
        program.step_detailed_available(queue, &[unavailable], &[0], &[false], 8, 4, 1, 1),
        Err(WaterComputeError::UnavailableNode(0))
    ));
    let falling = WaterNode {
        amount: Some(8),
        neighbors: [Some(1), None, None, None, None],
    };
    assert!(matches!(
        program.step_detailed_available(
            queue,
            &[falling, unavailable],
            &[0],
            &[true, false],
            8,
            4,
            1,
            2
        ),
        Err(WaterComputeError::SampleBudget)
    ));
    assert!(matches!(
        program.step_detailed_available(
            queue,
            &[falling, unavailable],
            &[0],
            &[true, false],
            8,
            4,
            2,
            2
        ),
        Err(WaterComputeError::UnavailableNode(1))
    ));
    Ok(())
}

fn verify_unknown_errors(
    program: &WaterTransferProgram,
    queue: &wgpu::Queue,
    empty: WaterNode,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_gpu::WaterNodeStatus::{Loaded, Unknown};
    let unknown = WaterNode {
        amount: None,
        neighbors: [None; 5],
    };
    let result = program.step_detailed_status(
        queue,
        &[empty, unknown],
        &[0],
        &[Loaded, Unknown],
        8,
        4,
        1,
        1,
    )?;
    assert_eq!(result.sampled, [true, false]);
    assert!(matches!(
        program.step_detailed_status(queue, &[unknown], &[0], &[Unknown], 8, 4, 1, 1),
        Err(WaterComputeError::UnknownNode(0))
    ));
    let source = WaterNode {
        amount: Some(8),
        neighbors: [Some(1), None, None, None, None],
    };
    assert!(matches!(
        program.step_detailed_status(
            queue,
            &[source, unknown],
            &[0],
            &[Loaded, Unknown],
            8,
            4,
            1,
            2
        ),
        Err(WaterComputeError::SampleBudget)
    ));
    assert!(matches!(
        program.step_detailed_status(
            queue,
            &[source, unknown],
            &[0],
            &[Loaded, Unknown],
            8,
            4,
            2,
            2
        ),
        Err(WaterComputeError::UnknownNode(1))
    ));
    assert_eq!(program.step(queue, &[empty], &[0], 8, 4, 1, 1)?, [Some(0)]);
    println!("PASS: lazy unknown-node errors, sample precedence and recovery");
    Ok(())
}

fn verify_pending(
    program: &WaterTransferProgram,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_gpu::WaterNodeStatus::{Loaded, Unknown};
    let empty = WaterNode {
        amount: Some(0),
        neighbors: [None; 5],
    };
    let source = WaterNode {
        amount: Some(8),
        neighbors: [Some(1), None, None, None, None],
    };
    let unknown = WaterNode {
        amount: None,
        neighbors: [None; 5],
    };
    let mut failed =
        program.begin_step_detailed_status(queue, &[unknown], &[0], &[Unknown], 8, 4, 1, 1)?;
    let mut success = program.begin_step_detailed_status(
        queue,
        &[source, empty],
        &[0],
        &[Loaded, Loaded],
        8,
        4,
        2,
        2,
    )?;
    // Native test drives completion; the public pending API itself never polls.
    device.poll(wgpu::PollType::wait_indefinitely())?;
    assert!(matches!(
        failed.try_result(),
        Err(WaterComputeError::UnknownNode(0))
    ));
    assert_eq!(
        success
            .try_result()?
            .ok_or("readback still pending")?
            .amounts,
        [Some(0), Some(8)]
    );
    assert!(matches!(
        success.try_result(),
        Err(WaterComputeError::Compute(
            voxy_render::ComputeError::Consumed
        ))
    ));
    assert!(matches!(
        failed.try_result(),
        Err(WaterComputeError::Compute(
            voxy_render::ComputeError::Consumed
        ))
    ));
    let mut abandoned =
        program.begin_step_detailed_status(queue, &[empty], &[0], &[Loaded], 8, 4, 1, 1)?;
    let _ = abandoned.try_result();
    drop(abandoned);
    assert_eq!(program.step(queue, &[empty], &[0], 8, 4, 1, 1)?, [Some(0)]);
    println!(
        "PASS: pending GPU water success/error isolation, single consumption and dropped-readback recovery"
    );
    Ok(())
}

fn verify_pending_world(
    program: &WaterTransferProgram,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    states: WaterStates,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_world::{CommitError, EditTxn, VoxelWrite};
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let active = [VoxelPos {
        x: 16,
        y: 18,
        z: 16,
    }];
    let budget = WaterBudget::default();
    let expected = cpu_plan(&scene.world, states, &active)?.0;
    let mut pending = program.begin_plan_world(
        queue,
        &scene.world,
        scene.world.registry(),
        states,
        &active,
        EditSource::Simulation,
        budget,
    )?;
    scene.world.commit(EditTxn {
        source: EditSource::Simulation,
        expected: vec![],
        writes: vec![VoxelWrite {
            pos: VoxelPos { x: 0, y: 20, z: 0 },
            block: states.0[0],
        }],
    })?;
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let stale = pending.try_plan()?.ok_or("world plan still pending")?;
    assert_eq!(stale, expected);
    let WaterPlan::Transaction { edit, .. } = stale else {
        return Err("missing transfer".into());
    };
    assert!(matches!(
        scene.world.commit(edit),
        Err(CommitError::RevisionConflict { .. })
    ));
    assert_eq!(scene.world.sample(active[0]), Sample::Loaded(states.0[7]));
    assert!(pending.try_plan().is_err());
    let mut fresh = program.begin_plan_world(
        queue,
        &scene.world,
        scene.world.registry(),
        states,
        &active,
        EditSource::Simulation,
        budget,
    )?;
    let expected = cpu_plan(&scene.world, states, &active)?.0;
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let plan = fresh.try_plan()?.ok_or("fresh plan still pending")?;
    assert_eq!(plan, expected);
    let WaterPlan::Transaction { edit, .. } = plan else {
        return Err("missing fresh transfer".into());
    };
    scene.world.commit(edit)?;
    assert_eq!(
        scene.world.sample(active[0]),
        Sample::Loaded(BlockStateId::AIR)
    );
    let mut empty = program.begin_plan_world(
        queue,
        &scene.world,
        scene.world.registry(),
        states,
        &[],
        EditSource::Simulation,
        budget,
    )?;
    assert_eq!(empty.try_plan()?, Some(WaterPlan::Settled));
    assert!(empty.try_plan().is_err());
    println!(
        "PASS: pending world plans retain revisions, reject stale commits and recover from fresh capture"
    );
    Ok(())
}
