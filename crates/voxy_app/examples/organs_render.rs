//! Renders the imported female reference organ assembly through the native renderer for visual inspection.
use glam::Vec3;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};
use voxy_render::{ModelAsset, ModelLimits, SceneMesh};

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/anatomy/hra-female");
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    let groups: Vec<String> = if args == ["--all"] {
        manifest["groups"]
            .as_object()
            .ok_or("invalid atlas manifest")?
            .keys()
            .cloned()
            .collect()
    } else if args.is_empty() {
        [
            "heart",
            "liver",
            "lungs",
            "brain",
            "small-intestine",
            "colon",
            "skeleton-partial",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    } else {
        args
    };
    for name in groups {
        if manifest["groups"].get(&name).is_none() {
            return Err(format!("unknown anatomical group: {name}").into());
        }
        let data = std::fs::read(root.join(format!("{name}.glb")))?;
        let asset = ModelAsset::parse(
            &data,
            &[],
            ModelLimits {
                bytes: 64 * 1024 * 1024,
                vertices: 4_000_000,
                indices: 12_000_000,
                ..ModelLimits::default()
            },
        )?;
        let meshes = asset.scene_meshes(&asset.skeleton.bind_pose())?;
        println!("{name}: {} structures imported", meshes.len());
        for mesh in meshes {
            let offset = u32::try_from(vertices.len())?;
            let mut shaded = mesh.vertices().to_vec();
            let mut normals = vec![Vec3::ZERO; shaded.len()];
            for t in mesh.indices().chunks_exact(3) {
                let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
                let pa = Vec3::from_array(shaded[a].position);
                let n = (Vec3::from_array(shaded[b].position) - pa)
                    .cross(Vec3::from_array(shaded[c].position) - pa);
                for i in [a, b, c] {
                    normals[i] += n;
                }
            }
            for (v, n) in shaded.iter_mut().zip(normals) {
                let light = 0.35
                    + 0.65
                        * n.normalize_or_zero()
                            .dot(Vec3::new(-0.4, 0.5, 1.).normalize())
                            .abs();
                for c in &mut v.color[..3] {
                    *c *= light;
                }
            }
            vertices.extend(shaded);
            indices.extend(mesh.indices().iter().map(|i| i + offset));
        }
    }
    let min = vertices.iter().fold(Vec3::splat(f32::INFINITY), |a, v| {
        a.min(Vec3::from_array(v.position))
    });
    let max = vertices
        .iter()
        .fold(Vec3::splat(f32::NEG_INFINITY), |a, v| {
            a.max(Vec3::from_array(v.position))
        });
    let center = (min + max) * 0.5;
    let scale = 1.6 / (max - min).max_element();
    println!(
        "Atlas bounds: {min:?} .. {max:?}; {} vertices",
        vertices.len()
    );
    for v in &mut vertices {
        v.position = ((Vec3::from_array(v.position) - center) * scale).to_array();
    }
    let mesh = SceneMesh::new(vertices, indices)?;
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Model smoke on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = renderer.upload_mesh(&device, &mesh)?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let camera = SceneCamera {
        eye: Vec3::new(0.4, 0.1, 2.4),
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 45f32.to_radians(),
            aspect: 0.75,
            near: 0.01,
            far: 10.,
        },
    };
    let transform = renderer.create_transform(&device, camera.view_projection()?)?;
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
        size: 768 * 2304,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut frames = Vec::new();
    for frame in 0..2 {
        let angle = if frame == 0 { 0.0 } else { 0.65 };
        transform.update(
            &queue,
            camera.view_projection()? * glam::Mat4::from_rotation_y(angle),
        )?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color {
                r: 0.08,
                g: 0.10,
                b: 0.14,
                a: 1.,
            },
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
                    bytes_per_row: Some(2304),
                    rows_per_image: Some(768),
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
        assert!(
            pixels.chunks_exact(4).filter(|p| p[0] > 70).count() > 1000,
            "body missing from render"
        );
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let mut pixels = Vec::new();
    for y in 0..768 {
        for frame in &frames {
            pixels.extend_from_slice(&frame[y * 2304..(y + 1) * 2304]);
        }
    }
    image::save_buffer(
        "/tmp/voxy-organs-preview.png",
        &pixels,
        1152,
        768,
        image::ColorType::Rgba8,
    )?;
    println!("PASS: anatomical organ assembly rendered in two views; /tmp/voxy-organs-preview.png");
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
            width: 576,
            height: 768,
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
