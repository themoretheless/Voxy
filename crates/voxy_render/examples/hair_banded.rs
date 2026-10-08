//! Qualification against exported full-model native Cosserat matrices.
use voxy_render::{ComputeProgram, GraphicsOptions};
#[path="support/compensated_math.rs"] mod arithmetic;
fn main()->Result<(),Box<dyn std::error::Error>> {
    let path=std::env::args().nth(1).ok_or("native systems JSON path required")?;
    let systems:Vec<serde_json::Value>=serde_json::from_slice(&std::fs::read(path)?)?;
    let n=systems[0]["rhs"].as_array().ok_or("rhs")?.len();
    let first=systems[0]["first"].as_u64().ok_or("first")? as usize;
    let end=systems[0]["end"].as_u64().ok_or("end")? as usize;
    let mut words=vec![systems.len() as u32,n as u32,first as u32,end as u32];
    let args:Vec<_>=std::env::args().collect();
    let workgroup=if let Some(index)=args.iter().position(|v|v=="--workgroup") {
        args.get(index+1).ok_or("workgroup value missing")?.parse::<u32>()?
    } else {32};
    if ![1,4,8,16,32,64].contains(&workgroup) {return Err("unsupported workgroup size".into());}
    let shared=std::env::args().any(|v|v=="--shared");
    let compensated=std::env::args().any(|v|v=="--compensated");
    let single_refinement=args.iter().any(|v|v=="--single-refinement");
    let double_refinement=args.iter().any(|v|v=="--double-refinement");
    if single_refinement && double_refinement {return Err("conflicting refinement options".into());}
    if (single_refinement || double_refinement) && !compensated {return Err("refinement options require compensated arithmetic".into());}
    let arithmetic_source=include_str!("../src/hair_banded_compensated.wgsl");
    let arithmetic_source=if double_refinement {arithmetic_source.replace("const division_refinements:u32=1u;","const division_refinements:u32=2u;")} else {arithmetic_source.to_owned()};
    let width=if compensated {2} else {1};
    let equilibrate=std::env::args().any(|v|v=="--equilibrate");
    let mut references=Vec::new();
    for system in &systems {
        let matrix:Vec<f64>=system["matrix"].as_array().ok_or("matrix")?.iter().map(|v|v.as_f64().unwrap()).collect();
        let rhs:Vec<f64>=system["rhs"].as_array().ok_or("rhs")?.iter().map(|v|v.as_f64().unwrap()).collect();
        assert_eq!(matrix.len(),n*9);assert_eq!(rhs.len(),n);
        assert_eq!(system["band"],9);assert_eq!(system["first"],first);assert_eq!(system["end"],end);
        let diagonal:Vec<f64>=(0..n).map(|i| if equilibrate {1./matrix[i*9].sqrt()} else {1.}).collect();
        assert!(diagonal.iter().all(|v|v.is_finite() && *v>0.));
        for i in 0..n { for offset in 0..9 {
            let value=if offset<=i {matrix[i*9+offset]*diagonal[i]*diagonal[i-offset]} else {0.};
            words.push((value as f32).to_bits());
            if compensated { words.push(((value-value as f32 as f64) as f32).to_bits()); }
        } }
        for (v,d) in rhs.iter().zip(&diagonal) {
            let value=v*d; words.push((value as f32).to_bits());
            if compensated { words.push(((value-value as f32 as f64) as f32).to_bits()); }
        }
        words.push(0);
        let mut a=matrix.clone();let mut x=rhs.clone();
        for i in first..end { let start=i.saturating_sub(8).max(first);
            for j in start..=i { let mut sum=a[i*9+i-j];
                for k in start.max(j.saturating_sub(8))..j { sum-=a[i*9+i-k]*a[j*9+j-k]; }
                a[i*9+i-j]=if i==j {sum.max(1e-30).sqrt()} else {sum/a[j*9]};
            }
        }
        for i in first..end { for j in i.saturating_sub(8).max(first)..i {x[i]-=a[i*9+i-j]*x[j];} x[i]/=a[i*9]; }
        for i in (first..end).rev() {for j in i+1..(i+9).min(end) {x[i]-=a[j*9+j-i]*x[j];} x[i]/=a[i*9];}
        references.push((matrix,rhs,x,diagonal));
    }
    let admitted=if compensated && equilibrate {
        let rows:Vec<_>=references.iter().map(|(matrix,rhs,_,_)|voxy_render::BandedSystem {matrix,rhs}).collect();
        let input=voxy_render::BandedSolveInput::new(&rows,first..end)?;
        assert_eq!(input.bytes(),bytemuck::cast_slice::<u32,u8>(&words));
        Some(input)
    } else {None};
    let instance=GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("ADAPTER {:?}",adapter.get_info());
    let timing_features=wgpu::Features::TIMESTAMP_QUERY;
    let timing_supported=adapter.features().contains(timing_features);
    let (device,queue)=pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features:if timing_supported {timing_features} else {wgpu::Features::empty()},
        ..Default::default()
    }))?;
    if compensated {
        arithmetic::qualify_source(&device,&queue,&arithmetic_source)?;
        let prefix=arithmetic_source.split("@compute").next().unwrap();
        let probe=format!("{prefix} @compute @workgroup_size(1) fn cs_main() {{save(0u,add(vec2<f32>(100000000.0,0.0),vec2<f32>(1.0,0.0))); save(2u,mul(vec2<f32>(1.0001,0.0),vec2<f32>(1.0001,0.0)));}}");
        let probe_program=pollster::block_on(ComputeProgram::new(&device,&probe))?;
        let probe_job=probe_program.create_job(&device,bytemuck::cast_slice(&[0u32;4]))?;
        let mut encoder=device.create_command_encoder(&Default::default());
        let dispatch=probe_job.encode(&mut encoder,[1,1,1])?;queue.submit([encoder.finish()]);
        let mut read=dispatch.begin_read();device.poll(wgpu::PollType::wait_indefinitely())?;
        let bytes=read.try_read()?.ok_or("arithmetic probe pending")?;
        let result:&[f32]=bytemuck::cast_slice(&bytes);
        println!("COMPENSATED ARITHMETIC PROBE {:?}",result);
        let product=(1.0001f32 as f64).powi(2);
        let product_error=((result[2] as f64 + result[3] as f64)-product).abs();
        if result[0]!=100000000. || result[1]!=1. || product_error>1e-13 {return Err("backend does not preserve compensated addition; do not integrate".into());}
    }
    let source=if compensated {arithmetic_source.as_str()} else {include_str!("../src/hair_banded.wgsl")};
    if shared && (!compensated || n>126 || workgroup!=32) {return Err("shared kernel requires compensated arithmetic, <=126 DOFs and group 32".into());}
    let source=if shared {format!("{}{}",source.split("fn load(").next().unwrap(),include_str!("../src/hair_banded_shared.wgsl"))} else {source.to_owned()};
    let dispatch_count=if shared {systems.len() as u32} else {(systems.len() as u32).div_ceil(workgroup)};
    let source=source.replace("@workgroup_size(32)",&format!("@workgroup_size({workgroup})"));
    let program=pollster::block_on(ComputeProgram::new(&device,&source))?;
    let job=program.create_job(&device,bytemuck::cast_slice(&words))?;
    if timing_supported {
        use wgpu::util::DeviceExt;
        let pristine=device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:Some("immutable native Cosserat input"),contents:bytemuck::cast_slice(&words),usage:wgpu::BufferUsages::COPY_SRC
        });
        let query=device.create_query_set(&wgpu::QuerySetDescriptor {label:Some("Cosserat GPU time"),ty:wgpu::QueryType::Timestamp,count:18});
        let resolve=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:144,usage:wgpu::BufferUsages::QUERY_RESOLVE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
        let readback=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:144,usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});
        let mut encoder=device.create_command_encoder(&Default::default());
        for repeat in 0..9 {
            encoder.copy_buffer_to_buffer(&pristine,0,job.buffer(),0,words.len() as u64*4);
            job.encode_step_with_timestamps(&mut encoder,[dispatch_count,1,1],wgpu::ComputePassTimestampWrites {
                query_set:&query,beginning_of_pass_write_index:Some(repeat*2),end_of_pass_write_index:Some(repeat*2+1)
            })?;
        }
        encoder.resolve_query_set(&query,0..18,&resolve,0);
        encoder.copy_buffer_to_buffer(&resolve,0,&readback,0,144);
        queue.submit([encoder.finish()]);
        let (tx,rx)=std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read,move |r|{let _=tx.send(r);});
        device.poll(wgpu::PollType::wait_indefinitely())?;rx.recv()??;
        let mapped=readback.slice(..).get_mapped_range()?;
        let times:&[u64]=bytemuck::cast_slice(&mapped);
        let mut samples=Vec::new();
        for pair in times.chunks_exact(2).skip(1) {
            if let Some(ticks)=pair[1].checked_sub(pair[0]).filter(|v|*v>0) {
                samples.push(ticks as f64 * queue.get_timestamp_period() as f64 / 1e6);
            } else { eprintln!("GPU timestamp sample unavailable: begin={} end={}",pair[0],pair[1]); }
        }
        drop(mapped);readback.unmap();
        samples.sort_by(f64::total_cmp);
        if samples.len()>=3 {
            println!("GPU SOLVE TIMING workgroup={} systems={} median_ms={} samples={} single_submission=true excludes_cpu_assembly_upload_readback=true",workgroup,systems.len(),samples[samples.len()/2],samples.len());
        } else { println!("GPU SOLVE TIMING unavailable: insufficient valid timestamp samples"); }
        queue.write_buffer(job.buffer(),0,bytemuck::cast_slice(&words));
    } else {println!("GPU SOLVE TIMING unavailable: timestamp query feature missing");}
    let mut encoder=device.create_command_encoder(&Default::default());
    let dispatch=job.encode(&mut encoder,[dispatch_count,1,1])?;
    queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes=read.try_read()?.ok_or("pending")?;
    if let Some(index)=args.iter().position(|v|v=="--solution-output") {
        let path=std::path::Path::new(args.get(index+1).ok_or("solution output path missing")?);
        if !path.is_absolute() {return Err("solution output must use an absolute path".into());}
        std::fs::write(path,&bytes)?;
    }
    if let Some(input)=&admitted {
        let corrections=input.decode_checked(&bytes,1e-4)?;
        assert_eq!(corrections.len(),systems.len());
        println!("PASS: admitted renderer API, complete batch residual and status validation");
    }
    let output:&[u32]=bytemuck::cast_slice(&bytes);
    let (mut position_error,mut angle_error,mut residual,mut failures)=(0f64,0f64,0f64,0usize);
    for (index,(a,b,reference,diagonal)) in references.iter().enumerate() {
        let base=4+index*(n*10*width+1);if output[base+n*10*width]!=0 {failures+=1;continue;}
        let x:Vec<f64>=output[base+n*9*width..base+n*10*width].chunks_exact(width).zip(diagonal).map(|(v,d)|v.iter().map(|word|f32::from_bits(*word) as f64).sum::<f64>() * d).collect();
        for i in first..end {
            let error=(x[i]-reference[i]).abs();
            if i%6<3 {position_error=position_error.max(error);} else {angle_error=angle_error.max(error);}
            let mut ax=a[i*9]*x[i];let mut scale=(a[i*9]*x[i]).abs()+b[i].abs();
            for j in i.saturating_sub(8).max(first)..i {let term=a[i*9+i-j]*x[j];ax+=term;scale+=term.abs();}
            for j in i+1..(i+9).min(end) {let term=a[j*9+j-i]*x[j];ax+=term;scale+=term.abs();}
            let r=(ax-b[i]).abs()/scale.max(1e-30);residual=residual.max(r);
            if !x[i].is_finite() {failures+=1;}
        }
    }
    println!("GPU BANDED compensated={} equilibrated={} systems={} failed={} position_error_m={} angle_error_rad={} component_backward_error={}",compensated,equilibrate,systems.len(),failures,position_error,angle_error,residual);
    if failures!=0 || position_error>1e-6 || angle_error>1e-4 || residual>1e-4 {return Err("native Cosserat accuracy gate failed; do not integrate".into());}
    println!("PASS: native full-model linear-system gate; nonlinear/contact integration remains CPU");Ok(())
}
