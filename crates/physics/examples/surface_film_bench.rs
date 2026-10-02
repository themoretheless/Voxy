//! Reproducible local CPU measurements; CSV on stdout, no performance guarantees.
use physics::surface_film::{Material, SurfaceFilm};
fn grid(n: usize) -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    let mut points = Vec::new();
    let mut cells = Vec::new();
    for y in 0..=n {
        for x in 0..=n {
            points.push([x as f64 * 0.1 / n as f64, y as f64 * 0.1 / n as f64, 0.]);
        }
    }
    for y in 0..n {
        for x in 0..n {
            let a = y * (n + 1) + x;
            let b = a + 1;
            let c = a + n + 1;
            let d = c + 1;
            cells.extend([[a, b, d], [a, d, c]]);
        }
    }
    (points, cells)
}
fn main() {
    println!("triangles,steps,simulated_s,elapsed_ms,ms_per_step,relative_volume_error");
    for n in [8, 16, 32] {
        let (points, cells) = grid(n);
        let count = cells.len();
        let mut film = SurfaceFilm::new(&points, cells, Material::default()).unwrap();
        film.deposit(count / 2, 1e-8).unwrap();
        let volume = film.total_volume();
        let now = std::time::Instant::now();
        for _ in 0..120 {
            film.step(1. / 120., [0., -9.81, 0.]).unwrap();
        }
        let ms = now.elapsed().as_secs_f64() * 1000.;
        let error = (film.total_volume() - volume).abs() / volume;
        assert!(error < 1e-10);
        println!("{count},120,1,{ms:.6},{:.6},{error:.6e}", ms / 120.);
    }
}
