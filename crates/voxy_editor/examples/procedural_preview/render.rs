use glam::Vec3;
use std::{error::Error, path::Path};
use voxy_render::{SceneCamera, SceneDraw, SceneProjection, SceneRenderer};

pub fn render(
    path: &Path,
    scene_override: Option<&voxy_scene::SceneDocument>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let registry = voxy_editor::editor_component_registry()?;
    let project = voxy_editor::project::open(path, &registry)?;
    let scene = scene_override.unwrap_or(&project.scene);
    let loaded = scene.load(&registry, 512)?;
    let camera = scene
        .objects
        .iter()
        .filter(|o| {
            loaded
                .graph
                .active_in_hierarchy(loaded.resolve(&o.id).unwrap())
                .unwrap_or(false)
        })
        .find_map(|o| o.components.get("editor.camera.v1"))
        .ok_or("authored camera required")?;
    let target: [f32; 3] = serde_json::from_value(camera["target"].clone())?;
    let target = Vec3::from_array(target);
    let yaw = camera["yaw"].as_f64().ok_or("camera yaw")? as f32;
    let pitch = camera["pitch"].as_f64().ok_or("camera pitch")? as f32;
    let distance = camera["distance"].as_f64().ok_or("camera distance")? as f32;
    let eye = target
        + Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        ) * distance;
    let projection = if camera["perspective"].as_bool().unwrap_or(true) {
        SceneProjection::Perspective {
            vertical_fov: std::f32::consts::FRAC_PI_3,
            aspect: 1.,
            near: distance * 0.001,
            far: distance * 100.,
        }
    } else {
        SceneProjection::Orthographic {
            left: -distance * 0.5,
            right: distance * 0.5,
            bottom: -distance * 0.5,
            top: distance * 0.5,
            near: distance * 0.001,
            far: distance * 100.,
        }
    };
    let view = SceneCamera {
        eye,
        target,
        up: Vec3::Y,
        projection,
    }
    .view_projection()?;
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    eprintln!("Preview adapter: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    pollster::block_on(renderer.reload_shader(&device, include_str!("../../src/material.wgsl")))?;
    let white = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();
    // Deliberately restricted to generated OBJ resources, not a general asset loader.
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        project.root.join(&project.descriptor.asset_manifest),
    )?)?;
    for object in &scene.objects {
        if !loaded
            .graph
            .active_in_hierarchy(loaded.resolve(&object.id).ok_or("missing object")?)?
        {
            continue;
        }
        let Some(id) = object
            .components
            .get("editor.model.v1")
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let source = manifest["assets"]
            .as_array()
            .ok_or("assets missing")?
            .iter()
            .find(|a| a["asset"].as_str() == Some(id))
            .and_then(|a| a["source"].as_str())
            .ok_or("asset missing")?;
        let source = project.root.join(source).canonicalize()?;
        if !source.starts_with(&project.root)
            || source.extension().and_then(|s| s.to_str()) != Some("obj")
        {
            return Err("preview requires project-local OBJ".into());
        }
        let asset = voxy_render::ObjAsset::parse(
            &std::fs::read_to_string(source)?,
            voxy_render::ObjLimits::default(),
        )?;
        meshes.push(renderer.upload_mesh(&device, &asset.mesh)?);
        let world = loaded
            .graph
            .world_matrix(loaded.resolve(&object.id).ok_or("object missing")?)?;
        let transform = renderer.create_transform(&device, view * world)?;
        let material = object.components.get("editor.material.v1");
        let lit = material.and_then(|v| v["lit"].as_bool()).unwrap_or(true);
        let tint = object
            .components
            .get("editor.material.v1")
            .map(|v| serde_json::from_value::<[f32; 4]>(v["tint"].clone()))
            .transpose()?
            .unwrap_or([1.; 4]);
        transform.update_scene_material(
            &queue,
            world,
            tint,
            [0.4, 0.8, 1., if lit { 0.8 } else { 0. }],
        )?;
        transforms.push(transform);
    }
    let size = wgpu::Extent3d {
        width: 1024,
        height: 1024,
        depth_or_array_layers: 1,
    };
    let texture = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("procedural preview"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = texture(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = texture(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("preview readback"),
        size: 1024 * 4096,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let draws: Vec<_> = meshes
        .iter()
        .zip(&transforms)
        .map(|(geometry, transform)| SceneDraw {
            geometry,
            texture: &white,
            transform,
            overlay: false,
        })
        .collect();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    renderer.encode(
        &mut encoder,
        &color.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        wgpu::Color {
            r: 0.075,
            g: 0.09,
            b: 0.12,
            a: 1.,
        },
        &draws,
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
                bytes_per_row: Some(4096),
                rows_per_image: Some(1024),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = sender.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let pixels = readback.slice(..).get_mapped_range()?;
    let mut rgb = Vec::with_capacity(1024 * 1024 * 3);
    for p in pixels.chunks_exact(4) {
        rgb.extend_from_slice(&p[..3]);
    }
    drop(pixels);
    readback.unmap();
    eprintln!("Rendered {} meshes", meshes.len());
    Ok(rgb)
}
