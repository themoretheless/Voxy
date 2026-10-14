//! Apparent contact angle of a sessile drop against one container wall.
//!
//! A geometric estimator, not a wetting law: the particles of one material are
//! fitted by a least-squares sphere and the angle follows from the sphere's
//! signed centre height above the wall plane (`θ = acos(-d/R)`, so a hemisphere
//! gives 90°, a centre on the wall side gives an acute wetting angle, a barely
//! touching sphere gives 180°). It exists so that `WallAdhesion` and the
//! cohesion strength can be calibrated against a measured angle.
use super::{Container, Error, Liquid, finite};

/// One of the six `Container` faces: axis and whether it is the lower bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WallFace {
    pub axis: usize,
    pub lower: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactAngleEstimate {
    /// Apparent contact angle inside the liquid, radians in `[0, π]`.
    pub angle_rad: f64,
    pub sphere_center: [f64; 3],
    pub sphere_radius: f64,
    /// Radius of the wetted disc on the wall, m (zero when the drop only touches).
    pub contact_radius: f64,
    pub particles: usize,
    /// Root-mean-square radial misfit of the particle centres, m.
    pub rms_misfit: f64,
}

impl Liquid {
    /// Fits a sphere to the centres of every particle of `material` and reports the
    /// apparent angle against `face`. Particle centres sit `particle_radius` away from
    /// the wall, so the plane used is the collision plane shifted by that radius.
    /// # Errors
    /// Unknown material or face, fewer than four particles of the material, a
    /// degenerate (coplanar or collinear) fit, or a sphere that does not reach the wall.
    pub fn apparent_contact_angle(
        &self,
        material: usize,
        walls: Container,
        face: WallFace,
    ) -> Result<ContactAngleEstimate, Error> {
        if material >= self.materials.len() || face.axis > 2 {
            return Err(Error::InvalidMaterial);
        }
        let points: Vec<[f64; 3]> = self
            .particles
            .iter()
            .filter(|p| p.material == material)
            .map(|p| p.position)
            .collect();
        let (center, radius, rms_misfit) = fit_sphere(&points)?;
        // Plane of particle centres resting on the face, and the inward normal.
        let plane = if face.lower {
            walls.min[face.axis] + self.config.particle_radius
        } else {
            walls.max[face.axis] - self.config.particle_radius
        };
        let signed_height = if face.lower {
            center[face.axis] - plane
        } else {
            plane - center[face.axis]
        };
        let ratio = -signed_height / radius;
        if !(-1.0..=1.0).contains(&ratio) || !ratio.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let angle_rad = ratio.acos();
        let contact_radius = (radius * radius - signed_height * signed_height)
            .max(0.0)
            .sqrt();
        Ok(ContactAngleEstimate {
            angle_rad,
            sphere_center: center,
            sphere_radius: radius,
            contact_radius,
            particles: points.len(),
            rms_misfit,
        })
    }
}

/// Algebraic least-squares sphere: minimises `Σ (|x|² - 2 c·x + k)²` with
/// `k = |c|² - R²`, a linear problem in `(c, k)` solved by Gaussian elimination.
fn fit_sphere(points: &[[f64; 3]]) -> Result<([f64; 3], f64, f64), Error> {
    if points.len() < 4 || points.iter().any(|p| !finite(*p)) {
        return Err(Error::InvalidParticle);
    }
    // Shift to the centroid for conditioning.
    let n = points.len() as f64;
    let mut centroid = [0.0; 3];
    for p in points {
        for a in 0..3 {
            centroid[a] += p[a] / n;
        }
    }
    let shifted: Vec<[f64; 3]> = points
        .iter()
        .map(|p| [p[0] - centroid[0], p[1] - centroid[1], p[2] - centroid[2]])
        .collect();
    // Unknowns u = (2c_x, 2c_y, 2c_z, -k); row x · u = |x|².
    let mut normal = [[0.0; 5]; 4];
    for p in &shifted {
        let row = [p[0], p[1], p[2], 1.0];
        let rhs = p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
        for i in 0..4 {
            for j in 0..4 {
                normal[i][j] += row[i] * row[j];
            }
            normal[i][4] += row[i] * rhs;
        }
    }
    let scale = normal
        .iter()
        .flat_map(|r| r[..4].iter())
        .fold(0.0f64, |m, v| m.max(v.abs()));
    if scale == 0.0 {
        return Err(Error::NumericalFailure);
    }
    for col in 0..4 {
        let pivot = (col..4)
            .max_by(|a, b| normal[*a][col].abs().total_cmp(&normal[*b][col].abs()))
            .ok_or(Error::NumericalFailure)?;
        if normal[pivot][col].abs() <= 1e-12 * scale {
            return Err(Error::NumericalFailure);
        }
        normal.swap(col, pivot);
        for row in 0..4 {
            if row == col {
                continue;
            }
            let factor = normal[row][col] / normal[col][col];
            for j in col..5 {
                normal[row][j] -= factor * normal[col][j];
            }
        }
    }
    let u: [f64; 4] = std::array::from_fn(|i| normal[i][4] / normal[i][i]);
    let local = [0.5 * u[0], 0.5 * u[1], 0.5 * u[2]];
    let radius_sq = u[3] + local.iter().map(|c| c * c).sum::<f64>();
    if !radius_sq.is_finite() || radius_sq <= 0.0 {
        return Err(Error::NumericalFailure);
    }
    let radius = radius_sq.sqrt();
    let misfit = shifted
        .iter()
        .map(|p| {
            let d =
                ((p[0] - local[0]).powi(2) + (p[1] - local[1]).powi(2) + (p[2] - local[2]).powi(2))
                    .sqrt();
            (d - radius).powi(2)
        })
        .sum::<f64>()
        / n;
    let center = [
        local[0] + centroid[0],
        local[1] + centroid[1],
        local[2] + centroid[2],
    ];
    if !finite(center) {
        return Err(Error::NumericalFailure);
    }
    Ok((center, radius, misfit.sqrt()))
}
