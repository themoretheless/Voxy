//! One-time diagnostic source/cache capture; never an FPS measurement.
use std::{io::Write, path::Path};
use voxy_render::{RayScene, WorldRayQueryJob};
fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &wgpu::Buffer,
    offset: u64,
    size: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exact affine vertex cache readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, offset, &staging, 0, size);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    rx.recv()??;
    let data = staging.slice(..).get_mapped_range()?.to_vec();
    staging.unmap();
    Ok(data)
}
fn write(root: &Path, name: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(data)?;
    file.sync_all()?;
    Ok(())
}
pub(super) fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    job: &WorldRayQueryJob,
    scene: &RayScene,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(buffer) = job.exact_vertex_cache_buffer() else {
        return Ok(());
    };
    let header = job
        .exact_vertex_cache_header()
        .ok_or("missing exact vertex cache header")?;
    let bytes = read(device, queue, buffer, u64::from(header) * 4, 64)?;
    let h: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    if h[0] != 1 || h[4] != 12 || h[5] != 0 || h[6] != h[2] {
        return Err("exact vertex cache did not pass complete preparation/check".into());
    }
    let revision = scene.geometry_revision();
    if [h[8], h[9]] != [revision as u32, (revision >> 32) as u32] {
        return Err("exact vertex cache source owner revision changed".into());
    }
    eprintln!(
        "STREET WORLD exact_vertex_cache_header={}",
        serde_json::json!({"ready":h[0],"slots":h[1],"world_vertices":h[2],"checked_all":h[6],"mismatches":h[5],"bytes":buffer.size(),"geometry_revision":[h[8],h[9]]})
    );
    if let Some(directory) = std::env::var_os("VOXY_STREET_WORLD_RAY_VERTEX_CACHE_DUMP") {
        let root = Path::new(&directory);
        std::fs::create_dir(root)?;
        let data = read(device, queue, buffer, 0, buffer.size())?;
        let bindings = scene
            .archive_candidate_source_bindings()
            .ok_or("missing source bindings")?;
        let wgpu::BindingResource::Buffer(vertices) = &bindings[0] else {
            return Err("missing source vertex binding".into());
        };
        let prefix = read(
            device,
            queue,
            vertices.buffer,
            vertices.offset,
            u64::from(header) * 4,
        )?;
        if data[..header as usize * 4] != prefix {
            return Err("exact vertex cache changed original source prefix".into());
        }
        let wgpu::BindingResource::Buffer(instances) = &bindings[2] else {
            return Err("missing source instance binding".into());
        };
        let size = instances.size.map_or(
            instances.buffer.size() - instances.offset,
            std::num::NonZeroU64::get,
        );
        let instance_bytes = read(device, queue, instances.buffer, instances.offset, size)?;
        write(root, "vertex-cache-full-street.bin", &data)?;
        write(root, "vertex-cache-full-street-source-prefix.bin", &prefix)?;
        write(root, "original-source-instances.bin", &instance_bytes)?;
        let description = serde_json::json!({"schema":"voxy-exact-vertex-cache-capture-v1","buffer_file":"vertex-cache-full-street.bin","source_prefix_file":"vertex-cache-full-street-source-prefix.bin","source_instances_file":"original-source-instances.bin","header_word":header,"ready_cleared_for_fallback_test":false,"geometry_revision":[h[8],h[9]],"CPU_gold_uploaded":false,"full_scene_source_geometry":true,"capture_before_ray_queries":true,"performance_qualified":false});
        write(
            root,
            "vertex-cache-full-street.json",
            &serde_json::to_vec_pretty(&description)?,
        )?;
    }
    Ok(())
}
