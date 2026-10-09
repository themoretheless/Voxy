use super::{
    ContactScene, SkinMaterial,
    contact::{closest, safe_path, sphere_in_range},
    dual::{D, cross as dcross, dot as ddot, sub as dsub},
};
use std::collections::{BTreeMap, BTreeSet};
pub type Point = [f64; 3];
fn add(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: Point, s: f64) -> Point {
    a.map(|v| v * s)
}
fn dot(a: Point, b: Point) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn finite(p: Point) -> bool {
    p.iter().all(|v| v.is_finite())
}
#[derive(Clone, Debug)]
struct Face {
    ids: [usize; 3],
    derivatives: [[f64; 2]; 3],
    area: f64,
    direction: f64,
    stiffness_scale: f64,
    density_scale: f64,
    memory: Vec<[f64; 3]>,
}
#[derive(Clone, Debug)]
struct Hinge {
    ids: [usize; 4],
    rest: f64,
    stiffness: f64,
}
/// A compliant attachment to deeper tissue or fascia, including a relative dashpot.
#[derive(Clone, Copy, Debug)]
pub struct Attachment {
    pub vertex: usize,
    /// Attachment position at the beginning of the step; the solver advances it by velocity * dt.
    pub target: Point,
    pub velocity: Point,
    pub stiffness: f64,
    pub viscosity: f64,
}
/// Solver limits and convergence requirements; steps are transactional.
#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    pub max_newton: usize,
    pub max_cg: usize,
    pub force_tolerance: f64,
    pub relative_tolerance: f64,
}
impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_newton: 32,
            max_cg: 256,
            force_tolerance: 1e-7,
            relative_tolerance: 1e-6,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct StepReport {
    pub iterations: usize,
    pub residual: f64,
    pub energy: f64,
    pub min_area_ratio: f64,
}
#[derive(Clone, Debug)]
pub struct Skin {
    element_workers: usize,
    rest: Vec<Point>,
    positions: Vec<Point>,
    velocities: Vec<Point>,
    mass: Vec<f64>,
    vertex_area: Vec<f64>,
    pins: BTreeMap<usize, Point>,
    faces: Vec<Face>,
    triangles: Vec<[usize; 3]>,
    hinges: Vec<Hinge>,
    material: SkinMaterial,
}
/// Kinematic surface measures relative to the stress-free mesh.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMetrics {
    /// Ordered minimum and maximum principal stretches (dimensionless).
    pub principal_stretches: [f64; 2],
    /// Current area divided by reference area.
    pub area_ratio: f64,
    /// Current incompressible total thickness, in metres.
    pub thickness: f64,
}
#[derive(Debug)]
struct Block {
    ids: Vec<usize>,
    g: Vec<f64>,
    h: Vec<f64>,
}
impl Block {
    fn from<const N: usize>(ids: &[usize], d: D<N>) -> Result<Self, &'static str> {
        if !d.finite() {
            return Err("skin energy overflow");
        }
        Ok(Self {
            ids: ids.to_vec(),
            g: d.g.to_vec(),
            h: d.h.into_iter().flatten().collect(),
        })
    }
}
fn evaluate_elements<T: Sync, R: Send>(
    elements: &[T],
    workers: usize,
    evaluate: impl Fn(&T) -> Result<R, &'static str> + Sync,
) -> Result<Vec<R>, &'static str> {
    if workers == 1 || elements.len() < 128 {
        return elements.iter().map(evaluate).collect();
    }
    std::thread::scope(|scope| {
        let evaluate = &evaluate;
        let handles: Vec<_> = elements
            .chunks(elements.len().div_ceil(workers))
            .map(|chunk| {
                scope.spawn(move || chunk.iter().map(evaluate).collect::<Result<Vec<R>, _>>())
            })
            .collect();
        let mut values = Vec::with_capacity(elements.len());
        for handle in handles {
            values.extend(
                handle
                    .join()
                    .map_err(|_| "skin element worker panicked")??,
            );
        }
        Ok(values)
    })
}

