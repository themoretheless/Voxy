//! Procedural broad-phase contract: candidates may include false positives.
use wgpu::util::DeviceExt;
const SOURCE: &str = r#"
enable wgpu_ray_query;
struct Input {origin:vec4<f32>,direction:vec4<f32>}
@group(0) @binding(0) var scene:acceleration_structure;
@group(0) @binding(1) var<storage,read> inputs:array<Input>;
@group(0) @binding(2) var<storage,read_write> outputs:array<vec4<u32>>;
@compute @workgroup_size(32) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs) {return;}let input=inputs[id.x];var query:ray_query;
 rayQueryInitialize(&query,scene,RayDesc(0u,255u,0.,100.,input.origin.xyz,input.direction.xyz));
 var mask=0u;var count=0u;var stable_mask=0u;var unknown=0u;
 while rayQueryProceed(&query) {
  let hit=rayQueryGetCandidateIntersection(&query);
  if hit.kind==3u {let bit=hit.instance_index*2u+hit.primitive_index;if bit<32u {mask|=1u<<bit;}count+=1u;if hit.instance_custom_data==0u||hit.instance_custom_data==7u {let stable=select(0u,1u,hit.instance_custom_data==7u)*2u+hit.primitive_index;if stable<32u {stable_mask|=1u<<stable;}}else{unknown+=1u;}}
 }
 outputs[id.x]=vec4<u32>(mask,count,stable_mask,unknown);
}
"#;
fn intersects(b: [f32; 6], origin: [f32; 3], direction: [f32; 3]) -> bool {
    let (mut lo, mut hi) = (0f64, 100f64);
    for axis in 0..3 {
        let (o, d) = (f64::from(origin[axis]), f64::from(direction[axis]));
        if d == 0. {
            if o < f64::from(b[axis]) || o > f64::from(b[axis + 3]) {
                return false;
            }
        } else {
            let a = (f64::from(b[axis]) - o) / d;
            let z = (f64::from(b[axis + 3]) - o) / d;
            lo = lo.max(a.min(z));
            hi = hi.min(a.max(z));
        }
    }
    lo <= hi
}
pub(super) fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let boxes = [[-1., -1., -1., 1., 1., 1.], [3., -1., -1., 5., 1., 1.]];
    let mut scene = voxy_render::RayScene::from_aabbs(device, &boxes, 2)?;
    assert_eq!(scene.primitive_count(), 2);
    assert_eq!(scene.triangle_count(), 0);
    assert!(matches!(
        voxy_render::RayScene::from_aabbs(device, &[[1., 0., 0., 0., 1., 1.]], 1),
        Err(voxy_render::RaySceneError::InvalidGeometry)
    ));
    let visibility = voxy_render::RayVisibilityPipeline::new(device)?;
    assert!(matches!(
        visibility.create_job(
            &scene,
            &[voxy_render::RaySegment::new(
                [0., 0., 3.],
                [0., 0., -3.],
                0.001
            )?]
        ),
        Err(voxy_render::RaySceneError::InvalidGeometry)
    ));
    let rays = [
        ([-2., 0., 0.], [1., 0., 0.]),
        ([4., 0., 3.], [0., 0., -1.]),
        ([7., 0., 3.], [0., 0., -1.]),
        ([0., 0., 3.], [0., 0., -1.]),
        ([0., 0., 0.], [1., 0., 0.]),
        ([0., 10., 3.], [0., 0., -1.]),
    ];
    let input: Vec<[f32; 8]> = rays
        .iter()
        .map(|(o, d)| [o[0], o[1], o[2], 0., d[0], d[1], d[2], 0.])
        .collect();
    let input_bytes: Vec<u8> = input
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("AABB candidate reference rays"),
        contents: &input_bytes,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: rays.len() as u64 * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: rays.len() as u64 * 16,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("procedural broad-phase coverage"),
        source: wgpu::ShaderSource::Wgsl(SOURCE.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: scene.binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut admitted = 0;
    let mut compacted = 0;
    for phase in 0..3 {
        if phase == 1 {
            scene.set_instance(1, [1., 0., 0., 0., 0., 1., 0., 10., 0., 0., 1., 0.], 7, 255)?;
        }
        if phase == 2 {
            scene.remove_instance(0)?;
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        if phase == 0 {
            scene.build(&mut encoder);
        } else {
            scene.build_instances(&mut encoder);
        }
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, rays.len() as u64 * 16);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let data = read.slice(..).get_mapped_range()?;
        for (i, (origin, direction)) in rays.iter().enumerate() {
            let native = u32::from_le_bytes(data[i * 16..i * 16 + 4].try_into()?);
            let actual = u32::from_le_bytes(data[i * 16 + 8..i * 16 + 12].try_into()?);
            assert_eq!(
                u32::from_le_bytes(data[i * 16 + 12..i * 16 + 16].try_into()?),
                0,
                "unexpected stable identity"
            );
            if native != actual {
                compacted += 1;
            }
            let mut expected = 0;
            for instance in 0..2 {
                if (instance == 0 && phase == 2) || (instance == 1 && phase == 0) {
                    continue;
                }
                for (primitive, b) in boxes.iter().enumerate() {
                    let mut b = *b;
                    if instance == 1 {
                        b[1] += 10.;
                        b[4] += 10.;
                    }
                    if intersects(b, *origin, *direction) {
                        expected |= 1u32 << (instance * 2 + primitive);
                    }
                }
            }
            assert_eq!(
                actual & expected,
                expected,
                "missing procedural candidate phase={phase} ray={i} full_mask={actual:#x}"
            );
            if phase == 2 {
                assert_eq!(actual & 3, 0, "removed instance still queried");
            }
            admitted += expected.count_ones();
        }
        drop(data);
        read.unmap();
    }
    println!(
        "RAY AABB CANDIDATE PASS: phases=3 rays_per_phase=6 required_candidates={admitted} cpu_f64_coverage=true false_positives_allowed=true transforms_removal=true stable_custom_identity=true native_slot_changes={compacted} triangle_consumers_rejected=true exact_narrow_phase_proven=false street_fps_proven=false"
    );
    Ok(())
}
