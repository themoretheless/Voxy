//! Diagnostic readback only. No CPU expected coordinate enters GPU inputs.
use std::path::Path;
use voxy_render::{RayScene, WorldRayQueryJob, WorldRayQueryKind};
fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    offset: u64,
    size: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exact vertex cache diagnostic readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, offset, &staging, 0, size);
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
    let bytes = staging.slice(..).get_mapped_range()?.to_vec();
    staging.unmap();
    Ok(bytes)
}
pub(super) fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    job: &WorldRayQueryJob,
    scene: &RayScene,
    root: Option<&Path>,
    kind: WorldRayQueryKind,
    order: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(buffer) = job.exact_vertex_cache_buffer() else {
        return Ok(());
    };
    let header = job
        .exact_vertex_cache_header()
        .ok_or("missing vertex cache header")?;
    let geometry_unready =
        std::env::var("VOXY_RAY_MATERIAL_SIDES_GEOMETRY_NOT_READY").as_deref() == Ok("1");
    let not_ready =
        std::env::var("VOXY_RAY_MATERIAL_SIDES_VERTEX_CACHE_NOT_READY").as_deref() == Ok("1");
    let mut encoder = device.create_command_encoder(&Default::default());
    // Initial source copying is required even when testing an unready cache.
    if job.geometry_bounds_buffer().is_none() || geometry_unready {
        job.build_scene(&mut encoder);
    }
    if geometry_unready {
        if let Some(bounds) = job.geometry_bounds_buffer() {
            encoder.clear_buffer(bounds, 0, Some(4));
        }
    }
    if not_ready {
        encoder.clear_buffer(buffer, u64::from(header) * 4, Some(4));
    }
    queue.submit([encoder.finish()]);
    let bytes = read(device, queue, buffer, 0, buffer.size())?;
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let h = &words[header as usize..header as usize + 16];
    assert_eq!(
        (h[0], h[4], h[5], h[6]),
        (u32::from(!not_ready), 12, 0, h[2])
    );
    let revision = scene.geometry_revision();
    assert_eq!([h[8], h[9]], [revision as u32, (revision >> 32) as u32]);
    let sources = scene
        .archive_candidate_source_bindings()
        .ok_or("missing original source bindings")?;
    let wgpu::BindingResource::Buffer(original) = &sources[0] else {
        return Err("source vertex buffer missing".into());
    };
    let prefix = read(
        device,
        queue,
        original.buffer,
        original.offset,
        u64::from(header) * 4,
    )?;
    assert_eq!(&bytes[..header as usize * 4], prefix);
    if let Some(root) = root {
        let name = format!("vertex-cache-{kind:?}-order-{order}");
        std::fs::write(root.join(format!("{name}.bin")), &bytes)?;
        std::fs::write(root.join(format!("{name}-source-prefix.bin")), &prefix)?;
        std::fs::write(
            root.join(format!("{name}.json")),
            serde_json::to_vec_pretty(
                &serde_json::json!({"schema":"voxy-exact-vertex-cache-capture-v1","buffer_file":format!("{name}.bin"),"source_prefix_file":format!("{name}-source-prefix.bin"),"header_word":header,"ready_cleared_for_fallback_test":not_ready,"geometry_revision":[revision as u32,(revision>>32)as u32],"source_prefix_bit_identical":true,"GPU_check_mismatches":h[5],"GPU_checked_vertices":h[6],"CPU_gold_uploaded":false,"FPS_qualified":false}),
            )?,
        )?;
    }
    println!(
        "EXACT VERTEX CACHE ready={} checked_all={} mismatches={} bytes={} original_source_prefix_identical=true",
        h[0],
        h[6],
        h[5],
        buffer.size()
    );
    Ok(())
}
