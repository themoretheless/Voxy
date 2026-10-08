use std::collections::HashMap;
use std::time::Instant;
fn main(){
 let points:Vec<[u32;3]>=(0..1_000_000u32).map(|i|{let j=i/2;[(j as f32*0.0001).to_bits(),((j%997) as f32*0.001).to_bits(),((j%101) as f32*0.003).to_bits()]}).collect();
 let mut array_times=Vec::new(); let mut packed_times=Vec::new();
 for run in 0..8 { for packed in if run%2==0 {[false,true]}else{[true,false]} {
 let begin=Instant::now();
 let count=if packed {let mut map=HashMap::with_capacity(points.len());for p in &points {let key=(u64::from(p[0])<<32|u64::from(p[1]),p[2]);let next=map.len();map.entry(key).or_insert(next);} std::hint::black_box(map.len())}
 else {let mut map=HashMap::with_capacity(points.len());for p in &points {let next=map.len();map.entry(*p).or_insert(next);}std::hint::black_box(map.len())};
 assert_eq!(count,500_000);let elapsed=begin.elapsed().as_secs_f64()*1000.;
 if run>1 {if packed {packed_times.push(elapsed)}else{array_times.push(elapsed)}}
 }}
 array_times.sort_by(f64::total_cmp);packed_times.sort_by(f64::total_cmp);
 println!("array_ms={:?} packed_ms={:?} median_array={} median_packed={}",array_times,packed_times,(array_times[2]+array_times[3])/2.,(packed_times[2]+packed_times[3])/2.);
}
