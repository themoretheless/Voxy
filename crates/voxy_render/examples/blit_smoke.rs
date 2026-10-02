//! Exact nearest-sampling and orientation proof on a real GPU.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = smoke_instance()?;
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Blit on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let make = |size, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("blit smoke"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        })
    };
    let source = make(
        2,
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let output = voxy_render::ProcessedColorTarget::new(&device, 4, 4, false)?;
    let target = output.texture();
    if voxy_render::ProcessedColorTarget::new(&device, 0, 4, false).is_ok() {
        return Err("zero-sized output was accepted".into());
    }
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    queue.write_texture(
        source.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        source.size(),
    );
    let blit = voxy_render::TextureBlit::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    blit.encode(
        &device,
        &mut encoder,
        &source.create_view(&wgpu::TextureViewDescriptor::default()),
        &target.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("blit readback"),
        size: 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        target.size(),
    );
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(index),
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    for y in 0..4 {
        for x in 0..4 {
            let expected = ((y / 2) * 2 + x / 2) * 4;
            let actual = y * 256 + x * 4;
            if mapped[actual..actual + 4] != pixels[expected..expected + 4] {
                return Err(format!("blit pixel mismatch at {x},{y}").into());
            }
        }
    }
    drop(mapped);
    buffer.unmap();
    println!("Blit orientation and 2x nearest scaling: all 16 pixels match");
    postprocessing_smoke(&device, &queue)?;
    guide_targets_smoke(&device, &adapter, &queue)?;
    guide_material_smoke(&device, &adapter, &queue)?;
    Ok(())
}

