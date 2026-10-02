use physics::soft_body::{SoftBody, Sphere};
fn main() -> Result<(), &'static str> {
    let mut body = SoftBody::new(
        vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![1., 0., 0., 0.],
        vec![[0, 1, 2, 3]],
    )?;
    let rest = body.volume();
    for frame in 0..720 {
        let radius = if frame < 360 {
            0.05 + 0.12 * (std::f64::consts::PI * frame as f64 / 360.).sin()
        } else {
            0.05
        };
        let spheres = [
            Sphere {
                center: [-0.03, 0., 0.],
                radius,
            },
            Sphere {
                center: [1.03, 0., 0.],
                radius,
            },
        ];
        body.step(
            1. / 240.,
            [0.; 3],
            1e-6,
            if frame < 360 { &spheres } else { &[] },
        )?;
        if frame % 120 == 0 {
            println!(
                "frame={frame} volume_ratio={:.5} vertices={:?}",
                body.volume() / rest,
                body.positions()
            );
        }
    }
    Ok(())
}
