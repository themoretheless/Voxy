//! Resident QR plus equality solve; manufactured bounds on original columns.
//! No unilateral active-set or physical rod admission is claimed.
use voxy_render::{ComputeProgram,GraphicsOptions,ResidentContactEqualityInput,JOINT_CONTACT_QR_SHADER,JOINT_CONTACT_EQUALITY_SHADER};
fn dot(a:&[f64],b:&[f64])->f64 {
    let mut sum=0f64;let mut correction=0f64;
    for (&a,&b) in a.iter().zip(b) {
        let p=a*b;let next=sum+p;
        correction+=if sum.abs()>=p.abs() {(sum-next)+p} else {(p-next)+sum};
        correction+=a.mul_add(b,-p);sum=next;
    }
    sum+correction
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    let instance=GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("EQUALITY ADAPTER {:?}",adapter.get_info());
    let (device,queue)=pollster::block_on(adapter.request_device(&Default::default()))?;
    let columns:Vec<Vec<f64>>=if let Some(path)=std::env::args().nth(1) {
        let data=std::fs::read(path)?;
        if data.len()<28 || &data[..4]!=b"VQC1" {return Err("invalid VQC1 fixture".into());}
        let count=u32::from_le_bytes(data[4..8].try_into()?) as usize;
        let width=u32::from_le_bytes(data[8..12].try_into()?) as usize;
        if count==0 || count>4096 || width==0 || width>65536 {return Err("fixture dimensions".into());}
        let mut offset=28+count*16;
        let needed=count.checked_mul(width).and_then(|n|n.checked_mul(8)).and_then(|n|n.checked_add(offset)).ok_or("fixture size")?;
        if data.len()<needed {return Err("truncated fixture".into());}
        (0..count).map(|_|(0..width).map(|_| {
            let value=f64::from_le_bytes(data[offset..offset+8].try_into().unwrap());offset+=8;value
        }).collect()).collect()
    } else {(0..17).map(|j|(0..129).map(|i|
        if i==j {1.} else {((i*13+j*7)%37) as f64*0.001-0.018}
    ).collect()).collect()};
    if columns.iter().flatten().any(|v|!v.is_finite()) {return Err("nonfinite fixture".into());}
    let count=columns.len();let width=columns[0].len();

    let expected_reactions:Vec<f64>=(0..count).map(|i|1e-4/(i+1) as f64).collect();
    let expected:Vec<f64>=(0..width).map(|i|dot(&columns.iter().map(|c|c[i]).collect::<Vec<_>>(),&expected_reactions)).collect();
    let bounds:Vec<f64>=columns.iter().map(|c|dot(c,&expected)).collect();
    let input=ResidentContactEqualityInput::new(&columns,&bounds)?;
    let qr=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER))?;
    let solve=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_EQUALITY_SHADER))?;
    let mut job=qr.create_job(&device,input.bytes())?;
    let mut encoder=device.create_command_encoder(&Default::default());
    for _ in 0..input.columns() {job.encode_step(&mut encoder,[1,1,1])?;}
    job.use_program(&solve)?;job.encode_step(&mut encoder,[1,1,1])?;
    let dispatch=job.encode_readback(&mut encoder)?;
    queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes=read.try_read()?.ok_or("pending equality readback")?;
    let output=input.decode(&bytes)?;
    let mut bound_error=0f64;
    for (c,&bound) in columns.iter().zip(&bounds) {
        let scale=c.iter().zip(&output.coordinates).map(|(a,b)|(a*b).abs()).sum::<f64>().max(1e-30);
        bound_error=bound_error.max((dot(c,&output.coordinates)-bound).abs()/scale);
    }
    let scale=expected.iter().map(|v|v.abs()).fold(0.,f64::max).max(1e-30);
    let mut force_error=0f64;let mut coordinate_error=0f64;
    for i in 0..width {
        let force=dot(&columns.iter().map(|c|c[i]).collect::<Vec<_>>(),&output.reactions);
        force_error=force_error.max((force-output.coordinates[i]).abs()/scale);
        coordinate_error=coordinate_error.max((expected[i]-output.coordinates[i]).abs()/scale);
    }
    if bound_error>1e-8 || force_error>1e-8 || coordinate_error>1e-8 || output.reactions.iter().any(|v|*v < -1e-10) {
        return Err(format!("original equality admission failed bounds={bound_error:e} force={force_error:e} coordinates={coordinate_error:e}").into());
    }
    println!("RESIDENT EQUALITY columns={count} coordinates={width} bound_error={bound_error:e} force_error={force_error:e} coordinate_error={coordinate_error:e} intermediate_readbacks=0 manufactured_bounds=true unilateral_solver=false physical_rod_admission=false");
    Ok(())
}
