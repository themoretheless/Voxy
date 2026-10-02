//! Fit a passive tensile material using measured CSV and independent validation.
use physics::biomechanics::{TensilePoint, fit_tensile, tensile_error};
fn read(path: &str) -> Result<Vec<TensilePoint>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut lines = content.lines();
    if lines.next() != Some("stretch,nominal_stress_pa") {
        return Err("expected CSV header stretch,nominal_stress_pa".into());
    }
    let mut points = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let (a, b) = line.split_once(',').ok_or("expected two CSV columns")?;
        points.push(TensilePoint {
            stretch: a.trim().parse()?,
            nominal_stress_pa: b.trim().parse()?,
        });
    }
    Ok(points)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: tissue_calibrate TRAIN.csv HELD_OUT.csv BULK_PA FIBER_EXPONENT".into());
    }
    if std::fs::canonicalize(&args[0])? == std::fs::canonicalize(&args[1])? {
        return Err("training and validation must be separate files".into());
    }
    let train = read(&args[0])?;
    let validation = read(&args[1])?;
    let material = fit_tensile(&train, args[2].parse()?, args[3].parse()?)?;
    let fit = tensile_error(&material, &train)?;
    let held_out = tensile_error(&material, &validation)?;
    println!(
        "shear_pa={:.9e} bulk_pa={:.9e} fiber_stiffness_pa={:.9e} exponent={}",
        material.shear_pa,
        material.bulk_pa,
        material.fibers[0].stiffness_pa,
        material.fibers[0].exponent
    );
    println!(
        "training_rmse_pa={:.9e} validation_rmse_pa={:.9e} validation_max_error_pa={:.9e}",
        fit.rmse_pa, held_out.rmse_pa, held_out.max_error_pa
    );
    println!(
        "Scope: passive incompressible uniaxial tension only. Bulk modulus and exponent supplied; organ physiology not validated."
    );
    Ok(())
}
