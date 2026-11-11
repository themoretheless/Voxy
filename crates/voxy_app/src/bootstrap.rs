//! World bootstrap, procedural actor construction, water discovery and CLI subcommands.
use super::*;

/// Spawn above every collidable cell under the actor's straddling AABB footprint.
pub(super) fn terrain_spawn(world: &World) -> Result<VoxelPos, String> {
    use voxy_world::{CollisionShape, Sample, VoxelView};
    let mut surface = None;
    for x in [15, 16] {
        for z in [15, 16] {
            let mut height = None;
            for y in (-32..=63).rev() {
                let Sample::Loaded(block) = world.sample(VoxelPos { x, y, z }) else {
                    return Err("spawn column is not fully loaded".into());
                };
                let definition = world.registry().get(block).ok_or("unknown spawn block")?;
                if definition.collision != CollisionShape::Empty {
                    height = Some(y);
                    break;
                }
            }
            let height = height.ok_or("spawn has no supporting terrain")?;
            surface = Some(surface.map_or(height, |prior: i64| prior.max(height)));
        }
    }
    let y = surface.ok_or("spawn footprint is empty")? + 1;
    if y > 61 {
        return Err("spawn lacks loaded headroom".into());
    }
    Ok(VoxelPos { x: 16, y, z: 16 })
}

