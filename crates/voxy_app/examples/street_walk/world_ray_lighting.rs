//! Opt-in complete original street geometry -> shared bounded world-query owner.
#[path = "vertex_cache_capture.rs"]
mod vertex_cache_capture;
use std::{num::NonZeroU32, sync::Arc};
use voxy_render::{ExactWorldNearest, ExactWorldOptions, RayScene, archive_scene::*};

pub fn enabled() -> bool {
    std::env::var("VOXY_STREET_WORLD_RAY_FRAME").as_deref() == Ok("1")
}

#[allow(unsafe_code)]
pub fn experimental_features() -> Result<wgpu::ExperimentalFeatures, Box<dyn std::error::Error>> {
    if !enabled() {
        return Ok(wgpu::ExperimentalFeatures::disabled());
    }
    if !std::env::args().any(|a| a == "--experimental-ray-query") {
        return Err("world ray frame requires --experimental-ray-query".into());
    }
    // SAFETY: this explicitly selected experimental ray-query run uses validated
    // archive geometry. The caller opts into wgpu's experimental backend contract.
    Ok(unsafe { wgpu::ExperimentalFeatures::enabled() })
}

fn tile_size() -> Result<[u32; 2], Box<dyn std::error::Error>> {
    let dimension = |name: &str, default: u32| -> Result<u32, Box<dyn std::error::Error>> {
        Ok(std::env::var(name)
            .ok()
            .map(|s| s.parse::<NonZeroU32>())
            .transpose()?
            .map_or(default, NonZeroU32::get))
    };
    let size = [
        dimension("VOXY_STREET_WORLD_RAY_TILE_WIDTH", 512)?,
        dimension("VOXY_STREET_WORLD_RAY_TILE_HEIGHT", 256)?,
    ];
    size[0]
        .checked_mul(size[1])
        .ok_or("world ray tile capacity overflow")?;
    Ok(size)
}

