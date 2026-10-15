//! Generate metric botanical GLBs and inspect them through the native PBR/shadow renderer.
use glam::{Mat4, Vec3};
use voxy_model::plants::{PlantKind, PlantOptions, generate_plant_glb, generate_world_plant_asset};
use voxy_render::archive_scene::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "artifacts/vegetation".into());
    std::fs::create_dir_all(&output)?;
    let mut images = Vec::new();
    let mut materials = Vec::new();
    let mut geometry = Vec::new();
    let mut low = Vec3::ZERO;
    let mut high = Vec3::ZERO;
    for (kind, height, x) in [(PlantKind::Tree, 6., -1.6), (PlantKind::Shrub, 1.6, 2.6)] {
        let options = PlantOptions {
            kind,
            seed: 42,
            height,
        };
        std::fs::write(
            format!("{output}/{kind:?}-42.glb"),
            generate_plant_glb(options)?,
        )?;
        // Preview the same native asset the app instances, not a GLB round trip.
        let asset = generate_world_plant_asset(options)?;
        let image_offset = images.len();
        let material_offset = materials.len();
        images.extend(asset.images.into_iter().map(ArchiveImageSource::Rgba));
        for mut material in asset.materials {
            for image in material.images.iter_mut().flatten() {
                *image += image_offset;
            }
            materials.push(material);
        }
        let world = Mat4::from_translation(Vec3::new(x, 0., 0.));
        for (id, mut mesh) in asset.geometry.into_iter().enumerate() {
            for v in &mesh.vertices {
                let p = world.transform_point3(Vec3::from_array(v.position));
                low = low.min(p);
                high = high.max(p);
            }
            println!(
                "{kind:?} surface {id}: {} vertices, {} triangles",
                mesh.vertices.len(),
                mesh.indices.len() / 3
            );
            mesh.instances = vec![ArchiveInstance::new(world)];
            mesh.material += material_offset;
            mesh.center = Vec3::new(x, height * 0.5, 0.);
            geometry.push(mesh);
        }
    }
    // Real geometry receives plant shadows; no painted shadow decals.
    geometry.push(ArchiveMeshSource {
        vertices: [
            [-12., -0.02, -12.],
            [-12., -0.02, 12.],
            [12., -0.02, 12.],
            [12., -0.02, -12.],
        ]
        .map(|position| ArchiveVertex {
            position,
            normal: [0., 1., 0.],
            tangent: [1., 0., 0., 1.],
            uv: [0.; 2],
        })
        .to_vec(),
        indices: vec![0, 1, 2, 0, 2, 3],
        instances: vec![ArchiveInstance::new(Mat4::IDENTITY)],
        material: materials.len(),
        center: Vec3::ZERO,
    });
    materials.push(ArchiveMaterialSource {
        parameters: ArchiveMaterial {
            diffuse: [0.3, 0.27, 0.20, 1.],
            specular_gloss: [1.; 4],
            emissive_normal: [0., 0., 0., 1.],
            surface: [0., 0.95, 0., -1.],
            workflow: [1., 0., 0., 0.],
        },
        images: [None; 4],
        double_sided: false,
    });
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = ArchiveSceneRenderer::new_with_images(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        1536,
        1024,
        images,
        materials,
        geometry,
        &[],
        Vec3::new(-0.6, 0.8, 0.4).normalize(),
        [1., 0.91, 0.78, 8500.],
        (low, high),
    )?;
    let output_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("botanical PBR preview"),
        size: wgpu::Extent3d {
            width: 1536,
            height: 1024,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let eye = Vec3::new(7.0, 3.6, 10.5);
    renderer.draw(
        &output_texture.create_view(&Default::default()),
        eye,
        (Vec3::new(0., 3., 0.) - eye).normalize(),
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("botanical readback"),
        size: 1536 * 1024 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1536 * 4),
                rows_per_image: Some(1024),
            },
        },
        output_texture.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let pixels = readback.slice(..).get_mapped_range()?;
    image::save_buffer(
        format!("{output}/preview.png"),
        &pixels,
        1536,
        1024,
        image::ColorType::Rgba8,
    )?;
    drop(pixels);
    readback.unmap();
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "Botanical GPU preview: {output}/preview.png on {}",
        adapter.get_info().name
    );
    Ok(())
}
