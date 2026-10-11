//! Hardware qualification of rectangular contact products, not a joint solve.
use voxy_render::{ComputeProgram,GraphicsOptions,JointContactDotInput,JOINT_CONTACT_DOT_SHADER};
fn main()->Result<(),Box<dyn std::error::Error>> {
    let mut cases=vec![("tail_and_mixed_sign".to_owned(),(0..67).map(|row|(0..129)
        .map(|i|if (i+row)%3==0 {0.} else {((i+row)%17) as f64/17.-0.5}).collect::<Vec<_>>()).collect::<Vec<_>>() )];
    cases.push(("rescale_intermediate_overflow".to_owned(),vec![vec![f64::MAX,f64::MAX],vec![-f64::MAX,-f64::MAX]]));
    cases.push(("qr_projection_update".to_owned(),(0..129).map(|i|(0..68).map(|j|((i*7+j*3)%31) as f64/31.-0.5).collect()).collect()));
    for path in std::env::args().skip(1) {
        let data=std::fs::read(&path)?;
        if data.len()<28 || &data[..4]!=b"VQC1" {return Err("invalid VQC1 contact fixture".into());}
        let rows=u32::from_le_bytes(data[4..8].try_into()?) as usize;
        let width=u32::from_le_bytes(data[8..12].try_into()?) as usize;
        if rows==0||rows>4096||width==0||width>65536 {return Err("fixture dimensions out of range".into());}
        let mut offset=28usize.checked_add(rows.checked_mul(16).ok_or("fixture size")?).ok_or("fixture size")?;
        let needed=rows.checked_mul(width).and_then(|n|n.checked_mul(8)).and_then(|n|offset.checked_add(n)).ok_or("fixture size")?;
        if data.len()<needed {return Err("truncated contact fixture".into());}
        let columns=(0..rows).map(|_|(0..width).map(|_| {
            let value=f64::from_le_bytes(data[offset..offset+8].try_into().unwrap());offset+=8;value
        }).collect::<Vec<_>>()).collect();
        cases.push((path,columns));
    }
    let instance=GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("ADAPTER {:?}",adapter.get_info());
    let (device,queue)=pollster::block_on(adapter.request_device(&Default::default()))?;
    let program=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_DOT_SHADER))?;
    for (name,columns) in cases {
        let coordinates:Vec<_>=if name=="rescale_intermediate_overflow" {vec![0.25,0.25]}
            else {(0..columns[0].len()).map(|i|((i%23) as f64-11.)/11.).collect()};
        let input=if name=="qr_projection_update" {
            let basis:Vec<Vec<f64>>=(1..coordinates.len()).map(|j|columns.iter().map(|c|c[j]).collect()).collect();
            let coefficients:Vec<_>=coordinates[1..].iter().map(|a|-a).collect();
            let vector:Vec<_>=columns.iter().map(|c|c[0]*coordinates[0]).collect();
            JointContactDotInput::projection(&basis,&coefficients,&vector)?
        } else {JointContactDotInput::new(&columns,&coordinates)?};
        let job=program.create_job(&device,input.bytes())?;
        let mut encoder=device.create_command_encoder(&Default::default());
        let (x,y,z)=input.dispatch();let dispatch=job.encode(&mut encoder,[x,y,z])?;
        queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let bytes=read.try_read()?.ok_or("GPU contact dot readback pending")?;
        let actual=input.decode(&bytes)?;assert_eq!(actual.len(),columns.len());
        let mut max_scaled_error=0f64;
        for (column,value) in columns.iter().zip(actual) {
            let mut sum=0f64;let mut correction=0f64;let mut scale=0f64;
            for (&a,&b) in column.iter().zip(&coordinates) {
                let product=a*b;let next=sum+product;
                correction+=if sum.abs()>=product.abs() {(sum-next)+product} else {(product-next)+sum};
                correction+=a.mul_add(b,-product);sum=next;scale+=product.abs();
            }
            let error=(value-(sum+correction)).abs()/scale.max(1e-30);
            if !error.is_finite()||error>1e-10 {return Err(format!("{name}: original dot mismatch {error}").into());}
            max_scaled_error=max_scaled_error.max(error);
        }
        println!("JOINT DOT case={name} rows={} width={} max_scaled_error={max_scaled_error:e} packing_error={:e}",columns.len(),coordinates.len(),input.packing_error());
    }
    Ok(())
}