#[derive(Debug)]
struct Evaluation {
    energy: f64,
    gradient: Vec<Point>,
    diagonal: Vec<Point>,
    blocks: Vec<Block>,
    inertia: Vec<f64>,
    min_area_ratio: f64,
}
fn variables<const N: usize, const P: usize>(
    positions: &[Point],
    ids: [usize; P],
) -> [[D<N>; 3]; P] {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if N == 0 {
                D::c(positions[ids[i]][j])
            } else {
                D::variable(positions[ids[i]][j], i * 3 + j)
            }
        })
    })
}
fn metric<const N: usize>(p: [[D<N>; 3]; 3], f: &Face) -> [D<N>; 3] {
    let columns: [[D<N>; 3]; 2] = std::array::from_fn(|j| {
        std::array::from_fn(|k| (0..3).fold(D::c(0.0), |s, i| s + p[i][k] * f.derivatives[i][j]))
    });
    [
        ddot(columns[0], columns[0]),
        ddot(columns[1], columns[1]),
        ddot(columns[0], columns[1]),
    ]
}
fn angle<const N: usize>(p: [[D<N>; 3]; 4]) -> D<N> {
    let [a, b, c, d] = p;
    let edge = dsub(b, a);
    let n1 = dcross(edge, dsub(c, a));
    let n2 = dcross(dsub(a, b), dsub(d, b));
    let e = edge.map(|v| v * ddot(edge, edge).sqrt().reciprocal());
    ddot(e, dcross(n1, n2)).atan2(ddot(n1, n2))
}
// A manifold edge check alone misses two disconnected surface fans meeting
// at one vertex. The link of each vertex must be one path or one closed cycle.
fn validate_vertex_fans(triangles: &[[usize; 3]], count: usize) -> Result<(), &'static str> {
    let mut links = vec![Vec::new(); count];
    for &[a, b, c] in triangles {
        for (vertex, edge) in [(a, (b, c)), (b, (c, a)), (c, (a, b))] {
            links[vertex].push(edge);
        }
    }
    for link in links {
        let mut graph = BTreeMap::<usize, Vec<usize>>::new();
        for (a, b) in link {
            graph.entry(a).or_default().push(b);
            graph.entry(b).or_default().push(a);
        }
        let endpoints = graph.values().filter(|v| v.len() == 1).count();
        if graph.is_empty() || graph.values().any(|v| v.len() > 2) || ![0, 2].contains(&endpoints) {
            return Err("nonmanifold skin vertex");
        }
        let mut pending = vec![*graph.keys().next().expect("nonempty vertex link")];
        let mut seen = BTreeSet::new();
        while let Some(vertex) = pending.pop() {
            if seen.insert(vertex) {
                pending.extend(&graph[&vertex]);
            }
        }
        if seen.len() != graph.len() {
            return Err("disconnected skin vertex fan");
        }
    }
    Ok(())
}
impl Skin {
    /// `rest` is the stress-free midsurface; `directions` provides one rest-space
    /// collagen axis per triangle. Mass is lumped from reference area and layer density.
    /// # Errors
    /// Rejects degenerate, duplicate, unused or inconsistently oriented topology,
    /// Rebuilds rest metrics, lumped masses, hinges and pins for a new stress-free shape.
    /// Keeps topology, material, regional scales and worker configuration. Resets motion
    /// and relaxation history; this is a shape edit, not an animation update.
    pub fn rebased(&self, rest: Vec<Point>) -> Result<Self, &'static str> {
        if rest.len() != self.rest.len() || !rest.iter().copied().all(finite) {
            return Err("invalid skin rest shape");
        }
        let mut directions = Vec::with_capacity(self.faces.len());
        for face in &self.faces {
            let [a, b, c] = face.ids.map(|i| rest[i]);
            let edge = sub(b, a);
            let length = dot(edge, edge).sqrt();
            let normal = cross(edge, sub(c, a));
            let area = dot(normal, normal).sqrt();
            if length <= 1e-9 || area <= 1e-12 || !length.is_finite() || !area.is_finite() {
                return Err("degenerate skin rest shape");
            }
            let e1 = mul(edge, 1. / length);
            let e2 = cross(mul(normal, 1. / area), e1);
            directions.push(add(
                mul(e1, face.direction.cos()),
                mul(e2, face.direction.sin()),
            ));
        }
        let pins: Vec<_> = self.pins.keys().copied().collect();
        let mut next = Self::new(
            rest,
            self.triangles.clone(),
            &pins,
            self.material.clone(),
            directions,
        )?;
        next.element_workers = self.element_workers;
        next.set_face_properties(
            &self
                .faces
                .iter()
                .map(|f| f.stiffness_scale)
                .collect::<Vec<_>>(),
            &self
                .faces
                .iter()
                .map(|f| f.density_scale)
                .collect::<Vec<_>>(),
        )?;
        Ok(next)
    }
    /// Regional multipliers for membrane/relaxation stiffness and surface mass density.
    /// Recomputes lumped masses and shared-edge bending consistently, transactionally.
    pub fn set_face_properties(
        &mut self,
        stiffness: &[f64],
        density: &[f64],
    ) -> Result<(), &'static str> {
        if stiffness.len() != self.faces.len()
            || density.len() != self.faces.len()
            || stiffness
                .iter()
                .chain(density)
                .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid regional skin properties");
        }
        let mut masses = vec![0.; self.rest.len()];
        let mut adjacency = BTreeMap::<(usize, usize), Vec<(f64, f64)>>::new();
        for (i, face) in self.faces.iter().enumerate() {
            for vertex in face.ids {
                masses[vertex] += face.area * self.material.area_density() * density[i] / 3.;
            }
            for [a, b] in [
                [face.ids[0], face.ids[1]],
                [face.ids[1], face.ids[2]],
                [face.ids[2], face.ids[0]],
            ] {
                adjacency
                    .entry((a.min(b), a.max(b)))
                    .or_default()
                    .push((face.area, stiffness[i]));
            }
        }
        let mut bending = Vec::new();
        for hinge in &self.hinges {
            let [a, b, _, _] = hinge.ids;
            let edge = sub(self.rest[a], self.rest[b]);
            let pair = &adjacency[&(a.min(b), a.max(b))];
            let scale = 2. / (1. / pair[0].1 + 1. / pair[1].1);
            bending.push(
                self.material.bending_rigidity() * dot(edge, edge) / (pair[0].0 + pair[1].0)
                    * scale,
            );
        }
        if masses
            .iter()
            .chain(&bending)
            .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("regional skin properties overflow");
        }
        self.mass = masses;
        for (i, face) in self.faces.iter_mut().enumerate() {
            face.stiffness_scale = stiffness[i];
            face.density_scale = density[i];
        }
        for (hinge, value) in self.hinges.iter_mut().zip(bending) {
            hinge.stiffness = value;
        }
        Ok(())
    }
    /// invalid materials, indices, pins and fiber directions.
    pub fn new(
        rest: Vec<Point>,
        triangles: Vec<[usize; 3]>,
        pins: &[usize],
        material: SkinMaterial,
        directions: Vec<Point>,
    ) -> Result<Self, &'static str> {
        material.validate()?;
        if rest.len() < 3
            || !rest.iter().copied().all(finite)
            || triangles.is_empty()
            || triangles.len() != directions.len()
            || pins.iter().any(|i| *i >= rest.len())
        {
            return Err("invalid skin mesh");
        }
        let mut faces = Vec::new();
        let mut mass = vec![0.0; rest.len()];
        let mut seen = BTreeSet::new();
        let mut adjacency: BTreeMap<(usize, usize), Vec<(usize, usize, usize, f64)>> =
            BTreeMap::new();
        let branch_count: usize = material.layers.iter().map(|l| l.relaxation.len()).sum();
        for (ids, direction) in triangles.iter().copied().zip(directions) {
            let mut key = ids;
            key.sort_unstable();
            if ids.iter().any(|i| *i >= rest.len()) || !seen.insert(key) || !finite(direction) {
                return Err("invalid skin triangle");
            }
            let [a, b, c] = ids.map(|i| rest[i]);
            let edge = sub(b, a);
            let length = dot(edge, edge).sqrt();
            let normal = cross(edge, sub(c, a));
            let twice_area = dot(normal, normal).sqrt();
            if !length.is_finite()
                || !twice_area.is_finite()
                || twice_area <= 1e-12
                || length <= 1e-9
            {
                return Err("degenerate skin triangle");
            }
            let e1 = mul(edge, 1.0 / length);
            let n = mul(normal, 1.0 / twice_area);
            let e2 = cross(n, e1);
            let x = dot(sub(c, a), e1);
            let y = dot(sub(c, a), e2);
            let derivatives = [
                [-1.0 / length, (x / length - 1.0) / y],
                [1.0 / length, -x / (length * y)],
                [0.0, 1.0 / y],
            ];
            let axis = [dot(direction, e1), dot(direction, e2)];
            if axis[0].hypot(axis[1]) < 1e-9 {
                return Err("fiber axis must have a tangential component");
            }
            let area = twice_area * 0.5;
            for i in ids {
                mass[i] += area * material.area_density() / 3.0;
            }
            for [a, b, c] in [
                [ids[0], ids[1], ids[2]],
                [ids[1], ids[2], ids[0]],
                [ids[2], ids[0], ids[1]],
            ] {
                adjacency
                    .entry((a.min(b), a.max(b)))
                    .or_default()
                    .push((a, b, c, area));
            }
            faces.push(Face {
                ids,
                derivatives,
                area,
                direction: axis[1].atan2(axis[0]),
                stiffness_scale: 1.,
                density_scale: 1.,
                memory: vec![[0.0; 3]; branch_count],
            });
        }
        if mass.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err("unused vertices or invalid skin mass");
        }
        validate_vertex_fans(&triangles, rest.len())?;
        let mut hinges = Vec::new();
        for entries in adjacency.into_values() {
            if entries.len() > 2 {
                return Err("nonmanifold skin edge");
            }
            if entries.len() == 2 {
                let (a, b, c, area1) = entries[0];
                let (b2, a2, d, area2) = entries[1];
                if a != a2 || b != b2 {
                    return Err("inconsistent skin winding");
                }
                let ids = [a, b, c, d];
                let rest_angle = angle(variables::<12, 4>(&rest, ids)).v;
                let length2 = dot(sub(rest[b], rest[a]), sub(rest[b], rest[a]));
                hinges.push(Hinge {
                    ids,
                    rest: rest_angle,
                    stiffness: 3.0 * material.bending_rigidity() * length2 / (area1 + area2),
                });
            }
        }
        if hinges
            .iter()
            .any(|h| !h.rest.is_finite() || !h.stiffness.is_finite())
        {
            return Err("invalid skin bending stiffness");
        }
        Ok(Self {
            positions: rest.clone(),
            element_workers: 1,
            velocities: vec![[0.0; 3]; rest.len()],
            vertex_area: mass.iter().map(|m| m / material.area_density()).collect(),
            mass,
            pins: pins.iter().map(|i| (*i, rest[*i])).collect(),
            rest,
            faces,
            triangles,
            hinges,
            material,
        })
    }
    #[must_use]
    pub fn positions(&self) -> &[Point] {
        &self.positions
    }
    /// Sets the CPU budget for independent element derivatives. Assembly remains ordered.
    /// # Errors
    /// Rejects zero or more than 64 workers without changing the previous setting.
    pub fn set_element_workers(&mut self, workers: usize) -> Result<(), &'static str> {
        if !(1..=64).contains(&workers) {
            return Err("invalid skin worker count");
        }
        self.element_workers = workers;
        Ok(())
    }
    #[must_use]
    pub fn rest_positions(&self) -> &[Point] {
        &self.rest
    }
    #[must_use]
    pub fn triangles(&self) -> &[[usize; 3]] {
        &self.triangles
    }
    #[must_use]
    pub fn velocities(&self) -> &[Point] {
        &self.velocities
    }
    #[must_use]
    pub fn masses(&self) -> &[f64] {
        &self.mass
    }
    #[must_use]
    pub fn material(&self) -> &SkinMaterial {
        &self.material
    }
    /// Sets a future kinematic target; no free particle is teleported.
    /// # Errors
    /// Rejects unpinned vertices and nonfinite coordinates.
    pub fn set_pin_target(&mut self, index: usize, target: Point) -> Result<(), &'static str> {
        if !self.pins.contains_key(&index) || !finite(target) {
            return Err("invalid skin pin");
        }
        self.pins.insert(index, target);
        Ok(())
    }
    /// Initializes a deformed pose and velocity without changing rest metric or memory.
    /// Use pin targets for animation. Useful for loading measured prestrain or a restart.
    /// # Errors
    /// Rejects invalid metrics, dimensions or velocities, preserving existing state.
    pub fn set_state(
        &mut self,
        positions: Vec<Point>,
        velocities: Vec<Point>,
    ) -> Result<(), &'static str> {
        if positions.len() != self.positions.len()
            || velocities.len() != positions.len()
            || !positions.iter().chain(&velocities).copied().all(finite)
        {
            return Err("invalid skin state");
        }
        self.evaluate(&positions, None, 0.0, &[], &ContactScene::default())?;
        for (&i, target) in &mut self.pins {
            *target = positions[i];
        }
        self.positions = positions;
        self.velocities = velocities;
        Ok(())
    }
    /// Current elastic forces, including the stored Maxwell stresses, in newtons.
    /// # Errors
    /// Returns an error for numerical overflow or collapsed geometry.
    pub fn internal_forces(&self) -> Result<Vec<Point>, &'static str> {
        Ok(self
            .evaluate(&self.positions, None, 0.0, &[], &ContactScene::default())?
            .gradient
            .into_iter()
            .map(|v| mul(v, -1.0))
            .collect())
    }
    /// Barrier contact forces at the current pose, including face-interior contact.
    /// # Errors
    /// Rejects invalid colliders, overlap or numerical overflow.
    pub fn contact_forces(&self, contacts: &ContactScene) -> Result<Vec<Point>, &'static str> {
        contacts.validate()?;
        let mut forces = vec![[0.; 3]; self.positions.len()];
        let offset = 0.5 * self.material.thickness();
        for face in &self.faces {
            for sphere in &contacts.spheres {
                if !sphere_in_range(
                    &self.positions,
                    face.ids,
                    sphere,
                    0.,
                    contacts.distance + offset,
                ) {
                    continue;
                }
                let points = variables::<9, 3>(&self.positions, face.ids);
                let center = sphere.center.map(D::c);
                let delta = dsub(closest(points, center), center);
                let gap = ddot(delta, delta).sqrt() - D::c(sphere.radius + offset);
                let energy = contacts.energy(gap)? * face.area;
                if !energy.finite() {
                    return Err("skin contact force overflow");
                }
                for (i, &vertex) in face.ids.iter().enumerate() {
                    for axis in 0..3 {
                        forces[vertex][axis] -= energy.g[i * 3 + axis];
                    }
                }
            }
        }
        for plane in &contacts.planes {
            for (i, &position) in self.positions.iter().enumerate() {
                let point: [D<3>; 3] =
                    std::array::from_fn(|axis| D::variable(position[axis], axis));
                let gap = ddot(point, plane.normal.map(D::c)) - D::c(plane.offset + offset);
                let energy = contacts.energy(gap)? * (self.vertex_area[i]);
                if !energy.finite() {
                    return Err("skin contact force overflow");
                }
                for axis in 0..3 {
                    forces[i][axis] -= energy.g[axis];
                }
            }
        }
        if !forces.iter().copied().all(finite) {
            return Err("skin contact force overflow");
        }
        Ok(forces)
    }
    /// Current energy stored in hyperelastic, bending and Maxwell branches, in joules.
    /// # Errors
    /// Returns an error for numerical overflow or collapsed geometry.
    pub fn stored_energy(&self) -> Result<f64, &'static str> {
        self.objective(&self.positions, None, 0.0, &[], &ContactScene::default())
    }
    /// Principal stretch, area ratio and thickness for each physical triangle.
    /// # Errors
    /// Rejects collapsed or numerically overflowing geometry.
    pub fn surface_metrics(&self) -> Result<Vec<SurfaceMetrics>, &'static str> {
        self.faces
            .iter()
            .map(|f| {
                let c = metric(variables::<0, 3>(&self.positions, f.ids), f).map(|v| v.v);
                let determinant = c[0] * c[1] - c[2] * c[2];
                let largest = 0.5 * (c[0] + c[1] + (c[0] - c[1]).hypot(2. * c[2]));
                let area_ratio = determinant.sqrt();
                let thickness = self.material.thickness() / area_ratio;
                let stretches = [(determinant / largest).sqrt(), largest.sqrt()];
                if !area_ratio.is_finite()
                    || area_ratio <= 1e-6
                    || !thickness.is_finite()
                    || stretches.iter().any(|v| !v.is_finite() || *v <= 0.)
                {
                    return Err("invalid skin surface metrics");
                }
                Ok(SurfaceMetrics {
                    principal_stretches: stretches,
                    area_ratio,
                    thickness,
                })
            })
            .collect()
    }
    /// Incompressible thickness t=t_rest/sqrt(det(C)) for every triangle, in metres.
    /// # Errors
    /// Rejects degenerate or overflowing geometry.
    pub fn thicknesses(&self) -> Result<Vec<f64>, &'static str> {
        self.faces
            .iter()
            .map(|f| {
                let c = metric(variables::<0, 3>(&self.positions, f.ids), f);
                let j = (c[0].v * c[1].v - c[2].v * c[2].v).sqrt();
                if !j.is_finite() || j <= 1e-6 {
                    Err("collapsed skin triangle")
                } else {
                    Ok(self.material.thickness() / j)
                }
            })
            .collect()
    }
    fn face_energy<const N: usize>(
        &self,
        c: [D<N>; 3],
        f: &Face,
        dt: f64,
    ) -> Result<D<N>, &'static str> {
        let mut w = self.material.energy(c, f.direction);
        let e = [
            (c[0] - D::c(1.0)) * 0.5,
            (c[1] - D::c(1.0)) * 0.5,
            c[2] * 0.5,
        ];
        let mut branch = 0;
        for layer in &self.material.layers {
            for r in &layer.relaxation {
                let q = f.memory[branch];
                branch += 1;
                let diff = std::array::from_fn::<_, 3, _>(|i| e[i] - D::c(q[i]));
                w = w
                    + (diff[0].square() + diff[1].square() + diff[2].square() * 2.0)
                        * (0.5 * layer.thickness * r.modulus * (r.time / (r.time + dt)));
            }
        }
        Ok(w * (f.area * f.stiffness_scale))
    }
    fn evaluate(
        &self,
        p: &[Point],
        prediction: Option<(&[Point], f64)>,
        dt: f64,
        attachments: &[Attachment],
        contacts: &ContactScene,
    ) -> Result<Evaluation, &'static str> {
        let mut energy = 0.0;
        let mut blocks = Vec::new();
        let mut gradient = vec![[0.0; 3]; p.len()];
        let mut diagonal = gradient.clone();
        let mut inertia = vec![0.0; p.len()];
        let mut min_area_ratio = f64::INFINITY;
        if let Some((predicted, h)) = prediction {
            for i in 0..p.len() {
                if self.pins.contains_key(&i) {
                    continue;
                }
                inertia[i] = self.mass[i] / (h * h);
                let delta = sub(p[i], predicted[i]);
                energy += 0.5 * inertia[i] * dot(delta, delta);
                gradient[i] = mul(delta, inertia[i]);
                diagonal[i] = [inertia[i]; 3];
            }
        }
        let faces = evaluate_elements(&self.faces, self.element_workers, |f| {
            let c = metric(variables::<9, 3>(p, f.ids), f);
            let det = c[0].v * c[1].v - c[2].v * c[2].v;
            if !det.is_finite() || det <= 1e-12 {
                return Err("collapsed skin triangle");
            }
            let d = self.face_energy(c, f, dt)?;
            Ok((d.v, det.sqrt(), Block::from(&f.ids, d)?))
        })?;
        for (value, area, block) in faces {
            min_area_ratio = min_area_ratio.min(area);
            energy += value;
            blocks.push(block);
        }
        let hinges = evaluate_elements(&self.hinges, self.element_workers, |hinge| {
            let mut delta = angle(variables::<12, 4>(p, hinge.ids)) - D::c(hinge.rest);
            delta.v = (delta.v + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            let d = delta.square() * (0.5 * hinge.stiffness);
            Ok((d.v, Block::from(&hinge.ids, d)?))
        })?;
        for (value, block) in hinges {
            energy += value;
            blocks.push(block);
        }
        // A barrier over closest triangle features catches spheres between mesh vertices.
        for face in &self.faces {
            let points = variables::<9, 3>(p, face.ids);
            for sphere in &contacts.spheres {
                if !sphere_in_range(
                    p,
                    face.ids,
                    sphere,
                    dt,
                    contacts.distance + 0.5 * self.material.thickness(),
                ) {
                    continue;
                }
                let center =
                    std::array::from_fn(|i| D::c(sphere.center[i] + sphere.velocity[i] * dt));
                let point = closest(points, center);
                let delta = dsub(point, center);
                let gap = ddot(delta, delta).sqrt()
                    - D::c(sphere.radius + 0.5 * self.material.thickness());
                let d = contacts.energy(gap)? * face.area;
                energy += d.v;
                blocks.push(Block::from(&face.ids, d)?);
            }
        }
        for plane in &contacts.planes {
            for (i, &point) in p.iter().enumerate() {
                let point: [D<3>; 3] = std::array::from_fn(|j| D::variable(point[j], j));
                let normal = plane.normal.map(D::c);
                let offset = plane.offset + dot(plane.normal, plane.velocity) * dt;
                let gap = ddot(point, normal) - D::c(offset + 0.5 * self.material.thickness());
                let d = contacts.energy(gap)? * (self.vertex_area[i]);
                energy += d.v;
                blocks.push(Block::from(&[i], d)?);
            }
        }
        for a in attachments {
            let i = a.vertex;
            let target = if dt > 0.0 {
                add(a.target, mul(a.velocity, dt))
            } else {
                a.target
            };
            let delta = sub(p[i], target);
            let k = a.stiffness;
            energy += 0.5 * k * dot(delta, delta);
            gradient[i] = add(gradient[i], mul(delta, k));
            diagonal[i] = add(diagonal[i], [k; 3]);
            inertia[i] += k;
            if dt > 0.0 {
                let delta = sub(sub(p[i], self.positions[i]), mul(a.velocity, dt));
                let k = a.viscosity / dt;
                energy += 0.5 * k * dot(delta, delta);
                gradient[i] = add(gradient[i], mul(delta, k));
                diagonal[i] = add(diagonal[i], [k; 3]);
                inertia[i] += k;
            }
        }
        for block in &blocks {
            for (i, &vertex) in block.ids.iter().enumerate() {
                for axis in 0..3 {
                    gradient[vertex][axis] += block.g[i * 3 + axis];
                    let row = i * 3 + axis;
                    diagonal[vertex][axis] += block.h[row * block.g.len() + row];
                }
            }
        }
        if !energy.is_finite() || !gradient.iter().chain(&diagonal).copied().all(finite) {
            return Err("skin objective overflow");
        }
        Ok(Evaluation {
            energy,
            gradient,
            diagonal,
            blocks,
            inertia,
            min_area_ratio,
        })
    }
    fn objective(
        &self,
        p: &[Point],
        prediction: Option<(&[Point], f64)>,
        dt: f64,
        attachments: &[Attachment],
        contacts: &ContactScene,
    ) -> Result<f64, &'static str> {
        let mut energy = 0.0;
        if let Some((predicted, h)) = prediction {
            for i in 0..p.len() {
                if self.pins.contains_key(&i) {
                    continue;
                }
                let inertia = self.mass[i] / (h * h);
                let delta = sub(p[i], predicted[i]);
                energy += 0.5 * inertia * dot(delta, delta);
            }
        }
        for f in &self.faces {
            let c = metric(variables::<0, 3>(p, f.ids), f);
            let det = c[0].v * c[1].v - c[2].v * c[2].v;
            if !det.is_finite() || det <= 1e-12 {
                return Err("collapsed skin triangle");
            }
            let d = self.face_energy(c, f, dt)?;
            energy += d.v;
        }
        for hinge in &self.hinges {
            let mut delta = angle(variables::<0, 4>(p, hinge.ids)) - D::c(hinge.rest);
            delta.v = (delta.v + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            let d = delta.square() * (0.5 * hinge.stiffness);
            energy += d.v;
        }
        // A barrier over closest triangle features catches spheres between mesh vertices.
        for face in &self.faces {
            let points = variables::<0, 3>(p, face.ids);
            for sphere in &contacts.spheres {
                if !sphere_in_range(
                    p,
                    face.ids,
                    sphere,
                    dt,
                    contacts.distance + 0.5 * self.material.thickness(),
                ) {
                    continue;
                }
                let center =
                    std::array::from_fn(|i| D::c(sphere.center[i] + sphere.velocity[i] * dt));
                let point = closest(points, center);
                let delta = dsub(point, center);
                let gap = ddot(delta, delta).sqrt()
                    - D::c(sphere.radius + 0.5 * self.material.thickness());
                let d = contacts.energy(gap)? * face.area;
                energy += d.v;
            }
        }
        for plane in &contacts.planes {
            for (i, &point) in p.iter().enumerate() {
                let point: [D<0>; 3] = point.map(D::c);
                let normal = plane.normal.map(D::c);
                let offset = plane.offset + dot(plane.normal, plane.velocity) * dt;
                let gap = ddot(point, normal) - D::c(offset + 0.5 * self.material.thickness());
                let d = contacts.energy(gap)? * (self.vertex_area[i]);
                energy += d.v;
            }
        }
        for a in attachments {
            let i = a.vertex;
            let target = if dt > 0.0 {
                add(a.target, mul(a.velocity, dt))
            } else {
                a.target
            };
            let delta = sub(p[i], target);
            let k = a.stiffness;
            energy += 0.5 * k * dot(delta, delta);
            if dt > 0.0 {
                let delta = sub(sub(p[i], self.positions[i]), mul(a.velocity, dt));
                let k = a.viscosity / dt;
                energy += 0.5 * k * dot(delta, delta);
            }
        }
        if !energy.is_finite() {
            return Err("skin objective overflow");
        }
        Ok(energy)
    }
    fn apply_into(&self, e: &Evaluation, x: &[Point], shift: f64, out: &mut [Point]) {
        for (i, (&v, result)) in x.iter().zip(out.iter_mut()).enumerate() {
            *result = mul(v, e.inertia[i] + shift);
        }
        for b in &e.blocks {
            for (i, &vertex) in b.ids.iter().enumerate() {
                for a in 0..3 {
                    let row_start = (i * 3 + a) * b.g.len();
                    let row = &b.h[row_start..row_start + b.g.len()];
                    let mut value = out[vertex][a];
                    for (&other, coefficients) in b.ids.iter().zip(row.chunks_exact(3)) {
                        value += coefficients[0] * x[other][0];
                        value += coefficients[1] * x[other][1];
                        value += coefficients[2] * x[other][2];
                    }
                    out[vertex][a] = value;
                }
            }
        }
        for &i in self.pins.keys() {
            out[i] = [0.0; 3];
        }
    }
    fn free_gradient(&self, e: &Evaluation) -> Vec<Point> {
        let mut g = e.gradient.clone();
        for &i in self.pins.keys() {
            g[i] = [0.0; 3];
        }
        g
    }
    fn cg(&self, e: &Evaluation, g: &[Point], shift: f64, limit: usize) -> Option<Vec<Point>> {
        let mut x = vec![[0.0; 3]; g.len()];
        let mut r: Vec<Point> = g.iter().map(|&v| mul(v, -1.0)).collect();
        let precondition = |r: &[Point]| {
            r.iter()
                .enumerate()
                .map(|(i, v)| std::array::from_fn(|a| v[a] / (e.diagonal[i][a] + shift).max(1e-9)))
                .collect::<Vec<Point>>()
        };
        let mut z = precondition(&r);
        let mut d = z.clone();
        let mut rz = vector_dot(&r, &z);
        let tolerance = vector_dot(&r, &r) * 1e-8;
        if rz <= 0.0 {
            return None;
        }
        let mut ad = vec![[0.0; 3]; g.len()];
        for _ in 0..limit {
            self.apply_into(e, &d, shift, &mut ad);
            let denom = vector_dot(&d, &ad);
            if denom <= 1e-30 || !denom.is_finite() {
                return None;
            }
            let alpha = rz / denom;
            for (xi, di) in x.iter_mut().zip(&d) {
                *xi = add(*xi, mul(*di, alpha));
            }
            for (ri, adi) in r.iter_mut().zip(&ad) {
                *ri = sub(*ri, mul(*adi, alpha));
            }
            if vector_dot(&r, &r) <= tolerance {
                return Some(x);
            }
            for (i, (residual, result)) in r.iter().zip(&mut z).enumerate() {
                *result =
                    std::array::from_fn(|a| residual[a] / (e.diagonal[i][a] + shift).max(1e-9));
            }
            let next = vector_dot(&r, &z);
            let beta = next / rz;
            for (di, &zi) in d.iter_mut().zip(&z) {
                *di = add(zi, mul(*di, beta));
            }
            rz = next;
        }
        if vector_dot(g, &x) < 0.0 {
            Some(x)
        } else {
            None
        }
    }
    /// Backward-Euler variational step with Newton/PCG, Hessian regularization and
    /// an energy-decreasing line search. Maxwell histories commit only on success.
    /// Forces are per-vertex newtons; acceleration is uniform m/s².
    /// # Errors
    /// Invalid input, singular geometry, overflow and nonconvergence preserve state.
    pub fn step(
        &mut self,
        dt: f64,
        acceleration: Point,
        forces: &[Point],
        attachments: &[Attachment],
        config: SolverConfig,
    ) -> Result<StepReport, &'static str> {
        self.step_with_contacts(
            dt,
            acceleration,
            forces,
            attachments,
            &ContactScene::default(),
            config,
        )
    }
    /// Implicit step with strictly positive-gap barrier contact.
    /// # Errors
    /// Rejects initial overlap, invalid contact data and nonconverged solves atomically.
    pub fn step_with_contacts(
        &mut self,
        dt: f64,
        acceleration: Point,
        forces: &[Point],
        attachments: &[Attachment],
        contacts: &ContactScene,
        config: SolverConfig,
    ) -> Result<StepReport, &'static str> {
        contacts.validate()?;
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 1.0 / 20.0
            || !finite(acceleration)
            || forces.len() != self.positions.len()
            || !forces.iter().copied().all(finite)
            || !(1..=128).contains(&config.max_newton)
            || !(1..=4096).contains(&config.max_cg)
            || !config.force_tolerance.is_finite()
            || config.force_tolerance <= 0.0
            || !config.relative_tolerance.is_finite()
            || !(0.0..1.0).contains(&config.relative_tolerance)
            || attachments.iter().any(|a| {
                a.vertex >= self.positions.len()
                    || !finite(a.target)
                    || !finite(a.velocity)
                    || !a.stiffness.is_finite()
                    || a.stiffness < 0.0
                    || !a.viscosity.is_finite()
                    || a.viscosity < 0.0
            })
        {
            return Err("invalid skin step");
        }
        let predicted: Vec<Point> = (0..self.positions.len())
            .map(|i| {
                add(
                    add(self.positions[i], mul(self.velocities[i], dt)),
                    mul(
                        add(acceleration, mul(forces[i], 1.0 / self.mass[i])),
                        dt * dt,
                    ),
                )
            })
            .collect();
        if !predicted.iter().copied().all(finite) {
            return Err("skin prediction overflow");
        }
        let mut p = self.positions.clone();
        for (&i, &target) in &self.pins {
            p[i] = target;
        }
        let mut initial = 0.0;
        for iteration in 0..=config.max_newton {
            let e = self.evaluate(&p, Some((&predicted, dt)), dt, attachments, contacts)?;
            let g = self.free_gradient(&e);
            let residual = vector_dot(&g, &g).sqrt();
            if iteration == 0 {
                initial = residual;
            }
            if residual <= config.force_tolerance + config.relative_tolerance * initial {
                if !safe_path(
                    &self.positions,
                    &p,
                    &self.triangles,
                    contacts,
                    0.5 * self.material.thickness(),
                    0.0,
                    dt,
                ) {
                    return Err("skin swept contact or element collapse");
                }

                let mut velocities = Vec::with_capacity(p.len());
                for (i, &point) in p.iter().enumerate() {
                    velocities.push(mul(sub(point, self.positions[i]), 1.0 / dt));
                }
                if !velocities.iter().copied().all(finite) {
                    return Err("skin velocity overflow");
                }
                for f in &mut self.faces {
                    let c = metric(variables::<0, 3>(&p, f.ids), f).map(|v| v.v);
                    let strain = [0.5 * (c[0] - 1.0), 0.5 * (c[1] - 1.0), 0.5 * c[2]];
                    let mut branch = 0;
                    for layer in &self.material.layers {
                        for r in &layer.relaxation {
                            // Algebraically equivalent to (q + dt/tau * E)/(1+dt/tau),
                            // without overflowing dt/tau for valid subnormal tau.
                            let retain = r.time / (r.time + dt);
                            let update = dt / (r.time + dt);
                            f.memory[branch] = std::array::from_fn(|i| {
                                retain * f.memory[branch][i] + update * strain[i]
                            });
                            branch += 1;
                        }
                    }
                }
                self.positions = p;
                self.velocities = velocities;
                return Ok(StepReport {
                    iterations: iteration,
                    residual,
                    energy: e.energy,
                    min_area_ratio: e.min_area_ratio,
                });
            }
            if iteration == config.max_newton {
                break;
            }
            let scale = e
                .diagonal
                .iter()
                .flatten()
                .map(|v| v.abs())
                .fold(1.0, f64::max);
            let mut shift = 0.0;
            let mut accepted = None;
            for _ in 0..16 {
                if let Some(direction) = self.cg(&e, &g, shift, config.max_cg) {
                    let slope = vector_dot(&g, &direction);
                    let mut fraction = 1.0;
                    for _ in 0..32 {
                        let trial: Vec<Point> = p
                            .iter()
                            .zip(&direction)
                            .map(|(&v, &d)| add(v, mul(d, fraction)))
                            .collect();
                        if !safe_path(
                            &p,
                            &trial,
                            &self.triangles,
                            contacts,
                            0.5 * self.material.thickness(),
                            dt,
                            dt,
                        ) {
                            fraction *= 0.5;
                            continue;
                        }
                        if let Ok(candidate) = self.objective(
                            &trial,
                            Some((&predicted, dt)),
                            dt,
                            attachments,
                            contacts,
                        ) {
                            if candidate <= e.energy + 1e-4 * fraction * slope {
                                accepted = Some(trial);
                                break;
                            }
                        }
                        fraction *= 0.5;
                    }
                }
                if accepted.is_some() {
                    break;
                }
                shift = if shift == 0.0 {
                    scale * 1e-6
                } else {
                    shift * 10.0
                };
            }
            p = accepted.ok_or("skin line search failed")?;
        }
        Err("skin Newton did not converge")
    }
}
fn vector_dot(a: &[Point], b: &[Point]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let mut sum = 0.0;
    let chunks_a = a.chunks_exact(4);
    let chunks_b = b.chunks_exact(4);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();
    for (ca, cb) in chunks_a.zip(chunks_b) {
        sum += (ca[0][0] * cb[0][0] + ca[0][1] * cb[0][1] + ca[0][2] * cb[0][2])
            + (ca[1][0] * cb[1][0] + ca[1][1] * cb[1][1] + ca[1][2] * cb[1][2])
            + (ca[2][0] * cb[2][0] + ca[2][1] * cb[2][1] + ca[2][2] * cb[2][2])
            + (ca[3][0] * cb[3][0] + ca[3][1] * cb[3][1] + ca[3][2] * cb[3][2]);
    }
    for (va, vb) in rem_a.iter().zip(rem_b) {
        sum += va[0] * vb[0] + va[1] * vb[1] + va[2] * vb[2];
    }
    sum
}
#[cfg(test)]
mod objective_tests {
    use super::*;
    use crate::skin::{ContactPlane, ContactSphere};