pub(super) fn terrain_water_cells(
    scene: &BootstrapScene,
    states: WaterStates,
) -> Result<Vec<VoxelPos>, String> {
    use voxy_world::VoxelView;
    let mut cells = Vec::new();
    for chunk in &scene.chunks {
        let snapshot = scene
            .world
            .chunk(chunk.pos)
            .ok_or("water chunk is not loaded")?;
        let base_x = chunk
            .pos
            .x
            .checked_mul(32)
            .ok_or("water coordinate overflow")?;
        let base_y = chunk
            .pos
            .y
            .checked_mul(32)
            .ok_or("water coordinate overflow")?;
        let base_z = chunk
            .pos
            .z
            .checked_mul(32)
            .ok_or("water coordinate overflow")?;
        for y in 0..32_u8 {
            for z in 0..32_u8 {
                for x in 0..32_u8 {
                    let local =
                        voxy_world::LocalPos::new(x, y, z).map_err(|error| error.to_string())?;
                    if states.0.contains(&snapshot.data.blocks.get(local.index())) {
                        cells.push(VoxelPos {
                            x: base_x + i64::from(x),
                            y: base_y + i64::from(y),
                            z: base_z + i64::from(z),
                        });
                    }
                }
            }
        }
    }
    Ok(cells)
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum WaterPublication {
    Settled,
    Committed,
    Retry,
}

pub(super) fn publish_water_plan(
    world: &mut World,
    active: &mut Vec<VoxelPos>,
    dirty: &mut BTreeSet<ChunkPos>,
    submitted: Vec<VoxelPos>,
    plan: WaterPlan,
) -> Result<WaterPublication, String> {
    let outcome = match plan {
        WaterPlan::Settled => WaterPublication::Settled,
        WaterPlan::Transaction { edit, next_active } => match world.commit(edit) {
            Ok(receipt) => {
                dirty.extend(invalidated_derived_chunks(&receipt));
                active.extend(next_active.into_vec());
                WaterPublication::Committed
            }
            Err(voxy_world::CommitError::RevisionConflict { .. }) => {
                active.extend(submitted);
                WaterPublication::Retry
            }
            Err(error) => return Err(error.to_string()),
        },
    };
    active.sort_unstable();
    active.dedup();
    Ok(outcome)
}

/// Select existing submerged terrain away from resident boundaries. The
/// autopilot destroys only this real support block to exercise edit-triggered flow.
pub(super) fn natural_water_bed(
    world: &World,
    states: WaterStates,
    water: &[VoxelPos],
) -> Option<VoxelPos> {
    use voxy_world::{BlockStateId, Sample, VoxelView};
    water.iter().find_map(|&source| {
        let bed = VoxelPos {
            y: source.y.checked_sub(1)?,
            ..source
        };
        let Sample::Loaded(block) = world.sample(bed) else {
            return None;
        };
        if block == BlockStateId::AIR || states.0.contains(&block) {
            return None;
        }
        // Flow can wake several neighbors over later ticks; keep a loaded margin.
        for dx in -2..=2 {
            for dy in -2..=2 {
                for dz in -2..=2 {
                    let pos = VoxelPos {
                        x: bed.x.checked_add(dx)?,
                        y: bed.y.checked_add(dy)?,
                        z: bed.z.checked_add(dz)?,
                    };
                    if !matches!(world.sample(pos), Sample::Loaded(_)) {
                        return None;
                    }
                }
            }
        }
        match physics_voxel::plan_explosion(
            world,
            world.registry(),
            EditSource::Player(1),
            Explosion {
                center: bed,
                radius: 0,
                ..Explosion::default()
            },
        ) {
            Ok(DestructionPlan::Transaction(_)) => Some(bed),
            _ => None,
        }
    })
}

/// Reactivate existing water touching committed edits, preserving pending work.
pub(super) fn wake_water_after_edits(
    world: &World,
    states: WaterStates,
    changed: &[voxy_world::VoxelWrite],
    active: &mut Vec<VoxelPos>,
) {
    use voxy_world::{Sample, VoxelView};
    let before = active.len();
    for edit in changed {
        for (dx, dy, dz) in [
            (0, 0, 0),
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ] {
            let (Some(x), Some(y), Some(z)) = (
                edit.pos.x.checked_add(dx),
                edit.pos.y.checked_add(dy),
                edit.pos.z.checked_add(dz),
            ) else {
                continue;
            };
            let position = VoxelPos { x, y, z };
            if let Sample::Loaded(block) = world.sample(position)
                && states.0.contains(&block)
            {
                active.push(position);
            }
        }
    }
    if active.len() > before {
        // Dedupe once per commit instead of rebuilding a set of the whole list. Order is not
        // observable by the planner (see `advance_water`); it only affects which cells the
        // `split_off(max_active)` partition defers when the list exceeds the budget.
        active.sort_unstable();
        active.dedup();
    }
}

pub(super) fn bootstrap_water_states(world: &World) -> Result<WaterStates, String> {
    let mut states = Vec::with_capacity(8);
    for level in 1..=8 {
        let key =
            ResourceKey::parse(format!("voxy:water_{level}")).map_err(|error| error.to_string())?;
        states.push(
            world
                .registry()
                .find(&key)
                .ok_or_else(|| format!("missing built-in state {key:?}"))?,
        );
    }
    let states: [_; 8] = states
        .try_into()
        .map_err(|_| "invalid built-in water state count".to_owned())?;
    WaterStates(states)
        .validate(world.registry())
        .map_err(|error| error.to_string())
}

pub(super) fn bootstrap_actor() -> Result<(AnimatedActor, SkinnedMesh), Box<dyn std::error::Error>>
{
    let joints = [
        ("root", Vec3::ZERO),
        ("left-leg", Vec3::new(-0.28, 1.05, 0.0)),
        ("right-leg", Vec3::new(0.28, 1.05, 0.0)),
        ("left-arm", Vec3::new(-0.68, 2.35, 0.0)),
        ("right-arm", Vec3::new(0.68, 2.35, 0.0)),
    ];
    let skeleton = Skeleton::new(
        joints
            .into_iter()
            .enumerate()
            .map(|(index, (name, pivot))| Joint {
                name: Arc::from(name),
                parent: (index != 0).then_some(0),
                bind_local: Transform {
                    translation: pivot,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::from_translation(-pivot),
            })
            .collect(),
    )?;
    let clip = Arc::new(AnimationClip::new(
        "walk-cycle",
        1.0,
        Playback::Loop,
        vec![
            JointTrack::default(),
            limb_swing(0.62),
            limb_swing(-0.62),
            limb_swing(-0.52),
            limb_swing(0.52),
        ],
        &skeleton,
    )?);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.5, 1.0, -0.28],
        [0.5, 2.5, 0.28],
        0,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.46, 2.5, -0.36],
        [0.46, 3.38, 0.36],
        0,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.5, 0.0, -0.24],
        [-0.06, 1.12, 0.24],
        1,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [0.06, 0.0, -0.24],
        [0.5, 1.12, 0.24],
        2,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.82, 1.05, -0.2],
        [-0.5, 2.4, 0.2],
        3,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [0.5, 1.05, -0.2],
        [0.82, 2.4, 0.2],
        4,
    );
    if animals::check_requested() {
        vertices.clear();
        indices.clear();
    }
    animals::append_mesh(&mut vertices, &mut indices, false, 5)?;
    animals::append_mesh(
        &mut vertices,
        &mut indices,
        true,
        5 + (animals::JOINTS / 2) as u16,
    )?;
    let mesh = SkinnedMesh::new(vertices, indices, 5 + animals::JOINTS as u16)?;
    Ok((
        AnimatedActor {
            skeleton,
            animator: Animator::new(clip),
        },
        mesh,
    ))
}

