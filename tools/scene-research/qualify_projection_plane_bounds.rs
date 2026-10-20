//! Raw GPU sign certificates for an independent exact-rational audit.
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Case { matrix: [f32;16], point: [f32;4] }
fn main() {
    let root=std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&root).unwrap();
    let mut cases=Vec::new();
    let identity=glam::Mat4::IDENTITY.to_cols_array();
    let near=[f32::from_bits(0x3f7fffff),1.,f32::from_bits(0x3f800001)];
    for &x in &near {for sign in [-1.,1.] {for z in [0.,0.5,1.] {
        cases.push(Case {matrix:identity,point:[sign*x,0.,z,1.]});
        cases.push(Case {matrix:identity,point:[0.,sign*x,z,1.]});
    }}}
    for z in [-f32::from_bits(1),-0.,0.,f32::from_bits(1),f32::MIN_POSITIVE,f32::MAX] {
        cases.push(Case {matrix:identity,point:[0.,0.,z,1.]});
    }
    let mut random_state=0x504c414e45424e44u64;
    let mut random=|| {random_state=random_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);(random_state>>32) as u32};
    let specials=[0u32,0x80000000,1,0x80000001,0x007fffff,0x00800000,0x21800000,0x3f000000,0x3f800000,0x40000000,0x5d7fffff,0x7f7fffff];
    for index in 0..8192 {
        let mut value=|| {let bits=random(); if index%2==0 {f32::from_bits(specials[bits as usize%specials.len()]^(bits&0x80000000))} else {f32::from_bits((bits&0x807fffff)|(((bits>>24)%255)<<23))}};
        let mut matrix=std::array::from_fn(|_|value());
        let point=[value(),value(),value(),1.];
        // Exercise cancellation between homogeneous and spatial coefficients.
        if index%4==0 {for column in 0..4 {matrix[column*4+2]=matrix[column*4+3];}}
        cases.push(Case {matrix,point});
    }
    let instance=voxy_render::GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
    let(device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let source=concat!(include_str!("../../crates/voxy_render/src/archive_projection_pair.wgsl"),include_str!("../../crates/voxy_render/src/archive_ray_projection.wgsl"),include_str!("../../crates/voxy_render/src/archive_ray_interval.wgsl"),include_str!("projection_plane_bounds_reference.wgsl"),r#"
struct Case {matrix:mat4x4<f32>,point:vec4<f32>}
@group(0) @binding(0) var<storage,read> cases:array<Case>;
@group(0) @binding(1) var<storage,read_write> words:array<u32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&cases) {return;}let c=cases[id.x];let base=id.x*16u;var mask=0u;
 for(var plane=0u;plane<6u;plane++) {
  if research_plane_inside_bound(c.matrix,c.point,plane) {mask|=1u<<plane;}
  let original=dda_plane_distance(c.matrix,c.point,plane);
  words[base+1u+2u*plane]=bitcast<u32>(original.x);words[base+2u+2u*plane]=bitcast<u32>(original.y);
 }
 words[base]=mask;words[base+13u]=0u;words[base+14u]=0u;words[base+15u]=0u;
}
"#);
    let module=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("plane sign admission audit"),source:wgpu::ShaderSource::Wgsl(source.into())});
    let pipeline=device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {label:None,layout:None,module:&module,entry_point:Some("main"),compilation_options:Default::default(),cache:None});
    let input=device.create_buffer_init(&wgpu::util::BufferInitDescriptor {label:None,contents:bytemuck::cast_slice(&cases),usage:wgpu::BufferUsages::STORAGE});
    let size=cases.len() as u64*64;
    let output=device.create_buffer(&wgpu::BufferDescriptor {label:None,size,usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
    let readback=device.create_buffer(&wgpu::BufferDescriptor {label:None,size,usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});
    let group=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(0),entries:&[wgpu::BindGroupEntry {binding:0,resource:input.as_entire_binding()},wgpu::BindGroupEntry {binding:1,resource:output.as_entire_binding()}]});
    let mut encoder=device.create_command_encoder(&Default::default());
    {let mut pass=encoder.begin_compute_pass(&Default::default());pass.set_pipeline(&pipeline);pass.set_bind_group(0,&group,&[]);pass.dispatch_workgroups((cases.len() as u32).div_ceil(64),1,1);}
    encoder.copy_buffer_to_buffer(&output,0,&readback,0,size);let submission=queue.submit([encoder.finish()]);
    let(tx,rx)=std::sync::mpsc::channel();readback.slice(..).map_async(wgpu::MapMode::Read,move|result|tx.send(result).unwrap());
    device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();
    let bytes=readback.slice(..).get_mapped_range().unwrap();
    std::fs::write(root.join("inputs.bin"),bytemuck::cast_slice(&cases)).unwrap();std::fs::write(root.join("certificates.bin"),&bytes).unwrap();
    println!("GPU_PLANE_SIGN_CORPUS cases={} finite_inputs=true subnormals=true signed_zero=true overflow=true cancellation=true exact_rational_audit_pending=true",cases.len());
}
