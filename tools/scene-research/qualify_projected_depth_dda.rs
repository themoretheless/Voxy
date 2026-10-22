//! Full camera -> depth intersection -> world reconstruction candidate gate.
use wgpu::util::DeviceExt;
mod depth_image_reference;
mod qualify_depth_mips;
mod qualify_ray_intervals;
mod ray_projection_reference;
use ray_projection_reference::{Case, oracle, oracle_at};
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ResultRow {
    pixel: [i32; 2],
    screen_parameter: [f32; 2],
    visited: u32,
    hit_padding: [u32; 3],
    world_parameter: [f32; 2],
    state: u32,
    padding: u32,
    world_high: [f32; 4],
    world_low: [f32; 4],
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(
        !args
            .iter()
            .any(|a| a == "--lazy-cell-bounds" || a == "--touch-guard"),
        "archived optimization requires its pinned research executable"
    );
    let resumable = args.iter().any(|a| a == "--resumable");
    let stack_traversal = args.iter().any(|a| a == "--stack-traversal");
    let pixel_leaves = args.iter().any(|a| a == "--pixel-leaves");
    let reciprocal_cache = args.iter().any(|a| a == "--reciprocal-cache");
    let near_first = args.iter().any(|a| a == "--near-first");
    let interval_cache = args.iter().any(|a| a == "--interval-cache");
    let plane_cache = args.iter().any(|a| a == "--plane-cache");
    let point_cache = args.iter().any(|a| a == "--point-cache");
    let tile_edge_stats = args.iter().any(|a| a == "--tile-edge-stats");
    let full_records = args.iter().any(|a| a == "--full-records");
    let tile_edge_cache = args.iter().any(|a| a == "--tile-edge-cache");
    let edge_cache = args.iter().any(|a| a == "--edge-cache");
    let depth_only = args.iter().any(|a| a == "--depth-only");
    let hierarchy = args.iter().any(|a| a == "--hierarchy");
    let far = args.iter().any(|a| a == "--far-bound");
    let cells = args.iter().any(|a| a == "--cell-bounds");
    let tiles = args.iter().any(|a| a == "--tiles");
    let native_depth = args.iter().any(|a| a == "--native-depth");
    let w = args
        .get(1)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(17);
    let h = args
        .get(2)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(13);
    assert!(w > 0 && h > 0);
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    assert!(
        !depth_only || native_depth,
        "depth-only requires actual native depth owner"
    );
    assert!(
        !stack_traversal || (native_depth && hierarchy),
        "stack traversal requires actual hierarchy owner"
    );
    assert!(
        !pixel_leaves || stack_traversal,
        "pixel leaves require stack traversal"
    );
    assert!(
        !reciprocal_cache || stack_traversal,
        "reciprocal cache requires stack traversal"
    );
    assert!(
        !near_first || stack_traversal,
        "near-first requires stack traversal"
    );
    assert!(!tile_edge_stats || (!resumable && !stack_traversal && tile_edge_cache));
    #[allow(deprecated)]
    let matrices = [
        glam::Mat4::perspective_rh(65f32.to_radians(), 16. / 9., 0.05, 1000.),
        glam::Mat4::perspective_rh(65f32.to_radians(), 16. / 9., 0.05, 1000.)
            * glam::Mat4::from_translation(glam::Vec3::new(0., -1.8, 0.)),
        glam::Mat4::orthographic_rh(-8., 8., -6., 6., 0.05, 200.),
        glam::Mat4::perspective_rh(100f32.to_radians(), 1., 0.001, 300.)
            * glam::Mat4::look_at_rh(
                glam::Vec3::new(12., 5., 16.),
                glam::Vec3::new(0., 1., 0.),
                glam::Vec3::Y,
            ),
    ];
    let mut cases = Vec::new();
    let mut state = 0x434f4d42494e4544u64;
    let mut random = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };
    for matrix in matrices {
        for _ in 0..2048 {
            let begin = [
                (random() % 1921) as f32 / 16. - 60.,
                (random() % 961) as f32 / 16. - 30.,
                (random() % 2561) as f32 / 16. - 100.,
                0.,
            ];
            let end = [
                (random() % 1921) as f32 / 16. - 60.,
                (random() % 961) as f32 / 16. - 30.,
                (random() % 2561) as f32 / 16. - 100.,
                0.,
            ];
            cases.push(Case {
                matrix: matrix.to_cols_array(),
                begin,
                end,
                sample: [0., w as f32, h as f32, 0.],
            });
        }
        let inverse =
            glam::DMat4::from_cols_array(&matrix.to_cols_array().map(f64::from)).inverse();
        let eye_h = inverse * glam::DVec4::new(0., 0., 1., 0.);
        if eye_h.w.abs() > 1e-12 {
            let eye = eye_h.truncate() / eye_h.w;
            let near_h = inverse * glam::DVec4::new(0., 0., 0., 1.);
            let forward = (near_h.truncate() / near_h.w - eye).normalize();
            for far in [1., 100., 10000.] {
                for reverse in [false, true] {
                    let a = eye + forward * far;
                    let b = eye - forward;
                    let (a, b) = if reverse { (b, a) } else { (a, b) };
                    cases.push(Case {
                        matrix: matrix.to_cols_array(),
                        begin: [a.x as f32, a.y as f32, a.z as f32, 0.],
                        end: [b.x as f32, b.y as f32, b.z as f32, 0.],
                        sample: [0., w as f32, h as f32, 0.],
                    });
                }
            }
        }
    }
    // Exact identity-camera grid and outer-edge rays, both directions and
    // coplanar depth. Even extents make the centre a closed tile boundary.
    for z in [0.2f32, 0.99, 0.999] {
        for axis in 0..2 {
            for fixed in [-1f32, 0., 1.] {
                let mut a = [fixed, -1., z, 0.];
                let mut b = [fixed, 1., z, 0.];
                if axis == 1 {
                    a.swap(0, 1);
                    b.swap(0, 1);
                }
                for ramp in [false, true] {
                    let mut begin = a;
                    let mut end = b;
                    if ramp {
                        begin[2] = 0.;
                        end[2] = 1.;
                    }
                    for (begin, end) in [(begin, end), (end, begin)] {
                        cases.push(Case {
                            matrix: glam::Mat4::IDENTITY.to_cols_array(),
                            begin,
                            end,
                            sample: [0., w as f32, h as f32, 0.],
                        });
                    }
                }
            }
        }
    }
    if args.iter().any(|a| a == "--targeted-rays") {
        cases.clear();
        for (a, b) in [
            ([-1., -1., 0.], [1., 1., 1.]),
            ([0., -1., 0.], [0., 1., 1.]),
            ([1., -1., 0.99], [1., 1., 0.99]),
            ([-1., -1., 0.99], [1., 1., 0.99]),
        ] {
            for (a, b) in [(a, b), (b, a)] {
                cases.push(Case {
                    matrix: glam::Mat4::IDENTITY.to_cols_array(),
                    begin: [a[0], a[1], a[2], 0.],
                    end: [b[0], b[1], b[2], 0.],
                    sample: [0., w as f32, h as f32, 0.],
                });
            }
        }
    }
    if args.iter().any(|a| a == "--near-axis-rays") {
        cases.clear();
        let mut push = |a: [f32; 3], b: [f32; 3]| {
            for (a, b) in [(a, b), (b, a)] {
                cases.push(Case {
                    matrix: glam::Mat4::IDENTITY.to_cols_array(),
                    begin: [a[0], a[1], a[2], 0.],
                    end: [b[0], b[1], b[2], 0.],
                    sample: [0., w as f32, h as f32, 0.],
                });
            }
        };
        for fixed in [0.25f32, 0.5, 0.75] {
            for axis in 0..2 {
                for offset in [-1i32, 1] {
                    let mut a = [-1., -1., 0.];
                    let mut b = [1., 1., 1.];
                    a[axis] = fixed;
                    b[axis] = f32::from_bits((fixed.to_bits() as i64 + i64::from(offset)) as u32);
                    push(a, b);
                }
            }
        }
        for fixed in [0.2f32, 0.99, 0.999] {
            for offset in [-1i32, 1] {
                push(
                    [-1., -1., fixed],
                    [
                        1.,
                        1.,
                        f32::from_bits((fixed.to_bits() as i64 + i64::from(offset)) as u32),
                    ],
                );
            }
        }
    }
    let source = concat!(
        include_str!("../../crates/voxy_render/src/archive_projection_pair.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_projection.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_depth_dda_paired.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_interval.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_depth_dda_resumable.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_depth_hierarchy.wgsl"),
        r#"
override ARCHIVE_RESUMABLE:bool=false;
fn trace_depth_dda_resumable(image:texture_2d<f32>,begin:DdaVectorPair,end:DdaVectorPair)->DepthDdaPairHit {
 var cursor=dda_cursor_init(image,begin,end);loop {if cursor.done {break;}dda_cursor_step(image,&cursor);}return cursor.hit;
}
struct Case {matrix:mat4x4<f32>,begin:vec4<f32>,end:vec4<f32>,sample:vec4<f32>}
struct ResultRow {hit:DepthDdaPairHit,world_parameter:vec2<f32>,state:u32,padding:u32,world_high:vec4<f32>,world_low:vec4<f32>}
@group(0) @binding(3) var tile_depth:texture_2d<f32>;
fn dda_tile_range(tile:vec2<i32>,level:i32)->vec2<f32>{return textureLoad(tile_depth,tile,level).xy;}
fn dda_tile_levels()->u32{return textureNumLevels(tile_depth);}
@group(0) @binding(4) var pixel_depth:texture_2d<f32>;
fn hierarchy_candidate_levels()->u32 {if ARCHIVE_STACK_PIXEL_LEAVES {return textureNumLevels(pixel_depth);}return dda_tile_levels();}
fn hierarchy_candidate_leaf_extent()->i32 {return select(8,1,ARCHIVE_STACK_PIXEL_LEAVES);}
fn hierarchy_candidate_range(tile:vec2<i32>,level:i32)->vec2<f32> {if ARCHIVE_STACK_PIXEL_LEAVES {return textureLoad(pixel_depth,tile,level).xy;}return dda_tile_range(tile,level);}

@group(0) @binding(0) var depth:texture_2d<f32>;
@group(0) @binding(1) var<storage,read> inputs:array<Case>;
@group(0) @binding(2) var<storage,read_write> outputs:array<ResultRow>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs){return;}let c=inputs[id.x];let segment=dda_project_segment(c.matrix,c.begin.xyz,c.end.xyz,c.sample.yz);var hit=precise_miss(0u);var parameter=vec2<f32>(0.);var point=DdaVectorPair(vec4<f32>(0.),vec4<f32>(0.));
 if segment.state==1u {if ARCHIVE_STACK_TRAVERSAL {hit=trace_depth_hierarchy_candidate(depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));}else if ARCHIVE_RESUMABLE {hit=trace_depth_dda_resumable(depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));}else{hit=trace_depth_dda_paired(depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));}if hit.pixel.x>=0 {parameter=dda_reconstruct_parameter_paired(segment,hit.parameter);point=dda_reconstruct_world_paired(segment,hit.parameter);}}
 var mismatch=0u;
 if (ARCHIVE_STACK_TRAVERSAL||ARCHIVE_RESUMABLE)&&segment.state==1u {
  let reference=trace_depth_dda_paired(depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));
  // Work counters legitimately differ; geometric identity and BOTH parameter
  // words must remain bit-identical, independently of the F64 error bound.
  if any(hit.pixel!=reference.pixel)||any(bitcast<vec2<u32>>(hit.parameter)!=bitcast<vec2<u32>>(reference.parameter))||(ARCHIVE_RESUMABLE&&!ARCHIVE_STACK_TRAVERSAL&&hit.visited!=reference.visited) {mismatch=1u;}
 }
 outputs[id.x]=ResultRow(hit,parameter,segment.state,select(mismatch,dda_tile_edge_reuses,ARCHIVE_DDA_TILE_EDGE_STATS),point.high,point.low);
}
@compute @workgroup_size(64) fn interval_proof(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs){return;}let c=inputs[id.x];let a=dda_interval_pair(c.begin.xy);let b=dda_interval_pair(c.end.xy);let sum=dda_interval_add(a,b);let product=dda_interval_mul(a,b);let quotient=dda_interval_div(a,b);let inverse=dda_interval_div(DdaInterval(vec2<f32>(1.),true),b);let cached_quotient=dda_interval_mul(a,inverse);
 // Observe the actual unchanged paired division, not only its real quotient.
 var paired_quotient=vec2<f32>(0.);
 let epsilon=0.00000011920928955078125;
 let bounded=abs(c.begin.x)<=2.&&abs(c.end.x)<=2.;
 let normalized=abs(c.begin.y)<=abs(c.begin.x)*epsilon&&abs(c.end.y)<=abs(c.end.x)*epsilon;
 let paired_domain=quotient.valid&&bounded&&normalized&&dda_interval_finite(c.begin.xy)&&dda_interval_finite(c.end.xy);
 if paired_domain {paired_quotient=projection_pair_div(c.begin.xy,c.end.xy);}
 outputs[id.x]=ResultRow(precise_miss(0u),paired_quotient,select(0u,1u,sum.valid)|select(0u,2u,product.valid)|select(0u,4u,quotient.valid)|select(0u,8u,dda_far_plane_inside(c.matrix,vec4<f32>(c.begin.xyz,1.)))|select(0u,16u,paired_domain)|select(0u,32u,cached_quotient.valid),0u,vec4<f32>(sum.bounds,product.bounds),vec4<f32>(quotient.bounds,cached_quotient.bounds));
}
"#
    );
    let source = if native_depth {
        source
            .replace("image:texture_2d<f32>", "image:texture_depth_2d")
            .replace("var depth:texture_2d<f32>", "var depth:texture_depth_2d")
            .replace(
                "textureLoad(image,vec2<i32>(x,y),0).x",
                "textureLoad(image,vec2<i32>(x,y),0)",
            )
    } else {
        source.to_owned()
    };
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("combined candidate DDA gate"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: if native_depth {
                        wgpu::TextureSampleType::Depth
                    } else {
                        wgpu::TextureSampleType::Float { filterable: false }
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants: &[
                ("ARCHIVE_RESUMABLE", if resumable { 1. } else { 0. }),
                ("ARCHIVE_STACK_NEAR_FIRST", if near_first { 1. } else { 0. }),
                (
                    "ARCHIVE_STACK_RECIPROCAL_CACHE",
                    if reciprocal_cache { 1. } else { 0. },
                ),
                (
                    "ARCHIVE_STACK_PIXEL_LEAVES",
                    if pixel_leaves { 1. } else { 0. },
                ),
                (
                    "ARCHIVE_STACK_TRAVERSAL",
                    if stack_traversal { 1. } else { 0. },
                ),
                (
                    "ARCHIVE_DDA_INTERVAL_CACHE",
                    if interval_cache { 1. } else { 0. },
                ),
                ("ARCHIVE_DDA_PLANE_CACHE", if plane_cache { 1. } else { 0. }),
                ("ARCHIVE_DDA_POINT_CACHE", if point_cache { 1. } else { 0. }),
                (
                    "ARCHIVE_DDA_TILE_EDGE_STATS",
                    if tile_edge_stats { 1. } else { 0. },
                ),
                (
                    "ARCHIVE_DDA_TILE_EDGE_CACHE",
                    if tile_edge_cache { 1. } else { 0. },
                ),
                ("ARCHIVE_DDA_EDGE_CACHE", if edge_cache { 1. } else { 0. }),
                ("ARCHIVE_DDA_HIERARCHY", if hierarchy { 1. } else { 0. }),
                ("ARCHIVE_DDA_TILES", if tiles { 1. } else { 0. }),
                ("ARCHIVE_DDA_CELL_BOUNDS", if cells { 1. } else { 0. }),
                ("ARCHIVE_DDA_FAR_BOUND", if far { 1. } else { 0. }),
            ],
            ..Default::default()
        },
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&cases),
        usage: wgpu::BufferUsages::STORAGE,
    });
    assert_eq!(std::mem::size_of::<ResultRow>(), 80);
    let size = (cases.len() * 80) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let input_view = texture.create_view(&Default::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("native depth attachment"),
        size: texture.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&Default::default());
    let copy_shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:None,source:wgpu::ShaderSource::Wgsl(r"
@group(0) @binding(0) var input_depth:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.,1.);}
@fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{return textureLoad(input_depth,vec2<i32>(p.xy),0).x;}
".into())});
    let copy = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &copy_shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &copy_shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    });
    let copy_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &copy.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&input_view),
        }],
    });
    let tile_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: w.div_ceil(8),
            height: h.div_ceil(8),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg32Float,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let native_bounds = if native_depth {
        Some(if hierarchy {
            voxy_render::archive_depth_bounds::ArchiveDepthBounds::new_hierarchy(
                &device, &depth, w, h,
            )
            .unwrap()
        } else {
            voxy_render::archive_depth_bounds::ArchiveDepthBounds::new(&device, &depth, w, h)
                .unwrap()
        })
    } else {
        assert!(!hierarchy, "hierarchy gate requires actual native owner");
        None
    };
    let tile_view = if let Some(owner) = &native_bounds {
        owner.texture.create_view(&Default::default())
    } else {
        tile_texture.create_view(&Default::default())
    };
    let pixel_bounds = if pixel_leaves {
        Some(
            voxy_render::archive_depth_bounds::ArchiveDepthBounds::new_pixel_hierarchy(
                &device, &depth, w, h,
            )
            .unwrap(),
        )
    } else {
        None
    };
    let pixel_view = pixel_bounds
        .as_ref()
        .map(|owner| owner.texture.create_view(&Default::default()));
    let view = if native_depth {
        &depth_view
    } else {
        &input_view
    };
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(
                    pixel_view.as_ref().unwrap_or(&tile_view),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&tile_view),
            },
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
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
    let (mut failures, mut expected_hits) = (0, 0);
    let mut visits = 0u64;
    let mut signature = 0xcbf29ce484222325u64;
    let mut exact_output = Vec::new();
    let (mut max_world, mut max_parameter) = (0f64, 0f64);
    let mut tile_reuses = 0u64;
    #[allow(clippy::possible_missing_else)]
    for pattern in 0..4 {
        let depths: Vec<f32> = (0..w * h)
            .map(|i| match pattern {
                0 => 0.99,
                1 => {
                    if i % w == w / 2 {
                        0.999
                    } else {
                        1.
                    }
                }
                2 => 1. - (1 + (i * 37) % 15) as f32 / 4096.,
                _ => {
                    if i % 5 == 0 {
                        1.
                    } else {
                        (1 + i % 15) as f32 / 16.
                    }
                }
            })
            .collect();
        let tw = w.div_ceil(8);
        let th = h.div_ceil(8);
        let mut minima = vec![[1f32, 0f32]; (tw * th) as usize];
        for y in 0..h {
            for x in 0..w {
                let k = ((y / 8) * tw + x / 8) as usize;
                let d = depths[(y * w + x) as usize];
                minima[k][0] = minima[k][0].min(d);
                if d < 1. {
                    minima[k][1] = minima[k][1].max(d);
                }
            }
        }
        queue.write_texture(
            tile_texture.as_image_copy(),
            bytemuck::cast_slice(&minima),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(tw * 8),
                rows_per_image: Some(th),
            },
            tile_texture.size(),
        );
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&depths),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            texture.size(),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        if native_depth {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&copy);
            pass.set_bind_group(0, &copy_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        if let Some(owner) = &native_bounds {
            owner.encode(&mut encoder);
        }
        if let Some(owner) = &pixel_bounds {
            owner.encode(&mut encoder);
        }
        if depth_only {
            let submission = queue.submit([encoder.finish()]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            qualify_depth_mips::run(
                &device,
                &queue,
                &native_bounds.as_ref().unwrap().texture,
                &depths,
                w,
                h,
            );
            if let Some(owner) = &pixel_bounds {
                qualify_depth_mips::run_with_leaf_extent(
                    &device,
                    &queue,
                    &owner.texture,
                    &depths,
                    w,
                    h,
                    1,
                );
            }
            continue;
        }
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups((cases.len() as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let data = read.slice(..).get_mapped_range().unwrap();
        for (i, row) in data.chunks_exact(80).enumerate() {
            let actual: ResultRow = bytemuck::pod_read_unaligned(row);
            visits += u64::from(actual.visited);
            if tile_edge_stats {
                tile_reuses += u64::from(actual.padding);
            }
            if full_records {
                exact_output.extend_from_slice(row);
            } else {
                exact_output.extend_from_slice(&row[..16]);
                exact_output.extend_from_slice(&row[32..]);
            }
            for byte in row[..16].iter().chain(row[32..].iter()) {
                signature ^= u64::from(*byte);
                signature = signature.wrapping_mul(0x100000001b3);
            }
            let c = cases[i];
            let expected = oracle(c).and_then(|(_, a, b, _, _, _)| {
                let to_pixel = |p: glam::DVec3| {
                    [
                        (p.x * 0.5 + 0.5) * w as f64,
                        (-p.y * 0.5 + 0.5) * h as f64,
                        p.z,
                    ]
                };
                depth_image_reference::intersect(
                    &depths,
                    w as usize,
                    h as usize,
                    to_pixel(a),
                    to_pixel(b),
                )
            });
            let state_ok = oracle(c).is_some() == (actual.state == 1)
                && actual.state != 2
                && (actual.padding == 0 || tile_edge_stats);
            let okay = if !state_ok {
                false
            } else if let Some((pixel, s)) = expected {
                expected_hits += 1;
                let (_, _, _, u, p, _) = oracle_at(c, s).unwrap();
                let world = glam::DVec3::new(
                    actual.world_high[0] as f64 + actual.world_low[0] as f64,
                    actual.world_high[1] as f64 + actual.world_low[1] as f64,
                    actual.world_high[2] as f64 + actual.world_low[2] as f64,
                );
                let error = (world - p).abs().max_element();
                let parameter_error =
                    (actual.world_parameter[0] as f64 + actual.world_parameter[1] as f64 - u).abs();
                max_world = max_world.max(error);
                max_parameter = max_parameter.max(parameter_error);
                let scale = c.begin[..3]
                    .iter()
                    .chain(c.end[..3].iter())
                    .map(|x| x.abs() as f64)
                    .fold(1f64, f64::max);
                actual.pixel == pixel
                    && error <= 4096. * (f32::EPSILON as f64).powi(2) * scale
                    && parameter_error <= 4096. * (f32::EPSILON as f64).powi(2)
            } else {
                actual.pixel == [-1, -1]
            };
            if !okay {
                if failures < 10 {
                    eprintln!(
                        "COMBINED_MISMATCH pattern={pattern} case={i} input={c:?} expected={expected:?} actual={actual:?}"
                    );
                }
                failures += 1;
            }
        }
        drop(data);
        read.unmap();
        if let Some(owner) = &native_bounds {
            qualify_depth_mips::run(&device, &queue, &owner.texture, &depths, w, h);
        }
        if let Some(owner) = &pixel_bounds {
            qualify_depth_mips::run_with_leaf_extent(
                &device,
                &queue,
                &owner.texture,
                &depths,
                w,
                h,
                1,
            );
        }
    }
    if depth_only {
        println!(
            "GPU_DEPTH_HIERARCHY_ONLY source={w}x{h} patterns=4 rays_tested=0 all_gpu_mips_checked=true production_speedup_qualified=false"
        );
        return;
    }
    qualify_ray_intervals::run(
        &device,
        &queue,
        &shader,
        &pipeline_layout,
        &layout,
        view,
        &tile_view,
    );
    if let Some([_, output_path]) = args.array_windows::<2>().find(|a| a[0] == "--output") {
        std::fs::write(output_path, &exact_output).unwrap();
    }
    println!(
        "TILE_EDGE_CACHE configured={tile_edge_cache} stats={tile_edge_stats} observed_reuses={tile_reuses} full_records={full_records}"
    );
    println!(
        "GPU_PROJECTED_DEPTH_DDA viewport={w}x{h} native_depth={native_depth} resumable={resumable} stack_traversal={stack_traversal} pixel_leaves={pixel_leaves} reciprocal_cache={reciprocal_cache} near_first={near_first} tiles={tiles} cells={cells} far={far} hierarchy={hierarchy} edge_cache={edge_cache} interval_cache={interval_cache} point_cache={point_cache} plane_cache={plane_cache} cases={} patterns=4 expected_hits={expected_hits} failures={failures} visited={visits} output_signature={signature:016x} max_world_error={max_world} max_parameter_error={max_parameter} production_enabled=false",
        cases.len() * 4
    );
    assert_eq!(failures, 0, "combined candidate is not admitted");
}
