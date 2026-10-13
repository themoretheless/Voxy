//! Standalone application path for admitted GPU liquid stepping, without a window.
use physics::liquid::{Liquid,Config,Material,Particle};
fn run()->Result<(),String> {
    let mut count=8usize;let mut steps=30usize;let mut reuse=true;
    let mut args=std::env::args().skip(1);
    while let Some(arg)=args.next() {match arg.as_str() {
        "--cold"=>reuse=false,
        "--particles"=>count=args.next().ok_or("missing particle count")?.parse().map_err(|_|"invalid particle count")?,
        "--steps"=>steps=args.next().ok_or("missing step count")?.parse().map_err(|_|"invalid step count")?,
        _=>return Err(format!("unknown option: {arg}")),
    }}
    if !(2..=256).contains(&count) || !(1..=10000).contains(&steps) {return Err("particles must be 2..256; steps must be 1..10000".into());}
    let instance=voxy_render::GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default())).map_err(|e|e.to_string())?;
    let info=adapter.get_info();
    if info.device_type==wgpu::DeviceType::Cpu {return Err("GPU adapter required".into());}
    let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).map_err(|e|e.to_string())?;
    let mut backend=pollster::block_on(voxy_demos::gpu_liquid::GpuLiquidBackend::new(&device,&queue)).map_err(|e|format!("{e:?}"))?;
    let particles=(0..count).map(|i|Particle {position:[i as f64*0.01,0.,0.],velocity:[0.01*i as f64,0.,0.],mass:0.1,material:0}).collect();
    let mut actual=Liquid::new(particles,vec![Material {rest_density:1.,sound_speed:1.,viscosity:0.01}],Config {smoothing_radius:1.,gravity:[0.;3],..Config::default()}).map_err(|e|format!("{e:?}"))?;
    let mut native=actual.clone();let mut admitted=0;let mut fallbacks=0;
    let mut admission_ns=0u128;
    let start=std::time::Instant::now();
    for _ in 0..steps {
        if !reuse {backend.clear_workspace();}
        let (_,stats)=actual.step_with_compute_backend(1e-5,None,&mut backend,1e-12,2e-11).map_err(|e|format!("{e:?}"))?;
        admission_ns+=stats.pipeline_admission_ns;admitted+=stats.pipeline_admitted;fallbacks+=stats.pipeline_native_fallbacks;
    }
    let gpu_ms=start.elapsed().as_secs_f64()*1000.;let start=std::time::Instant::now();
    for _ in 0..steps {native.step(1e-5,None).map_err(|e|format!("{e:?}"))?;}
    let native_ms=start.elapsed().as_secs_f64()*1000.;
    let mut position_error=0f64;let mut velocity_error=0f64;
    for (a,b) in actual.particles().iter().zip(native.particles()) {
        if a.mass!=b.mass || a.material!=b.material {return Err("particle identity mismatch".into());}
        for axis in 0..3 {position_error=position_error.max((a.position[axis]-b.position[axis]).abs());velocity_error=velocity_error.max((a.velocity[axis]-b.velocity[axis]).abs());}
    }
    let timing=backend.timings();
    println!("{}",serde_json::json!({"adapter":info.name,"reuse_storage":reuse,"steps":steps,"particles":count,"gpu_with_reference_admission_ms":gpu_ms,"native_ms":native_ms,
        "preparation_ms":timing.preparation_ns as f64/1e6,"encode_submit_ms":timing.encode_submit_ns as f64/1e6,
        "wait_ms":timing.wait_ns as f64/1e6,"decode_ms":timing.decode_ns as f64/1e6,"physical_reference_admission_ms":admission_ns as f64/1e6,
        "gpu_submissions":backend.stats().submissions,"decoded":backend.stats().decoded,"workspace_reuses":backend.stats().workspace_reuses,"workspace_creations":backend.stats().workspace_creations,"topology_creations":backend.stats().topology_creations,"topology_reuses":backend.stats().topology_reuses,"admitted":admitted,"native_fallbacks":fallbacks,
        "max_position_error":position_error,"max_velocity_error":velocity_error,"rendered_fps":null,"scope":"headless application liquid step"}));
    if fallbacks!=0 || admitted==0 || position_error>=1e-10 || velocity_error>=1e-9 {return Err(format!("qualification failed: {:?}",backend.last_error()));}
    Ok(())
}
fn main() {if let Err(error)=run() {eprintln!("{error}");std::process::exit(1);}}
