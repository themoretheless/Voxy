use super::*;

#[test]
#[ignore = "requires GPU; scene ABI deformation readback"]
fn gpu_scene_skinning_independent_instances_and_last_good_pose() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(gpu.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let asset = crate::ModelAsset::parse(
        include_bytes!("../../../examples/assets/animated-triangle.glb"),
        &[],
        crate::ModelLimits::default(),
    )
    .unwrap();
    let crate::ModelGeometry::Skinned(mesh) = &asset.primitives[0].geometry else {
        panic!("skin required")
    };
    let source = skinner
        .upload_source(Arc::new(mesh.clone()), 0, 4096)
        .unwrap();
    assert_eq!(source.allocation_bytes(), mesh.vertices().len() as u64 * 64);
    let palette = asset
        .skin_matrices(&asset.sample_pose(None, 0.).unwrap())
        .unwrap();
    let color = [0.25, 0.5, 0.75, 1.];
    let first = skinner
        .create_instance(
            &renderer,
            source.clone(),
            &palette,
            color,
            source.allocation_bytes(),
            4096,
        )
        .unwrap();
    let live = source.allocation_bytes() + first.allocation_bytes();
    assert!(matches!(
        skinner.create_instance(&renderer, source.clone(), &palette, color, live, live),
        Err(SceneSkinError::BudgetExceeded { .. })
    ));
    let second = skinner
        .create_instance(&renderer, source.clone(), &palette, color, live, 4096)
        .unwrap();
    assert!(Arc::ptr_eq(&first.source, &second.source));
    assert!(!std::sync::Arc::ptr_eq(
        &first.geometry.vertices,
        &second.geometry.vertices
    ));
    assert!(!Arc::ptr_eq(&first.palette, &second.palette));
    let read = |instance: &SceneSkinInstance, pose: Option<&[Mat4]>| {
        let size = instance.geometry.vertices.size();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        if let Some(pose) = pose {
            skinner
                .encode_pose(&queue, &mut encoder, instance, pose)
                .unwrap();
        }
        encoder.copy_buffer_to_buffer(&instance.geometry.vertices, 0, &staging, 0, size);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = staging.slice(..).get_mapped_range().unwrap();
        let vertices = bytemuck::cast_slice::<u8, SceneVertex>(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        vertices
    };
    let mut results = Vec::new();
    for (instance, time) in [(&first, 0.25), (&second, 0.75), (&first, 0.25)] {
        let pose = asset.sample_pose(Some(0), time).unwrap();
        let palette = asset.skin_matrices(&pose).unwrap();
        let actual = read(instance, Some(&palette));
        let expected = mesh
            .posed_scene_mesh(&palette, Mat4::IDENTITY, color)
            .unwrap();
        for (a, b) in actual.iter().zip(expected.vertices()) {
            assert!(
                glam::Vec3::from_array(a.position)
                    .abs_diff_eq(glam::Vec3::from_array(b.position), 1e-5)
            );
            assert_eq!(a.uv, b.uv);
            assert_eq!(a.color, b.color);
        }
        results.push(actual);
    }
    assert_eq!(results[0], results[2]);
    assert_ne!(results[0], results[1]);
    // Both outputs use the actual scene draw pipeline, with the CPU reference
    // in the neighboring viewport. This exercises the unchanged scene ABI.
    let reference = renderer
        .upload_mesh(
            &device,
            &SceneMesh::new(results[0].clone(), mesh.indices().to_vec()).unwrap(),
        )
        .unwrap();
    let white = renderer
        .upload_texture(&device, &queue, 1, 1, &[255; 4])
        .unwrap();
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for vertex in &results[0] {
        let p = glam::Vec3::from_array(vertex.position);
        min = min.min(p);
        max = max.max(p);
    }
    let center = (min + max) * 0.5;
    let scale = 1.5 / (max.x - min.x).max(max.y - min.y);
    let transform = renderer
        .create_transform(
            &device,
            Mat4::from_translation(glam::Vec3::new(0., 0., 0.5))
                * Mat4::from_scale(glam::Vec3::splat(scale))
                * Mat4::from_translation(-center),
        )
        .unwrap();
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 64,
                height: 32,
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
    let color_target = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let left = [SceneDraw {
        geometry: first.geometry(),
        texture: &white,
        transform: &transform,
        overlay: true,
    }];
    let right = [SceneDraw {
        geometry: &reference,
        texture: &white,
        transform: &transform,
        overlay: true,
    }];
    let mut encoder = device.create_command_encoder(&Default::default());
    let other = SceneSkinner {
        device: device.clone(),
        pipeline: skinner.pipeline.clone(),
        identity: Arc::new(()),
    };
    assert!(matches!(
        other.encode_pose(&queue, &mut encoder, &first, &palette),
        Err(SceneSkinError::ForeignSkinner)
    ));
    renderer
        .encode_views(
            &mut encoder,
            &color_target.create_view(&Default::default()),
            &depth.create_view(&Default::default()),
            wgpu::Color::BLACK,
            &[
                SceneView {
                    viewport: [0, 0, 32, 32],
                    draws: &left,
                },
                SceneView {
                    viewport: [32, 0, 32, 32],
                    draws: &right,
                },
            ],
        )
        .unwrap();
    let pixels = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 32,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        color_target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &pixels,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(32),
            },
        },
        wgpu::Extent3d {
            width: 64,
            height: 32,
            depth_or_array_layers: 1,
        },
    );
    let submitted = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    pixels
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = pixels.slice(..).get_mapped_range().unwrap();
    let mut colored = 0;
    for y in 0..32 {
        for x in 0..32 {
            let a = &mapped[y * 256 + x * 4..y * 256 + x * 4 + 4];
            let b = &mapped[y * 256 + (x + 32) * 4..y * 256 + (x + 32) * 4 + 4];
            assert_eq!(a, b);
            colored += usize::from(a[0] > 0);
        }
    }
    assert!(colored > 50);
    drop(mapped);
    pixels.unmap();
    let mut encoder = device.create_command_encoder(&Default::default());
    assert!(matches!(
        skinner.encode_pose(&queue, &mut encoder, &first, &[]),
        Err(SceneSkinError::Pose(_))
    ));
    let bad = vec![Mat4::from_cols_array(&[f32::NAN; 16]); palette.len()];
    assert!(matches!(
        skinner.encode_pose(&queue, &mut encoder, &first, &bad),
        Err(SceneSkinError::Pose(_))
    ));
    queue.submit([encoder.finish()]);
    assert_eq!(read(&first, None), results[0]);
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires GPU; skeletal LOD shared deformation streams"]
fn gpu_skeletal_lod_shares_pose_streams_and_rejects_failed_admission() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let lod = crate::skinned_lod::tests::source();
    let source = skinner
        .upload_source(Arc::new(lod.mesh().clone()), 0, 8192)
        .unwrap();
    let mut palette = vec![Mat4::IDENTITY; 2];
    let instance = skinner
        .create_instance(
            &renderer,
            source.clone(),
            &palette,
            [1.; 4],
            source.allocation_bytes(),
            8192,
        )
        .unwrap();
    let live = source.allocation_bytes() + instance.allocation_bytes();
    assert!(matches!(
        skinner.create_lod_level(&instance, &lod, 1, live, live + 11),
        Err(SceneSkinError::BudgetExceeded { .. })
    ));
    assert!(
        skinner
            .create_lod_level(&instance, &lod, 99, live, 8192)
            .is_err()
    );
    let level = skinner
        .create_lod_level(&instance, &lod, 1, live, live + 12)
        .unwrap();
    assert_eq!(level.index_allocation_bytes(), 12);
    assert_eq!(instance.geometry.index_count(), 6);
    assert_eq!(level.geometry().index_count(), 3);
    assert!(std::sync::Arc::ptr_eq(
        &level.geometry.vertices,
        &instance.geometry.vertices
    ));
    assert!(std::sync::Arc::ptr_eq(
        &level.geometry.normals,
        &instance.geometry.normals
    ));
    assert!(std::sync::Arc::ptr_eq(
        &level.geometry.material_coordinates,
        &instance.geometry.material_coordinates
    ));
    assert!(!std::sync::Arc::ptr_eq(
        &level.geometry.indices,
        &instance.geometry.indices
    ));
    let other = SceneSkinner::new(&renderer).unwrap();
    assert!(matches!(
        other.create_lod_level(&instance, &lod, 1, live, 8192),
        Err(SceneSkinError::ForeignSkinner)
    ));
    let mut changed_vertices = lod.mesh().vertices().to_vec();
    changed_vertices[0].position[0] += 0.1;
    let changed = crate::SkinnedLodMesh::new(
        Arc::new(
            crate::SkinnedMesh::new(changed_vertices, lod.mesh().indices().to_vec(), 2).unwrap(),
        ),
        vec![],
    )
    .unwrap();
    assert!(
        skinner
            .create_lod_level(&instance, &changed, 0, live, 8192)
            .is_err()
    );
    palette[1] = Mat4::from_translation(glam::Vec3::new(0., 0., 0.5));
    let bytes = level.geometry.vertices.size();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    skinner
        .encode_pose(&queue, &mut encoder, &instance, &palette)
        .unwrap();
    encoder.copy_buffer_to_buffer(&level.geometry.vertices, 0, &staging, 0, bytes);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = staging.slice(..).get_mapped_range().unwrap();
    let actual = bytemuck::cast_slice::<u8, SceneVertex>(&mapped);
    let expected = lod
        .mesh()
        .posed_scene_mesh(&palette, Mat4::IDENTITY, [1.; 4])
        .unwrap();
    for (actual, expected) in actual.iter().zip(expected.vertices()) {
        for (a, b) in actual.position.iter().zip(expected.position) {
            assert!((a - b).abs() < 1e-6);
        }
    }
    drop(mapped);
    staging.unmap();
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires GPU; upstream 19-bone rig and host timing diagnostics"]
fn gpu_reference_rig_full_clip_matches_cpu_positions() {
    reference_rig_gpu_acceptance(
        include_bytes!("../../../examples/assets/rigged-figure/RiggedFigure.glb"),
        "reference-rig",
        370,
        22,
        1,
    );
}

