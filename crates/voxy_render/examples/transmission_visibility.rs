//! GPU regression: full-alpha transmission must select nearest geometry,
//! independent of instance order. Partial-alpha layers retain composition.
use glam::{Mat4, Vec3};
use voxy_render::archive_scene::{
    ArchiveInstance, ArchiveMaterial, ArchiveMaterialSource, ArchiveMeshSource,
    ArchiveSceneRenderer, ArchiveVertex,
};
fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layers: &[(f32, f32)],
    separate: bool,
) -> Vec<u8> {
    let mut materials = Vec::new();
    let mut meshes = Vec::new();
    for (i, &(z, alpha)) in layers.iter().enumerate() {
        let mut parameters = ArchiveMaterial::metallic_roughness(
            if alpha < 1. {
                [0.8, 0.02, 0.01, alpha]
            } else {
                [0.2, 0.3, 0.4, alpha]
            },
            0.,
            0.5,
            0.2,
            -2.,
        );
        parameters.emissive_normal = if z > -0.5 {
            [0.3, 0.01, 0.02, 1.]
        } else {
            [0.01, 0.3, 0.02, 1.]
        };
        materials.push(ArchiveMaterialSource {
            parameters,
            images: [None; 4],
            double_sided: true,
        });
        let vertices = [[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]]
            .map(|position| ArchiveVertex {
                position,
                normal: [0., 0., 1.],
                tangent: [1., 0., 0., 1.],
                uv: [0.5; 2],
            })
            .to_vec();
        meshes.push(ArchiveMeshSource {
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            instances: vec![ArchiveInstance::new(
                Mat4::from_translation(Vec3::new(0., 0., z))
                    * Mat4::from_rotation_y(if z < -0.5 { 0.4 } else { 0. }),
            )],
            material: i,
            center: Vec3::ZERO,
        });
    }
    if !separate && layers.len() > 1 {
        // Same material and same mesh, so mesh-level sorting cannot repair order.
        let instances = meshes
            .iter()
            .flat_map(|m| m.instances.iter().copied())
            .collect();
        meshes.truncate(1);
        meshes[0].instances = instances;
    }
    let backdrop_id = materials.len();
    materials.push(ArchiveMaterialSource {
        parameters: ArchiveMaterial::metallic_roughness([0.02, 0.02, 0.02, 1.], 0., 0.7, 0., -1.),
        images: [None; 4],
        double_sided: true,
    });
    meshes.push(ArchiveMeshSource {
        vertices: [
            [-4., -4., -2.],
            [4., -4., -2.],
            [4., 4., -2.],
            [-4., 4., -2.],
        ]
        .map(|position| ArchiveVertex {
            position,
            normal: [0., 0., 1.],
            tangent: [1., 0., 0., 1.],
            uv: [0.5; 2],
        })
        .to_vec(),
        indices: vec![0, 1, 2, 0, 2, 3],
        instances: vec![ArchiveInstance::new(Mat4::IDENTITY)],
        material: backdrop_id,
        center: Vec3::new(0., 0., -2.),
    });
    let mut renderer = ArchiveSceneRenderer::new(
        device,
        queue,
        wgpu::TextureFormat::Rgba16Float,
        32,
        32,
        vec![],
        materials,
        meshes,
        &[],
        Vec3::new(0.2, 1., 0.3).normalize(),
        [1., 1., 1., 10.],
        (Vec3::splat(-3.), Vec3::splat(3.)),
    )
    .unwrap();
    renderer.set_exposure_ev100(0.).unwrap();
    renderer.set_spatial_antialiasing(false);
    renderer.set_linear_output(true);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("visibility reference"),
        size: wgpu::Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    renderer.draw(
        &output.create_view(&Default::default()),
        Vec3::new(0., 0., 2.),
        -Vec3::Z,
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 8192,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(32),
            },
        },
        wgpu::Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let pixels = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    pixels
}
fn center(p: &[u8]) -> &[u8] {
    &p[16 * 256 + 16 * 8..16 * 256 + 17 * 8]
}
fn main() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::FLOAT32_FILTERABLE,
        ..Default::default()
    }))
    .unwrap();
    if std::env::args().any(|arg| arg == "--partial-reference") {
        let p = render(&device, &queue, &[(0., 1.), (0.5, 0.5)], true);
        println!("PARTIAL_REFERENCE {:?}", center(&p));
        return;
    }
    let near = render(&device, &queue, &[(0., 1.)], true);
    assert!(
        center(&near)[..6].iter().any(|&v| v != 0),
        "fixture rendered black; visibility comparison invalid"
    );
    let hidden = render(&device, &queue, &[(0., 1.), (-1., 1.)], true);
    assert_eq!(
        center(&near),
        center(&hidden),
        "hidden glass replaced nearest surface"
    );
    let instanced = render(&device, &queue, &[(0., 1.), (-1., 1.)], false);
    assert_eq!(
        center(&near),
        center(&instanced),
        "hidden instance replaced nearest surface"
    );
    let reverse = render(&device, &queue, &[(-1., 1.), (0., 1.)], true);
    assert_eq!(
        center(&near),
        center(&reverse),
        "nearest visibility depends on source order"
    );
    let partial = render(&device, &queue, &[(0.5, 0.5), (0., 1.)], true);
    assert_ne!(
        center(&near),
        center(&partial),
        "partial foreground alpha lost"
    );
    println!(
        "TRANSMISSION_VISIBILITY nearest_mesh=true nearest_instance=true source_order_independent=true partial_alpha_retained=true adapter={}",
        adapter.get_info().name
    );
}
