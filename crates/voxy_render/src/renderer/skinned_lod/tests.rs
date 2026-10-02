use super::super::{
    CameraView, ChunkMeta, camera_uniform, create_camera_bind_group, create_camera_layout,
    create_material_bind_group, create_material_layout, default_material_pack,
};
use crate::skinned::{create_skin_layout, create_skinned_pipeline, object_uniform};
use crate::skinned_lod_gpu::GpuSkinnedLod;
use crate::{LodPolicy, MaterialSet, SceneCamera, SceneProjection};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[test]
#[ignore = "requires real GPU; explicit skeletal LOD color/depth acceptance"]
fn gpu_skeletal_lod_pose_switches_indices_and_preserves_shared_streams() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let camera_layout = create_camera_layout(&device);
    let material_layout = create_material_layout(&device);
    let skin_layout = create_skin_layout(&device);
    let pipeline = create_skinned_pipeline(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        &camera_layout,
        &material_layout,
        &skin_layout,
    );
    let view = CameraView {
        eye: Vec3::new(0., 0., 3.),
        target: Vec3::ZERO,
        ..CameraView::default()
    };
    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&camera_uniform(64, 64, view)),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let meta = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&ChunkMeta::zeroed()),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let light = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&0_u32),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let camera_bind =
        create_camera_bind_group(&device, &camera_layout, &camera_buffer, &meta, &light);
    let materials = MaterialSet::upload(&device, &queue, &default_material_pack());
    let material_bind = create_material_bind_group(&device, &material_layout, &materials);
    let half = 3. * (view.vertical_fov_radians * 0.5).tan();
    let camera = SceneCamera {
        eye: view.eye,
        target: view.target,
        up: view.up,
        projection: SceneProjection::Orthographic {
            left: -half,
            right: half,
            bottom: -half,
            top: half,
            near: 0.1,
            far: 512.,
        },
    };
    let source = crate::skinned_lod::tests::source();
    let pose = source
        .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
        .unwrap();
    let (mut gpu, mut lod) =
        GpuSkinnedLod::upload(&device, &skin_layout, pose, 0, 0, 4096).unwrap();
    let vertex = gpu.vertex.clone();
    let palette = gpu.joint_buffer.clone();
    let object = gpu.object_buffer.clone();
    let base = gpu.index.clone();
    let pinned = lod.bytes;
    assert!(lod.ensure(&device, 1, pinned + 12).unwrap());
    let target = |format| {
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
    };
    let color = target(wgpu::TextureFormat::Rgba8Unorm);
    let depth = target(wgpu::TextureFormat::Depth32Float);
    let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 32768,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let policy = LodPolicy {
        target_pixels: 2.,
        hysteresis: 0.1,
    };
    let mut motion = None;
    let mut baseline = None;
    for frame in 0..5 {
        let joints = [
            Mat4::IDENTITY,
            if frame == 2 {
                Mat4::from_translation(Vec3::Z)
            } else {
                Mat4::IDENTITY
            },
        ];
        if frame > 0 {
            let pose = source.prepare(&joints, Mat4::IDENTITY).unwrap();
            let desired = pose
                .select_for_camera(camera, [64, 64], policy, Some(lod.selected))
                .unwrap();
            lod.pose = pose;
            if frame == 4 {
                lod.bind(0, &mut gpu, &mut motion).unwrap();
                assert!(lod.evict(1).unwrap());
            }
            let selected = lod.fallback(desired).unwrap();
            assert_eq!(selected, usize::from(frame == 1 || frame == 3));
            queue.write_buffer(&gpu.joint_buffer, 0, bytemuck::cast_slice(&joints));
            queue.write_buffer(
                &gpu.object_buffer,
                0,
                bytemuck::bytes_of(&object_uniform(Mat4::IDENTITY, 0)),
            );
            lod.bind(selected, &mut gpu, &mut motion).unwrap();
            assert!(
                !motion
                    .as_ref()
                    .unwrap()
                    .history
                    .prepare(&joints, Mat4::IDENTITY)
                    .unwrap()
                    .history_valid
            );
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_bind_group(2, &gpu.bind_group, &[]);
            pass.set_vertex_buffer(0, gpu.vertex.slice(..));
            pass.set_index_buffer(gpu.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..gpu.index_count, 0, 0..1);
        }
        for (texture, offset, aspect) in [
            (&color, 0, wgpu::TextureAspect::All),
            (&depth, 16384, wgpu::TextureAspect::DepthOnly),
        ] {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(64),
                    },
                },
                wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
            );
        }
        let submitted = queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap();
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        {
            let bytes = readback.slice(..).get_mapped_range().unwrap();
            let pixels = bytes[..16384].to_vec();
            let depths: Vec<f32> = bytes[16384..]
                .chunks_exact(4)
                .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
                .collect();
            if let Some((original_pixels, original_depths)) = &baseline {
                assert_eq!(
                    &pixels, original_pixels,
                    "same fixture silhouette/material frame={frame}"
                );
                let original_depths: &Vec<f32> = original_depths;
                if frame == 2 {
                    assert!(
                        depths
                            .iter()
                            .zip(original_depths)
                            .any(|(now, old)| now - old > 0.0001)
                    );
                } else {
                    assert!(
                        depths
                            .iter()
                            .zip(original_depths)
                            .all(|(now, old)| (now - old).abs() < 0.000001)
                    );
                }
            } else {
                assert!(
                    pixels
                        .chunks_exact(4)
                        .filter(|pixel| pixel[..3] != [0; 3])
                        .count()
                        > 200
                );
                assert!(depths.iter().filter(|depth| **depth > 0.).count() > 200);
                baseline = Some((pixels, depths));
            }
        }
        readback.unmap();
        if let Some(motion) = &mut motion {
            assert!(motion.commit_presented());
        }
        assert_eq!(gpu.vertex, vertex);
        assert_eq!(gpu.joint_buffer, palette);
        assert_eq!(gpu.object_buffer, object);
    }
    assert_eq!(gpu.index, base);
    assert_eq!(lod.bytes, pinned);
    assert!(pollster::block_on(scope.pop()).is_none());
}