#[test]
#[ignore = "requires GPU; 24-bone Fox with Survey, Walk and Run"]
fn gpu_fox_all_clips_match_cpu_and_render() {
    reference_rig_gpu_acceptance(
        include_bytes!("../../../examples/assets/fox/Fox.glb"),
        "fox",
        1728,
        26,
        3,
    );
}

#[test]
#[ignore = "requires GPU; STEP interpolation through the skeletal renderer"]
fn gpu_reference_rig_step_clips_match_cpu_and_render() {
    let glb = step_glb(include_bytes!(
        "../../../examples/assets/rigged-figure/RiggedFigure.glb"
    ));
    reference_rig_gpu_acceptance(&glb, "reference-rig-step", 370, 22, 1);
}

#[test]
#[ignore = "requires GPU; intermediate STEP keys in Survey, Walk and Run"]
fn gpu_fox_step_clips_match_cpu_and_render() {
    let glb = step_glb(include_bytes!("../../../examples/assets/fox/Fox.glb"));
    reference_rig_gpu_acceptance(&glb, "fox", 1728, 26, 3);
}

#[test]
#[ignore = "requires GPU; cubic rig pose and normal comparison"]
fn gpu_reference_rig_cubic_clips_match_cpu_and_render() {
    let original = include_bytes!("../../../examples/assets/rigged-figure/RiggedFigure.glb");
    let source = gltf::Gltf::from_slice(original).unwrap();
    let mut binary = source.blob.unwrap();
    let json_size = u32::from_le_bytes(original[12..16].try_into().unwrap()) as usize;
    let mut document: serde_json::Value =
        serde_json::from_slice(&original[20..20 + json_size]).unwrap();
    let mut outputs = std::collections::BTreeMap::new();
    for animation in document["animations"].as_array().unwrap() {
        for sampler in animation["samplers"].as_array().unwrap() {
            let index = sampler["output"].as_u64().unwrap() as usize;
            outputs.insert(index, ());
        }
    }
    for index in outputs.keys() {
        let mut accessor = document["accessors"][*index].clone();
        assert_eq!(accessor["componentType"], 5126);
        let view = &document["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
        let count = accessor["count"].as_u64().unwrap() as usize;
        let width = match accessor["type"].as_str().unwrap() {
            "VEC3" => 3,
            "VEC4" => 4,
            _ => panic!("TRS required"),
        };
        let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize
            + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
        let stride = view["byteStride"]
            .as_u64()
            .map_or(width * 4, |v| v as usize);
        let values: Vec<_> = (0..count)
            .map(|key| binary[offset + key * stride..offset + key * stride + width * 4].to_vec())
            .collect();
        let begin = binary.len();
        for value in values {
            binary.extend(vec![0; width * 4]);
            binary.extend(value);
            binary.extend(vec![0; width * 4]);
        }
        accessor["count"] = (count * 3).into();
        accessor["byteOffset"] = 0.into();
        accessor["bufferView"] = document["bufferViews"].as_array().unwrap().len().into();
        document["bufferViews"].as_array_mut().unwrap().push(
            serde_json::json!({"buffer":0,"byteOffset":begin,"byteLength":binary.len()-begin}),
        );
        let output = document["accessors"].as_array().unwrap().len();
        document["accessors"].as_array_mut().unwrap().push(accessor);
        for animation in document["animations"].as_array_mut().unwrap() {
            for sampler in animation["samplers"].as_array_mut().unwrap() {
                if sampler["output"].as_u64().unwrap() as usize == *index {
                    sampler["output"] = output.into();
                    sampler["interpolation"] = "CUBICSPLINE".into();
                }
            }
        }
    }
    document["buffers"][0]["byteLength"] = binary.len().into();
    let mut json = serde_json::to_vec(&document).unwrap();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let size = 28 + json.len() + binary.len();
    let mut glb = Vec::new();
    glb.extend(b"glTF");
    glb.extend(2u32.to_le_bytes());
    glb.extend((size as u32).to_le_bytes());
    glb.extend((json.len() as u32).to_le_bytes());
    glb.extend(b"JSON");
    glb.extend(json);
    glb.extend((binary.len() as u32).to_le_bytes());
    glb.extend(b"BIN\0");
    glb.extend(binary);
    reference_rig_gpu_acceptance(&glb, "reference-rig-cubic", 370, 22, 1);
}

#[test]
#[ignore = "requires GPU; mirrored bind hierarchy and animated negative scale"]
fn gpu_mirrored_rig_matches_cpu_positions_normals_and_render() {
    let original = include_bytes!("../../../examples/assets/rigged-figure/RiggedFigure.glb");
    let json_size = u32::from_le_bytes(original[12..16].try_into().unwrap()) as usize;
    for animated in [false, true] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&original[20..20 + json_size]).unwrap();
        let parsed = gltf::Gltf::from_slice(original).unwrap();
        let mut binary = parsed.blob.as_ref().unwrap().clone();
        if animated {
            let channel = parsed
                .animations()
                .next()
                .unwrap()
                .channels()
                .find(|c| {
                    c.target().node().index() == 2
                        && c.target().property() == gltf::animation::Property::Scale
                })
                .unwrap();
            let accessor = channel.sampler().output();
            assert_eq!(accessor.data_type(), gltf::accessor::DataType::F32);
            let view = accessor.view().unwrap();
            let stride = view.stride().unwrap_or(12);
            for key in 0..accessor.count() {
                let offset = view.offset() + accessor.offset() + key * stride;
                let value = f32::from_le_bytes(binary[offset..offset + 4].try_into().unwrap());
                binary[offset..offset + 4].copy_from_slice(&(-value.abs()).to_le_bytes());
            }
        } else {
            for index in [0, 4, 8, 12] {
                document["nodes"][0]["matrix"][index] =
                    (-document["nodes"][0]["matrix"][index].as_f64().unwrap()).into();
            }
        }
        let mut json = serde_json::to_vec(&document).unwrap();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let size = 28 + json.len() + binary.len();
        let mut glb = Vec::new();
        glb.extend(b"glTF");
        glb.extend(2u32.to_le_bytes());
        glb.extend((size as u32).to_le_bytes());
        glb.extend((json.len() as u32).to_le_bytes());
        glb.extend(b"JSON");
        glb.extend(json);
        glb.extend((binary.len() as u32).to_le_bytes());
        glb.extend(b"BIN\0");
        glb.extend(binary);
        reference_rig_gpu_acceptance(
            &glb,
            if animated {
                "animated-mirror"
            } else {
                "bind-mirror"
            },
            370,
            22,
            1,
        );
    }
}

