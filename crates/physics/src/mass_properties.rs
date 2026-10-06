//! Explicit additive mass distributions, independent of overlapping contact geometry.
type Vector = [f64; 3];
type Matrix = [[f64; 3]; 3];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniformBoxMass {
    pub mass: f64,
    pub center: Vector,
    /// Affine half-edge vectors in the supplied common frame.
    pub half_edges: [Vector; 3],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassProperties {
    pub mass: f64,
    pub center: Vector,
    /// Full tensor about the center of mass, in the supplied common frame.
    pub inertia: Matrix,
    /// Descending positive moments, with physical triangle inequalities.
    pub principal_moments: Vector,
    /// Right-handed orthonormal principal axes in columns.
    pub principal_axes: Matrix,
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: Vector, b: Vector) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn finite(v: Vector) -> bool {
    v.iter().all(|v| v.is_finite())
}
impl UniformBoxMass {
    fn validate(self) -> Result<(), &'static str> {
        if !self.mass.is_finite()
            || self.mass <= 0.
            || !finite(self.center)
            || self.half_edges.iter().any(|v| !finite(*v))
        {
            return Err("invalid box mass distribution");
        }
        let mut edges = self.half_edges;
        for edge in &mut edges {
            let scale = edge.iter().map(|v| v.abs()).fold(0., f64::max);
            if scale == 0. {
                return Err("degenerate mass volume");
            }
            *edge = edge.map(|v| v / scale);
        }
        let determinant = dot(edges[0], cross(edges[1], edges[2]));
        if !determinant.is_finite() || determinant == 0. {
            return Err("degenerate mass volume");
        }
        Ok(())
    }
}
/// Add declared mass components, never infer mass from collider union/overlap.
/// Each component is a homogeneous affine volume. Overlapping components are
/// explicitly additive physical constituents, not Boolean-unioned geometry.
/// # Errors
/// Empty/excess parts, invalid mass/volume, overflow or unresolved principal frame.
pub fn from_boxes(
    parts: &[UniformBoxMass],
    max_parts: usize,
) -> Result<MassProperties, &'static str> {
    if parts.is_empty() || parts.len() > max_parts {
        return Err("mass distribution budget");
    }
    let mut mass = 0.;
    for part in parts {
        part.validate()?;
        mass += part.mass;
    }
    if !mass.is_finite() || mass <= 0. {
        return Err("mass sum overflow");
    }
    let origin = parts[0].center;
    let mut relative = [0.; 3];
    for part in parts {
        for k in 0..3 {
            relative[k] += (part.mass / mass) * (part.center[k] - origin[k]);
        }
    }
    let center = std::array::from_fn(|k| origin[k] + relative[k]);
    if !finite(center) {
        return Err("mass center overflow");
    }
    let mut inertia = [[0.; 3]; 3];
    for part in parts {
        let weight = part.mass.sqrt();
        let intrinsic_weight = weight / 3_f64.sqrt();
        let mut terms = [[0.; 3]; 4];
        for (term, edge) in terms.iter_mut().zip(part.half_edges) {
            *term = edge.map(|v| intrinsic_weight * v);
        }
        // Subtract in the origin-relative frame to preserve small center offsets.
        terms[3] = std::array::from_fn(|k| weight * ((part.center[k] - origin[k]) - relative[k]));
        for term in terms {
            if !finite(term) {
                return Err("inertia scale overflow");
            }
            for i in 0..3 {
                inertia[i][i] += (0..3)
                    .filter(|&k| k != i)
                    .map(|k| term[k] * term[k])
                    .sum::<f64>();
                for j in i + 1..3 {
                    inertia[i][j] -= term[i] * term[j];
                    inertia[j][i] = inertia[i][j];
                }
            }
        }
    }
    if inertia.iter().flatten().any(|v| !v.is_finite()) {
        return Err("inertia tensor overflow");
    }
    let scale = inertia.iter().flatten().map(|v| v.abs()).fold(0., f64::max);
    if scale == 0. {
        return Err("unresolved inertia");
    }
    let normalized = inertia.map(|row| row.map(|v| v / scale));
    let (moments, mut axes) = crate::symmetric_eigen::normalized(normalized)?;
    // Jacobi ordering may reflect the basis; Spin needs a proper rotation.
    let columns = std::array::from_fn::<_, 3, _>(|k| [axes[0][k], axes[1][k], axes[2][k]]);
    if dot(columns[0], cross(columns[1], columns[2])) < 0. {
        for row in &mut axes {
            row[2] = -row[2];
        }
    }
    let moments = moments.map(|v| v * scale);
    if moments.iter().any(|v| !v.is_finite() || *v <= 0.)
        || (0..3).any(|k| moments[k] > moments[(k + 1) % 3] + moments[(k + 2) % 3])
    {
        return Err("nonphysical or unresolved principal inertia");
    }
    // Admission checks reconstruction and orthonormality, not convergence intent.
    for i in 0..3 {
        for j in 0..3 {
            let reconstructed: f64 = (0..3)
                .map(|k| axes[i][k] * (moments[k] / scale) * axes[j][k])
                .sum();
            let orthogonal: f64 = (0..3).map(|k| axes[k][i] * axes[k][j]).sum();
            if (reconstructed - normalized[i][j]).abs() > 2e-12
                || (orthogonal - if i == j { 1. } else { 0. }).abs() > 2e-12
            {
                return Err("unresolved principal frame");
            }
        }
    }
    Ok(MassProperties {
        mass,
        center,
        inertia,
        principal_moments: moments,
        principal_axes: axes,
    })
}
