//! Optional post-frame readback. No renderer input, descriptor or arithmetic changes.
use crate::{ComputeMemoryBudget, ComputeStorage};
use std::path::Path;

const LOAD: &str = r"
@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<u32>>;
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let size=textureDimensions(source);
 if any(id.xy>=size) {return;}
 result[id.y*size.x+id.x]=bitcast<vec4<u32>>(textureLoad(source,vec2<i32>(id.xy),0));
}
";

fn buffers(
    device: &wgpu::Device,
    size: u64,
) -> Result<[ComputeStorage; 2], Box<dyn std::error::Error>> {
    Ok(ComputeMemoryBudget::for_device(device).allocate_buffers([
        voxy_gpu_core::ManagedBufferDescriptor {
            label: "HDR diagnostic words",
            size,
            contents: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        },
        voxy_gpu_core::ManagedBufferDescriptor {
            label: "HDR diagnostic readback",
            size,
            contents: None,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        },
    ])?)
}

fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |value| {
            let _ = tx.send(value);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    rx.recv()??;
    let bytes = buffer.slice(..).get_mapped_range()?.to_vec();
    buffer.unmap();
    Ok(bytes)
}

pub(crate) fn native_output(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if texture.depth_or_array_layers() != 1 {
        return Err("single-plane diagnostic requires one texture layer".into());
    }
    native_subresource(device, queue, texture, 0, 0)
}

fn native_subresource(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip: u32,
    layer: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if texture.dimension() != wgpu::TextureDimension::D2
        || mip >= texture.mip_level_count()
        || layer >= texture.depth_or_array_layers()
    {
        return Err("invalid original texture subresource".into());
    }
    let width = (texture.width() >> mip).max(1);
    let height = (texture.height() >> mip).max(1);
    let pixel_bytes = match texture.format() {
        wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rg32Float => 8,
        wgpu::TextureFormat::Rg16Float
        | wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::R32Float
        | wgpu::TextureFormat::Depth32Float => 4,
        wgpu::TextureFormat::Rgba32Float => 16,
        _ => return Err("unsupported native HDR diagnostic format".into()),
    };
    if !texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
        return Err("native HDR diagnostic copy requires COPY_SRC".into());
    }
    let row = width * pixel_bytes;
    let stride = row.div_ceil(256) * 256;
    let [staging] = ComputeMemoryBudget::for_device(device).allocate_buffers([
        voxy_gpu_core::ManagedBufferDescriptor {
            label: "native HDR diagnostic output",
            size: u64::from(stride) * u64::from(height),
            contents: None,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        },
    ])?;
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: mip,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: layer,
            },
            aspect: if texture.format() == wgpu::TextureFormat::Depth32Float {
                wgpu::TextureAspect::DepthOnly
            } else {
                wgpu::TextureAspect::All
            },
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let padded = read(device, queue, encoder, &staging)?;
    Ok(padded
        .chunks_exact(stride as usize)
        .flat_map(|r| r[..row as usize].iter().copied())
        .collect())
}

/// Inspect every original mip and layer; no source usage flags or pixels are changed.
pub(crate) fn capture_texture_chain(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    root: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if texture.dimension() != wgpu::TextureDimension::D2 || texture.sample_count() != 1 {
        return Err("texture chain diagnostic requires single-sample D2 source".into());
    }
    std::fs::create_dir(root)?;
    let mut records = Vec::new();
    let mut sampled_pipeline = None;
    for layer in 0..texture.depth_or_array_layers() {
        for mip in 0..texture.mip_level_count() {
            let width = (texture.width() >> mip).max(1);
            let height = (texture.height() >> mip).max(1);
            let native = texture.usage().contains(wgpu::TextureUsages::COPY_SRC);
            let bytes = if native {
                native_subresource(device, queue, texture, mip, layer)?
            } else {
                let pipeline = sampled_pipeline.get_or_insert_with(|| {
                    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("original texture subresource words"),
                        source: wgpu::ShaderSource::Wgsl(LOAD.into()),
                    });
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: None,
                        layout: None,
                        module: &module,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        cache: None,
                    })
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                });
                let size = u64::from(width) * u64::from(height) * 16;
                let [output, staging] = buffers(device, size)?;
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: output.as_entire_binding(),
                        },
                    ],
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_compute_pass(&Default::default());
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &bind, &[]);
                    pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
                }
                encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, size);
                read(device, queue, encoder, &staging)?
            };
            let file = format!("layer-{layer:02}-mip-{mip:02}.bin");
            std::fs::write(root.join(&file), &bytes)?;
            records.push(serde_json::json!({"mip":mip,"layer":layer,"size":[width,height],"bytes":bytes.len(),"blake3":blake3::hash(&bytes).to_hex().to_string(),"file":file,"capture_format":if native {"original native source words"}else{"original textureLoad RGBA32Float words"},"source_bits_preserved_exactly":native}));
        }
    }
    let proof = serde_json::json!({"source_format":format!("{:?}",texture.format()),"mips":texture.mip_level_count(),"layers":texture.depth_or_array_layers(),"subresources":records,"source_resource_modified":false});
    std::fs::write(root.join("chain.json"), serde_json::to_vec_pretty(&proof)?)?;
    Ok(proof)
}

