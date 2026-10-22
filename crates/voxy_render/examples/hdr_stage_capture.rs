//! GPU regression for read-only diagnostic capture, not a performance benchmark.
use voxy_render::{ComputeMemoryBudget, ComputeStorage};
use wgpu::util::DeviceExt;
#[path = "../src/archive_hdr_diagnostics.rs"]
mod diagnostics;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("new output directory"));
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("HDR CAPTURE GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let halves: Vec<u16> = (0..=u16::MAX).filter(|v| v & 0x7c00 != 0x7c00).collect();
    assert_eq!(halves.len(), 63488);
    let half_bytes = bytemuck::cast_slice(&halves);
    let make = |label: &str, w, h, format, usage, bytes: &[u8], pixel_bytes| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: usage | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * pixel_bytes),
                rows_per_image: Some(h),
            },
            texture.size(),
        );
        texture
    };
    let sampled = make(
        "all finite fp16 source texels",
        128,
        124,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::TEXTURE_BINDING,
        half_bytes,
        8,
    );
    let native = make(
        "native fp16 output without sampled usage",
        128,
        124,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::COPY_SRC,
        half_bytes,
        8,
    );
    let motion = make(
        "native RG16 source",
        256,
        124,
        wgpu::TextureFormat::Rg16Float,
        wgpu::TextureUsages::COPY_SRC,
        half_bytes,
        4,
    );
    let patterns = [
        0u32, 0x80000000, 1, 0x80000001, 0x007fffff, 0x807fffff, 0x00800000, 0x3f800000,
        0x7f7fffff, 0xff7fffff, 0x7f800000, 0x7fc12345, 0xffc54321,
    ];
    let singles: Vec<u32> = (0..140).map(|i| patterns[i % patterns.len()]).collect();
    let float = make(
        "native f32 source retains subnormal and NaN payloads",
        7,
        5,
        wgpu::TextureFormat::Rgba32Float,
        wgpu::TextureUsages::COPY_SRC,
        bytemuck::cast_slice::<u32, u8>(&singles),
        16,
    );
    let words: Vec<u32> = (0..68u32)
        .map(|i| i.wrapping_mul(0x9e3779b9) ^ 0xa5a5a5a5)
        .collect();
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("original uniform input words"),
        contents: bytemuck::cast_slice::<u32, u8>(&words),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let proof = diagnostics::capture(
        &device,
        &queue,
        vec![
            ("sampled-half", sampled.create_view(&Default::default())),
            ("native-half", native.create_view(&Default::default())),
            ("native-motion", motion.create_view(&Default::default())),
            ("native-float", float.create_view(&Default::default())),
        ],
        Some(&uniform),
        &root,
        true,
    )?;
    let actual = std::fs::read(root.join("sampled-half.rgba32.bin"))?;
    assert_eq!(actual.len(), halves.len() * 4);
    for (index, (expected, got)) in halves.iter().zip(actual.chunks_exact(4)).enumerate() {
        assert_eq!(
            half::f16::from_bits(*expected).to_f32().to_bits(),
            u32::from_le_bytes(got.try_into().unwrap()),
            "finite half index {index}"
        );
    }
    assert_eq!(
        std::fs::read(root.join("native-half.native.bin"))?,
        half_bytes
    );
    assert_eq!(
        std::fs::read(root.join("native-motion.native.bin"))?,
        half_bytes
    );
    assert_eq!(
        std::fs::read(root.join("native-float.native.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&singles)
    );
    assert_eq!(
        std::fs::read(root.join("temporal-uniform.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&words)
    );
    assert_eq!(proof["stages"].as_array().unwrap().len(), 4);
    let storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("original storage words with subnormal and NaN payloads"),
        contents: bytemuck::cast_slice::<u32, u8>(&singles),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let storage_proof = diagnostics::capture_storage(
        &device,
        &queue,
        &storage,
        &root.join("native-storage.bin"),
        true,
    )?;
    assert_eq!(
        std::fs::read(root.join("native-storage.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&singles)
    );
    assert_eq!(storage_proof["source_bits_preserved_exactly"], true);
    assert!(
        diagnostics::capture_storage(
            &device,
            &queue,
            &uniform,
            &root.join("invalid-uniform-storage.bin"),
            true
        )
        .is_err()
    );

    let projection_words: Vec<u32> = (0..16).map(|i| patterns[i % patterns.len()]).collect();
    let camera_words: Vec<u32> = (0..112).map(|i| patterns[i % patterns.len()]).collect();
    let projection_source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("actual-sized probe projection uniform"),
        contents: bytemuck::cast_slice::<u32, u8>(&projection_words),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let camera_source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("actual-sized camera uniform"),
        contents: bytemuck::cast_slice::<u32, u8>(&camera_words),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let probe_words: Vec<u32> = (0..32).map(|i| patterns[i % patterns.len()]).collect();
    let probe_storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("two original coefficient records"),
        contents: bytemuck::cast_slice::<u32, u8>(&probe_words),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let face_path = root.join("probe-face");
    let face_view = sampled.create_view(&Default::default());
    diagnostics::capture_probe_face(
        &device,
        &queue,
        diagnostics::ProbeFaceSources {
            face_view: &face_view,
            uniform_inputs: Vec::new(),
            raster_inputs: vec![
                ("probe-test-world", float.create_view(&Default::default())),
                (
                    "probe-test-background",
                    native.create_view(&Default::default()),
                ),
            ],
            projection: &projection_source,
            camera: &camera_source,
            coefficients: &probe_storage,
            point_bits: [0, 0x80000000, 1],
            probe_index: 1,
            face_index: 5,
        },
        &face_path,
    )?;
    assert!(
        diagnostics::capture_probe_face(
            &device,
            &queue,
            diagnostics::ProbeFaceSources {
                face_view: &face_view,
                uniform_inputs: Vec::new(),
                raster_inputs: Vec::new(),
                projection: &projection_source,
                camera: &camera_source,
                coefficients: &probe_storage,
                point_bits: [0; 3],
                probe_index: 2,
                face_index: 0
            },
            &root.join("invalid-probe")
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(face_path.join("projection-uniform.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&projection_words)
    );
    assert_eq!(
        std::fs::read(face_path.join("camera-uniform.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&camera_words)
    );
    assert_eq!(
        std::fs::read(face_path.join("probe-sh.native.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&probe_words)
    );
    assert_eq!(
        std::fs::read(face_path.join("current-probe-sh.native.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&probe_words[16..32])
    );
    assert_eq!(
        std::fs::read(face_path.join("probe-face-radiance.rgba32.bin"))?,
        actual
    );
    assert_eq!(
        std::fs::read(face_path.join("probe-test-world.native.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&singles)
    );
    assert_eq!(
        std::fs::read(face_path.join("probe-test-background.native.bin"))?,
        half_bytes
    );
    let light_words: Vec<u32> = (0..1028u32)
        .map(|i| i.wrapping_mul(0x9e3779b9) ^ 0x7fc12345)
        .collect();
    let light_source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("full punctual light uniform words"),
        contents: bytemuck::cast_slice::<u32, u8>(&light_words),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    diagnostics::capture_uniform(
        &device,
        &queue,
        &light_source,
        &root.join("full-light-uniform.bin"),
    )?;
    assert_eq!(
        std::fs::read(root.join("full-light-uniform.bin"))?,
        bytemuck::cast_slice::<u32, u8>(&light_words)
    );
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("original depth observation fixture"),
        size: wgpu::Extent3d {
            width: 67,
            height: 5,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth_shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("varying original depth source"),source:wgpu::ShaderSource::Wgsl("@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.,1.);} @fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32 {return (floor(p.x)+67.*floor(p.y)+1.)/336.;}".into())});
    let depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("varying depth fixture"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &depth_shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &depth_shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    });
    for (index, value) in [0., 0.123456789, 0.5, 0.9999999, 1., 0.]
        .into_iter()
        .enumerate()
    {
        let view = depth.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("actual depth clear"),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(value),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if index == 5 {
                pass.set_pipeline(&depth_pipeline);
                pass.draw(0..3, 0..1);
            }
        }
        queue.submit([encoder.finish()]);
        let native_depth = diagnostics::native_output(&device, &queue, &depth)?;
        let dir = root.join(format!("depth-{index}"));
        diagnostics::capture(&device, &queue, vec![("depth", view)], None, &dir, true)?;
        let observed = std::fs::read(dir.join("depth.r32.bin"))?;
        if index == 5 {
            let unique: std::collections::HashSet<_> = native_depth.chunks_exact(4).collect();
            assert_eq!(
                unique.len(),
                335,
                "each source texel must differ to detect coordinate/readback errors"
            );
        }
        assert_eq!(native_depth.len(), 67 * 5 * 4);
        assert_eq!(observed.len(), 67 * 5 * 4);
        assert_eq!(observed, native_depth, "compact observer must preserve every original depth word");
    }
    let mut chain_subresources = 0usize;
    let mut chain_words = 0usize;
    for native_copy in [true, false] {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("distinct layered mip fixture"),
            size: wgpu::Extent3d {
                width: 7,
                height: 5,
                depth_or_array_layers: 6,
            },
            mip_level_count: 3,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | if native_copy {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            view_formats: &[],
        });
        let mut expected = Vec::new();
        for layer in 0..6u32 {
            for mip in 0..3u32 {
                let w = (7u32 >> mip).max(1);
                let h = (5u32 >> mip).max(1);
                let values: Vec<u32> = (0..w * h * 4)
                    .map(|i| {
                        if native_copy && i % 13 == 0 {
                            patterns[(i as usize / 13) % patterns.len()]
                        } else {
                            ((layer * 10000 + mip * 1000 + i + 1) as f32 / 128.).to_bits()
                        }
                    })
                    .collect();
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: mip,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: layer,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytemuck::cast_slice::<u32, u8>(&values),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(w * 16),
                        rows_per_image: Some(h),
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
                expected.push(values);
            }
        }
        let dir = root.join(if native_copy {
            "native-mip-chain"
        } else {
            "sampled-mip-chain"
        });
        let chain = diagnostics::capture_texture_chain(&device, &queue, &texture, &dir)?;
        let records = chain["subresources"].as_array().unwrap();
        assert_eq!(records.len(), 18);
        for (record, words) in records.iter().zip(expected) {
            assert_eq!(
                std::fs::read(dir.join(record["file"].as_str().unwrap()))?,
                bytemuck::cast_slice::<u32, u8>(&words)
            );
            chain_words += words.len();
            chain_subresources += 1;
        }
    }
    let mut normalized_channels = 0usize;
    let normalized = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("all normalized byte values across layers and mips"),
        size: wgpu::Extent3d {
            width: 17,
            height: 9,
            depth_or_array_layers: 3,
        },
        mip_level_count: 3,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut expected_bytes = Vec::new();
    for layer in 0..3u32 {
        for mip in 0..3u32 {
            let w = (17u32 >> mip).max(1);
            let h = (9u32 >> mip).max(1);
            let values: Vec<u8> = (0..w * h * 4)
                .map(|i| i.wrapping_mul(71).wrapping_add(layer * 37 + mip * 17) as u8)
                .collect();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &normalized,
                    mip_level: mip,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &values,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            expected_bytes.push(values);
        }
    }
    let dir = root.join("normalized-byte-chain");
    let chain = diagnostics::capture_texture_chain(&device, &queue, &normalized, &dir)?;
    for (record, original) in chain["subresources"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected_bytes)
    {
        assert_eq!(record["source_bits_preserved_exactly"], false);
        let observed = std::fs::read(dir.join(record["file"].as_str().unwrap()))?;
        assert_eq!(observed.len(), original.len() * 4);
        for (word, expected) in observed.chunks_exact(4).zip(original) {
            let value = f32::from_le_bytes(word.try_into().unwrap());
            assert!(value.is_finite() && (0.0..=1.0).contains(&value));
            assert_eq!((value * 255.).round() as u8, expected);
            normalized_channels += 1;
        }
    }
    let summary = serde_json::json!({"status":"actual_GPU_diagnostic_capture_preserves_all_finite_fp16_and_native_f32_words","normalized_source_channels_recovered_exactly":normalized_channels,"layered_mip_subresources_compared":chain_subresources,"layered_mip_words_compared":chain_words,"depth32_observed_words_compared_to_native_copy":2010,"compact_depth_readback_preserves_all_original_words":true,"distinct_depth_source_texels":335,"finite_fp16_sampled_channels":63488,"native_fp16_channels":63488,"native_RG16_channels":63488,"native_f32_channels_with_subnormal_and_NaN_payloads":140,"original_GPU_uniform_words":68,"multi_workgroup_light_uniform_words":1028,"native_storage_words":140,"probe_face_composite_capture_passed":true,"probe_raster_input_views_capture_passed":true,"nonzero_probe_record_slice_words":16,"invalid_probe_record_rejected":true,"actual_sized_projection_uniform_words":16,"actual_sized_camera_uniform_words":112,"invalid_non_COPY_SRC_storage_rejected":true,"expected_outputs_uploaded":false,"full_scene_HDR_instability_fixed":false,"minimum_120_fps_qualified":false});
    std::fs::write(
        root.join("proof.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{summary}");
    Ok(())
}
