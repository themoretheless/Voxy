//! Fragment connectivity and exact consistent-mass kinematic diagnostics.
use super::{QuadraticBody, Vec3, dot};
#[derive(Clone, Debug)]
pub struct QuadraticFragment {
    pub nodes: Vec<usize>,
    pub mass_kg: f64,
    pub center_m: Vec3,
    pub momentum_kg_m_s: Vec3,
    pub velocity_m_s: Vec3,
    pub angular_momentum_kg_m2_s: Vec3,
    pub kinetic_j: f64,
}
impl QuadraticBody {
    /// Cell connectivity plus any surviving cohesive integration-point bond.
    /// Fully broken faces do not reconnect fragments on compressive closure.
    /// This does not convert deformable pieces into rigid bodies.
    #[must_use]
    pub fn fragment_nodes(&self) -> Vec<Vec<usize>> {
        fn root(parents: &mut [usize], mut node: usize) -> usize {
            while parents[node] != node {
                parents[node] = parents[parents[node]];
                node = parents[node];
            }
            node
        }
        let mut parents: Vec<_> = (0..self.positions.len()).collect();
        let mut join = |a, b| {
            let a = root(&mut parents, a);
            let b = root(&mut parents, b);
            parents[a.max(b)] = a.min(b);
        };
        for cell in &self.cells {
            for &node in &cell.nodes[1..] {
                join(cell.nodes[0], node);
            }
        }
        for face in &self.interfaces {
            if !face.is_fully_broken() {
                let (minus, plus) = face.sides();
                for node in minus.into_iter().chain(plus) {
                    join(minus[0], node);
                }
            }
        }
        let mut groups = std::collections::BTreeMap::<usize, Vec<usize>>::new();
        for node in 0..parents.len() {
            groups
                .entry(root(&mut parents, node))
                .or_default()
                .push(node);
        }
        groups.into_values().collect()
    }
}
pub(super) fn summarize(
    nodes: Vec<usize>,
    mass: &[Vec<f64>],
    positions: &[Vec3],
    velocities: &[Vec3],
) -> Result<QuadraticFragment, &'static str> {
    let mut included = vec![false; positions.len()];
    for &node in &nodes {
        included[node] = true;
    }
    let mut result = QuadraticFragment {
        nodes,
        mass_kg: 0.,
        center_m: [0.; 3],
        momentum_kg_m_s: [0.; 3],
        velocity_m_s: [0.; 3],
        angular_momentum_kg_m2_s: [0.; 3],
        kinetic_j: 0.,
    };
    for &i in &result.nodes {
        // Negative corner row sums are valid for T10. Do not clamp/lump them.
        let weight: f64 = result.nodes.iter().map(|&j| mass[i][j]).sum();
        result.mass_kg += weight;
        for axis in 0..3 {
            result.center_m[axis] += weight * positions[i][axis];
            result.momentum_kg_m_s[axis] += weight * velocities[i][axis];
        }
        for (j, &value) in mass[i].iter().enumerate() {
            if !included[j] {
                if value != 0. {
                    return Err("quadratic fragment mass couples distinct components");
                }
                continue;
            }
            result.kinetic_j += 0.5 * value * dot(velocities[i], velocities[j]);
            let angular = crate::plasticity::mesh::cross(positions[i], velocities[j]);
            for (axis, &component) in angular.iter().enumerate() {
                result.angular_momentum_kg_m2_s[axis] += value * component;
            }
        }
    }
    if !result.mass_kg.is_finite() || result.mass_kg <= 0. {
        return Err("invalid quadratic fragment mass");
    }
    result.center_m = result.center_m.map(|v| v / result.mass_kg);
    result.velocity_m_s = result.momentum_kg_m_s.map(|v| v / result.mass_kg);
    if !result.kinetic_j.is_finite()
        || result.kinetic_j < 0.
        || result
            .center_m
            .iter()
            .chain(&result.momentum_kg_m_s)
            .chain(&result.velocity_m_s)
            .chain(&result.angular_momentum_kg_m2_s)
            .any(|v| !v.is_finite())
    {
        return Err("quadratic fragment diagnostic overflow");
    }
    Ok(result)
}

