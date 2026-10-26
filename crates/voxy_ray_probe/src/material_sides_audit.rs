//! Exact material-sided nearest policy through the existing GPU job owner.
#[path = "exact_vertex_cache_audit.rs"]
mod exact_vertex_cache_audit;
use glam::{Mat4, Vec3};
use std::sync::Arc;
use voxy_render::{
    ExactWorldOptions, RayScene, RaySceneError, WorldRayQueryJob, WorldRayQueryKind,
    archive_scene::*,
};

type Expected = (u32, u32, f32, f32, u32);
#[derive(Clone)]
struct Case {
    name: &'static str,
    words: [u32; 20],
    geometric: Expected,
    surface: Expected,
}
fn ray(
    name: &'static str,
    x: f32,
    z: f32,
    dz: f32,
    geometric: Expected,
    surface: Expected,
) -> Case {
    let mut words = [0; 20];
    words[0] = x.to_bits();
    words[1] = (-0.5f32).to_bits();
    words[2] = z.to_bits();
    words[3] = 1;
    words[10] = dz.to_bits();
    Case {
        name,
        words,
        geometric,
        surface,
    }
}
fn mesh(
    x: f32,
    z: f32,
    mirror: bool,
    reverse: bool,
    singular: bool,
    material: usize,
) -> ArchiveMeshSource {
    let transform = Mat4::from_translation(Vec3::new(x, 0., z))
        * Mat4::from_scale(Vec3::new(
            if singular {
                0.
            } else if mirror {
                -1.
            } else {
                1.
            },
            1.,
            1.,
        ));
    ArchiveMeshSource {
        vertices: [[-1., -1., 0.], [1., -1., 0.], [-1., 1., 0.]]
            .map(|position| ArchiveVertex {
                position,
                normal: [0., 0., 1.],
                tangent: [1., 0., 0., 1.],
                uv: [0.; 2],
            })
            .to_vec(),
        indices: if reverse {
            vec![0, 2, 1]
        } else {
            vec![0, 1, 2]
        },
        instances: vec![ArchiveInstance::new(transform)],
        material,
        center: Vec3::ZERO,
    }
}
pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let exact_workers = std::env::var("VOXY_RAY_MATERIAL_SIDES_WORKERS")
        .ok()
        .map(|s| s.parse::<std::num::NonZeroU32>())
        .transpose()?;
    let exact_group = std::env::var("VOXY_RAY_MATERIAL_SIDES_GROUP")
        .ok()
        .map(|s| s.parse::<std::num::NonZeroU32>())
        .transpose()?;
    let hit_uv = std::env::var("VOXY_RAY_MATERIAL_SIDES_UV").as_deref() == Ok("1");
    let record_bytes = if hit_uv { 112 } else { 80 };
    let hit_position = std::env::var("VOXY_RAY_MATERIAL_SIDES_POSITION").as_deref() == Ok("1");
    let deferred = std::env::var("VOXY_RAY_MATERIAL_SIDES_DEFERRED").as_deref() == Ok("1");
    let native = std::env::var("VOXY_RAY_MATERIAL_SIDES_NATIVE").as_deref() == Ok("1");
    let separate = std::env::var("VOXY_RAY_MATERIAL_SIDES_SEPARATE").as_deref() == Ok("1");
    let previous = std::env::var("VOXY_RAY_MATERIAL_SIDES_PREVIOUS").as_deref() == Ok("1");
    let known = std::env::var("VOXY_RAY_MATERIAL_SIDES_KNOWN").as_deref() == Ok("1");
    let mut materials = (0..4)
        .map(|i| ArchiveMaterialSource {
            parameters: ArchiveMaterial::metallic_roughness([1.; 4], 0., 0.5, 0., -1.),
            images: [None; 4],
            double_sided: i == 1,
        })
        .collect::<Vec<_>>();
    materials[2].parameters.surface[3] = 0.5;
    materials[2].images[0] = Some(0);
    materials[3].parameters.surface[2] = 0.8;
    if known {
        materials[2].images[0] = None;
    }
    let mut meshes = vec![
        mesh(0., 0., false, false, false, 0),
        mesh(0., 0.5, false, true, false, 0),
        mesh(4., 0., true, false, false, 0),
        mesh(8., 0., false, false, false, 1),
        mesh(12., 0., false, false, true, 0),
        mesh(100000000., 0., false, false, false, 0),
        mesh(16., 0., false, false, false, 2),
        mesh(20., 0., false, false, false, 3),
        mesh(24., 0., false, false, false, 0),
        mesh(24., 0., false, true, false, 0),
    ];
    meshes.push(mesh(28., 0., false, false, false, 0));
    let mut coplanar = mesh(27.5, 0., false, false, false, 0);
    for (v, position) in
        coplanar
            .vertices
            .iter_mut()
            .zip([[0., -1., -0.75], [0., 1., -0.75], [0., -1., -0.5]])
    {
        v.position = position;
        v.normal = [1., 0., 0.];
    }
    meshes.push(coplanar);
    for _ in 0..8 {
        meshes.push(mesh(36., 0., false, false, false, 0));
    }
    let mut oblique = mesh(40., 0., false, false, false, 0);
    for (v, position) in
        oblique
            .vertices
            .iter_mut()
            .zip([[-1., 0., 1.], [0., -1., 1.], [0., 0., 0.]])
    {
        v.position = position;
        v.normal = [1., 1., 1.];
    }
    meshes.push(oblique);
    let event_front = meshes.len() as u32;
    meshes.extend([
        mesh(48., 0., false, false, false, 0),
        mesh(48., -0.5, false, false, false, 2),
    ]);
    let event_near = meshes.len() as u32;
    meshes.extend([
        mesh(52., 0., false, false, false, 0),
        mesh(52., 0.5, false, false, false, 2),
    ]);
    let event_tie = meshes.len() as u32;
    meshes.extend([
        mesh(56., 0., false, false, false, 0),
        mesh(56., 0., false, false, false, 2),
    ]);
    let event_reverse = meshes.len() as u32;
    meshes.extend([
        mesh(60., -0.5, false, false, false, 2),
        mesh(60., 0., false, false, false, 0),
    ]);
    let event_tiny_far = meshes.len() as u32;
    meshes.extend([
        mesh(64., 0., false, false, false, 0),
        mesh(64., -2f32.powi(-40), false, false, false, 2),
    ]);
    let event_tiny_near = meshes.len() as u32;
    meshes.extend([
        mesh(68., 0., false, false, false, 0),
        mesh(68., 2f32.powi(-40), false, false, false, 2),
    ]);
    let event_priority = meshes.len() as u32;
    meshes.push(mesh(72., -0.5, false, false, false, 0));
    let mut side_uncertain = mesh(71.5, 0., false, false, false, 0);
    for (v, position) in
        side_uncertain
            .vertices
            .iter_mut()
            .zip([[0., -1., 0.5], [0., 1., 0.5], [0., -1., 0.]])
    {
        v.position = position;
        v.normal = [1., 0., 0.];
    }
    meshes.push(side_uncertain);
    meshes.push(mesh(72., -1., false, false, false, 2));

    // Directed canonical paired ray: a low origin word changes the source ID.
    // Hardware/high-word hints remain optional; the complete source walk wins.
    let paired_hint_slots =
        if std::env::var("VOXY_RAY_MATERIAL_SIDES_PAIRED_HINT_CASE").as_deref() == Ok("1") {
            let first = meshes.len() as u32;
            let unit = 2f32.powi(-20);
            let z = 36. * unit;
            for xs in [[45.5, 47.5, 47.5], [47., 49., 49.]] {
                let mut triangle = mesh(0., 0., false, false, false, 0);
                for (vertex, point) in triangle.vertices.iter_mut().zip([
                    [xs[0] * unit, -1., z],
                    [xs[1] * unit, -1., z],
                    [xs[2] * unit, 1., z],
                ]) {
                    vertex.position = point;
                }
                meshes.push(triangle);
            }
            Some(first)
        } else {
            None
        };

    // World x=80+2^-150 cannot fit the compact cache; the source ray still
    // reaches it at exact t=1/2 using both original subnormal low words.
    let vertex_cache_fallback_slot = if std::env::var(
        "VOXY_RAY_MATERIAL_SIDES_VERTEX_CACHE_FALLBACK_CASE",
    )
    .as_deref()
        == Ok("1")
    {
        let slot = meshes.len() as u32;
        let mut triangle = mesh(80., 0., false, false, false, 0);
        triangle.instances = vec![ArchiveInstance::new(
            Mat4::from_translation(Vec3::new(80., 0., 0.))
                * Mat4::from_scale(Vec3::new(0.5, 1., 1.)),
        )];
        for (vertex, point) in triangle.vertices.iter_mut().zip([
            [f32::from_bits(1), -1., -1.],
            [f32::from_bits(1), 1., -1.],
            [f32::from_bits(1), -1., 1.],
        ]) {
            vertex.position = point;
            vertex.normal = [1., 0., 0.];
        }
        meshes.push(triangle);
        Some(slot)
    } else {
        None
    };

    // UV primary records have ten candidate slots rather than six. Preserve
    // stable existing source IDs while exercising overflow of either layout.
    if hit_uv {
        for _ in 0..3 {
            meshes.push(mesh(36., 0., false, false, false, 0));
        }
    }
    // Two coincident farther events must not survive as ties of a closer hit.
    let transient_tie_slot = meshes.len() as u32;
    meshes.extend([mesh(84.,0.,false,false,false,0),mesh(84.,0.,false,false,false,0),mesh(84.,0.5,false,false,false,0)]);
    let scene = Arc::new(RayScene::from_archive_candidates(device, &meshes, 0.)?);
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.build(&mut encoder);
    let submission = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    let hit = |slot, t| (1, slot, t, 0., 0);
    let miss = (0, u32::MAX, 0., 0., 0);
    let unknown = (2, u32::MAX, 0., 0., 0);
    let mut cases = vec![
        ray(
            "skip nearer back face",
            -0.5,
            1.,
            -2.,
            hit(1, 0.25),
            hit(0, 0.5),
        ),
        ray(
            "reverse ray skips opposite face",
            -0.5,
            -1.,
            2.,
            hit(0, 0.5),
            hit(1, 0.75),
        ),
        ray("miss", 32., 1., -2., miss, miss),
        ray("mirrored back face", 4.5, 1., -2., hit(2, 0.5), miss),
        ray(
            "mirrored front face",
            4.5,
            -1.,
            2.,
            hit(2, 0.5),
            hit(2, 0.5),
        ),
        ray("two sided front", 7.5, 1., -2., hit(3, 0.5), hit(3, 0.5)),
        ray("two sided back", 7.5, -1., 2., hit(3, 0.5), hit(3, 0.5)),
        ray(
            "singular one sided orientation unknown",
            12.,
            1.,
            -2.,
            hit(4, 0.5),
            unknown,
        ),
        ray(
            "unknown front alpha remains unknown",
            15.5,
            1.,
            -2.,
            unknown,
            unknown,
        ),
        ray(
            "unknown alpha is irrelevant on back face",
            15.5,
            -1.,
            2.,
            unknown,
            miss,
        ),
        ray(
            "transmission remains optical",
            19.5,
            1.,
            -2.,
            (1, 7, 0.5, 0., 2),
            (1, 7, 0.5, 0., 2),
        ),
        ray(
            "one sided optical back is rejected",
            19.5,
            -1.,
            2.,
            (1, 7, 0.5, 0., 2),
            miss,
        ),
        ray(
            "same distance tie front",
            23.5,
            1.,
            -2.,
            hit(8, 0.5),
            hit(8, 0.5),
        ),
        ray(
            "same distance tie back",
            23.5,
            -1.,
            2.,
            hit(8, 0.5),
            hit(9, 0.5),
        ),
    ];
    let mut c = ray(
        "one sided coplanar remains unknown",
        -0.5,
        0.,
        0.,
        hit(0, 0.),
        unknown,
    );
    c.words[8] = 0.25f32.to_bits();
    cases.push(c);
    let mut c = ray(
        "two sided coplanar remains closed",
        7.5,
        0.,
        0.,
        hit(3, 0.),
        hit(3, 0.),
    );
    c.words[8] = 0.25f32.to_bits();
    cases.push(c);
    let mut c = ray(
        "large origin retains low coordinate",
        100000000.,
        1.,
        -2.,
        hit(5, 0.5),
        hit(5, 0.5),
    );
    c.words[4] = (-0.5f32).to_bits();
    cases.push(c);
    let mut c = ray(
        "t low word survives side filtering",
        -0.5,
        1.,
        -2.,
        (1, 1, 0.25, 2f32.powi(-26), 0),
        (1, 0, 0.5, 2f32.powi(-26), 0),
    );
    c.words[6] = 2f32.powi(-25).to_bits();
    cases.push(c);
    let mut c = ray(
        "inactive stays inactive",
        -0.5,
        1.,
        -2.,
        (3, u32::MAX, 0., 0., 0),
        (3, u32::MAX, 0., 0., 0),
    );
    c.words[3] = 0;
    cases.push(c);
    let mut c = ray(
        "invalid producer admission stays unknown",
        -0.5,
        1.,
        -2.,
        unknown,
        unknown,
    );
    c.words[3] = 2;
    cases.push(c);
    if known {
        cases[8].geometric = (1, 6, 0.5, 0., 1);
        cases[8].surface = (1, 6, 0.5, 0., 1);
        cases[9].geometric = (1, 6, 0.5, 0., 1);
    }
    cases.push(ray(
        "farther ambiguous side cannot precede known hit",
        27.5,
        1.,
        -2.,
        hit(10, 0.5),
        hit(10, 0.5),
    ));
    let mut overflow = ray(
        "candidate list overflow retries complete source",
        35.5,
        0.,
        0.,
        hit(12, 0.),
        unknown,
    );
    overflow.words[8] = 0.25f32.to_bits();
    cases.push(overflow);
    for (name, sign, surface) in [
        (
            "tiny front determinant keeps low direction",
            -1.,
            hit(20, 0.5),
        ),
        ("tiny back determinant stays back", 1., miss),
    ] {
        let eta = 2f32.powi(-40) * sign;
        let mut c = ray(name, 39.625, 0.5, 0., hit(20, 0.5), surface);
        c.words[1] = (-0.125f32).to_bits();
        c.words[8] = 0.25f32.to_bits();
        c.words[9] = (-0.25f32).to_bits();
        c.words[6] = (-eta * 0.5).to_bits();
        c.words[14] = eta.to_bits();
        cases.push(c);
    }
    let mut c = ray(
        "exactly coplanar oblique remains unknown",
        39.625,
        0.5,
        0.,
        hit(20, 0.),
        unknown,
    );
    c.words[1] = (-0.125f32).to_bits();
    c.words[8] = 0.25f32.to_bits();
    c.words[9] = (-0.25f32).to_bits();
    cases.push(c);
    if hit_position {
        for (name, dx) in [
            ("rational t exact position", 0.),
            ("rational t residual position", 1.),
        ] {
            let mut c = ray(
                name,
                -0.5,
                1.,
                -3.,
                (
                    1,
                    1,
                    f32::from_bits(0x3e2aaaab),
                    f32::from_bits(0xb1aaaaab),
                    0,
                ),
                (
                    1,
                    0,
                    f32::from_bits(0x3eaaaaab),
                    f32::from_bits(0xb22aaaab),
                    0,
                ),
            );
            c.words[8] = (dx as f32).to_bits();
            cases.push(c);
        }
    }
    cases.extend([
        ray(
            "unknown alpha strictly behind known hit",
            47.5,
            1.,
            -2.,
            if known {
                hit(event_front, 0.5)
            } else {
                unknown
            },
            hit(event_front, 0.5),
        ),
        ray(
            "unknown alpha before known hit",
            51.5,
            1.,
            -2.,
            if known {
                (1, event_near + 1, 0.25, 0., 1)
            } else {
                unknown
            },
            if known {
                (1, event_near + 1, 0.25, 0., 1)
            } else {
                unknown
            },
        ),
        ray(
            "unknown alpha tied with known hit",
            55.5,
            1.,
            -2.,
            if known { hit(event_tie, 0.5) } else { unknown },
            if known { hit(event_tie, 0.5) } else { unknown },
        ),
        ray(
            "far unknown stored before closer known source",
            59.5,
            1.,
            -2.,
            if known {
                hit(event_reverse + 1, 0.5)
            } else {
                unknown
            },
            hit(event_reverse + 1, 0.5),
        ),
        ray(
            "unknown alpha tiny farther same rounded t",
            63.5,
            1.,
            -2.,
            if known {
                hit(event_tiny_far, 0.5)
            } else {
                unknown
            },
            hit(event_tiny_far, 0.5),
        ),
        ray(
            "unknown alpha tiny nearer same rounded t",
            67.5,
            1.,
            -2.,
            if known {
                (1, event_tiny_near + 1, 0.5, -2f32.powi(-41), 1)
            } else {
                unknown
            },
            if known {
                (1, event_tiny_near + 1, 0.5, -2f32.powi(-41), 1)
            } else {
                unknown
            },
        ),
        ray(
            "earlier side uncertainty dominates later alpha uncertainty",
            71.5,
            1.,
            -2.,
            if known {
                hit(event_priority + 1, 0.25)
            } else {
                unknown
            },
            unknown,
        ),
    ]);
    if let Some(first) = paired_hint_slots {
        let mut original = ray(
            "paired native hint low origin changes source ID",
            48.,
            36.,
            -36.,
            hit(first, 1. - 2f32.powi(-20)),
            hit(first, 1. - 2f32.powi(-20)),
        );
        original.words[1] = 0;
        original.words[4] = (-2f32.powi(-20)).to_bits();
        original.words[8] = (-48f32).to_bits();
        let mut high_only = original.clone();
        high_only.name = "paired native hint high only control source ID";
        high_only.words[4] = 0;
        high_only.geometric = hit(first + 1, 1. - 2f32.powi(-20));
        high_only.surface = high_only.geometric;
        cases.extend([original, high_only]);
    }
    if let Some(slot) = vertex_cache_fallback_slot {
        let mut case = ray(
            "wide exact vertex cache fallback keeps subnormal ray",
            80.,
            -0.5,
            0.,
            hit(slot, 0.5),
            hit(slot, 0.5),
        );
        case.words[4] = 1;
        case.words[12] = 0x80000001;
        cases.push(case);
    }
    cases.push(ray("farther transient ties discarded by nearer winner",83.5,1.,-2.,hit(transient_tie_slot+2,0.25),hit(transient_tie_slot+2,0.25)));
    let repeats = if deferred {
        std::env::var("VOXY_RAY_MATERIAL_SIDES_REPEATS")
            .ok()
            .map(|s| s.parse::<std::num::NonZeroU32>())
            .transpose()?
            .map_or(1, std::num::NonZeroU32::get)
    } else {
        1
    };
    assert!(repeats <= 128, "bounded scheduling correctness fixture");
    if repeats > 1 {
        let original = cases;
        cases = (0..repeats)
            .flat_map(|_| original.iter().cloned())
            .collect();
    }
    for (i, c) in cases.iter_mut().enumerate() {
        c.words[18] = 300 + i as u32;
        c.words[19] = 0x73696465;
    }
    let source = cases.iter().flat_map(|c| c.words).collect::<Vec<_>>();
    let source_bytes = bytemuck::cast_slice::<u32, u8>(&source);
    let bytes = source_bytes.len() as u64;
    let root = std::env::var_os("VOXY_RAY_MATERIAL_SIDES_OUTPUT").map(std::path::PathBuf::from);
    if let Some(root) = &root {
        std::fs::create_dir_all(root)?;
        std::fs::write(root.join("rays.bin"), source_bytes)?;
        let geometry=meshes.iter().map(|m|serde_json::json!({"positions":m.vertices.iter().map(|v|v.position.map(f32::to_bits)).collect::<Vec<_>>(),"world":m.instances[0].world.map(|v|v.map(f32::to_bits)),"indices":m.indices,"material":m.material})).collect::<Vec<_>>();
        let source_materials=materials.iter().map(|m|serde_json::json!({"words":bytemuck::bytes_of(&m.parameters).chunks_exact(4).map(|v|u32::from_le_bytes(v.try_into().unwrap())).collect::<Vec<_>>(),"image":m.images[0],"double_sided":m.double_sided})).collect::<Vec<_>>();
        std::fs::write(
            root.join("fixture-source.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"geometry":geometry,"materials":source_materials,"surface_policy":"first-material-event-exact","unknown_events_behind_known_nearer_hit_are_irrelevant":true}),
            )?,
        )?;
    }
    assert!(matches!(
        WorldRayQueryJob::with_options(
            device,
            scene.clone(),
            &materials,
            1,
            1,
            ExactWorldOptions {
                query_kind: WorldRayQueryKind::MaterialSurface,
                defer_nearest: true,
                certify_visibility: false,
                ..Default::default()
            }
        ),
        Err(RaySceneError::Unsupported)
    ));
    for options in [
        ExactWorldOptions {
            compare_wide_same_producer: true, wide_source_nodes: true,
            previous_hit_seed: true, defer_nearest: true, certify_visibility: true,
            ..Default::default()
        },
        ExactWorldOptions {
            compare_wide_same_producer: true, defer_nearest: true, certify_visibility: true,
            ..Default::default()
        },
        ExactWorldOptions {
            compare_wide_same_producer: true, wide_source_nodes: true,
            traversal_work_stats: true, defer_nearest: true, certify_visibility: true,
            ..Default::default()
        },
        ExactWorldOptions {
            compare_wide_same_producer: true, wide_source_nodes: true,
            query_kind: WorldRayQueryKind::HistoryAdmission, guarded_input: true,
            defer_nearest: true, certify_visibility: true, ..Default::default()
        },
        ExactWorldOptions {
            geometry_bounds_check: true,
            ..Default::default()
        },
        ExactWorldOptions {
            geometry_bounds_cache: true,
            certify_visibility: true,
            ..Default::default()
        },
        ExactWorldOptions {
            geometry_bounds_cache: true,
            defer_nearest: true,
            ..Default::default()
        },
        ExactWorldOptions {
            geometry_bounds_cache: true,
            defer_nearest: true,
            certify_visibility: true,
            query_kind: WorldRayQueryKind::HistoryAdmission,
            ..Default::default()
        },
        ExactWorldOptions {
            geometry_bounds_cache: true,
            defer_nearest: true,
            certify_visibility: true,
            query_kind: WorldRayQueryKind::OpaqueVisibility,
            ..Default::default()
        },
        ExactWorldOptions {
            geometry_bounds_cache: true,
            defer_nearest: true,
            certify_visibility: true,
            native_source_witness: true,
            ..Default::default()
        },
        ExactWorldOptions { wide_source_near_first: true, ..Default::default() },
    ] {
        assert!(matches!(
            WorldRayQueryJob::with_options(device, scene.clone(), &materials, 1, 1, options),
            Err(RaySceneError::Unsupported)
        ));
    }
    let mut reports = Vec::new();
    let mut native_used = 0u32;
    let mut previous_used = 0u32;
    for certify_order in [false, true] {
        for kind in [
            WorldRayQueryKind::Nearest,
            WorldRayQueryKind::MaterialSurface,
        ] {
            println!(
                "MATERIAL SIDES prepare policy={kind:?} deferred={deferred} native={native} separate={separate} previous={previous} known={known} order={certify_order}"
            );
            let job = WorldRayQueryJob::with_options(
                device,
                scene.clone(),
                &materials,
                1,
                cases.len() as u32,
                ExactWorldOptions {
                    query_kind: kind,
                    hit_position,
                    certified_ratio_seed: std::env::var("VOXY_RAY_MATERIAL_SIDES_RATIO_SEED")
                        .as_deref()
                        == Ok("1"),
                    exact_vertex_cache: std::env::var("VOXY_RAY_MATERIAL_SIDES_EXACT_VERTEX_CACHE")
                        .as_deref()
                        == Ok("1"),
                    node_reciprocal: std::env::var("VOXY_RAY_MATERIAL_SIDES_NODE_RECIPROCAL").as_deref() == Ok("1"),
                    wide_source_nodes: std::env::var("VOXY_RAY_MATERIAL_SIDES_WIDE_NODES").as_deref() == Ok("1"),
            wide_source_near_first: std::env::var("VOXY_RAY_MATERIAL_SIDES_WIDE_NEAR_FIRST").as_deref() == Ok("1"),
                    compare_wide_same_producer: std::env::var("VOXY_RAY_MATERIAL_SIDES_COMPARE_WIDE").as_deref() == Ok("1"),
                    skip_zero_prefix_multiply: std::env::var(
                        "VOXY_RAY_MATERIAL_SIDES_SKIP_ZERO_PREFIX_MULTIPLY",
                    )
                    .as_deref()
                        == Ok("1"),
                    cached_relative_interval: std::env::var(
                        "VOXY_RAY_MATERIAL_SIDES_CACHED_RELATIVE_INTERVAL",
                    )
                    .as_deref()
                        == Ok("1"),
                    fused_source_affine: std::env::var(
                        "VOXY_RAY_MATERIAL_SIDES_FUSED_SOURCE_AFFINE",
                    )
                    .as_deref()
                        == Ok("1"),
                    exact_winner_arena_slots: std::env::var("VOXY_RAY_MATERIAL_SIDES_WINNER_SLOTS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .and_then(std::num::NonZeroU32::new),
                    exact_winner_worker_limit: std::env::var(
                        "VOXY_RAY_MATERIAL_SIDES_WINNER_WORKERS",
                    )
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .and_then(std::num::NonZeroU32::new),
                    geometry_bounds_cache: deferred
                        && std::env::var("VOXY_RAY_MATERIAL_SIDES_GEOMETRY_BOUNDS").as_deref()
                            == Ok("1"),
                    geometry_bounds_check: deferred
                        && std::env::var("VOXY_RAY_MATERIAL_SIDES_GEOMETRY_BOUNDS").as_deref()
                            == Ok("1"),
                    hit_uv,
                    exact_worker_limit: if deferred { exact_workers } else { None },
                    exact_group_size: if deferred { exact_group } else { None },
                    guarded_input: true,
                    certify_order,
                    certify_visibility: deferred,
                    defer_nearest: deferred,
                    native_seed: native,
                    separate_native_seed: separate,
                    native_triangle_hint: std::env::var(
                        "VOXY_RAY_MATERIAL_SIDES_NATIVE_TRIANGLE_HINT",
                    )
                    .as_deref()
                        == Ok("1"),
                    previous_hit_seed: previous,
                    ..Default::default()
                },
            )?;
            assert_eq!(job.query_kind(), kind);
            let wide_requested = std::env::var("VOXY_RAY_MATERIAL_SIDES_WIDE_NODES").as_deref() == Ok("1");
            if wide_requested && deferred { assert!(job.wide_source_nodes_enabled(), "small source fixture must actually admit BVH8"); }
            if wide_requested {
                if let Some(root) = &root {
                    std::fs::write(root.join(format!("wide-source-{kind:?}-order-{certify_order}.json")),
                        serde_json::to_vec_pretty(&serde_json::json!({"requested":true,"actually_admitted":job.wide_source_nodes_enabled(),"node_reciprocal":job.node_reciprocal_enabled(),"scheduling_block_blake3":job.wide_source_digest(),"original_exact_binary_fallback_retained":true,"performance_qualified":false}))?)?;
                }
            }

            if std::env::var_os("VOXY_RAY_MATERIAL_SIDES_WINNER_SLOTS").is_some()
                || std::env::var_os("VOXY_RAY_MATERIAL_SIDES_WINNER_WORKERS").is_some()
            {
                let (slots, workers) = job
                    .exact_winner_workspace()
                    .ok_or("winner workspace not connected")?;
                if let Ok(requested) = std::env::var("VOXY_RAY_MATERIAL_SIDES_WINNER_SLOTS") {
                    assert_eq!(slots, requested.parse::<u32>()?);
                }
                if let Ok(requested) = std::env::var("VOXY_RAY_MATERIAL_SIDES_WINNER_WORKERS") {
                    assert!(workers <= requested.parse::<u32>()?);
                }
                if let Some(root) = &root {
                    std::fs::write(
                        root.join(format!(
                            "winner-workspace-{kind:?}-order-{certify_order}.json"
                        )),
                        serde_json::to_vec_pretty(
                            &serde_json::json!({"temporary_slots":slots,"worker_cap":workers,"full_fallback_slots":512,"full_fallback_workers":job.exact_worker_count(),"integer_precision_bits":2048,"same_scratch_allocation":true,"non_hit_or_exhausted_winner_retries_complete_source":true,"FPS_qualified":false}),
                        )?,
                    )?;
                }
                println!(
                    "WINNER WORKSPACE slots={slots} worker_cap={workers} full_slots=512 full_workers={}",
                    job.exact_worker_count()
                );
            }
            if job.geometry_bounds_buffer().is_some() {
                let not_ready = std::env::var("VOXY_RAY_MATERIAL_SIDES_GEOMETRY_NOT_READY")
                    .as_deref()
                    == Ok("1");
                let mut encoder = device.create_command_encoder(&Default::default());
                if !not_ready {
                    job.build_scene(&mut encoder);
                }
                let read = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("checked geometry bounds header"),
                    size: 64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_buffer_to_buffer(
                    job.geometry_bounds_buffer().unwrap(),
                    0,
                    &read,
                    0,
                    64,
                );
                let submission = queue.submit([encoder.finish()]);
                let (tx, rx) = std::sync::mpsc::channel();
                read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                device.poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })?;
                rx.recv()??;
                let bytes = read.slice(..).get_mapped_range()?.to_vec();
                read.unmap();
                let h: Vec<u32> = bytes
                    .chunks_exact(4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .collect();
                assert_eq!(
                    (h[0], h[4], h[5], h[6], h[7]),
                    if not_ready {
                        (0, 19, 0, 0, 1)
                    } else {
                        (1, 19, 0, h[2], 1)
                    }
                );
                println!(
                    "GEOMETRY BOUNDS ready={} checked_all={} mismatches={} bytes={}",
                    h[0],
                    h[6],
                    h[5],
                    job.geometry_bounds_bytes()
                );
            }
            exact_vertex_cache_audit::capture(
                device,
                queue,
                &job,
                &scene,
                root.as_deref(),
                kind,
                certify_order,
            )?;
            let primary_bytes = cases.len() as u64 * record_bytes;
            assert_eq!(u64::from(job.output_record_words()) * 4, record_bytes);
            for replay in 0..if previous { 2 } else { 1 } {
                let mut current_source = source.clone();
                if previous && replay == 1 {
                    current_source[..18].copy_from_slice(&cases[1].words[..18]);
                }
                let current_bytes = bytemuck::cast_slice::<u32, u8>(&current_source);
                queue.write_buffer(job.input(), 0, current_bytes);
                let read = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("material surface exact/deferred input/output readback"),
                    size: primary_bytes
                        + bytes
                        + if hit_position {
                            cases.len() as u64 * 32
                        } else {
                            0
                        },
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                job.encode(&mut encoder, cases.len() as u32)?;
                encoder.copy_buffer_to_buffer(job.output(), 0, &read, 0, primary_bytes);
                encoder.copy_buffer_to_buffer(job.input(), 0, &read, primary_bytes, bytes);
                if let Some(offset) = job.world_position_offset_bytes() {
                    assert_eq!(
                        offset % u64::from(device.limits().min_storage_buffer_offset_alignment),
                        0
                    );
                    encoder.copy_buffer_to_buffer(
                        job.output(),
                        offset,
                        &read,
                        primary_bytes + bytes,
                        cases.len() as u64 * 32,
                    );
                }
                let submission = queue.submit([encoder.finish()]);
                let (tx, rx) = std::sync::mpsc::channel();
                read.slice(..)
                    .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
                device.poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })?;
                rx.recv()??;
                let mapped = read.slice(..).get_mapped_range()?;
                let after = &mapped[primary_bytes as usize..(primary_bytes + bytes) as usize];
                for (i, (actual, original)) in after
                    .chunks_exact(4)
                    .zip(current_bytes.chunks_exact(4))
                    .enumerate()
                {
                    if !deferred || ![7, 11, 15].contains(&(i % 20)) {
                        assert_eq!(
                            actual, original,
                            "protected coefficient/admission/target/ID word {i}"
                        );
                    }
                }
                let mut routes = [0u32; 4];
                for (index, (c, row)) in cases
                    .iter()
                    .zip(mapped[..primary_bytes as usize].chunks_exact(record_bytes as usize))
                    .enumerate()
                {
                    let w = row
                        .chunks_exact(4)
                        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                        .collect::<Vec<_>>();
                    let expected_case = if previous && replay == 1 && index == 0 {
                        &cases[1]
                    } else {
                        c
                    };
                    let expected = if kind == WorldRayQueryKind::Nearest {
                        expected_case.geometric
                    } else {
                        expected_case.surface
                    };
                    assert_eq!(w[0], expected.0, "state {} {kind:?}: {w:?}", c.name);
                    assert_eq!(w[6], expected.1, "source identity {} {kind:?}", c.name);
                    assert_eq!([w[4], w[5]], [c.words[18], c.words[19]]);
                    if c.name == "farther transient ties discarded by nearer winner" {assert_eq!(w[19],0,"superseded farther ties leaked into final nearest result");}
                    if expected.0 == 1 {
                        assert_eq!(w[7], 0);
                        assert_eq!(
                            [w[8], w[9]],
                            [expected.2.to_bits(), expected.3.to_bits()],
                            "t hi/lo {} {kind:?}",
                            c.name
                        );
                        assert_eq!(w[14], 1);
                        if !c.name.starts_with("rational ") {
                            assert_eq!(w[15], 0);
                        } else {
                            assert_ne!(w[15] & 1, 0);
                        }
                        assert_eq!((w[18] >> 2) & 3, expected.4);
                    } else {
                        assert_eq!(&w[8..16], &[0; 8], "unusable parameter leaked {}", c.name);
                    }
                    if hit_uv {
                        assert_eq!(&w[20..24], &[0; 4]);
                        assert_eq!(&w[24..28], &[if expected.0 == 1 { 3 } else { 0 }, 0, 0, 0]);
                    }
                    if c.name == "invalid producer admission stays unknown" {
                        assert_eq!(w[1], 512);
                    }
                    for (i, bit) in [2048u32, 4096, 8192, 16384].into_iter().enumerate() {
                        routes[i] += u32::from(w[18] & bit != 0);
                    }
                    native_used += u32::from(w[18] & 16384 != 0);
                    previous_used += u32::from(w[18] & 32768 != 0);
                    if deferred && c.name.starts_with("tiny ") {
                        assert_ne!(
                            w[18] & 8192,
                            0,
                            "uncertain orientation must use exact candidates"
                        );
                    }
                    if previous
                        && replay == 1
                        && kind == WorldRayQueryKind::MaterialSurface
                        && index == 0
                    {
                        assert_eq!(w[18] & 32768, 0, "old back face hint was accepted");
                        assert_ne!(
                            w[18] & 4096,
                            0,
                            "fresh front winner did not use certified dispatch"
                        );
                    }
                    if deferred && c.name == "candidate list overflow retries complete source" {
                        assert_eq!(
                            w[18] & (4096 | 8192),
                            0,
                            "overflow must run complete exact source"
                        );
                    }
                }
                if deferred {
                    assert!(routes[0] > 0, "no certified miss exercised");
                    assert!(
                        routes[1] + routes[2] > 0,
                        "no optimized exact dispatch exercised"
                    );
                }
                let policy = if kind == WorldRayQueryKind::Nearest {
                    "geometric"
                } else {
                    "material"
                };
                if let Some(root) = &root {
                    if hit_position {
                        std::fs::write(
                            root.join(format!(
                                "{policy}-order{certify_order}-replay{replay}-position.bin"
                            )),
                            &mapped[(primary_bytes + bytes) as usize..],
                        )?;
                    }
                    std::fs::write(
                        root.join(format!(
                            "{policy}-order{certify_order}-replay{replay}-input-before.bin"
                        )),
                        current_bytes,
                    )?;
                    std::fs::write(
                        root.join(format!("{policy}-order{certify_order}-replay{replay}.bin")),
                        &mapped[..primary_bytes as usize],
                    )?;
                    std::fs::write(
                        root.join(format!(
                            "{policy}-order{certify_order}-replay{replay}-input-after.bin"
                        )),
                        after,
                    )?;
                }
                if hit_position
                    && std::env::var("VOXY_RAY_MATERIAL_SIDES_PARTIAL").as_deref() == Ok("1")
                {
                    // A short encode must clear an old hit only within its bound
                    // input range, preserving every following output/tail record.
                    let mut inactive = current_source[..20].to_vec();
                    inactive[3] = 0;
                    queue.write_buffer(job.input(), 0, bytemuck::cast_slice(&inactive));
                    let size = primary_bytes + cases.len() as u64 * 32;
                    let partial = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("partial query stale-position check"),
                        size,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    });
                    let mut encoder = device.create_command_encoder(&Default::default());
                    job.encode(&mut encoder, 1)?;
                    encoder.copy_buffer_to_buffer(job.output(), 0, &partial, 0, primary_bytes);
                    encoder.copy_buffer_to_buffer(
                        job.output(),
                        job.world_position_offset_bytes().unwrap(),
                        &partial,
                        primary_bytes,
                        cases.len() as u64 * 32,
                    );
                    let submission = queue.submit([encoder.finish()]);
                    let (tx, rx) = std::sync::mpsc::channel();
                    partial
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
                    device.poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })?;
                    rx.recv()??;
                    let result = partial.slice(..).get_mapped_range()?;
                    assert_eq!(u32::from_le_bytes(result[..4].try_into().unwrap()), 3);
                    assert_eq!(
                        &result[primary_bytes as usize..primary_bytes as usize + 32],
                        &[0; 32]
                    );
                    assert_eq!(
                        &result[record_bytes as usize..primary_bytes as usize],
                        &mapped[record_bytes as usize..primary_bytes as usize],
                        "short encode changed unrequested primary records"
                    );
                    assert_eq!(
                        &result[primary_bytes as usize + 32..],
                        &mapped[(primary_bytes + bytes) as usize + 32..],
                        "short encode changed unrequested position records"
                    );
                    if let Some(root) = &root {
                        std::fs::write(
                            root.join(format!(
                                "{policy}-order{certify_order}-replay{replay}-partial.bin"
                            )),
                            &result,
                        )?;
                    }
                    println!("MATERIAL SIDES partial hit-to-inactive position clear passed");
                }
                reports.push(serde_json::json!({"policy":policy,"certify_order":certify_order,"replay":replay,"rays":cases.len(),"routes_miss_winner_candidates_native":routes,"all_expected_states_ids_t_pairs_and_kinds_passed":true,"all_protected_input_words_unchanged":true,"all_input_bytes_unchanged":!deferred,"queue_owns_only_padding_words":[7,11,15]}));
                println!("MATERIAL SIDES passed policy={policy} replay={replay} routes={routes:?}");
                if let Some(proof) = job.read_same_producer_comparison(queue)? {
                    if let Some(root) = &root { std::fs::write(root.join(format!("same-producer-{policy}-order{certify_order}-replay{replay}.json")),serde_json::to_vec_pretty(&proof)?)?; }
                    assert_eq!(proof["all_protected_words_equal"], true, "same-producer comparison failed: {proof}");
                    let partial = hit_position && std::env::var("VOXY_RAY_MATERIAL_SIDES_PARTIAL").as_deref() == Ok("1");
                    assert_eq!(proof["compared_records"], cases.len() as u64 + u64::from(partial), "all requested full and partial records must be compared");
                }

            }
        }
    }
    if native && known {
        assert!(
            native_used > 0,
            "native hint was not source-certified and consumed"
        );
    }
    if previous && known {
        assert!(
            previous_used > 0,
            "previous hit was not re-certified and consumed"
        );
    }
    let report = serde_json::json!({"schema":"voxy-material-surface-nearest-v2","surface_policy":"first-material-event-exact","fixture_repeats":repeats,"exact_workers_requested":exact_workers.map(std::num::NonZeroU32::get),"exact_group_requested":exact_group.map(std::num::NonZeroU32::get),"hit_uv":hit_uv,"output_record_words":record_bytes/4,"partial_position_check":std::env::var("VOXY_RAY_MATERIAL_SIDES_PARTIAL").as_deref()==Ok("1"),"hit_position":hit_position,"deferred":deferred,"native":native,"separate_native":separate,"previous":previous,"known_material_table":known,"native_hints_consumed":native_used,"previous_hits_consumed":previous_used,"previous_ray_changed":previous,"cases":cases.iter().map(|c|c.name).collect::<Vec<_>>(),"runs":reports,"uncertified_deferred_combination_rejected":true,"render_consumer_connected":false,"fps_qualified":false});
    if let Some(root) = &root {
        std::fs::write(root.join("proof.json"), serde_json::to_vec_pretty(&report)?)?;
    }
    println!("MATERIAL SIDES {report}");
    Ok(())
}
