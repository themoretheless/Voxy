use std::{hint::black_box, time::Instant};
fn build(mesh_base: u32) -> Vec<u32> {
    let mut result = Vec::with_capacity(469 * 60 * 20 * 24);
    let stride = 60 * 4;
    for rod in 0..469 {
        let base = mesh_base + rod * 21 * stride;
        for fibre in 0..60 {
            for ring in 0..20 {
                for side in 0..4 {
                    let a = base + ring * stride + fibre * 4 + side;
                    let b = base + ring * stride + fibre * 4 + (side + 1) % 4;
                    result.extend([a,b,b+stride,a,b+stride,a+stride]);
                }
            }
        }
    }
    result
}
fn main() {
    let cache = build(0);
    let mut old = Vec::new(); let mut cached = Vec::new();
    for run in 0..10 {
        let modes = if run % 2 == 0 { [false,true] } else { [true,false] };
        for mode in modes {
            let start = Instant::now();
            let result = if mode {
                cache.iter().map(|index| index + 60_866).collect::<Vec<_>>()
            } else {
                build(black_box(60_866))
            };
            let elapsed = start.elapsed().as_secs_f64()*1000.;
            assert_eq!(result.len(), cache.len());
            black_box(&result);
            if run >= 2 { if mode {cached.push(elapsed)} else {old.push(elapsed)} }
        }
    }
    old.sort_by(f64::total_cmp); cached.sort_by(f64::total_cmp);
    println!("indices={} rebuild_ms={:.3} cached_offset_ms={:.3}",cache.len(),(old[3]+old[4])/2.,(cached[3]+cached[4])/2.);
}