pub(crate) fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    views: Vec<(&str, wgpu::TextureView)>,
    uniform: Option<&wgpu::Buffer>,
    root: &Path,
    raw: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    std::fs::create_dir(root)?;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("read only actual HDR stage texels"),
        source: wgpu::ShaderSource::Wgsl(LOAD.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("post frame HDR stage capture"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let depth_code = LOAD
        .replace("var source:texture_2d<f32>", "var source:texture_depth_2d")
        .replace("result:array<vec4<u32>>", "result:array<u32>")
        .replace(
            "bitcast<vec4<u32>>(textureLoad(source,vec2<i32>(id.xy),0))",
            "bitcast<u32>(textureLoad(source,vec2<i32>(id.xy),0))",
        );
    let mut depth_pipeline = None;
    let mut stages = Vec::new();
    for (name, view) in views {
        let texture = view.texture();
        let depth = texture.format() == wgpu::TextureFormat::Depth32Float;
        let size = u64::from(texture.width()) * u64::from(texture.height()) * if depth {4} else {16};
        if texture.format() != wgpu::TextureFormat::Depth32Float
            && texture.usage().contains(wgpu::TextureUsages::COPY_SRC)
        {
            let bytes = native_output(device, queue, texture)?;
            let half = matches!(
                texture.format(),
                wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rg16Float
            );
            let nonfinite = if half {
                bytes
                    .chunks_exact(2)
                    .filter(|b| u16::from_le_bytes((*b).try_into().unwrap()) & 0x7c00 == 0x7c00)
                    .count()
            } else {
                bytes
                    .chunks_exact(4)
                    .filter(|b| {
                        u32::from_le_bytes((*b).try_into().unwrap()) & 0x7f800000 == 0x7f800000
                    })
                    .count()
            };
            if raw {
                std::fs::write(root.join(format!("{name}.native.bin")), &bytes)?;
            }
            stages.push(serde_json::json!({"stage":name,"source_format":format!("{:?}",texture.format()),"size":[texture.width(),texture.height()],"capture_format":"native source bit words, row-major top-down, padding excluded","source_bits_preserved_exactly":true,"nonfinite_channels":nonfinite,"bytes":bytes.len(),"blake3":blake3::hash(&bytes).to_hex().to_string(),"raw_saved":raw}));
            continue;
        }
        let pipeline = if texture.format() == wgpu::TextureFormat::Depth32Float {
            depth_pipeline.get_or_insert_with(|| {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("read only original shadow depth"),
                    source: wgpu::ShaderSource::Wgsl(depth_code.clone().into()),
                });
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("original shadow depth capture"),
                    layout: None,
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                })
            })
        } else {
            &pipeline
        };
        let [output, staging] = buffers(device, size)?;
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(texture.width().div_ceil(8), texture.height().div_ceil(8), 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, size);
        let bytes = read(device, queue, encoder, &staging)?;
        let nonfinite = bytes
            .chunks_exact(4)
            .filter(|b| u32::from_le_bytes((*b).try_into().unwrap()) & 0x7f800000 == 0x7f800000)
            .count();
        if raw {
            let suffix = if depth {"r32.bin"} else {"rgba32.bin"};
            std::fs::write(root.join(format!("{name}.{suffix}")), &bytes)?;
        }
        stages.push(serde_json::json!({"stage":name,"source_format":format!("{:?}",texture.format()),"size":[texture.width(),texture.height()],"capture_format":if depth {"R32Float bit words, row-major top-down"} else {"RGBA32Float bit words, row-major top-down"},"captured_channels":if depth {1} else {4},"textureLoad_only":true,"finite_source_channels_preserved_exactly":true,"nonfinite_channels":nonfinite,"bytes":bytes.len(),"blake3":blake3::hash(&bytes).to_hex().to_string(),"raw_saved":raw}));
    }
    let mut uniform_hash = serde_json::Value::Null;
    if let Some(uniform) = uniform {
        uniform_hash = capture_uniform(device, queue, uniform, &root.join("temporal-uniform.bin"))?;
    }
    let proof = serde_json::json!({"scope":"post completed frame actual stage textureLoad and original GPU temporal uniform","stages":stages,"temporal_uniform":uniform_hash,"rendered_inputs_modified":false,"frame_performance_qualified":false});
    std::fs::write(root.join("stages.json"), serde_json::to_vec_pretty(&proof)?)?;
    Ok(proof)
}