pub fn prepare(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    geometry: &[ArchiveMeshSource],
    materials: &[ArchiveMaterialSource],
    images: usize,
) -> Result<Option<Arc<ExactWorldNearest>>, Box<dyn std::error::Error>> {
    if !enabled() {
        return Ok(None);
    }
    let tile = tile_size()?;
    let hit_position =
        std::env::var("VOXY_STREET_WORLD_RAY_POSITION_CONSUMER").as_deref() == Ok("1");
    let exact_workers = std::env::var("VOXY_STREET_WORLD_RAY_EXACT_WORKERS")
        .ok()
        .map(|s| s.parse::<NonZeroU32>())
        .transpose()?
        .unwrap_or(NonZeroU32::new(128).unwrap());
    let exact_group = std::env::var("VOXY_STREET_WORLD_RAY_EXACT_GROUP")
        .ok()
        .map(|s| s.parse::<NonZeroU32>())
        .transpose()?;
    let started = std::time::Instant::now();
    let scene = Arc::new(
        if std::env::var("VOXY_STREET_WORLD_RAY_PRIMITIVE_GROUPS").as_deref() == Ok("1") {
            RayScene::from_archive_candidates_with_primitive_groups(device, geometry, 0.)?
        } else {
            RayScene::from_archive_candidates(device, geometry, 0.)?
        },
    );
    let job = Arc::new(ExactWorldNearest::with_options(
        device,
        scene.clone(),
        materials,
        images,
        tile[0] * tile[1],
        ExactWorldOptions {
            hit_position,
            certified_ratio_seed: std::env::var("VOXY_STREET_WORLD_RAY_CERTIFIED_RATIO_SEED")
                .as_deref()
                == Ok("1"),
            exact_vertex_cache: std::env::var("VOXY_STREET_WORLD_RAY_EXACT_VERTEX_CACHE")
                .as_deref()
                == Ok("1"),
            wide_source_nodes: std::env::var("VOXY_STREET_WORLD_RAY_WIDE_NODES").as_deref() == Ok("1"),
            skip_zero_prefix_multiply: std::env::var(
                "VOXY_STREET_WORLD_RAY_SKIP_ZERO_PREFIX_MULTIPLY",
            )
            .as_deref()
                == Ok("1"),
            cached_relative_interval: std::env::var(
                "VOXY_STREET_WORLD_RAY_CACHED_RELATIVE_INTERVAL",
            )
            .as_deref()
                == Ok("1"),
            fused_source_affine: std::env::var("VOXY_STREET_WORLD_RAY_FUSED_SOURCE_AFFINE")
                .as_deref()
                == Ok("1"),
            exact_winner_arena_slots: std::env::var("VOXY_STREET_WORLD_RAY_WINNER_SLOTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .and_then(std::num::NonZeroU32::new),
            exact_winner_worker_limit: std::env::var("VOXY_STREET_WORLD_RAY_WINNER_WORKERS")
                .ok()
                .and_then(|v| v.parse().ok())
                .and_then(std::num::NonZeroU32::new),
            geometry_bounds_cache: std::env::var("VOXY_STREET_WORLD_RAY_GEOMETRY_BOUNDS_CACHE")
                .as_deref()
                == Ok("1"),
            geometry_bounds_check: std::env::var("VOXY_STREET_WORLD_RAY_GEOMETRY_BOUNDS_CHECK")
                .as_deref()
                == Ok("1"),
            history_position_admission: std::env::var(
                "VOXY_STREET_WORLD_RAY_HISTORY_POSITION_ADMISSION",
            )
            .as_deref()
                == Ok("1"),
            certify_order: hit_position,
            query_kind: if hit_position {
                voxy_render::WorldRayQueryKind::MaterialSurface
            } else if std::env::var("VOXY_STREET_WORLD_RAY_HISTORY_ADMISSION").as_deref() == Ok("1")
            {
                voxy_render::WorldRayQueryKind::HistoryAdmission
            } else {
                voxy_render::WorldRayQueryKind::Nearest
            },
            history_any_hit: !hit_position
                && std::env::var("VOXY_STREET_WORLD_RAY_ANY_HIT_ADMISSION").as_deref() == Ok("1"),
            native_seed: std::env::var("VOXY_STREET_WORLD_RAY_SEPARATE_NATIVE_SEED").as_deref()
                == Ok("1"),
            traversal_work_stats: std::env::var("VOXY_STREET_WORLD_RAY_WORK_STATS").as_deref()
                == Ok("1"),
            node_reciprocal: std::env::var("VOXY_STREET_WORLD_RAY_NODE_RECIPROCAL").as_deref()
                == Ok("1"),
            native_source_witness: !hit_position
                && std::env::var("VOXY_STREET_WORLD_RAY_NATIVE_SOURCE_WITNESS").as_deref()
                    == Ok("1"),
            native_triangle_hint: std::env::var("VOXY_STREET_WORLD_RAY_NATIVE_TRIANGLE_HINT")
                .as_deref()
                == Ok("1"),
            separate_native_seed: std::env::var("VOXY_STREET_WORLD_RAY_SEPARATE_NATIVE_SEED")
                .as_deref()
                == Ok("1"),
            guarded_input: true,
            certify_visibility: true,
            defer_nearest: true,
            exact_worker_limit: Some(exact_workers),
            exact_group_size: exact_group,
            ..Default::default()
        },
    )?);
    let mut encoder = device.create_command_encoder(&Default::default());
    job.build_scene(&mut encoder);
    let submission = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    eprintln!(
        "STREET WORLD SOURCE ready meshes={} instances={} exact_workers={} source_simplified=false preparation_seconds={:.3}",
        geometry.len(),
        geometry.iter().map(|m| m.instances.len()).sum::<usize>(),
        job.exact_worker_count(),
        started.elapsed().as_secs_f64()
    );
    eprintln!(
        "STREET WORLD consumer_any_hit_admission={} native_source_witness={} node_reciprocal={}",
        job.history_any_hit_enabled(),
        job.native_source_witness_enabled(),
        job.node_reciprocal_enabled()
    );
    eprintln!(
        "STREET WORLD exact_position_consumer={} material_surface_policy={} exact_workers={} exact_group={}",
        hit_position,
        hit_position,
        job.exact_worker_count(),
        job.exact_workgroup_size()
    );
    eprintln!("STREET WORLD SCHEDULING {}", serde_json::json!({
        "kind": format!("{:?}",job.query_kind()),
        "wide_source_nodes": job.wide_source_nodes_enabled(),
        "wide_source_block_blake3": job.wide_source_digest(),
        "source_simplified": false
    }));
    if std::env::var_os("VOXY_STREET_WORLD_RAY_WINNER_SLOTS").is_some()
        || std::env::var_os("VOXY_STREET_WORLD_RAY_WINNER_WORKERS").is_some()
    {
        if let Some((slots, workers)) = job.exact_winner_workspace() {
            eprintln!(
                "STREET WORLD winner_slots={slots} winner_worker_cap={workers} full_slots=512 full_workers={}",
                job.exact_worker_count()
            );
        }
    }
    if let Some(buffer) = job.geometry_bounds_buffer() {
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("prepared source geometry bounds header"),
            size: 64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(buffer, 0, &read, 0, 64);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let bytes = read.slice(..).get_mapped_range()?.to_vec();
        read.unmap();
        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        if words[0] != 1 || words[5] != 0 || (words[7] == 1 && words[6] != words[2]) {
            return Err("source geometry bounds cache did not pass preparation".into());
        }
        eprintln!(
            "STREET WORLD geometry_bounds_header={}",
            serde_json::json!({"ready":words[0],"slots":words[1],"triangles":words[2],"mismatches":words[5],"checked":words[6],"check":words[7],"geometry_revision":[words[8],words[9]]})
        );
    }
    eprintln!(
        "STREET WORLD geometry_bounds_bytes={} geometry_bounds_check={}",
        job.geometry_bounds_bytes(),
        std::env::var("VOXY_STREET_WORLD_RAY_GEOMETRY_BOUNDS_CHECK").as_deref() == Ok("1")
    );
    vertex_cache_capture::capture(device, queue, &job, &scene)?;
    Ok(Some(job))
}

pub fn connect(
    renderer: &mut ArchiveSceneRenderer,
    job: Option<Arc<ExactWorldNearest>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(job) = job {
        let tile = tile_size()?;
        let capacity = job.capacity();
        if capacity != tile[0] * tile[1] {
            return Err("world ray tile capacity changed after preparing its owner".into());
        }
        renderer.set_world_ray_frame(job, tile)?;
        eprintln!(
            "STREET WORLD FRAME connected=true regions={} tile={}x{} ray_capacity={} shared_query_owner=true original_full_frame_DDA_skipped=true",
            renderer.world_ray_region_count(),
            tile[0],
            tile[1],
            capacity
        );
    }
    Ok(())
}
