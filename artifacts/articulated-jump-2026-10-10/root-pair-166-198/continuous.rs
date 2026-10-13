use physics::hair::{CapsuleMotion,CapsuleSweep,CapsuleSweepOptions,sweep_capsules};
use std::io::Read;
fn n(r:&mut impl Read)->u32 {let mut b=[0;4];r.read_exact(&mut b).unwrap();u32::from_le_bytes(b)}
fn f(r:&mut impl Read)->f64 {let mut b=[0;8];r.read_exact(&mut b).unwrap();f64::from_le_bytes(b)}
fn points(r:&mut impl Read,n:usize)->Vec<[f64;3]> {(0..n).map(|_|std::array::from_fn(|_|f(r))).collect()}
fn trim(mut m:CapsuleMotion)->CapsuleMotion {for p in [&mut m.start,&mut m.end] {p[0]=std::array::from_fn(|k|p[0][k]*0.8+p[1][k]*0.2);}m}
fn main(){
 let mut r=std::io::Cursor::new(std::fs::read(std::env::args().nth(1).unwrap()).unwrap());let mut magic=[0;4];r.read_exact(&mut magic).unwrap();assert_eq!(&magic,b"VJR1");
 let count=n(&mut r);let radius=f(&mut r);let mut rods=Vec::new();
 for _ in 0..count {let count=n(&mut r) as usize;let old=points(&mut r,count);let now=points(&mut r,count);let delta=points(&mut r,count);points(&mut r,count-1);rods.push((old,now,delta));}
 for scale in [0.,1.] {
  let motion=|rod:usize,i:usize| {let (old,now,delta)=&rods[rod];CapsuleMotion {start:[old[i],old[i+1]],end:std::array::from_fn(|p|std::array::from_fn(|k|now[i+p][k]+scale*delta[i+p][k])),radius}};
  let mut counts=[0usize;4];
  for i in 0..rods[166].0.len()-1 {for j in 0..rods[198].0.len()-1 {
   let a=motion(166,i);let b=motion(198,j);let regions=if i==0&&j==0 {vec![(trim(a),b),(a,trim(b))]} else {vec![(a,b)]};
   for (a,b) in regions {let q=sweep_capsules(a,b,CapsuleSweepOptions {tolerance_m:5e-11,..Default::default()}).unwrap();let index=match q {CapsuleSweep::Clear=>0,CapsuleSweep::Approach{..}=>1,CapsuleSweep::InitialContact{..}=>2,CapsuleSweep::IterationLimit{..}=>3};counts[index]+=1;if index!=0 {println!("pair166198 scale={scale} segments={i},{j} query={q:?}");}}
  }}
  println!("pair166198 scale={scale} clear={} approach={} initial={} exhausted={}",counts[0],counts[1],counts[2],counts[3]);
 }
}
