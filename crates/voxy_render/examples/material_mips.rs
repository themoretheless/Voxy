//! Real scene implicit-LOD proof using a minified black/white checkerboard.
use image::ImageEncoder;
use voxy_render::{
    GraphicsBackend, GraphicsOptions, ImageAsset, ImageLimits, SceneDraw, SceneMesh, SceneRenderer,
    TextureSampling,
};

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("VOXY_ASSET_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => GraphicsBackend::Auto,
        Ok("metal") => GraphicsBackend::Metal,
        Ok("vulkan") => GraphicsBackend::Vulkan,
        Ok("gl") => GraphicsBackend::OpenGl,
        Ok("dx12") => GraphicsBackend::DirectX12,
        _ => return Err("invalid VOXY_ASSET_BACKEND".into()),
    };
    let instance = GraphicsOptions {
        backend,
        ..Default::default()
    }
    .create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Material mip probe: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut pixels = Vec::new();
    for y in 0..256 {
        for x in 0..256 {
            let value = if (x + y) % 2 == 0 { 0 } else { 255 };
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png).write_image(
        &pixels,
        256,
        256,
        image::ExtendedColorType::Rgba8,
    )?;
    let image = ImageAsset::decode(&png, ImageLimits::default())?;
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = renderer.upload_mesh(&device, &SceneMesh::quad([1.0; 4]))?;
    let transform = renderer.create_transform(
        &device,
        glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.5)),
    )?;
    let base = renderer.upload_image(&device, &queue, &image, TextureSampling::default())?;
    let chain = image.mip_chain();
    for sampling in [
        TextureSampling {
            anisotropy: 0,
            ..TextureSampling::default()
        },
        TextureSampling {
            anisotropy: 17,
            ..TextureSampling::default()
        },
        TextureSampling {
            anisotropy: 4,
            ..TextureSampling::default()
        },
    ] {
        assert!(
            renderer
                .upload_image_mips(&device, &queue, &chain, sampling)
                .is_err()
        );
    }
    let mips = renderer.upload_image_mips(
        &device,
        &queue,
        &chain,
        TextureSampling {
            anisotropy: 4,
            min_filter: voxy_render::TextureFilter::Linear,
            mag_filter: voxy_render::TextureFilter::Linear,
            ..TextureSampling::default()
        },
    )?;
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("minification probe"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("minification depth"),
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        ..wgpu::TextureDescriptor {
            label: None,
            size: color.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mip pixels"),
        size: 16384,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut results = Vec::new();
    for texture in [&base, &mips] {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            &[SceneDraw {
                geometry: &geometry,
                texture,
                transform: &transform,
                overlay: false,
            }],
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            color.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let bytes = readback.slice(..).get_mapped_range()?;
        results.push(bytes.to_vec());
        drop(bytes);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    for y in 20..44 {
        for x in 20..44 {
            let offset = (y * 64 + x) * 4;
            let pixel = &results[1][offset..offset + 4];
            assert!(
                pixel[..3].iter().all(|value| value.abs_diff(128) <= 2),
                "mip averaging failed: {pixel:?}"
            );
            assert_eq!(pixel[3], 255);
        }
    }
    assert_ne!(
        results[0], results[1],
        "mip sampler still selected the base checkerboard"
    );
    println!("MIP LOD PASS: minified checkerboard is linear-gray, base-only rendering differs");
    Ok(())
}
