//! SI-unit Cosserat guide rods with implicit stretch/shear and material-frame bend/twist.
//! No attraction of free particles to world-space groom positions.
mod contact;
mod direct;
mod math;
mod contact_replay;
mod contact_model;
pub use contact::{TriangleMotion,sweep_capsule_triangle,TriangleMesh,CapsuleMotion,CapsuleSweepOptions,CapsuleSweep,sweep_capsules,swept_capsule_pairs,swept_capsule_contacts};
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
pub enum ContactSource {
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
    // Physical signed residual = metric_scale * unit-normal plane residual.
    metric_scale:f64,
    trajectory_time:Option<f64>,
}
impl RodContact {
    fn physical_gap(&self,p:V)->f64 {self.metric_scale*dot(sub(p,self.target),self.normal)}
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
#[path="contact_diagnostics.rs"]
mod contact_diagnostics;
pub use contact_diagnostics::HairContactDiagnostic;
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
            contact.trajectory_time.is_none() && contact.source==source && contact.segment==segment
                && (contact.fraction-fraction).abs()*self.lengths[segment]<=1e-12
                && dot(contact.normal,normal)>1.-1e-12
                && dot(sub(contact.target,target),normal).abs()<=1e-12
        }) {
            let existing=&mut self.contacts[index];
            existing.fraction=fraction;existing.normal=normal;existing.target=target;existing.metric_scale=1.;
            index
        } else {self.contacts.push(RodContact {segment,fraction,normal,target,source,surface_velocity:[0.;3],metric_scale:1.,trajectory_time:None});self.contacts.len()-1}
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
#[path = "contact_response_system.rs"]
mod contact_response_system;
pub use contact_response_system::HairResponseSystem;
/// Accelerator ownership remains outside physics; one callback receives all
/// independent rod matrices from the current nonlinear iteration.
pub trait HairLinearSolver {
    /// Explicit qualification opt-in for contact response batches.
    fn contact_responses_enabled(&self)->bool {false}
    fn solve(&mut self, systems: &[HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str>;
    /// Shared-matrix contact loads. Existing backends retain the native path.
    fn solve_responses(&mut self,systems:&[HairResponseSystem])->Result<Vec<Vec<Vec<f64>>>, &'static str> {
        systems.iter().map(HairResponseSystem::solve_native).collect()
    }
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
        self.validate_for_rhs(values,&self.rhs)
    }
    /// Admit a response to another load on this same physical matrix.
    pub fn validate_load_correction(&self,values:&[f64],rhs:&[f64])->Result<(), &'static str> {
        self.validate_shape()?;
        self.validate_for_rhs(values,rhs)
    }
    fn validate_for_rhs(&self,values:&[f64],rhs:&[f64])->Result<(), &'static str> {
        if rhs.len()!=self.rhs.len() || rhs.iter().any(|v|!v.is_finite()) {return Err("invalid hair response load");}
        if values.len()!=self.rhs.len() || values.iter().any(|v|!v.is_finite()) {return Err("invalid hair accelerator correction");}
        if (0..self.active.start).chain(self.active.end..values.len()).any(|i|values[i]!=rhs[i]) {return Err("hair accelerator changed fixed DOFs");}
        for i in self.active.clone() {
            let (ax,scale)=self.row_product(values,i,rhs);
            if !ax.is_finite() || !scale.is_finite() || (ax-rhs[i]).abs()>1e-8*scale.max(1e-30) {return Err("hair accelerator residual exceeds tolerance");}
        }
        Ok(())
    }
    fn row_product(&self,values:&[f64],i:usize,rhs:&[f64])->(f64,f64) {
        let term=self.matrix[i*direct::BAND]*values[i];let mut ax=term;let mut scale=term.abs()+rhs[i].abs();
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
        self.load_residual(values,&self.rhs)
    }
    pub fn load_residual(&self,values:&[f64],rhs:&[f64])->Result<Vec<f64>, &'static str> {
        self.validate_shape()?;
        if rhs.len()!=self.rhs.len() || rhs.iter().any(|v|!v.is_finite()) {return Err("invalid hair response load");}
        if values.len()!=self.rhs.len() || values.iter().any(|v|!v.is_finite()) {return Err("invalid hair residual correction");}
        let mut residual=vec![0.;values.len()];
        for i in self.active.clone() {
            residual[i]=rhs[i]-self.row_product(values,i,rhs).0;
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
#[path="phase_diagnostics.rs"]
mod phase_diagnostics;
pub use phase_diagnostics::{HairPhaseDiagnostic,HairFrictionDiagnostic};
mod projection_diagnostics;
pub use projection_diagnostics::{HairContactProjectionDiagnostic,HairProjectionPairDiagnostic};
use projection_diagnostics::HairProjectionTrace;
#[derive(Clone, Debug)]
pub struct HairSystem {
    rods: Vec<HairRod>,
    trace_rod:Option<usize>,
    last_trace:Vec<HairPhaseDiagnostic>,
    last_friction_trace:Vec<HairFrictionDiagnostic>,
    last_projection_trace:Vec<HairContactProjectionDiagnostic>,
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
    /// Experimental constrained structural motion with swept strand admission.
    /// Mesh contacts retain nonlinear discrete admission; no moving-mesh CCD.
    pub swept_strand_positions: bool,
    /// Experimental continuous mesh trajectory admission. Joint positions also
    /// resolve discovered space-time contacts through the shared nonlinear loop.
    /// Unknown paths and unsupported oriented crossings reject the staged step.
    pub continuous_mesh_admission: bool,
    swept_strand_initialized: bool,
    /// Experimental force-balanced friction pressure; full jump qualification pending.
    pub recover_friction_pressure: bool,
    /// Experimental time-aligned mesh sampling; full-model stretch qualification is pending.
    pub sample_collider_motion: bool,
    /// Experimental additional structural iterations. With joint positions,
    /// each elastic correction is followed by fresh nonlinear contact admission.
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
            trace_rod:None,
            last_trace:Vec::new(),
            last_friction_trace:Vec::new(),
            last_projection_trace:Vec::new(),
            iterations: 16,
            workers: 1,
            substeps: 4,
            contact_radius: 40e-6,
            self_collision: true,
            joint_contact_velocities: false,
            joint_contact_positions: false,
            swept_strand_positions: false,
            continuous_mesh_admission: false,
            swept_strand_initialized: false,
            recover_friction_pressure: false,
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
        system.trace_rod=self.trace_rod;
        system.joint_contact_velocities = self.joint_contact_velocities;
        system.joint_contact_positions = self.joint_contact_positions;
        system.swept_strand_positions = self.swept_strand_positions;
        system.continuous_mesh_admission = self.continuous_mesh_admission;
        system.recover_friction_pressure = self.recover_friction_pressure;
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
        self.step_validated(dt,roots,gravity,air_velocity,meshes,None, |_| Ok(()))
    }
    /// The complete step commits only after the accelerator and contacts succeed.
    pub fn step_with_solver(&mut self,dt:f64,roots:&[RootPose],gravity:V,air_velocity:V,meshes:&[TriangleMesh],solver:&mut dyn HairLinearSolver)->Result<(), &'static str> {
        self.step_validated(dt,roots,gravity,air_velocity,meshes,Some(solver), |_| Ok(()))
    }
    /// Apply application-specific admission before publishing a solved state.
    /// Rejection preserves poses, velocities, contact history and diagnostics.
    /// Accelerator side effects are external and cannot be rolled back.
    pub fn step_validated(
        &mut self, dt:f64, roots:&[RootPose], gravity:V, air_velocity:V,
        meshes:&[TriangleMesh], solver:Option<&mut dyn HairLinearSolver>,
        validate:impl FnOnce(&HairSystem)->Result<(), &'static str>,
    )->Result<(), &'static str> {
        let mut staged=self.clone();
        let result=staged.step_impl(dt,roots,gravity,air_velocity,meshes,solver)
            .and_then(|()| {
                let admission=validate(&staged);
                if admission.is_err() {
                    // Only completed steps use the endpoint collider geometry.
                    // Capture before rollback, including prediction/inertia data
                    // needed to reproduce elastic corrections after contact.
                    if let Some(path)=std::env::var_os("VOXY_HAIR_ADMISSION_REPLAY_EXPORT") {
                        let mut capture=||->Result<(),String> {
                            let substeps=staged.substeps.max((dt*240.).ceil() as usize);
                            let h=dt/substeps as f64;
                            // Completed rods retain the LAST substep's old_x.
                            // Match its collider interval, not the full frame.
                            let sampled=if staged.continuous_mesh_admission {
                                let start=(substeps-1) as f64/substeps as f64;
                                Some(meshes.iter().map(|mesh|mesh.motion_interval(start,1.)).collect::<Result<Vec<_>,_>>().map_err(str::to_owned)?)
                            } else {None};
                            let capture_meshes=sampled.as_deref().unwrap_or(meshes);
                            let pairs=if staged.self_collision {contact::refresh_strand_responses(&mut staged.rods,staged.contact_radius,&[])} else {Vec::new()};
                            Self::refresh_mesh_geometry(&mut staged.rods,capture_meshes,staged.contact_radius,staged.continuous_mesh_admission).map_err(str::to_owned)?;
                            let merit=Self::contact_merit(&staged.rods,&pairs,staged.contact_radius).map_err(str::to_owned)?;
                            contact_replay::save(std::path::Path::new(&path),&staged.rods,capture_meshes,h,staged.contact_radius,staged.self_collision,merit,staged.continuous_mesh_admission).map_err(|error|error.to_string())
                        };
                        if let Err(error)=capture() {eprintln!("HAIR ADMISSION REPLAY EXPORT ERROR: {error}");}
                    }
                }
                admission
            });
        if let Err(error)=result {
            // Opt-in observation of rejected staged state; the public state
            // still remains untouched. Export errors never change admission.
            if let Some(path)=std::env::var_os("VOXY_HAIR_FAILURE_TRACE_EXPORT") {
                let trace=format!("error: {error}\nphases: {:#?}\nprojections: {:#?}\nfriction: {:#?}\n",
                    staged.last_trace,staged.last_projection_trace,staged.last_friction_trace);
                if let Err(write_error)=std::fs::write(&path,trace) {eprintln!("HAIR FAILURE TRACE EXPORT ERROR: {write_error}");}
            }
            return Err(error);
        }
        *self=staged;
        Ok(())
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
            direct::apply_correction(rod,&correction,dt,&system.rhs)?;
            rod.solve_matrix=system.matrix;rod.solve_rhs=system.rhs;
        }
        Ok(())
    }
    fn reconcile_positions(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,self_collision:bool,history:&mut Vec<contact::StrandResponse>,trace:Option<HairProjectionTrace<'_>>)->Result<(), &'static str> {
        Self::reconcile_positions_with_solver(rods,meshes,dt,radius,self_collision,history,None,trace)
    }
    fn reconcile_positions_with_solver(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,self_collision:bool,history:&mut Vec<contact::StrandResponse>,solver:Option<&mut dyn HairLinearSolver>,trace:Option<HairProjectionTrace<'_>>)->Result<(), &'static str> {
        Self::reconcile_positions_mode(rods,meshes,dt,radius,self_collision,history,solver,trace,false)
    }
    fn refresh_mesh_geometry(rods:&mut [HairRod],meshes:&[TriangleMesh],radius:f64,continuous:bool)->Result<(), &'static str> {
        for rod in rods.iter_mut() {contact::refresh_mesh_constraints(rod,meshes,radius);}
        if continuous {mesh_motion::refresh(rods,meshes,radius)?;}
        Ok(())
    }
    fn reconcile_positions_mode(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,self_collision:bool,history:&mut Vec<contact::StrandResponse>,mut solver:Option<&mut dyn HairLinearSolver>,mut trace:Option<HairProjectionTrace<'_>>,continuous_mesh:bool)->Result<(), &'static str> {
        let mut last_worst=None;
        let mut first_gap=0.;
        let mut merit_history=Vec::with_capacity(128);
        let mut model=contact_model::ContactModel::new(rods.len());
        let motion_start=(continuous_mesh && self_collision).then(|| {
            let mut start=rods.to_vec();
            for rod in &mut start {rod.x.clone_from(&rod.old_x);rod.q.clone_from(&rod.old_q);}
            start
        });
        for iteration in 0..128 {
            let mut current=if self_collision {contact::refresh_strand_responses(rods,radius,&[])} else {Vec::new()};
            Self::refresh_mesh_geometry(rods,meshes,radius,continuous_mesh)?;
            let merit=Self::contact_merit(rods,&current,radius)?;
            if std::env::var_os("VOXY_HAIR_CONTACT_PARTS_TRACE").is_some() {Self::trace_contact_parts(rods,&current,radius,iteration,0.)?;}
            let before:Vec<_>=rods.iter().map(|rod|(rod.x.clone(),rod.q.clone())).collect();
            let direction_export=std::env::var_os("VOXY_HAIR_REJECTED_CONTACT_DIRECTION_EXPORT");
            let before_contacts=Some(rods.iter().map(|rod|rod.contacts.clone()).collect::<Vec<_>>());
            let mut snapshot=trace.as_ref().map(|trace|HairContactProjectionDiagnostic::begin(trace,iteration,rods,&current));
            let before_pairs=current.clone();
            if continuous_mesh {model.install(rods);}
            let projection=if let Some(start)=&motion_start {
                let mut reactions=Vec::new();
                let result=if let Some(backend)=solver.as_mut() {contact::advance_swept_strands(rods,dt,radius,Some(&mut **backend),&mut reactions,Some(start))} else {contact::advance_swept_strands(rods,dt,radius,None,&mut reactions,Some(start))};
                current=reactions;
                result.map(|fraction|fraction==1.)
            } else if continuous_mesh {
                if let Some(backend)=solver.as_mut() {contact::reconcile_elastic_contact_positions_with_solver(rods,&mut current,dt,radius,Some(&mut **backend))} else {contact::reconcile_elastic_contact_positions_with_solver(rods,&mut current,dt,radius,None)}
            } else {
                if let Some(backend)=solver.as_mut() {contact::reconcile_contact_positions_with_solver(rods,&mut current,dt,radius,Some(&mut **backend))} else {contact::reconcile_contact_positions(rods,&mut current,dt,radius)}
            };
            let mut complete=projection.map_err(|error| {
                if let Some(path)=std::env::var_os("VOXY_HAIR_CONTACT_REPLAY_EXPORT") {
                    let path=std::path::Path::new(&path);
                    if let Err(export_error)=contact_replay::save(path,rods,meshes,dt,radius,self_collision,merit,continuous_mesh) {eprintln!("HAIR CONTACT REPLAY EXPORT ERROR {export_error}");}
                    let mut rows=Vec::new();
                    for (r,rod) in rods.iter().enumerate() {
                        for c in &rod.contacts {
                            if !matches!(c.source,ContactSource::Mesh(_)) {continue;}
                            rows.push(format!("{{\"rod\":{r},\"segment\":{},\"fraction\":{:?},\"metric_scale\":{:?},\"time\":{},\"normal\":{:?},\"target\":{:?},\"endpoints\":{:?}}}",c.segment,c.fraction,c.metric_scale,c.trajectory_time.map_or("null".to_owned(),|t|format!("{t:?}")),c.normal,c.target,&rod.x[c.segment..=c.segment+1]));
                        }
                    }
                    if let Err(export_error)=std::fs::write(path.with_extension("contacts.json"),format!("[{}]",rows.join(","))) {eprintln!("HAIR CONTACT ROW EXPORT ERROR {export_error}");}
                }
                error
            })?;
            let mut fresh=if self_collision {contact::refresh_strand_responses(rods,radius,&[])} else {Vec::new()};
            Self::refresh_mesh_geometry(rods,meshes,radius,continuous_mesh)?;
            let full_merit=Self::contact_merit(rods,&fresh,radius)?;
            if std::env::var_os("VOXY_HAIR_CONTACT_PARTS_TRACE").is_some() {Self::trace_contact_parts(rods,&fresh,radius,iteration,1.)?;}
            if continuous_mesh && full_merit>merit {
                let added=model.observe_rejected_candidate(rods);
                if added>0 {
                    // The rejected candidate supplies model rows only. Keep
                    // the pose and all reactions uncommitted, then solve the
                    // enlarged model under the same H at this frozen pose.
                    for (rod,(x,q)) in rods.iter_mut().zip(&before) {rod.x.clone_from(x);rod.q.clone_from(q);}
                    Self::refresh_mesh_geometry(rods,meshes,radius,continuous_mesh)?;
                    if std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some() {eprintln!("HAIR CONTACT MODEL iteration={iteration} added_rows={added} before={merit:e} rejected={full_merit:e}");}
                    continue;
                }
            }
            let mut accepted_scale=1.;
            if full_merit>merit {
                let full:Vec<_>=rods.iter().map(|rod|(rod.x.clone(),rod.q.clone())).collect();
                let mut slope=0.;
                for (r,contacts) in before_contacts.as_ref().unwrap().iter().enumerate() {
                    let mut worst=vec![(0f64,0f64);rods[r].lengths.len()];
                    for c in contacts.iter().filter(|c|matches!(c.source,ContactSource::Mesh(_))) {
                        let i=c.segment;let t=c.fraction;
                        let p=add(mul(before[r].0[i],1.-t),mul(before[r].0[i+1],t));
                        let d=add(mul(sub(full[r].0[i],before[r].0[i]),1.-t),mul(sub(full[r].0[i+1],before[r].0[i+1]),t));
                        let depth=(-c.physical_gap(p)-1e-10).max(0.);
                        let derivative=-dot(c.normal,d)*c.metric_scale;
                        if depth>worst[i].0 {worst[i]=(depth,derivative);} else if depth==worst[i].0 {worst[i].1=worst[i].1.max(derivative);}
                    }
                    slope+=worst.iter().map(|(depth,derivative)|2.*depth*derivative).sum::<f64>();
                }
                for pair in &before_pairs {
                    let (a,i,u)=pair.a;let (b,j,v)=pair.b;
                    let pa=add(mul(before[a].0[i],1.-u),mul(before[a].0[i+1],u));
                    let pb=add(mul(before[b].0[j],1.-v),mul(before[b].0[j+1],v));
                    let da=add(mul(sub(full[a].0[i],before[a].0[i]),1.-u),mul(sub(full[a].0[i+1],before[a].0[i+1]),u));
                    let db=add(mul(sub(full[b].0[j],before[b].0[j]),1.-v),mul(sub(full[b].0[j+1],before[b].0[j+1]),v));
                    let distance=len(sub(pa,pb));
                    let derivative=if distance>0. {dot(sub(pa,pb),sub(da,db))/distance} else {len(sub(da,db))};
                    slope-=2.*(2.*radius-distance-1e-10).max(0.)*derivative;
                }
                let backtrack_scale=|scale:f64,value:f64| {
                    let denominator=2.*(value-merit-scale*slope);
                    let candidate=if slope<0. && denominator>0. {-slope*scale*scale/denominator} else {0.5*scale};
                    if candidate.is_finite() {candidate.clamp(0.1*scale,0.5*scale)} else {0.5*scale}
                };
                let mut scale=backtrack_scale(1.,full_merit);
                let mut last_trial_scale=scale;
                let mut accepted=None;
                // Closest features can change at different scales. Search the
                // actual refreshed geometry rather than testing only one half step.
                for _backtrack in 1..=12 {
                    last_trial_scale=scale;
                    for ((rod,(x,q)),(full_x,full_q)) in rods.iter_mut().zip(&before).zip(&full) {
                        for i in 1..rod.x.len() {rod.x[i]=add(x[i],mul(sub(full_x[i],x[i]),scale));}
                        for i in 1..rod.q.len() {
                            rod.q[i]=qunit(qm(exp(mul(log(qm(full_q[i],conj(q[i]))),scale)),q[i]));
                        }
                    }
                    let trial_pairs=if self_collision {contact::refresh_strand_responses(rods,radius,&[])} else {Vec::new()};
                    Self::refresh_mesh_geometry(rods,meshes,radius,continuous_mesh)?;
                    let trial_merit=Self::contact_merit(rods,&trial_pairs,radius)?;
                    if std::env::var_os("VOXY_HAIR_CONTACT_PARTS_TRACE").is_some() {Self::trace_contact_parts(rods,&trial_pairs,radius,iteration,scale)?;}
                    if trial_merit<merit && trial_merit<full_merit {
                        accepted=Some((scale,trial_pairs));break;
                    }
                    scale=backtrack_scale(scale,trial_merit);
                }
                if let Some((scale,pairs))=accepted {
                    fresh=pairs;
                    accepted_scale=scale;
                    for response in &mut current {response.impulse*=scale;}
                    complete=false;
                } else {
                    // Do not publish a known worsening increment or its
                    // reactions. The enclosing step transaction preserves all
                    // public state; retain the pre-increment staged pose too.
                    if let (Some(path),Some(contacts))=(&direction_export,&before_contacts) {
                        let mut rows=Vec::new();
                        for (r,list) in contacts.iter().enumerate() {
                            for c in list.iter().filter(|c|matches!(c.source,ContactSource::Mesh(_))) {
                                let i=c.segment;let t=c.fraction;
                                let p=add(mul(before[r].0[i],1.-t),mul(before[r].0[i+1],t));
                                let end=add(mul(full[r].0[i],1.-t),mul(full[r].0[i+1],t));
                                let metric_scale=c.metric_scale;
                                rows.push(format!("{{\"rod\":{r},\"segment\":{i},\"fraction\":{t:?},\"metric_scale\":{metric_scale:?},\"normal\":{:?},\"target\":{:?},\"gap\":{:?},\"directional_gap\":{:?}}}",c.normal,c.target,c.physical_gap(p),c.metric_scale*dot(sub(end,p),c.normal)));
                            }
                        }
                        let bx:Vec<_>=before.iter().map(|pose|&pose.0).collect();
                        let fx:Vec<_>=full.iter().map(|pose|&pose.0).collect();
                        let mut trial_rows=Vec::new();
                        for (r,rod) in rods.iter().enumerate() {
                            for c in rod.contacts.iter().filter(|c|matches!(c.source,ContactSource::Mesh(_))) {
                                let t=c.fraction;let i=c.segment;
                                let p=add(mul(rod.x[i],1.-t),mul(rod.x[i+1],t));
                                trial_rows.push(format!("{{\"rod\":{r},\"segment\":{i},\"fraction\":{t:?},\"gap\":{:?},\"metric_scale\":{:?},\"normal\":{:?}}}",c.physical_gap(p),c.metric_scale,c.normal));
                            }
                        }
                        let payload=format!("{{\"iteration\":{iteration},\"before_merit\":{merit:?},\"full_merit\":{full_merit:?},\"before_positions\":{bx:?},\"full_positions\":{fx:?},\"mesh_contacts\":[{}],\"smallest_trial_scale\":{last_trial_scale:?},\"smallest_trial_mesh_contacts\":[{}]}}",rows.join(","),trial_rows.join(","));
                        if let Err(error)=std::fs::write(path,payload) {eprintln!("HAIR CONTACT DIRECTION EXPORT ERROR: {error}");}
                    }
                    for (rod,(x,q)) in rods.iter_mut().zip(before) {rod.x=x;rod.q=q;}
                    Self::refresh_mesh_geometry(rods,meshes,radius,continuous_mesh)?;
                    if let Some(path)=std::env::var_os("VOXY_HAIR_CONTACT_REPLAY_EXPORT") {
                        if let Err(error)=contact_replay::save(std::path::Path::new(&path),rods,meshes,dt,radius,self_collision,merit,continuous_mesh) {
                            eprintln!("HAIR CONTACT REPLAY EXPORT ERROR: {error}");
                        }
                    }
                    if std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some() {
                        eprintln!("HAIR NONLINEAR REJECT iteration={iteration} before={merit:e} full={full_merit:e} reason=no_descent");
                    }
                    return Err("joint hair contact direction has no descending geometry step");
                }
            }
            if std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some() {
                let accepted_merit=Self::contact_merit(rods,&fresh,radius)?;
                eprintln!("HAIR NONLINEAR MERIT iteration={iteration} before={merit:e} full={full_merit:e} accepted={accepted_merit:e} scale={accepted_scale:e} tangent_complete={complete}");
            }
            merit_history.push(Self::contact_merit(rods,&fresh,radius)?);
            // The common trust scale preserves paired reactions. Re-query all
            // geometry before the next nonlinear increment rather than fixing
            // one strand to the other's preceding position.
            if let Some(snapshot)=snapshot.as_mut() {snapshot.record_reactions(&current);}
            history.extend(current);
            // A solved tangent problem is not proof of separation in the new
            // nonlinear geometry: rotations can change a closest feature or
            // create another pair. Admit only a freshly queried feasible pose.
            let strand_admitted=if self_collision {
                contact::strand_geometry_admitted(rods,&fresh,radius)?
            } else {true};
            if let (Some(trace),Some(mut snapshot))=(trace.as_mut(),snapshot) {snapshot.complete(rods);trace.output.push(snapshot);}
            let mut unresolved=!strand_admitted;
            let mut worst=None;
            for (index,rod) in rods.iter().enumerate() {
                for contact in &rod.contacts {
                    let i=contact.segment;let t=contact.fraction;
                    let position=add(mul(rod.x[i],1.-t),mul(rod.x[i+1],t));
                    let gap=contact.physical_gap(position);
                    if !gap.is_finite() {return Err("joint hair contact geometry overflow");}
                    unresolved|=gap < -1e-10;
                    if gap < worst.as_ref().map_or(0.,|value:&(usize,usize,f64,f64,ContactSource,V,V,V,V,V)|value.3) {
                        worst=Some((index,i,t,gap,contact.source,rod.x[0],rod.x[i],rod.x[i+1],contact.normal,contact.target));
                    }
                }
            }
            for rod in rods.iter_mut() {rod.contacts.retain(|contact|matches!(contact.source,ContactSource::Mesh(_)));}
            if std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some() {
                eprintln!("HAIR NONLINEAR ADMISSION iteration={iteration} tangent_complete={complete} strand_admitted={strand_admitted} unresolved={unresolved} worst_gap_m={:?}",worst.as_ref().map(|value|value.3));
            }
            if complete && !unresolved {if continuous_mesh {mesh_motion::admit(rods,meshes,radius)?;} return Ok(());}
            if iteration==0 {first_gap=worst.as_ref().map_or(0.,|value|value.3);}
            last_worst=worst;
            // Spend more nonlinear work only while the actual refreshed
            // penetration merit contracts over an eight-increment window.
            // This changes a computational budget, never feasibility gates.
            if iteration>=31 {
                let end=merit_history.len()-1;
                if end>=8 && !(merit_history[end-8]>0. && merit_history[end]<=0.9*merit_history[end-8]) {break;}
            }
        }
        if let Some(path)=std::env::var_os("VOXY_HAIR_CONTACT_REPLAY_EXPORT") {
            let pairs=if self_collision {contact::refresh_strand_responses(rods,radius,&[])} else {Vec::new()};
            if let Ok(merit)=Self::contact_merit(rods,&pairs,radius) {
                if let Err(error)=contact_replay::save(std::path::Path::new(&path),rods,meshes,dt,radius,self_collision,merit,continuous_mesh) {eprintln!("HAIR CONTACT REPLAY EXPORT ERROR: {error}");}
            }
        }
        if std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some() {eprintln!("HAIR NONLINEAR MERIT HISTORY {merit_history:?}");}
        eprintln!("HAIR NONLINEAR CONTACT NONCONVERGENCE first_gap_m={first_gap} worst=(rod,segment,fraction,gap,source,root,a,b,normal,target) {last_worst:?}");
        Err("joint hair contact position linearization did not converge")
    }
    fn trace_contact_parts(rods:&[HairRod],pairs:&[contact::StrandResponse],radius:f64,iteration:usize,scale:f64)->Result<(), &'static str> {
        let mut mesh_merit=0.;let mut worst=None;
        for (r,rod) in rods.iter().enumerate() {
            for c in rod.contacts.iter().filter(|c|matches!(c.source,ContactSource::Mesh(_))) {
                let p=add(mul(rod.x[c.segment],1.-c.fraction),mul(rod.x[c.segment+1],c.fraction));
                let gap=c.physical_gap(p);
                mesh_merit+=gap.min(0.).powi(2);
                if gap<worst.as_ref().map_or(0.,|v:&(usize,usize,f64,f64,V,V)|v.3) {worst=Some((r,c.segment,c.fraction,gap,c.normal,c.target));}
            }
        }
        let mut strand_merit=0.;
        for pair in pairs {
            let (a,i,s)=pair.a;let (b,j,t)=pair.b;
            let pa=add(mul(rods[a].x[i],1.-s),mul(rods[a].x[i+1],s));
            let pb=add(mul(rods[b].x[j],1.-t),mul(rods[b].x[j+1],t));
            strand_merit+=(len(sub(pa,pb))-2.*radius).min(0.).powi(2);
        }
        if !(mesh_merit+strand_merit).is_finite() {return Err("contact merit diagnostic overflow");}
        eprintln!("HAIR CONTACT PARTS iteration={iteration} scale={scale:e} mesh_raw={mesh_merit:e} strand={strand_merit:e} worst_mesh={worst:?}");
        Ok(())
    }
    fn contact_merit(rods:&[HairRod],pairs:&[contact::StrandResponse],radius:f64)->Result<f64, &'static str> {
        let mut merit=0.;
        for rod in rods {
            let mut segment_depth=vec![0f64;rod.lengths.len()];
            for contact in &rod.contacts {
                if !matches!(contact.source,ContactSource::Mesh(_)) {continue;}
                let p=add(mul(rod.x[contact.segment],1.-contact.fraction),mul(rod.x[contact.segment+1],contact.fraction));
                let gap=contact.physical_gap(p);
                // Discovery adds a row only beyond the physical gap tolerance.
                // Measure excess violation so activation is continuous at that
                // same boundary instead of adding a tolerance-squared jump.
                segment_depth[contact.segment]=segment_depth[contact.segment].max(-gap-1e-10);
                if !gap.is_finite() {return Err("contact merit overflow");}
            }
            merit+=segment_depth.iter().map(|depth|depth*depth).sum::<f64>();
        }
        for pair in pairs {
            let (a,i,s)=pair.a;let (b,j,t)=pair.b;
            let pa=add(mul(rods[a].x[i],1.-s),mul(rods[a].x[i+1],s));
            let pb=add(mul(rods[b].x[j],1.-t),mul(rods[b].x[j+1],t));
            let gap=len(sub(pa,pb))-2.*radius;
            merit+=(-gap-1e-10).max(0.).powi(2);
            if !gap.is_finite() {return Err("contact merit overflow");}
        }
        if !merit.is_finite() {return Err("contact merit overflow");}
        Ok(merit)
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
        let swept_positions=self.swept_strand_positions || (self.continuous_mesh_admission && self.self_collision && self.joint_contact_positions);
        if swept_positions && (!self.joint_contact_positions || !self.self_collision || self.terminal_contact_iterations!=0) {
            return Err("swept strand motion requires joint self contacts and no terminal structural bypass");
        }
        if self.recover_friction_pressure && !self.joint_contact_positions {return Err("friction pressure recovery requires joint positions");}
        let timed_sampling=self.sample_collider_motion || self.continuous_mesh_admission;
        if timed_sampling && meshes.iter().filter_map(TriangleMesh::motion_duration).any(|duration|(duration-dt).abs()>1e-12*dt) {
            return Err("collider motion duration differs from hair step");
        }
        let mut sampled_meshes=(timed_sampling && (self.continuous_mesh_admission || meshes.iter().any(|mesh|mesh.motion_duration().is_some()))).then(||meshes.to_vec());
        self.last_trace.clear();
        self.last_friction_trace.clear();
        self.last_projection_trace.clear();
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
        if (self.swept_strand_positions || (self.continuous_mesh_admission && self.self_collision)) && !self.swept_strand_initialized {
            // Authored guide curves are not necessarily collision-free. Admit
            // an initial pose transactionally, before continuous time evolution.
            // A numerical clearance margin prevents touching import geometry
            // from starting below the continuous-query precision band.
            let radius=self.contact_radius+4e-10;
            let mut initial_meshes=meshes.to_vec();
            for (initial,source) in initial_meshes.iter_mut().zip(meshes) {
                if source.motion_duration().is_some() {initial.sample_motion(source,0.)?;}
            }
            let mut initial_reactions=Vec::new();
            if let Some(backend)=solver.as_mut().filter(|s|s.contact_responses_enabled()) {
                Self::reconcile_positions_with_solver(&mut self.rods,&initial_meshes,dt,radius,true,&mut initial_reactions,Some(&mut **backend),None)?;
            } else {Self::reconcile_positions(&mut self.rods,&initial_meshes,dt,radius,true,&mut initial_reactions,None)?;}
            let admitted:Vec<Vec<V>>=self.rods.iter().map(|rod|rod.x.clone()).collect();
            if contact::strand_fraction(&self.rods,&admitted,self.contact_radius)?!=1.
                || self.rods.iter().any(|rod|rod.max_relative_stretch()>0.05) {
                return Err("initial swept hair pose failed clearance or strain admission");
            }
            self.swept_strand_initialized=true;
            self.record_phase("initial-contact-admission",0,0);
        }
        for substep in 0..substeps {
            let meshes=if let Some(sampled)=&mut sampled_meshes {
                for (sample,source) in sampled.iter_mut().zip(meshes) {
                    *sample=source.motion_interval(substep as f64/substeps as f64,(substep+1) as f64/substeps as f64)?;
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
            let mut structural_start=None;
            if swept_positions {
                // Keep prediction as an inertial target, not a published free
                // pose: that unrestricted move caused the captured tunnelling.
                let old:Vec<Vec<V>>=self.rods.iter().map(|rod|rod.old_x.clone()).collect();
                for rod in &mut self.rods {
                    rod.x[1..].copy_from_slice(&rod.old_x[1..]);
                    rod.q[1..].copy_from_slice(&rod.old_q[1..]);
                }
                let mut previous=self.rods.clone();
                for (rod,points) in previous.iter_mut().zip(old) {rod.x=points;}
                // Root motion belongs to the first coupled structural path.
                // The staged new roots are the exact boundary conditions for
                // force assembly, not a separately published root-only pose.
                structural_start=Some(previous);
                for rod in &mut self.rods {contact::refresh_mesh_constraints(rod,meshes,self.contact_radius);}
            }
            self.record_phase("predicted",substep,0);
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
                if swept_positions {
                    let started=profiling.then(std::time::Instant::now);
                    // Newton iterations do not advance physical time. In the
                    // continuous pipeline every candidate shares this substep's
                    // original clock, including later structural corrections.
                    let motion_start=if self.continuous_mesh_admission || first==0 {structural_start.as_deref()} else {None};
                    if let Some(backend)=solver.as_deref_mut() {
                        contact::advance_swept_strands(&mut self.rods,dt,radius,Some(backend),&mut strand_responses,motion_start)?;
                    } else {contact::advance_swept_strands(&mut self.rods,dt,radius,None,&mut strand_responses,motion_start)?;}
                    self.record_phase("structural",substep,first);
                    if let Some(started)=started {self.last_profile.structural_ms+=started.elapsed().as_secs_f64()*1000.;}
                    for rod in &mut self.rods {contact::refresh_mesh_constraints(rod,meshes,radius);}
                } else if let Some(solver)=solver.as_deref_mut() {
                    for iteration in first..end {
                        let started=profiling.then(std::time::Instant::now);
                        Self::solve_external_rods(&mut self.rods,dt,solver)?;
                        self.record_phase("structural",substep,iteration);
                        if let Some(started)=started {self.last_profile.structural_ms+=started.elapsed().as_secs_f64()*1000.;}
                        let started=profiling.then(std::time::Instant::now);
                        for rod in &mut self.rods {if joint_positions {contact::refresh_mesh_constraints(rod,meshes,radius);} else {contact::mesh_contacts(rod,meshes,radius);}}
                        if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                    }
                } else if !parallel {
                    for (index,rod) in self.rods.iter_mut().enumerate() {
                        let started = profiling.then(std::time::Instant::now);
                        rod.solve(dt, first % 2 != 0)?;
                        if self.trace_rod==Some(index) {self.last_trace.push(HairPhaseDiagnostic::capture("structural",substep,first,index,rod));}
                        if let Some(started) = started { self.last_profile.structural_ms += started.elapsed().as_secs_f64() * 1000.; }
                        let started = profiling.then(std::time::Instant::now);
                        if joint_positions {contact::refresh_mesh_constraints(rod,meshes,radius);} else {contact::mesh_contacts(rod, meshes, radius);}
                        if let Some(started) = started { self.last_profile.mesh_contacts_ms += started.elapsed().as_secs_f64() * 1000.; }
                    }
                } else {
                    let chunk = self.rods.len().div_ceil(self.workers);
                    let trace_rod=self.trace_rod;
                    let profiles = std::thread::scope(|scope| {
                        let mut workers = Vec::new();
                        for (chunk_index,rods) in self.rods.chunks_mut(chunk).enumerate() {
                            workers.push(scope.spawn(move || {
                                let mut profile = HairStepProfile::default();
                                let mut trace=Vec::new();
                                for (local_index,rod) in rods.iter_mut().enumerate() {
                                    for iteration in first..end {
                                        let started = profiling.then(std::time::Instant::now);
                                        rod.solve(dt, iteration % 2 != 0)?;
                                        let index=chunk_index*chunk+local_index;
                                        if trace_rod==Some(index) {trace.push(HairPhaseDiagnostic::capture("structural",substep,iteration,index,rod));}
                                        if let Some(started) = started { profile.structural_ms += started.elapsed().as_secs_f64() * 1000.; }
                                        let started = profiling.then(std::time::Instant::now);
                                        if joint_positions {contact::refresh_mesh_constraints(rod,meshes,radius);} else {contact::mesh_contacts(rod, meshes, radius);}
                                        if let Some(started) = started { profile.mesh_contacts_ms += started.elapsed().as_secs_f64() * 1000.; }
                                    }
                                }
                                Ok::<_, &'static str>((profile,trace))
                            }));
                        }
                        workers.into_iter().map(|worker| worker.join().map_err(|_| "hair profile worker panicked").and_then(|result| result)).collect::<Result<Vec<_>, _>>()
                    })?;
                    for (profile,trace) in profiles {
                        self.last_trace.extend(trace);
                        self.last_profile.structural_ms += profile.structural_ms;
                        self.last_profile.mesh_contacts_ms += profile.mesh_contacts_ms;
                    }
                }
                self.record_phase("structural-mesh",substep,end);
                if joint_positions && !self.self_collision {
                    let started=profiling.then(std::time::Instant::now);
                    let trace=if let Some(rod)=self.trace_rod {Some(HairProjectionTrace {rod,substep,structural_iteration:end,output:&mut self.last_projection_trace})} else {None};
                    if let Some(backend)=solver.as_mut().filter(|s|s.contact_responses_enabled()) {Self::reconcile_positions_mode(&mut self.rods,meshes,dt,radius,false,&mut strand_responses,Some(&mut **backend),trace,self.continuous_mesh_admission)?;} else {Self::reconcile_positions_mode(&mut self.rods,meshes,dt,radius,false,&mut strand_responses,None,trace,self.continuous_mesh_admission)?;}
                    if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                }
                if self.self_collision && (joint_positions || end % 4 == 0 || end == self.iterations) {
                    let started = profiling.then(std::time::Instant::now);
                    if joint_positions {
                        let trace=if let Some(rod)=self.trace_rod {Some(HairProjectionTrace {rod,substep,structural_iteration:end,output:&mut self.last_projection_trace})} else {None};
                        if let Some(backend)=solver.as_mut().filter(|s|s.contact_responses_enabled()) {Self::reconcile_positions_mode(&mut self.rods,meshes,dt,radius,true,&mut strand_responses,Some(&mut **backend),trace,self.continuous_mesh_admission)?;} else {Self::reconcile_positions_mode(&mut self.rods,meshes,dt,radius,true,&mut strand_responses,None,trace,self.continuous_mesh_admission)?;}
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
                self.record_phase("contact-positions",substep,end);
            }
            // Additional coupled iterations: an elastic correction may reopen
            // penetration, so it must recheck geometry before publication.
            for _ in 0..self.terminal_contact_iterations {
                let started=profiling.then(std::time::Instant::now);
                if let Some(solver)=solver.as_deref_mut() {
                    Self::solve_external_rods(&mut self.rods,dt,solver)?;
                } else {for rod in &mut self.rods {direct::solve(rod,dt)?;}}
                if let Some(started)=started {self.last_profile.structural_ms+=started.elapsed().as_secs_f64()*1000.;}
                let started=profiling.then(std::time::Instant::now);
                for rod in &mut self.rods {contact::refresh_mesh_constraints(rod,meshes,self.contact_radius);}
                if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                if self.joint_contact_positions {
                    let started=profiling.then(std::time::Instant::now);
                    if let Some(backend)=solver.as_mut().filter(|s|s.contact_responses_enabled()) {
                        Self::reconcile_positions_mode(&mut self.rods,meshes,dt,self.contact_radius,self.self_collision,&mut strand_responses,Some(&mut **backend),None,self.continuous_mesh_admission)?;
                    } else {
                        Self::reconcile_positions_mode(&mut self.rods,meshes,dt,self.contact_radius,self.self_collision,&mut strand_responses,None,None,self.continuous_mesh_admission)?;
                    }
                    if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                }
            }
            if self.self_collision {
                let started=profiling.then(std::time::Instant::now);
                strand_responses=contact::refresh_strand_responses(&mut self.rods,self.contact_radius,&strand_responses);
                if let Some(started)=started {self.last_profile.self_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
            }
            if self.recover_friction_pressure {
                let started=profiling.then(std::time::Instant::now);
                if let Some(backend)=solver.as_mut().filter(|s|s.contact_responses_enabled()) {
                    contact::recover_friction_pressure(&self.rods,&mut strand_responses,dt,self.contact_radius,Some(&mut **backend))?;
                } else {contact::recover_friction_pressure(&self.rods,&mut strand_responses,dt,self.contact_radius,None)?;}
                if let Some(started)=started {self.last_profile.self_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
            }
            if self.continuous_mesh_admission {
                let started=profiling.then(std::time::Instant::now);
                let admission=mesh_motion::admit(&self.rods,meshes,self.contact_radius);
                if let Some(started)=started {self.last_profile.mesh_contacts_ms+=started.elapsed().as_secs_f64()*1000.;}
                admission?;
                if self.self_collision {contact::admit_staged_strands(&self.rods,self.contact_radius)?;}
                self.record_phase("continuous-mesh-admission",substep,self.iterations);
            }
            self.record_phase("before-finish",substep,self.iterations);
            for rod in &mut self.rods {
                rod.finish(dt);
            }
            self.record_phase("rod-velocities",substep,self.iterations);
            if self.trace_rod.is_some() {contact::finish_strand_contacts_with_diagnostics(&mut self.rods,&strand_responses,dt,substep,Some(&mut self.last_friction_trace));} else {contact::finish_strand_contacts(&mut self.rods,&strand_responses,dt);}
            self.record_phase("strand-friction",substep,self.iterations);
            if self.joint_contact_velocities {
                if let Some(backend)=solver.as_mut().filter(|s|s.contact_responses_enabled()) {contact::stabilize_contact_velocities_with_solver(&mut self.rods,&strand_responses,dt,self.contact_radius,Some(&mut **backend))?;} else {contact::stabilize_contact_velocities(&mut self.rods,&strand_responses,dt,self.contact_radius)?;}
            }
            self.record_phase("finished",substep,self.iterations);
        }
        if self.rods.iter().any(|r| {
            r.x.iter().chain(&r.velocity).chain(&r.omega).any(|p| !finite(*p)) || r.q.iter().flatten().any(|q| !q.is_finite())
        }) {
            return Err("hair solver overflow");
        }
        Ok(())
    }
}

#[cfg(test)]
mod contact_merit_tests {
    use super::*;
    #[test]
    fn continuous_joint_step_activates_an_unseen_pair_and_admits_its_entire_path() {
        let radius=40e-6;
        let mut rods:Vec<_>=[-0.5e-3,0.5e-3].into_iter().map(|x|HairRod::new(
            vec![[x,0.,0.],[x,0.,0.1],[x,0.,0.2]],HairMaterial::default()).unwrap()).collect();
        let start=rods.clone();
        for (r,rod) in rods.iter_mut().enumerate() {
            for point in &mut rod.predicted_x[1..] {point[0]+=if r==0 {0.55e-3} else {-0.55e-3};}
        }
        assert!(contact::refresh_strand_responses(&mut rods,radius,&[]).is_empty());
        let mut history=Vec::new();
        HairSystem::reconcile_positions_mode(&mut rods,&[],1./240.,radius,true,&mut history,None,None,true).unwrap();
        let pairs=contact::refresh_strand_responses(&mut rods,radius,&[]);
        assert!(contact::strand_geometry_admitted(&rods,&pairs,radius).unwrap());
        let end:Vec<_>=rods.iter().map(|rod|rod.x.clone()).collect();
        assert_eq!(contact::strand_fraction(&start,&end,radius).unwrap(),1.);
        for (rod,old) in rods.iter().zip(&start) {
            assert_eq!(rod.x[0],old.x[0]);
            assert!((rod.x[2][0]-old.x[2][0]).abs()>0.3e-3,"a frozen guide cannot qualify this motion");
            assert!(rod.max_relative_stretch()<0.05);
        }
    }
    #[test]
    fn newly_discovered_contact_merit_is_continuous_at_admission_tolerance() {
        let mut rod=HairRod::new(vec![[0.,0.,0.],[0.,0.,1.],[0.,0.,2.]],HairMaterial::default()).unwrap();
        let empty=HairSystem::contact_merit(&[rod.clone()],&[],40e-6).unwrap();
        let row=rod.record_contact(1,0.,[1.,0.,0.],[1e-10,0.,1.],ContactSource::Mesh(0));
        assert_eq!(HairSystem::contact_merit(&[rod.clone()],&[],40e-6).unwrap(),empty);
        rod.contacts[row].target[0]+=1e-13;
        let violation=HairSystem::contact_merit(&[rod],&[],40e-6).unwrap();
        assert!(violation>0. && violation<2e-26,"activation must not add a tolerance-squared discontinuity");
    }
    #[test]
    fn mesh_merit_is_invariant_to_redundant_segment_witnesses() {
        let mut rod=HairRod::new(vec![[0.,0.,0.],[0.,0.,1.],[0.,0.,2.]],HairMaterial::default()).unwrap();
        rod.record_contact(1,0.,[1.,0.,0.],[0.1,0.,1.],ContactSource::Mesh(0));
        let initial=HairSystem::contact_merit(&[rod.clone()],&[],40e-6).unwrap();
        rod.record_contact(1,0.5,[1.,0.,0.],[0.1,0.,1.5],ContactSource::Mesh(0));
        assert_eq!(initial,HairSystem::contact_merit(&[rod.clone()],&[],40e-6).unwrap());
        rod.record_contact(1,0.25,[1.,0.,0.],[0.2,0.,1.25],ContactSource::Mesh(0));
        assert!((HairSystem::contact_merit(&[rod],&[],40e-6).unwrap()-(0.2f64-1e-10).powi(2)).abs()<1e-15);
    }
}

#[path="mesh_motion.rs"]
mod mesh_motion;
