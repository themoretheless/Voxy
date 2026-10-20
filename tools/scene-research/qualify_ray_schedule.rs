//! Capture actual native renderer outputs; compare independent process variants.
//! This proves native RGBA16F frame equality, not unrounded intermediate float32.
use glam::{Mat4, Vec3};
use voxy_render::archive_scene::*;
fn read_queue_proof(device:&wgpu::Device,queue:&wgpu::Queue,source:&wgpu::Buffer)->[u32;10] {
 let read=device.create_buffer(&wgpu::BufferDescriptor {label:Some("independent all-ID queue coverage readback"),size:40,usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});
 let mut encoder=device.create_command_encoder(&Default::default());encoder.copy_buffer_to_buffer(source,0,&read,0,40);let submission=queue.submit([encoder.finish()]);
 let(tx,rx)=std::sync::mpsc::channel();read.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();
 let bytes=read.slice(..).get_mapped_range().unwrap();let result=std::array::from_fn(|i|u32::from_le_bytes(bytes[i*4..i*4+4].try_into().unwrap()));drop(bytes);read.unmap();result
}
fn quad(center: Vec3, axis: Vec3, up: Vec3, material: usize) -> ArchiveMeshSource {
    let normal = axis.cross(up).normalize();
    let vertices = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)].map(|(x, y)| ArchiveVertex {
        position: (center + axis * x + up * y).to_array(),
        normal: normal.to_array(),
        tangent: [
            axis.normalize().x,
            axis.normalize().y,
            axis.normalize().z,
            1.,
        ],
        uv: [(x + 1.) * 0.5, (y + 1.) * 0.5],
    });
    ArchiveMeshSource {
        vertices: vertices.to_vec(),
        indices: vec![0, 1, 2, 0, 2, 3],
        instances: vec![ArchiveInstance::new(Mat4::IDENTITY)],
        material,
        center,
    }
}
fn compare_raw_gi(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    a: &wgpu::Texture,
    b: &wgpu::Texture,
    kind: &str,
) -> usize {
    let row = a.width() * 16;
    assert_eq!(row % 256, 0);
    let bytes = u64::from(row) * u64::from(a.height());
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raw GI float32 comparison"),
        size: bytes * 2,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    for (i, texture) in [a, b].into_iter().enumerate() {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: i as u64 * bytes,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(a.height()),
                },
            },
            a.size(),
        );
    }
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = read.slice(..).get_mapped_range().unwrap();
    let n = bytes as usize;
    let mismatches = data[..n]
        .chunks_exact(4)
        .zip(data[n..].chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    if mismatches != 0 {
        std::fs::write("/tmp/voxy-gi-compute-mismatch.bin", &data[..n]).unwrap();
        std::fs::write("/tmp/voxy-gi-fragment-mismatch.bin", &data[n..]).unwrap();
        for (i, (a, b)) in data[..n]
            .chunks_exact(4)
            .zip(data[n..].chunks_exact(4))
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .take(16)
        {
            eprintln!(
                "{kind} mismatch pixel=({}, {}) channel={} compute={:08x} fragment={:08x}",
                i / 4 % 96,
                i / 4 / 96,
                i % 4,
                u32::from_le_bytes(a.try_into().unwrap()),
                u32::from_le_bytes(b.try_into().unwrap())
            );
        }
    }
    assert_eq!(
        mismatches, 0,
        "raw float32 ray output differs across fragment and compute scheduling"
    );
    assert!(
        data[..n]
            .chunks_exact(4)
            .all(|v| f32::from_le_bytes(v.try_into().unwrap()).is_finite())
    );
    let hits = data[..n]
        .chunks_exact(16)
        .filter(|p| f32::from_le_bytes(p[12..16].try_into().unwrap()) > 0.)
        .count();
    drop(data);
    read.unmap();
    hits
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).expect("output file");
    let hdr = args.iter().any(|a| a == "--hdr");
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::FLOAT32_FILTERABLE
            | if std::env::var_os("VOXY_STREET_RAY_PERSISTENT").is_some(){wgpu::Features::SUBGROUP}else{wgpu::Features::empty()}
            | if args.iter().any(|a| a == "--timing") {
                assert!(adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY));
                wgpu::Features::TIMESTAMP_QUERY
            } else {
                wgpu::Features::empty()
            },
        ..Default::default()
    }))
    .unwrap();
    let mapped = args.iter().any(|a| a == "--maps");
    let images = if mapped {
        [
            vec![
                0.6, 0.3, 0.2, 0.2, 0.2, 0.8, 0.4, 1., 0.5, 0.4, 0.8, 1., 0.9, 0.6, 0.1, 0.4,
            ],
            vec![
                0.3, 0.2, 0.8, 0.9, 0.8, 0.7, 0.2, 0.5, 0.2, 0.9, 0.7, 0.3, 0.9, 0.4, 0.5, 0.8,
            ],
            vec![
                0.6, 0.4, 0.98, 0.6, 0.4, 0.6, 0.98, 0.9, 0.55, 0.6, 0.98, 0.7, 0.45, 0.4, 0.98,
                0.5,
            ],
            vec![
                1.2, 0.1, 0.4, 1., 0.2, 2.5, 0.1, 1., 0.3, 0.2, 1.8, 1., 1.1, 0.9, 0.2, 1.,
            ],
        ]
        .into_iter()
        .map(|pixels| ArchiveImageSource::Float {
            width: 2,
            height: 2,
            pixels,
        })
        .collect()
    } else {
        vec![]
    };
    let mut materials = Vec::new();
    let mut geometry = Vec::new();
    for i in 0..12usize {
        let mut p = ArchiveMaterial::metallic_roughness(
            [0.1 + i as f32 * 0.06, 0.4, 0.2, 1.],
            if i % 3 == 0 { 1. } else { 0. },
            [0.02, 0.15, 0.35, 0.8][i % 4],
            0.,
            -1.,
        );
        if i == 11 {
            p = ArchiveMaterial::GLASS;
        }
        if i == 10 {
            p = ArchiveMaterial::MIRROR;
        }
        if i == 9 {
            p.emissive_normal = [2., 0.1, 0.3, 1.];
        }
        if i < 9 {
            p.workflow[2] = i as f32;
        }
        if mapped && i == 4 {
            p.surface[2] = 0.65;
        }
        let maps = if mapped && [1, 2, 4, 9].contains(&i) {
            p.workflow[1] = if i == 2 { 2. } else { 1. };
            if i == 1 {
                p.workflow[0] = 0.;
                p.surface[3] = 0.5;
            }
            [Some(0), Some(1), Some(2), Some(3)]
        } else {
            [None; 4]
        };
        materials.push(ArchiveMaterialSource {
            parameters: p,
            images: maps,
            double_sided: true,
        });
        geometry.push(quad(
            Vec3::new(
                (i % 4) as f32 * 1.1 - 1.65,
                (i / 4) as f32 * 0.85 - 0.9,
                -3. - (i % 2) as f32 * 0.3,
            ),
            Vec3::X * 0.5,
            Vec3::Y * 0.38,
            i,
        ));
    }
    geometry.push(quad(
        Vec3::new(0., -1.5, -3.),
        Vec3::X * 4.,
        Vec3::NEG_Z * 4.,
        10,
    ));
    let environment = hdr.then(|| ArchiveEnvironmentSource {
        lighting: voxy_render::HdrImageAsset::from_rgb(
            2,
            1,
            vec![2., 0.3, 1., 0.2, 4., 1.],
            voxy_render::ImageLimits::default(),
        )
        .unwrap(),
        background: voxy_render::HdrImageAsset::from_rgb(
            2,
            1,
            vec![2., 0.3, 1., 0.2, 4., 1.],
            voxy_render::ImageLimits::default(),
        )
        .unwrap(),
        intensity: 2.,
    });
    let mut renderer = ArchiveSceneRenderer::new_with_environment(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba16Float,
        96,
        64,
        images,
        materials,
        geometry,
        &[ArchiveLight {
            position_kind: [1., 1., -1., 8.],
            colour_intensity: [0.8, 0.3, 1., 300.],
        }],
        Vec3::new(-0.5, 1., 0.3).normalize(),
        [1., 0.9, 0.8, 100000.],
        (Vec3::splat(-8.), Vec3::splat(8.)),
        environment,
    )
    .unwrap();
    assert_eq!(renderer.opaque_ray_hierarchy_enabled(),std::env::var_os("VOXY_STREET_RAY_HIERARCHY").is_some());
    assert_eq!(renderer.opaque_ray_hierarchy_reciprocal_enabled(),std::env::var_os("VOXY_STREET_RAY_HIERARCHY_RECIPROCAL").is_some());
    assert_eq!(renderer.opaque_ray_hierarchy_near_first_enabled(),std::env::var_os("VOXY_STREET_RAY_HIERARCHY_NEAR_FIRST").is_some());
    println!("GPU_RAY_NEAR_FIRST enabled={}",renderer.opaque_ray_hierarchy_near_first_enabled());
    println!("GPU_RAY_HIERARCHY enabled={} reciprocal_cache={} owned_bounds_bytes={}",renderer.opaque_ray_hierarchy_enabled(),renderer.opaque_ray_hierarchy_reciprocal_enabled(),renderer.opaque_ray_hierarchy_bounds_bytes());
    let stages = renderer.opaque_ray_stages_configuration();
    if std::env::var_os("VOXY_STREET_RAY_STAGES").is_some() {
        let info = stages.expect("staged owner must be active");
        assert!(info[0] > 0);
        assert!(info[1] <= u64::from(device.limits().max_storage_buffer_binding_size));
        if std::env::var_os("VOXY_STREET_RAY_CHUNK_TEST").is_some() {
            assert_eq!(info[0], 16, "fixture must cross partial chunk boundaries");
        }
    }
    println!(
        "GPU_RAY_STAGES configuration={stages:?} forced_fallback={}",
        std::env::var_os("VOXY_STREET_RAY_FORCE_FALLBACK").is_some()
    );
    if std::env::var_os("VOXY_STREET_RAY_QUEUE").is_some() {
        assert!(renderer.opaque_ray_queue_enabled(),"queue request must activate the real owner");
        if std::env::var_os("VOXY_STREET_GI_PRECISION_PROOF").is_some(){assert!(renderer.opaque_ray_queue_proof_buffer().is_some());}
    }
    assert_eq!(renderer.opaque_ray_persistent_enabled(),std::env::var_os("VOXY_STREET_RAY_PERSISTENT").is_some());
    println!("GPU_RAY_PERSISTENT enabled={} quantum=16 group_cap=256",renderer.opaque_ray_persistent_enabled());
    renderer.set_linear_output(true);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("native scheduling proof"),
        size: wgpu::Extent3d {
            width: 96,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 96 * 64 * 8,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut all = Vec::new();
    let mut raw_ssr_hits = 0usize;
    let mut raw_gi_hits = 0usize;
    let mut raw_gi_frames = 0usize;
    let mut queue_epoch=0u32;let mut queue_proof_frames=0u32;let mut queue_active_jobs=0u64;
    for frame in 0..24 {
        if frame==18&&args.iter().any(|a|a=="--history-cut") {renderer.reset_temporal_history();}
        if frame == 12 {
            renderer
                .set_antialiasing(voxy_render::ArchiveAntialiasing::Temporal)
                .unwrap();
        }
        renderer
            .set_surface_wetness(Some(if frame < 12 { 0. } else { 0.8 }))
            .unwrap();
        renderer.set_weather_time(frame as f32 * 0.04);
        renderer.draw(
            &view,
            Vec3::new((frame as f32 * 0.13).sin() * 0.15, 0., 1.),
            Vec3::new(0., -0.08, -1.).normalize(),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(96 * 8),
                    rows_per_image: Some(64),
                },
            },
            texture.size(),
        );
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = read.slice(..).get_mapped_range().unwrap();
        assert!(
            mapped
                .chunks_exact(2)
                .all(|v| u16::from_le_bytes(v.try_into().unwrap()) & 0x7c00 != 0x7c00),
            "nonfinite native pixel"
        );
        all.extend_from_slice(&mapped);
        drop(mapped);
        read.unmap();
        if let Some(buffer)=renderer.opaque_ray_queue_proof_buffer() {
            let counters=read_queue_proof(&device,&queue,buffer);
            assert_eq!(u64::from(counters[7]),renderer.opaque_ray_queue_encoded_frames(),"GPU queue epoch must match actual CPU scheduling");
            if counters[7]!=queue_epoch {
                assert_eq!(&counters[..4],&[0;4],"invalid, duplicate, missing or incorrectly sized ray queue");
                assert_eq!(&counters[8..10],&[0;2],"queued hit words differ from original-ID traversal");
                let kinds=if std::env::var_os("VOXY_STREET_COMPUTE_SSR").is_some(){2}else{1};
                assert_eq!(counters[4],96*64*kinds,"every original ray ID must be processed");
                assert_eq!(counters[5],96*64*kinds,"coverage must include every chunk and ray kind");
                queue_epoch=counters[7];queue_proof_frames+=1;queue_active_jobs+=u64::from(counters[6]);
            }
        }
        if let Some((actual, reference)) = renderer.opaque_gi_precision_textures() {
            raw_gi_hits += compare_raw_gi(&device, &queue, actual, reference, "GI");
            if let Some((actual, reference)) = renderer.opaque_ssr_precision_textures() {
                raw_ssr_hits += compare_raw_gi(&device, &queue, actual, reference, "SSR");
            }
            raw_gi_frames += 1;
        }
    }
    if std::env::var_os("VOXY_STREET_GI_PRECISION_PROOF").is_some() {
        assert_eq!(raw_gi_frames, 24);
        if std::env::var_os("VOXY_STREET_COMPUTE_SSR").is_some() {
            assert!(raw_ssr_hits > 0, "reflection precision fixture must hit");
            println!(
                "GPU_RAW_SSR_PRECISION frames=24 float32_components=589824 actual_hits={raw_ssr_hits} every_bit_exact=true"
            );
        }
        assert!(raw_gi_hits > 0, "raw GI fixture must include actual hits");
        println!(
            "GPU_RAW_GI_PRECISION frames={raw_gi_frames} float32_components={} actual_hits={raw_gi_hits} every_bit_exact=true",
            raw_gi_frames * 96 * 64 * 4
        );
    }
    if renderer.opaque_ray_queue_proof_buffer().is_some() {
        assert!(queue_proof_frames>0);
        assert_eq!(u64::from(queue_proof_frames),renderer.opaque_ray_queue_encoded_frames());
        if std::env::var_os("VOXY_STREET_RAY_FORCE_FALLBACK").is_some(){assert_eq!(queue_active_jobs,0);}else{assert!(queue_active_jobs>0);}
        println!("GPU_RAY_QUEUE_COVERAGE frames={queue_proof_frames} active_jobs={queue_active_jobs} all_original_ids_exactly_once=true all_chunks_and_ray_kinds=true inactive_and_fallback_retained=true intersection_words_bit_exact=true work_count_equality_required={}",!renderer.opaque_ray_hierarchy_enabled());
    }
    assert!(
        all.chunks_exact(8).any(|p| p[..6].iter().any(|&v| v != 0)),
        "empty fixture"
    );
    assert!(
        all[..96 * 64 * 8] != all[23 * 96 * 64 * 8..24 * 96 * 64 * 8],
        "fixture must change"
    );
    if args.iter().any(|a| a == "--timing") {
        assert_eq!(renderer.gpu_gi_times_ms.len(), renderer.gpu_times_ms.len());
        assert_eq!(renderer.gpu_ssr_times_ms.len(), renderer.gpu_times_ms.len());
        let reflections: Vec<_> = renderer
            .gpu_ssr_times_ms
            .iter()
            .filter_map(|v| *v)
            .collect();
        if std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some()
            && std::env::var_os("VOXY_STREET_COMPUTE_SSR").is_some()
        {
            assert!(!reflections.is_empty());
            assert!(reflections.iter().all(|v| v.is_finite() && *v > 0.));
        } else {
            assert!(reflections.is_empty());
        }
        println!(
            "GPU_SSR_TIMING samples={} intervals_ms={reflections:?}",
            reflections.len()
        );
        let samples: Vec<_> = renderer.gpu_gi_times_ms.iter().filter_map(|v| *v).collect();
        if std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some() {
            assert!(!samples.is_empty(), "no GI timing samples");
            assert!(
                samples
                    .iter()
                    .all(|v| v.iter().all(|ms| ms.is_finite() && *ms > 0.))
            );
        } else {
            assert!(samples.is_empty());
        }
        println!(
            "GPU_GI_TIMING total_samples={} gi_samples={} guide_and_compute_intervals_ms={samples:?} no_stage_sum_used=true",
            renderer.gpu_times_ms.len(),
            samples.len()
        );
    }
    if let Some(directory)=std::env::var_os("VOXY_STREET_RAY_CAPTURE_DIR") {renderer.save_opaque_ray_capture(std::path::Path::new(&directory)).unwrap();}
    std::fs::write(path, &all).unwrap();
    println!(
        "GPU_RAY_SCHEDULE frames=24 materials=12 hdr={hdr} maps={mapped} native_rgba16f_bytes={} paired_dda={} early_gi={} finite=true moving_camera=true wet_and_dry=true temporal_history=true intermediate_float32_proof=false fps_proof=false",
        all.len(),
        std::env::var_os("VOXY_STREET_PAIRED_DDA").is_some(),
        std::env::var_os("VOXY_STREET_EARLY_GI").is_some()
    );
}
