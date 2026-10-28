use super::*;
use crate::{SkinnedMorphTarget, SkinnedVertex};

fn mesh() -> Arc<SkinnedMesh> {
    Arc::new(
        SkinnedMesh::new(
            [[1e30, 0., 0.], [1e30, 1., 0.], [1e30, 0., 1.]]
                .map(|position| SkinnedVertex {
                    position,
                    normal: [0., 0., 1.],
                    uv: [0.25, 0.75],
                    joints: [0; 4],
                    weights: [65535, 0, 0, 0],
                })
                .to_vec(),
            vec![0, 1, 2],
            1,
        )
        .unwrap(),
    )
}
fn morph() -> Arc<SkinnedMorphMesh> {
    Arc::new(
        SkinnedMorphMesh::new(
            mesh(),
            vec![
                SkinnedMorphTarget {
                    positions: Some(vec![[1., 0., 0.]; 3]),
                    normals: Some(vec![[1., 0., 0.]; 3]),
                },
                SkinnedMorphTarget {
                    positions: Some(vec![[-1e30, 0., 0.]; 3]),
                    normals: None,
                },
            ],
            9,
        )
        .unwrap(),
    )
}

#[test]
fn exact_morph_shader_parses_and_validates() {
    let source = morph_shader_source();
    assert!(source.contains("morph_component(i,0u)"));
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
fn morph_source_instance_admission_and_preflight_preserve_existing_owners() {
    let (device, queue) = wgpu::Device::noop(&Default::default());
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let budget = crate::ComputeMemoryBudget::configure(&device, 65536).unwrap();
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let before = budget.stats();
    assert!(skinner.upload_morph_source(morph(), 0, 335).is_err());
    assert_eq!(budget.stats(), before);
    let source = skinner.upload_morph_source(morph(), 0, 65536).unwrap();
    assert_eq!(source.allocation_bytes(), 336);
    let joints = [Mat4::IDENTITY];
    let weights = [1., 1.];
    let pinned = budget.stats();
    assert!(
        skinner
            .create_instance(&renderer, source.clone(), &joints, [1.; 4], 336, 65536)
            .is_err()
    );
    assert!(
        skinner
            .create_morph_instance(
                &renderer,
                source.clone(),
                &joints,
                &weights,
                [1.; 4],
                336,
                336
            )
            .is_err()
    );
    assert_eq!(budget.stats(), pinned);
    let instance = skinner
        .create_morph_instance(
            &renderer,
            source.clone(),
            &joints,
            &weights,
            [1.; 4],
            336,
            65536,
        )
        .unwrap();
    let full = budget.stats();
    assert!(skinner.prepare_pose(&instance, &joints).is_err());
    assert!(
        skinner
            .prepare_morph_pose(&instance, &joints, &[f32::NAN, 1.])
            .is_err()
    );
    assert!(
        skinner
            .prepare_morph_pose(&instance, &joints, &[1.])
            .is_err()
    );
    assert!(
        skinner
            .prepare_morph_pose(&instance, &joints, &[1., -f32::MAX])
            .is_err()
    );
    assert_eq!(budget.stats(), full);
    let prepared = skinner
        .prepare_morph_pose(&instance, &joints, &weights)
        .unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    skinner
        .encode_prepared_pose(&queue, &mut encoder, &prepared)
        .unwrap();
    assert!(
        skinner
            .encode_prepared_poses_batch(
                &queue,
                &mut encoder,
                &[
                    prepared,
                    skinner
                        .prepare_morph_pose(&instance, &joints, &weights)
                        .unwrap()
                ]
            )
            .is_err()
    );
    let other = SceneSkinner::new(&renderer).unwrap();
    assert!(
        other
            .prepare_morph_pose(&instance, &joints, &weights)
            .is_err()
    );
    queue.submit([encoder.finish()]);
    drop((instance, source));
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
}

#[test]
#[ignore = "requires physical GPU; exact morph output readback, not a benchmark"]
fn gpu_exact_morph_precedes_skinning_and_preserves_independent_instance_weights() {
    let graphics = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(graphics.request_adapter(&Default::default())).unwrap();
    eprintln!("morph correctness adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let data = morph();
    let source = skinner
        .upload_morph_source(data.clone(), 0, 1 << 20)
        .unwrap();
    let joints = [Mat4::IDENTITY];
    let first = skinner
        .create_morph_instance(
            &renderer,
            source.clone(),
            &joints,
            &[1., 1.],
            [1.; 4],
            source.allocation_bytes(),
            1 << 20,
        )
        .unwrap();
    let second = skinner
        .create_morph_instance(
            &renderer,
            source.clone(),
            &joints,
            &[-0.5, 1.],
            [1.; 4],
            source.allocation_bytes() + first.allocation_bytes(),
            1 << 20,
        )
        .unwrap();
    for values in [[1., 1.], [1.5, 1.], [-0.5, 1.], [0., 1.]] {
        let second_weights = [-0.5, 1.];
        let prepared = [
            skinner
                .prepare_morph_pose(&first, &joints, &values)
                .unwrap(),
            skinner
                .prepare_morph_pose(&second, &joints, &second_weights)
                .unwrap(),
        ];
        let mut encoder = device.create_command_encoder(&Default::default());
        skinner
            .encode_prepared_poses_batch(&queue, &mut encoder, &prepared)
            .unwrap();
        let size = 3 * (9 + 3) * 4;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("morph position readback"),
            size: size * 2,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&first.geometry.vertices, 0, &staging, 0, 108);
        encoder.copy_buffer_to_buffer(&first.geometry.normals, 0, &staging, 108, 36);
        encoder.copy_buffer_to_buffer(&second.geometry.vertices, 0, &staging, size, 108);
        encoder.copy_buffer_to_buffer(&second.geometry.normals, 0, &staging, size + 108, 36);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = staging.slice(..).get_mapped_range().unwrap();
        let actual: &[f32] = bytemuck::cast_slice(&bytes);
        for (instance_weights, output) in
            [values, second_weights].iter().zip(actual.chunks_exact(36))
        {
            let expected = data
                .evaluate(instance_weights)
                .unwrap()
                .posed_scene_mesh(&joints, Mat4::IDENTITY, [1.; 4])
                .unwrap();
            for (vertex, expected) in output[..27].chunks_exact(9).zip(expected.vertices()) {
                assert_eq!(
                    vertex[..3].iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    expected.position.map(f32::to_bits)
                );
                assert_eq!(&vertex[3..5], &expected.uv);
            }
            for (normal, expected) in output[27..]
                .chunks_exact(3)
                .zip(expected.authored_normals().unwrap())
            {
                assert!(
                    normal
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| (*a - *b).abs() < 1e-6)
                );
            }
        }
        drop(bytes);
        staging.unmap();
        assert!(
            skinner
                .prepare_morph_pose(&first, &joints, &[f32::NAN, 1.])
                .is_err()
        );
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires physical GPU; exact morph integer arithmetic readback and rational oracle payload"]
fn gpu_exact_morph_component_edges_and_full_target_budget() {
    use wgpu::util::DeviceExt;
    let graphics = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(graphics.request_adapter(&Default::default())).unwrap();
    eprintln!("exact morph arithmetic adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut weights = vec![1f32; 256];
    weights[..9].copy_from_slice(&[
        1.,
        1.5,
        -0.5,
        2f32.powi(-100),
        2f32.powi(100),
        f32::from_bits(1),
        2f32.powi(-24),
        -1.,
        -2f32.powi(100),
    ]);
    let mut cases = Vec::<(f32, Vec<f32>)>::new();
    let mut add = |base, terms: &[(usize, f32)]| {
        let mut deltas = vec![0f32; 256];
        for &(index, value) in terms {
            deltas[index] = value;
        }
        cases.push((base, deltas));
    };
    add(1e30, &[(0, 1.), (7, 1e30)]);
    add(1., &[(6, 1.)]);
    add(1., &[(6, 1.), (5, 2f32.powi(100))]);
    add(0., &[(0, f32::from_bits(1))]);
    add(0., &[(2, f32::from_bits(1))]);
    add(0., &[(1, f32::from_bits(1))]);
    add(f32::MAX, &[(7, f32::MAX)]);
    add(1., &[(4, f32::MAX), (8, f32::MAX)]);
    cases.push((0., vec![1.; 256]));
    let mut seed = 0x97bd_0291_u32;
    while cases.len() < 41 {
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let mut bits = seed;
            if (bits >> 23) & 255 == 255 {
                bits ^= 1 << 23;
            }
            f32::from_bits(bits)
        };
        let base = next();
        let mut deltas = vec![0f32; 256];
        for value in &mut deltas[..9] {
            *value = next();
        }
        if crate::exact_morph_component(base, deltas.iter().copied().zip(weights.iter().copied()))
            .is_some()
        {
            cases.push((base, deltas));
        }
    }
    let count = cases.len();
    let mut source = vec![0f32; count * 16];
    for (index, (base, _)) in cases.iter().enumerate() {
        source[index * 16] = *base;
    }
    let mut deltas = vec![0f32; count * 256 * 6];
    for shape in 0..256 {
        for (index, (_, values)) in cases.iter().enumerate() {
            deltas[(shape * count + index) * 6] = values[shape];
        }
    }
    let terms = include_str!("../world_exact_fused_affine.wgsl")
        .split("fn affine_fused_axis")
        .next()
        .unwrap();
    let shader_source = format!(
        "{terms}\n{}\n{}",
        include_str!("morph.wgsl"),
        r"
struct Parameters {color:vec4<f32>,count:u32,pad0:u32,pad1:u32,pad2:u32}
@group(0) @binding(0) var<storage,read> source:array<f32>;
@group(0) @binding(4) var<uniform> parameters:Parameters;
@group(0) @binding(7) var<storage,read_write> output:array<u32>;
@compute @workgroup_size(64) fn validate_morph(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x<parameters.count {output[id.x]=bitcast<u32>(morph_component(id.x,0u));}
}"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("exact morph arithmetic oracle"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("validate_morph"),
        compilation_options: Default::default(),
        cache: None,
    });
    let storage = |values: &[f32]| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(values),
            usage: wgpu::BufferUsages::STORAGE,
        })
    };
    let source_buffer = storage(&source);
    let delta_buffer = storage(&deltas);
    let weights_buffer = storage(&weights);
    let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&[0u32, 0, 0, 0, count as u32, 256, 0, 0]),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: count as u64 * 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: count as u64 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: source_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: delta_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: weights_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups((count as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, count as u64 * 4);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = staging.slice(..).get_mapped_range().unwrap();
    let actual: &[u32] = bytemuck::cast_slice(&mapped);
    let mut records = Vec::new();
    for ((base, deltas), &actual) in cases.iter().zip(actual) {
        let expected = crate::exact_morph_component(
            *base,
            deltas.iter().copied().zip(weights.iter().copied()),
        )
        .unwrap()
        .to_bits();
        assert_eq!(actual, expected, "base bits {:08x}", base.to_bits());
        records.push(serde_json::json!({"base_bits":base.to_bits(),"terms":deltas.iter().zip(&weights).filter(|(d,_)|d.to_bits()&0x7fff_ffff!=0).map(|(d,w)|[d.to_bits(),w.to_bits()]).collect::<Vec<_>>(),"gpu_bits":actual,"cpu_bits":expected,"cpu64_bits":crate::exact_morph_component64(*base,deltas.iter().copied().zip(weights.iter().copied())).unwrap().to_bits()}));
    }
    println!(
        "MORPH_EXACT_CASES {}",
        serde_json::to_string(&records).unwrap()
    );
    drop(mapped);
    staging.unmap();
    assert!(pollster::block_on(scope.pop()).is_none());
}
