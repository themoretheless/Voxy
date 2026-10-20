//! CPU-only strand simulation benchmark, with trajectory checksum.
use physics::strand::{Strand, StrandConfig};
use std::{hint::black_box, time::Instant};
fn main() {
    for count in [3usize, 32] {
        let mut strands: Vec<_> = (0..96)
            .map(|_| {
                Strand::new(
                    (0..count).map(|i| [0., i as f64 * 0.02, 0.]).collect(),
                    StrandConfig {
                        iterations: 8,
                        stiffness: 650.,
                        damping: 0.0001,
                        radius: 0.0003,
                    },
                )
                .unwrap()
            })
            .collect();
        let mut samples = Vec::new();
        for tick in 0..200 {
            let t = tick as f64 / 120.;
            let start = Instant::now();
            for strand in &mut strands {
                strand
                    .step(1. / 120., [t.sin() * 0.01, 0., 0.], [0.6, -9.81, 0.2], &[])
                    .unwrap();
            }
            if tick >= 20 {
                samples.push(start.elapsed().as_secs_f64() * 1e6);
            }
        }
        samples.sort_by(f64::total_cmp);
        let hash = strands
            .iter()
            .flat_map(|s| s.positions())
            .flat_map(|p| p.iter())
            .fold(0u64, |sum, v| sum.rotate_left(7) ^ v.to_bits());
        println!(
            "{count},{:.3},{:016x}",
            samples[samples.len() / 2],
            black_box(hash)
        );
    }
}
