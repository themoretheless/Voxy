//! Browser execution proof of asynchronous GPU-assisted continuous collision.
use super::browser::{error, yield_browser};
use physics_voxel::{AnchoredAabb, SweepConfig, SweepResult, sweep_aabb};
use voxy_gpu::{GpuSweepError, PendingVoxelSweep, VoxelRegionProgram};
use voxy_world::{BlockStateId, EditSource, EditTxn, VoxelPos, VoxelView, VoxelWrite};
use wasm_bindgen::JsValue;

pub(crate) async fn validate(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<u32, JsValue> {
    let program = VoxelRegionProgram::new(device).await.map_err(error)?;
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).map_err(error)?;
    let config = SweepConfig::default();
    let cases = sweep_cases();
    for (anchor, displacement) in cases {
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
        )
        .map_err(error)?;
        let mut pending = program
            .begin_sweep(
                queue,
                &scene.world,
                scene.world.registry(),
                aabb,
                displacement,
                config,
            )
            .map_err(error)?;
        let actual = read(&mut pending, &scene.world).await?;
        if actual != expected {
            return Err(error("browser GPU/CPU sweeps differ"));
        }
        if !matches!(
            pending.try_sweep(&scene.world),
            Err(GpuSweepError::Compute(voxy_render::ComputeError::Consumed))
        ) {
            return Err(error("browser sweep consumed twice"));
        }
    }
    let aabb = AnchoredAabb {
        anchor: cases[0].0,
        min: [0.1; 3],
        max: [0.9; 3],
    };
    let mut pending = program
        .begin_sweep(
            queue,
            &scene.world,
            scene.world.registry(),
            aabb,
            cases[0].1,
            config,
        )
        .map_err(error)?;
    scene
        .world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![],
            writes: vec![VoxelWrite {
                pos: VoxelPos { x: 0, y: 0, z: 0 },
                block: BlockStateId::AIR,
            }],
        })
        .map_err(error)?;
    reject_stale(&mut pending, &scene.world).await?;
    validate_character(&program, queue).await?;
    validate_character_recovery(&program, queue).await?;
    validate_vehicle(&program, queue).await?;
    Ok(485)
}
async fn read(
    pending: &mut PendingVoxelSweep,
    view: &impl VoxelView,
) -> Result<SweepResult, JsValue> {
    let deadline = js_sys::Date::now() + 30_000.0;
    loop {
        if let Some(result) = pending.try_sweep(view).map_err(error)? {
            return Ok(result);
        }
        if js_sys::Date::now() >= deadline {
            return Err(error("browser sweep timed out"));
        }
        yield_browser().await?;
    }
}

async fn reject_stale(
    pending: &mut PendingVoxelSweep,
    view: &impl VoxelView,
) -> Result<(), JsValue> {
    let deadline = js_sys::Date::now() + 30_000.0;
    loop {
        match pending.try_sweep(view) {
            Err(GpuSweepError::StaleWorld) => break,
            Ok(None) => {}
            _ => return Err(error("browser stale sweep accepted")),
        }
        if js_sys::Date::now() >= deadline {
            return Err(error("browser stale sweep timed out"));
        }
        yield_browser().await?;
    }
    Ok(())
}

fn sweep_cases() -> [(VoxelPos, [f64; 3]); 4] {
    [
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
    ]
}

async fn validate_character(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), JsValue> {
    use physics::{AnchoredAabb, CharacterConfig, CharacterInput, CharacterState, Origin};
    let scene = voxy_runtime::build_bootstrap_scene(42, 0).map_err(error)?;
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
    let mut expected = initial;
    for tick in 0..240 {
        let input = CharacterInput {
            planar_velocity: [if tick < 120 { 0.5 } else { -0.5 }, 0.0],
            jump_pressed: tick == 180,
        };
        let expected_report = physics::step_character(
            &cpu,
            &mut expected,
            input,
            1.0 / 60.0,
            CharacterConfig::default(),
        )
        .map_err(error)?;
        let mut pending = voxy_gpu::PendingGpuCharacter::new(
            actual,
            input,
            1.0 / 60.0,
            CharacterConfig::default(),
            scene.world.registry(),
        );
        let deadline = js_sys::Date::now() + 30_000.0;
        let (next, report) = loop {
            if let Some(result) = pending
                .try_step(program, queue, &scene.world)
                .map_err(error)?
            {
                break result;
            }
            if js_sys::Date::now() >= deadline {
                return Err(error("browser character task timed out"));
            }
            yield_browser().await?;
        };
        if next != expected || report != expected_report {
            return Err(error(format!(
                "browser GPU/CPU character differs at tick {tick}"
            )));
        }
        if pending.try_step(program, queue, &scene.world).is_ok() {
            return Err(error("browser character task consumed twice"));
        }
        actual = next;
    }
    Ok(())
}

