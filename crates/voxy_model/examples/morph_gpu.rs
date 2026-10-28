//! Headless GPU correctness for original imported morph primitives, not a benchmark.
use std::sync::Arc;
use voxy_model::{ModelLimits, MorphModelAsset};
use voxy_render::{SceneRenderer, SceneSkinner};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: morph_gpu model.glb")?;
    let bytes = std::fs::read(path)?;
    let asset = MorphModelAsset::parse(&bytes, &[], ModelLimits::default())?;
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let adapter_info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let skinner = SceneSkinner::new(&renderer)?;
    let idle = asset
        .animations()
        .iter()
        .position(|clip| clip.name() == "Idle")
        .ok_or("Idle clip missing")?;
    let mut animator = asset.animator(idle)?;
    let initial = animator.advance_wall(asset.skeleton(), 0.)?;
    let initial_weights = asset.frame_weights(&initial)?;
    let mut live = 0;
    let mut sources = Vec::new();
    for primitive in 0..asset.primitive_count() {
        if let Some(source) = asset.morph_source(primitive)? {
            let source = skinner.upload_morph_source(Arc::new(source), live, 64 << 20)?;
            live += source.allocation_bytes();
            let gpu = skinner.create_morph_instance(
                &renderer,
                source.clone(),
                &initial.skin_matrices,
                &initial_weights[primitive],
                [1.; 4],
                live,
                64 << 20,
            )?;
            live += gpu.allocation_bytes();
            sources.push((primitive, gpu));
        }
    }
    if sources.len() != asset.morph_primitive_count() {
        return Err("missing imported morph primitive".into());
    }
    let mut checked_vertices = 0;
    let mut max_position_error = 0f64;
    let mut frames = 0;
    for clip_name in ["Idle", "Walking", "Jump"] {
        let clip = asset
            .animations()
            .iter()
            .find(|clip| clip.name() == clip_name)
            .ok_or("source clip missing")?;
        animator.transition_to(clip.clone(), 0.2)?;
        for step in 0..4 {
            let mut frame = animator.advance_wall(asset.skeleton(), 1. / 60.)?;
            // Explicit authoring override; source channels/GLB are unchanged.
            frame.properties = voxy_animation::PropertySet::new(
                frame
                    .properties
                    .entries()
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _))| {
                        (
                            name.clone(),
                            voxy_animation::PropertyValue::Float([0.5, -0.25, 1.5][(i + step) % 3]),
                        )
                    })
                    .collect(),
            )?;
            let weights = asset.frame_weights(&frame)?;
            let expected = asset.scene_meshes(&frame)?;
            let poses = sources
                .iter()
                .map(|(primitive, gpu)| {
                    skinner.prepare_morph_pose(gpu, &frame.skin_matrices, &weights[*primitive])
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut encoder = device.create_command_encoder(&Default::default());
            skinner.encode_prepared_poses_batch(&queue, &mut encoder, &poses)?;
            let sizes = sources
                .iter()
                .map(|(primitive, _)| expected[*primitive].vertices().len() as u64 * 36)
                .collect::<Vec<_>>();
            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("original morph readback"),
                size: sizes.iter().sum(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut offset = 0;
            for ((_, gpu), size) in sources.iter().zip(&sizes) {
                encoder.copy_buffer_to_buffer(&gpu.geometry().vertices, 0, &staging, offset, *size);
                offset += size;
            }
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            staging
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
            device.poll(wgpu::PollType::wait_indefinitely())?;
            rx.recv()??;
            let mapped = staging.slice(..).get_mapped_range()?;
            let actual: &[f32] = bytemuck::cast_slice(&mapped);
            let mut first = 0;
            for ((primitive, _), size) in sources.iter().zip(&sizes) {
                let end = first + *size as usize / 4;
                for (gpu, cpu) in actual[first..end]
                    .chunks_exact(9)
                    .zip(expected[*primitive].vertices())
                {
                    for axis in 0..3 {
                        let error = (f64::from(gpu[axis]) - f64::from(cpu.position[axis])).abs();
                        if !error.is_finite() || error > 1e-5 {
                            return Err(format!(
                                "posed GPU mismatch primitive {primitive}: {error}"
                            )
                            .into());
                        }
                        max_position_error = max_position_error.max(error);
                    }
                    checked_vertices += 1;
                }
                first = end;
            }
            drop(mapped);
            staging.unmap();
            frames += 1;
        }
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "{}",
        serde_json::json!({"scope":"headless_gpu_imported_morph_correctness","adapter":adapter_info.name,"backend":format!("{:?}",adapter_info.backend),"source_primitives":asset.primitive_count(),"gpu_morph_primitives":sources.len(),"frames":frames,"checked_vertices":checked_vertices,"max_world_position_error":max_position_error,"explicit_weight_overrides":true,"source_channels_unchanged":true,"full_model_image_verified":false,"editor_integrated":false,"fps_qualified":false})
    );
    Ok(())
}