fn step_glb(original: &[u8]) -> Vec<u8> {
    let json_size = u32::from_le_bytes(original[12..16].try_into().unwrap()) as usize;
    let mut document: serde_json::Value =
        serde_json::from_slice(&original[20..20 + json_size]).unwrap();
    for animation in document["animations"].as_array_mut().unwrap() {
        for sampler in animation["samplers"].as_array_mut().unwrap() {
            sampler["interpolation"] = "STEP".into();
        }
    }
    let mut json = serde_json::to_vec(&document).unwrap();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let binary_chunk = &original[20 + json_size..];
    let size = 20 + json.len() + binary_chunk.len();
    let mut glb = Vec::new();
    glb.extend(b"glTF");
    glb.extend(2u32.to_le_bytes());
    glb.extend((size as u32).to_le_bytes());
    glb.extend((json.len() as u32).to_le_bytes());
    glb.extend(b"JSON");
    glb.extend(json);
    glb.extend(binary_chunk);
    glb
}

fn reference_rig_gpu_acceptance(
    bytes: &[u8],
    label: &str,
    vertices: usize,
    joints: usize,
    clips: usize,
) {
    let asset = crate::ModelAsset::parse(bytes, &[], crate::ModelLimits::default()).unwrap();
    assert_eq!(asset.animations.len(), clips);
    assert_eq!(asset.primitives.len(), 1);
    let crate::ModelGeometry::Skinned(mesh) = &asset.primitives[0].geometry else {
        panic!("skin required");
    };
    assert_eq!(mesh.vertices().len(), vertices);
    assert_eq!(usize::from(mesh.joint_count()), joints);
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let info = adapter.get_info();
    let timestamp_features = wgpu::Features::TIMESTAMP_QUERY;
    let timestamps_supported = adapter.features().contains(timestamp_features);
    let descriptor = wgpu::DeviceDescriptor {
        required_features: if timestamps_supported {
            timestamp_features
        } else {
            wgpu::Features::empty()
        },
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).unwrap();
    let queries = timestamps_supported.then(|| {
        device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("reference rig GPU interval"),
            ty: wgpu::QueryType::Timestamp,
            count: 4,
        })
    });
    let resolved = queries.as_ref().map(|_| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    });
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let color_format = if label == "fox" {
        wgpu::TextureFormat::Rgba8UnormSrgb
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    };
    let mut renderer = SceneRenderer::new(&device, color_format);
    pollster::block_on(renderer.reload_shader(
        &device,
        include_str!("../../../../voxy_editor/src/material.wgsl"),
    ))
    .unwrap();
    let transform = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    transform
        .update_scene_material(
            &queue,
            Mat4::IDENTITY,
            if label == "fox" {
                [1.; 4]
            } else {
                [0.35, 0.55, 0.75, 1.]
            },
            [-0.3, 0.7, 0.6, 0.9],
        )
        .unwrap();
    let texture = if let Some(material) = asset.primitives[0].base_color_texture {
        let gltf = gltf::Gltf::from_slice(bytes).unwrap();
        let image = gltf.images().nth(material.image).unwrap();
        let gltf::image::Source::View { view, .. } = image.source() else {
            panic!("embedded image required");
        };
        let blob = gltf.blob.as_ref().unwrap();
        let image = crate::ImageAsset::decode(
            &blob[view.offset()..view.offset() + view.length()],
            crate::ImageLimits::default(),
        )
        .unwrap();
        if material.use_mips {
            renderer
                .upload_image_mips(&device, &queue, &image.mip_chain(), material.sampling)
                .unwrap()
        } else {
            renderer
                .upload_image(&device, &queue, &image, material.sampling)
                .unwrap()
        }
    } else {
        renderer
            .upload_texture(&device, &queue, 1, 1, &[255; 4])
            .unwrap()
    };
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 128,
                height: 128,
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
        color_format,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let source = skinner
        .upload_source(Arc::new(mesh.clone()), 0, 1024 * 1024)
        .unwrap();
    let palette = asset.skin_matrices(&asset.skeleton.bind_pose()).unwrap();
    let instance = skinner
        .create_instance(
            &renderer,
            source.clone(),
            &palette,
            asset.primitives[0].color,
            source.allocation_bytes(),
            1024 * 1024,
        )
        .unwrap();
    let bytes = instance.geometry.vertices.size();
    let normal_bytes = instance.geometry.normals.size();
    let timestamp_bytes = if timestamps_supported { 32 } else { 0 };
    let image_offset = bytes + normal_bytes + timestamp_bytes;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: image_offset + 512 * 128,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut max_error = 0.0_f32;
    let mut max_normal_error = 0.0_f32;
    let mut motion = 0.0_f32;
    let mut first_positions: Option<Vec<[f32; 3]>> = None;
    let mut cpu_us = Vec::new();
    let mut preflight_us = Vec::new();
    let mut encode_us = Vec::new();
    let mut submit_read_us = Vec::new();
    let mut gpu_interval_us = Vec::new();
    let mut render_us = Vec::new();
    let mut gpu_frame_us = Vec::new();
    for (clip, sample) in (0..clips).flat_map(|clip| (0..65).map(move |sample| (clip, sample))) {
        let time = asset.animations[clip].duration() * sample as f32 / 64.;
        let started = std::time::Instant::now();
        let pose = asset.sample_pose(Some(clip), time).unwrap();
        let palette = asset.skin_matrices(&pose).unwrap();
        let expected = mesh.posed_positions(&palette, Mat4::IDENTITY).unwrap();
        cpu_us.push(started.elapsed().as_secs_f64() * 1e6);
        let started = std::time::Instant::now();
        let view = if label == "fox" {
            Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2)
        } else {
            Mat4::IDENTITY
        };
        let min = expected
            .iter()
            .map(|p| view.transform_point3(glam::Vec3::from_array(*p)))
            .fold(glam::Vec3::splat(f32::INFINITY), glam::Vec3::min);
        let max = expected
            .iter()
            .map(|p| view.transform_point3(glam::Vec3::from_array(*p)))
            .fold(glam::Vec3::splat(f32::NEG_INFINITY), glam::Vec3::max);
        let extent = (max - min).max_element();
        transform
            .update(
                &queue,
                Mat4::from_translation(glam::Vec3::new(0., 0., 0.5))
                    * Mat4::from_scale(glam::Vec3::new(1.5 / extent, 1.5 / extent, 0.2 / extent))
                    * Mat4::from_translation(-(min + max) * 0.5)
                    * view,
            )
            .unwrap();
        let prepared = skinner.prepare_pose(&instance, &palette).unwrap();
        preflight_us.push(started.elapsed().as_secs_f64() * 1e6);
        let started = std::time::Instant::now();
        let mut encoder = device.create_command_encoder(&Default::default());
        skinner
            .encode_prepared_pose_profiled(
                &queue,
                &mut encoder,
                &prepared,
                queries
                    .as_ref()
                    .map(|query_set| wgpu::ComputePassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }),
            )
            .unwrap();
        let draws = [SceneDraw {
            geometry: instance.geometry(),
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        if let Some(queries) = &queries {
            renderer
                .encode_profiled(
                    &mut encoder,
                    &color_view,
                    &depth_view,
                    wgpu::Color::BLACK,
                    &draws,
                    wgpu::RenderPassTimestampWrites {
                        query_set: queries,
                        beginning_of_pass_write_index: Some(2),
                        end_of_pass_write_index: Some(3),
                    },
                )
                .unwrap();
        } else {
            renderer.encode(
                &mut encoder,
                &color_view,
                &depth_view,
                wgpu::Color::BLACK,
                &draws,
            );
        }
        encoder.copy_texture_to_buffer(
            color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: image_offset,
                    bytes_per_row: Some(512),
                    rows_per_image: Some(128),
                },
            },
            wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_buffer_to_buffer(&instance.geometry.vertices, 0, &staging, 0, bytes);
        encoder.copy_buffer_to_buffer(&instance.geometry.normals, 0, &staging, bytes, normal_bytes);
        encode_us.push(started.elapsed().as_secs_f64() * 1e6);
        let started = std::time::Instant::now();
        let mut submission = queue.submit([encoder.finish()]);
        // Counter resolution follows completed rendering on a separate command
        // buffer; this also detects backends that expose unwritten stage samples.
        if let (Some(queries), Some(resolved)) = (&queries, &resolved) {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            let mut resolve = device.create_command_encoder(&Default::default());
            resolve.resolve_query_set(queries, 0..4, resolved, 0);
            resolve.copy_buffer_to_buffer(resolved, 0, &staging, bytes + normal_bytes, 32);
            submission = queue.submit([resolve.finish()]);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        submit_read_us.push(started.elapsed().as_secs_f64() * 1e6);
        let mapped = staging.slice(..).get_mapped_range().unwrap();
        let actual = bytemuck::cast_slice::<u8, SceneVertex>(&mapped[..bytes as usize]);
        let actual_normals = bytemuck::cast_slice::<u8, [f32; 3]>(
            &mapped[bytes as usize..(bytes + normal_bytes) as usize],
        );
        if timestamps_supported {
            let offset = (bytes + normal_bytes) as usize;
            let begin = u64::from_le_bytes(mapped[offset..offset + 8].try_into().unwrap());
            let end = u64::from_le_bytes(mapped[offset + 8..offset + 16].try_into().unwrap());
            assert!(end >= begin, "GPU timestamp interval went backwards");
            let interval = (end - begin) as f64 * f64::from(queue.get_timestamp_period()) / 1000.;
            assert!(interval.is_finite());
            gpu_interval_us.push(interval);
            let render_begin =
                u64::from_le_bytes(mapped[offset + 16..offset + 24].try_into().unwrap());
            let render_end =
                u64::from_le_bytes(mapped[offset + 24..offset + 32].try_into().unwrap());
            if sample == 0 {
                println!(
                    "REFERENCE_RIG_FRAME_PIXELS colored={}",
                    mapped[image_offset as usize..]
                        .chunks_exact(4)
                        .filter(|p| p[2] > 0)
                        .count()
                );
            }
            assert!(
                render_end >= render_begin && render_end >= begin,
                "sample={sample} compute_begin={begin} compute_end={end} render_begin={render_begin} render_end={render_end}"
            );
            let period = f64::from(queue.get_timestamp_period()) / 1000.;
            render_us.push((render_end - render_begin) as f64 * period);
            gpu_frame_us.push((render_end - begin) as f64 * period);
        }
        let expected_normals = mesh.posed_normals(&palette, Mat4::IDENTITY).unwrap();
        for (actual, expected) in actual_normals.iter().zip(&expected_normals) {
            let normal = glam::Vec3::from_array(*actual);
            assert!(normal.is_finite());
            max_normal_error =
                max_normal_error.max((normal - glam::Vec3::from_array(*expected)).length());
        }
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
            for component in 0..3 {
                let value = actual.position[component];
                assert!(value.is_finite());
                max_error = max_error.max((value - expected[component]).abs());
                if let Some(first) = &first_positions {
                    motion = motion.max((value - first[index][component]).abs());
                }
            }
        }
        if first_positions.is_none() {
            first_positions = Some(actual.iter().map(|v| v.position).collect());
        }
        let pixels = &mapped[image_offset as usize..];
        assert!(
            pixels.chunks_exact(4).filter(|p| p[2] > 0).count() > 100,
            "reference rig absent from render"
        );
        if label == "fox" {
            assert!(
                pixels
                    .chunks_exact(4)
                    .filter(|p| p[0] > p[2].saturating_add(10))
                    .count()
                    > 50,
                "authored Fox texture absent"
            );
        }
        if sample == 32
            && let Some(directory) = std::env::var_os("VOXY_RIG_FRAME_IMAGE_DIR")
        {
            let path = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join(format!("{label}-clip-{clip}-frame.rgba")), pixels).unwrap();
        }
        drop(mapped);
        staging.unmap();
    }
    let prepared = skinner.prepare_pose(&instance, &palette).unwrap();
    let foreign = SceneSkinner::new(&renderer).unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    assert!(matches!(
        foreign.encode_prepared_pose(&queue, &mut encoder, &prepared),
        Err(SceneSkinError::ForeignSkinner)
    ));
    assert!(max_error < 1e-4, "CPU/GPU position error {max_error}");
    assert!(
        max_normal_error < 2e-5,
        "CPU/GPU normal error {max_normal_error}"
    );
    if label == "reference-rig-step" {
        // This fixture has only keys at 0 and clip duration. Loop playback
        // wraps the final key, so STEP must hold the initial pose throughout.
        assert!(motion < 1e-6, "two-endpoint STEP loop unexpectedly moved");
    } else {
        assert!(motion > 0.01, "reference rig animation did not move");
    }
    assert!(pollster::block_on(scope.pop()).is_none());
    let stats = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        (values[values.len() / 2], values[values.len() * 95 / 100])
    };
    println!(
        "REFERENCE_RIG_GPU_TIMESTAMPS asset={label} supported={timestamps_supported} period_ns={} interval_us={:?} samples={}",
        queue.get_timestamp_period(),
        if gpu_interval_us.is_empty() {
            None
        } else {
            Some(stats(gpu_interval_us.clone()))
        },
        gpu_interval_us.len()
    );
    println!(
        "REFERENCE_RIG_RENDER_GPU asset={label} supported={timestamps_supported} viewport=128x128 render_us={:?} compute_to_render_end_us={:?}",
        if render_us.is_empty() {
            None
        } else {
            Some(stats(render_us))
        },
        if gpu_frame_us.is_empty() {
            None
        } else {
            Some(stats(gpu_frame_us))
        }
    );
    println!(
        "REFERENCE_RIG_PROFILE asset={label} adapter={:?} backend={:?} vertices={} joints={} clips={clips} samples={} max_position_error={} max_normal_error={} motion={} cpu_pose_skin_us={:?} preflight_us={:?} prepared_encode_us={:?} submit_readback_us={:?} logical_bytes={}",
        info.name,
        info.backend,
        mesh.vertices().len(),
        mesh.joint_count(),
        clips * 65,
        max_error,
        max_normal_error,
        motion,
        stats(cpu_us),
        stats(preflight_us),
        stats(encode_us),
        stats(submit_read_us),
        source.allocation_bytes() + instance.allocation_bytes()
    );
}

