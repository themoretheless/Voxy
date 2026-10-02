//! SI-unit Cosserat guide rods with implicit stretch/shear and material-frame bend/twist.
//! No attraction of free particles to world-space groom positions.
mod contact;
mod direct;
mod math;
pub use contact::TriangleMesh;
use math::*;

/// Circular fiber constitutive parameters. Values are tunable, not measured for this asset.
#[derive(Clone, Copy, Debug)]
pub struct HairMaterial {
    pub radius: f64,
    pub density: f64,
    pub young_modulus: f64,
    pub poisson_ratio: f64,
    /// Exponential damping rate (s^-1), not a per-frame multiplier.
    pub damping: f64,
    pub friction: f64,
    /// Aerodynamic relaxation toward air velocity (s^-1).
    pub air_drag: f64,
}
impl Default for HairMaterial {
    fn default() -> Self {
        Self {
            radius: 40e-6,
            density: 1300.,
            young_modulus: 4e9,
            poisson_ratio: 0.35,
            damping: 0.6,
            friction: 0.25,
            air_drag: 2.0,
        }
    }
}
impl HairMaterial {
    pub fn area(self) -> f64 {
        std::f64::consts::PI * self.radius.powi(2)
    }
    pub fn bending_rigidity(self) -> f64 {
        self.young_modulus * std::f64::consts::PI * self.radius.powi(4) / 4.
    }
    pub fn twisting_rigidity(self) -> f64 {
        self.bending_rigidity() / (1. + self.poisson_ratio)
    }
    fn valid(self) -> bool {
        [self.radius, self.density, self.young_modulus]
            .iter()
            .all(|x| x.is_finite() && *x > 0.)
            && [
                self.area(),
                self.bending_rigidity(),
                self.twisting_rigidity(),
            ]
            .iter()
            .all(|x| x.is_finite() && *x > 1e-30)
            && self.poisson_ratio.is_finite()
            && (-1.0..0.5).contains(&self.poisson_ratio)
            && [self.damping, self.friction, self.air_drag]
                .iter()
                .all(|x| x.is_finite() && *x >= 0.)
    }
}
/// Root position and rigid rotation relative to the initial groom's frame.
#[derive(Clone, Copy, Debug)]
pub struct RootPose {
    pub position: V,
    pub rotation: Q,
}
#[derive(Clone, Debug)]
pub struct HairRod {
    rest_x: Vec<V>,
    x: Vec<V>,
    velocity: Vec<V>,
    q: Vec<Q>,
    omega: Vec<V>,
    old_x: Vec<V>,
    predicted_x: Vec<V>,
    predicted_q: Vec<Q>,
    contact_targets: Vec<V>,
    // Exact nearest-surface queries warm-start from the previous triangle.
    nearest_cache: Vec<Vec<usize>>,
    clearance_cache: Vec<Vec<Option<contact::Clearance>>>,
    contact_candidates: Vec<usize>,
    solve_matrix: Vec<f64>,
    solve_rhs: Vec<f64>,
    old_q: Vec<Q>,
    rest_q: Vec<Q>,
    rest_relative: Vec<Q>,
    lengths: Vec<f64>,
    inv_mass: Vec<f64>,
    inv_inertia: Vec<f64>,

