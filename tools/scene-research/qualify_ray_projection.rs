//! Independent f64 camera-plane oracle for the candidate DDA bridge.
use wgpu::util::DeviceExt;
mod ray_projection_reference;
use ray_projection_reference::{Case, oracle};
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Projected {
    begin: [f32; 4],
    end: [f32; 4],
    world_begin: [f32; 4],
    world_end: [f32; 4],
    world_begin_low: [f32; 4],
    world_end_low: [f32; 4],
    begin_low: [f32; 4],
    end_low: [f32; 4],
    range: [f32; 4],
    clip_w: [f32; 4],
    state: u32,
    padding: [u32; 3],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ResultRow {
    segment: Projected,
    point: [f32; 4],
    point_low: [f32; 4],
    parameter: [f32; 4],
    reprojected: [f32; 4],
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let far = args.iter().any(|a| a == "--far-bound");
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    #[allow(deprecated)]
    let matrices = [
        glam::Mat4::perspective_rh(65f32.to_radians(), 16. / 9., 0.05, 1000.),
        glam::Mat4::perspective_rh(65f32.to_radians(), 16. / 9., 0.05, 1000.)
            * glam::Mat4::from_translation(glam::Vec3::new(0., -1.8, 0.)),
        glam::Mat4::orthographic_rh(-8., 8., -6., 6., 0.05, 200.),
        glam::Mat4::perspective_rh(100f32.to_radians(), 1., 0.001, 300.)
            * glam::Mat4::look_at_rh(
                glam::Vec3::new(12., 5., 16.),
                glam::Vec3::new(0., 1., 0.),
                glam::Vec3::Y,
            ),
    ];
    let mut cases = Vec::new();
    let mut state = 0x50524f4a454354u64;
    let mut random = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };
    for matrix in matrices {
        for (a, b) in [
            ([0., 0., -1.], [2., 0.5, -20.]),
            ([-1., 1., -3.], [0., 0., 3.]),
            ([0., 0., -2.], [0., 0., -100.]),
            ([0., 0., -100.], [0., 0., -0.01]),
            ([0., 0., 1.], [0., 0., 3.]),
            ([0., 0., -5.], [0., 0., -5.]),
            ([0., 0., -10000.], [0., 0., -5.]),
        ] {
            for s in [0., 0.001, 0.25, 0.5, 0.75, 0.999, 1.] {
                cases.push(Case {
                    matrix: matrix.to_cols_array(),
                    begin: [a[0], a[1], a[2], 0.],
                    end: [b[0], b[1], b[2], 0.],
                    sample: [s, 3840., 2160., 0.],
                });
            }
        }
        let inverse =
            glam::DMat4::from_cols_array(&matrix.to_cols_array().map(f64::from)).inverse();
        let eye_h = inverse * glam::DVec4::new(0., 0., 1., 0.);
        if eye_h.w.abs() > 1e-12 {
            let eye = eye_h.truncate() / eye_h.w;
            let near_h = inverse * glam::DVec4::new(0., 0., 0., 1.);
            let near = near_h.truncate() / near_h.w;
            let forward = (near - eye).normalize();
            let near_distance = (near - eye).length();
            for far in [1., 100., 10000.] {
                for close in [0., -1., near_distance * 0.1] {
                    for reverse in [false, true] {
                        for s in [0., 0.001, 0.25, 0.5, 0.75, 0.999, 1.] {
                            let a = eye + forward * far;
                            let b = eye + forward * close;
                            let (begin, end) = if reverse { (b, a) } else { (a, b) };
                            cases.push(Case {
                                matrix: matrix.to_cols_array(),
                                begin: [begin.x as f32, begin.y as f32, begin.z as f32, 0.],
                                end: [end.x as f32, end.y as f32, end.z as f32, 0.],
                                sample: [s, 3840., 2160., 0.],
                            });
                        }
                    }
                }
            }
        }
        for _ in 0..4096 {
            let begin = [
                (random() % 1921) as f32 / 16. - 60.,
                (random() % 961) as f32 / 16. - 30.,
                (random() % 2561) as f32 / 16. - 100.,
                0.,
            ];
            let end = [
                (random() % 1921) as f32 / 16. - 60.,
                (random() % 961) as f32 / 16. - 30.,
                (random() % 2561) as f32 / 16. - 100.,
                0.,
            ];
            cases.push(Case {
                matrix: matrix.to_cols_array(),
                begin,
                end,
                sample: [(random() % 1025) as f32 / 1024., 3840., 2160., 0.],
            });
        }
    }
    let valid_base = cases[0];
    for kind in 0..8 {
        let mut c = valid_base;
        match kind {
            0 => c.begin[0] = f32::NAN,
            1 => c.end[2] = f32::INFINITY,
            2 => c.matrix[0] = f32::NAN,
            3 => c.matrix[15] = f32::INFINITY,
            4 => c.sample[1] = 0.,
            5 => c.sample[2] = -1.,
            6 => c.sample[1] = f32::NAN,
            _ => c.sample[2] = f32::INFINITY,
        }
        cases.push(c);
    }
    let source = concat!(
        include_str!("../../crates/voxy_render/src/archive_projection_pair.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_projection.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_interval.wgsl"),
        r"
struct Case {matrix:mat4x4<f32>,begin:vec4<f32>,end:vec4<f32>,sample:vec4<f32>}
struct ResultRow {segment:DdaProjectedSegment,point:vec4<f32>,point_low:vec4<f32>,parameter:vec4<f32>,reprojected:vec4<f32>}
@group(0) @binding(0) var<storage,read> inputs:array<Case>;
@group(0) @binding(1) var<storage,read_write> outputs:array<ResultRow>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=arrayLength(&inputs){return;}let c=inputs[id.x];let segment=dda_project_segment(c.matrix,c.begin.xyz,c.end.xyz,c.sample.yz);
 var point=vec4<f32>(0.);var point_low=vec4<f32>(0.);var parameter=vec4<f32>(0.);var reprojected=vec4<f32>(0.);
 if segment.state==1u {let world=dda_reconstruct_world(segment,c.sample.x);point=world.high;point_low=world.low;parameter=vec4<f32>(dda_reconstruct_parameter(segment,c.sample.x),0.,0.);reprojected=vec4<f32>(dda_clip_ndc(dda_project_world_pair(c.matrix,world)),1.);}
 outputs[id.x]=ResultRow(segment,point,point_low,parameter,reprojected);
}
"
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("perspective world reconstruction candidate"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants: &[("ARCHIVE_DDA_FAR_BOUND", if far { 1. } else { 0. })],
            ..Default::default()
        },
        cache: None,
    });
    assert_eq!(std::mem::size_of::<Case>(), 112);
    assert_eq!(std::mem::size_of::<ResultRow>(), 240);
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&cases),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (cases.len() * 240) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
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
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
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
        pass.dispatch_workgroups((cases.len() as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
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
    let bytes = read.slice(..).get_mapped_range().unwrap();
    let (mut failures, mut valid, mut rejected, mut unsupported) = (0, 0, 0, 0);
    let (mut max_world, mut max_ndc, mut max_parameter) = (0f64, 0f64, 0f64);
    for (i, row) in bytes.chunks_exact(240).enumerate() {
        let actual: ResultRow = bytemuck::pod_read_unaligned(row);
        let c = cases[i];
        if !c
            .matrix
            .iter()
            .chain(c.begin.iter())
            .chain(c.end.iter())
            .all(|v| v.is_finite())
            || !c.sample[1].is_finite()
            || !c.sample[2].is_finite()
            || c.sample[1] <= 0.
            || c.sample[2] <= 0.
        {
            assert_eq!(
                actual.segment.state, 2,
                "invalid projection inputs must request fallback"
            );
            unsupported += 1;
            continue;
        }
        let expected = oracle(c);
        let okay = if let Some((range, _pa, _pb, u, p, ndc)) = expected {
            valid += 1;
            let actual_point = glam::DVec3::new(
                actual.point[0] as f64 + actual.point_low[0] as f64,
                actual.point[1] as f64 + actual.point_low[1] as f64,
                actual.point[2] as f64 + actual.point_low[2] as f64,
            );
            let world_error = (actual_point - p).abs().max_element();
            let ndc_error = (glam::DVec3::new(
                actual.reprojected[0] as f64,
                actual.reprojected[1] as f64,
                actual.reprojected[2] as f64,
            ) - ndc)
                .abs()
                .max_element();
            let parameter_error =
                (actual.parameter[0] as f64 + actual.parameter[1] as f64 - u).abs();
            max_world = max_world.max(world_error);
            max_ndc = max_ndc.max(ndc_error);
            max_parameter = max_parameter.max(parameter_error);
            let scale = c.begin[..3]
                .iter()
                .chain(c.end[..3].iter())
                .map(|v| v.abs() as f64)
                .fold(1f64, f64::max);
            actual.segment.state == 1
                && world_error <= 256. * (f32::EPSILON as f64).powi(2) * scale
                && ndc_error <= 32. * f32::EPSILON as f64
                && parameter_error <= 256. * (f32::EPSILON as f64).powi(2)
                && (actual.segment.range[0] as f64 + actual.segment.range[1] as f64 - range[0])
                    .abs()
                    <= 256. * (f32::EPSILON as f64).powi(2)
                && (actual.segment.range[2] as f64 + actual.segment.range[3] as f64 - range[1])
                    .abs()
                    <= 256. * (f32::EPSILON as f64).powi(2)
        } else {
            rejected += 1;
            actual.segment.state == 0
        };
        if !okay {
            if failures < 12 {
                eprintln!(
                    "PROJECTION_MISMATCH index={i} case={c:?} expected={expected:?} actual={actual:?}"
                );
            }
            failures += 1;
        }
    }
    if let Some([_, output_path]) = args.windows(2).find(|a| a[0] == "--output") {
        std::fs::write(output_path, &bytes).unwrap();
    }
    println!(
        "GPU_RAY_PROJECTION far_bound={far} cases={} valid={valid} rejected={rejected} unsupported={unsupported} failures={failures} max_world_error={max_world} max_ndc_error={max_ndc} max_parameter_error={max_parameter} production_enabled=false",
        cases.len()
    );
    assert_eq!(failures, 0, "projection bridge is not admitted");
}
