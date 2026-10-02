//! Prescribed tangential traction smears an isothermal thin film; no contact solver.
use physics::surface_film::{FilmRheology, Material, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let viscosity = std::env::args()
        .nth(1)
        .map(|s| s.parse::<f64>())
        .transpose()?
        .unwrap_or(0.05);
    let stress = std::env::args()
        .nth(2)
        .map(|s| s.parse::<f64>())
        .transpose()?
        .unwrap_or(1.0);
    let hb = match std::env::args().nth(3).as_deref() {
        None | Some("newtonian") => false,
        Some("hb") => true,
        Some(_) => return Err("expected newtonian or hb as third argument".into()),
    };
    println!("herschel_bulkley={hb}");
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [0.01, 0.0, 0.0],
            [0.01, 0.0, 0.01],
            [0.0, 0.0, 0.01],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            density: 1000.0,
            viscosity,
            surface_tension: 0.0,
            wetting: 0.0,
        },
    )?;
    film.deposit(0, 5e-8)?;
    let mass = film.total_mass();
    println!("time,first_thickness,second_thickness,total_mass");
    for i in 0..=20 {
        let h = film.thickness();
        println!("{},{},{},{}", i as f64 * 0.1, h[0], h[1], film.total_mass());
        if i < 20 {
            if hb {
                film.step_with_rheology(
                    0.1,
                    [0.0; 3],
                    &[[-stress, 0.0, 0.0]; 2],
                    0.001,
                    FilmRheology {
                        consistency: viscosity,
                        flow_index: 0.5,
                        yield_stress: 0.5,
                        profile_samples: 64,
                    },
                )?;
            } else {
                film.step_with_surface_shear(0.1, [0.0; 3], &[[-stress, 0.0, 0.0]; 2], 0.001)?;
            }
        }
    }
    let error = (film.total_mass() - mass).abs();
    println!("mass_error={error}");
    if error > 1e-15 {
        return Err("smearing mass conservation failed".into());
    }
    Ok(())
}
