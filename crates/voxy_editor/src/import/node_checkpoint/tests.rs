use super::*;
fn source() -> (Vec<ImportedNode>, Vec<ImageAsset>) {
    let mesh = SceneMesh::new(
        vec![
            SceneVertex {
                position: [0., 0., 0.],
                uv: [0., 0.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [1., 0., 0.],
                uv: [1., 0.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0., 1., 0.],
                uv: [0., 1.],
                color: [1.; 4],
            },
        ],
        vec![0, 1, 2],
    )
    .unwrap()
    .with_normals(vec![[0., 0., 1.]; 3])
    .unwrap();
    let node = ImportedNode {
        name: "mesh 🦊".into(),
        parent: None,
        local: Transform {
            translation: Vec3::new(-0., f32::from_bits(1), 0.),
            rotation: glam::Quat::from_array([0., -0., 0.0001, 1.0000001]),
            scale: Vec3::new(-1., 2., 0.5),
        },
        mesh: Some(mesh),
        image: Some(0),
        sampling: TextureSampling {
            wrap_u: TextureWrap::Mirror,
            wrap_v: TextureWrap::Repeat,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            mipmap_filter: Some(TextureFilter::Linear),
            anisotropy: 16,
        },
        use_mips: true,
        geometry_key: Some((9, 3)),
    };
    let mut child = node.clone();
    child.name = "child".into();
    child.parent = Some(0);
    child.mesh = None;
    child.image = None;
    child.sampling = TextureSampling::default();
    child.geometry_key = None;
    let image = ImageAsset::from_rgba(
        2,
        2,
        vec![1, 2, 3, 4, 9, 8, 7, 6, 255, 0, 127, 255, 0, 255, 1, 255],
        ImageLimits::default(),
    )
    .unwrap();
    (vec![node, child], vec![image])
}
#[test]
fn exact_node_fields_image_mips_and_120_hierarchy_transforms_survive_file() {
    let limits = NodeCheckpointLimits::default();
    let (nodes, images) = source();
    let checkpoint = ImportedNodesCheckpoint::capture(&nodes, &images, limits).unwrap();
    let path = std::env::temp_dir().join(format!(
        "voxy-imported-nodes-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, checkpoint.to_json(limits.max_bytes).unwrap()).unwrap();
    let json = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let restored = ImportedNodesCheckpoint::from_json(&json, limits.max_bytes)
        .unwrap()
        .prepare(limits)
        .unwrap();
    assert_eq!(
        ImportedNodesCheckpoint::capture(&restored.nodes, &restored.images, limits).unwrap(),
        checkpoint
    );
    assert_eq!(
        images[0]
            .mip_chain()
            .iter()
            .map(ImageAsset::checkpoint)
            .collect::<Vec<_>>(),
        restored.images[0]
            .mip_chain()
            .iter()
            .map(ImageAsset::checkpoint)
            .collect::<Vec<_>>()
    );
    for frame in 0..120 {
        let instance = Mat4::from_rotation_y(frame as f32 / 119.);
        let matrix = |nodes: &[ImportedNode]| {
            let a = instance * nodes[0].local.matrix().unwrap();
            (a * nodes[1].local.matrix().unwrap())
                .to_cols_array()
                .map(f32::to_bits)
        };
        assert_eq!(matrix(&nodes), matrix(&restored.nodes));
    }
}
#[test]
fn invalid_hierarchy_transform_source_and_aggregate_limits_reject_without_live_changes() {
    let limits = NodeCheckpointLimits::default();
    let (nodes, images) = source();
    let original = ImportedNodesCheckpoint::capture(&nodes, &images, limits).unwrap();
    let mut collision = original.clone();
    let mesh =
        SceneMeshCheckpoint::from_bytes(collision.nodes[0].mesh.as_ref().unwrap(), limits.geometry)
            .unwrap()
            .prepare(limits.geometry)
            .unwrap();
    let mut vertices = mesh.vertices().to_vec();
    vertices[0].position[0] = 100.;
    let other = SceneMesh::new(vertices, mesh.indices().to_vec()).unwrap();
    let mut duplicate = collision.nodes[0].clone();
    duplicate.parent = Some(0);
    duplicate.mesh = Some(other.checkpoint().to_bytes(limits.geometry).unwrap());
    collision.nodes.push(duplicate);
    assert!(collision.prepare(limits).is_err());
    let mut cases = Vec::new();
    let mut r = original.clone();
    r.nodes[0].parent = Some(1);
    cases.push(r);
    let mut r = original.clone();
    r.nodes[1].image = Some(9);
    cases.push(r);
    let mut r = original.clone();
    r.nodes[1].local[0] = f32::NAN.to_bits();
    cases.push(r);
    let mut r = original.clone();
    r.nodes[0].sampling.mip = Some(255);
    cases.push(r);
    let mut r = original.clone();
    r.version = 0;
    cases.push(r);
    let mut r = original.clone();
    *r.images[0].last_mut().unwrap() ^= 1;
    cases.push(r);
    for r in cases {
        assert!(r.prepare(limits).is_err());
    }
    for limits in [
        NodeCheckpointLimits {
            max_pixel_bytes: 15,
            ..limits
        },
        NodeCheckpointLimits {
            max_vertices: 2,
            ..limits
        },
        NodeCheckpointLimits {
            max_indices: 2,
            ..limits
        },
        NodeCheckpointLimits {
            max_bytes: 1,
            ..limits
        },
        NodeCheckpointLimits {
            max_string_bytes: 0,
            ..limits
        },
    ] {
        assert!(original.prepare(limits).is_err());
    }
    assert_eq!(
        ImportedNodesCheckpoint::capture(&nodes, &images, limits).unwrap(),
        original
    );
    let empty = ImportedNodesCheckpoint::capture(&[], &[], limits)
        .unwrap()
        .prepare(limits)
        .unwrap();
    assert!(empty.nodes.is_empty() && empty.images.is_empty());
}
