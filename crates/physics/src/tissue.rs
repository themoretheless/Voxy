//! Compliant soft tissues in local metre/kilogram/second coordinates.
#![allow(clippy::many_single_char_names)]
use crate::strand::SphereCollider;
// Conventional scalar constraint notation follows the XPBD equations.
type V = [f64; 3];
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: V, s: f64) -> V {
    a.map(|v| v * s)
}
fn dot(a: V, b: V) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn finite(a: V) -> bool {
    a.iter().all(|v| v.is_finite())
}
fn volume(p: &[V], ids: [usize; 4]) -> f64 {
    let [a, b, c, d] = ids.map(|i| p[i]);
    dot(sub(b, a), cross(sub(c, a), sub(d, a))) / 6.0
}
#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub stretch_compliance: f64,
    pub volume_compliance: f64,
    /// Exponential velocity decay rate in inverse seconds.
    pub damping: f64,
    pub particle_radius: f64,
}
impl Default for Material {
    fn default() -> Self {
        Self {
            stretch_compliance: 1e-5,
            volume_compliance: 1e-8,
            damping: 3.0,
            particle_radius: 0.01,
        }
    }
}
/// Illustrative presets, not measured human tissue constants.
#[derive(Clone, Copy, Debug)]
pub enum TissueKind {
    Skin,
    Buttock,
    Breast,
    Lip,
    Sphincter,
    /// Abstract anchored volumetric shaft; not a calibrated anatomical model.
    Penis,
}
impl TissueKind {
    #[must_use]
    pub fn material(self) -> Material {
        let stretch_compliance = match self {
            Self::Skin => 2e-6,
            Self::Buttock => 2e-4,
            Self::Breast => 5e-4,
            Self::Lip => 5e-5,
            Self::Sphincter => 1e-5,
            Self::Penis => 8e-5,
        };
        Material {
            stretch_compliance,
            ..Material::default()
        }
    }
}
#[derive(Clone, Debug)]
struct Edge {
    ids: [usize; 2],
    rest: f64,
    muscle: bool,
    lambda: f64,
}
#[derive(Clone, Debug)]
struct Tet {
    ids: [usize; 4],
    rest: f64,
    lambda: f64,
}
#[derive(Clone, Debug)]
pub struct Tissue {
    positions: Vec<V>,
    velocities: Vec<V>,
    weights: Vec<f64>,
    edges: Vec<Edge>,
    tets: Vec<Tet>,
    material: Material,
    activation: f64,
    hardening: f64,
    surface: Vec<[usize; 3]>,
}
impl Tissue {
    /// Zero inverse mass pins a particle. Muscle edges shorten up to 35%.
    /// # Errors
    /// Rejects nonfinite data, invalid topology, degenerate cells and materials.
    pub fn new(
        positions: Vec<V>,
        weights: Vec<f64>,
        edges: Vec<([usize; 2], bool)>,
        tets: Vec<[usize; 4]>,
        material: Material,
    ) -> Result<Self, &'static str> {
        if positions.is_empty()
            || positions.len() != weights.len()
            || !positions.iter().copied().all(finite)
            || weights.iter().any(|v| !v.is_finite() || *v < 0.0)
            || [
                material.stretch_compliance,
                material.volume_compliance,
                material.damping,
                material.particle_radius,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err("invalid tissue data");
        }
        let mut links = Vec::new();
        for (ids, muscle) in edges {
            if ids.iter().any(|i| *i >= positions.len()) {
                return Err("invalid edge index");
            }
            let rest = dot(
                sub(positions[ids[0]], positions[ids[1]]),
                sub(positions[ids[0]], positions[ids[1]]),
            )
            .sqrt();
            if !rest.is_finite() || rest <= 1e-9 {
                return Err("degenerate edge");
            }
            links.push(Edge {
                ids,
                rest,
                muscle,
                lambda: 0.0,
            });
        }
        let mut cells = Vec::new();
        for ids in tets {
            if ids.iter().any(|i| *i >= positions.len()) {
                return Err("invalid tetrahedron index");
            }
            let rest = volume(&positions, ids);
            if !rest.is_finite() || rest.abs() < 1e-12 {
                return Err("degenerate tetrahedron");
            }
            cells.push(Tet {
                ids,
                rest,
                lambda: 0.0,
            });
        }
        let mut boundary = std::collections::BTreeMap::<[usize; 3], ([usize; 3], usize)>::new();
        for cell in &cells {
            for opposite in 0..4 {
                let mut face = [0; 3];
                let mut count = 0;
                for j in 0..4 {
                    if j != opposite {
                        face[count] = cell.ids[j];
                        count += 1;
                    }
                }
                let normal = cross(
                    sub(positions[face[1]], positions[face[0]]),
                    sub(positions[face[2]], positions[face[0]]),
                );
                if dot(
                    normal,
                    sub(positions[cell.ids[opposite]], positions[face[0]]),
                ) > 0.0
                {
                    face.swap(1, 2);
                }
                let mut key = face;
                key.sort_unstable();
                let entry = boundary.entry(key).or_insert((face, 0));
                entry.1 += 1;
                if entry.1 > 2 {
                    return Err("nonmanifold tissue surface");
                }
            }
        }
        let surface = boundary
            .into_values()
            .filter(|(_, n)| *n == 1)
            .map(|(face, _)| face)
            .collect();
        Ok(Self {
            velocities: vec![[0.0; 3]; positions.len()],
            positions,
            weights,
            edges: links,
            tets: cells,
            material,
            activation: 0.0,
            hardening: 0.0,
            surface,
        })
    }
    #[must_use]
    pub fn positions(&self) -> &[V] {
        &self.positions
    }
    /// Solver inverse particle masses in kg^-1. Zero denotes a fixed particle.
    #[must_use]
    pub fn inverse_masses(&self) -> &[f64] {
        &self.weights
    }
    pub fn edges(&self) -> impl Iterator<Item = [usize; 2]> + '_ {
        self.edges.iter().map(|e| e.ids)
    }
    /// Cell indices in the same stable ordering as positions().
    pub fn surface_triangles(&self) -> &[[usize; 3]] {
        &self.surface
    }
    pub fn tetrahedra(&self) -> impl Iterator<Item = [usize; 4]> + '_ {
        self.tets.iter().map(|t| t.ids)
    }
    #[must_use]
    pub fn volumes(&self) -> Vec<f64> {
        self.tets
            .iter()
            .map(|t| volume(&self.positions, t.ids))
            .collect()
    }
    /// # Errors
    /// Activation must be finite and between zero and one.
    pub fn set_activation(&mut self, value: f64) -> Result<(), &'static str> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err("invalid activation");
        }
        self.activation = value;
        Ok(())
    }
    /// Moves a pinned skeleton attachment without teleporting free tissue.
    /// # Errors
    /// Rejects invalid indices, free particles and nonfinite targets.
    pub fn move_pin(&mut self, index: usize, target: V) -> Result<(), &'static str> {
        if index >= self.positions.len() || self.weights[index] != 0.0 || !finite(target) {
            return Err("invalid pin");
        }
        self.positions[index] = target;
        Ok(())
    }
    /// Fixed steps <= 1/30 s. Discrete particle/sphere contact; no self-contact.
    /// # Errors
    /// Invalid inputs and numerical overflow leave the state unchanged.
    #[allow(clippy::too_many_lines)]
    pub fn step(
        &mut self,
        dt: f64,
        acceleration: V,
        colliders: &[SphereCollider],
        iterations: usize,
    ) -> Result<(), &'static str> {
        self.step_with_contact(dt, acceleration, colliders, iterations, 0.0)
    }
    /// Strain hardening coefficient (dimensionless). Zero keeps the linear edge law.
    /// # Errors
    /// Rejects negative or nonfinite coefficients without changing state.
    pub fn set_hardening(&mut self, hardening: f64) -> Result<(), &'static str> {
        if !hardening.is_finite() || hardening < 0.0 {
            return Err("invalid tissue hardening");
        }
        self.hardening = hardening;
        Ok(())
    }
    /// Advances with inelastic sphere contact and Coulomb velocity friction.
    /// Particle and triangle/sphere contacts are discrete, without swept collision.
    /// Friction impulse is capped by mu times accumulated normal projection
    /// multiplier / dt; multipliers reset at the start of every physical step.
    /// # Errors
    /// Invalid input, numerical overflow, or inverted cells leave state unchanged.
    pub fn step_with_contact(
        &mut self,
        dt: f64,
        acceleration: V,
        colliders: &[SphereCollider],
        iterations: usize,
        friction: f64,
    ) -> Result<(), &'static str> {
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 1.0 / 30.0
            || !finite(acceleration)
            || !friction.is_finite()
            || friction < 0.0
            || !(1..=128).contains(&iterations)
            || colliders
                .iter()
                .any(|c| !finite(c.center) || !c.radius.is_finite() || c.radius < 0.0)
        {
            return Err("invalid tissue step");
        }
        let mut next = self.clone();
        let old = &self.positions;
        for i in 0..next.positions.len() {
            if next.weights[i] > 0.0 {
                next.velocities[i] = mul(
                    add(next.velocities[i], mul(acceleration, dt)),
                    (-next.material.damping * dt).exp(),
                );
                next.positions[i] = add(next.positions[i], mul(next.velocities[i], dt));
            }
        }
        let mut particle_normal = vec![0.; next.positions.len() * colliders.len()];
        let mut surface_normal = vec![0.; next.surface.len() * colliders.len()];
        for e in &mut next.edges {
            e.lambda = 0.0;
        }
        for t in &mut next.tets {
            t.lambda = 0.0;
        }
        for _ in 0..iterations {
            for e in &mut next.edges {
                let [a, b] = e.ids;
                let delta = sub(next.positions[a], next.positions[b]);
                let length = dot(delta, delta).sqrt();
                if length < 1e-12 {
                    continue;
                }
                let alpha = next.material.stretch_compliance / (dt * dt);
                let rest = e.rest
                    * (if e.muscle {
                        1.0 - 0.35 * next.activation
                    } else {
                        1.0
                    });
                let extension = length - rest;
                let strain = extension / rest;
                // C²/2alpha is quadratic + quartic elastic energy.
                let factor = (1.0 + 0.5 * next.hardening * strain * strain).sqrt();
                let gradient = (1.0 + next.hardening * strain * strain) / factor;
                let constraint = extension * factor;
                let denom = (next.weights[a] + next.weights[b]) * gradient * gradient + alpha;
                if denom <= 0.0 {
                    continue;
                }
                let dl = (-constraint - alpha * e.lambda) / denom;
                e.lambda += dl;
                let n = mul(delta, gradient / length);
                next.positions[a] = add(next.positions[a], mul(n, next.weights[a] * dl));
                next.positions[b] = sub(next.positions[b], mul(n, next.weights[b] * dl));
            }
            for t in &mut next.tets {
                let [a, b, c, d] = t.ids.map(|i| next.positions[i]);
                let gb = mul(cross(sub(c, a), sub(d, a)), 1.0 / 6.0);
                let gc = mul(cross(sub(d, a), sub(b, a)), 1.0 / 6.0);
                let gd = mul(cross(sub(b, a), sub(c, a)), 1.0 / 6.0);
                let gradients = [mul(add(add(gb, gc), gd), -1.0), gb, gc, gd];
                let alpha = next.material.volume_compliance / (dt * dt);
                let denom = alpha
                    + (0..4)
                        .map(|j| next.weights[t.ids[j]] * dot(gradients[j], gradients[j]))
                        .sum::<f64>();
                if denom <= 0.0 {
                    continue;
                }
                let dl = (-(volume(&next.positions, t.ids) - t.rest) - alpha * t.lambda) / denom;
                t.lambda += dl;
                for (j, g) in gradients.iter().enumerate() {
                    let i = t.ids[j];
                    next.positions[i] = add(next.positions[i], mul(*g, next.weights[i] * dl));
                }
            }
            for (i, p) in next.positions.iter_mut().enumerate() {
                if next.weights[i] == 0.0 {
                    continue;
                }
                for (collider_index, c) in colliders.iter().enumerate() {
                    let delta = sub(*p, c.center);
                    let length = dot(delta, delta).sqrt();
                    let radius = c.radius + next.material.particle_radius;
                    if length < radius {
                        let n = if length > 1e-12 {
                            mul(delta, 1.0 / length)
                        } else {
                            [1.0, 0.0, 0.0]
                        };
                        particle_normal[i * colliders.len() + collider_index] +=
                            (radius - length) / next.weights[i];
                        *p = add(c.center, mul(n, radius));
                    }
                }
            }
            crate::tissue_contact::project(
                &mut next.positions,
                &next.weights,
                &next.surface,
                colliders,
                next.material.particle_radius,
                &mut surface_normal,
            )?;
        }
        for (i, p) in next.positions.iter().enumerate() {
            next.velocities[i] = mul(sub(*p, old[i]), 1.0 / dt);
            if next.weights[i] == 0.0 {
                continue;
            }
            for (collider_index, collider) in colliders.iter().enumerate() {
                let delta = sub(*p, collider.center);
                let distance = dot(delta, delta).sqrt();
                if distance <= collider.radius + next.material.particle_radius + 1e-9
                    && distance > 1e-12
                {
                    let normal = mul(delta, 1.0 / distance);
                    let normal_impulse = particle_normal[i * colliders.len() + collider_index] / dt;
                    let current_incoming = dot(next.velocities[i], normal).min(0.0);
                    next.velocities[i] = sub(next.velocities[i], mul(normal, current_incoming));
                    let tangent = sub(
                        next.velocities[i],
                        mul(normal, dot(next.velocities[i], normal)),
                    );
                    let speed = dot(tangent, tangent).sqrt();
                    if speed > 1e-12 {
                        next.velocities[i] = sub(
                            next.velocities[i],
                            mul(
                                tangent,
                                (friction * normal_impulse * next.weights[i] / speed).min(1.0),
                            ),
                        );
                    }
                }
            }
        }
        for (face_index, face) in next.surface.iter().enumerate() {
            for (sphere_index, sphere) in colliders.iter().enumerate() {
                if let Some((barycentric, normal, _)) = crate::tissue_contact::contact(
                    face.map(|i| next.positions[i]),
                    sphere.center,
                    sphere.radius + next.material.particle_radius + 1e-9,
                ) {
                    let denom: f64 = (0..3)
                        .map(|j| next.weights[face[j]] * barycentric[j] * barycentric[j])
                        .sum();
                    if denom <= 1e-20 {
                        continue;
                    }
                    let relative: V = std::array::from_fn(|k| {
                        (0..3)
                            .map(|j| next.velocities[face[j]][k] * barycentric[j])
                            .sum()
                    });
                    let incoming = dot(relative, normal).min(0.0);
                    let tangent = sub(relative, mul(normal, dot(relative, normal)));
                    let speed = dot(tangent, tangent).sqrt();
                    let fraction = if speed > 1e-12 {
                        (friction
                            * surface_normal[face_index * colliders.len() + sphere_index]
                            * denom
                            / (dt * speed))
                            .min(1.0)
                    } else {
                        0.0
                    };
                    let impulse = sub(mul(normal, -incoming), mul(tangent, fraction));
                    for j in 0..3 {
                        next.velocities[face[j]] = add(
                            next.velocities[face[j]],
                            mul(impulse, next.weights[face[j]] * barycentric[j] / denom),
                        );
                    }
                }
            }
        }
        if particle_normal
            .iter()
            .chain(&surface_normal)
            .any(|v| !v.is_finite())
        {
            return Err("tissue contact reaction overflow");
        }
        if next
            .tets
            .iter()
            .any(|t| volume(&next.positions, t.ids) * t.rest <= 0.0)
        {
            return Err("inverted tissue cell");
        }
        if !next.positions.iter().copied().all(finite)
            || !next.velocities.iter().copied().all(finite)
        {
            return Err("tissue overflow");
        }
        for face in &next.surface {
            for sphere in colliders {
                if crate::tissue_contact::contact(
                    face.map(|i| next.positions[i]),
                    sphere.center,
                    sphere.radius + next.material.particle_radius,
                )
                .is_some_and(|(_, _, depth)| depth > 1e-7)
                {
                    return Err("surface contact did not converge");
                }
            }
        }
        *self = next;
        Ok(())
    }
}
/// Creates an octahedral volume, a triangulated skin patch, or a muscle ring.
/// # Errors
/// Rejects invalid centers through the validated solver constructor.
pub fn sample(kind: TissueKind, center: V) -> Result<Tissue, &'static str> {
    let (points, weights, edges, tets) = match kind {
        TissueKind::Skin => {
            let p: Vec<V> = (0..25)
                .map(|i| {
                    [
                        f64::from(i % 5) * 0.15 - 0.3,
                        f64::from(i / 5) * 0.15 - 0.3,
                        0.0,
                    ]
                })
                .collect();
            let w = (0..25).map(|i| if i >= 20 { 0.0 } else { 1.0 }).collect();
            let mut e = Vec::new();
            for i in 0..25 {
                if i % 5 < 4 {
                    e.push(([i, i + 1], false));
                }
                if i < 20 {
                    e.push(([i, i + 5], false));
                }
                if i < 20 && i % 5 < 4 {
                    e.push(([i, i + 6], false));
                    e.push(([i + 1, i + 5], false));
                }
                if i < 15 {
                    e.push(([i, i + 10], false));
                }
            }
            (p, w, e, Vec::new())
        }
        TissueKind::Penis => {
            // Six square cross sections; each prism is split consistently into
            // six tetrahedra around its body diagonal. The base is attached.
            let mut p = Vec::new();
            for j in 0..6 {
                for [y, z] in [[-0.09, -0.09], [0.09, -0.09], [-0.09, 0.09], [0.09, 0.09]] {
                    p.push([f64::from(j) * 0.14 - 0.35, y, z]);
                }
            }
            let mut t = Vec::new();
            for j in 0..5 {
                let a = j * 4;
                for ids in [
                    [0, 1, 3, 7],
                    [0, 3, 2, 7],
                    [0, 2, 6, 7],
                    [0, 6, 4, 7],
                    [0, 4, 5, 7],
                    [0, 5, 1, 7],
                ] {
                    t.push(ids.map(|i| a + i));
                }
            }
            let mut e = Vec::new();
            for cell in &t {
                for a in 0..4 {
                    for b in a + 1..4 {
                        let mut ids = [cell[a], cell[b]];
                        ids.sort_unstable();
                        if !e.iter().any(|(other, _)| *other == ids) {
                            e.push((ids, false));
                        }
                    }
                }
            }
            let w = (0..24).map(|i| if i < 4 { 0.0 } else { 1.0 }).collect();
            (p, w, e, t)
        }
        TissueKind::Sphincter => {
            let p: Vec<V> = (0..24)
                .map(|i| {
                    let a = f64::from(i) * std::f64::consts::TAU / 24.0;
                    [0.35 * a.cos(), 0.35 * a.sin(), 0.0]
                })
                .collect();
            let e = (0..24)
                .map(|i| ([i, (i + 1) % 24], true))
                .chain((0..24).map(|i| ([i, (i + 2) % 24], true)))
                .collect();
            (p, vec![1.0; 24], e, Vec::new())
        }
        _ => {
            let (x, y, z) = match kind {
                TissueKind::Buttock => (0.36, 0.3, 0.28),
                TissueKind::Breast => (0.3, 0.36, 0.3),
                _ => (0.42, 0.12, 0.16),
            };
            let p = vec![
                [0.0; 3],
                [x, 0.0, 0.0],
                [-x, 0.0, 0.0],
                [0.0, y, 0.0],
                [0.0, -y, 0.0],
                [0.0, 0.0, z],
                [0.0, 0.0, -z],
            ];
            let mut t = Vec::new();
            for a in [1, 2] {
                for b in [3, 4] {
                    for c in [5, 6] {
                        t.push([0, a, b, c]);
                    }
                }
            }
            let mut e = Vec::new();
            for cell in &t {
                for a in 0..4 {
                    for b in a + 1..4 {
                        let mut ids = [cell[a], cell[b]];
                        ids.sort_unstable();
                        if !e.iter().any(|(other, _)| *other == ids) {
                            e.push((ids, matches!(kind, TissueKind::Lip) && !ids.contains(&0)));
                        }
                    }
                }
            }
            (p, vec![1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0], e, t)
        }
    };
    Tissue::new(
        points.into_iter().map(|p| add(p, center)).collect(),
        weights,
        edges,
        tets,
        kind.material(),
    )
}

