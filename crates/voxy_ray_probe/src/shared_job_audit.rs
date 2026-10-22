//! Actual GPU producer -> renderer exact job -> GPU consumer, one submission.
use glam::{Mat4, Vec3};
use std::sync::Arc;
use voxy_render::{ExactWorldNearest, RayScene, archive_scene::*};
fn audit_one(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &Arc<RayScene>,
    materials: &[ArchiveMaterialSource],
    expected: &[(u32, u32, u32, u32, u32, u32)],
    certify_order: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let image_count = materials
        .iter()
        .flat_map(|m| m.images.iter().flatten())
        .max()
        .map_or(0, |&i| i + 1);
    let defer_nearest = std::env::var("VOXY_RAY_SHARED_JOB_DEFERRED").as_deref() == Ok("1");
    let job = ExactWorldNearest::with_options(
        device,
        scene.clone(),
        materials,
        image_count,
        8,
        voxy_render::ExactWorldOptions {
            certify_order,
            certify_visibility: defer_nearest,
            defer_nearest,
            exact_worker_limit: std::env::var("VOXY_RAY_SHARED_WORKERS")
                .ok()
                .map(|s| s.parse::<std::num::NonZeroU32>())
                .transpose()?,
            exact_group_size: std::env::var("VOXY_RAY_SHARED_GROUP")
                .ok()
                .map(|s| s.parse::<std::num::NonZeroU32>())
                .transpose()?,
            previous_hit_seed: defer_nearest
                && std::env::var("VOXY_RAY_SHARED_PREVIOUS_SEED").as_deref() == Ok("1"),
            native_seed: defer_nearest
                && std::env::var("VOXY_RAY_SHARED_NATIVE_SEED").as_deref() == Ok("1"),
            ..Default::default()
        },
    )?;
    let producer=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("independent GPU ray producer"),source:wgpu::ShaderSource::Wgsl(r"
@group(0) @binding(0) var<storage,read_write> rays:array<u32>;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;let base=i*20u;for(var k=0u;k<20u;k++){rays[base+k]=0u;}
 var x=-0.5;var y=-0.5;var z=1.;var dz=-2.;var low=0.;
 if i==1u {x=4.;y=4.;}if i==2u{x=100000000.;low=-0.5;}
 if i==3u{z=2.;}if i==4u{z=-0.5;dz=-1.;}if i==5u{z=0.;}
 rays[base]=bitcast<u32>(x);rays[base+1u]=bitcast<u32>(y);rays[base+2u]=bitcast<u32>(z);rays[base+4u]=bitcast<u32>(low);rays[base+10u]=bitcast<u32>(dz);rays[base+18u]=100u+i;
}".into())});
    let pp = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &producer,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let pg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pp.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: job.input().as_entire_binding(),
        }],
    });
    let count = expected.len() as u32;
    let consumed = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU consumer output"),
        size: u64::from(count) * 80,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let consumer=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("independent GPU intersection consumer"),source:wgpu::ShaderSource::Wgsl(r"
@group(0) @binding(0) var<storage,read> intersections:array<u32>;
@group(0) @binding(1) var<storage,read_write> consumed:array<u32>;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>){for(var k=0u;k<20u;k++){consumed[id.x*20u+k]=intersections[id.x*20u+k];}}
".into())});
    let cp = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &consumer,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let cg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &cp.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: job.output().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: consumed.as_entire_binding(),
            },
        ],
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(count) * 80,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.build(&mut encoder);
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pp);
        pass.set_bind_group(0, &pg, &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
    assert!(job.encode(&mut encoder, 9).is_err());
    job.encode(&mut encoder, count)?;
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&cp);
        pass.set_bind_group(0, &cg, &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&consumed, 0, &read, 0, u64::from(count) * 80);
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
    let mapped = read.slice(..).get_mapped_range()?;
    for (index, (row, &(state, slot, t, u, v, kind))) in
        mapped.chunks_exact(80).zip(expected).enumerate()
    {
        let w: Vec<u32> = row
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        assert_eq!(w[0], state, "status ray {index}: {w:?}");
        assert_eq!(w[4], 100 + index as u32);
        assert_eq!(w[6], slot, "owner ID ray {index}");
        if state == 2 {
            assert_eq!(w[1], 256, "unresolved source alpha reason");
            assert_ne!(w[18] & 16, 0);
        }
        if state == 1 {
            assert_eq!(w[7], 0);
            assert_eq!(
                [w[8], w[9], w[10], w[11], w[12], w[13]],
                [t, 0, u, 0, v, 0],
                "exact parameters ray {index}"
            );
            assert_eq!(w[14], 1);
            assert_eq!(w[15], 0);
            assert_eq!((w[18] >> 2) & 3, kind);
        }
    }
    drop(mapped);
    read.unmap();
    Ok(())
}
fn audit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &Arc<RayScene>,
    materials: &[ArchiveMaterialSource],
    expected: &[(u32, u32, u32, u32, u32, u32)],
) -> Result<(), Box<dyn std::error::Error>> {
    for certify_order in [false, true] {
        audit_one(device, queue, scene, materials, expected, certify_order)?;
    }
    Ok(())
}
pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let mesh = ArchiveMeshSource {
        vertices: [[-1., -1., 0.], [1., -1., 0.], [-1., 1., 0.]]
            .map(|position| ArchiveVertex {
                position,
                normal: [0., 0., 1.],
                tangent: [1., 0., 0., 1.],
                uv: [0.; 2],
            })
            .to_vec(),
        indices: vec![0, 1, 2],
        instances: [
            Mat4::IDENTITY,
            Mat4::from_translation(Vec3::new(100000000., 0., 0.)),
            Mat4::from_translation(Vec3::new(0., 0., -1.)),
        ]
        .map(ArchiveInstance::new)
        .to_vec(),
        material: 0,
        center: Vec3::ZERO,
    };
    let materials = vec![ArchiveMaterialSource {
        parameters: ArchiveMaterial::metallic_roughness([1.; 4], 0., 0.5, 0., -1.),
        images: [None; 4],
        double_sided: true,
    }];
    let mut scene = Arc::new(RayScene::from_archive_candidates(device, &[mesh], 0.)?);
    let half = 0.5_f32.to_bits();
    let quarter = 0.25_f32.to_bits();
    audit(
        device,
        queue,
        &scene,
        &materials,
        &[
            (1, 0, half, quarter, quarter, 0),
            (0, u32::MAX, 0, 0, 0, 0),
            (1, 1, half, quarter, quarter, 0),
            (1, 0, 1_f32.to_bits(), quarter, quarter, 0),
            (1, 2, half, quarter, quarter, 0),
            (1, 0, 0, quarter, quarter, 0),
        ],
    )?;
    // Material admission remains coupled to the actual source parameter bits.
    let mut material_cases = materials;
    material_cases[0].parameters.diffuse[3] = 0.;
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(1, 0, half, quarter, quarter, 0)],
    )?; // Opaque alpha zero is still geometry.
    material_cases[0].parameters.surface[3] = -2.;
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(0, u32::MAX, 0, 0, 0, 0)],
    )?;
    material_cases[0].parameters.surface[2] = 0.92;
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(1, 0, half, quarter, quarter, 2)],
    )?;
    material_cases[0].parameters.diffuse[3] = 1.;
    material_cases[0].images[0] = Some(0);
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(2, u32::MAX, 0, 0, 0, 0)],
    )?;
    material_cases[0].parameters.workflow[1] = 2.;
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(1, 0, half, quarter, quarter, 2)],
    )?;
    material_cases[0].images = [None; 4];
    material_cases[0].parameters.surface[2] = 0.;
    material_cases[0].parameters.surface[3] = 0.5;
    material_cases[0].parameters.diffuse[3] = 0.5;
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(1, 0, half, quarter, quarter, 1)],
    )?;
    material_cases[0].parameters.diffuse[3] = f32::from_bits(0.5_f32.to_bits() - 1);
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(0, u32::MAX, 0, 0, 0, 0)],
    )?;
    material_cases[0].parameters = ArchiveMaterial::metallic_roughness([1.; 4], 0., 0.5, 0., -1.);
    Arc::get_mut(&mut scene)
        .ok_or("live query retained mutable scene")?
        .remove_instance(0)?;
    audit(
        device,
        queue,
        &scene,
        &material_cases,
        &[(1, 2, 1_f32.to_bits(), quarter, quarter, 0)],
    )?;
    println!(
        "SHARED EXACT JOB PASS GPU_producer_consumer=true same_submission=true low_origin_bits=true closed_endpoints=true removed_instance=true exact_pairs=true source_owner=true material_source_bits=true unresolved_alpha=true cutoff_one_ulp=true order_certificate_off_on=true fps_qualified=false"
    );
    Ok(())
}
