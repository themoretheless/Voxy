//! Compare complete GPU projection records with and without far-plane admission.
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Case {
    matrix: [f32; 16],
    point: [f32; 4],
}
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&root).unwrap();
    let mut cases: Vec<Case> = Vec::new();
    if let Some(path) = std::env::args().nth(2).filter(|arg| !arg.starts_with("--")) {
        let inputs = std::fs::read(path).unwrap();
        assert_eq!(inputs.len() % 80, 0);
        cases.extend(
            inputs
                .chunks_exact(80)
                .map(bytemuck::pod_read_unaligned::<Case>),
        );
    }
    // Finite homogeneous rows whose separately expanded products overflow,
    // while their far-plane difference and final projected coordinates fit.
    for scale in [0.25f32, 0.5, 0.75, 1.] {
        let big = f32::MAX * scale;
        let mut matrix = glam::Mat4::IDENTITY.to_cols_array();
        // Unit lower triangular: determinant is exactly one, so the
        // directed numerical counterexample does not require a singular map.
        matrix[3] = big * 0.4;
        matrix[7] = big * 0.35;
        matrix[2] = -big * 0.4;
        matrix[6] = big * 0.5;
        cases.push(Case {
            matrix,
            point: [1., 1., 0., 1.],
        });
    }
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let source = concat!(
        include_str!("../../crates/voxy_render/src/archive_projection_pair.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_projection.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_interval.wgsl"),
        r#"
struct Case {matrix:mat4x4<f32>,point:vec4<f32>}
@group(0) @binding(0) var<storage,read> cases:array<Case>;
@group(0) @binding(1) var<storage,read_write> outputs:array<DdaProjectedSegment>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&cases) {return;}let c=cases[id.x];
 let segment=dda_project_segment(c.matrix,c.point.xyz,c.point.xyz,vec2<f32>(3840.,2160.));
 outputs[id.x]=segment;
}

"#
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("plane sign admission audit"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&cases),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = cases.len() as u64 * 176;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: size * 2,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    for far in [false, true] {
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[("ARCHIVE_DDA_FAR_BOUND", if far { 1. } else { 0. })],
                ..Default::default()
            },
            cache: None,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
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
                    resource: output.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups((cases.len() as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, u64::from(far) * size, size);
    }
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = readback.slice(..).get_mapped_range().unwrap();
    std::fs::write(root.join("inputs.bin"), bytemuck::cast_slice(&cases)).unwrap();
    std::fs::write(root.join("projection-pair.bin"), &bytes).unwrap();
    let split = size as usize;
    let mut changed = 0;
    let mut states_changed = 0;
    for (a, b) in bytes[..split]
        .chunks_exact(176)
        .zip(bytes[split..].chunks_exact(176))
    {
        changed += usize::from(a != b);
        states_changed += usize::from(a[160..164] != b[160..164]);
    }
    std::fs::write(root.join("summary.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases.len(),"full_segment_records_changed":changed,"projection_states_changed":states_changed,"full_street_qualified":false,"fps_qualified":false})) .unwrap()).unwrap();
    println!(
        "GPU_FAR_PLANE_FALLBACK cases={} records_changed={changed} states_changed={states_changed}",
        cases.len()
    );
    if std::env::args().any(|arg| arg == "--require-equivalent") {
        assert_eq!(
            changed, 0,
            "far-plane admission changed a complete projection record"
        );
    }
}
