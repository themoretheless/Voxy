//! Rasterizes the production voxel vertex logic with diagnostic AO output.
use voxy_render::{GraphicsBackend, GraphicsOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let backend = match arguments.next().as_deref() {
        Some("metal") => GraphicsBackend::Metal,
        Some("vulkan") => GraphicsBackend::Vulkan,
        Some("gl") => GraphicsBackend::OpenGl,
        Some("dx12") => GraphicsBackend::DirectX12,
        _ => return Err("expected metal|vulkan|gl|dx12".into()),
    };
    let require_nvidia = match arguments.next().as_deref() {
        None => false,
        Some("--require-nvidia") => true,
        _ => return Err("expected optional --require-nvidia after backend".into()),
    };
    if arguments.next().is_some() {
        return Err("unexpected voxel diagonal argument".into());
    }
    let instance = GraphicsOptions {
        backend,
        ..Default::default()
    }
    .create_instance();
    let adapter = if require_nvidia {
        pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
            .into_iter()
            .find(|adapter| {
                let info = adapter.get_info();
                info.vendor == 0x10de
                    && matches!(
                        info.device_type,
                        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
                    )
            })
            .ok_or("voxel diagonal hardware gate requires a physical NVIDIA graphics adapter")?
    } else {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?
    };
    let info = adapter.get_info();
    println!("Voxel diagonal GPU: {info:?}");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    for flipped in [false, true] {
        render_and_check(&device, &queue, fixture_source(flipped)?, flipped)?;
    }

    println!("PASS: production voxel Uv/Vu diagonals, 128 AO pixels match analytic interpolation");
    Ok(())
}

fn fixture_source(flipped: bool) -> Result<String, Box<dyn std::error::Error>> {
    let production = include_str!("../src/voxel.wgsl");
    let structs = production
        .split("struct VertexInput")
        .nth(1)
        .ok_or("missing vertex input")?
        .split("fn sample_light")
        .next()
        .ok_or("missing vertex structures")?;
    let vertex = production
        .split("fn face_normal")
        .nth(1)
        .ok_or("missing vertex functions")?
        .split("@fragment")
        .next()
        .ok_or("missing fragment boundary")?;
    let vertex = vertex
        .replace("@vertex\n", "")
        .replace(
            "vec3<f32>(chunk_meta[input.chunk_slot.x].relative_origin.xyz)",
            "vec3<f32>(0.0)",
        )
        .replace(
            "camera.view_proj * vec4<f32>(local, 1.0)",
            "vec4<f32>(local, 1.0)",
        )
        .replace(
            "sample_light(input.chunk_slot.x, input.origin.xyz)",
            "vec2<f32>(1.0)",
        );
    let fixture = format!(
        r"
@vertex fn fixture(@builtin(vertex_index) id: u32) -> VertexOutput {{
    var input: VertexInput;
    input.vertex_index = id;
    input.extent_face = vec4<u32>(1u, 1u, 5u, 0u);
    input.ao_diagonal = vec4<u32>(0u, 3u, 0u, {}u);
    var output = vs_main(input);
    output.position = vec4<f32>(output.uv * 2.0 - 1.0, 0.5, 1.0);
    return output;
}}
@fragment fn ao_fragment(input: VertexOutput) -> @location(0) vec4<f32> {{
    return vec4<f32>(input.ao, input.ao, input.ao, 1.0);
}}",
        if flipped { 131 } else { 3 }
    );
    let source = format!("struct VertexInput{structs}\nfn face_normal{vertex}\n{fixture}");
    Ok(source)
}

fn render_and_check(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: String,
    flipped: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production voxel diagonal AO diagnostic"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("fixture"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("ao_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 8,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..6, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(8),
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    let bytes = readback.slice(..).get_mapped_range()?;
    verify_pixels(&bytes, flipped);
    Ok(())
}

fn verify_pixels(bytes: &[u8], flipped: bool) {
    for y in 0..8_usize {
        for x in 0..8_usize {
            #[allow(clippy::cast_precision_loss)]
            let (u, v) = ((x as f64 + 0.5) / 8.0, 1.0 - (y as f64 + 0.5) / 8.0);
            let expected = if flipped {
                (u + v).min(2.0 - u - v)
            } else {
                (u - v).abs()
            } * 255.0;
            let pixel = &bytes[y * 256 + x * 4..y * 256 + x * 4 + 4];
            assert!(
                pixel[..3]
                    .iter()
                    .all(|&channel| (f64::from(channel) - expected).abs() <= 1.0),
                "AO mismatch: flipped={flipped}, ({x},{y}), {pixel:?}, expected {expected}"
            );
            assert_eq!(pixel[3], 255);
        }
    }
}