#[test]
#[ignore = "requires GPU; affine skin normal tangency and rejected collapsed pose"]
fn gpu_skin_normals_follow_inverse_transpose_and_keep_last_good() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    pollster::block_on(renderer.reload_shader(
        &device,
        include_str!("../../../../voxy_editor/src/material.wgsl"),
    ))
    .unwrap();
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let normal = glam::Vec3::ONE.normalize();
    let mesh = crate::SkinnedMesh::new(
        [[0., 0., 0.], [1., -1., 0.], [1., 1., -2.]]
            .into_iter()
            .map(|position| crate::SkinnedVertex {
                position,
                normal: normal.to_array(),
                uv: [0.; 2],
                joints: [0, 1, 0, 0],
                weights: [32768, 32767, 0, 0],
            })
            .collect(),
        vec![0, 1, 2],
        2,
    )
    .unwrap();
    let source = skinner.upload_source(Arc::new(mesh), 0, 8192).unwrap();
    let instance = skinner
        .create_instance(&renderer, source, &[Mat4::IDENTITY; 2], [1.; 4], 0, 8192)
        .unwrap();
    let read = |pose: Option<&[Mat4]>| {
        let size = instance.geometry.normals.size();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        if let Some(pose) = pose {
            skinner
                .encode_pose(&queue, &mut encoder, &instance, pose)
                .unwrap();
        }
        encoder.copy_buffer_to_buffer(&instance.geometry.normals, 0, &staging, 0, size);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = staging.slice(..).get_mapped_range().unwrap();
        let result = bytemuck::cast_slice::<u8, [f32; 3]>(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        result
    };
    for (case, palette) in [
        [Mat4::IDENTITY; 2],
        [
            Mat4::from_rotation_y(0.3) * Mat4::from_scale(glam::Vec3::new(2., 0.5, 1.)),
            Mat4::from_rotation_x(-0.7) * Mat4::from_scale(glam::Vec3::new(0.75, 1.5, 3.)),
        ],
        [Mat4::from_scale(glam::Vec3::new(-2., 0.5, 3.)); 2],
        [Mat4::from_scale(glam::Vec3::new(1., 1e-9, 1.)); 2],
    ]
    .into_iter()
    .enumerate()
    {
        let actual = read(Some(&palette));
        let linear =
            glam::Mat3::from_mat4(palette[0] * (32768. / 65535.) + palette[1] * (32767. / 65535.));
        let expected = (linear.inverse().transpose() * normal).normalize();
        let tangent_a = linear * glam::Vec3::new(1., -1., 0.);
        let tangent_b = linear * glam::Vec3::new(1., 1., -2.);
        let cpu_mesh = instance
            .source
            .mesh
            .posed_scene_mesh(&palette, Mat4::IDENTITY, [1.; 4])
            .unwrap();
        let cpu_cache = NormalCache::from_mesh(&cpu_mesh);
        if case == 1 {
            let coordinates: Vec<_> = instance
                .source
                .mesh
                .vertices()
                .iter()
                .map(|v| v.position)
                .collect();
            let cpu_mesh = cpu_mesh.with_material_coordinates(coordinates).unwrap();
            let correct = renderer.upload_mesh(&device, &cpu_mesh).unwrap();
            let wrong_normal = (linear * normal).normalize().to_array();
            let wrong_mesh = cpu_mesh
                .clone()
                .with_normals(vec![wrong_normal; 3])
                .unwrap();
            let wrong = renderer.upload_mesh(&device, &wrong_mesh).unwrap();
            lit_normal_comparison(
                &renderer,
                &device,
                &queue,
                [instance.geometry(), &correct, &wrong],
                &cpu_mesh,
            );
        }

        for (value, cpu) in actual.iter().zip(&cpu_cache.normals) {
            assert!(
                (glam::Vec3::from_array(*value) - glam::Vec3::from_array(*cpu)).length() < 1e-5
            );
            let n = glam::Vec3::from_array(*value);
            assert!(n.is_finite());
            assert!(
                (n - expected).length() < 1e-5,
                "normal {n:?} expected {expected:?}"
            );
            assert!(n.dot(tangent_a).abs() < 1e-5);
            assert!(n.dot(tangent_b).abs() < 1e-5);
        }
    }
    let accepted = read(None);
    let collapsed = [Mat4::from_scale(glam::Vec3::new(1., 0., 1.)); 2];
    assert!(matches!(
        skinner.prepare_pose(&instance, &collapsed),
        Err(SceneSkinError::Pose(
            crate::SkinnedUploadError::SingularNormalMatrix
        ))
    ));
    assert_eq!(read(None), accepted);
    let near_fold = Mat4::from_cols(
        glam::Vec4::new(1., 1., 0., 0.),
        glam::Vec4::new(1., 1. + f32::EPSILON, 0., 0.),
        glam::Vec4::Z,
        glam::Vec4::W,
    );
    assert!(skinner.prepare_pose(&instance, &[near_fold; 2]).is_err());
    assert_eq!(read(None), accepted);
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
fn authored_normal_changes_invalidate_upload_cache_without_geometry_changes() {
    let original = SceneMesh::quad([1.; 4]);
    let count = original.vertices().len();
    let authored = original
        .clone()
        .with_normals(vec![[1., 0., 0.]; count])
        .unwrap();
    let mut cache = NormalCache::from_mesh(&authored);
    assert_eq!(cache.normals, vec![[1., 0., 0.]; count]);
    let changed = original
        .clone()
        .with_normals(vec![[0., 1., 0.]; count])
        .unwrap();
    assert!(cache.refresh(&changed));
    assert_eq!(cache.normals, vec![[0., 1., 0.]; count]);
    assert!(!cache.refresh(&changed));
    assert!(cache.refresh(&original));
    assert_eq!(cache.normals, smooth_normals(&original));
    assert!(original.clone().with_normals(vec![]).is_err());
    assert!(
        original
            .with_normals(vec![[f32::NAN, 0., 0.]; count])
            .is_err()
    );
}

fn lit_normal_comparison(
    renderer: &SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    geometries: [&SceneGeometry; 3],
    mesh: &SceneMesh,
) {
    let positions: Vec<_> = mesh
        .vertices()
        .iter()
        .map(|v| glam::Vec3::from_array(v.position))
        .collect();
    let face = (positions[1] - positions[0])
        .cross(positions[2] - positions[0])
        .normalize();
    let view = Mat4::from_quat(glam::Quat::from_rotation_arc(face, glam::Vec3::Z));
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for vertex in mesh.vertices() {
        let p = view.transform_point3(glam::Vec3::from_array(vertex.position));
        min = min.min(p);
        max = max.max(p);
    }
    let matrix = Mat4::from_translation(glam::Vec3::new(0., 0., 0.5))
        * Mat4::from_scale(glam::Vec3::splat(1.5 / (max - min).max_element()))
        * Mat4::from_translation(-(min + max) * 0.5)
        * view;
    let transform = renderer.create_transform(device, matrix).unwrap();
    transform
        .update_scene_material(
            queue,
            Mat4::IDENTITY,
            [0.35, 0.55, 0.75, 1.],
            [-0.3, 0.7, 0.6, 0.9],
        )
        .unwrap();
    transform
        .update_view_position(
            queue,
            (positions[0] + positions[1] + positions[2]) / 3. + face * 10.,
        )
        .unwrap();
    let texture = renderer
        .upload_texture(device, queue, 1, 1, &[255; 4])
        .unwrap();
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 384,
                height: 128,
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
        wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let draws = geometries.map(|geometry| {
        [SceneDraw {
            geometry,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }]
    });
    let views: Vec<_> = draws
        .iter()
        .enumerate()
        .map(|(i, draws)| SceneView {
            viewport: [i as u32 * 128, 0, 128, 128],
            draws,
        })
        .collect();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 1536 * 128,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer
        .encode_views(
            &mut encoder,
            &color.create_view(&Default::default()),
            &depth.create_view(&Default::default()),
            wgpu::Color::BLACK,
            &views,
        )
        .unwrap();
    encoder.copy_texture_to_buffer(
        color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1536),
                rows_per_image: Some(128),
            },
        },
        wgpu::Extent3d {
            width: 384,
            height: 128,
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = staging.slice(..).get_mapped_range().unwrap();
    let mut colored = 0;
    let mut wrong_diff = 0;
    let mut max_delta = 0u8;
    for y in 0..128 {
        for x in 0..128 {
            let left = &mapped[y * 1536 + x * 4..y * 1536 + x * 4 + 4];
            let middle = &mapped[y * 1536 + (x + 128) * 4..y * 1536 + (x + 128) * 4 + 4];
            let wrong = &mapped[y * 1536 + (x + 256) * 4..y * 1536 + (x + 256) * 4 + 4];
            for channel in 0..3 {
                max_delta = max_delta.max(left[channel].abs_diff(middle[channel]));
            }
            colored += usize::from(left[2] > 0);
            wrong_diff += usize::from(left[..3] != wrong[..3]);
        }
    }
    assert!(colored > 100, "lit geometry absent");
    assert!(max_delta <= 1, "CPU/GPU lit mismatch {max_delta}");
    assert!(
        wrong_diff > 100,
        "old forward normal formula did not change lighting"
    );
    println!(
        "LIT_NORMAL_COMPARISON colored_pixels={colored} max_cpu_gpu_channel_delta={max_delta} old_formula_difference_pixels={wrong_diff}"
    );
    if let Some(directory) = std::env::var_os("VOXY_NORMAL_IMAGE_DIR") {
        let path = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("lit-normal-comparison.rgba"), &mapped).unwrap();
    }
    drop(mapped);
    staging.unmap();
}

