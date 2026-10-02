//! Real-file threaded OBJ reload, verified through real GPU readback.
use glam::{Mat4, Vec3};
use std::time::{Duration, Instant};
use voxy_assets::{
    AssetCatalog, AssetId, AssetImportWorker, AssetStatus, FileInputs, ImportCompletion,
    SourceDependencies, SourcePollWorker,
};
use voxy_render::{GraphicsOptions, ObjAsset, ObjLimits, SceneDraw, SceneMesh, SceneRenderer};
fn completion<T: Send + Sync + 'static>(
    worker: &mut AssetImportWorker<T>,
) -> Result<ImportCompletion<T>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = worker.try_result()? {
            return Ok(result);
        }
        if Instant::now() >= deadline {
            return Err("import timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn scan(worker: &mut SourcePollWorker) -> Result<Vec<AssetId>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = worker.try_result()? {
            return Ok(result);
        }
        if Instant::now() >= deadline {
            return Err("scan timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn imported_frames() -> Result<Vec<SceneMesh>, Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "voxy-obj-reload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&root)?;
    let id = AssetId("quad.obj".into());
    let original = include_str!("assets/quad.obj");
    std::fs::write(root.join(&id.0), original)?;
    let owner = std::thread::current().id();
    let mut imports = AssetImportWorker::new(
        FileInputs::new(&root)?,
        1,
        4096,
        move |asset, provider, inputs| {
            assert_ne!(std::thread::current().id(), owner);
            let source = inputs
                .read(asset.clone(), |id, limit| provider.read(id, limit))
                .map_err(|e| format!("{e:?}"))?;
            let text = std::str::from_utf8(&source.bytes).map_err(|e| e.to_string())?;
            ObjAsset::parse(
                text,
                ObjLimits {
                    source_bytes: 4096,
                    attributes: 64,
                    vertices: 64,
                    triangles: 64,
                },
            )
            .map_err(|e| e.to_string())
        },
    )?;
    let mut catalog = AssetCatalog::new(1, 1)?;
    let mut sources = SourceDependencies::new(1, 1);
    let ticket = catalog.request(id.clone())?;
    imports.submit(&ticket)?;
    let result = completion(&mut imports)?;
    catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
    let held = catalog.snapshot(&id).unwrap();
    let mut frames = vec![held.value().mesh.clone()];
    let mut watcher = SourcePollWorker::new(FileInputs::new(&root)?, 1, 4096)?;
    watcher.request(&sources, 1)?;
    assert_eq!(scan(&mut watcher)?, vec![id.clone()]);
    let shifted = original
        .replace("v -0.5", "v 0.0")
        .replace("v 0.5", "v 1.0");
    for (text, valid) in [("not valid OBJ", false), (shifted.as_str(), true)] {
        std::fs::write(root.join(&id.0), text)?;
        watcher.request(&sources, 1)?;
        for asset in sources.affected(scan(&mut watcher)?) {
            let ticket = catalog.request(asset)?;
            imports.submit(&ticket)?;
            let result = completion(&mut imports)?;
            assert_eq!(result.result.is_ok(), valid);
            catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
        }
        assert_eq!(catalog.pending(), 0);
        assert_eq!(
            matches!(catalog.status(&id), Some(AssetStatus::Ready)),
            valid
        );
        frames.push(catalog.snapshot(&id).unwrap().value().mesh.clone());
    }
    frames.push(held.value().mesh.clone());
    imports.close().join().map_err(|_| "import panic")?;
    watcher.close().join().map_err(|_| "watch panic")?;
    std::fs::remove_dir_all(root)?;
    Ok(frames)
}
fn imported_images() -> Result<Vec<voxy_render::ImageAsset>, Box<dyn std::error::Error>> {
    use image::ImageEncoder;
    let root = std::env::temp_dir().join(format!("voxy-image-reload-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let id = AssetId("material.png".into());
    let owner = std::thread::current().id();
    let mut imports = AssetImportWorker::new(
        FileInputs::new(&root)?,
        1,
        4096,
        move |asset, provider, inputs| {
            assert_ne!(std::thread::current().id(), owner);
            let source = inputs
                .read(asset.clone(), |id, limit| provider.read(id, limit))
                .map_err(|error| format!("{error:?}"))?;
            voxy_render::ImageAsset::decode(&source.bytes, voxy_render::ImageLimits::default())
                .map_err(|error| error.to_string())
        },
    )?;
    let mut catalog = AssetCatalog::new(1, 1)?;
    let mut sources = SourceDependencies::new(1, 1);
    let mut watcher = SourcePollWorker::new(FileInputs::new(&root)?, 1, 4096)?;
    let mut frames = Vec::new();
    let mut held = None;
    for color in [Some([255, 255, 255, 255]), None, Some([255, 0, 0, 255])] {
        let mut bytes = Vec::new();
        if let Some(color) = color {
            image::codecs::png::PngEncoder::new(&mut bytes).write_image(
                &color.repeat(if frames.is_empty() { 1 } else { 2 }),
                if frames.is_empty() { 1 } else { 2 },
                1,
                image::ExtendedColorType::Rgba8,
            )?;
        } else {
            bytes.extend_from_slice(b"corrupt PNG");
        }
        std::fs::write(root.join(&id.0), bytes)?;
        if !frames.is_empty() {
            watcher.request(&sources, 1)?;
            assert_eq!(sources.affected(scan(&mut watcher)?), vec![id.clone()]);
        }
        let ticket = catalog.request(id.clone())?;
        imports.submit(&ticket)?;
        let result = completion(&mut imports)?;
        assert_eq!(result.result.is_ok(), color.is_some());
        catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
        let snapshot = catalog.snapshot(&id).ok_or("missing image snapshot")?;
        frames.push(snapshot.value().clone());
        if held.is_none() {
            held = Some(snapshot);
        }
    }
    frames.push(held.ok_or("missing held image")?.value().clone());
    assert_eq!((frames[0].width(), frames[0].height()), (1, 1));
    assert_eq!((frames[2].width(), frames[2].height()), (2, 1));
    imports.close().join().map_err(|_| "image import panic")?;
    watcher.close().join().map_err(|_| "image watcher panic")?;
    std::fs::remove_dir_all(root)?;
    Ok(frames)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    render_frames(&imported_frames()?, None)?;
    render_frames(
        &vec![SceneMesh::quad([1.0; 4]); 4],
        Some(&imported_images()?),
    )
}
#[allow(clippy::too_many_lines)]
fn render_frames(
    meshes: &[SceneMesh],
    images: Option<&[voxy_render::ImageAsset]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("VOXY_ASSET_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => voxy_render::GraphicsBackend::Auto,
        Ok("metal") => voxy_render::GraphicsBackend::Metal,
        Ok("vulkan") => voxy_render::GraphicsBackend::Vulkan,
        Ok("gl") => voxy_render::GraphicsBackend::OpenGl,
        Ok("dx12") => voxy_render::GraphicsBackend::DirectX12,
        _ => return Err("VOXY_ASSET_BACKEND expects auto|metal|vulkan|gl|dx12".into()),
    };
    let instance = GraphicsOptions {
        backend,
        ..Default::default()
    }
    .create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Asset reload on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let transform =
        renderer.create_transform(&device, Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5)))?;
    let color = target(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        &device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("model readback"),
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut centers = Vec::new();
    let mut frames = Vec::new();
    let mut geometry = renderer.upload_mesh(&device, meshes.first().ok_or("no meshes")?)?;
    geometry.set_depth_mode(voxy_render::SceneDepthMode::Transparent);
    for (index, mesh) in meshes.iter().enumerate() {
        if let Some(images) = images {
            if images[index].width() == 2 {
                assert!(
                    renderer
                        .upload_image_mips(
                            &device,
                            &queue,
                            &images[index..=index],
                            voxy_render::TextureSampling::default()
                        )
                        .is_err()
                );
                texture = renderer.upload_image_mips(
                    &device,
                    &queue,
                    &images[index].mip_chain(),
                    voxy_render::TextureSampling::default(),
                )?;
            } else {
                texture = renderer.upload_image(
                    &device,
                    &queue,
                    &images[index],
                    voxy_render::TextureSampling::default(),
                )?;
            }
        }
        renderer.replace_mesh(&device, &mut geometry, mesh)?;
        assert_eq!(
            geometry.depth_mode(),
            voxy_render::SceneDepthMode::Transparent
        );
        let previous_mode = geometry.depth_mode();
        assert!(
            renderer
                .replace_mesh(
                    &device,
                    &mut geometry,
                    &SceneMesh::quad([f32::NAN, 0.0, 0.0, 1.0])
                )
                .is_err()
        );
        assert_eq!(geometry.depth_mode(), previous_mode);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            &[SceneDraw {
                geometry: &geometry,
                texture: &texture,
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
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let pixels = readback.slice(..).get_mapped_range()?;
        let (mut count, mut sum) = (0u32, 0u32);
        for (index, pixel) in pixels.chunks_exact(4).enumerate() {
            if pixel[0] > 200 {
                count += 1;
                sum += u32::try_from(index % 64)?;
            }
        }
        assert!(count > 50, "imported quad missing");
        centers.push(f64::from(sum) / f64::from(count));
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    assert_eq!(frames[0], frames[1], "failed reload changed pixels");
    assert_eq!(frames[0], frames[3], "held old version changed pixels");
    if images.is_some() {
        assert!(
            (centers[2] - centers[0]).abs() < 0.01,
            "material update moved geometry"
        );
        assert_ne!(
            frames[2], frames[0],
            "corrected PNG did not change material pixels"
        );
    } else {
        assert!(
            (centers[2] - centers[0] - 16.0).abs() < 1.0,
            "reload did not shift quad: {centers:?}"
        );
    }
    let mut pixels = Vec::new();
    for y in 0..64 {
        for frame in &frames {
            pixels.extend_from_slice(&frame[y * 256..(y + 1) * 256]);
        }
    }
    image::save_buffer(
        if images.is_some() {
            "/tmp/voxy-image-hot-reload.png"
        } else {
            "/tmp/voxy-asset-hot-reload.png"
        },
        &pixels,
        256,
        64,
        image::ColorType::Rgba8,
    )?;
    if images.is_some() {
        println!(
            "GPU IMAGE RELOAD PASS: corrupt PNG retains pixels, corrected PNG changes material, held old version retains pixels"
        );
    } else {
        println!(
            "GPU ASSET RELOAD PASS: failed reload retains pixels, corrected OBJ moves 16 pixels, held old version retains pixels"
        );
    }
    Ok(())
}
fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}