fn limb_swing(amount: f32) -> JointTrack {
    JointTrack {
        rotations: [amount, -amount, amount]
            .into_iter()
            .zip([0.0, 0.5, 1.0])
            .map(|(angle, time)| QuatKey {
                time,
                value: Quat::from_rotation_x(angle),
            })
            .collect(),
        ..JointTrack::default()
    }
}

fn push_actor_box(
    vertices: &mut Vec<SkinnedVertex>,
    indices: &mut Vec<u32>,
    min: [f32; 3],
    max: [f32; 3],
    joint: u16,
) {
    let corners = [
        [min[0], min[1], min[2]],
        [max[0], min[1], min[2]],
        [max[0], max[1], min[2]],
        [min[0], max[1], min[2]],
        [min[0], min[1], max[2]],
        [max[0], min[1], max[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ];
    let faces = [
        ([0, 3, 2, 1], [0.0, 0.0, -1.0]),
        ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
        ([0, 4, 7, 3], [-1.0, 0.0, 0.0]),
        ([1, 2, 6, 5], [1.0, 0.0, 0.0]),
        ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
        ([3, 7, 6, 2], [0.0, 1.0, 0.0]),
    ];
    for (face, normal) in faces {
        let start = u32::try_from(vertices.len()).expect("actor mesh stays small");
        for (corner, uv) in face
            .into_iter()
            .zip([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
        {
            vertices.push(actor_vertex(corners[corner], normal, uv, joint));
        }
        indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }
}

pub(crate) fn actor_vertex(
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    joint: u16,
) -> SkinnedVertex {
    SkinnedVertex {
        position,
        normal,
        uv,
        joints: [joint, 0, 0, 0],
        weights: [u16::MAX, 0, 0, 0],
    }
}

pub(super) fn loading_title(percent: u8, stage: &str) -> String {
    let percent = percent.min(100);
    let filled = usize::from(percent / 5);
    let empty = 20_usize.saturating_sub(filled);
    format!(
        "Voxy Tamagotchi |{}{}| {percent:3}%  {stage}",
        "█".repeat(filled),
        "░".repeat(empty)
    )
}

pub(super) fn build_startup_scene(
    graphics: voxy_render::GraphicsOptions,
    radius: i32,
) -> Result<BootstrapScene, voxy_runtime::BootstrapError> {
    let backend =
        terrain_options::TerrainBackend::parse(std::env::args().skip(1)).map_err(|error| {
            eprintln!("Terrain selection failed: {error}");
            voxy_world::GenerationError::BackendFailure
        })?;
    let factory = |palette,
                   water|
     -> Result<Box<dyn voxy_world::ChunkGenerator>, voxy_world::GenerationError> {
        if backend == terrain_options::TerrainBackend::Cpu {
            return Ok(Box::new(voxy_world::ProceduralTerrainGenerator::new(
                palette, water,
            )));
        }
        if let terrain_options::TerrainBackend::Cuda { ordinal } = backend {
            let generator =
                voxy_gpu::CudaTerrainGenerator::new(ordinal, palette, water).map_err(|error| {
                    eprintln!("CUDA terrain initialization failed: {error}");
                    voxy_world::GenerationError::BackendFailure
                })?;
            eprintln!("CUDA terrain: {:?}", generator.capabilities());
            Ok(Box::new(generator))
        } else {
            let generator =
                pollster::block_on(voxy_gpu::GpuTerrainGenerator::new(graphics, palette, water))
                    .map_err(|error| {
                        eprintln!("GPU terrain initialization failed: {error}");
                        voxy_world::GenerationError::BackendFailure
                    })?;
            eprintln!("GPU terrain: {:?}", generator.capabilities().adapter);
            Ok(Box::new(generator))
        }
    };
    if std::env::args().any(|arg| arg == "--voxel-plants") {
        voxy_runtime::build_generated_scene(0x56_4f_58_59, radius, factory)
    } else {
        voxy_runtime::build_generated_scene_with_mesh_plants(0x56_4f_58_59, radius, factory)
    }
}

pub(super) fn handle_early_cli_commands(
    arguments: &[String],
) -> Result<Option<()>, Box<dyn std::error::Error>> {
    if let Some(index) = arguments.iter().position(|arg| arg == "--cpu-replay") {
        let path = arguments
            .get(index + 1)
            .ok_or("--cpu-replay requires a CSV path")?;
        let scenario = arguments
            .iter()
            .position(|arg| arg == "--cpu-replay-scenario")
            .map(|i| {
                arguments
                    .get(i + 1)
                    .map(String::as_str)
                    .ok_or("missing CPU replay scenario")
            })
            .transpose()?
            .unwrap_or("motion");
        cpu_replay::run(
            std::path::Path::new(path),
            scenario,
            arguments
                .iter()
                .any(|arg| arg == "--cpu-replay-profile-pose"),
        )?;
        return Ok(Some(()));
    }
    if arguments.first().is_some_and(|arg| arg == "--vegetation") {
        let kind = match arguments.get(1).map(String::as_str) {
            None | Some("tree") => voxy_model::plants::PlantKind::Tree,
            Some("shrub") => voxy_model::plants::PlantKind::Shrub,
            _ => return Err("--vegetation expects tree or shrub".into()),
        };
        let seed = arguments
            .iter()
            .position(|a| a == "--seed")
            .map(|i| {
                arguments
                    .get(i + 1)
                    .ok_or("--seed requires a value")?
                    .parse::<u64>()
                    .map_err(|_| "invalid seed")
            })
            .transpose()?
            .unwrap_or(42);
        let height = arguments
            .iter()
            .position(|a| a == "--height")
            .map(|i| {
                arguments
                    .get(i + 1)
                    .ok_or("--height requires a value")?
                    .parse::<f32>()
                    .map_err(|_| "invalid height")
            })
            .transpose()?
            .unwrap_or(if kind == voxy_model::plants::PlantKind::Tree {
                7.0
            } else {
                1.6
            });
        let bytes = voxy_model::plants::generate_plant_glb(voxy_model::plants::PlantOptions {
            kind,
            seed,
            height,
        })?;
        let output = arguments
            .iter()
            .position(|a| a == "--output")
            .map(|i| {
                arguments
                    .get(i + 1)
                    .map(std::path::PathBuf::from)
                    .ok_or("--output requires a path")
            })
            .transpose()?
            .unwrap_or_else(|| {
                std::path::PathBuf::from(format!("artifacts/vegetation/{kind:?}-{seed}.glb"))
            });
        if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&output, bytes)?;
        println!("Generated plant: {}", output.display());
        if !arguments.iter().any(|a| a == "--export-only") {
            let preview = output.with_extension("preview.glb");
            std::fs::write(
                &preview,
                voxy_model::plants::generate_plant_editor_glb(voxy_model::plants::PlantOptions {
                    kind,
                    seed,
                    height,
                })?,
            )?;
            println!(
                "Simplified preview written to {} (open with `cargo run -p voxy_editor -- {}`)",
                preview.display(),
                preview.display()
            );
        }
        return Ok(Some(()));
    }

    if let Some(command) = arguments.first().filter(|arg| {
        matches!(
            arg.as_str(),
            "--game-package" | "--model" | "--model-manifest"
        )
    }) {
        return Err(format!(
            "`{command}` belongs to the standalone scene/editor runtime; run `cargo run -p voxy_editor -- {}` instead",
            arguments.join(" ")
        )
        .into());
    }
    Ok(None)
}