/// Consistent-mass projection onto translation and rotation, without changing
/// the actual deformable velocity field or discarding its residual energy.
#[derive(Clone, Debug)]
pub struct QuadraticFragmentRotation {
    pub fragment: QuadraticFragment,
    pub inertia_kg_m2: [[f64; 3]; 3],
    /// Angular momentum about the fragment center, excluding orbital motion.
    pub spin_kg_m2_s: Vec3,
    pub angular_velocity_rad_s: Vec3,
    pub translation_kinetic_j: f64,
    pub rotation_kinetic_j: f64,
    pub deformation_kinetic_j: f64,
    pub energy_defect_j: f64,
}
pub(super) fn project_rotation(
    fragment: QuadraticFragment,
    mass: &[Vec<f64>],
    positions: &[Vec3],
    velocities: &[Vec3],
) -> Result<QuadraticFragmentRotation, &'static str> {
    let mut inertia = [[0.; 3]; 3];
    let mut spin = [0.; 3];
    for &i in &fragment.nodes {
        let ri: Vec3 = std::array::from_fn(|a| positions[i][a] - fragment.center_m[a]);
        for &j in &fragment.nodes {
            let rj: Vec3 = std::array::from_fn(|a| positions[j][a] - fragment.center_m[a]);
            let vj: Vec3 = std::array::from_fn(|a| velocities[j][a] - fragment.velocity_m_s[a]);
            let l = crate::plasticity::mesh::cross(ri, vj);
            for a in 0..3 {
                spin[a] += mass[i][j] * l[a];
                for b in 0..3 {
                    inertia[a][b] +=
                        mass[i][j] * ((if a == b { dot(ri, rj) } else { 0. }) - ri[b] * rj[a]);
                }
            }
        }
    }
    // Symmetry is exact in the consistent integral; midpoint removes reduction asymmetry.
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        let value = inertia[a][b].midpoint(inertia[b][a]);
        inertia[a][b] = value;
        inertia[b][a] = value;
    }
    if inertia
        .iter()
        .flatten()
        .chain(spin.iter())
        .any(|v| !v.is_finite())
    {
        return Err("fragment inertia overflow");
    }
    let solved = crate::plasticity::mesh::solve_dense(
        inertia.iter().map(|r| r.to_vec()).collect(),
        spin.to_vec(),
    )
    .map_err(|_| "singular or unrepresentable fragment inertia")?;
    let omega: Vec3 = solved
        .try_into()
        .map_err(|_| "invalid fragment angular solve")?;
    let mut residual = vec![[0.; 3]; positions.len()];
    for &i in &fragment.nodes {
        let r = std::array::from_fn(|a| positions[i][a] - fragment.center_m[a]);
        let rotation = crate::plasticity::mesh::cross(omega, r);
        residual[i] =
            std::array::from_fn(|a| velocities[i][a] - fragment.velocity_m_s[a] - rotation[a]);
    }
    let translation = 0.5 * fragment.mass_kg * dot(fragment.velocity_m_s, fragment.velocity_m_s);
    let rotation = 0.5 * dot(omega, spin);
    let mut deformation = 0.;
    for &i in &fragment.nodes {
        for &j in &fragment.nodes {
            deformation += 0.5 * mass[i][j] * dot(residual[i], residual[j]);
        }
    }
    let defect = fragment.kinetic_j - translation - rotation - deformation;
    let scale = fragment.kinetic_j.abs() + translation.abs() + rotation.abs() + deformation.abs();
    if ![translation, rotation, deformation, defect, scale]
        .iter()
        .all(|v| v.is_finite())
        || rotation < 0.
        || deformation < 0.
        || defect.abs() > 1e-10 * scale.max(f64::MIN_POSITIVE)
    {
        return Err("fragment rotation energy partition failure");
    }
    Ok(QuadraticFragmentRotation {
        fragment,
        inertia_kg_m2: inertia,
        spin_kg_m2_s: spin,
        angular_velocity_rad_s: omega,
        translation_kinetic_j: translation,
        rotation_kinetic_j: rotation,
        deformation_kinetic_j: deformation,
        energy_defect_j: defect,
    })
}
