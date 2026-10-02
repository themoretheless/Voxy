//! Physical NVIDIA water transaction parity; no solver fallback.
use physics_voxel::{WaterBudget, WaterPlan, WaterStates, step_water};
use std::sync::Arc;
use voxy_gpu::{CudaWaterTransferProgram, WaterComputeError, WaterNode, WaterNodeStatus};
use voxy_world::{BlockStateId, EditSource, ResourceKey, Sample, VoxelPos, VoxelView};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let ordinal = args.next().map_or(Ok(0), |value| value.parse::<usize>())?;
    if args.next().is_some() {
        return Err("expected at most one CUDA device ordinal".into());
    }
    let compute = Arc::new(voxy_cuda::CudaCompute::new(ordinal, 16 * 1024 * 1024)?);
    println!("CUDA water: {:?}", compute.capabilities()?);
    let program = CudaWaterTransferProgram::new(Arc::clone(&compute));
    verify_graph_failures(&program)?;
    verify_downward_capacity(&program)?;
    verify_ordered_cascade(&program)?;
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let mut levels = [BlockStateId::AIR; 8];
    for (index, level) in levels.iter_mut().enumerate() {
        *level = scene
            .world
            .registry()
            .find(&ResourceKey::parse(format!("voxy:water_{}", index + 1))?)
            .ok_or("missing water state")?;
    }
    let states = WaterStates(levels);
    verify_stale_recovery(&program, &scene.world, states)?;
    let positions: Vec<_> = (0..32)
        .flat_map(|x| (0..32).flat_map(move |y| (0..32).map(move |z| VoxelPos { x, y, z })))
        .collect();
    let mut active: Vec<_> = positions
        .into_iter()
        .filter(|&pos| match scene.world.sample(pos) {
            Sample::Loaded(block) => levels.contains(&block),
            _ => false,
        })
        .collect();
    if active.is_empty() {
        return Err("fixture contains no water".into());
    }
    let mut changed = 0;
    for _ in 0..16 {
        let expected = step_water(
            &scene.world,
            scene.world.registry(),
            states,
            &active,
            EditSource::Simulation,
            WaterBudget::default(),
        )?;
        let actual = program.plan_world(
            &scene.world,
            scene.world.registry(),
            states,
            &active,
            EditSource::Simulation,
            WaterBudget::default(),
        )?;
        if actual != expected {
            return Err("CUDA/CPU water transaction or activation mismatch".into());
        }
        match actual {
            WaterPlan::Settled => active.clear(),
            WaterPlan::Transaction { edit, next_active } => {
                scene.world.commit(edit)?;
                active = next_active.into_vec();
                changed += 1;
            }
        }
    }
    if changed == 0 {
        return Err("fixture never committed water".into());
    }
    if compute.reserved_device_bytes()? != 0 {
        return Err("CUDA water success/failure reservations leaked".into());
    }
    println!(
        "PASS: CUDA water successful and failed world/graph requests release device reservations"
    );
    println!("PASS: CUDA water 16 exact CPU world plans, revisions, writes and next-active parity");
    Ok(())
}

fn verify_stale_recovery(
    program: &CudaWaterTransferProgram,
    original: &voxy_world::World,
    states: WaterStates,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_world::{CommitError, EditTxn, VoxelWrite};
    let mut world = original.clone();
    let source = VoxelPos {
        x: 16,
        y: 25,
        z: 16,
    };
    let below = VoxelPos {
        x: 16,
        y: 24,
        z: 16,
    };
    world.commit(EditTxn {
        source: EditSource::Player(1),
        expected: vec![],
        writes: vec![
            VoxelWrite {
                pos: source,
                block: states.0[7],
            },
            VoxelWrite {
                pos: below,
                block: BlockStateId::AIR,
            },
        ],
    })?;
    let budget = WaterBudget::default();
    let active = [source];
    let stale = program.plan_world(
        &world,
        world.registry(),
        states,
        &active,
        EditSource::Simulation,
        budget,
    )?;
    let WaterPlan::Transaction { edit, .. } = stale else {
        return Err("stale fixture did not transfer water".into());
    };
    world.commit(EditTxn {
        source: EditSource::Player(1),
        expected: vec![],
        writes: vec![VoxelWrite {
            pos: VoxelPos { x: 0, y: 20, z: 0 },
            block: states.0[0],
        }],
    })?;
    let before: Vec<_> = edit
        .writes
        .iter()
        .map(|write| (write.pos, world.sample(write.pos)))
        .collect();
    if !matches!(
        world.commit(edit),
        Err(CommitError::RevisionConflict { .. })
    ) {
        return Err("stale CUDA water plan accepted".into());
    }
    for (pos, sample) in before {
        if world.sample(pos) != sample {
            return Err("stale CUDA plan partially published".into());
        }
    }
    let expected = step_water(
        &world,
        world.registry(),
        states,
        &active,
        EditSource::Simulation,
        budget,
    )?;
    let fresh = program.plan_world(
        &world,
        world.registry(),
        states,
        &active,
        EditSource::Simulation,
        budget,
    )?;
    if fresh != expected {
        return Err("fresh CUDA water recovery differs from CPU".into());
    }
    let WaterPlan::Transaction { edit, .. } = fresh else {
        return Err("fresh fixture did not transfer water".into());
    };
    world.commit(edit)?;
    if world.sample(source) != Sample::Loaded(BlockStateId::AIR)
        || world.sample(below) != Sample::Loaded(states.0[7])
    {
        return Err("fresh CUDA water transfer not published".into());
    }
    println!(
        "PASS: CUDA water stale revision rejection, no partial publication and fresh CPU parity"
    );
    Ok(())
}

