//! Real GPU material views share image storage but preserve sampler behavior.
use super::*;
use voxy_render::{
    ImageAsset, ImageLimits, SceneDraw, SceneMesh, SceneVertex, TextureFilter, TextureWrap,
};

#[test]
#[ignore = "requires real GPU; native material sampler cache and rendered pixels"]
fn gpu_material_sampler_views_share_storage_and_render_distinct_filter_and_wrap_modes() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("MODEL SAMPLER GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mesh = SceneMesh::new(
        vec![
            SceneVertex {
                position: [-1., 1., 0.],
                uv: [-0.5, -0.5],
                color: [1.; 4],
            },
            SceneVertex {
                position: [-1., -1., 0.],
                uv: [-0.5, 1.5],
                color: [1.; 4],
            },
            SceneVertex {
                position: [1., -1., 0.],
                uv: [1.5, 1.5],
                color: [1.; 4],
            },
            SceneVertex {
                position: [1., 1., 0.],
                uv: [1.5, -0.5],
                color: [1.; 4],
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .unwrap();
    let asset = EditorAsset {
        mesh: mesh.clone(),
        lod: None,
        animated: None,
        skinned_lod: None,
        nodes: vec![],
        images: vec![
            ImageAsset::from_rgba(
                2,
                2,
                vec![
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                ],
                ImageLimits::default(),
            )
            .unwrap(),
        ],
    };
    let nearest = TextureSampling {
        wrap_u: TextureWrap::Clamp,
        wrap_v: TextureWrap::Clamp,
        min_filter: TextureFilter::Nearest,
        mag_filter: TextureFilter::Nearest,
        mipmap_filter: Some(TextureFilter::Nearest),
        anisotropy: 1,
    };
    let linear = TextureSampling {
        min_filter: TextureFilter::Linear,
        mag_filter: TextureFilter::Linear,
        mipmap_filter: Some(TextureFilter::Linear),
        ..nearest
    };
    let repeat = TextureSampling {
        wrap_u: TextureWrap::Repeat,
        wrap_v: TextureWrap::Repeat,
        ..nearest
    };
    let mut cache = ResidencyCache::default();
    let mut views = Vec::new();
    let mut images = BTreeMap::new();
    let mut bind = |sampling, mips| {
        model_texture(
            &renderer,
            &device,
            &queue,
            &asset,
            Some(0),
            sampling,
            mips,
            &mut cache,
            &mut views,
            &mut images,
        )
        .unwrap()
        .unwrap()
    };
    let a = bind(nearest, false);
    let b = bind(linear, false);
    let c = bind(repeat, false);
    let d = bind(nearest, true);
    assert!(Arc::ptr_eq(&a, &bind(nearest, false)));
    assert!(Arc::ptr_eq(&b, &bind(linear, false)));
    assert!(Arc::ptr_eq(&c, &bind(repeat, false)));
    assert!(Arc::ptr_eq(&d, &bind(nearest, true)));
    assert!(!Arc::ptr_eq(&a, &b));
    assert!(!Arc::ptr_eq(&a, &c));
    assert!(!Arc::ptr_eq(&a, &d));
    assert!(std::ptr::eq(a.texture(), b.texture()));
    assert!(std::ptr::eq(a.texture(), c.texture()));
    assert!(std::ptr::eq(a.texture(), d.texture()));
    assert_eq!(cache.live_bytes(), mip_bytes(2, 2));
    let geometry = renderer.upload_mesh(&device, &mesh).unwrap();
    let transform = renderer
        .create_transform(&device, glam::Mat4::IDENTITY)
        .unwrap();
    let render =
        |texture: &SceneTexture| pixels(&renderer, &device, &queue, &geometry, texture, &transform);
    let original = render(&a);
    let filtered = render(&b);
    let wrapped = render(&c);
    assert_ne!(original, filtered);
    assert_ne!(original, wrapped);
    let colors = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .map(|p| <[u8; 4]>::try_from(p).unwrap())
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(colors(&original).len(), 4);
    assert!(colors(&filtered).len() > 100);
    assert_eq!(original, render(&a));
    assert_eq!(original, render(&d));
    assert!(pollster::block_on(scope.pop()).is_none());
}

pub(crate) fn pixels(
    renderer: &SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    geometry: &SceneGeometry,
    texture: &SceneTexture,
    transform: &voxy_render::SceneTransform,
) -> Vec<u8> {
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("model sampler pixels"),
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
    };
    let color = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sampler readback"),
        size: 64 * 64 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.encode(
        &mut encoder,
        &color.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        wgpu::Color::BLACK,
        &[SceneDraw {
            geometry,
            texture,
            transform,
            overlay: true,
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
            buffer: &buffer,
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
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let pixels = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    pixels
}
