#[test]
fn disjoint_views_validate_regions_and_record_shared_geometry() {
    let empty = [];
    let regions = |rects: &[[u32; 4]]| {
        rects
            .iter()
            .map(|&viewport| SceneView {
                viewport,
                draws: &empty,
            })
            .collect::<Vec<_>>()
    };
    assert!(validate_view_regions([64, 32], &regions(&[[0, 0, 32, 32], [32, 0, 32, 32],])).is_ok());
    for rects in [
        vec![[0, 0, 0, 32]],
        vec![[0, 0, 32, 0]],
        vec![[33, 0, 32, 32]],
        vec![[u32::MAX, 0, 2, 1]],
        vec![[0, 0, 33, 32], [32, 0, 32, 32]],
        vec![[0, 0, 64, 32], [1, 1, 1, 1]],
    ] {
        assert_eq!(
            validate_view_regions([64, 32], &regions(&rects)),
            Err(SceneError::InvalidGeometry)
        );
    }
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = renderer
        .upload_mesh(&device, &SceneMesh::quad([1.; 4]))
        .unwrap();
    let texture = renderer
        .upload_texture(&device, &queue, 1, 1, &[255; 4])
        .unwrap();
    let left_transform = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    let right_transform = renderer
        .create_transform(&device, Mat4::from_scale(glam::Vec3::splat(0.5)))
        .unwrap();
    let left = [SceneDraw {
        geometry: &geometry,
        texture: &texture,
        transform: &left_transform,
        overlay: false,
    }];
    let right = [SceneDraw {
        geometry: &geometry,
        texture: &texture,
        transform: &right_transform,
        overlay: false,
    }];
    let target = |format| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("disjoint views test"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    let color = target(wgpu::TextureFormat::Rgba8Unorm);
    let depth = target(wgpu::TextureFormat::Depth32Float);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    assert_eq!(
        renderer.encode_views(
            &mut encoder,
            &color,
            &depth,
            wgpu::Color::BLACK,
            &regions(&[[0, 0, 33, 32], [32, 0, 32, 32]])
        ),
        Err(SceneError::InvalidGeometry)
    );
    assert_eq!(
        renderer.encode_views_msaa4(
            &mut encoder,
            &color,
            &depth,
            &color,
            wgpu::Color::BLACK,
            &[]
        ),
        Err(SceneError::MultisamplingNotEnabled)
    );
    assert_eq!(
        renderer.encode_view_frame(&mut encoder, &color, &depth, wgpu::Color::BLACK, &[], &left),
        Err(SceneError::InvalidGeometry)
    );
    renderer
        .encode_views(
            &mut encoder,
            &color,
            &depth,
            wgpu::Color::BLACK,
            &[
                SceneView {
                    viewport: [0, 0, 32, 32],
                    draws: &left,
                },
                SceneView {
                    viewport: [32, 0, 32, 32],
                    draws: &right,
                },
            ],
        )
        .unwrap();
    queue.submit([encoder.finish()]);
}

#[test]
fn material_layer_split_preserves_binding_coordinates_and_parameters() {
    let vertices = (0..6)
        .map(|i| SceneVertex {
            position: [
                if i % 3 == 1 { 1. } else { 0. },
                if i % 3 == 2 { 1. } else { 0. },
                (i / 3) as f32 * 0.001,
            ],
            uv: [if i < 3 { 0. } else { -2. }, 0.00001],
            color: [1.; 4],
        })
        .collect();
    let coordinates = vec![[0.123, 0.456, 0.789]; 6];
    let mesh = SceneMesh::new(vertices, vec![0, 1, 2, 3, 4, 5])
        .unwrap()
        .with_material_coordinates(coordinates.clone())
        .unwrap()
        .with_material_parameters([1.44, 0.3, 0.2, 0.1])
        .unwrap();
    let (opaque, film) = mesh.split_material_layer(-2., -6.).unwrap().unwrap();
    assert_eq!(opaque.indices(), &[0, 1, 2]);
    assert_eq!(film.indices(), &[3, 4, 5]);
    for output in [&opaque, &film] {
        assert_eq!(
            output.explicit_material_coordinates(),
            Some(coordinates.as_slice())
        );
        assert_eq!(output.material_parameters, mesh.material_parameters);
        assert_eq!(
            output
                .vertices()
                .iter()
                .map(|v| v.position)
                .collect::<Vec<_>>(),
            mesh.vertices()
                .iter()
                .map(|v| v.position)
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(mesh.vertices()[3].uv[0], -2.);
    assert_eq!(film.vertices()[3].uv[0], -6.);
}
#[test]
fn material_layer_split_rejects_ambiguous_faces_and_clears_absent_layer() {
    let mut mesh = SceneMesh::quad([1.; 4]);
    assert!(mesh.split_material_layer(-2., -6.).unwrap().is_none());
    mesh.vertices[0].uv[0] = -2.;
    assert_eq!(
        mesh.split_material_layer(-2., -6.).unwrap_err(),
        SceneError::InvalidGeometry
    );
    assert_eq!(
        mesh.split_material_layer(f32::NAN, -6.).unwrap_err(),
        SceneError::NonFiniteVertex
    );
}
#[test]
fn explicit_material_coordinates_survive_geometry_deformation() {
    let mut mesh = SceneMesh::quad([1.; 4]);
    let coordinates = mesh.material_coordinates().into_owned();
    assert!(
        mesh.clone()
            .with_material_coordinates(vec![[0.; 3]; 1])
            .is_err()
    );
    let mut invalid = coordinates.clone();
    invalid[0][0] = f32::NAN;
    assert!(mesh.clone().with_material_coordinates(invalid).is_err());
    mesh = mesh.with_material_coordinates(coordinates.clone()).unwrap();
    for v in &mut mesh.vertices {
        v.position[0] *= 1.5;
        v.position[1] += 0.2;
    }
    assert_eq!(mesh.material_coordinates().as_ref(), coordinates.as_slice());
    assert_ne!(
        mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        coordinates
    );
}

#[test]
fn cancelling_and_degenerate_fans_have_finite_zero_normals() {
    let v = |position| super::SceneVertex {
        position,
        uv: [0.; 2],
        color: [1.; 4],
    };
    let mesh = super::SceneMesh::new(
        vec![v([0., 0., 0.]), v([1., 0., 0.]), v([0., 1., 0.])],
        vec![0, 1, 2, 0, 2, 1, 0, 0, 0],
    )
    .unwrap();
    assert!(super::smooth_normals(&mesh).iter().all(|n| *n == [0.; 3]));
}
#[test]
fn smooth_normals_weld_seams_and_follow_deformation() {
    let v = |position| super::SceneVertex {
        position,
        uv: [0., 0.],
        color: [1.; 4],
    };
    let vertices = vec![
        v([0., 0., 0.]),
        v([1., 0., 0.]),
        v([0., 1., 0.]),
        v([-0., 0., 0.]),
        v([0., 1., 0.]),
        v([0., 0., 1.]),
    ];
    let mesh = super::SceneMesh::new(vertices.clone(), vec![0, 1, 2, 3, 4, 5]).unwrap();
    let mut cache = super::NormalCache::from_mesh(&mesh);
    assert!(!cache.refresh(&mesh));
    let mut recolored = vertices.clone();
    recolored[0].color = [0.2; 4];
    recolored[0].uv = [0.4, 0.8];
    let recolored = super::SceneMesh::new(recolored, vec![0, 1, 2, 3, 4, 5]).unwrap();
    assert!(!cache.refresh(&recolored));
    let normals = super::smooth_normals(&mesh);
    assert_eq!(normals[0], normals[3]);
    assert_eq!(normals[2], normals[4]);
    let expected = glam::Vec3::new(1., 0., 1.).normalize();
    assert!((glam::Vec3::from_array(normals[0]) - expected).length() < 1e-6);
    let mut changed = vertices;
    changed[5].position = [0., 0., 2.];
    let mesh = super::SceneMesh::new(changed, vec![0, 1, 2, 3, 4, 5]).unwrap();
    assert!(cache.refresh(&mesh));
    assert!(!cache.refresh(&mesh));
    let expected = glam::Vec3::new(2., 0., 1.).normalize();
    assert!((glam::Vec3::from_array(super::smooth_normals(&mesh)[0]) - expected).length() < 1e-6);
}
use super::*;

#[test]
fn mesh_admission_rejects_every_nonfinite_vertex_lane() {
    let base = SceneVertex {
        position: [0.; 3],
        uv: [0.; 2],
        color: [1.; 4],
    };
    for lane in 0..9 {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut v = base;
            match lane {
                0..=2 => v.position[lane] = value,
                3..=4 => v.uv[lane - 3] = value,
                _ => v.color[lane - 5] = value,
            }
            assert!(matches!(
                SceneMesh::new(vec![v; 3], vec![0, 1, 2]),
                Err(SceneError::NonFiniteVertex)
            ));
        }
    }
    let v = SceneVertex {
        position: [f32::from_bits(1), -0., f32::MAX],
        uv: [0.; 2],
        color: [1.; 4],
    };
    assert!(SceneMesh::new(vec![v; 3], vec![0, 1, 2]).is_ok());
}

#[test]
fn rejects_malformed_mesh_and_non_finite_helper_input() {
    let valid = SceneMesh::quad([1.0; 4]);
    assert_eq!(
        SceneMesh::new(valid.vertices.clone(), vec![0, 1, 9]).unwrap_err(),
        SceneError::InvalidIndex
    );
    assert_eq!(
        SceneMesh::new(valid.vertices.clone(), vec![0, 1]).unwrap_err(),
        SceneError::InvalidGeometry
    );
    let invalid = SceneMesh::quad([f32::NAN; 4]);
    assert_eq!(
        SceneMesh::validate(&invalid.vertices, &invalid.indices),
        Err(SceneError::NonFiniteVertex)
    );
}

#[test]
fn alpha_sort_tracks_camera_and_rejects_bad_transforms_atomically() {
    let near = SceneMesh::quad([1.0; 4]);
    let mut vertices = near.vertices.clone();
    vertices.extend(near.vertices.iter().map(|v| SceneVertex {
        position: [v.position[0], v.position[1], -2.0],
        ..*v
    }));
    let mut mesh = SceneMesh::new(vertices, vec![0, 1, 2, 4, 5, 6]).unwrap();
    mesh.sort_back_to_front(Mat4::IDENTITY).unwrap();
    assert_eq!(mesh.indices, [4, 5, 6, 0, 1, 2]);
    mesh.sort_back_to_front(Mat4::from_rotation_y(std::f32::consts::PI))
        .unwrap();
    assert_eq!(mesh.indices, [0, 1, 2, 4, 5, 6]);
    let before = mesh.indices.clone();
    for invalid in [
        Mat4::ZERO,
        Mat4::from_cols_array(&[f32::NAN; 16]),
        Mat4::from_cols(
            glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
            glam::Vec4::Y,
            glam::Vec4::Z,
            glam::Vec4::W,
        ),
    ] {
        assert_eq!(
            mesh.sort_back_to_front(invalid),
            Err(SceneError::InvalidTransform)
        );
        assert_eq!(mesh.indices, before);
    }
}

#[test]
fn resource_validation_precedes_gpu_upload() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    for wrap in [TextureWrap::Clamp, TextureWrap::Repeat, TextureWrap::Mirror] {
        for filter in [TextureFilter::Nearest, TextureFilter::Linear] {
            renderer
                .upload_texture_with_sampling(
                    &device,
                    &queue,
                    1,
                    1,
                    &[255; 4],
                    TextureSampling {
                        wrap_u: wrap,
                        wrap_v: wrap,
                        min_filter: filter,
                        mag_filter: filter,
                        ..TextureSampling::default()
                    },
                )
                .unwrap();
        }
    }
    assert!(matches!(
        renderer.upload_texture_with_sampling(
            &device,
            &queue,
            1,
            1,
            &[255; 4],
            TextureSampling {
                anisotropy: 2,
                mipmap_filter: Some(TextureFilter::Nearest),
                ..TextureSampling::default()
            },
        ),
        Err(SceneError::InvalidTexture)
    ));
    let image = renderer
        .upload_texture(&device, &queue, 1, 1, &[255; 4])
        .unwrap();
    let rebound = renderer
        .texture_binding(
            &device,
            &image,
            TextureSampling {
                min_filter: TextureFilter::Nearest,
                ..TextureSampling::default()
            },
            1,
        )
        .unwrap();
    assert_eq!(image.texture(), rebound.texture());
    for count in [0, 2] {
        assert!(matches!(
            renderer.texture_binding(&device, &image, TextureSampling::default(), count),
            Err(SceneError::InvalidTexture)
        ));
    }
    let (foreign, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    assert!(matches!(
        renderer.texture_binding(&foreign, &image, TextureSampling::default(), 1),
        Err(SceneError::DeviceMismatch)
    ));
    for (vertices, indices) in [(0, 3), (4, 0), (usize::MAX, 3), (4, usize::MAX)] {
        assert!(matches!(
            renderer.reserve_geometry(&device, vertices, indices),
            Err(SceneError::GeometryCapacityExceeded)
        ));
    }
    let mut reserved = renderer.reserve_geometry(&device, 8, 12).unwrap();
    assert_eq!(reserved.capacity(), (8, 12));
    assert_eq!(reserved.index_count, 0);
    let mesh = SceneMesh::quad([1.0; 4]);
    let mut geometry = renderer.upload_mesh(&device, &mesh).unwrap();
    let triangle = SceneMesh::new(mesh.vertices.clone(), vec![0, 1, 2]).unwrap();
    reserved.update(&queue, &triangle).unwrap();
    reserved.clear();
    reserved.update(&queue, &mesh).unwrap();
    assert_eq!(reserved.index_count(), 6);
    geometry.update(&queue, &triangle).unwrap();
    assert_eq!(geometry.index_count(), 3);
    let oversized = SceneMesh::new(mesh.vertices.clone(), vec![0, 1, 2, 0, 2, 3, 0, 1, 2]).unwrap();
    assert_eq!(
        geometry.update(&queue, &oversized),
        Err(SceneError::GeometryCapacityExceeded)
    );
    assert_eq!(geometry.index_count(), 3);
    assert_eq!(
        geometry.update(&queue, &SceneMesh::quad([f32::NAN; 4])),
        Err(SceneError::NonFiniteVertex)
    );
    assert_eq!(geometry.index_count(), 3);
    for (width, height, rgba) in [
        (0, 1, &[][..]),
        (1, 1, &[255; 3][..]),
        (u32::MAX, u32::MAX, &[][..]),
    ] {
        assert!(matches!(
            renderer.upload_texture(&device, &queue, width, height, rgba),
            Err(SceneError::InvalidTexture)
        ));
    }
    assert!(matches!(
        renderer.create_transform(&device, Mat4::from_cols_array(&[f32::NAN; 16])),
        Err(SceneError::InvalidTransform)
    ));
    assert!(pollster::block_on(scope.pop()).is_none());
}
