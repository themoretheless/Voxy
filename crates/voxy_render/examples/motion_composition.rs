use glam::{Mat4, Vec3};
use voxy_render::{
    DepthMotionPass, PreviousPositionVertex, RasterMotionPass, RayReconstructionGuides,
    ReconstructionGuideMesh, ReconstructionGuidePass, ReconstructionGuideVertex,
};
#[allow(clippy::too_many_lines)]
fn probe(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let guides = RayReconstructionGuides::new(device, adapter, 4, 4)?;
    let pass = ReconstructionGuidePass::for_guides(device, &guides);
    let inputs = pass.primary_inputs(device, Mat4::IDENTITY, [0.0, 0.0, 3.0], &guides)?;
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("motion composition depth"),
        size: guides.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let positions = [
        [-1.0, 0.0, 0.5],
        [1.0, 0.0, 0.5],
        [1.0, 1.0, 0.5],
        [-1.0, 0.0, 0.5],
        [1.0, 1.0, 0.5],
        [-1.0, 1.0, 0.5],
    ];
    let vertices = positions
        .map(|p| ReconstructionGuideVertex::new(p, [0.0, 0.0, 1.0], [0.5; 3], 0.0, 0.0))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mesh = ReconstructionGuideMesh::upload(device, &vertices)?;
    let cameras = [
        Mat4::IDENTITY,
        Mat4::from_translation(Vec3::new(-0.2, 0.0, 0.0)),
    ];
    let base = DepthMotionPass::new(device, &depth, cameras, 1.0, false)?;
    let left = [
        [-1.0, 0.0, 0.5],
        [0.0, 0.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, 0.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, 1.0, 0.5],
    ]
    .map(|current| PreviousPositionVertex {
        current,
        previous: [current[0] + 0.4, current[1], current[2]],
    });
    let overlay = RasterMotionPass::new(device, &depth, cameras, &left, false)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("motion composition readback"),
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
    base.encode(&mut encoder);
    overlay.encode_over(&mut encoder, base.output())?;
    encoder.copy_texture_to_buffer(
        base.output().as_image_copy(),
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
            let expected = if row >= 2 {
                0.0
            } else if column < 2 {
                0.1
            } else {
                -0.1
            };
            assert!(x.is_finite() && (x - expected).abs() < 0.0001 && y.abs() < 0.0001);
        }
    }
    drop(bytes);
    readback.unmap();
    println!(
        "MOTION COMPOSITION PASS: moving geometry overwrite, preserved static motion, zero background"
    );
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        probe(&device, &adapter, &queue)
    })
}
