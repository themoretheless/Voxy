use voxy_render::ComputeProgram;
pub fn qualify(device: &wgpu::Device, queue: &wgpu::Queue)->Result<(),Box<dyn std::error::Error>> {
    qualify_source(device,queue,include_str!("../../src/hair_banded_compensated.wgsl"))
}
pub fn qualify_source(device:&wgpu::Device,queue:&wgpu::Queue,source:&str)->Result<(),Box<dyn std::error::Error>> {
    let count=1024usize;let mut words=vec![count as u32,0,0,0];let mut inputs=Vec::new();
    for i in 0..count {
        let a=(1.0+(i*37%997) as f64/997.)*10f64.powi((i%9) as i32-4);
        let b=if i%3==0 {-a*(1.-1e-7)} else {(1.+(i*71%991) as f64/991.)*10f64.powi((i%7) as i32-3)};
        for value in [a,b] {let hi=value as f32;words.push(hi.to_bits());words.push(((value-hi as f64) as f32).to_bits());}
        words.extend([0;8]);inputs.push((a,b));
    }
    let prefix=source.split("@compute").next().unwrap();
    let shader=format!("{prefix} @compute @workgroup_size(32) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {{if id.x>=data[0] {{return;}} let base=4u+id.x*12u; let a=load(base);let b=load(base+2u);save(base+4u,add(a,b));save(base+6u,mul(a,b));save(base+8u,divide(a,b));save(base+10u,root(a));}}");
    let program=pollster::block_on(ComputeProgram::new(&device,&shader))?;let job=program.create_job(&device,bytemuck::cast_slice(&words))?;
    let mut encoder=device.create_command_encoder(&Default::default());let dispatch=job.encode(&mut encoder,[(count as u32).div_ceil(32),1,1])?;queue.submit([encoder.finish()]);
    let mut read=dispatch.begin_read();device.poll(wgpu::PollType::wait_indefinitely())?;let bytes=read.try_read()?.ok_or("pending")?;let result:&[u32]=bytemuck::cast_slice(&bytes);
    let mut errors=[0f64;4];let mut worst=[0usize;4];
    for (i,&(a,b)) in inputs.iter().enumerate() {
        let expected=[a+b,a*b,a/b,a.sqrt()];
        for operation in 0..4 {
            let offset=4+i*12+4+operation*2;
            let value=f32::from_bits(result[offset]) as f64+f32::from_bits(result[offset+1]) as f64;
            if !value.is_finite() {return Err("nonfinite compensated output".into());}
            let scale=if operation==0 {a.abs()+b.abs()} else {expected[operation].abs()};
            let error=(value-expected[operation]).abs()/scale.max(1e-30);
            if error>errors[operation] {errors[operation]=error;worst[operation]=i;}
        }
    }
    println!("COMPENSATED RUNTIME normalized_errors={errors:?} worst_samples={worst:?}");
    if errors.iter().any(|v|*v>1e-12) {return Err("compensated runtime arithmetic gate failed".into());}
    println!("PASS: 1024 runtime pairs, cancellation and dynamic ranges; matrix gate remains separate");Ok(())
}
