use super::*;
fn assets() -> Assets {
    use crate::import::{self, ImportedNode};
    use voxy_assets::{FileInputs, ImportInputs, SourcePath};
    use voxy_render::{ImageAsset, ImageLimits, SceneMesh, SceneVertex};
    let mesh = SceneMesh::new(
        vec![
            SceneVertex {
                position: [-0.5, -0.5, 0.],
                uv: [0., 0.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0.5, -0.5, 0.],
                uv: [1., 0.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0., 0.5, 0.],
                uv: [0.5, 1.],
                color: [1.; 4],
            },
        ],
        vec![0, 1, 2],
    )
    .unwrap();
    let asset = EditorAsset {
        mesh: mesh.clone(),
        lod: None,
        animated: None,
        skinned_lod: None,
        images: vec![ImageAsset::from_rgba(2, 2, vec![127; 16], ImageLimits::default()).unwrap()],
        nodes: vec![ImportedNode {
            name: "part".into(),
            parent: None,
            local: Default::default(),
            mesh: Some(mesh),
            image: Some(0),
            sampling: Default::default(),
            use_mips: true,
            geometry_key: Some((0, 0)),
        }],
    };
    let procedural = ImportInputs::new(0, 0)
        .finish(asset, |_, _| unreachable!())
        .unwrap();
    let mut second = procedural.value().clone();
    second.images[0] = ImageAsset::from_rgba(2, 2, vec![255; 16], ImageLimits::default()).unwrap();
    let second = ImportInputs::new(0, 0)
        .finish(second, |_, _| unreachable!())
        .unwrap();
    let provider = FileInputs::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../voxy_render/examples/assets"),
    )
    .unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    let animated = import::load(
        &SourcePath::new("animated-triangle.glb").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    let animated = inputs
        .finish(animated, |id, limit| provider.read(id, limit))
        .unwrap();
    BTreeMap::from([
        (AssetId("a".into()), Arc::new(procedural)),
        (AssetId("b".into()), Arc::new(second)),
        (AssetId("c".into()), Arc::new(animated)),
    ])
}
#[test]
#[ignore = "requires real GPU; detached model batch validation"]
fn gpu_batch_preserves_live_cache_on_budget_and_late_validation_failures() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("MODEL BATCH GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let assets = assets();
    let mut cache = ResidencyCache::with_budget(32 * 1024 * 1024);
    let old = ModelGraphics::upload_on(
        &renderer,
        &device,
        &queue,
        assets[&AssetId("a".into())].value(),
        &mut cache,
    )
    .unwrap();
    let old_bytes = old.geometry_bytes();
    let candidate_bytes: u64 = assets
        .values()
        .map(|asset| ModelGraphics::geometry_estimate(asset.value()))
        .sum();
    cache.geometry_budget = old_bytes + candidate_bytes - 1;
    cache.geometry_live = old_bytes;
    let before = cache.state();
    macro_rules! target {
        () => {
            UploadTarget {
                renderer: &renderer,
                device: &device,
                queue: &queue,
                cache: &cache,
                live_geometry: old_bytes,
                replaced_geometry: old_bytes,
            }
        };
    }
    let rejected = match target!().stage(assets.clone()) {
        Err(e) => e,
        Ok(_) => panic!("aggregate peak budget must reject"),
    };
    assert!(rejected.error.contains("budget"));
    assert!(Arc::ptr_eq(
        &rejected.assets[&AssetId("a".into())],
        &assets[&AssetId("a".into())]
    ));
    assert_eq!(cache.state(), before);
    cache.geometry_budget = old_bytes + candidate_bytes;
    let before = cache.state();
    let rejected = match target!().stage_with(assets.clone(), || Err("late owner failure".into())) {
        Err(e) => e,
        Ok(_) => panic!("late failure must reject"),
    };
    assert_eq!(rejected.error, "late owner failure");
    assert_eq!(cache.state(), before);
    let pending = target!()
        .stage_with(assets.clone(), || {
            let _invalid = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("intentional batch scope error"),
                size: 16,
                usage: wgpu::BufferUsages::empty(),
                mapped_at_creation: false,
            });
            Ok(())
        })
        .unwrap_or_else(|e| panic!("{}", e.error));
    let rejected = match pollster::block_on(pending.validate()) {
        Err(e) => e,
        Ok(_) => panic!("device validation must reject"),
    };
    assert!(!rejected.error.is_empty());
    assert_eq!(cache.state(), before);
    let pending = target!()
        .stage(assets.clone())
        .unwrap_or_else(|e| panic!("{}", e.error));
    let ready = pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.error));
    assert_eq!(cache.state(), before);
    assert_eq!(ready.cache.geometry_live, candidate_bytes);
    assert_eq!(ready.models.len(), 3);
    for (id, source) in &assets {
        assert!(Arc::ptr_eq(&ready.assets[id], source));
        assert!(ready.models[id].geometry_residency_valid(source.value()));
    }
    assert_eq!(old.geometry_bytes(), old_bytes);
    assert!(cache.live_bytes() < ready.cache.live_bytes());
    // Dropping a pending batch must leave the device's error-scope stack usable.
    drop(
        target!()
            .stage(assets)
            .unwrap_or_else(|e| panic!("{}", e.error)),
    );
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    assert!(pollster::block_on(scope.pop()).is_none());
}
