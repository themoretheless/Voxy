//! Turbidity of a grain suspension: geometric extinction `Q π r²` per grain
//! over the carrier volume, binned onto a grid the renderer already reads.
//! `Q` is an explicit dimensionless efficiency; no material calibration is inferred.

use super::Particle;
use crate::liquid::{DropletExtinctionGrid, Error};

/// Mean extinction coefficient (1/m) of grains sharing a carrier volume.
/// # Errors
/// Nonpositive volume or invalid efficiency.
pub fn turbidity_extinction_m_inverse(
    grains: &[Particle],
    carrier_volume_m3: f64,
    extinction_efficiency: f64,
) -> Result<f64, &'static str> {
    if !carrier_volume_m3.is_finite()
        || carrier_volume_m3 <= 0.
        || !extinction_efficiency.is_finite()
        || extinction_efficiency < 0.
    {
        return Err("invalid turbidity settings");
    }
    let cross_section: f64 = grains
        .iter()
        .map(|g| std::f64::consts::PI * g.radius_m() * g.radius_m())
        .sum();
    let extinction = extinction_efficiency * cross_section / carrier_volume_m3;
    if extinction.is_finite() {
        Ok(extinction)
    } else {
        Err("turbidity overflow")
    }
}

/// Bins grain cross-sections per containing cell, like `droplet_extinction_grid`.
/// Grains outside the grid are counted in the returned figure.
/// # Errors
/// Invalid grid or efficiency.
pub fn suspension_extinction_grid(
    grains: &[Particle],
    origin: [f64; 3],
    spacing: [f64; 3],
    shape: [usize; 3],
    extinction_efficiency: f64,
) -> Result<(DropletExtinctionGrid, usize), Error> {
    if !extinction_efficiency.is_finite() || extinction_efficiency < 0. {
        return Err(Error::InvalidConfig);
    }
    let count = shape
        .iter()
        .try_fold(1usize, |a, &b| a.checked_mul(b))
        .ok_or(Error::InvalidConfig)?;
    if count == 0 || spacing.iter().any(|s| !s.is_finite() || *s <= 0.) {
        return Err(Error::InvalidConfig);
    }
    let volume = spacing.iter().product::<f64>();
    let mut field = vec![0.; count];
    let mut outside = 0;
    for grain in grains {
        let p = grain.position_m();
        let mut ijk = [0usize; 3];
        let mut inside = true;
        for k in 0..3 {
            let upper = origin[k] + spacing[k] * shape[k] as f64;
            if p[k] < origin[k] || p[k] >= upper {
                inside = false;
                break;
            }
            ijk[k] = (((p[k] - origin[k]) / spacing[k]).floor() as usize).min(shape[k] - 1);
        }
        if !inside {
            outside += 1;
            continue;
        }
        let cell = ijk[0] + shape[0] * (ijk[1] + shape[1] * ijk[2]);
        field[cell] +=
            extinction_efficiency * std::f64::consts::PI * grain.radius_m().powi(2) / volume;
    }
    Ok((
        DropletExtinctionGrid::from_field(origin, spacing, shape, field)?,
        outside,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grains(n: usize, radius: f64, x: f64) -> Vec<Particle> {
        (0..n)
            .map(|i| {
                Particle::new(radius, 2500., [x, 0.1 + 0.0001 * i as f64, 0.1], [0.; 3]).unwrap()
            })
            .collect()
    }

    #[test]
    fn beer_lambert_transmittance_matches_the_analytic_optical_depth() {
        let radius = 1e-4;
        let grains = grains(1000, radius, 0.25);
        let volume = 0.5 * 0.5 * 0.5;
        let extinction = turbidity_extinction_m_inverse(&grains, volume, 2.).unwrap();
        let expected = 2. * 1000. * std::f64::consts::PI * radius * radius / volume;
        assert!((extinction - expected).abs() <= 1e-12 * expected);
        let (grid, outside) =
            suspension_extinction_grid(&grains, [0.; 3], [0.5; 3], [2, 1, 1], 2.).unwrap();
        assert_eq!(outside, 0);
        assert!((grid.extinction_m_inverse()[0] - expected).abs() <= 1e-12 * expected);
        assert_eq!(grid.extinction_m_inverse()[1], 0.);
        let depth = grid
            .optical_depth_segment([0., 0.25, 0.25], [1., 0.25, 0.25])
            .unwrap();
        assert!((depth - expected * 0.5).abs() <= 1e-12 * expected);
        let transmittance = grid
            .transmittance_segment([0., 0.25, 0.25], [1., 0.25, 0.25])
            .unwrap();
        assert!((transmittance - (-expected * 0.5).exp()).abs() <= 1e-12);
    }

    #[test]
    fn outside_grains_are_counted_and_settings_are_validated() {
        let grains = grains(3, 1e-4, 5.);
        let (grid, outside) =
            suspension_extinction_grid(&grains, [0.; 3], [1.; 3], [1, 1, 1], 1.).unwrap();
        assert_eq!(outside, 3);
        assert_eq!(grid.extinction_m_inverse(), &[0.]);
        assert!(turbidity_extinction_m_inverse(&grains, 0., 1.).is_err());
        assert!(turbidity_extinction_m_inverse(&grains, 1., -1.).is_err());
        assert!(suspension_extinction_grid(&grains, [0.; 3], [0.; 3], [1, 1, 1], 1.).is_err());
        assert!(suspension_extinction_grid(&grains, [0.; 3], [1.; 3], [0, 1, 1], 1.).is_err());
    }
}
