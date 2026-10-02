//! GPU render of exact voxel wear geometry; saves before/after comparison.
use glam::{Mat4, Vec3};
use physics_voxel::VoxelWear;
use std::{collections::BTreeMap, sync::Arc};
use voxy_core::{ChunkPos, VoxelPos, WorldEpoch};
use voxy_render::{SceneDraw, SceneMesh, SceneRenderer, SceneVertex};
use voxy_world::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(BlockRegistry::new(vec![
        BlockDef {
            key: ResourceKey::parse("voxy:air")?,
            render: RenderKind::Invisible,
            occlusion: Occlusion::None,
            collision: CollisionShape::Empty,
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 0,
        },
        BlockDef {
            key: ResourceKey::parse("voxy:stone")?,
            render: RenderKind::Opaque,
            occlusion: Occlusion::FullCube,
            collision: CollisionShape::FullCube,
            face_materials: [MaterialId(1); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 20,
        },
    ])?);
    let stone = registry.find(&ResourceKey::parse("voxy:stone")?).unwrap();
    let mut world = World::new(
        WorldEpoch::new(1).unwrap(),
        registry,
        WorldLimits::default(),
    );
    world.insert_generated(GeneratedChunk {
        pos: ChunkPos { x: 0, y: 0, z: 0 },
        data: ChunkData {
            blocks: PalettedBlocks::uniform(stone),
            block_data: BTreeMap::new(),
        },
    })?;
    let mut wear = VoxelWear::new(
        &world,
        VoxelPos { x: 8, y: 8, z: 8 },
        1.,
        2000.,
        physics::wear::Material::new(1., 1.)?,
    )?;
    let before = wear.surface().unwrap();
    wear.advance(&mut world, 1., 0.4)?;
    let after = wear.surface().unwrap();
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let mut geometries = Vec::new();
    for surface in [before, after] {
        let vertices = surface
            .vertices
            .into_iter()
            .map(|position| SceneVertex {
                position: position.map(|v| v as f32),
                uv: [0.; 2],
                color: [0.6, 0.75, 0.95, 1.],
            })
            .collect();
        let indices = surface
            .triangles
            .into_iter()
            .flatten()
            .map(u32::from)
            .collect();
        geometries.push(renderer.upload_mesh(&device, &SceneMesh::new(vertices, indices)?)?);
    }
    #[allow(deprecated)]
    let camera = Mat4::orthographic_rh(-1.7, 1.7, -1.0, 1.0, 0.1, 10.)
        * Mat4::look_at_rh(Vec3::new(2., 2., 4.), Vec3::new(0., 0.5, 0.), Vec3::Y);
    let transforms = [-0.8, 0.8].map(|x| {
        renderer
            .create_transform(
                &device,
                camera * Mat4::from_translation(Vec3::new(x - 0.5, 0., -0.5)),
            )
            .unwrap()
    });
    let size = wgpu::Extent3d {
        width: 512,
        height: 256,
        depth_or_array_layers: 1,
    };
    let make = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wear surface"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = make(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = make(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    let draws = (0..2)
        .map(|i| SceneDraw {
            geometry: &geometries[i],
            texture: &texture,
            transform: &transforms[i],
            overlay: false,
        })
        .collect::<Vec<_>>();
    renderer.encode(
        &mut encoder,
        &color.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        wgpu::Color {
            r: 0.04,
            g: 0.05,
            b: 0.07,
            a: 1.,
        },
        &draws,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("wear readback"),
        size: 512 * 256 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(512 * 4),
                rows_per_image: Some(256),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let pixels = readback.slice(..).get_mapped_range()?;
    let mut coverage = [0_usize; 2];
    for (i, pixel) in pixels.chunks_exact(4).enumerate() {
        if pixel[2] > 100 {
            coverage[(i % 512) / 256] += 1;
        }
    }
    assert!(
        coverage[0] > coverage[1] && coverage[1] > 1000,
        "both blocks must render, with reduced worn-block coverage: {coverage:?}"
    );
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-wear-surface.png".into());
    image::save_buffer(&path, &pixels, 512, 256, image::ColorType::Rgba8)?;
    println!(
        "Saved {path}; remaining mass={} kg, debris={} kg; GPU {:?}",
        wear.remaining_mass_kg(),
        wear.debris_mass_kg(),
        adapter.get_info().backend
    );
    Ok(())
}
