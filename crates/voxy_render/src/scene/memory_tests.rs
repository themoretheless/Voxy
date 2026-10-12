use super::*;
use crate::{ComputeError, ComputeMemoryBudget, LodIndexSet};

fn read_vertices(device: &wgpu::Device, queue: &wgpu::Queue, geometry: &SceneGeometry) -> Vec<u8> {
    let size = geometry.vertices.size();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("budget regression readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(&geometry.vertices, 0, &readback, 0, size);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let data = readback.slice(..).get_mapped_range().unwrap().to_vec();
    readback.unmap();
    data
}

#[test]
#[ignore = "requires physical GPU; shared mesh budget and retirement qualification"]
fn gpu_shared_mesh_budget_preserves_replacement_and_lod_owners() {
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("physical GPU required for mesh budget qualification");
    eprintln!("mesh budget adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mesh = SceneMesh::quad([1.; 4]);
    let bytes = SceneRenderer::mesh_allocation_bytes(&mesh);
    let variants = LodIndexSet::new(
        mesh.vertices.len(),
        vec![(0., mesh.indices.clone()), (0.01, mesh.indices.clone())],
    )
    .unwrap();
    let bundle_bytes = SceneRenderer::lod_mesh_allocation_bytes(&mesh, &variants).unwrap();
    let budget = ComputeMemoryBudget::configure(&device, bundle_bytes + 20).unwrap();
    let compute = budget
        .allocate_storage("shared compute owner", &[0; 4])
        .unwrap();
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    // Extra copy usage only for independent GPU-byte verification.
    let mut geometry = renderer
        .upload_mesh_with_usage(&device, &mesh, wgpu::BufferUsages::COPY_SRC)
        .unwrap();
    geometry.set_depth_mode(SceneDepthMode::Xray);
    assert_eq!(budget.stats().allocated_bytes, bytes + 4);
    let expected = bytemuck::cast_slice::<SceneVertex, u8>(&mesh.vertices).to_vec();
    assert_eq!(read_vertices(&device, &queue, &geometry), expected);
    let before = budget.stats();
    assert_eq!(
        renderer.replace_mesh(&device, &mut geometry, &mesh),
        Err(SceneError::MemoryBudget)
    );
    assert_eq!(geometry.depth_mode(), SceneDepthMode::Xray);
    assert_eq!(budget.stats(), before);
    assert_eq!(read_vertices(&device, &queue, &geometry), expected);
    // Enough room for early individual buffers, but not a complete bundle:
    // failed upload must neither allocate nor retire a partial candidate.
    assert!(matches!(
        renderer.upload_lod_mesh(&device, &mesh, &variants),
        Err(SceneError::MemoryBudget)
    ));
    assert!(matches!(
        renderer.reserve_geometry(&device, mesh.vertices.len(), mesh.indices.len()),
        Err(SceneError::MemoryBudget)
    ));
    assert_eq!(budget.stats(), before);
    let spare = budget
        .allocate_storage("no partial budget leak", &[0; 16])
        .unwrap();
    drop(spare);
    drop(compute);
    drop(geometry);
    assert_eq!(budget.stats().allocated_bytes, bytes + 20);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);

    let mut bundle = renderer.upload_lod_mesh(&device, &mesh, &variants).unwrap();
    assert_eq!(budget.stats().allocated_bytes, bundle_bytes);
    assert_eq!(budget.stats().allocated_buffers, 6); // Four shared streams, two indices.
    let vertex_owner = bundle.level(1).unwrap().vertices.clone();
    let shared_bytes = vertex_owner.size();
    assert_eq!(
        renderer.replace_lod_mesh(&device, &mut bundle, &mesh, &variants),
        Err(SceneError::MemoryBudget)
    );
    assert_eq!(budget.stats().allocated_bytes, bundle_bytes);
    assert_eq!(budget.stats().retired_buffers, 0);
    drop(bundle);
    assert_eq!(budget.stats().retired_buffers, 5);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, shared_bytes);
    assert_eq!(budget.stats().allocated_buffers, 1);
    drop(vertex_owner);
    assert_eq!(budget.stats().retired_buffers, 1);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
    let reserved = renderer
        .reserve_geometry(&device, mesh.vertices.len(), mesh.indices.len())
        .unwrap();
    assert_eq!(budget.stats().allocated_bytes, bytes);
    assert_eq!(reserved.index_count, 0);
    drop(reserved);
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
    assert!(pollster::block_on(scope.pop()).is_none());
    assert!(matches!(
        budget.allocate_storage("exhaustion", &vec![0; (bundle_bytes + 24) as usize]),
        Err(ComputeError::MemoryBudget)
    ));
}