    normals: Vec<V>,
    material: HairMaterial,
}
impl HairRod {
    pub fn new(points: Vec<V>, material: HairMaterial) -> Result<Self, &'static str> {
        let lengths: Vec<_> = points.windows(2).map(|p| len(sub(p[1], p[0]))).collect();
        if points.len() < 3
            || !points.iter().copied().all(finite)
            || !material.valid()
            || lengths.iter().any(|l| !l.is_finite() || *l < 1e-6)
        {
            return Err("invalid hair rod");
        }
        let n = points.len();
        let area = material.area();
        let mut masses = vec![0.; n];
        for (i, l) in lengths.iter().enumerate() {
            let m = material.density * area * l * 0.5;
            masses[i] += m;
            masses[i + 1] += m;
        }
        let mut inv_mass: Vec<_> = masses.iter().map(|m| 1. / m).collect();
        inv_mass[0] = 0.;
        // Segment frame has transverse inertia m*l^2/12, regularized by circular section.
        let mut inv_inertia: Vec<_> = lengths
            .iter()
            .map(|l| {
                1. / (material.density * area * l * (l * l / 12. + material.radius.powi(2) / 4.))
            })
            .collect();
        inv_inertia[0] = 0.;
        let mut q = Vec::with_capacity(n - 1);
        // Parallel transport material frames to avoid arbitrary rest twist in curved grooms.
        for i in 0..n - 1 {
            let tangent = unit(sub(points[i + 1], points[i]));
            if i == 0 {
                q.push(from_z(tangent));
            } else {
                let prev = rotate(q[i - 1], [0., 0., 1.]);
                let axis = cross(prev, tangent);
                let rot = if dot(prev, tangent) < -0.999999 {
                    exp(mul(rotate(q[i - 1], [1., 0., 0.]), std::f64::consts::PI))
                } else {
                    qunit([axis[0], axis[1], axis[2], 1. + dot(prev, tangent)])
                };
                q.push(qunit(qm(rot, q[i - 1])));
            }
        }
        let rest_relative = q.windows(2).map(|q| qm(conj(q[0]), q[1])).collect();
        Ok(Self {
            predicted_x: points.clone(),
            contact_targets: points.clone(),
            nearest_cache: Vec::new(),
            clearance_cache: Vec::new(),
            contact_candidates: Vec::new(),
            solve_matrix: Vec::new(),
            solve_rhs: Vec::new(),
            predicted_q: q.clone(),
            old_x: points.clone(),
            old_q: q.clone(),
            rest_q: q.clone(),
            rest_x: points.clone(),
            x: points,
            q,
            velocity: vec![[0.; 3]; n],
            omega: vec![[0.; 3]; n - 1],
            lengths,
            inv_mass,
            inv_inertia,
            rest_relative,

            normals: vec![[0.; 3]; n],
            material,
        })
    }
    /// Canonical stress-free curve, independent of the current dynamic pose.
    pub fn rest_positions(&self) -> &[V] {
        &self.rest_x
    }
    /// Rebuild segment lengths, frames, masses and inertia for an edited curve.
    /// Material and node count are preserved; velocities/contact caches reset.
    /// Failure leaves the original rod unchanged.
    pub fn rebased(&self, points: Vec<V>) -> Result<Self, &'static str> {
        if points.len() != self.rest_x.len() {
            return Err("hair rebase changes node count");
        }
        Self::new(points, self.material)
    }
    pub fn positions(&self) -> &[V] {
        &self.x
    }
    pub fn orientations(&self) -> &[Q] {
        &self.q
    }
    pub fn rest_lengths(&self) -> &[f64] {
        &self.lengths
    }
    pub fn mass(&self) -> f64 {
        self.lengths.iter().sum::<f64>() * self.material.density * self.material.area()
    }
    pub fn max_relative_stretch(&self) -> f64 {
        self.x
            .windows(2)
            .zip(&self.lengths)
            .map(|(p, l)| (len(sub(p[1], p[0])) / l - 1.).abs())
            .fold(0., f64::max)
    }
    fn predict(&mut self, dt: f64, root: RootPose, gravity: V, air: V) {
        self.old_x.clone_from(&self.x);
        self.old_q.clone_from(&self.q);

        self.normals.fill([0.; 3]);
        self.x[0] = root.position;
        self.q[0] = qunit(qm(root.rotation, self.rest_q[0]));
        let drag = (-self.material.air_drag * dt).exp();
        let damping = (-self.material.damping * dt).exp();
        for i in 1..self.x.len() {
            self.velocity[i] = add(
                mul(self.velocity[i], damping * drag),
                add(mul(air, 1. - drag), mul(gravity, dt)),
            );
            self.x[i] = add(self.x[i], mul(self.velocity[i], dt));
        }
        for i in 1..self.q.len() {
            self.omega[i] = mul(self.omega[i], damping);
            apply(&mut self.q[i], mul(self.omega[i], dt));
        }
        self.predicted_x.clone_from(&self.x);
        self.predicted_q.clone_from(&self.q);
    }
    fn solve(&mut self, dt: f64, _reverse: bool) {
        direct::solve(self, dt);
    }
    fn finish(&mut self, dt: f64) {
        for i in 0..self.x.len() {
            self.velocity[i] = mul(sub(self.x[i], self.old_x[i]), 1. / dt);
            let normal = self.normals[i];
            if len(normal) > 1e-12 {
                let n = unit(normal);
                let vn = dot(self.velocity[i], n);
                let tangent = sub(self.velocity[i], mul(n, vn));
                let removed = self.material.friction * vn.abs();
                let factor = (1. - removed / len(tangent).max(1e-30)).max(0.);
                self.velocity[i] = add(mul(tangent, factor), mul(n, vn.max(0.)));
            }
        }
        for i in 0..self.q.len() {
            self.omega[i] = mul(log(qm(self.q[i], conj(self.old_q[i]))), 1. / dt);
        }
    }
}
/// Coupled guide solve. Segment contacts run inside the structural iterations.
#[derive(Clone, Debug)]
pub struct HairSystem {
    rods: Vec<HairRod>,
    pub iterations: usize,
    /// Independent guide solves run before deterministic shared contacts.
    pub workers: usize,
    pub substeps: usize,
    pub contact_radius: f64,
    pub self_collision: bool,
}
impl HairSystem {
    pub fn new(rods: Vec<HairRod>) -> Result<Self, &'static str> {
        if rods.is_empty() {
            return Err("empty hair system");
        }
        Ok(Self {
            rods,
            iterations: 16,
            workers: 1,
            substeps: 4,
            contact_radius: 40e-6,
            self_collision: true,
        })
    }
    /// Stage all guide rebuilds before publishing a new system. Preserves solver
    /// settings; no dynamic pose or velocity is transferred to the edited groom.
    pub fn rebased(&self, curves: Vec<Vec<V>>) -> Result<Self, &'static str> {
        if curves.len() != self.rods.len() {
            return Err("hair rebase changes guide count");
        }
        let rods = self
            .rods
            .iter()
            .zip(curves)
            .map(|(rod, curve)| rod.rebased(curve))
            .collect::<Result<Vec<_>, _>>()?;
        let mut system = Self::new(rods)?;
        system.iterations = self.iterations;
        system.workers = self.workers;
        system.substeps = self.substeps;
        system.contact_radius = self.contact_radius;
        system.self_collision = self.self_collision;
        Ok(system)
    }
    pub fn rods(&self) -> &[HairRod] {
        &self.rods
    }
    /// Mesh is in world coordinates. Invalid arguments do not mutate state.
    pub fn step(
        &mut self,
        dt: f64,
        roots: &[RootPose],
        gravity: V,
        air_velocity: V,
        meshes: &[TriangleMesh],
    ) -> Result<(), &'static str> {
        if !dt.is_finite()
            || dt < 1e-6
            || dt > 1. / 20.
            || self.substeps == 0
            || self.substeps > 32
            || self.workers == 0
            || self.workers > 64
            || self.iterations == 0
            || self.iterations > 128
            || !self.contact_radius.is_finite()
            || self.contact_radius <= 0.
            || roots.len() != self.rods.len()
            || !finite(gravity)
            || !finite(air_velocity)
            || roots.iter().any(|r| {
                !finite(r.position)
                    || r.rotation.iter().any(|x| !x.is_finite())
                    || (r.rotation.iter().map(|x| x * x).sum::<f64>() - 1.).abs() > 1e-5
            })
        {
            return Err("invalid hair step");
        }
        // Meshes are immutable for this step. Invalidate distance bounds before
        // every new call so animated/refitted or replaced surfaces cannot reuse them.
        for rod in &mut self.rods {
            for cache in &mut rod.clearance_cache {
                cache.fill(None);
            }
        }
        let substeps = self.substeps.max((dt * 240.0).ceil() as usize);
        let dt = dt / substeps as f64;
        for substep in 0..substeps {
            for (rod, root) in self.rods.iter_mut().zip(roots) {
                // Interpolate moving attachments over substeps rather than teleporting on first substep.
                let t = 1. / (substeps - substep) as f64;
                let position = add(rod.x[0], mul(sub(root.position, rod.x[0]), t));
                let current = qm(rod.q[0], conj(rod.rest_q[0]));
                let rotation = qm(exp(mul(log(qm(root.rotation, conj(current))), t)), current);
                rod.predict(dt, RootPose { position, rotation }, gravity, air_velocity);
            }
            // Guides are independent until shared self-contact. Keep workers
            // alive across each independent batch instead of spawning per iteration.
            let parallel = self.workers > 1 && self.rods.len() >= 8;
            let batch = if parallel {
                if self.self_collision {
                    4
                } else {
                    self.iterations
                }
            } else {
                1
            };
            for first in (0..self.iterations).step_by(batch) {
                let end = (first + batch).min(self.iterations);
                let radius = self.contact_radius;
                if !parallel {
                    for rod in &mut self.rods {
                        rod.solve(dt, first % 2 != 0);
                        contact::mesh_contacts(rod, meshes, radius);
                    }
                } else {
                    let chunk = self.rods.len().div_ceil(self.workers);
                    std::thread::scope(|scope| {
                        for rods in self.rods.chunks_mut(chunk) {
                            scope.spawn(move || {
                                for rod in rods {
                                    for iteration in first..end {
                                        rod.solve(dt, iteration % 2 != 0);
                                        contact::mesh_contacts(rod, meshes, radius);
                                    }
                                }
                            });
                        }
                    });
                }
                if self.self_collision && (end % 4 == 0 || end == self.iterations) {
                    contact::self_contacts(&mut self.rods, radius);
                }
            }
            for rod in &mut self.rods {
                rod.finish(dt);
            }
        }
        if self.rods.iter().any(|r| {
            r.x.iter().any(|p| !finite(*p)) || r.q.iter().flatten().any(|q| !q.is_finite())
        }) {
            return Err("hair solver overflow");
        }
        Ok(())
    }
}
