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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContactSource {
    Mesh(usize),
    Strand { other_rod: usize, other_segment: usize },
}
#[derive(Clone, Debug)]
struct RodContact {
    segment: usize,
    fraction: f64,
    normal: V,
    target: V,
    source: ContactSource,
    surface_velocity: V,
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
    contacts: Vec<RodContact>,
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
    surface_velocity_sum: Vec<V>,
    surface_velocity_weight: Vec<f64>,
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
            contacts: Vec::new(),
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
            surface_velocity_sum: vec![[0.; 3]; n],
            surface_velocity_weight: vec![0.; n],
            material,
        })
    }
    fn record_contact(&mut self, mut segment:usize, mut fraction:f64, normal:V, target:V, source:ContactSource) -> usize {
        // Canonicalize a shared endpoint so node and capsule queries do not
        // count the same geometric contact twice.
        if fraction==1. && segment+1<self.lengths.len() {segment+=1;fraction=0.;}
        if let Some(index)=self.contacts.iter().position(|contact| {
            contact.source==source && contact.segment==segment
                && (contact.fraction-fraction).abs()*self.lengths[segment]<=1e-12
                && dot(contact.normal,normal)>1.-1e-12
                && dot(sub(contact.target,target),normal).abs()<=1e-12
        }) {
            let existing=&mut self.contacts[index];
            existing.fraction=fraction;existing.normal=normal;existing.target=target;
            index
        } else {self.contacts.push(RodContact {segment,fraction,normal,target,source,surface_velocity:[0.;3]});self.contacts.len()-1}
    }
    fn record_point_contact(&mut self, point:usize, normal:V, target:V, source:ContactSource) -> usize {
        let segment=point.min(self.lengths.len()-1);
        let fraction=if point==self.lengths.len() {1.} else {0.};
        self.record_contact(segment,fraction,normal,target,source)
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
        self.surface_velocity_sum.fill([0.;3]);
        self.surface_velocity_weight.fill(0.);
        self.contacts.clear();
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
    fn solve(&mut self, dt: f64, _reverse: bool) -> Result<(), &'static str> {
        direct::solve(self, dt)
    }
    fn finish(&mut self, dt: f64) {
        for i in 0..self.x.len() {
            self.velocity[i] = mul(sub(self.x[i], self.old_x[i]), 1. / dt);
            let normal = self.normals[i];
            if len(normal) > 1e-12 {
                let n = unit(normal);
                let surface=mul(self.surface_velocity_sum[i],1./self.surface_velocity_weight[i].max(1e-30));
                let relative=sub(self.velocity[i],surface);
                let vn = dot(relative, n);
                let tangent = sub(relative, mul(n, vn));
                let removed = self.material.friction * vn.abs();
                let factor = (1. - removed / len(tangent).max(1e-30)).max(0.);
                self.velocity[i] = add(surface,add(mul(tangent, factor), mul(n, vn.max(0.))));
            }
        }
        for i in 0..self.q.len() {
            self.omega[i] = mul(log(qm(self.q[i], conj(self.old_q[i]))), 1. / dt);
        }
    }
    fn record_surface_response(&mut self, point: usize, correction: V, velocity: V) {
        self.normals[point]=add(self.normals[point],correction);
        let weight=len(correction);
        self.surface_velocity_sum[point]=add(self.surface_velocity_sum[point],mul(velocity,weight));
        self.surface_velocity_weight[point]+=weight;
    }
}
/// Solver phase costs. Independent guide costs are summed worker times, not wall time.
#[derive(Clone, Copy, Debug, Default)]
pub struct HairStepProfile {
    pub structural_ms: f64,
    pub mesh_contacts_ms: f64,
    pub self_contacts_ms: f64,
}
/// Native assembled Cosserat system for accelerator qualification.
/// Lower triangular row bands include the diagonal at offset zero.
#[derive(Clone, Debug)]
pub struct HairLinearSystem {
    pub band_width: usize,
    pub matrix: Vec<f64>,
    pub rhs: Vec<f64>,
    pub active: std::ops::Range<usize>,
}
/// Accelerator ownership remains outside physics; one callback receives all
/// independent rod matrices from the current nonlinear iteration.
pub trait HairLinearSolver {
    fn solve(&mut self, systems: &[HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str>;
}
impl HairLinearSystem {
    fn validate_shape(&self) -> Result<(), &'static str> {
        if self.band_width != direct::BAND || self.rhs.len() < 12 || self.rhs.len() % 6 != 0
            || self.matrix.len() != self.rhs.len() * direct::BAND || self.active != (6..self.rhs.len()-3)
            || self.matrix.iter().chain(&self.rhs).any(|v| !v.is_finite()) {return Err("invalid hair linear system");}
        Ok(())
    }
    pub fn solve_native(&self) -> Result<Vec<f64>, &'static str> {
        self.validate_shape()?;
        let mut matrix=self.matrix.clone();let mut rhs=self.rhs.clone();
        direct::cholesky(&mut matrix,&mut rhs,self.active.clone());
        self.validate_correction(&rhs)?;Ok(rhs)
    }
    /// Admit an accelerator result against the original physical system.
    pub fn validate_correction(&self, values:&[f64]) -> Result<(), &'static str> {
        self.validate_shape()?;
        if values.len()!=self.rhs.len() || values.iter().any(|v|!v.is_finite()) {return Err("invalid hair accelerator correction");}
        if (0..self.active.start).chain(self.active.end..values.len()).any(|i|values[i]!=self.rhs[i]) {return Err("hair accelerator changed fixed DOFs");}
        for i in self.active.clone() {
            let (ax,scale)=self.row_product(values,i);
            if !ax.is_finite() || !scale.is_finite() || (ax-self.rhs[i]).abs()>1e-8*scale.max(1e-30) {return Err("hair accelerator residual exceeds tolerance");}
        }
        Ok(())
    }
    fn row_product(&self,values:&[f64],i:usize)->(f64,f64) {
        let term=self.matrix[i*direct::BAND]*values[i];let mut ax=term;let mut scale=term.abs()+self.rhs[i].abs();
        for j in i.saturating_sub(direct::BAND-1).max(self.active.start)..i {
            let term=self.matrix[i*direct::BAND+i-j]*values[j];ax+=term;scale+=term.abs();
        }
        for j in i+1..(i+direct::BAND).min(self.active.end) {
            let term=self.matrix[j*direct::BAND+j-i]*values[j];ax+=term;scale+=term.abs();
        }
        (ax,scale)
    }
    /// Original f64 residual for mixed-precision refinement. Fixed degrees of
    /// freedom receive zero residual and therefore no refinement increment.
    pub fn correction_residual(&self,values:&[f64])->Result<Vec<f64>, &'static str> {
        self.validate_shape()?;
        if values.len()!=self.rhs.len() || values.iter().any(|v|!v.is_finite()) {return Err("invalid hair residual correction");}
        let mut residual=vec![0.;values.len()];
        for i in self.active.clone() {
            residual[i]=self.rhs[i]-self.row_product(values,i).0;
            if !residual[i].is_finite() {return Err("hair correction residual overflow");}
        }
        Ok(residual)
    }
}
impl HairRod {
    /// Capture the same assembly used by the native solver without changing the rod.
    pub fn linear_system(&self, dt: f64) -> Result<HairLinearSystem, &'static str> {
        if !dt.is_finite() || !(1e-6..=1./20.).contains(&dt) { return Err("invalid hair linear-system timestep"); }
        let mut staged = self.clone();
        let (matrix, rhs) = direct::assemble(&mut staged, dt)?;
        if matrix.iter().chain(&rhs).any(|v| !v.is_finite()) { return Err("hair linear-system overflow"); }
        let count = rhs.len();
        Ok(HairLinearSystem { band_width: direct::BAND, matrix, rhs, active: 6..count-3 })
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
    /// Experimental joint normal-velocity projection. Full-model stretch and
    /// long-cycle accelerator qualification remain required before default use.
    pub joint_contact_velocities: bool,
    /// Experimental joint elastic position contact projection.
    pub joint_contact_positions: bool,
    /// Experimental time-aligned mesh sampling; full-model stretch qualification is pending.
    pub sample_collider_motion: bool,
    /// Experimental structural reconciliation after the final contact projection.
    pub terminal_contact_iterations: usize,
    pub profiling: bool,
    pub last_profile: HairStepProfile,
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
            joint_contact_velocities: false,
            joint_contact_positions: false,
            sample_collider_motion: false,
            terminal_contact_iterations: 0,
            profiling: false,
            last_profile: HairStepProfile::default(),
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
        system.joint_contact_velocities = self.joint_contact_velocities;
        system.joint_contact_positions = self.joint_contact_positions;
        system.sample_collider_motion = self.sample_collider_motion;
        system.terminal_contact_iterations = self.terminal_contact_iterations;
        system.profiling = self.profiling;
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
        let mut staged=self.clone();
        staged.step_impl(dt,roots,gravity,air_velocity,meshes,None)?;
        *self=staged;
        Ok(())
    }
    /// The complete step commits only after the accelerator and contacts succeed.
    pub fn step_with_solver(&mut self,dt:f64,roots:&[RootPose],gravity:V,air_velocity:V,meshes:&[TriangleMesh],solver:&mut dyn HairLinearSolver)->Result<(), &'static str> {
        let mut staged=self.clone();
        staged.step_impl(dt,roots,gravity,air_velocity,meshes,Some(solver))?;
        *self=staged;Ok(())
    }
    fn solve_external_rods(rods:&mut [HairRod],dt:f64,solver:&mut dyn HairLinearSolver)->Result<(), &'static str> {
        let systems=rods.iter_mut().map(|rod| {
            let (matrix,rhs)=direct::assemble(rod,dt)?;let n=rhs.len();
            Ok(HairLinearSystem {band_width:direct::BAND,matrix,rhs,active:6..n-3})
        }).collect::<Result<Vec<_>, &'static str>>()?;
        let corrections=solver.solve(&systems)?;
        if corrections.len()!=systems.len() {return Err("hair accelerator changed batch size");}
        for (system,correction) in systems.iter().zip(&corrections) {system.validate_correction(correction)?;}
        for ((rod,system),correction) in rods.iter_mut().zip(systems).zip(corrections) {
            direct::apply_correction(rod,&correction);
            rod.solve_matrix=system.matrix;rod.solve_rhs=system.rhs;
        }
        Ok(())
    }
    fn reconcile_positions(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,self_collision:bool,history:&mut Vec<contact::StrandResponse>,workers:usize)->Result<(), &'static str> {
        let mut last_worst=None;
        let mut first_gap=0.;
        for iteration in 0..32 {
            let mut current=if self_collision {contact::refresh_strand_responses(rods,radius,&[])} else {Vec::new()};
            for rod in rods.iter_mut() {contact::refresh_mesh_constraints(rod,meshes,radius);}
            let complete=contact::reconcile_contact_positions(rods,&mut current,dt,radius,workers)?;
            // The common trust scale preserves paired reactions. Re-query all
            // geometry before the next nonlinear increment rather than fixing
            // one strand to the other's preceding position.
            history.extend(current);
            // A solved tangent problem is not proof of separation in the new
            // nonlinear geometry: rotations can change a closest feature or
            // create another pair. Admit only a freshly queried feasible pose.
            if self_collision {let _=contact::refresh_strand_responses(rods,radius,&[]);}
            for rod in rods.iter_mut() {contact::refresh_mesh_constraints(rod,meshes,radius);}
            let mut unresolved=false;
            let mut worst=None;
            for (index,rod) in rods.iter().enumerate() {
                for contact in &rod.contacts {
                    let i=contact.segment;let t=contact.fraction;
                    let position=add(mul(rod.x[i],1.-t),mul(rod.x[i+1],t));
                    let gap=dot(sub(position,contact.target),contact.normal);
                    if !gap.is_finite() {return Err("joint hair contact geometry overflow");}
                    unresolved|=gap < -1e-10;
                    if gap < worst.as_ref().map_or(0.,|value:&(usize,usize,f64,f64,ContactSource,V,V,V,V,V)|value.3) {
                        worst=Some((index,i,t,gap,contact.source,rod.x[0],rod.x[i],rod.x[i+1],contact.normal,contact.target));
                    }
                }
            }
            for rod in rods.iter_mut() {rod.contacts.retain(|contact|matches!(contact.source,ContactSource::Mesh(_)));}
            if complete && !unresolved {return Ok(());}
            if iteration==0 {first_gap=worst.as_ref().map_or(0.,|value|value.3);}
            last_worst=worst;
        }
        eprintln!("HAIR NONLINEAR CONTACT NONCONVERGENCE first_gap_m={first_gap} worst=(rod,segment,fraction,gap,source,root,a,b,normal,target) {last_worst:?}");
        Err("joint hair contact position linearization did not converge")
    }
    fn step_impl(&mut self,dt:f64,roots:&[RootPose],gravity:V,air_velocity:V,meshes:&[TriangleMesh],mut solver:Option<&mut dyn HairLinearSolver>)->Result<(), &'static str> {
        if !dt.is_finite()
            || dt < 1e-6
            || dt > 1. / 20.
            || self.substeps == 0
            || self.substeps > 32
            || self.workers == 0
            || self.workers > 64
            || self.iterations == 0
            || self.iterations > 128
            || self.terminal_contact_iterations > 32
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
        if self.sample_collider_motion && meshes.iter().filter_map(TriangleMesh::motion_duration).any(|duration|(duration-dt).abs()>1e-12*dt) {
            return Err("collider motion duration differs from hair step");
        }
        let mut sampled_meshes=(self.sample_collider_motion && meshes.iter().any(|mesh|mesh.motion_duration().is_some())).then(||meshes.to_vec());
        self.last_profile = HairStepProfile::default();
        let profiling = self.profiling;
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
            let meshes=if let Some(sampled)=&mut sampled_meshes {
                for (sample,source) in sampled.iter_mut().zip(meshes) {
                    sample.sample_motion(source,(substep+1) as f64/substeps as f64)?;
                }
                for rod in &mut self.rods {for cache in &mut rod.clearance_cache {cache.fill(None);}}
                sampled.as_slice()
            } else {meshes};
            let mut strand_responses=Vec::new();
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
            let batch = if self.joint_contact_positions {
                1
            } else if parallel {
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
                let joint_positions=self.joint_contact_positions;
                if let Some(solver)=solver.as_deref_mut() {
                    for _ in first..end {
                        let started=profiling.then(std::time::Instant::now);
                        Self::solve_external_rods(&mut self.rods,dt,solver)?;
                        if let Some(started)=started {self.last_profile.structural_ms+=started.elapsed().as_secs_f64()*1000.;}
                        let started=profiling.then(std::time::Instant::now);
                        for rod in &mut self.rods {if joint_positions {contact::refresh_mesh_constraints(rod,meshes,radius);} else {contact::mesh_contacts(rod,meshes,radius);}}
                        if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                    }
                } else if !parallel {
                    for rod in &mut self.rods {
                        let started = profiling.then(std::time::Instant::now);
                        rod.solve(dt, first % 2 != 0)?;
                        if let Some(started) = started { self.last_profile.structural_ms += started.elapsed().as_secs_f64() * 1000.; }
                        let started = profiling.then(std::time::Instant::now);
                        if joint_positions {contact::refresh_mesh_constraints(rod,meshes,radius);} else {contact::mesh_contacts(rod, meshes, radius);}
                        if let Some(started) = started { self.last_profile.mesh_contacts_ms += started.elapsed().as_secs_f64() * 1000.; }
                    }
                } else {
                    let chunk = self.rods.len().div_ceil(self.workers);
                    let profiles = std::thread::scope(|scope| {
                        let mut workers = Vec::new();
                        for rods in self.rods.chunks_mut(chunk) {
                            workers.push(scope.spawn(move || {
                                let mut profile = HairStepProfile::default();
                                for rod in rods {
                                    for iteration in first..end {
                                        let started = profiling.then(std::time::Instant::now);
                                        rod.solve(dt, iteration % 2 != 0)?;
                                        if let Some(started) = started { profile.structural_ms += started.elapsed().as_secs_f64() * 1000.; }
                                        let started = profiling.then(std::time::Instant::now);
                                        if joint_positions {contact::refresh_mesh_constraints(rod,meshes,radius);} else {contact::mesh_contacts(rod, meshes, radius);}
                                        if let Some(started) = started { profile.mesh_contacts_ms += started.elapsed().as_secs_f64() * 1000.; }
                                    }
                                }
                                Ok::<_, &'static str>(profile)
                            }));
                        }
                        workers.into_iter().map(|worker| worker.join().map_err(|_| "hair profile worker panicked").and_then(|result| result)).collect::<Result<Vec<_>, _>>()
                    })?;
                    for profile in profiles {
                        self.last_profile.structural_ms += profile.structural_ms;
                        self.last_profile.mesh_contacts_ms += profile.mesh_contacts_ms;
                    }
                }
                if joint_positions && !self.self_collision {
                    let started=profiling.then(std::time::Instant::now);
                    Self::reconcile_positions(&mut self.rods,meshes,dt,radius,false,&mut strand_responses,self.workers)?;
                    if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                }
                if self.self_collision && (joint_positions || end % 4 == 0 || end == self.iterations) {
                    let started = profiling.then(std::time::Instant::now);
                    if joint_positions {
                        Self::reconcile_positions(&mut self.rods,meshes,dt,radius,true,&mut strand_responses,self.workers)?;
                    } else {strand_responses.extend(contact::self_contacts(&mut self.rods, radius));}
                    if let Some(started) = started { self.last_profile.self_contacts_ms += started.elapsed().as_secs_f64() * 1000.; }
                    // A strand reaction can move a guide into the body after
                    // its independent mesh phase. Refresh those constraints
                    // before the shared velocity projection.
                    if self.joint_contact_velocities {
                        let started=profiling.then(std::time::Instant::now);
                        for rod in &mut self.rods {contact::refresh_mesh_constraints(rod,meshes,radius);}
                        if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                    }
                }
            }
            // Position projection must not be the last operation on an elastic rod:
            // reconcile its physical strain with the final unilateral constraints.
            for _ in 0..self.terminal_contact_iterations {
                let started=profiling.then(std::time::Instant::now);
                if let Some(solver)=solver.as_deref_mut() {
                    Self::solve_external_rods(&mut self.rods,dt,solver)?;
                } else {for rod in &mut self.rods {direct::solve(rod,dt)?;}}
                if let Some(started)=started {self.last_profile.structural_ms+=started.elapsed().as_secs_f64()*1000.;}
                let started=profiling.then(std::time::Instant::now);
                for rod in &mut self.rods {contact::refresh_mesh_constraints(rod,meshes,self.contact_radius);}
                if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
            }
            if self.self_collision {
                let started=profiling.then(std::time::Instant::now);
                strand_responses=contact::refresh_strand_responses(&mut self.rods,self.contact_radius,&strand_responses);
                if let Some(started)=started {self.last_profile.self_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
            }
            for rod in &mut self.rods {
                rod.finish(dt);
            }
            contact::finish_strand_contacts(&mut self.rods,&strand_responses,dt);
            if self.joint_contact_velocities {
                contact::stabilize_contact_velocities(&mut self.rods,&strand_responses,dt,self.contact_radius,self.workers)?;
            }
        }
        if self.rods.iter().any(|r| {
            r.x.iter().chain(&r.velocity).chain(&r.omega).any(|p| !finite(*p)) || r.q.iter().flatten().any(|q| !q.is_finite())
        }) {
            return Err("hair solver overflow");
        }
        Ok(())
    }
}