#[cfg(test)]
mod reaction_friction_tests {
    use super::*;

    #[test]
    fn friction_budget_has_impulse_units_and_does_not_reverse_sliding() {
        for dt in [0.001, 0.002, 0.004] {
            for inv_mass in [0.25, 1., 4.] {
                for iterations in [1, 32] {
                    let mut b = Tissue::new(
                        vec![[0., 1., 0.]],
                        vec![inv_mass],
                        vec![],
                        vec![],
                        Material {
                            damping: 0.,
                            particle_radius: 0.,
                            ..Material::default()
                        },
                    )
                    .unwrap();
                    b.velocities[0] = [1., -0.2, 0.];
                    let predicted = add(b.positions[0], mul(b.velocities[0], dt));
                    let normal_delta_v = (1. - dot(predicted, predicted).sqrt()) / dt;
                    let sphere = SphereCollider {
                        center: [0.; 3],
                        radius: 1.,
                    };
                    let mut free = b.clone();
                    free.step_with_contact(dt, [0.; 3], &[sphere], iterations, 0.)
                        .unwrap();
                    b.step_with_contact(dt, [0.; 3], &[sphere], iterations, 0.5)
                        .unwrap();
                    let n = b.positions[0];
                    let tangent = sub(free.velocities[0], mul(n, dot(free.velocities[0], n)));
                    let speed = dot(tangent, tangent).sqrt();
                    let reduction = sub(free.velocities[0], b.velocities[0]);
                    let actual = dot(reduction, reduction).sqrt();
                    assert!((actual - (0.5 * normal_delta_v).min(speed)).abs() < 1e-10);
                    assert!(dot(b.velocities[0], tangent) >= -1e-12);
                }
            }
        }
    }

