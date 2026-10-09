//! Hardware experiment: GPU CGS2 products/updates/norms/normalization.
//! Packing, orchestration and independent admission remain on CPU.
//! This does not publish a physical contact solution or replace native MGS.
use voxy_render::{ComputeProgram,GraphicsOptions,JointContactDotInput,JOINT_CONTACT_DOT_SHADER,JOINT_CONTACT_NORMALIZE_SHADER,JOINT_CONTACT_NORMALIZE_SERIAL_SHADER};
use voxy_render::{ResidentContactQrInput,JOINT_CONTACT_QR_SHADER};
fn dot(a:&[f64],b:&[f64])->f64 {
    let mut sum=0f64;let mut correction=0f64;
    for (&a,&b) in a.iter().zip(b) {
        let p=a*b;let next=sum+p;
        correction+=if sum.abs()>=p.abs() {(sum-next)+p} else {(p-next)+sum};
        correction+=a.mul_add(b,-p);sum=next;
    }
    sum+correction
}
fn evaluate(device:&wgpu::Device,queue:&wgpu::Queue,program:&ComputeProgram,input:JointContactDotInput)->Result<Vec<f64>,Box<dyn std::error::Error>> {
    let job=program.create_job(device,input.bytes())?;
    let mut encoder=device.create_command_encoder(&Default::default());
    let (x,y,z)=input.dispatch();let dispatch=job.encode(&mut encoder,[x,y,z])?;
    queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes=read.try_read()?.ok_or("pending QR readback")?;
    Ok(input.decode(&bytes)?)
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    let instance=GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("QR ADAPTER {:?}",adapter.get_info());
    let (device,queue)=pollster::block_on(adapter.request_device(&Default::default()))?;
    let program=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_DOT_SHADER))?;
    let serial=std::env::var_os("VOXY_QR_SERIAL_NORMALIZE").is_some();
    let normalize=pollster::block_on(ComputeProgram::new(&device,if serial {JOINT_CONTACT_NORMALIZE_SERIAL_SHADER} else {JOINT_CONTACT_NORMALIZE_SHADER}))?;
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
    let mut basis:Vec<Vec<f64>>=Vec::new();let mut triangular:Vec<Vec<f64>>=Vec::new();
    let mut norm_error=0f64;
    let resident=std::env::var_os("VOXY_QR_RESIDENT").is_some();
    if resident {
        let input=ResidentContactQrInput::new(&columns)?;
        let resident_program=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER))?;
        let job=resident_program.create_job(&device,input.bytes())?;
        let mut encoder=device.create_command_encoder(&Default::default());
        for _ in 0..input.columns() {job.encode_step(&mut encoder,[1,1,1])?;}
        let dispatch=job.encode_readback(&mut encoder)?;
        queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let bytes=read.try_read()?.ok_or("pending resident QR")?;
        let output=input.decode(&bytes)?;basis=output.basis;triangular=output.triangular_columns;
    } else {
    for original in &columns {
        let mut residual=original.clone();let mut coefficients=vec![0.;basis.len()];
        for _ in 0..2 {
            if basis.is_empty() {break;}
            let alpha=evaluate(&device,&queue,&program,JointContactDotInput::new(&basis,&residual)?)?;
            residual=evaluate(&device,&queue,&program,JointContactDotInput::projection(&basis,&alpha,&residual)?)?;
            for (total,a) in coefficients.iter_mut().zip(alpha) {*total+=a;}
        }
        let squared=evaluate(&device,&queue,&program,JointContactDotInput::new(std::slice::from_ref(&residual),&residual)?)?[0];
        let reference=dot(&residual,&residual);
        let error=(squared-reference).abs()/reference.abs().max(f64::MIN_POSITIVE);
        if squared<0. || !error.is_finite() || error>1e-10 {return Err(format!("GPU squared norm admission failed: {error:e}").into());}
        norm_error=norm_error.max(error);
        let normalized=evaluate(&device,&queue,&normalize,JointContactDotInput::normalization(&residual)?)?;
        let norm=normalized[0];
        let norm_reference=reference.sqrt();
        if (norm-norm_reference).abs()/norm_reference>1e-10 {return Err("GPU norm mismatch".into());}
        if !norm.is_finite() || norm<=1e-12 {return Err("rank-deficient or nonfinite QR candidate".into());}
        coefficients.push(norm);triangular.push(coefficients);
        basis.push(normalized[1..].to_vec());
    }
    }
    let mut orthogonality=0f64;let mut reconstruction=0f64;
    for (j,q) in basis.iter().enumerate() {
        for (i,p) in basis.iter().enumerate() {
            orthogonality=orthogonality.max((dot(q,p)-if i==j {1.} else {0.}).abs());
        }
        for coordinate in 0..width {
            let actual=dot(&(0..=j).map(|i|basis[i][coordinate]).collect::<Vec<_>>(),&triangular[j]);
            reconstruction=reconstruction.max((actual-columns[j][coordinate]).abs());
        }
    }
    if orthogonality>1e-10 || reconstruction>1e-10 {return Err(format!("QR admission failed: orthogonality={orthogonality:e} reconstruction={reconstruction:e}").into());}
    println!("QR columns={count} coordinates={width} resident_mgs2={resident} orthogonality={orthogonality:e} reconstruction={reconstruction:e} squared_norm_error={norm_error:e} squared_norm_reference_checked={} gpu_normalization=true serial_norm_kernel={serial} cpu_orchestration=true physical_solution=false",!resident);
    Ok(())
}