#[test]
#[ignore = "requires physical GPU; atomic shared skeletal memory admission"]
fn gpu_skeletal_device_budget_admits_whole_instances_and_retains_source() {
    let gpu = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    eprintln!("skeletal budget adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let lod = crate::skinned_lod::tests::source();
    let mesh = Arc::new(lod.mesh().clone());
    let mut joints = vec![Mat4::IDENTITY; usize::from(mesh.joint_count())];
    let color = [0.25, 0.5, 0.75, 1.];
    let preview = mesh
        .posed_scene_mesh(&joints, Mat4::IDENTITY, color)
        .unwrap();
    let source_bytes = mesh.vertices().len() as u64 * 64;
    let instance_bytes =
        SceneRenderer::mesh_allocation_bytes(&preview) + joints.len() as u64 * 64 + 32;
    let budget =
        crate::ComputeMemoryBudget::configure(&device, source_bytes + 2 * instance_bytes + 4)
            .unwrap();
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let source = skinner.upload_source(mesh.clone(), 0, u64::MAX).unwrap();
    assert_eq!(budget.stats().allocated_bytes, source_bytes);
    let first = skinner
        .create_instance(&renderer, source.clone(), &joints, color, 0, u64::MAX)
        .unwrap();
    assert_eq!(first.allocation_bytes(), instance_bytes);
    assert_eq!(
        budget.stats().allocated_bytes,
        source_bytes + instance_bytes
    );
    assert_eq!(budget.stats().allocated_buffers, 8);
    // Geometry and palette individually fit; the complete instance is 32 bytes
    // too large. Reject before allocating/retiring any part of the candidate.
    let compute = budget
        .allocate_storage("competing compute", &[0; 36])
        .unwrap();
    let before = budget.stats();
    assert!(matches!(
        skinner.create_instance(&renderer, source.clone(), &joints, color, 0, u64::MAX),
        Err(SceneSkinError::Scene(SceneError::MemoryBudget))
    ));
    assert_eq!(budget.stats(), before);
    drop(compute);
    assert!(matches!(
        skinner.create_instance(&renderer, source.clone(), &joints, color, 0, u64::MAX),
        Err(SceneSkinError::Scene(SceneError::MemoryBudget))
    ));
    assert_eq!(budget.stats().retired_buffers, 1);
    budget.discard_retired().unwrap();
    let second = skinner
        .create_instance(&renderer, source.clone(), &joints, color, 0, u64::MAX)
        .unwrap();
    assert_eq!(
        budget.stats().allocated_bytes,
        source_bytes + 2 * instance_bytes
    );
    assert_eq!(budget.stats().allocated_buffers, 15);
    let before = budget.stats();
    assert!(matches!(
        skinner.upload_source(mesh.clone(), 0, u64::MAX),
        Err(SceneSkinError::Scene(SceneError::MemoryBudget))
    ));
    assert!(matches!(
        skinner.create_lod_level(&first, &lod, 1, 0, u64::MAX),
        Err(SceneSkinError::Scene(SceneError::MemoryBudget))
    ));
    assert_eq!(budget.stats(), before);

    // The original instance still computes valid output after failed admissions.
    joints[1] = Mat4::from_translation(glam::Vec3::new(0.25, 0.1, 0.));
    let size = first.geometry.vertices.size();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("skeletal budget output"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    skinner
        .encode_pose(&queue, &mut encoder, &first, &joints)
        .unwrap();
    encoder.copy_buffer_to_buffer(&first.geometry.vertices, 0, &readback, 0, size);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = readback.slice(..).get_mapped_range().unwrap();
    let actual: &[SceneVertex] = bytemuck::cast_slice(&mapped);
    let expected = mesh
        .posed_scene_mesh(&joints, Mat4::IDENTITY, color)
        .unwrap();
    assert_eq!(actual.len(), expected.vertices().len());
    for (a, b) in actual.iter().zip(expected.vertices()) {
        assert!(
            glam::Vec3::from_array(a.position)
                .abs_diff_eq(glam::Vec3::from_array(b.position), 1e-5)
        );
        assert_eq!(a.uv, b.uv);
        assert_eq!(a.color, b.color);
    }
    drop(mapped);
    readback.unmap();
    drop(source);
    drop(first);
    assert_eq!(budget.stats().retired_buffers, 7);
    budget.discard_retired().unwrap();
    assert_eq!(
        budget.stats().allocated_bytes,
        source_bytes + instance_bytes
    );
    assert_eq!(budget.stats().allocated_buffers, 8);
    let level = skinner
        .create_lod_level(&second, &lod, 1, 0, u64::MAX)
        .unwrap();
    let level_bytes = level.index_allocation_bytes();
    let shared_bytes = second.geometry.vertices.size()
        + second.geometry.normals.size()
        + second.geometry.material_coordinates.size()
        + second.geometry.material_parameters.size();
    drop(second);
    // The LOD still owns four streams; source, base indices, palette and params retire.
    assert_eq!(budget.stats().retired_buffers, 4);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, shared_bytes + level_bytes);
    assert_eq!(budget.stats().allocated_buffers, 5);
    drop(level);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
    assert_eq!(budget.stats().allocated_buffers, 0);
    assert!(pollster::block_on(scope.pop()).is_none());
    eprintln!(
        "SKELETAL_DEVICE_BUDGET_PASS source_bytes={source_bytes} instance_bytes={instance_bytes} partial_admission_rejected=true output_matches_cpu=true final_charged_bytes=0"
    );
}