/// Copy original storage words after the producer fence, without interpreting
/// floating-point values or changing any renderer source resource descriptor.
pub(crate) fn capture_storage(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &wgpu::Buffer,
    path: &Path,
    raw: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let size = source.size();
    if size == 0 || !size.is_multiple_of(4) || !source.usage().contains(wgpu::BufferUsages::COPY_SRC) {
        return Err("native storage diagnostic requires nonempty aligned COPY_SRC buffer".into());
    }
    let [staging] = ComputeMemoryBudget::for_device(device).allocate_buffers([
        voxy_gpu_core::ManagedBufferDescriptor {
            label: "original irradiance coefficient readback",
            size,
            contents: None,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        },
    ])?;
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, 0, &staging, 0, size);
    let bytes = read(device, queue, encoder, &staging)?;
    if raw {
        std::fs::write(path, &bytes)?;
    }
    Ok(
        serde_json::json!({"bytes":size,"blake3":blake3::hash(&bytes).to_hex().to_string(),"source_bits_preserved_exactly":true,"raw_saved":raw,"source_buffer_usage_modified":false}),
    )
}

/// Read the actual uniform as original integer words; no f32 interpretation.
pub(crate) fn capture_uniform(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    uniform: &wgpu::Buffer,
    path: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let size = uniform.size();
    assert!(size > 0 && size.is_multiple_of(16));
    let count = size / 16;
    let code = format!(
        "@group(0) @binding(0) var<uniform> source:array<vec4<u32>,{count}>; @group(0) @binding(1) var<storage,read_write> result:array<vec4<u32>>; @compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {{if id.x<{count}u {{result[id.x]=source[id.x];}}}}"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("actual temporal uniform words"),
        source: wgpu::ShaderSource::Wgsl(code.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let [output, staging] = buffers(device, size)?;
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((count as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, size);
    let bytes = read(device, queue, encoder, &staging)?;
    std::fs::write(path, &bytes)?;
    let proof = serde_json::json!({"bytes":size,"blake3":blake3::hash(&bytes).to_hex().to_string(),"GPU_original_uniform_words":true});
    Ok(proof)
}

pub(crate) struct ProbeFaceSources<'a> {
    pub face_view: &'a wgpu::TextureView,
    pub uniform_inputs: Vec<(String, &'a wgpu::Buffer)>,
    pub raster_inputs: Vec<(&'static str, wgpu::TextureView)>,
    pub projection: &'a wgpu::Buffer,
    pub camera: &'a wgpu::Buffer,
    pub coefficients: &'a wgpu::Buffer,
    pub point_bits: [u32; 3],
    pub probe_index: usize,
    pub face_index: usize,
}
/// Observe one fully completed original projection, without drawing a new face.
pub(crate) fn capture_probe_face(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sources: ProbeFaceSources<'_>,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let begin = sources
        .probe_index
        .checked_mul(64)
        .ok_or("probe record offset overflow")?;
    let end = begin
        .checked_add(64)
        .ok_or("probe record extent overflow")?;
    if !sources.coefficients.size().is_multiple_of(64) || end as u64 > sources.coefficients.size() {
        return Err("probe record outside coefficient storage".into());
    }
    let mut views = vec![("probe-face-radiance", sources.face_view.clone())];
    views.extend(sources.raster_inputs);
    let mut proof = capture(device, queue, views, None, root, true)?;
    proof["scope"] =
        "original completed probe face, original uniforms and post-projection coefficients".into();
    proof["projection_uniform"] = capture_uniform(
        device,
        queue,
        sources.projection,
        &root.join("projection-uniform.bin"),
    )?;
    proof["camera_uniform"] = capture_uniform(
        device,
        queue,
        sources.camera,
        &root.join("camera-uniform.bin"),
    )?;
    let mut uniforms = serde_json::Map::new();
    for (name, buffer) in sources.uniform_inputs {
        uniforms.insert(
            name.clone(),
            capture_uniform(device, queue, buffer, &root.join(format!("{name}.bin")))?,
        );
    }
    proof["lighting_material_uniforms"] = serde_json::Value::Object(uniforms);
    proof["probe_coefficients_after_face"] = capture_storage(
        device,
        queue,
        sources.coefficients,
        &root.join("probe-sh.native.bin"),
        true,
    )?;
    let all_coefficients = std::fs::read(root.join("probe-sh.native.bin"))?;
    let current = &all_coefficients[begin..end];
    std::fs::write(root.join("current-probe-sh.native.bin"), current)?;
    proof["current_probe_coefficients_after_face"] = serde_json::json!({"bytes":64,"source_offset_bytes":begin,"blake3":blake3::hash(current).to_hex().to_string(),"original_GPU_word_slice":true});
    proof["point_bits"] = serde_json::json!(sources.point_bits);
    proof["probe_index"] = serde_json::json!(sources.probe_index);
    proof["face_index"] = serde_json::json!(sources.face_index);
    proof["extra_rendered_faces"] = serde_json::json!(0);
    std::fs::write(root.join("stages.json"), serde_json::to_vec_pretty(&proof)?)?;
    Ok(())
}
