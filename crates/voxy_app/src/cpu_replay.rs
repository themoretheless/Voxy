//! Window/device-free replay of the main client's CPU character/animation/pose path.
//! It intentionally excludes GPU writes, rendering, water/chunk workers and presentation.
use super::*;
use std::{fs::File, io::Write, path::Path};

pub fn run(
    path: &Path,
    scenario: &str,
    profile_pose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(scenario, "idle" | "motion" | "catchup") {
        return Err("CPU replay scenario must be idle, motion or catchup".into());
    }
    const WARMUP: usize = 120;
    const FRAMES: usize = 240;
    let scene = voxy_runtime::build_procedural_scene(42, 1)?;
    let (mut actor, mesh) = bootstrap_actor()?;
    let geometry = blake3::hash(bytemuck::cast_slice(mesh.vertices()))
        .to_hex()
        .to_string();
    let mut history = voxy_render::SkinnedMotionHistory::new(mesh.clone());
    let mut app = VoxyApp::default();
    app.initialize_gameplay(&scene)?;
    let mut palette = Vec::new();
    let mut published = Vec::new();
    let mut model = Mat4::IDENTITY;
    let mut checksum = blake3::Hasher::new();
    let mut rows: Vec<[f64; 8]> = Vec::new();
    let mut profiles = Vec::new();
    let mut ticks_total = 0;
    for frame in 0..WARMUP + FRAMES {
        let ticks = if scenario == "catchup" { 2 } else { frame % 2 };
        let mut times = [0.; 8];
        let mut pose_profile = voxy_render::RenderPoseProfile::default();
        let all = Instant::now();
        for _ in 0..ticks {
            ticks_total += 1;
            let moving = scenario != "idle";
            let direction = if ticks_total % 120 < 60 { 1. } else { -1. };
            let movement = Vec3::Z * if moving { direction } else { 0. };
            let start = Instant::now();
            let character = app.character.as_mut().unwrap();
            let config = CharacterConfig::default();
            step_character_in_field(
                &scene.world,
                scene.world.registry(),
                character,
                CharacterInput {
                    planar_velocity: [0., f64::from(movement.z * WALK_SPEED)],
                    jump_pressed: false,
                },
                f64::from(FIXED_DT),
                config,
                &[0., config.gravity, 0.],
            )?;
            app.actor_position = character_position(character);
            if moving {
                app.actor_yaw = movement.x.atan2(movement.z);
            }
            times[1] += start.elapsed().as_secs_f64() * 1e6;
            let start = Instant::now();
            actor.animator.set_speed(if moving { 1.55 } else { 0.65 })?;
            let animation = actor.animator.advance(&actor.skeleton, FIXED_DT)?;
            times[2] += start.elapsed().as_secs_f64() * 1e6;
            let start = Instant::now();
            let mut bodies: Vec<_> = app
                .companions
                .iter()
                .map(animals::Companion::body)
                .collect();
            for (index, animal) in app.companions.iter_mut().enumerate() {
                let mut obstacles: Vec<_> = bodies
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .map(|(_, b)| *b)
                    .collect();
                obstacles.push(character.body);
                animal.tick_with_bodies(
                    &scene.world,
                    app.actor_position + Vec3::X * if index == 0 { 1.2 } else { -1.2 },
                    FIXED_DT,
                    &obstacles,
                )?;
                bodies[index] = animal.body();
            }
            times[3] += start.elapsed().as_secs_f64() * 1e6;
            let start = Instant::now();
            model = Mat4::from_rotation_translation(
                Quat::from_rotation_y(app.actor_yaw),
                app.actor_position,
            );
            palette.clear();
            palette.extend_from_slice(&animation.skin_matrices);
            let inverse = model.inverse();
            for animal in &app.companions {
                animal.append_palette(inverse, &mut palette)?;
            }
            times[4] += start.elapsed().as_secs_f64() * 1e6;
        }
        // First no-tick frame has no admitted pose, just as the native client.
        if !palette.is_empty() {
            if ticks > 0 {
                let start = Instant::now();
                if profile_pose {
                    let (result, profile) = mesh.validate_render_pose_profiled(&palette, model);
                    result?;
                    if !profile.cpu_clock_available {
                        return Err("CPU pose clocks unavailable".into());
                    }
                    pose_profile = profile;
                } else {
                    mesh.validate_render_pose(&palette, model)?;
                }
                // CPU snapshot publication only; GPU uploads/LOD preparation are excluded.
                published.clone_from(&palette);
                std::hint::black_box(published.as_slice());
                times[5] = start.elapsed().as_secs_f64() * 1e6;
            }
            let start = Instant::now();
            history.presented(&published, model)?;
            times[6] = start.elapsed().as_secs_f64() * 1e6;
        }
        let start = Instant::now();
        let spheres: Vec<_> = app
            .companions
            .iter()
            .flat_map(animals::Companion::shadow_spheres)
            .collect();
        std::hint::black_box(&spheres);
        times[7] = start.elapsed().as_secs_f64() * 1e6;
        times[0] = all.elapsed().as_secs_f64() * 1e6;
        checksum.update(bytemuck::cast_slice(&palette));
        checksum.update(bytemuck::bytes_of(&model));
        checksum.update(bytemuck::cast_slice(&spheres));
        checksum.update(format!("{:?}", app.character).as_bytes());
        if frame >= WARMUP {
            rows.push(times);
            if profile_pose {
                profiles.push((ticks > 0, pose_profile));
            }
        }
    }
    let names = [
        "total_cpu",
        "character",
        "animation",
        "companions",
        "palette",
        "render_admission",
        "temporal_admission",
        "shadow_extraction",
    ];
    let mut csv = File::create(path)?;
    writeln!(
        csv,
        "frame,fixed_ticks,{}",
        names.map(|n| format!("{n}_us")).join(",")
    )?;
    for (i, row) in rows.iter().enumerate() {
        writeln!(
            csv,
            "{i},{},{}",
            if scenario == "catchup" {
                2
            } else {
                (i + WARMUP) % 2
            },
            row.map(|x| format!("{x:.6}")).join(",")
        )?;
    }
    if profile_pose {
        let mut output = File::create(path.with_extension("pose.csv"))?;
        writeln!(
            output,
            "frame,active,groups,workers,fallback,total_wall_us,caller_cpu_us,process_cpu_us,spawn_wall_us,spawn_cpu_us,main_work_wall_us,main_work_cpu_us,worker_work_wall_sum_us,worker_work_wall_max_us,worker_cpu_sum_us,worker_cpu_max_us,launch_delay_max_us,join_wall_us,join_cpu_us"
        )?;
        for (frame, (active, p)) in profiles.iter().enumerate() {
            let values = [
                p.total_wall_us,
                p.caller_cpu_us,
                p.process_cpu_us,
                p.spawn_wall_us,
                p.spawn_cpu_us,
                p.main_work_wall_us,
                p.main_work_cpu_us,
                p.worker_work_wall_sum_us,
                p.worker_work_wall_max_us,
                p.worker_cpu_sum_us,
                p.worker_cpu_max_us,
                p.launch_delay_max_us,
                p.join_wall_us,
                p.join_cpu_us,
            ];
            writeln!(
                output,
                "{frame},{active},{},{},{},{}",
                p.groups,
                p.workers,
                p.fallback,
                values.map(|v| format!("{v:.6}")).join(",")
            )?;
        }
    }
    let mut stats = serde_json::Map::new();
    for (column, name) in names.into_iter().enumerate() {
        let mut values: Vec<_> = rows.iter().map(|r| r[column]).collect();
        values.sort_by(f64::total_cmp);
        let percentile = |p: f64| values[(p * values.len() as f64).ceil() as usize - 1];
        stats.insert(name.into(),serde_json::json!({"p50_us":percentile(0.5),"p95_us":percentile(0.95),"p99_us":percentile(0.99),"max_us":values.last(),"sum_us":values.iter().sum::<f64>()}));
    }
    let report = serde_json::json!({"pose_profiled":profile_pose,"scenario":scenario,"seed":42,"warmup_frames":WARMUP,"measured_frames":FRAMES,"fixed_dt":FIXED_DT,"fixed_ticks_total":ticks_total,"vertices":mesh.vertices().len(),"geometry_blake3":geometry,"indices_blake3":blake3::hash(bytemuck::cast_slice(mesh.indices())).to_hex().to_string(),"state_blake3":checksum.finalize().to_hex().to_string(),"stages":stats,"gpu_created":false,"limitations":["CPU replay, not presented frame p99","fixed synthetic 120 Hz schedule or two-tick stress; same production routines, separate orchestration","water, chunk rebuild, vehicle, LOD, GPU upload, GPU execution and surface excluded","stage timers and profiling overhead included; geometry setup excluded"]});
    std::fs::write(
        path.with_extension("json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    Ok(())
}