    #[test]
    fn elastic_pressure_produces_friction_without_incoming_normal_velocity() {
        let mut body = Tissue::new(
            vec![[0., 0.51, 0.], [0., 0.49, 0.]],
            vec![1., 0.],
            vec![([0, 1], false)],
            vec![],
            Material {
                damping: 0.,
                particle_radius: 0.01,
                ..Material::default()
            },
        )
        .unwrap();
        body.edges[0].rest = 0.01;
        body.velocities[0] = [1., 0., 0.];
        let mut sliding = body.clone();
        let sphere = SphereCollider {
            center: [0.; 3],
            radius: 0.5,
        };
        sliding
            .step_with_contact(0.001, [0.; 3], &[sphere], 64, 0.)
            .unwrap();
        body.step_with_contact(0.001, [0.; 3], &[sphere], 64, 0.5)
            .unwrap();
        let tangent_speed = |b: &Tissue| {
            let p = b.positions[0];
            let n = mul(p, 1. / dot(p, p).sqrt());
            let t = sub(b.velocities[0], mul(n, dot(b.velocities[0], n)));
            dot(t, t).sqrt()
        };
        let free = tangent_speed(&sliding);
        let friction = tangent_speed(&body);
        println!("elastic pressure: tangent speed {free:.9} -> {friction:.9} m/s");
        assert!(free > 0.9);
        assert!(friction < free - 0.1);
        assert_eq!(body.positions[1], [0., 0.49, 0.]);
        assert!(dot(body.positions[0], body.positions[0]).sqrt() >= 0.51 - 1e-12);
    }
}