    #[test]
    fn contiguous_hessian_rows_preserve_operator_bits() {
        let skin = patch(5, 5, 0.01, SkinMaterial::default()).unwrap();
        let posed: Vec<_> = skin.rest.iter()
            .map(|p| [1.02 * p[0], 0.98 * p[1], 0.003 * (20. * p[0]).sin()]).collect();
        let e = skin.evaluate(&posed, Some((&skin.rest, 1. / 120.)),
            1. / 120., &[], &ContactScene::default()).unwrap();
        let x: Vec<_> = posed.iter().enumerate()
            .map(|(i, p)| [p[0] - 0.01, p[1] + 0.002, (i as f64 * 0.7).sin()]).collect();
        for shift in [0., 0.001, 100.] {
            let mut expected: Vec<_> = x.iter().enumerate()
                .map(|(i, &v)| mul(v, e.inertia[i] + shift)).collect();
            for b in &e.blocks {
                for (i, &vertex) in b.ids.iter().enumerate() {
                    for a in 0..3 {
                        for (j, &other) in b.ids.iter().enumerate() {
                            for c in 0..3 {
                                expected[vertex][a] +=
                                    b.h[(i * 3 + a) * b.g.len() + j * 3 + c] * x[other][c];
                            }
                        }
                    }
                }
            }
            for &i in skin.pins.keys() { expected[i] = [0.; 3]; }
            let mut actual = vec![[0.; 3]; x.len()];
            skin.apply_into(&e, &x, shift, &mut actual);
            for (actual, expected) in actual.iter().flatten().zip(expected.iter().flatten()) {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
    }

    #[test]
    fn parallel_elements_preserve_derivatives_and_integrated_state_exactly() {
        let serial = patch(12, 12, 0.01, SkinMaterial::default()).unwrap();
        let mut parallel = serial.clone();
        assert!(parallel.set_element_workers(0).is_err());
        assert!(parallel.set_element_workers(65).is_err());
        parallel.set_element_workers(4).unwrap();
        let posed: Vec<_> = serial
            .rest
            .iter()
            .map(|p| [1.02 * p[0], 0.98 * p[1], 0.003 * (20. * p[0]).sin()])
            .collect();
        let contacts = ContactScene::default();
        let a = serial
            .evaluate(&posed, None, 1. / 120., &[], &contacts)
            .unwrap();
        let b = parallel
            .evaluate(&posed, None, 1. / 120., &[], &contacts)
            .unwrap();
        assert_eq!(a.energy.to_bits(), b.energy.to_bits());
        assert_eq!(a.min_area_ratio.to_bits(), b.min_area_ratio.to_bits());
        assert_eq!(a.gradient, b.gradient);
        assert_eq!(a.diagonal, b.diagonal);
        assert_eq!(a.blocks.len(), b.blocks.len());
        for (a, b) in a.blocks.iter().zip(&b.blocks) {
            assert_eq!(a.ids, b.ids);
            assert_eq!(a.g, b.g);
            assert_eq!(a.h, b.h);
        }
        let mut serial = serial;
        let forces = vec![[0.; 3]; serial.positions.len()];
        for _ in 0..3 {
            serial
                .step(
                    1. / 120.,
                    [0., 0., -9.81],
                    &forces,
                    &[],
                    SolverConfig::default(),
                )
                .unwrap();
            parallel
                .step(
                    1. / 120.,
                    [0., 0., -9.81],
                    &forces,
                    &[],
                    SolverConfig::default(),
                )
                .unwrap();
            assert_eq!(serial.positions(), parallel.positions());
            assert_eq!(serial.velocities(), parallel.velocities());
        }
    }

    #[test]
    fn scalar_objective_matches_full_derivatives_with_history_and_contacts() {
        let mut skin = patch(4, 4, 0.01, SkinMaterial::default()).unwrap();
        skin.pins.insert(0, [0.; 3]);
        for face in &mut skin.faces {
            for memory in &mut face.memory {
                *memory = [0.02, -0.01, 0.003];
            }
        }
        let attachments = [Attachment {
            vertex: 5,
            target: [0.011, 0.012, 0.],
            velocity: [0.002, -0.001, 0.003],
            stiffness: 120.,
            viscosity: 0.4,
        }];
        let contacts = ContactScene {
            spheres: vec![ContactSphere {
                center: [0.015, 0.015, -0.008],
                radius: 0.006,
                velocity: [0.0001, 0., 0.],
            }],
            planes: vec![ContactPlane {
                normal: [0., 0., 1.],
                offset: -0.002,
                velocity: [0., 0., 0.0001],
            }],
            distance: 0.003,
            ..ContactScene::default()
        };
        for sample in 0..20 {
            let t = f64::from(sample) * 0.001;
            let p: Vec<_> = skin
                .rest
                .iter()
                .map(|v| [v[0] * (1. + t), v[1] * (1. - t * 0.3), v[0] * v[1] * t])
                .collect();
            let prediction = Some((skin.positions.as_slice(), 1. / 120.));
            let full = skin
                .evaluate(&p, prediction, 1. / 120., &attachments, &contacts)
                .unwrap();
            let scalar = skin
                .objective(&p, prediction, 1. / 120., &attachments, &contacts)
                .unwrap();
            assert!((full.energy - scalar).abs() <= 1e-13 * (1. + full.energy.abs()));
            skin.positions = p;
            let loaded = skin
                .evaluate(&skin.positions, None, 0., &[], &contacts)
                .unwrap();
            let bare = skin
                .evaluate(&skin.positions, None, 0., &[], &ContactScene::default())
                .unwrap();
            let direct = skin.contact_forces(&contacts).unwrap();
            for i in 0..direct.len() {
                for axis in 0..3 {
                    assert!(
                        (direct[i][axis] - (bare.gradient[i][axis] - loaded.gradient[i][axis]))
                            .abs()
                            < 1e-10
                    );
                }
            }
        }
        let collapsed = vec![[0.; 3]; skin.positions.len()];
        assert!(
            skin.objective(&collapsed, None, 0., &[], &contacts)
                .is_err()
        );
    }

    #[test]
    #[ignore = "manual release performance measurement"]
    fn benchmark_objective_evaluations() {
        let skin = patch(20, 20, 0.01, SkinMaterial::default()).unwrap();
        let p: Vec<_> = skin.rest.iter().map(|v| [v[0] * 1.02, v[1], 0.]).collect();
        let contacts = ContactScene::default();
        let started = std::time::Instant::now();
        for _ in 0..100 {
            std::hint::black_box(
                skin.evaluate(std::hint::black_box(&p), None, 0.01, &[], &contacts)
                    .unwrap(),
            );
        }
        let full = started.elapsed().as_secs_f64();
        let started = std::time::Instant::now();
        for _ in 0..100 {
            std::hint::black_box(
                skin.objective(std::hint::black_box(&p), None, 0.01, &[], &contacts)
                    .unwrap(),
            );
        }
        let scalar = started.elapsed().as_secs_f64();
        println!(
            "SKIN OBJECTIVE: full {:.3} ms, scalar {:.3} ms per evaluation; ratio {:.2}",
            full * 10.,
            scalar * 10.,
            full / scalar
        );
    }
}
/// A regular stress-free XY patch, with rest-space X as its collagen axis.
/// # Errors
/// Rejects invalid size, dimensions, material or topology.
pub fn patch(
    columns: usize,
    rows: usize,
    spacing: f64,
    material: SkinMaterial,
) -> Result<Skin, &'static str> {
    if !(2..=128).contains(&columns)
        || !(2..=128).contains(&rows)
        || !spacing.is_finite()
        || spacing <= 0.0
    {
        return Err("invalid skin patch");
    }
    let rest = (0..columns * rows)
        .map(|i| {
            [
                (i % columns) as f64 * spacing,
                (i / columns) as f64 * spacing,
                0.0,
            ]
        })
        .collect();
    let mut triangles = Vec::new();
    for y in 0..rows - 1 {
        for x in 0..columns - 1 {
            let a = y * columns + x;
            triangles.extend([
                [a, a + 1, a + columns + 1],
                [a, a + columns + 1, a + columns],
            ]);
        }
    }
    let count = triangles.len();
    Skin::new(rest, triangles, &[], material, vec![[1.0, 0.0, 0.0]; count])
}
