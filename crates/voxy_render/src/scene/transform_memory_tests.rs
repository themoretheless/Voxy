use super::*;
use crate::{ComputeError, ComputeMemoryBudget};

#[test]
fn transform_admission_rejects_before_charging_and_retains_retired_uniforms() {
    let (device, queue) = wgpu::Device::noop(&Default::default());
    let budget = ComputeMemoryBudget::configure(&device, 260).unwrap();
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let compute = budget
        .allocate_storage("competing compute", &[0; 4])
        .unwrap();
    let transform = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    assert_eq!(transform.allocation_bytes(), 256);
    let full = budget.stats();
    assert!(matches!(
        renderer.create_transform(&device, Mat4::IDENTITY),
        Err(SceneError::MemoryBudget)
    ));
    assert!(matches!(
        renderer.create_transform(&device, Mat4::from_cols_array(&[f32::NAN; 16])),
        Err(SceneError::InvalidTransform)
    ));
    assert_eq!(budget.stats(), full);
    transform
        .update(&queue, Mat4::from_scale(glam::Vec3::splat(2.)))
        .unwrap();
    drop(transform);
    assert_eq!(budget.stats().allocated_bytes, 260);
    assert_eq!(budget.stats().retired_buffers, 1);
    assert!(matches!(
        renderer.create_transform(&device, Mat4::IDENTITY),
        Err(SceneError::MemoryBudget)
    ));
    budget.discard_retired().unwrap();
    let retry = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    drop((retry, compute));
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
}

#[test]
#[ignore = "requires physical GPU; transform uniform ownership and shared admission"]
fn gpu_transform_budget_preserves_independent_uniforms_after_exhaustion() {
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("transform budget adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mesh = SceneMesh::quad([1.; 4]);
    let mesh_bytes = SceneRenderer::mesh_allocation_bytes(&mesh);
    let budget = ComputeMemoryBudget::configure(&device, mesh_bytes + 520).unwrap();
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = renderer.upload_mesh(&device, &mesh).unwrap();
    let image = renderer
        .upload_texture(&device, &queue, 1, 1, &[255; 4])
        .unwrap();
    let compute = budget
        .allocate_storage("competing compute", &[0; 4])
        .unwrap();
    let first = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    let second_matrix = Mat4::from_scale(glam::Vec3::splat(0.5));
    let second = renderer.create_transform(&device, second_matrix).unwrap();
    let full = budget.stats();
    assert_eq!(full.allocated_bytes, mesh_bytes + 520);
    assert!(matches!(
        renderer.create_transform(&device, Mat4::IDENTITY),
        Err(SceneError::MemoryBudget)
    ));
    assert!(matches!(
        budget.allocate_storage("over budget", &[0; 4]),
        Err(ComputeError::MemoryBudget)
    ));
    assert!(matches!(
        renderer.upload_texture(&device, &queue, 1, 1, &[255; 4]),
        Err(SceneError::MemoryBudget)
    ));
    assert_eq!(budget.stats(), full);
    let updated = Mat4::from_translation(glam::Vec3::new(0.25, -0.125, 0.));
    first.update(&queue, updated).unwrap();
    first
        .update_view_position(&queue, glam::Vec3::new(3., 4., 5.))
        .unwrap();
    first.update_pbr_material(&queue, 0.375, 0.625).unwrap();

    // A separate GPU program reads the actual uniform bindings, without a CPU mirror.
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("independent transform readback"),
        source: wgpu::ShaderSource::Wgsl(r#"
            struct Uniform { values: array<vec4<f32>,16> }
            @group(0) @binding(0) var<uniform> first: Uniform;
            @group(0) @binding(1) var<uniform> second: Uniform;
            @group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>,32>;
            @compute @workgroup_size(1) fn main() {
                for(var i=0u;i<16u;i++) { output[i]=first.values[i]; output[i+16u]=second.values[i]; }
            }
        "#.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 512,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 512,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: first.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: second.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, 512);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let data = staging.slice(..).get_mapped_range().unwrap();
    let actual: &[f32] = bytemuck::cast_slice(&data);
    let expected = |matrix: Mat4| {
        let mut v = Vec::new();
        v.extend(matrix.to_cols_array());
        v.extend(matrix.to_cols_array());
        v.extend([0., 0., 2., 1.]);
        v.extend(Mat4::IDENTITY.to_cols_array());
        v.extend([1.; 4]);
        v.extend([0., 0., 1., 0.]);
        v.extend([0.; 4]);
        v
    };
    let mut first_expected = expected(updated);
    first_expected[32..36].copy_from_slice(&[3., 4., 5., 1.]);
    first_expected[61..64].copy_from_slice(&[0.375, 0.625, 1.]);
    assert_eq!(&actual[..64], first_expected);
    assert_eq!(&actual[64..], expected(second_matrix));
    drop(data);
    staging.unmap();
    // Discard external bindings/encoders before explicit retirement confirmation.
    drop((binding, pipeline, first, second));
    assert_eq!(budget.stats().retired_buffers, 2);
    assert!(matches!(
        renderer.create_transform(&device, Mat4::IDENTITY),
        Err(SceneError::MemoryBudget)
    ));
    budget.discard_retired().unwrap();
    let retry = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    drop((retry, geometry, image, compute));
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
fn empty_scene_and_skinner_preserve_configured_device_owner() {
    let (device, _queue) = wgpu::Device::noop(&Default::default());
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let budget = ComputeMemoryBudget::configure(&device, 256).unwrap();
    let transform = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    drop(transform);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
    drop(budget);
    assert_eq!(renderer.compute_memory_budget().max_bytes(), 256);
    assert!(ComputeMemoryBudget::configure(&device, 512).is_err());
    let first = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    assert!(matches!(
        renderer.create_transform(&device, Mat4::IDENTITY),
        Err(SceneError::MemoryBudget)
    ));
    drop(first);
    renderer.compute_memory_budget().discard_retired().unwrap();
    let skinner = SceneSkinner::new(&renderer).unwrap();
    drop(renderer);
    // Only the empty skinning producer owns the ledger now; no live/retired
    // GPU allocations or external budget handle can accidentally mask expiry.
    assert!(ComputeMemoryBudget::configure(&device, 512).is_err());
    let shared = ComputeMemoryBudget::for_device(&device);
    assert_eq!(shared.max_bytes(), 256);
    let full = shared
        .allocate_storage("skin owner persistence", &[0; 256])
        .unwrap();
    assert!(matches!(
        shared.allocate_storage("overflow", &[0; 4]),
        Err(ComputeError::MemoryBudget)
    ));
    drop(full);
    shared.discard_retired().unwrap();
    drop(shared);
    assert_eq!(ComputeMemoryBudget::for_device(&device).max_bytes(), 256);
    drop(skinner);
}

#[test]
fn empty_used_scene_cannot_reconfigure_unbounded_history() {
    let (device, _queue) = wgpu::Device::noop(&Default::default());
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let transform = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    drop(transform);
    assert_eq!(renderer.compute_memory_budget().stats().allocated_bytes, 0);
    assert!(ComputeMemoryBudget::configure(&device, 256).is_err());
}
