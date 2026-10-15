//! Diagnostic measurement of the production surface BRDF excerpt on real GPU.
//! Does not measure visibility, HDR attachment conversion or complete scene output.
use voxy_gpu_core::{ComputeProgram,GraphicsOptions};
fn main() {
 let source=include_str!("../../crates/voxy_render/src/surface_lighting.wgsl");
 let structures=source.lines().filter(|l|l.starts_with("struct Surface ")||l.starts_with("struct Light ")).collect::<Vec<_>>().join("\n");
 let begin=source.find("fn smith_lambda(").expect("production GGX helper missing");
 let end=source.find("@compute").expect("production entry point missing");
 let diffuse_line=source.lines().find(|l|l.contains("var brdf = diffuse /")).expect("audit must track production diffuse policy").trim();
 let specular_line=source.lines().find(|l|l.contains("brdf += specular_brdf(")).expect("audit must track production specular policy");
 let start=specular_line.find("brdf +=").unwrap();let finish=specular_line[start..].find(';').unwrap()+start+1;
 let shader=format!("{}\n@group(0) @binding(0) var<storage,read_write> results:array<u32>;\nvar<private> light:Light;\n{}\n@compute @workgroup_size(64) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {{let count=results[0];if id.x>=count {{return;}}light.camera=vec4<f32>(0.0,0.0,1.0,1.0);let c=(f32(id.x)+0.5)/f32(count);let direction=vec3<f32>(sqrt(max(0.0,1.0-c*c)),0.0,c);let surface=Surface(vec4<f32>(0.0,0.0,0.0,1.0),vec4<f32>(0.0,0.0,1.0,1.0));let diffuse=vec3<f32>(1.0);let f0=vec3<f32>(0.04);{} {} results[id.x+1u]=bitcast<u32>(brdf.x*c*6.283185307179586);}}",structures,&source[begin..end],diffuse_line,&specular_line[start..finish]);
 let instance=GraphicsOptions::default().create_instance();
 let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
 let info=adapter.get_info();assert_ne!(info.device_type,wgpu::DeviceType::Cpu);
 let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
 let program=pollster::block_on(ComputeProgram::new(&device,&shader)).unwrap();
 for samples in [1024u32,8192u32] {
  let mut words=vec![0u32;samples as usize+1];words[0]=samples;
  let bytes=words.iter().flat_map(|v|v.to_le_bytes()).collect::<Vec<_>>();
  let job=program.create_job(&device,&bytes).unwrap();
  let mut encoder=device.create_command_encoder(&Default::default());job.encode_step(&mut encoder,[samples.div_ceil(64),1,1]).unwrap();
  let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();
  device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None}).unwrap();
  let output=read.try_read().unwrap().unwrap();assert_eq!(output.len(),bytes.len());assert_eq!(&output[..4],&bytes[..4]);
  let energy=output[4..].chunks_exact(4).map(|v|f32::from_bits(u32::from_le_bytes(v.try_into().unwrap())) as f64).sum::<f64>()/f64::from(samples);
  assert!(energy.is_finite());assert!((energy-1.01230638252).abs()<1e-5,"GPU excerpt diverges from independent CPU quadrature: {energy}");
  println!("{{\"adapter\":{:?},\"backend\":\"{:?}\",\"samples\":{samples},\"hemispherical_reflectance\":{energy},\"energy_excess\":{},\"scope\":\"production BRDF excerpt; not complete renderer validation\"}}",info.name,info.backend,energy-1.0);
 }
}