async fn validate_character_recovery(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), JsValue> {
    use physics::{
        AnchoredAabb, CharacterConfig, CharacterError, CharacterInput, CharacterState, Origin,
    };
    use voxy_gpu::{CharacterGpuQueryError, PendingGpuCharacter};
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).map_err(error)?;
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
    let deadline = js_sys::Date::now() + 30_000.0;
    while stale.completed_sweeps() == 0 {
        if stale
            .try_step(program, queue, &scene.world)
            .map_err(error)?
            .is_some()
        {
            return Err(error("browser stale fixture finished before second query"));
        }
        if js_sys::Date::now() >= deadline {
            return Err(error("browser first character contact timed out"));
        }
        yield_browser().await?;
    }
    invalidate_character_chunk(&mut scene.world)?;
    if !matches!(
        stale.try_step(program, queue, &scene.world),
        Err(CharacterError::Sweep(CharacterGpuQueryError::Gpu(
            GpuSweepError::StaleWorld
        )))
    ) {
        return Err(error("browser stale character accepted"));
    }
    if !matches!(
        stale.try_step(program, queue, &scene.world),
        Err(CharacterError::Sweep(CharacterGpuQueryError::Gpu(
            GpuSweepError::Compute(voxy_render::ComputeError::Consumed)
        )))
    ) {
        return Err(error("browser stale character consumed twice"));
    }
    drop(stale);
    let cpu = physics_voxel::VoxelCollisionWorld {
        view: &scene.world,
        registry: scene.world.registry(),
    };
    let mut expected = initial;
    let expected_report =
        physics::step_character(&cpu, &mut expected, input, 0.2, CharacterConfig::default())
            .map_err(error)?;
    let mut fresh = PendingGpuCharacter::new(
        initial,
        input,
        0.2,
        CharacterConfig::default(),
        scene.world.registry(),
    );
    let deadline = js_sys::Date::now() + 30_000.0;
    loop {
        if let Some((state, report)) = fresh
            .try_step(program, queue, &scene.world)
            .map_err(error)?
        {
            if state != expected || report != expected_report {
                return Err(error("browser fresh character recovery differs"));
            }
            break;
        }
        if js_sys::Date::now() >= deadline {
            return Err(error("browser character recovery timed out"));
        }
        yield_browser().await?;
    }
    Ok(())
}

fn add_character_floor(world: &mut voxy_world::World) -> Result<(), JsValue> {
    let stone = world
        .registry()
        .find(&voxy_world::ResourceKey::parse("voxy:stone").map_err(error)?)
        .ok_or_else(|| error("missing stone"))?;
    world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![],
            writes: [10, 11]
                .map(|x| VoxelWrite {
                    pos: VoxelPos { x, y: 20, z: 10 },
                    block: stone,
                })
                .to_vec(),
        })
        .map_err(error)?;
    Ok(())
}

fn invalidate_character_chunk(world: &mut voxy_world::World) -> Result<(), JsValue> {
    world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![],
            writes: vec![VoxelWrite {
                pos: VoxelPos { x: 0, y: 0, z: 0 },
                block: BlockStateId::AIR,
            }],
        })
        .map_err(error)?;
    Ok(())
}

async fn validate_vehicle(
    program: &VoxelRegionProgram,
    queue: &wgpu::Queue,
) -> Result<(), JsValue> {
    use physics_voxel::{AnchoredAabb, CharacterState, VehicleConfig, VehicleInput, VehicleState};
    let scene = voxy_runtime::build_bootstrap_scene(42, 0).map_err(error)?;
    let mut actual = VehicleState {
        chassis: CharacterState {
            body: AnchoredAabb {
                anchor: VoxelPos { x: 8, y: 20, z: 8 },
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
        let expected_report = physics_voxel::step_vehicle(
            &scene.world,
            scene.world.registry(),
            &mut expected,
            input,
            1.0 / 60.0,
            config,
        )
        .map_err(error)?;
        let mut pending = voxy_gpu::PendingGpuVehicle::new(
            actual,
            input,
            1.0 / 60.0,
            config,
            scene.world.registry(),
        );
        let deadline = js_sys::Date::now() + 30_000.0;
        let (next, report) = loop {
            if let Some(result) = pending
                .try_step(program, queue, &scene.world)
                .map_err(error)?
            {
                break result;
            }
            if js_sys::Date::now() >= deadline {
                return Err(error("browser vehicle timed out"));
            }
            yield_browser().await?;
        };
        if next != expected || report != expected_report {
            return Err(error(format!("browser vehicle differs at tick {tick}")));
        }
        if !matches!(
            pending.try_step(program, queue, &scene.world),
            Err(voxy_gpu::VehicleGpuError::Consumed)
        ) {
            return Err(error("browser vehicle consumed twice"));
        }
        actual = next;
        forward |= next.longitudinal_speed > 0.1;
        reverse |= next.longitudinal_speed < -0.1;
        braked |= input.brake && next.longitudinal_speed == 0.0;
    }
    if !(forward && reverse && braked) {
        return Err(error("browser vehicle fixture lacks forward/reverse/brake"));
    }
    Ok(())
}
