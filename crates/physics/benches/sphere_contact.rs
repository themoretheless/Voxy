//! Controlled native-film contact workload; timings exclude setup and process startup.
use physics::surface_film::{Material, SurfaceFilm};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count: usize = std::env::args()
        .skip(1)
        .find(|arg| arg != "--bench")
        .map(|x| x.parse())
        .transpose()?
        .unwrap_or(1000);
    if count == 0 || count > 1_000_000 {
        return Err("expected 1..=1000000 queries per pass".into());
    }
    let mut points = Vec::new();
    for z in 0..=4 {
        for x in 0..=16 {
            points.push([-1.0 + x as f64 / 8.0, 0.0, -0.3 + z as f64 * 0.15]);
        }
    }
    let mut triangles = Vec::new();
    for z in 0..4 {
        for x in 0..16 {
            let a = z * 17 + x;
            triangles.extend([[a, a + 1, a + 18], [a, a + 18, a + 17]]);
        }
    }
    let film = SurfaceFilm::new(&points, triangles, Material::default())?;
    let queries: Vec<_> = (0..count)
        .map(|i| {
            let x = -0.8 + 1.6 * ((i * 37) % 997) as f64 / 997.0;
            let z = -0.2 + 0.4 * ((i * 71) % 991) as f64 / 991.0;
            match i % 4 {
                0 => ([x, 0.5, z], [x, -0.5, z]),
                1 => ([x, 0.8, z], [x, 0.7, z]),
                2 => ([1.02, 0.1, z], [1.02, -0.1, z]),
                _ => ([x, 0.2, z], [x, 0.2, z]),
            }
        })
        .collect();
    let mut times = Vec::new();
    for pass in 0..5 {
        let start = Instant::now();
        let mut checksum = 0.0;
        let mut hits = 0;
        for &(a, b) in &queries {
            if let Some(hit) =
                black_box(&film).first_sphere_hit(black_box(a), black_box(b), black_box(0.032))?
            {
                checksum += hit.time + hit.cell as f64 + hit.normal.iter().sum::<f64>();
                hits += 1;
            }
        }
        let elapsed = start.elapsed().as_secs_f64();
        times.push(elapsed);
        println!(
            "pass={pass},queries={count},hits={hits},seconds={elapsed},checksum={checksum:.17}"
        );
    }
    times.sort_by(f64::total_cmp);
    println!(
        "median_seconds={},median_queries_per_second={}",
        times[2],
        count as f64 / times[2]
    );
    Ok(())
}
