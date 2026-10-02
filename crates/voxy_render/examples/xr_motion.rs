//! Headless stereo temporal proof; does not emulate headset presentation.
use glam::{Quat, Vec3};
use voxy_render::{
    DepthMotionPass, RayReconstructionGuides, ReconstructionGuideMesh, ReconstructionGuidePass,
    ReconstructionGuideVertex, XrFov, XrMotionHistory, XrView,
};

fn eyes(separation: f32) -> [XrView; 2] {
    std::array::from_fn(|eye| XrView {
        position: Vec3::new(if eye == 0 { -separation } else { separation }, 0.0, 0.0),
        orientation: Quat::IDENTITY,
        fov: XrFov {
            left: if eye == 0 { -0.7 } else { -0.9 },
            right: if eye == 0 { 0.9 } else { 0.7 },
            down: -0.6,
            up: 0.8,
        },
        position_valid: true,
        orientation_valid: true,
    })
}

#[allow(clippy::too_many_lines)]
fn verify_eye(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
    view: XrView,
    matrices: voxy_render::MotionMatrices,
    expected: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let guides = RayReconstructionGuides::new(device, adapter, 4, 4)?;
    let pass = ReconstructionGuidePass::for_guides(device, &guides);
    let inputs =
        pass.primary_inputs(device, matrices.current, view.position.to_array(), &guides)?;
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stereo motion depth"),
        size: guides.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    // A front-facing world plane covers both asymmetric frusta at distance 2.
    let vertices = [
        [-10.0, -10.0, -2.0],
        [10.0, -10.0, -2.0],
        [10.0, 10.0, -2.0],
        [-10.0, -10.0, -2.0],
        [10.0, 10.0, -2.0],
        [-10.0, 10.0, -2.0],
    ]
    .map(|p| ReconstructionGuideVertex::new(p, [0.0, 0.0, 1.0], [0.5; 3], 0.0, 0.0))
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    let mesh = ReconstructionGuideMesh::upload(device, &vertices)?;
    let motion = DepthMotionPass::new(
        device,
        &depth,
        [matrices.current, matrices.previous],
        1.0,
        !matrices.history_valid,
    )?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("stereo motion readback"),
        size: 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    pass.encode(
        &mut encoder,
        &guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        &inputs,
        &[&mesh],
    )?;
    motion.encode(&mut encoder);
    encoder.copy_texture_to_buffer(
        motion.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        guides.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = readback.slice(..).get_mapped_range()?;
    for row in 0..4 {
        for column in 0..4 {
            let offset = row * 256 + column * 4;
            let x = half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                .to_f32();
            let y = half::f16::from_bits(u16::from_le_bytes(
                bytes[offset + 2..offset + 4].try_into()?,
            ))
            .to_f32();
            assert!(
                x.is_finite() && (x - expected).abs() < 0.0001 && y.abs() < 0.0001,
                "stereo pixel ({column},{row}): ({x},{y}), expected ({expected},0)"
            );
        }
    }
    drop(bytes);
    readback.unmap();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let backend = match std::env::var("VOXY_XR_BACKEND").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("auto") => voxy_render::GraphicsBackend::Auto,
            Ok("metal") => voxy_render::GraphicsBackend::Metal,
            Ok("vulkan") => voxy_render::GraphicsBackend::Vulkan,
            Ok("gl") => voxy_render::GraphicsBackend::OpenGl,
            Ok("dx12") => voxy_render::GraphicsBackend::DirectX12,
            _ => return Err("VOXY_XR_BACKEND expects auto|metal|vulkan|gl|dx12".into()),
        };
        let instance = voxy_render::GraphicsOptions {
            backend,
            ..Default::default()
        }
        .create_instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        println!("XR motion adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        let mut history = XrMotionHistory::default();
        let first = eyes(0.03);
        let initial = history.prepare(first, 0.1, 100.0)?;
        for eye in 0..2 {
            verify_eye(&device, &adapter, &queue, first[eye], initial[eye], 0.0)?;
        }
        history.presented(first, 0.1, 100.0)?;
        // A skipped frame must not replace the last submitted eye poses.
        history.prepare(eyes(0.5), 0.1, 100.0)?;
        let current = eyes(0.04);
        let matrices = history.prepare(current, 0.1, 100.0)?;
        for eye in 0..2 {
            let width = current[eye].fov.right.tan() - current[eye].fov.left.tan();
            let delta = current[eye].position.x - first[eye].position.x;
            verify_eye(
                &device,
                &adapter,
                &queue,
                current[eye],
                matrices[eye],
                delta / (2.0 * width),
            )?;
        }
        let mut lost = current;
        lost[1].orientation_valid = false;
        assert!(history.prepare(lost, 0.1, 100.0).is_err());
        let recovered = history.prepare(current, 0.1, 100.0)?;
        for eye in 0..2 {
            verify_eye(&device, &adapter, &queue, current[eye], recovered[eye], 0.0)?;
        }
        history.presented(current, 0.1, 100.0)?;
        for (near, far, separation) in [(0.2, 100.0, 0.08), (0.2, 1000.0, 0.12)] {
            let changed = eyes(separation);
            let reset = history.prepare(changed, near, far)?;
            for eye in 0..2 {
                assert!(!reset[eye].history_valid);
                verify_eye(&device, &adapter, &queue, changed[eye], reset[eye], 0.0)?;
            }
            history.presented(changed, near, far)?;
        }
        history.reset();
        let teleported = eyes(0.3);
        let reset = history.prepare(teleported, 0.2, 1000.0)?;
        for eye in 0..2 {
            assert!(!reset[eye].history_valid);
            verify_eye(&device, &adapter, &queue, teleported[eye], reset[eye], 0.0)?;
        }
        println!(
            "XR MOTION PASS: asymmetric eyes, opposite motion, skipped frame, tracking recovery, clipping changes, teleport"
        );
        Ok(())
    })
}
