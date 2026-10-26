//! Check the actual production rejection envelope against all original samples.
fn function(source: &str, name: &str) -> String {
    let start = source.find(&format!("fn {name}(")).unwrap();
    let mut depth = 0;
    let mut opened = false;
    for (i, c) in source[start..].char_indices() {
        if c == '{' {
            depth += 1;
            opened = true;
        }
        if c == '}' {
            depth -= 1;
        }
        if opened && depth == 0 {
            return source[start..=start + i].to_owned();
        }
    }
    panic!("unterminated production function")
}

#[test]
fn shader_with_batch_rejection_validates() {
    let module = naga::front::wgsl::parse_str(&super::archive_shader_source()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
#[ignore = "physical Metal rejection oracle; VOXY_GI_BATCH_PROOF must name a fresh directory"]
#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn accepted_envelopes_never_skip_an_original_hit() -> Result<(), Box<dyn std::error::Error>> {
    use wgpu::util::DeviceExt;
    let root = std::path::PathBuf::from(
        std::env::var_os("VOXY_GI_BATCH_PROOF").ok_or("proof directory required")?,
    );
    std::fs::create_dir(&root)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let (width, height) = (259u32, 131u32);
    let mut state = 0x7167_924bu32;
    let mut next = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        (state >> 8) as f32 / 16777216.
    };
    let depths: Vec<f32> = (0..width * height)
        .map(|i| {
            let x = i % width;
            let y = i / width;
            if y < height / 2 {
                1.
            } else {
                0.2 + ((x / 16 + y / 8) % 17) as f32 / 22.
            }
        })
        .collect();
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("original level-zero depths"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        depth.as_image_copy(),
        bytemuck::cast_slice(&depths),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        depth.size(),
    );
    let (mut w, mut h) = (
        width.div_ceil(8).next_power_of_two(),
        height.div_ceil(8).next_power_of_two(),
    );
    let bounds = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("independent minimum-depth hierarchy"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: w.max(h).ilog2() + 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut values = vec![[1f32, 0.]; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            for dy in 0..8 {
                for dx in 0..8 {
                    let z = depths[((y * 8 + dy).min(height - 1) * width
                        + (x * 8 + dx).min(width - 1)) as usize];
                    values[(y * w + x) as usize][0] = values[(y * w + x) as usize][0].min(z);
                }
            }
        }
    }
    let mut level = 0;
    loop {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &bounds,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&values),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 8),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        if w == 1 && h == 1 {
            break;
        }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut reduced = vec![[1., 0.]; (nw * nh) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for dy in 0..2 {
                    for dx in 0..2 {
                        reduced[(y * nw + x) as usize][0] = f32::min(
                            reduced[(y * nw + x) as usize][0],
                            values
                                [((y * 2 + dy).min(h - 1) * w + (x * 2 + dx).min(w - 1)) as usize]
                                [0],
                        );
                    }
                }
            }
        }
        values = reduced;
        w = nw;
        h = nh;
        level += 1;
    }
    const COUNT: u32 = 262144;
    let mut inputs = Vec::<[f32; 28]>::new();
    for i in 0..COUNT {
        let translation = [0., 8192., 262144., 100_000_000.][(i % 4) as usize];
        let offset = glam::Vec3::new(translation, translation * 0.5, -translation);
        let rotation = glam::Mat4::from_rotation_y((next() - 0.5) * 1.5)
            * glam::Mat4::from_rotation_x((next() - 0.5) * 0.5);
        let projection = if i % 8 == 0 {
            glam::Mat4::orthographic_rh(-32., 32., -16., 16., 0.01, 200.)
        } else {
            glam::Mat4::perspective_rh(1.5, width as f32 / height as f32, 0.01, 200.)
        };
        let vp = projection * rotation * glam::Mat4::from_translation(-offset);
        let origin = offset
            + glam::Vec3::new(
                (next() - 0.5) * 60.,
                (next() - 0.5) * 30.,
                -0.01 - next() * 80.,
            );
        let direction = glam::Vec3::new(next() - 0.5, next() - 0.5, next() - 0.5).normalize();
        let mut input = [0f32; 28];
        input[..16].copy_from_slice(&vp.to_cols_array());
        input[16..19].copy_from_slice(&origin.to_array());
        input[20..23].copy_from_slice(&direction.to_array());
        input[24] = 0.08;
        for _ in 0..(i % 73) {
            input[24] += 0.04 + 0.06 * input[24];
        }
        if i % 128 == 0 {
            input[16] = f32::NAN;
        } else if i % 128 == 1 {
            input[21] = f32::INFINITY;
        } else if i % 128 == 2 {
            input[0] = f32::NAN;
        }
        inputs.push(input);
    }
    let production = include_str!("archive_scene.wgsl");
    let source = format!(
        "{}\n{}\n{}",
        function(include_str!("archive_ray_projection.wgsl"), "dda_finite4"),
        format!(
            "struct GiBatchLine {{origin:vec4<f32>,direction:vec4<f32>,error:vec4<f32>,valid:bool}}\n{}\n{}",
            function(production, "gi_batch_line"),
            function(production, "gi_batch_in_front")
        ),
        r"
struct Camera {vp:mat4x4<f32>,viewport:vec4<f32>}
var<private> camera:Camera;
struct Input {vp:mat4x4<f32>,origin:vec4<f32>,direction:vec4<f32>,times:vec4<f32>}
@group(0) @binding(0) var<storage,read> samples:array<Input>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<u32>>;
@group(0) @binding(2) var ray_depth_bounds:texture_2d<f32>;
@group(0) @binding(3) var original_depth:texture_2d<f32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=arrayLength(&samples){return;}let s=samples[id.x];
 camera=Camera(s.vp,vec4<f32>(259.,131.,0.,0.));
 var end=s.times.x;for(var i=1u;i<8u;i++){end+=0.04+0.06*end;}
 let accepted=gi_batch_in_front(gi_batch_line(s.origin.xyz,s.direction.xyz),s.times.x,end);
 var bad=0u;var t=s.times.x;
 if accepted {for(var i=0u;i<8u;i++){
  let q=camera.vp*vec4<f32>(s.origin.xyz+s.direction.xyz*t,1.);
  let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;
  if !dda_finite4(q)||q.w<=0.||any(uv<=vec2<f32>(0.))||any(uv>=vec2<f32>(1.)){bad+=1u;}
  else{let z=textureLoad(original_depth,vec2<i32>(uv*camera.viewport.xy),0).x;if z<1.&&q.z/q.w>=z{bad+=1u;}}
  t+=0.04+0.06*t;
 }}
 result[id.x]=vec4<u32>(u32(accepted),bad,0u,0u);
}"
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("actual batch rejection vs original samples"),
        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&inputs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = u64::from(COUNT) * 16;
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let views = [
        bounds.create_view(&Default::default()),
        depth.create_view(&Default::default()),
    ];
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: result.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&views[1]),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(COUNT.div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&result, 0, &read, 0, size);
    let submission = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    let (send, receive) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        send.send(r).unwrap();
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receive.recv()??;
    let bytes = read.slice(..).get_mapped_range()?;
    let words: &[[u32; 4]] = bytemuck::cast_slice(&bytes);
    let accepted: u64 = words.iter().map(|r| u64::from(r[0])).sum();
    let mismatches: u64 = words.iter().map(|r| u64::from(r[1])).sum();
    std::fs::write(root.join("oracle.wgsl"), source)?;
    std::fs::write(
        root.join("proof.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"samples":COUNT,"accepted_groups":accepted,"checked_original_samples":accepted*8,"false_rejections":mismatches,"input_blake3":blake3::hash(bytemuck::cast_slice(&inputs)).to_hex().to_string(),"native_output_blake3":blake3::hash(&bytes).to_hex().to_string(),"adapter":format!("{:?}",adapter.get_info())}),
        )?,
    )?;
    assert!(
        accepted > 1000,
        "a vacuous rejection oracle is not evidence"
    );
    assert_eq!(
        mismatches, 0,
        "an accepted group contains an original hit or exit"
    );
    Ok(())
}
