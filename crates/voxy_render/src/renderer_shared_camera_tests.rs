use super::*;
use crate::frame_upload_test_support as support;
struct Probe {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    output: wgpu::Buffer,
    output_group: wgpu::BindGroup,
}
fn buffer(device: &wgpu::Device, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data,
        usage,
    })
}
fn probe(device: &wgpu::Device, size: usize) -> Probe {
    let entries: Vec<_> = (0..3)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: if binding == 0 {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage { read_only: true }
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        })
        .collect();
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    });
    let out_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout), Some(&out_layout)],
        immediate_size: 0,
    });
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:None,source:wgpu::ShaderSource::Wgsl(format!(r"
        struct Camera {{v:array<vec4u,{}>}}
        @group(0) @binding(0) var<uniform> camera:Camera;
        @group(0) @binding(1) var<storage,read> metadata:array<u32>;
        @group(0) @binding(2) var<storage,read> light:array<u32>;
        @group(1) @binding(0) var<storage,read_write> output:array<vec4u>;
        @compute @workgroup_size(1) fn main() {{for(var i=0u;i<{}u;i++){{output[i]=camera.v[i]^vec4u(metadata[0]+light[0]);}}}}
    ",size/16,size/16).into())});
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: size as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let output_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &out_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output.as_entire_binding(),
        }],
    });
    Probe {
        layout,
        pipeline,
        output,
        output_group,
    }
}
fn dispatch(probe: &Probe, group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
    let mut pass = encoder.begin_compute_pass(&Default::default());
    pass.set_pipeline(&probe.pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.set_bind_group(1, &probe.output_group, &[]);
    pass.dispatch_workgroups(1, 1, 1);
}
fn with_camera(kind: usize, frame: u32, callback: impl FnOnce(&[u8])) {
    let mut camera = CameraView::default();
    camera.eye.x += frame as f32 * 0.01;
    if kind == 0 {
        callback(bytemuck::bytes_of(&camera_uniform(640, 480, camera)));
    } else {
        callback(bytemuck::bytes_of(&photoreal_camera_uniform(
            640,
            480,
            camera,
            &BodycamRig::default(),
            Vec3::X,
            Vec3::Z,
            frame as f32 * 0.01,
        )));
    }
}
#[test]
#[ignore = "physical Apple Metal; all camera words, overflow and metadata refresh"]
fn gpu_frame_renderer_cameras_preserve_words_and_binding_refresh() {
    let (_, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    for (kind, size) in [
        (0, size_of::<CameraUniform>()),
        (1, size_of::<PhotorealCameraUniform>()),
    ] {
        let probe = probe(&device, size);
        let shared_legacy = buffer(
            &device,
            &vec![0; size],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let legacy = buffer(
            &device,
            &vec![0; size],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let mut metadata = buffer(
            &device,
            bytemuck::cast_slice(&[7_u32; 4]),
            wgpu::BufferUsages::STORAGE,
        );
        let light = buffer(
            &device,
            bytemuck::cast_slice(&[11_u32; 4]),
            wgpu::BufferUsages::STORAGE,
        );
        let mut shared =
            SharedCameraBindings::new(&device, &probe.layout, &shared_legacy, &metadata, &light)
                .unwrap();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (size * 12) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut expected = Vec::new();
        let mut pending = Vec::new();
        for frame in 0..6 {
            if frame == 3 {
                metadata = buffer(
                    &device,
                    bytemuck::cast_slice(&[13_u32; 4]),
                    wgpu::BufferUsages::STORAGE,
                );
                shared.refresh(&device, &probe.layout, &metadata, &light);
            }
            let legacy_group =
                create_camera_bind_group(&device, &probe.layout, &legacy, &metadata, &light);
            let shared_group =
                create_camera_bind_group(&device, &probe.layout, &shared_legacy, &metadata, &light);
            with_camera(kind, frame, |data| {
                expected.push(
                    bytemuck::cast_slice::<u8, u32>(data)
                        .iter()
                        .map(|v| v ^ (if frame < 3 { 18 } else { 24 }))
                        .collect::<Vec<_>>(),
                );
                for mode in 0..2 {
                    let mut encoder = device.create_command_encoder(&Default::default());
                    let group = if mode == 0 {
                        queue.write_buffer(&legacy, 0, data);
                        legacy_group.clone()
                    } else {
                        shared.select(&mut encoder, &shared_legacy, &shared_group, data)
                    };
                    dispatch(&probe, &group, &mut encoder);
                    encoder.copy_buffer_to_buffer(
                        &probe.output,
                        0,
                        &read,
                        (frame as u64 * 2 + mode) * size as u64,
                        size as u64,
                    );
                    if mode == 0 {
                        queue.submit([encoder.finish()]);
                    } else {
                        pending.push(encoder.finish());
                    }
                }
            });
        }
        queue.submit(pending);
        let actual = support::read(&device, &read);
        for (frame, pair) in actual.chunks_exact(size * 2).enumerate() {
            assert_eq!(&pair[..size], &pair[size..]);
            assert_eq!(
                bytemuck::cast_slice::<u8, u32>(&pair[..size]),
                expected[frame]
            );
        }
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
#[test]
#[ignore = "manual alternating production camera generation, binding and GPU-consumption benchmark"]
fn paired_frame_renderer_cameras_benchmark() {
    let (_, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    println!("path,count,pair,mode,cpu_us,drained_us");
    for (kind, size, path) in [
        (0, size_of::<CameraUniform>(), "voxel_camera"),
        (1, size_of::<PhotorealCameraUniform>(), "photoreal_camera"),
    ] {
        let probe = probe(&device, size);
        let metadata = buffer(
            &device,
            bytemuck::cast_slice(&[7_u32; 4]),
            wgpu::BufferUsages::STORAGE,
        );
        let light = buffer(
            &device,
            bytemuck::cast_slice(&[11_u32; 4]),
            wgpu::BufferUsages::STORAGE,
        );
        let legacy = buffer(
            &device,
            &vec![0; size],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let group = create_camera_bind_group(&device, &probe.layout, &legacy, &metadata, &light);
        let shared =
            SharedCameraBindings::new(&device, &probe.layout, &legacy, &metadata, &light).unwrap();
        support::pairs(&device, &queue, path, size, |use_shared, frame, encoder| {
            with_camera(kind, frame, |data| {
                let group = if use_shared {
                    shared.select(encoder, &legacy, &group, data)
                } else {
                    queue.write_buffer(&legacy, 0, data);
                    group.clone()
                };
                dispatch(&probe, &group, encoder);
            })
        });
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
