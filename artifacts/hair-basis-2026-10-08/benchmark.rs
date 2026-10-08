use glam::{Vec3,Quat,Mat3};
use std::time::Instant;
fn build<const BASIS:bool>() -> f64 {
 let started=Instant::now();let mut points=Vec::with_capacity(469*21*60*4);
 for i in 0..469 {for j in 0..21 {
  let frame=std::hint::black_box(Quat::from_rotation_y(i as f32*0.003)*Quat::from_rotation_x(j as f32*0.02));
  let p=Vec3::new(i as f32*0.0001,j as f32*0.01,0.1);
  let u=frame*Vec3::X;let v=frame*Vec3::Y;
  let matrix=Mat3::from_cols(u,v,frame*Vec3::Z);
  let radius=0.00030*(1.-0.6*j as f32/20.);
  let section=[u,v,-u,-v].map(|n|n*radius);
  for f in 0..60 {let offset=std::hint::black_box(Vec3::new(f as f32*0.00007,0.002,0.001));
   if BASIS {let center=p+matrix*offset;for n in section {points.push(center+n);}}
   else {let offset=frame*offset;for n in [u,v,-u,-v]{points.push(p+offset+n*radius);}}
  }
 }}
 std::hint::black_box(&points);started.elapsed().as_secs_f64()*1000.
}
fn main(){let mut old=Vec::new();let mut new=Vec::new();for run in 0..8 {let (a,b)=if run%2==0 {(build::<false>(),build::<true>())}else{let b=build::<true>();(build::<false>(),b)};if run>1 {old.push(a);new.push(b)}}old.sort_by(f64::total_cmp);new.sort_by(f64::total_cmp);println!("quaternion_ms={:?} basis_ms={:?} median_quaternion={} median_basis={}",old,new,(old[2]+old[3])/2.,(new[2]+new[3])/2.);}
