//! Actual-device qualification of two pipelines sharing one resident allocation.
use voxy_render::{ComputeProgram,ComputeMemoryBudget,GraphicsOptions};
fn main()->Result<(),Box<dyn std::error::Error>> {
    let instance=GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("RESIDENT ADAPTER {:?}",adapter.get_info());
    let (device,queue)=pollster::block_on(adapter.request_device(&Default::default()))?;
    let first=pollster::block_on(ComputeProgram::new(&device,"@group(0) @binding(0) var<storage,read_write> data:array<u32>; @compute @workgroup_size(64) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {if id.x<arrayLength(&data) {data[id.x]*=3u;}}"))?;
    let second=pollster::block_on(ComputeProgram::new(&device,"@group(0) @binding(0) var<storage,read_write> data:array<u32>; @compute @workgroup_size(64) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {if id.x<arrayLength(&data) {data[id.x]+=7u;}}"))?;
    let values:Vec<u32>=(0..129).collect();
    let mut job=first.create_job(&device,bytemuck::cast_slice(&values))?;
    let stats=ComputeMemoryBudget::for_device(&device).stats();
    let buffer=job.buffer().clone();let mut encoder=device.create_command_encoder(&Default::default());
    job.encode_step(&mut encoder,[3,1,1])?;
    job.use_program(&second)?;
    assert_eq!(job.buffer(),&buffer);assert_eq!(ComputeMemoryBudget::for_device(&device).stats(),stats);
    job.encode_step(&mut encoder,[3,1,1])?;
    let dispatch=job.encode_readback(&mut encoder)?;
    queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes=read.try_read()?.ok_or("pending resident readback")?;
    for (i,b) in bytes.chunks_exact(4).enumerate() {assert_eq!(u32::from_le_bytes(b.try_into()?),i as u32*3+7);}
    println!("RESIDENT CHAIN coordinates=129 pipelines=2 storage_allocations=1 intermediate_readbacks=0 admitted=true");
    Ok(())
}
