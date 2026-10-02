//! GPU readback proves occlusion, reveal, transparency and unchanged world depth.
use glam::{Mat4, Vec3};
use voxy_render::{GraphicsOptions, SceneDepthMode, SceneDraw, SceneMesh, SceneRenderer};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, queue) = request_gpu()?;
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let white = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let mut shell = renderer.upload_mesh(&device, &SceneMesh::quad([0.0, 0.0, 1.0, 0.5]))?;
    let mut internal = renderer.upload_mesh(&device, &SceneMesh::quad([1.0, 0.0, 0.0, 1.0]))?;
    let shell_transform =
        renderer.create_transform(&device, Mat4::from_translation(Vec3::new(0.0, 0.0, 0.2)))?;
    let internal_transform =
        renderer.create_transform(&device, Mat4::from_translation(Vec3::new(0.0, 0.0, 0.8)))?;
    let color = target(&device, wgpu::TextureFormat::Rgba8Unorm);
    let depth = target(&device, wgpu::TextureFormat::Depth32Float);
    let ui_geometry = renderer.upload_mesh(&device, &SceneMesh::quad([0.0, 1.0, 0.0, 0.5]))?;
    let isolated_depth = target(&device, wgpu::TextureFormat::Depth32Float);
    let mut far_internal = renderer.upload_mesh(&device, &SceneMesh::quad([0.0, 1.0, 0.0, 1.0]))?;
    far_internal.set_depth_mode(SceneDepthMode::Xray);
    let far_transform =
        renderer.create_transform(&device, Mat4::from_translation(Vec3::new(0.0, 0.0, 0.9)))?;
    let readback = readback_buffer(&device);
    for masked in [false, true] {
        for ui in [false, true] {
            for isolated in [false, true] {
                for (shell_mode, internal_mode, expected, expected_depth) in scenarios() {
                    let masked = masked && isolated && internal_mode == SceneDepthMode::Xray;
                    internal.update(
                        &queue,
                        &SceneMesh::quad([1.0, 0.0, 0.0, if masked { 0.0 } else { 1.0 }]),
                    )?;
                    let expected = if masked { [0, 255, 0, 255] } else { expected };
                    shell.set_depth_mode(shell_mode);
                    internal.set_depth_mode(internal_mode);
                    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    let mut draws = vec![
                        SceneDraw {
                            geometry: &shell,
                            texture: &white,
                            transform: &shell_transform,
                            overlay: false,
                        },
                        SceneDraw {
                            geometry: &internal,
                            texture: &white,
                            transform: &internal_transform,
                            overlay: false,
                        },
                    ];
                    let expected = if ui {
                        // Deliberately submit UI first; it must still appear over internals.
                        draws.insert(
                            0,
                            SceneDraw {
                                geometry: &ui_geometry,
                                texture: &white,
                                transform: &far_transform,
                                overlay: true,
                            },
                        );
                        [expected[0] / 2, expected[1] / 2 + 128, expected[2] / 2, 255]
                    } else {
                        expected
                    };
                    if isolated && internal_mode == SceneDepthMode::Xray {
                        // Later farther geometry must not overwrite the nearer red interior.
                        draws.push(SceneDraw {
                            geometry: &far_internal,
                            texture: &white,
                            transform: &far_transform,
                            overlay: false,
                        });
                    }
                    encode_scene(
                        &renderer,
                        &mut encoder,
                        [&color, &depth, &isolated_depth],
                        isolated,
                        &draws,
                    );
                    verify_pixels(
                        &device,
                        &queue,
                        encoder,
                        &color,
                        &depth,
                        &readback,
                        (expected, expected_depth),
                    )?;
                    if let Some(error) = pollster::block_on(scope.pop()) {
                        return Err(error.into());
                    }
                }
            }
        }
    }
    println!(
        "XRAY PASS: normal occlusion, through-body reveal, shell blending, isolated self-occlusion, unchanged world depth, UI composed last, masked pixels do not occlude"
    );
    Ok(())
}
fn readback_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

fn request_gpu() -> Result<(wgpu::Device, wgpu::Queue), Box<dyn std::error::Error>> {
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    println!("X-ray GPU: {:?}", adapter.get_info());
    Ok((device, queue))
}

fn encode_scene(
    renderer: &SceneRenderer,
    encoder: &mut wgpu::CommandEncoder,
    targets: [&wgpu::Texture; 3],
    isolated: bool,
    draws: &[SceneDraw<'_>],
) {
    let [color, depth, isolated_depth] = targets;
    if isolated {
        renderer.encode_with_xray_depth(
            encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            &isolated_depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            draws,
        );
    } else {
        renderer.encode(
            encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            draws,
        );
    }
}

type Scenario = (SceneDepthMode, SceneDepthMode, [u8; 4], f32);
fn scenarios() -> [Scenario; 4] {
    [
        (
            SceneDepthMode::Opaque,
            SceneDepthMode::Opaque,
            [0, 0, 128, 255],
            0.2,
        ),
        (
            SceneDepthMode::Opaque,
            SceneDepthMode::Xray,
            [255, 0, 0, 255],
            0.2,
        ),
        (
            SceneDepthMode::Transparent,
            SceneDepthMode::Opaque,
            [128, 0, 128, 255],
            0.8,
        ),
        (
            SceneDepthMode::Transparent,
            SceneDepthMode::Xray,
            [255, 0, 0, 255],
            1.0,
        ),
    ]
}
fn verify_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    color: &wgpu::Texture,
    depth: &wgpu::Texture,
    readback: &wgpu::Buffer,
    expected_state: ([u8; 4], f32),
) -> Result<(), Box<dyn std::error::Error>> {
    let (expected, expected_depth) = expected_state;
    let copy = |encoder: &mut wgpu::CommandEncoder, texture: &wgpu::Texture, aspect| {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            texture.size(),
        );
    };
    copy(&mut encoder, color, wgpu::TextureAspect::All);
    queue.submit([encoder.finish()]);
    read(device, readback)?;
    let bytes = readback.slice(..).get_mapped_range()?;
    let pixel = &bytes[32 * 256 + 32 * 4..32 * 256 + 32 * 4 + 4];
    assert!(
        pixel.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
        "pixel {pixel:?}, expected {expected:?}"
    );
    drop(bytes);
    readback.unmap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    copy(&mut encoder, depth, wgpu::TextureAspect::DepthOnly);
    queue.submit([encoder.finish()]);
    read(device, readback)?;
    let bytes = readback.slice(..).get_mapped_range()?;
    let offset = 32 * 256 + 32 * 4;
    let actual_depth = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
    assert!(
        (actual_depth - expected_depth).abs() < 1e-5,
        "unexpected depth {actual_depth}"
    );
    drop(bytes);
    readback.unmap();

    Ok(())
}

fn read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    Ok(())
}
fn target(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
