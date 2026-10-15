use super::*;
include!("environment_pixel_fixture.rs");
#[test]
#[ignore = "requires physical GPU; real SceneRenderer HDR pixels and material routing"]
fn real_hdr_scene_matches_ibl_oracle_after_atlas_install() {
    use crate::{
        GgxEnergyAtlasBuild, GgxEnergyAtlasSlot, GgxEnergyBuildBudget, GgxEnergyGpuPrecompute,
        GgxEnergyRefinementPlan, GgxEnergyRefinementPolicy,
    };
    const W: u32 = 192;
    const H: u32 = 64;
    const COLOR: [f32; 3] = [0.8, 0.3, 0.1];
    let output = std::env::var_os("VOXY_GGX_PIXEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/voxy-ggx-scene-pixels"));
    std::fs::create_dir_all(&output).unwrap();
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (d, q) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = pollster::block_on(GgxEnergyGpuPrecompute::new(&d)).unwrap();
    let env = crate::GgxEnvironmentPrefilter::new(&d, 4).unwrap();
    let white = vec![[0x3c00u16; 4]; 4 * 4 * 6];
    q.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: env.input(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&white),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(32),
            rows_per_image: Some(4),
        },
        wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 6,
        },
    );
    let legacy = crate::GgxDfgLut::new(&d, 64).unwrap();
    let mut encoder = d.create_command_encoder(&Default::default());
    env.encode(&mut encoder);
    legacy.encode(&mut encoder);
    q.submit([encoder.finish()]);
    let mut slot = GgxEnergyAtlasSlot::new(legacy, 0);
    let mut scene = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba16Float);
    pollster::block_on(scene.enable_environment_lighting(&d, &env, slot.current())).unwrap();
    let texture = scene.upload_texture(&d, &q, 1, 1, &[255; 4]).unwrap();
    let parameters = [
        [PIXEL_ORACLE[4].uv[0], PIXEL_ORACLE[4].uv[1]],
        [PIXEL_ORACLE[13].uv[0], PIXEL_ORACLE[13].uv[1]],
        [PIXEL_ORACLE[13].uv[0], 0.],
    ];
    let mut geometries = Vec::new();
    let mut transforms = Vec::new();
    for [nv, r] in parameters {
        let normal = [(1. - nv * nv).sqrt(), 0., nv];
        let mesh = SceneMesh::quad([1.; 4])
            .with_normals(vec![normal; 4])
            .unwrap();
        geometries.push(scene.upload_mesh(&d, &mesh).unwrap());
        let transform = scene
            .create_transform(&d, Mat4::from_scale(glam::Vec3::splat(2.)))
            .unwrap();
        transform
            .update_scene_material(
                &q,
                Mat4::IDENTITY,
                [COLOR[0], COLOR[1], COLOR[2], 1.],
                [0., 0., 10., 0.],
            )
            .unwrap();
        transform.update_pbr_material(&q, r, 1.).unwrap();
        transform
            .update_view_position(&q, glam::Vec3::new(0., 0., 1e12))
            .unwrap();
        transforms.push(transform);
    }
    let allocate = |format, usage| {
        d.create_texture(&wgpu::TextureDescriptor {
            label: Some("actual scene HDR pixel qualification"),
            size: wgpu::Extent3d {
                width: W,
                height: H,
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
    let color = allocate(
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = allocate(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let render = |scene: &SceneRenderer| {
        let draws = (0..3)
            .map(|i| {
                [SceneDraw {
                    geometry: &geometries[i],
                    texture: &texture,
                    transform: &transforms[i],
                    overlay: false,
                }]
            })
            .collect::<Vec<_>>();
        let views = draws
            .iter()
            .enumerate()
            .map(|(i, draws)| SceneView {
                viewport: [i as u32 * 64, 0, 64, 64],
                draws,
            })
            .collect::<Vec<_>>();
        let staging = d.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(W * H * 8),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = d.create_command_encoder(&Default::default());
        scene
            .encode_views(
                &mut encoder,
                &color_view,
                &depth_view,
                wgpu::Color::BLACK,
                &views,
            )
            .unwrap();
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(W * 8),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
        );
        q.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        loop {
            d.poll(wgpu::PollType::Poll).unwrap();
            match rx.try_recv() {
                Ok(r) => {
                    r.unwrap();
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                Err(e) => panic!("pixel readback disconnected {e}"),
            }
        }
        let bytes = staging.slice(..).get_mapped_range().unwrap();
        bytes
            .chunks_exact(8)
            .map(|p| {
                std::array::from_fn::<_, 4, _>(|c| {
                    half::f16::from_bits(u16::from_le_bytes(
                        p[2 * c..2 * c + 2].try_into().unwrap(),
                    ))
                    .to_f32()
                })
            })
            .collect::<Vec<_>>()
    };
    let before = render(&scene);
    slot.request_generation(1).unwrap();
    let regions =
        GgxEnergyRefinementPlan::regions_from_seeds(&[[1., 0.005], [0.05, 0.25]]).unwrap();
    let mut build = GgxEnergyAtlasBuild::new(
        &d,
        &q,
        1,
        regions,
        PIXEL_ORACLE.to_vec(),
        GgxEnergyRefinementPolicy::try_new(1e-5, 8).unwrap(),
        GgxEnergyBuildBudget {
            max_nodes: 4096,
            max_probes: 512,
            max_candidate_atlas_bytes: 2 * 1024 * 1024,
        },
    )
    .unwrap();
    let ready = loop {
        if let Some(ready) = build.poll(&gpu, 1).unwrap() {
            break ready;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    let prepared =
        pollster::block_on(scene.prepare_environment_lighting(&d, &env, ready.atlas(), None))
            .unwrap();
    SceneRenderer::commit_environment_atlas_batch(vec![(&mut scene, prepared)], &mut slot, ready)
        .unwrap();
    let after = render(&scene);
    let save = |name: &str, pixels: &[[f32; 4]]| {
        let mut rgba = Vec::with_capacity(pixels.len() * 4);
        for p in pixels {
            for &linear in &p[..3] {
                assert!(linear.is_finite());
                let x = linear.clamp(0., 1.);
                let srgb = if x <= 0.0031308 {
                    12.92 * x
                } else {
                    1.055 * x.powf(1. / 2.4) - 0.055
                };
                rgba.push((srgb * 255.).round() as u8);
            }
            rgba.push(255);
        }
        image::RgbaImage::from_raw(W, H, rgba)
            .unwrap()
            .save(output.join(format!("{name}.png")))
            .unwrap();
        std::fs::write(
            output.join(format!("{name}.f32le")),
            pixels
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    };
    save("legacy-scene", &before);
    save("atlas-scene", &after);
    let mut reports = Vec::new();
    let mut failures = 0;
    for (i, [nv, r]) in parameters.into_iter().enumerate() {
        let reference = if i == 0 {
            PIXEL_ORACLE[4].reference
        } else if i == 1 {
            PIXEL_ORACLE[13].reference
        } else {
            let b = (1. - f64::from(nv)).powi(5);
            [1. - b, b, 0.]
        };
        let expected = std::array::from_fn::<_, 3, _>(|c| {
            f64::from(COLOR[c]) * reference[0] + reference[1] + 0.02 * f64::from(COLOR[c])
        });
        let pixel = after[(H / 2 * W + i as u32 * 64 + 32) as usize];
        let old = before[(H / 2 * W + i as u32 * 64 + 32) as usize];
        let tolerance = expected.map(|v| 2f64.powf(v.abs().log2().floor() - 10.) + 1e-4 + 1e-6);
        let errors = std::array::from_fn::<_, 3, _>(|c| (f64::from(pixel[c]) - expected[c]).abs());
        let passed = (0..3).all(|c| errors[c] <= tolerance[c]);
        failures += usize::from(!passed);
        reports.push(serde_json::json!({"panel":i,"nv":nv,"authored_roughness":r,"expected_single_scattering_plus_ambient":expected,"legacy_pixel":old,"atlas_pixel":pixel,"absolute_error":errors,"half_storage_and_moment_budget":tolerance,"passed":passed}));
    }
    let report = serde_json::json!({"adapter":adapter.get_info().name,"status":if failures==0{"passed"}else{"failed"},"failures":failures,"format":"RGBA16Float","panels":["polished near-normal","rough grazing","exact mirror grazing"],"results":reports,"scope":"actual SceneRenderer geometry, environment and HDR attachment; single-scattering IBL only, no photoreal character or FPS claim"});
    std::fs::write(
        output.join("pixels.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    assert_eq!(
        failures,
        0,
        "actual scene material routing differs from independent IBL oracle; report {}",
        output.display()
    );
}