fn verify_graph_failures(
    program: &CudaWaterTransferProgram,
) -> Result<(), Box<dyn std::error::Error>> {
    let empty = WaterNode {
        amount: Some(0),
        neighbors: [None; 5],
    };
    let source = WaterNode {
        amount: Some(8),
        neighbors: [Some(1), None, None, None, None],
    };
    let loaded = [WaterNodeStatus::Loaded; 2];
    assert!(matches!(
        program.step_detailed_status(&[source, empty], &[0], &loaded, 8, 4, 1, 2),
        Err(WaterComputeError::SampleBudget)
    ));
    assert!(matches!(
        program.step_detailed_status(&[source, empty], &[0], &loaded, 8, 4, 2, 1),
        Err(WaterComputeError::WriteBudget)
    ));
    for (status, unknown) in [
        (WaterNodeStatus::Unavailable, false),
        (WaterNodeStatus::Unknown, true),
    ] {
        let hidden = WaterNode {
            amount: None,
            neighbors: [None; 5],
        };
        let result = program.step_detailed_status(
            &[source, hidden],
            &[0],
            &[WaterNodeStatus::Loaded, status],
            8,
            4,
            2,
            2,
        );
        if unknown {
            assert!(matches!(result, Err(WaterComputeError::UnknownNode(1))));
        } else {
            assert!(matches!(result, Err(WaterComputeError::UnavailableNode(1))));
        }
        let dormant = program.step_detailed_status(
            &[hidden, empty],
            &[1],
            &[status, WaterNodeStatus::Loaded],
            8,
            4,
            1,
            1,
        )?;
        assert_eq!(dormant.sampled, [false, true]);
        assert_eq!(dormant.amounts, [None, Some(0)]);
    }
    let missing = WaterNode {
        neighbors: [None; 5],
        ..source
    };
    assert!(matches!(
        program.step_detailed_status(&[missing], &[0], &[WaterNodeStatus::Loaded], 8, 4, 2, 2),
        Err(WaterComputeError::MissingNeighbor)
    ));
    let overflow = WaterNode {
        neighbors: [Some(u32::MAX - 1), None, None, None, None],
        ..source
    };
    assert!(matches!(
        program.step_detailed_status(&[overflow], &[0], &[WaterNodeStatus::Loaded], 8, 4, 2, 2),
        Err(WaterComputeError::CoordinateOverflow)
    ));
    let fresh = program.step_detailed_status(&[source, empty], &[0], &loaded, 8, 4, 2, 2)?;
    assert_eq!(fresh.amounts, [Some(0), Some(8)]);
    assert_eq!(fresh.sampled, [true, true]);
    println!("PASS: CUDA water lazy faults, read/write budgets, provenance and fresh recovery");
    Ok(())
}

fn verify_downward_capacity(
    program: &CudaWaterTransferProgram,
) -> Result<(), Box<dyn std::error::Error>> {
    for source in 0_u8..=8 {
        for destination in 0_u8..=8 {
            for limit in 1_u8..=8 {
                let nodes = [
                    WaterNode {
                        amount: Some(source),
                        neighbors: [Some(1), Some(2), Some(2), Some(2), Some(2)],
                    },
                    WaterNode {
                        amount: Some(destination),
                        neighbors: [Some(2); 5],
                    },
                    WaterNode {
                        amount: None,
                        neighbors: [Some(2); 5],
                    },
                ];
                let result = program.step_detailed_status(
                    &nodes,
                    &[0],
                    &[WaterNodeStatus::Loaded; 3],
                    limit,
                    4,
                    3,
                    2,
                )?;
                let transfer = source.min((8 - destination).min(limit));
                if result.amounts != [Some(source - transfer), Some(destination + transfer), None]
                    || result.sampled != [true, source != 0, source > transfer]
                {
                    return Err(format!("CUDA downward capacity mismatch: source={source}, destination={destination}, limit={limit}").into());
                }
            }
        }
    }
    println!(
        "PASS: CUDA water 648 exhaustive downward capacity, limit, volume and lazy-read cases"
    );
    Ok(())
}

fn verify_ordered_cascade(
    program: &CudaWaterTransferProgram,
) -> Result<(), Box<dyn std::error::Error>> {
    let nodes = [
        WaterNode {
            amount: Some(8),
            neighbors: [Some(1), Some(3), Some(3), Some(3), Some(3)],
        },
        WaterNode {
            amount: Some(0),
            neighbors: [Some(2), Some(3), Some(3), Some(3), Some(3)],
        },
        WaterNode {
            amount: Some(0),
            neighbors: [Some(3); 5],
        },
        WaterNode {
            amount: None,
            neighbors: [Some(3); 5],
        },
    ];
    for (active, limit, amounts, sampled) in [
        (
            &[0_u32][..],
            8,
            [Some(0), Some(8), Some(0), None],
            [true, true, false, false],
        ),
        (
            &[0_u32, 1][..],
            8,
            [Some(0), Some(0), Some(8), None],
            [true, true, true, false],
        ),
        (
            &[0_u32, 1][..],
            4,
            [Some(4), Some(0), Some(4), None],
            [true, true, true, true],
        ),
    ] {
        let result = program.step_detailed_status(
            &nodes,
            active,
            &[WaterNodeStatus::Loaded; 4],
            limit,
            4,
            4,
            2,
        )?;
        if result.amounts != amounts || result.sampled != sampled {
            return Err("CUDA water ordered cascade or final write accounting mismatch".into());
        }
    }
    println!("PASS: CUDA water ordered active cascade, lazy reads and final write accounting");
    Ok(())
}
