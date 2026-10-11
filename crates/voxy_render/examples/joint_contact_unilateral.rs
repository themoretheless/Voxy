//! Hybrid active-set qualification: GPU equality solves, CPU contact selection.
//! Original physical admission and native fallback are owned by physics.
use voxy_render::{ComputeProgram,GraphicsOptions,ResidentContactEqualityInput,JOINT_CONTACT_QR_SHADER,JOINT_CONTACT_EQUALITY_SHADER};
use physics::hair::{HairLinearSystem,HairResponseSystem};
use std::io::Read;
fn dot(a:&[f64],b:&[f64])->f64 {
    let mut sum=0f64;let mut correction=0f64;
    for (&a,&b) in a.iter().zip(b) {
        let p=a*b;let next=sum+p;
        correction+=if sum.abs()>=p.abs() {(sum-next)+p} else {(p-next)+sum};
        correction+=a.mul_add(b,-p);sum=next;
    }
    sum+correction
}

fn equality(device:&wgpu::Device,queue:&wgpu::Queue,qr:&ComputeProgram,solve:&ComputeProgram,columns:&[Vec<f64>],bounds:&[f64])->Result<(Vec<f64>,Vec<f64>),Box<dyn std::error::Error>> {
    let input=ResidentContactEqualityInput::new(columns,bounds)?;
    let mut job=qr.create_job(device,input.bytes())?;let mut encoder=device.create_command_encoder(&Default::default());
    for _ in 0..input.columns() {job.encode_step(&mut encoder,[1,1,1])?;}
    job.use_program(solve)?;job.encode_step(&mut encoder,[1,1,1])?;
    let dispatch=job.encode_readback(&mut encoder)?;
    queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes=read.try_read()?.ok_or("pending active equality readback")?;
    let output=input.decode(&bytes)?;Ok((output.coordinates,output.reactions))
}
fn integer(input:&mut std::io::Cursor<Vec<u8>>)->Result<usize,Box<dyn std::error::Error>> {let mut b=[0;4];input.read_exact(&mut b)?;Ok(u32::from_le_bytes(b) as usize)}
fn scalar(input:&mut std::io::Cursor<Vec<u8>>)->Result<f64,Box<dyn std::error::Error>> {let mut b=[0;8];input.read_exact(&mut b)?;Ok(f64::from_le_bytes(b))}
fn values(input:&mut std::io::Cursor<Vec<u8>>,n:usize)->Result<Vec<f64>,Box<dyn std::error::Error>> {(0..n).map(|_|scalar(input)).collect()}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_negative_equality_reaction_along_feasible_dual_segment() {
        let columns=vec![vec![10.,0.],vec![1.,1.]];let mut calls=0;
        let (state,reactions)=HairResponseSystem::solve_contact_coordinates_with_equality_accelerator(&columns,&[10.,3.],1e-12,|c,b,_| {
            calls+=1;
            let r=if c.len()==1 {vec![b[0]/dot(&c[0],&c[0])]}
                else {
                    let a=dot(&c[0],&c[0]);let d=dot(&c[1],&c[1]);let cross=dot(&c[0],&c[1]);let determinant=a*d-cross*cross;
                    vec![(d*b[0]-cross*b[1])/determinant,(a*b[1]-cross*b[0])/determinant]
                };
            let state=(0..2).map(|i|dot(&c.iter().map(|v|v[i]).collect::<Vec<_>>(),&r)).collect();
            Some((state,r))
        }).unwrap();
        assert_eq!(calls,3);assert_eq!(state,vec![1.5,1.5]);assert_eq!(reactions,vec![0.,1.5]);
        for i in 0..2 {let gap=dot(&columns[i],&state)-[10.,3.][i];assert!(if reactions[i]>0. {gap.abs()<=1e-12} else {gap>=-1e-12});}
    }
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    let path=std::env::args().nth(1).ok_or("expected complete VQC1 physical fixture")?;
    let mut input=std::io::Cursor::new(std::fs::read(path)?);let mut magic=[0;4];input.read_exact(&mut magic)?;
    if &magic!=b"VQC1" {return Err("invalid fixture magic".into());}
    let rows=integer(&mut input)?;let width=integer(&mut input)?;let systems=integer(&mut input)?;let _refinement=integer(&mut input)?;
    let tolerance=scalar(&mut input)?;
    if rows==0 || rows>512 || width==0 || width>65536 || systems==0 || systems>512 {return Err("fixture limits".into());}
    let bounds=values(&mut input,rows)?;let _effective=values(&mut input,rows)?;
    for _ in 0..rows {let _=values(&mut input,width)?;}
    let mut requests=Vec::new();
    for _ in 0..systems {
        let n=integer(&mut input)?;let band=integer(&mut input)?;let lo=integer(&mut input)?;let hi=integer(&mut input)?;
        if n==0 || n>65536 || band==0 || band>64 || lo>hi || hi>n {return Err("system limits".into());}
        let matrix=values(&mut input,n*band)?;let rhs=values(&mut input,n)?;
        let loads=(0..rows).map(|_|values(&mut input,n)).collect::<Result<Vec<_>,_>>()?;
        let request=HairResponseSystem {system:HairLinearSystem {band_width:band,matrix,rhs,active:lo..hi},loads};request.validate()?;requests.push(request);
    }
    if input.position() as usize!=input.get_ref().len() || requests.iter().map(|r|r.system.rhs.len()).sum::<usize>()!=width {return Err("fixture trailing data/width".into());}
    let instance=GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("HYBRID ADAPTER {:?}",adapter.get_info());let (device,queue)=pollster::block_on(adapter.request_device(&Default::default()))?;
    let qr=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER))?;
    let solve=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_EQUALITY_SHADER))?;
    let mut equality_calls=0;let mut candidate_calls=0;
    let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,tolerance,
        |columns,bounds,tolerance| {
            candidate_calls+=1;
            HairResponseSystem::solve_contact_coordinates_with_equality_accelerator(columns,bounds,tolerance,|columns,bounds,_| {
                equality_calls+=1;match equality(&device,&queue,&qr,&solve,columns,bounds) {
                    Ok(solution)=>Some(solution),Err(error)=>{eprintln!("GPU EQUALITY REJECTED {error}");None}
                }
            })
        })?;
    HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,tolerance)?;
    println!("HYBRID PHYSICAL systems={systems} rows={rows} coordinates={width} tolerance={tolerance:e} accelerated={accelerated} candidate_calls={candidate_calls} equality_calls={equality_calls} original_physical_admission=true cpu_active_selection=true");
    Ok(())
}
