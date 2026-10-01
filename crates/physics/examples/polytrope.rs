//! Dimensionless n=1.5 stellar profile; pipe stdout to a CSV file.
use physics::astrophysics_star::lane_emden;
fn main() -> Result<(), String> {
    let profile = lane_emden(1.5, 0.01, 5.0, 1000).map_err(|e| format!("{e:?}"))?;
    println!("xi,theta,density_over_central,enclosed_mass");
    for point in profile.points {
        println!(
            "{},{},{},{}",
            point.xi,
            point.theta,
            point.theta.powf(profile.index),
            point.mass
        );
    }
    Ok(())
}
