use super::*;
#[test]
fn shared_partitions_keep_vertex_owners_and_reject_invalid_replacement() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let scene = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mesh = SceneMesh::quad([1.; 4]);
    let mut parts = scene.upload_shared_mesh_partitions(&device, &mesh).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &parts[0].vertices,
        &parts[1].vertices
    ));
    assert!(std::sync::Arc::ptr_eq(&parts[0].normals, &parts[1].normals));
    assert!(
        parts[0]
            .update_index_partition_if_changed(&queue, &mesh.indices()[..3])
            .unwrap()
    );
    parts[1]
        .update_index_partition(&queue, &mesh.indices()[3..])
        .unwrap();
    let cached = parts[0].partition_cache.as_ptr();
    assert!(
        !parts[0]
            .update_index_partition_if_changed(&queue, &mesh.indices()[..3])
            .unwrap()
    );
    assert_eq!(parts[0].partition_cache.as_ptr(), cached);
    assert_eq!(parts[0].partition_cache, mesh.indices()[..3]);
    assert_eq!(parts[0].index_count(), 3);
    assert_eq!(parts[1].index_count(), 3);
    let mut deformed = mesh.clone();
    deformed.vertices[0].position[2] = 0.1;
    parts[0]
        .update_shared_vertex_streams(&queue, &deformed)
        .unwrap();
    assert_eq!(parts[0].index_count(), 3);
    assert!(parts[0].partitioned_indices);
    assert_eq!(parts[0].partition_cache, mesh.indices()[..3]);
    deformed.indices.truncate(3);
    parts[0]
        .update_shared_vertex_streams(&queue, &deformed)
        .unwrap();
    assert_eq!(parts[0].index_count(), 3);
    assert!(!parts[0].partitioned_indices);
    assert!(parts[0].partition_cache.is_empty());
    parts[0]
        .update_index_partition(&queue, &mesh.indices()[..3])
        .unwrap();
    parts[0].update(&queue, &mesh).unwrap();
    assert_eq!(parts[0].index_count(), 6);
    assert!(
        parts[0]
            .update_index_partition(&queue, &[0, 1, 999])
            .is_err()
    );
    assert_eq!(parts[0].index_count(), 6);
    let invalid = pollster::block_on(scene.set_geometry_opaque_shader(
        &device,
        &mut parts[0],
        "invalid wgsl",
    ));
    assert!(invalid.is_err());
    assert!(parts[0].opaque_shader.is_none());
}