fn guide_material_smoke(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_render::{
        RayReconstructionGuides, ReconstructionGuideMesh, ReconstructionGuidePass,
        ReconstructionGuideVertex, ReconstructionMaterial,
    };
    let guides = RayReconstructionGuides::new(device, adapter, 4, 4)?;
    let descriptor = wgpu::TextureDescriptor {
        label: Some("RR material smoke input"),
        size: guides.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    };
    let distance = device.create_texture(&descriptor);
    queue.write_texture(
        distance.as_image_copy(),
        bytemuck::cast_slice(&[0.25_f32; 16]),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(4),
        },
        distance.size(),
    );
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        ..descriptor.clone()
    });
    let vertices = [[-1.0, -1.0, 0.5], [3.0, -1.0, 0.5], [-1.0, 3.0, 0.5]]
        .map(|position| {
            ReconstructionGuideVertex::new(position, [0.0, 0.0, 1.0], [0.8, 0.4, 0.2], 1.0, 0.5)
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mesh = ReconstructionGuideMesh::upload(device, &vertices)?;
    let pass = ReconstructionGuidePass::for_guides(device, &guides);
    let inputs = pass.inputs(device, glam::Mat4::IDENTITY, [0.0, 0.0, 3.0], &distance)?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    pass.encode(
        &mut encoder,
        &guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        &inputs,
        &[&mesh],
    )?;
    queue.submit([encoder.finish()]);
    let material = ReconstructionMaterial::new([0.8, 0.4, 0.2], 1.0, 0.5)?;
    for (index, source) in [
        guides.normal_roughness(),
        guides.diffuse_albedo(),
        guides.specular_albedo(),
        guides.specular_hit_distance(),
    ]
    .into_iter()
    .enumerate()
    {
        let target = device.create_texture(&wgpu::TextureDescriptor {
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            ..descriptor.clone()
        });
        let blit = voxy_render::TextureBlit::new(device, wgpu::TextureFormat::Rgba8Unorm);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        blit.encode(
            device,
            &mut encoder,
            &source.create_view(&wgpu::TextureViewDescriptor::default()),
            &target.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let pixels = read_overlay_pixels(device, queue, &target, encoder)?;
        check_guide_pixels(index, &pixels, &material)?;
    }
    println!("RR GPU material guides and CPU specular model match all 16 pixels");
    Ok(())
}

fn guide_targets_smoke(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_render::{RayReconstructionGuides, ReconstructionGuideError};
    for format in [
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::R32Float,
    ] {
        println!(
            "RR guide {format:?}: {:?}",
            adapter.get_texture_format_features(format)
        );
    }
    let mut guides = RayReconstructionGuides::new(device, adapter, 4, 4)?;
    assert_eq!(
        guides.resize(device, adapter, 0, 4),
        Err(ReconstructionGuideError::InvalidDimensions)
    );
    assert_eq!(guides.size().width, 4);
    let views = [
        guides.normal_roughness(),
        guides.diffuse_albedo(),
        guides.specular_albedo(),
        guides.specular_hit_distance(),
    ]
    .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let attachments = views.each_ref().map(|view| {
        Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RR four-guide attachment smoke"),
            color_attachments: &attachments,
            ..Default::default()
        });
    }
    let index = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(index),
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    guides.resize(device, adapter, 8, 8)?;
    for texture in [
        guides.normal_roughness(),
        guides.diffuse_albedo(),
        guides.specular_albedo(),
        guides.specular_hit_distance(),
    ] {
        assert_eq!(texture.size(), guides.size());
        assert_eq!(texture.width(), 8);
    }
    println!("RR four-guide MRT allocation/clear and atomic resize passed");
    Ok(())
}

fn tone_mapping_smoke(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    srgb: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_render::{ProcessedColorTarget, TextureBlit};
    let format = output_format(srgb);
    for exposure in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        if TextureBlit::tone_mapped(device, format, exposure).is_some() {
            return Err("invalid tone mapping exposure accepted".into());
        }
    }
    let hdr = ProcessedColorTarget::new(device, 1, 1, true)?;
    let output = ProcessedColorTarget::new(device, 1, 1, false)?;
    let blit = TextureBlit::tone_mapped(device, format, 2.0).ok_or("valid exposure rejected")?;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tone map readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("initialize HDR sample"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: hdr.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 4.0,
                        g: 1.0,
                        b: 0.0,
                        a: 0.5,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    let display = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("display output"),
        size: output.texture().size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let display_view = display.create_view(&wgpu::TextureViewDescriptor::default());
    let target_view = &display_view;
    blit.encode(device, &mut encoder, hdr.view(), target_view);
    encoder.copy_texture_to_buffer(
        display.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        output.texture().size(),
    );
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(index),
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    // HDR (4,1,0) * exposure 2 -> Reinhard (8/9,2/3,0); alpha unchanged.
    let expected = if srgb {
        [242u8, 213, 0, 128]
    } else {
        [227u8, 170, 0, 128]
    };
    for (actual, expected) in mapped[..4].iter().zip(expected) {
        if actual.abs_diff(expected) > 1 {
            return Err(format!("HDR tone map mismatch: {:?}", &mapped[..4]).into());
        }
    }
    drop(mapped);
    buffer.unmap();
    println!("HDR tone mapping, exposure and alpha readback passed (sRGB={srgb})");
    Ok(())
}

fn smoke_instance() -> Result<wgpu::Instance, Box<dyn std::error::Error>> {
    use voxy_render::{GraphicsBackend, GraphicsOptions};
    let backend = match std::env::var("VOXY_BLIT_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => GraphicsBackend::Auto,
        Ok("metal") => GraphicsBackend::Metal,
        Ok("dx12") => GraphicsBackend::DirectX12,
        Ok("vulkan") => GraphicsBackend::Vulkan,
        Ok("gl") => GraphicsBackend::OpenGl,
        _ => return Err("VOXY_BLIT_BACKEND expects auto|metal|dx12|vulkan|gl".into()),
    };
    let options = GraphicsOptions {
        backend,
        ..Default::default()
    };
    if backend == GraphicsBackend::OpenGl {
        let event_loop = winit::event_loop::EventLoop::new()?;
        Ok(options.create_instance_with_display(event_loop.owned_display_handle()))
    } else {
        Ok(options.create_instance())
    }
}

fn overlay_composition_smoke(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    srgb: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_render::{SceneDraw, SceneMesh, SceneRenderer, TextureBlit};
    let format = output_format(srgb);
    let make = |size, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("overlay composition proof"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let source = make(
        1,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    queue.write_texture(
        source.as_image_copy(),
        &[255, 0, 0, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        source.size(),
    );
    let target = make(
        4,
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = make(
        4,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let renderer = SceneRenderer::new(device, format);
    let mesh = renderer.upload_mesh(device, &SceneMesh::quad([0.0, 1.0, 0.0, 0.5]))?;
    let texture = renderer.upload_texture(device, queue, 1, 1, &[255; 4])?;
    let transform = renderer.create_transform(device, glam::Mat4::IDENTITY)?;
    let draws = [SceneDraw {
        geometry: &mesh,
        texture: &texture,
        transform: &transform,
        overlay: true,
    }];
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    renderer.encode(
        &mut encoder,
        &target_view,
        &depth_view,
        wgpu::Color::BLACK,
        &[],
    );
    TextureBlit::new(device, format).encode(
        device,
        &mut encoder,
        &source.create_view(&wgpu::TextureViewDescriptor::default()),
        &target_view,
    );
    renderer.encode_overlays(&mut encoder, &target_view, &depth_view, &draws);
    let mapped = read_overlay_pixels(device, queue, &target, encoder)?;
    let blended = if srgb {
        [188u8, 188, 0, 255]
    } else {
        [128u8, 128, 0, 255]
    };
    for y in 0..4 {
        for x in 0..4 {
            let expected = if (1..3).contains(&x) && (1..3).contains(&y) {
                blended
            } else {
                [255, 0, 0, 255]
            };
            let pixel = &mapped[y * 256 + x * 4..y * 256 + x * 4 + 4];
            if pixel
                .iter()
                .zip(expected)
                .any(|(actual, expected)| actual.abs_diff(expected) > 1)
            {
                return Err(format!("overlay mismatch {x},{y}: {pixel:?}").into());
            }
        }
    }
    println!(
        "Composed background and single overlay alpha blend match all 16 pixels (sRGB={srgb})"
    );
    Ok(())
}

fn read_overlay_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &wgpu::Texture,
    mut encoder: wgpu::CommandEncoder,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("overlay pixel readback"),
        size: 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        target.size(),
    );
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(index),
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let pixels = mapped.as_ref().to_vec();
    drop(mapped);
    buffer.unmap();
    Ok(pixels)
}

fn output_format(srgb: bool) -> wgpu::TextureFormat {
    if srgb {
        wgpu::TextureFormat::Rgba8UnormSrgb
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    }
}

fn depth_visualization_smoke(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = wgpu::Extent3d {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
    };
    let make = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth diagnostic proof"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let depth = make(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let target = make(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let blit = voxy_render::TextureBlit::depth(device, wgpu::TextureFormat::Rgba8Unorm);
    for (value, expected) in [(0.0, 0u8), (0.25, 64), (0.75, 191), (1.0, 255)] {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("initialize known depth"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(value),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
        }
        blit.encode(device, &mut encoder, &depth_view, &target_view);
        let pixels = read_overlay_pixels(device, queue, &target, encoder)?;
        for y in 0..4 {
            for x in 0..4 {
                let pixel = &pixels[y * 256 + x * 4..y * 256 + x * 4 + 4];
                if pixel[..3]
                    .iter()
                    .any(|actual| actual.abs_diff(expected) > 1)
                    || pixel[3] != 255
                {
                    return Err(
                        format!("depth {value} diagnostic mismatch at {x},{y}: {pixel:?}").into(),
                    );
                }
            }
        }
    }
    println!("Depth diagnostic readback matches 0, 0.25, 0.75, 1 across all pixels");
    Ok(())
}

fn postprocessing_smoke(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    for srgb in [false, true] {
        tone_mapping_smoke(device, queue, srgb)?;
        overlay_composition_smoke(device, queue, srgb)?;
    }
    depth_visualization_smoke(device, queue)
}

fn check_guide_pixels(
    index: usize,
    pixels: &[u8],
    material: &voxy_render::ReconstructionMaterial,
) -> Result<(), Box<dyn std::error::Error>> {
    for (pixel_index, pixel) in pixels
        .chunks_exact(256)
        .flat_map(|row| row[..16].chunks_exact(4))
        .enumerate()
    {
        let expected = match index {
            0 => [0.0, 0.0, 255.0, 128.0],
            1 => [0.0, 0.0, 0.0, 255.0],
            3 => [64.0, 0.0, 0.0, 255.0],
            _ => {
                let x = u16::try_from(pixel_index % 4)?;
                let y = u16::try_from(pixel_index / 4)?;
                let sample = material.sample(
                    [0.0, 0.0, 1.0],
                    [0.75 - f32::from(x) * 0.5, f32::from(y) * 0.5 - 0.75, 2.5],
                )?;
                sample.specular_albedo.map(|value| value * 255.0)
            }
        };
        if pixel
            .iter()
            .zip(expected)
            .any(|(actual, expected)| (f32::from(*actual) - expected).abs() > 1.1)
        {
            return Err(format!(
                "RR guide {index}, pixel {pixel_index}: {pixel:?} != {expected:?}"
            )
            .into());
        }
    }
    Ok(())
}
