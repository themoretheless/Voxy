//! Actual GPU shadow-depth checks: overlapping casters, clear, deformation, ownership.
use glam::{Mat4, Quat, Vec3};
use voxy_render::{SceneError, SceneMesh, SceneRenderer, ShadowMap};
fn copy(map: &ShadowMap, encoder: &mut wgpu::CommandEncoder, buffer: &wgpu::Buffer, offset: u64) {
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: map.texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::DepthOnly,
        },
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset,
                bytes_per_row: Some(256),
                rows_per_image: Some(8),
            },
        },
        wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
    );
}
fn read(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let data = buffer.slice(..).get_mapped_range()?.to_vec();
    buffer.unmap();
    Ok(data)
}
fn verify(bytes: &[u8], offset: usize, left: f32, right: f32) {
    for y in 0..8 {
        for x in 0..8 {
            let i = offset + y * 256 + x * 4;
            let actual = f32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
            let expected = if x < 4 { left } else { right };
            assert!(
                (actual - expected).abs() < 1e-6,
                "depth ({x},{y}): {actual} != {expected}"
            );
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        println!("SHADOW GPU: {:?}", adapter.get_info());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let mut near = renderer.upload_mesh(&device, &SceneMesh::quad([1.0; 4]))?;
        let far = renderer.upload_mesh(&device, &SceneMesh::quad([1.0; 4]))?;
        let map = ShadowMap::new(&device, 8, 8)?;
        assert!(matches!(
            ShadowMap::new(&device, 0, 8),
            Err(SceneError::InvalidGeometry)
        ));
        assert!(matches!(
            map.prepare(&near, Mat4::from_cols_array(&[f32::NAN; 16])),
            Err(SceneError::InvalidTransform)
        ));
        let near_matrix = Mat4::from_scale_rotation_translation(
            Vec3::new(1., 2., 1.),
            Quat::IDENTITY,
            Vec3::new(-0.5, 0., 0.25),
        );
        let far_matrix = Mat4::from_scale_rotation_translation(
            Vec3::new(2., 2., 1.),
            Quat::IDENTITY,
            Vec3::new(0., 0., 0.75),
        );
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow depth proof"),
            size: 4096,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        map.encode(
            &mut encoder,
            &[
                map.prepare(&near, near_matrix)?,
                map.prepare(&far, far_matrix)?,
            ],
        )?;
        copy(&map, &mut encoder, &buffer, 0);
        map.encode(&mut encoder, &[])?;
        copy(&map, &mut encoder, &buffer, 2048);
        queue.submit([encoder.finish()]);
        let first = read(&device, &buffer)?;
        verify(&first, 0, 0.25, 0.75);
        verify(&first, 2048, 1., 1.);
        // Prior draws have completed before updating the retained caster geometry.
        let original = SceneMesh::quad([1.; 4]);
        let mut vertices = original.vertices().to_vec();
        for vertex in &mut vertices {
            vertex.position[2] += 0.25;
        }
        let moved = SceneMesh::new(vertices, original.indices().to_vec())?;
        near.update(&queue, &moved)?;
        let mut encoder = device.create_command_encoder(&Default::default());
        map.encode(
            &mut encoder,
            &[
                map.prepare(&far, far_matrix)?,
                map.prepare(&near, near_matrix)?,
            ],
        )?;
        copy(&map, &mut encoder, &buffer, 0);
        queue.submit([encoder.finish()]);
        verify(&read(&device, &buffer)?, 0, 0.5, 0.75);
        let (foreign, _) = adapter.request_device(&Default::default()).await?;
        let foreign_renderer = SceneRenderer::new(&foreign, wgpu::TextureFormat::Rgba8Unorm);
        let foreign_geometry = foreign_renderer.upload_mesh(&foreign, &SceneMesh::quad([1.; 4]))?;
        assert!(matches!(
            map.prepare(&foreign_geometry, Mat4::IDENTITY),
            Err(SceneError::DeviceMismatch)
        ));
        let foreign_map = ShadowMap::new(&foreign, 8, 8)?;
        let foreign_draw = foreign_map.prepare(&foreign_geometry, Mat4::IDENTITY)?;
        let mut encoder = device.create_command_encoder(&Default::default());
        assert!(matches!(
            map.encode(&mut encoder, &[foreign_draw]),
            Err(SceneError::DeviceMismatch)
        ));
        println!(
            "SHADOW DEPTH PASS: 192 GPU pixels, nearest overlapping caster, empty clear, retained geometry deformation, reverse draw order, nonfinite transform and foreign-device rejection"
        );
        Ok(())
    })
}
