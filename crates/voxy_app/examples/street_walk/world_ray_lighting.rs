//! Opt-in complete original street geometry -> shared bounded world-query owner.
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

fn tile_size()->Result<[u32;2],Box<dyn std::error::Error>> {
    let dimension=|name:&str,default:u32|->Result<u32,Box<dyn std::error::Error>> {Ok(std::env::var(name).ok().map(|s|s.parse::<NonZeroU32>()).transpose()?.map_or(default,NonZeroU32::get))};
    let size=[dimension("VOXY_STREET_WORLD_RAY_TILE_WIDTH",512)?,dimension("VOXY_STREET_WORLD_RAY_TILE_HEIGHT",256)?];
    size[0].checked_mul(size[1]).ok_or("world ray tile capacity overflow")?;Ok(size)
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
    let tile=tile_size()?;
    let started = std::time::Instant::now();
    let scene = Arc::new(if std::env::var("VOXY_STREET_WORLD_RAY_PRIMITIVE_GROUPS").as_deref()==Ok("1") {RayScene::from_archive_candidates_with_primitive_groups(device,geometry,0.)?}else{RayScene::from_archive_candidates(device,geometry,0.)?});
    let job = Arc::new(ExactWorldNearest::with_options(
        device,
        scene,
        materials,
        images,
        tile[0]*tile[1],
        ExactWorldOptions {
            query_kind: if std::env::var("VOXY_STREET_WORLD_RAY_HISTORY_ADMISSION").as_deref()==Ok("1") {voxy_render::WorldRayQueryKind::HistoryAdmission}else{voxy_render::WorldRayQueryKind::Nearest},
            history_any_hit: std::env::var("VOXY_STREET_WORLD_RAY_ANY_HIT_ADMISSION").as_deref()==Ok("1"),
            native_seed: std::env::var("VOXY_STREET_WORLD_RAY_SEPARATE_NATIVE_SEED").as_deref()==Ok("1"),
            traversal_work_stats: std::env::var("VOXY_STREET_WORLD_RAY_WORK_STATS").as_deref()==Ok("1"),
            node_reciprocal: std::env::var("VOXY_STREET_WORLD_RAY_NODE_RECIPROCAL").as_deref()==Ok("1"),
            native_source_witness: std::env::var("VOXY_STREET_WORLD_RAY_NATIVE_SOURCE_WITNESS").as_deref()==Ok("1"),
            separate_native_seed: std::env::var("VOXY_STREET_WORLD_RAY_SEPARATE_NATIVE_SEED").as_deref()==Ok("1"),
            guarded_input: true,
            certify_visibility: true,
            defer_nearest: true,
            exact_worker_limit: NonZeroU32::new(128),
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
    eprintln!("STREET WORLD consumer_any_hit_admission={} native_source_witness={} node_reciprocal={}",job.history_any_hit_enabled(),job.native_source_witness_enabled(),job.node_reciprocal_enabled());
    Ok(Some(job))
}

pub fn connect(
    renderer: &mut ArchiveSceneRenderer,
    job: Option<Arc<ExactWorldNearest>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(job) = job {
        let tile=tile_size()?;let capacity=job.capacity();
        if capacity!=tile[0]*tile[1] {return Err("world ray tile capacity changed after preparing its owner".into());}
        renderer.set_world_ray_frame(job, tile)?;
        eprintln!(
            "STREET WORLD FRAME connected=true regions={} tile={}x{} ray_capacity={} shared_query_owner=true original_full_frame_DDA_skipped=true",
            renderer.world_ray_region_count(),tile[0],tile[1],capacity
        );
    }
    Ok(())
}
