//! Exact affine receiver image-hit fixtures for GI and SSR consumer validation.
use super::*;

// A real small receiver sits on a reflected live ray at exactly t=1/64.
// Its affine source transform represents origin + direction*(60/64) exactly;
// it does not cover that ray's primary raster pixel. No hit record is injected.
pub(super) fn exact_image_hit_fixture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    live_ray: &[u8],
    output: &wgpu::Texture,
    root: &Path,
    optical: bool,
    one_sided: bool,
    tail: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    exact_image_hit_fixture_impl(
        device, queue, live_ray, output, root, optical, one_sided, tail, false,
    )
}
#[allow(clippy::fn_params_excessive_bools)]
fn exact_image_hit_fixture_impl(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    live_ray: &[u8],
    output: &wgpu::Texture,
    root: &Path,
    optical: bool,
    one_sided: bool,
    tail: bool,
    canonicalized: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let position_consumer =
        std::env::var("VOXY_RAY_SHARED_POSITION_CONSUMER").as_deref() == Ok("1");
    let residual_position = position_consumer
        && tail
        && std::env::var("VOXY_RAY_SHARED_POSITION_RESIDUAL").as_deref() == Ok("1");
    let words: Vec<u32> = live_ray
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    let origin = Vec3::from_array(std::array::from_fn(|axis| f32::from_bits(words[axis])));
    let direction = Vec3::from_array(std::array::from_fn(|axis| {
        ((f64::from(f32::from_bits(words[8 + axis])) + f64::from(f32::from_bits(words[12 + axis])))
            / 60.) as f32
    }));
    let mesh = |points: [[f32; 3]; 4], normal: [f32; 3], world: Mat4| ArchiveMeshSource {
        vertices: points
            .into_iter()
            .map(|position| ArchiveVertex {
                position,
                normal,
                tangent: [1., 0., 0., 1.],
                uv: [0.5; 2],
            })
            .collect(),
        indices: vec![0, 1, 2, 0, 2, 3],
        instances: vec![ArchiveInstance::new(world)],
        material: 0,
        center: Vec3::ZERO,
    };
    let front = mesh(
        [[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]],
        [0., 0., 1.],
        Mat4::IDENTITY,
    );
    let corners = if tail {
        [
            [-0.00390625, -0.001953125],
            [0.00390625, -0.001953125],
            [0.00390625, 0.00390625],
            [-0.00390625, 0.00390625],
        ]
    } else {
        [[-0.05, -0.04], [0.06, -0.04], [0.06, 0.07], [-0.05, 0.07]]
    };
    let mut receiver = mesh(
        corners.map(|xy| [direction.x + xy[0], direction.y + xy[1], direction.z]),
        [0., 0., -1.],
        Mat4::from_scale_rotation_translation(
            Vec3::splat(if tail { 1.5 } else { 0.9375 }),
            glam::Quat::IDENTITY,
            origin,
        ),
    );
    if residual_position {
        for v in &mut receiver.vertices {
            v.position[2] += 0.0009765625;
        }
    }
    let position_tilt = std::env::var("VOXY_RAY_SHARED_POSITION_TILT_NUMERATOR")
        .ok()
        .map(|v| v.parse::<u16>())
        .transpose()?
        .unwrap_or(0);
    if residual_position && position_tilt != 0 {
        let slope = f32::from(position_tilt) / 16.;
        for v in &mut receiver.vertices {
            v.position[2] += (v.position[0] - direction.x) * slope;
        }
    }
    receiver.material = usize::from(optical || one_sided);
    if position_consumer && one_sided {
        receiver.indices = vec![0, 2, 1, 0, 3, 2];
    }
    let meshes = vec![front, receiver];
    let mut materials = vec![ArchiveMaterialSource {
        parameters: ArchiveMaterial::MIRROR,
        images: [None; 4],
        double_sided: true,
    }];
    if optical || one_sided {
        let mut glass = ArchiveMaterial::MIRROR;
        if optical {
            glass.surface[2] = 1.;
        }
        materials.push(ArchiveMaterialSource {
            parameters: glass,
            images: [None; 4],
            double_sided: !one_sided,
        });
    }
    if one_sided {
        materials[0].double_sided = false;
    }
    let prefix_base = if optical {
        "optical-image-hit"
    } else if one_sided {
        "one-sided-image-hit"
    } else if tail {
        "tail-image-hit"
    } else {
        "exact-image-hit"
    };
    let prefix = if residual_position {
        "position-residual-image-hit".to_owned()
    } else if position_consumer {
        format!("position-{prefix_base}")
    } else {
        prefix_base.to_owned()
    };
    let history_admission = !position_consumer
        && std::env::var("VOXY_RAY_SHARED_HISTORY_ADMISSION").as_deref() == Ok("1");
    let scene = Arc::new(archive_source_scene(device, &meshes)?);
    if std::env::var("VOXY_RAY_SHARED_HISTORY_POSITION_ADMISSION").as_deref() == Ok("1") {
        for (kind, point) in [
            (voxy_render::WorldRayQueryKind::Nearest, true),
            (voxy_render::WorldRayQueryKind::HistoryAdmission, true),
            (voxy_render::WorldRayQueryKind::OpaqueVisibility, true),
            (voxy_render::WorldRayQueryKind::MaterialSurface, false),
        ] {
            let bad = ExactWorldNearest::with_options(
                device,
                scene.clone(),
                &materials,
                0,
                16,
                ExactWorldOptions {
                    query_kind: kind,
                    hit_position: point,
                    history_position_admission: true,
                    ..Default::default()
                },
            );
            assert!(
                matches!(bad, Err(voxy_render::RaySceneError::Unsupported)),
                "history position admission escaped its explicit render-only scope"
            );
        }
    }
    let position_exact_vertex_cache =
        std::env::var("VOXY_RAY_SHARED_POSITION_EXACT_VERTEX_CACHE").as_deref() == Ok("1");
    let position_cache_not_ready =
        std::env::var("VOXY_RAY_SHARED_POSITION_VERTEX_CACHE_NOT_READY").as_deref() == Ok("1");
    let position_native_seed =
        std::env::var("VOXY_RAY_SHARED_POSITION_NATIVE_SEED").as_deref() == Ok("1");
    let job = Arc::new(ExactWorldNearest::with_options(
        device,
        scene,
        &materials,
        0,
        16,
        ExactWorldOptions {
            wide_source_nodes: std::env::var("VOXY_RAY_SHARED_POSITION_WIDE_NODES").as_deref()
                == Ok("1"),
            skip_zero_prefix_multiply: std::env::var(
                "VOXY_RAY_SHARED_POSITION_SKIP_ZERO_PREFIX_MULTIPLY",
            )
            .as_deref()
                == Ok("1"),
            cached_relative_interval: std::env::var(
                "VOXY_RAY_SHARED_POSITION_CACHED_RELATIVE_INTERVAL",
            )
            .as_deref()
                == Ok("1"),
            geometry_bounds_cache: std::env::var("VOXY_RAY_SHARED_POSITION_GEOMETRY_BOUNDS_CACHE")
                .as_deref()
                == Ok("1"),
            fused_source_affine: std::env::var("VOXY_RAY_SHARED_POSITION_FUSED_SOURCE_AFFINE")
                .as_deref()
                == Ok("1"),
            exact_vertex_cache: position_exact_vertex_cache,
            native_seed: position_native_seed,
            separate_native_seed: position_native_seed,
            native_triangle_hint: std::env::var("VOXY_RAY_SHARED_POSITION_NATIVE_TRIANGLE_HINT")
                .as_deref()
                == Ok("1"),
            certified_ratio_seed: std::env::var("VOXY_RAY_SHARED_CERTIFIED_RATIO_SEED").as_deref()
                == Ok("1"),
            history_position_admission: std::env::var("VOXY_RAY_SHARED_HISTORY_POSITION_ADMISSION")
                .as_deref()
                == Ok("1"),
            hit_position: position_consumer,
            query_kind: if position_consumer {
                voxy_render::WorldRayQueryKind::MaterialSurface
            } else if history_admission {
                voxy_render::WorldRayQueryKind::HistoryAdmission
            } else {
                voxy_render::WorldRayQueryKind::Nearest
            },
            history_any_hit: !position_consumer
                && std::env::var("VOXY_RAY_SHARED_ANY_HIT_ADMISSION").as_deref() == Ok("1"),
            guarded_input: true,
            certify_visibility: true,
            defer_nearest: true,
            ..Default::default()
        },
    )?);
    if std::env::var("VOXY_RAY_SHARED_POSITION_WIDE_NODES").as_deref() == Ok("1") {
        assert!(
            job.wide_source_nodes_enabled(),
            "small consumer fixture must actually admit BVH8"
        );
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    job.build_scene(&mut encoder);
    if position_cache_not_ready {
        let buffer = job
            .exact_vertex_cache_buffer()
            .ok_or("cache not-ready fixture requires an enabled vertex cache")?;
        let header = job
            .exact_vertex_cache_header()
            .ok_or("vertex cache header missing")?;
        encoder.clear_buffer(buffer, u64::from(header) * 4, Some(4));
    }
    queue.submit([encoder.finish()]);
    let vertex_cache_diagnostics = if position_exact_vertex_cache {
        let buffer = job
            .exact_vertex_cache_buffer()
            .ok_or("enabled cache missing")?;
        let header = job
            .exact_vertex_cache_header()
            .ok_or("cache header missing")? as usize;
        let bytes = read(device, queue, buffer, buffer.size())?;
        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        let h = &words[header..header + 16];
        assert_eq!(
            (h[0], h[4], h[5], h[6]),
            (u32::from(!position_cache_not_ready), 12, 0, h[2])
        );
        let admitted = (0..h[2] as usize)
            .filter(|&i| (0..3).all(|axis| words[h[3] as usize + i * 12 + axis * 4] & 32768 != 0))
            .count();
        assert!(
            admitted > 0,
            "directed fixture must exercise admitted lossless records"
        );
        Some(
            serde_json::json!({"ready":h[0],"world_vertices":h[2],"checked_all":h[6],"mismatches":h[5],"admitted_vertices":admitted}),
        )
    } else {
        None
    };
    let source_geometry:Vec<_>=meshes.iter().map(|m|serde_json::json!({"vertices":m.vertices.iter().map(|v|v.position.map(f32::to_bits)).collect::<Vec<_>>(),"world":m.instances[0].world.map(|r|r.map(f32::to_bits)),"indices":m.indices,"material":m.material})).collect();
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
        (Vec3::splat(-2.), Vec3::splat(4.)),
    )?;
    for _ in 0..2 {
        renderer.draw(
            &output.create_view(&Default::default()),
            Vec3::new(0.1, 0., 3.),
            -Vec3::Z,
        );
    }
    renderer.add_world_ray_region(job.clone(), WorldRayKind::Reflection, [12, 12], [4, 4])?;
    renderer.draw(
        &output.create_view(&Default::default()),
        Vec3::new(0.1, 0., 3.),
        -Vec3::Z,
    );
    let input = read(device, queue, job.input(), 16 * 80)?;
    let actual_words: Vec<u32> = input[..80]
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect();
    let same = (0..4)
        .all(|part| (0..3).all(|axis| actual_words[part * 4 + axis] == words[part * 4 + axis]));
    if !same {
        assert!(
            !canonicalized,
            "canonical front receiver ray did not stabilize"
        );
        // A subdivided parent scene can rasterize a slightly different origin.
        // Seed this directed receiver from ITS actual GPU front-quad ray instead
        // of weakening the exact t assertion or discarding paired low words.
        drop(renderer);
        drop(job);
        return exact_image_hit_fixture_impl(
            device,
            queue,
            &input[..80],
            output,
            root,
            optical,
            one_sided,
            tail,
            true,
        );
    }
    let hits = read(device, queue, job.output(), 16 * 80)?;
    let first: Vec<u32> = hits[..80]
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    let position_admitted = first[0] == 2 && first[1] == 16384;
    if position_admitted {
        assert!(position_consumer && residual_position);
        assert_ne!(first[18] & 131072, 0);
    } else if history_admission && (optical || one_sided || tail) {
        assert_eq!(first[0], 2);
        assert_eq!(
            first[1],
            if one_sided && job.history_any_hit_enabled() {
                8192
            } else if optical {
                2048
            } else if one_sided {
                1024
            } else {
                4096
            }
        );
    } else {
        assert_eq!(first[0], 1, "live reflected ray must hit the receiver");
        assert_eq!(first[6], 1, "source identity must be the receiver");
        if residual_position {
            assert_ne!(first[15] & 1, 0);
        } else if tail {
            assert_eq!(first[8], 0x3ccccccd);
            assert_eq!(first[15] & 1, 1);
        } else {
            assert_eq!(
                [first[8], first[9], first[15] & 1],
                [0x3c800000, 0, 0],
                "receiver must have exact t=1/64 with no remaining rational tail"
            );
        }
    }
    let counts = read(
        device,
        queue,
        renderer.world_ray_region_audit_buffer(0).unwrap(),
        32,
    )?;
    let counts: Vec<u32> = counts
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    assert_eq!(counts.iter().sum::<u32>(), 16);
    if position_consumer {
        let position = read(
            device,
            queue,
            job.output(),
            job.world_position_offset_bytes().unwrap() + 16 * 32,
        )?;
        let position = &position[job.world_position_offset_bytes().unwrap() as usize..];
        let first_position: Vec<u32> = position[..32]
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        if position_admitted {
            assert_eq!(first_position, vec![0; 8]);
        } else {
            assert_eq!(first_position[6], 7);
            if residual_position {
                assert_ne!(
                    first_position[7], 0,
                    "negative fixture must retain an actual rational world-position tail"
                );
            } else {
                assert_eq!(
                    first_position[7], 0,
                    "direct fixture must have a fully represented exact world point"
                );
            }
        }
        std::fs::write(root.join(format!("{prefix}-position.bin")), position)?;
        if residual_position {
            assert_eq!(counts[2], 0);
            assert!(counts[3] + counts[5] > 0);
        } else if optical {
            assert_eq!(counts[2], 0);
            assert!(counts[4] > 0);
        } else {
            assert!(
                counts[2] > 0,
                "exact point and material-sided source must feed radiance"
            );
        }
    } else if optical || one_sided || tail {
        assert!(
            counts[if history_admission {
                5
            } else if optical {
                4
            } else if tail {
                3
            } else {
                7
            }] > 0,
            "unsupported source hit must retain full original handling"
        );
        assert_eq!(counts[2], 0);
    } else {
        assert!(
            counts[2] > 0,
            "source hit must reach actual radiance reconstruction"
        );
    }
    image_dump(
        device,
        queue,
        output,
        8,
        &root.join(format!("{prefix}-native.bin")),
    )?;
    image_dump(
        device,
        queue,
        renderer
            .opaque_gi_precision_textures()
            .ok_or("GI precision texture required")?
            .0,
        16,
        &root.join(format!("{prefix}-gi.bin")),
    )?;
    image_dump(
        device,
        queue,
        renderer
            .opaque_ssr_precision_textures()
            .ok_or("SSR precision texture required")?
            .0,
        16,
        &root.join(format!("{prefix}-ssr.bin")),
    )?;
    std::fs::write(root.join(format!("{prefix}-input.bin")), input)?;
    std::fs::write(root.join(format!("{prefix}-output.bin")), hits)?;
    std::fs::write(
        root.join(format!("{prefix}-fixture.json")),
        serde_json::to_vec_pretty(
            &serde_json::json!({"wide_source_nodes":job.wide_source_nodes_enabled(),"wide_source_block_blake3":job.wide_source_digest(),"skip_zero_prefix_multiply":job.skip_zero_prefix_multiply_enabled(),"exact_integer_source_blake3":job.exact_integer_source_digest(),"fused_source_affine":std::env::var("VOXY_RAY_SHARED_POSITION_FUSED_SOURCE_AFFINE").as_deref()==Ok("1"),"exact_vertex_cache_diagnostics":vertex_cache_diagnostics,"exact_vertex_cache":position_exact_vertex_cache,"exact_vertex_cache_ready_cleared":position_cache_not_ready,"native_seed":position_native_seed,"native_triangle_hint":std::env::var("VOXY_RAY_SHARED_POSITION_NATIVE_TRIANGLE_HINT").as_deref()==Ok("1"),"history_position_admission":std::env::var("VOXY_RAY_SHARED_HISTORY_POSITION_ADMISSION").as_deref()==Ok("1"),"first_position_admitted":position_admitted,"residual_position":residual_position,"position_consumer":position_consumer,"source_geometry":source_geometry,"expected_first_t_words":if tail {None}else{Some([0x3c800000u32,0])},"expected_first_t_rational":if residual_position {"derive-from-original-source"}else if tail {"1/40"}else{"1/64"},"branch_counts":counts,"image_size":[32,32],"region":[12,12,4,4],"optical":optical,"one_sided":one_sided,"tail":tail,"history_admission":history_admission,"any_hit_admission_enabled":job.history_any_hit_enabled(),"full_street_qualified":false}),
        )?,
    )?;
    if one_sided
        && !position_consumer
        && std::env::var("VOXY_RAY_SHARED_POSITIVE_CACHE").as_deref() == Ok("1")
    {
        assert!(
            job.history_any_hit_enabled(),
            "positive cache fixture requires material fallback admission"
        );
        assert_eq!(
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref(),
            Ok("1")
        );
        renderer.clear_world_ray_regions();
        renderer.set_world_ray_frame(job.clone(), [4, 4])?;
        assert_eq!(
            renderer.paired_dda_tile_edge_cache_enabled(),
            std::env::var("VOXY_STREET_DDA_TILE_EDGE_CACHE").as_deref() == Ok("1")
        );
        assert_eq!(
            renderer.world_ray_consumer_sorted(),
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_SORTED").as_deref() == Ok("1"),
            "prepared frame did not capture requested sorted consumer mode"
        );
        for _ in 0..3 {
            renderer.draw(
                &output.create_view(&Default::default()),
                Vec3::new(0.1, 0., 3.),
                -Vec3::Z,
            );
        }
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        let mut positive = Vec::new();
        for (kind, pair) in [
            (
                "gi",
                renderer
                    .opaque_gi_precision_textures()
                    .ok_or("positive GI reference missing")?,
            ),
            (
                "ssr",
                renderer
                    .opaque_ssr_precision_textures()
                    .ok_or("positive SSR reference missing")?,
            ),
        ] {
            let actual = root.join(format!("positive-cache-{kind}-actual.bin"));
            let reference = root.join(format!("positive-cache-{kind}-reference.bin"));
            image_dump(device, queue, pair.0, 16, &actual)?;
            image_dump(device, queue, pair.1, 16, &reference)?;
            let bytes = std::fs::read(actual)?;
            assert_eq!(
                bytes,
                std::fs::read(reference)?,
                "full positive cached {kind} differs from independent inline reference"
            );
            let count = bytes
                .chunks_exact(16)
                .filter(|pixel| {
                    let rgba: Vec<f32> = pixel
                        .chunks_exact(4)
                        .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
                        .collect();
                    rgba[3] > 0. && rgba[..3].iter().any(|&v| v != 0.)
                })
                .count();
            assert!(
                count > 0,
                "positive cache fixture has no nonzero {kind} radiance"
            );
            positive.push(count);
        }
        let mut totals = [0u64; 4];
        let mut counters = Vec::new();
        for region in 0..renderer.world_ray_region_count() {
            let bytes = read(
                device,
                queue,
                renderer.world_ray_region_audit_buffer(region).unwrap(),
                48,
            )?;
            let words: Vec<u32> = bytes
                .chunks_exact(4)
                .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                .collect();
            assert_eq!(words[10], 0, "positive cache Float32 mismatch");
            assert_eq!(words[11], 0);
            for i in 0..4 {
                totals[i] += u64::from(words[8 + i]);
            }
            counters.push(words);
        }
        assert!(
            totals[1] > totals[0],
            "positive cached contributions were not checked"
        );
        std::fs::write(
            root.join("positive-cache-proof.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"image_size":[32,32],"tile":[4,4],"regions":renderer.world_ray_region_count(),"positive_rgb_alpha_pixels_gi_ssr":positive,"images_match_independent_inline_bits":true,"checked_nonpruned_contributions":totals[1]-totals[0],"totals_pruned_checked_mismatch_nonfinite":totals,"raw_region_counters":counters,"full_street_qualified":false,"fps_qualified":false}),
            )?,
        )?;
        println!(
            "POSITIVE CONSUMER CACHE PASS positive_gi={} positive_ssr={} checked_nonpruned={} full_images_bit_exact=true",
            positive[0],
            positive[1],
            totals[1] - totals[0]
        );
    }
    println!(
        "WORLD IMAGE HIT PASS optical={optical} original_live_ray=true source_affine_geometry=true exact_t={} radiance_branch_count={} optical_fallback_count={} no_uploaded_hit_record=true",
        if residual_position {
            "derive-from-original-source"
        } else if tail {
            "1/40"
        } else {
            "1/64"
        },
        counts[2],
        counts[4]
    );
    Ok(())
}
