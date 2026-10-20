//! Hardware candidate coverage against every hit in a real lossless snapshot.
//! The original exact DDA remains the geometric oracle; this is not FPS proof.
use std::path::Path;
use wgpu::util::DeviceExt;
pub(super) fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}
pub(super) fn i32_at(data: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}
pub(super) fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}
fn lower(value: f64) -> f32 {
    let r = value as f32;
    if f64::from(r) > value {
        r.next_down()
    } else {
        r
    }
}
fn upper(value: f64) -> f32 {
    let r = value as f32;
    if f64::from(r) < value { r.next_up() } else { r }
}
pub(super) struct Capture {
    pub(super) kind: String,
    pub(super) segments: Vec<u8>,
    pub(super) hits: Vec<u8>,
    pub(super) order: Vec<u8>,
}
pub(super) struct Snapshot {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) count: u32,
    pub(super) tw: u32,
    pub(super) th: u32,
    pub(super) captures: Vec<Capture>,
    pub(super) depth: Vec<u8>,
    pub(super) boxes: Vec<[f32; 6]>,
    pub(super) margin: [f64; 3],
    pub(super) active: u64,
    pub(super) envelope_hits: u64,
    pub(super) meta: serde_json::Value,
}
pub(super) fn load(
    device: &wgpu::Device,
    root: &Path,
) -> Result<Snapshot, Box<dyn std::error::Error>> {
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("metadata.json"))?)?;
    if meta["schema"] != "voxy-ray-chunk-v1"
        || meta["byte_order"] != "little"
        || meta["segment_stride"] != 176
        || meta["hit_stride"] != 32
    {
        return Err("unsupported lossless snapshot schema".into());
    }
    let width = u32::try_from(meta["viewport"][0].as_u64().ok_or("viewport width")?)?;
    let height = u32::try_from(meta["viewport"][1].as_u64().ok_or("viewport height")?)?;
    let count = u32::try_from(meta["ray_count_per_kind"].as_u64().ok_or("ray count")?)?;
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || count == 0
        || u64::from(count) * 176 > u64::from(device.limits().max_storage_buffer_binding_size)
    {
        return Err("snapshot dimensions exceed diagnostic limits".into());
    }
    let mut captures = Vec::new();
    let mut margin = [0f64; 3];
    let mut active = 0u64;
    for kind in meta["ray_kinds"].as_array().ok_or("ray kinds")? {
        let kind = kind.as_str().ok_or("ray kind")?;
        if kind != "gi" && kind != "ssr" {
            return Err("unknown ray kind".into());
        }
        let segments = std::fs::read(root.join(format!("{kind}.segments.bin")))?;
        let hits = std::fs::read(root.join(format!("{kind}.hits.bin")))?;
        if segments.len() != count as usize * 176 || hits.len() != count as usize * 32 {
            return Err("incomplete snapshot records".into());
        }
        let order = std::fs::read(root.join(format!("{kind}.order.bin")))?;
        if order.len() != count as usize * 4 {
            return Err("incomplete original ray queue".into());
        }
        let mut seen = vec![false; count as usize];
        for raw in order.chunks_exact(4) {
            let id = u32_at(raw, 0) as usize;
            if id >= seen.len() || std::mem::replace(&mut seen[id], true) {
                return Err("original ray queue is not a complete permutation".into());
            }
        }
        for s in segments.chunks_exact(176) {
            if u32_at(s, 160) != 1 {
                continue;
            }
            active += 1;
            for axis in 0..3 {
                let (b, e, bl, el) = (
                    f32_at(s, axis * 4),
                    f32_at(s, 16 + axis * 4),
                    f32_at(s, 96 + axis * 4),
                    f32_at(s, 112 + axis * 4),
                );
                let d = e - b;
                if ![b, e, bl, el, d].iter().all(|v| v.is_finite()) {
                    return Err("nonfinite active hardware ray".into());
                }
                // Bound origin/end residuals and the correctly-rounded Float32 subtraction.
                // The extra term uses the existing DDA qualification scale; it is not a
                // universal proof. Every actual source hit is checked against the envelope
                // below, before any hardware query or performance claim can be admitted.
                let subtraction = (f64::from(e) - f64::from(b) - f64::from(d)).abs();
                let roundoff = 4096.
                    * f64::from(f32::EPSILON).powi(2)
                    * (f64::from(b).abs() + f64::from(e).abs() + 1.);
                let error = 2. * f64::from(bl).abs() + f64::from(el).abs() + subtraction + roundoff;
                margin[axis] = margin[axis].max(error);
            }
        }
        captures.push(Capture {
            kind: kind.to_owned(),
            segments,
            hits,
            order,
        });
    }
    if captures.is_empty() {
        return Err("empty ray snapshot".into());
    }
    let depth = std::fs::read(root.join("depth.bin"))?;
    if depth.len() != width as usize * height as usize * 4 {
        return Err("incomplete native depth".into());
    }
    let tw = width.div_ceil(8);
    let th = height.div_ceil(8);
    let mut boxes = Vec::new();
    for ty in 0..th {
        for tx in 0..tw {
            let (mut zlo, mut zhi) = (1f32, 0f32);
            for y in ty * 8..((ty + 1) * 8).min(height) {
                for x in tx * 8..((tx + 1) * 8).min(width) {
                    let d = f32_at(&depth, (y as usize * width as usize + x as usize) * 4);
                    if !d.is_finite() || !(0. ..=1.).contains(&d) {
                        return Err("capture requires finite native depth in [0,1]".into());
                    }
                    zlo = zlo.min(d);
                    if d < 1. {
                        zhi = zhi.max(d);
                    }
                }
            }
            if zlo == 1. {
                zhi = 1.;
            }
            boxes.push([
                lower(f64::from(tx * 8) - margin[0]),
                lower(f64::from(ty * 8) - margin[1]),
                lower(f64::from(zlo) - margin[2]),
                upper(f64::from(((tx + 1) * 8).min(width)) + margin[0]),
                upper(f64::from(((ty + 1) * 8).min(height)) + margin[1]),
                upper(f64::from(zhi) + margin[2]),
            ]);
        }
    }
    let mut envelope_hits = 0u64;
    for c in &captures {
        for (s, h) in c.segments.chunks_exact(176).zip(c.hits.chunks_exact(32)) {
            let (x, y) = (i32_at(h, 0), i32_at(h, 4));
            if x < 0 {
                continue;
            }
            if x as u32 >= width || y < 0 || y as u32 >= height || u32_at(s, 160) != 1 {
                return Err("invalid original hit identity/state".into());
            }
            let b = boxes[(y as u32 / 8 * tw + x as u32 / 8) as usize];
            let t = f64::from(f32_at(h, 8)) + f64::from(f32_at(h, 12));
            if !(0. ..=1.).contains(&t) {
                return Err("original hit parameter outside hardware segment".into());
            }
            for axis in 0..3 {
                let origin = f32_at(s, axis * 4);
                let delta = f32_at(s, 16 + axis * 4) - origin;
                let at = f64::from(origin) + t * f64::from(delta);
                if at < f64::from(b[axis]) || at > f64::from(b[axis + 3]) {
                    return Err(format!(
                        "hardware envelope excludes original {} hit axis {axis}",
                        c.kind
                    )
                    .into());
                }
            }
            envelope_hits += 1;
        }
    }
    eprintln!(
        "CPU_ENVELOPE active_source_rays={active} checked_source_hits={envelope_hits} boxes={} margin={margin:?} universal_proof=false",
        boxes.len()
    );
    Ok(Snapshot {
        width,
        height,
        count,
        tw,
        th,
        captures,
        depth,
        boxes,
        margin,
        active,
        envelope_hits,
        meta,
    })
}
pub(super) fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let Snapshot {
        width,
        height,
        count,
        tw,
        th,
        captures,
        boxes,
        margin,
        active,
        envelope_hits,
        ..
    } = load(device, root)?;
    let scene = voxy_render::RayScene::from_aabbs(device, &boxes, 1)?;
    let start = std::time::Instant::now();
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.build(&mut encoder);
    let submission = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    let build_wall_ms = start.elapsed().as_secs_f64() * 1000.;
    let source = concat!(
        "enable wgpu_ray_query;\n",
        include_str!("../../voxy_render/src/archive_projection_pair.wgsl"),
        include_str!("../../voxy_render/src/archive_ray_projection.wgsl"),
        include_str!("../../voxy_render/src/archive_ray_interval.wgsl"),
        r#"
struct Hit {pixel:vec2<i32>,parameter:vec2<f32>,visited:u32,p0:u32,p1:u32,p2:u32}
struct Grid {tiles:vec2<u32>,viewport:vec2<u32>}
@group(0) @binding(0) var scene:acceleration_structure;
@group(0) @binding(1) var<storage,read> inputs:array<DdaProjectedSegment>;
@group(0) @binding(2) var<storage,read> reference:array<Hit>;
@group(0) @binding(3) var<storage,read_write> outputs:array<vec4<u32>>;
@group(0) @binding(4) var<uniform> grid:Grid;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs) {return;}let s=inputs[id.x];let hit=reference[id.x];let has_hit=hit.pixel.x>=0;
 var covered=select(1u,0u,has_hit);if s.state!=1u {outputs[id.x]=vec4<u32>(0u,covered,0u,0u);return;}
 let delta=s.end.xyz-s.begin.xyz;
 if !dda_finite4(vec4<f32>(delta,0.))||all(delta==vec3<f32>(0.)) {outputs[id.x]=vec4<u32>(0u,covered,0u,2u);return;}
 let expected=u32(max(0,hit.pixel.y))/8u*grid.tiles.x+u32(max(0,hit.pixel.x))/8u;
 var query:ray_query;rayQueryInitialize(&query,scene,RayDesc(0u,255u,0.,1.,s.begin.xyz,delta));var count=0u;var errors=0u;
 while rayQueryProceed(&query) {let candidate=rayQueryGetCandidateIntersection(&query);if candidate.kind!=3u||candidate.primitive_index>=grid.tiles.x*grid.tiles.y||candidate.instance_custom_data!=0u {errors+=1u;}else{count+=1u;if has_hit&&candidate.primitive_index==expected {covered=1u;}}}
 outputs[id.x]=vec4<u32>(count,covered,errors,1u);
}
"#
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("complete captured hardware candidate coverage"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let grid: Vec<u8> = [tw, th, width, height]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    let grid = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &grid,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let mut reports = Vec::new();
    for c in captures {
        let segments = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lossless original ray payload"),
            contents: &c.segments,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let reference = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("original exact DDA hits"),
            contents: &c.hits,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bytes = u64::from(count) * 16;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
                    resource: segments.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: reference.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: grid.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, bytes);
        let start = std::time::Instant::now();
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let query_readback_wall_ms = start.elapsed().as_secs_f64() * 1000.;
        let data = read.slice(..).get_mapped_range()?;
        let (mut candidates, mut missing, mut errors, mut traced, mut fallback, mut source_hits) =
            (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
        for (index, row) in data.chunks_exact(16).enumerate() {
            candidates += u64::from(u32_at(row, 0));
            missing += u64::from(u32_at(row, 4) == 0);
            errors += u64::from(u32_at(row, 8));
            traced += u64::from(u32_at(row, 12) == 1);
            fallback += u64::from(u32_at(row, 12) == 2);
            source_hits += u64::from(i32_at(&c.hits, index * 32) >= 0);
        }
        drop(data);
        read.unmap();
        let report = serde_json::json!({"kind":c.kind,"rays":count,"source_hits":source_hits,"hardware_traced":traced,"fallback_required":fallback,"candidates":candidates,"missing_source_hits":missing,"invalid_candidates":errors,"query_and_readback_wall_ms":query_readback_wall_ms});
        println!("HARDWARE_COVERAGE {report}");
        reports.push(report);
        if missing != 0 || errors != 0 || fallback != 0 {
            return Err(
                "hardware candidate coverage is not admitted for this complete snapshot".into(),
            );
        }
    }
    println!(
        "{}",
        serde_json::json!({"schema":"voxy-hardware-candidate-coverage-v1","scope":"complete captured chunks only, not full-frame or universal proof","viewport":[width,height],"box_count":scene.primitive_count(),"input_aabb_bytes":boxes.len()*24,"margin":margin,"active_source_rays":active,"cpu_envelope_hits":envelope_hits,"build_wall_ms":build_wall_ms,"results":reports,"exact_narrow_phase_proven":false,"street_hardware_connected":false,"minimum_120_presented_fps_passed":false})
    );
    Ok(())
}
