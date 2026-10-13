use super::*;
#[test]
#[ignore = "requires physical GPU; shared colour mip/HDR admission and lifetime"]
fn gpu_image_hdr_budget_shares_views_and_preserves_uploaded_mips() {
    let gpu = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    eprintln!("texture budget adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mesh = SceneMesh::quad([1.; 4]);
    let mesh_bytes = SceneRenderer::mesh_allocation_bytes(&mesh);
    let budget = crate::ComputeMemoryBudget::configure(&device, mesh_bytes + 44 + 56 + 4).unwrap();
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = renderer.upload_mesh(&device, &mesh).unwrap();
    let compute = budget.allocate_storage("shared compute", &[0; 4]).unwrap();
    let base = [255, 0, 0, 255].repeat(8);
    let middle = [0, 255, 0, 255].repeat(2);
    let tail = [0, 0, 255, 255];
    let image = renderer
        .upload_texture_levels(
            &device,
            &queue,
            &[(4, 2, &base), (2, 1, &middle), (1, 1, &tail)],
            TextureSampling::default(),
        )
        .unwrap();
    assert_eq!(image.allocation_bytes(), 44);
    let alias = renderer
        .texture_binding(&device, &image, TextureSampling::default(), 1)
        .unwrap();
    let before = budget.stats();
    assert_eq!(before.allocated_textures, 1);
    assert!(matches!(
        renderer.texture_binding(&device, &image, TextureSampling::default(), 4),
        Err(SceneError::InvalidTexture)
    ));
    assert_eq!(budget.stats(), before);
    let hdr = crate::HdrMipPyramid::new(&device, 3, 2).unwrap();
    assert_eq!(hdr.allocation_bytes(), 56);
    let hdr_alias = renderer
        .bind_image(
            &device,
            hdr.managed_texture(),
            TextureSampling::default(),
            2,
        )
        .unwrap();
    assert_eq!(hdr_alias.allocation_bytes(), 56);
    assert_eq!(budget.stats().allocated_bytes, mesh_bytes + 104);
    assert_eq!(budget.stats().allocated_textures, 2);
    assert_eq!(budget.stats().allocated_buffers, 6);
    let full = budget.stats();
    assert!(matches!(
        renderer.upload_texture(&device, &queue, 1, 1, &[255; 4]),
        Err(SceneError::MemoryBudget)
    ));
    assert!(matches!(
        renderer.create_sampled_color(&device, 1, 1),
        Err(SceneError::MemoryBudget)
    ));
    assert!(matches!(
        crate::HdrMipPyramid::new(&device, 1, 1),
        Err(crate::RendererError::Scene(SceneError::MemoryBudget))
    ));
    assert_eq!(budget.stats(), full);

    // Independently read the GPU's three uploaded sRGB mip texels after
    // failed admission. End-point primary colours decode exactly to 0/1.
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mip budget readback"), source: wgpu::ShaderSource::Wgsl(r"
                @group(0) @binding(0) var image: texture_2d<f32>;
                @group(0) @binding(1) var<storage, read_write> colors: array<vec4<f32>, 3>;
                @compute @workgroup_size(1) fn main() {
                    for (var i = 0u; i < 3u; i++) { colors[i] = textureLoad(image, vec2<i32>(0), i32(i)); }
                }
            ".into()),
        });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 48,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 48,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let view = image.texture().create_view(&Default::default());
    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: result.as_entire_binding(),
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
    encoder.copy_buffer_to_buffer(&result, 0, &staging, 0, 48);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = staging.slice(..).get_mapped_range().unwrap();
    assert_eq!(
        bytemuck::cast_slice::<u8, [f32; 4]>(&mapped),
        &[[1., 0., 0., 1.], [0., 1., 0., 1.], [0., 0., 1., 1.]]
    );
    drop(mapped);
    staging.unmap();
    drop((view, binding, pipeline));
    drop(image);
    drop(hdr);
    assert_eq!(budget.stats().retired_textures, 0);
    drop(alias);
    drop(hdr_alias);
    drop(geometry);
    drop(compute);
    assert_eq!(budget.stats().retired_textures, 2);
    assert_eq!(budget.stats().retired_buffers, 6);
    let mut ticket = budget.begin_retirement(&queue);
    assert_eq!(budget.stats().allocated_bytes, mesh_bytes + 104);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert!(ticket.try_finish().unwrap());
    assert!(ticket.try_finish().unwrap());
    assert_eq!(budget.stats(), crate::ComputeMemoryStats::default());
    let replacement = renderer.create_sampled_color(&device, 1, 1).unwrap();
    assert_eq!(replacement.allocation_bytes(), 4);
    drop(replacement);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats(), crate::ComputeMemoryStats::default());
    assert!(pollster::block_on(scope.pop()).is_none());
    eprintln!(
        "TEXTURE_DEVICE_BUDGET_PASS image_mips_bytes=44 hdr_npot_mips_bytes=56 views_share_owner=true uploaded_mips_survive_rejection=true final_charged_bytes=0"
    );
}
