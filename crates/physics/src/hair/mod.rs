//! SI-unit Cosserat guide rods with implicit stretch/shear and material-frame bend/twist.
//! No attraction of free particles to world-space groom positions.
mod contact;
mod contact_model;
mod contact_replay;
mod direct;
mod math;
mod ordered_queries;
pub use contact::{
    CapsuleMotion, CapsuleSweep, CapsuleSweepOptions, TriangleMesh, TriangleMotion,
    sweep_capsule_triangle, sweep_capsules, swept_capsule_contacts, swept_capsule_pairs,
};
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
    Strand {
        other_rod: usize,
        other_segment: usize,
    },
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
    metric_scale: f64,
    trajectory_time: Option<f64>,
}
impl RodContact {
    fn physical_gap(&self, p: V) -> f64 {
        self.metric_scale * dot(sub(p, self.target), self.normal)
    }
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
#[path = "contact_diagnostics.rs"]
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
    fn record_contact(
        &mut self,
        mut segment: usize,
        mut fraction: f64,
        normal: V,
        target: V,
        source: ContactSource,
    ) -> usize {
        // Canonicalize a shared endpoint so node and capsule queries do not
        // count the same geometric contact twice. Only equal endpoint weights,
        // unit-metric normals and plane offsets identify the same physical row;
        // nearby planes can exclude different motions.
        if fraction == 1. && segment + 1 < self.lengths.len() {
            segment += 1;
            fraction = 0.;
        }
        if let Some(index) = self.contacts.iter().position(|contact| {
            contact.trajectory_time.is_none()
                && contact.source == source
                && contact.segment == segment
                && contact.metric_scale == 1.
                && contact.fraction == fraction
                && contact.normal == normal
                && dot(contact.target, normal) == dot(target, normal)
        }) {
            let existing = &mut self.contacts[index];
            existing.fraction = fraction;
            existing.normal = normal;
            existing.target = target;
            existing.metric_scale = 1.;
            index
        } else {
            self.contacts.push(RodContact {
                segment,
                fraction,
                normal,
                target,
                source,
                surface_velocity: [0.; 3],
                metric_scale: 1.,
                trajectory_time: None,
            });
            self.contacts.len() - 1
        }
    }
    fn record_point_contact(
        &mut self,
        point: usize,
        normal: V,
        target: V,
        source: ContactSource,
    ) -> usize {
        let segment = point.min(self.lengths.len() - 1);
        let fraction = if point == self.lengths.len() { 1. } else { 0. };
        self.record_contact(segment, fraction, normal, target, source)
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
        self.surface_velocity_sum.fill([0.; 3]);
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
                let surface = mul(
                    self.surface_velocity_sum[i],
                    1. / self.surface_velocity_weight[i].max(1e-30),
                );
                let relative = sub(self.velocity[i], surface);
                let vn = dot(relative, n);
                let tangent = sub(relative, mul(n, vn));
                let removed = self.material.friction * vn.abs();
                let factor = (1. - removed / len(tangent).max(1e-30)).max(0.);
                self.velocity[i] = add(surface, add(mul(tangent, factor), mul(n, vn.max(0.))));
            }
        }
        for i in 0..self.q.len() {
            self.omega[i] = mul(log(qm(self.q[i], conj(self.old_q[i]))), 1. / dt);
        }
    }
    fn record_surface_response(&mut self, point: usize, correction: V, velocity: V) {
        self.normals[point] = add(self.normals[point], correction);
        let weight = len(correction);
        self.surface_velocity_sum[point] =
            add(self.surface_velocity_sum[point], mul(velocity, weight));
        self.surface_velocity_weight[point] += weight;
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
pub use contact_response_system::{
    HairContactCoordinateRequest, HairContactEqualityProposal, HairContactEqualityRequest,
    HairResponseSystem,
};
/// Accelerator ownership remains outside physics; one callback receives all
/// independent rod matrices from the current nonlinear iteration.
pub trait HairLinearSolver {
    /// Opt in to square-root joint contacts; defaults preserve legacy backends.
    fn joint_contact_coordinates_enabled(&self) -> bool {
        false
    }
    fn solve_joint_coordinates(
        &mut self,
        _columns: &[Vec<f64>],
        _bounds: &[f64],
        _tolerance: f64,
    ) -> Option<(Vec<f64>, Vec<f64>)> {
        None
    }
    /// Explicit precision-preserving coordinate route; never approximated via legacy callbacks.
    fn joint_contact_expanded_enabled(&self) -> bool {
        false
    }
    /// Permit a precision-preserving retry after an ordinary GPU rejection.
    fn joint_contact_expanded_fallback_enabled(&self) -> bool {
        false
    }

    fn solve_joint_expanded_batch(
        &mut self,
        _requests: &[HairExpandedCoordinateRequest<'_>],
    ) -> Option<Vec<(Vec<HairResponseCoefficient>, Vec<f64>)>> {
        None
    }
    /// Independent candidates retain input order. A rejected operator must not
    /// discard a valid sibling; every candidate still needs physical admission.
    fn solve_joint_expanded_candidates(
        &mut self,
        requests: &[HairExpandedCoordinateRequest<'_>],
    ) -> Option<Vec<Option<(Vec<HairResponseCoefficient>, Vec<f64>)>>> {
        self.solve_joint_expanded_batch(requests)
            .map(|values| values.into_iter().map(Some).collect())
    }

    fn joint_contact_hints_enabled(&self) -> bool {
        false
    }
    /// Optional dual hints owned by one immutable physical projection. Backends
    /// may ignore them; hints never bypass original physical admission.
    fn solve_joint_coordinates_seeded(
        &mut self,
        columns: &[Vec<f64>],
        bounds: &[f64],
        tolerance: f64,
        _seeds: &[f64],
    ) -> Option<(Vec<f64>, Vec<f64>)> {
        self.solve_joint_coordinates(columns, bounds, tolerance)
    }
    fn joint_contact_coordinate_batches_enabled(&self) -> bool {
        false
    }
    /// Independent immutable coordinate operators. Legacy backends retain
    /// sequential behavior; each returned candidate still needs physical admission.
    fn solve_joint_coordinates_batch(
        &mut self,
        requests: &[HairContactCoordinateRequest<'_>],
    ) -> Option<Vec<(Vec<f64>, Vec<f64>)>> {
        requests
            .iter()
            .map(|r| self.solve_joint_coordinates_seeded(r.columns, r.bounds, r.tolerance, r.seeds))
            .collect()
    }
    /// Preserve ordinary coordinate candidates independently when supported.
    fn solve_joint_coordinate_candidates(
        &mut self,
        requests: &[HairContactCoordinateRequest<'_>],
    ) -> Option<Vec<Option<(Vec<f64>, Vec<f64>)>>> {
        self.solve_joint_coordinates_batch(requests)
            .map(|values| values.into_iter().map(Some).collect())
    }
    /// Physical owner reports whether the candidate survived original admission.
    fn joint_contact_result(&mut self, _accelerated: bool) {}
    /// Explicit qualification opt-in for contact response batches.
    fn contact_responses_enabled(&self) -> bool {
        false
    }
    fn solve(&mut self, systems: &[HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str>;
    /// Shared-matrix contact loads. Existing backends retain the native path.
    fn solve_responses(
        &mut self,
        systems: &[HairResponseSystem],
    ) -> Result<Vec<Vec<Vec<f64>>>, &'static str> {
        systems
            .iter()
            .map(HairResponseSystem::solve_native)
            .collect()
    }
}
impl HairLinearSystem {
    fn validate_shape(&self) -> Result<(), &'static str> {
        if self.band_width != direct::BAND
            || self.rhs.len() < 12
            || !self.rhs.len().is_multiple_of(6)
            || self.matrix.len() != self.rhs.len() * direct::BAND
            || self.active != (6..self.rhs.len() - 3)
            || self.matrix.iter().chain(&self.rhs).any(|v| !v.is_finite())
        {
            return Err("invalid hair linear system");
        }
        Ok(())
    }
    pub fn solve_native(&self) -> Result<Vec<f64>, &'static str> {
        self.validate_shape()?;
        let mut matrix = self.matrix.clone();
        let mut rhs = self.rhs.clone();
        direct::cholesky(&mut matrix, &mut rhs, self.active.clone());
        self.validate_correction(&rhs)?;
        Ok(rhs)
    }
    /// Admit an accelerator result against the original physical system.
    pub fn validate_correction(&self, values: &[f64]) -> Result<(), &'static str> {
        self.validate_shape()?;
        self.validate_for_rhs(values, &self.rhs)
    }
    /// Admit a response to another load on this same physical matrix.
    pub fn validate_load_correction(
        &self,
        values: &[f64],
        rhs: &[f64],
    ) -> Result<(), &'static str> {
        self.validate_shape()?;
        self.validate_for_rhs(values, rhs)
    }
    fn validate_for_rhs(&self, values: &[f64], rhs: &[f64]) -> Result<(), &'static str> {
        if rhs.len() != self.rhs.len() || rhs.iter().any(|v| !v.is_finite()) {
            return Err("invalid hair response load");
        }
        if values.len() != self.rhs.len() || values.iter().any(|v| !v.is_finite()) {
            return Err("invalid hair accelerator correction");
        }
        if (0..self.active.start)
            .chain(self.active.end..values.len())
            .any(|i| values[i] != rhs[i])
        {
            return Err("hair accelerator changed fixed DOFs");
        }
        for i in self.active.clone() {
            let (ax, scale) = self.row_product(values, i, rhs);
            if !ax.is_finite()
                || !scale.is_finite()
                || (ax - rhs[i]).abs() > 1e-8 * scale.max(1e-30)
            {
                return Err("hair accelerator residual exceeds tolerance");
            }
        }
        Ok(())
    }
    fn row_product(&self, values: &[f64], i: usize, rhs: &[f64]) -> (f64, f64) {
        let term = self.matrix[i * direct::BAND] * values[i];
        let mut ax = term;
        let mut scale = term.abs() + rhs[i].abs();
        for j in i.saturating_sub(direct::BAND - 1).max(self.active.start)..i {
            let term = self.matrix[i * direct::BAND + i - j] * values[j];
            ax += term;
            scale += term.abs();
        }
        for j in i + 1..(i + direct::BAND).min(self.active.end) {
            let term = self.matrix[j * direct::BAND + j - i] * values[j];
            ax += term;
            scale += term.abs();
        }
        (ax, scale)
    }
    /// Original f64 residual for mixed-precision refinement. Fixed degrees of
    /// freedom receive zero residual and therefore no refinement increment.
    pub fn correction_residual(&self, values: &[f64]) -> Result<Vec<f64>, &'static str> {
        self.load_residual(values, &self.rhs)
    }
    pub fn load_residual(&self, values: &[f64], rhs: &[f64]) -> Result<Vec<f64>, &'static str> {
        self.validate_shape()?;
        if rhs.len() != self.rhs.len() || rhs.iter().any(|v| !v.is_finite()) {
            return Err("invalid hair response load");
        }
        if values.len() != self.rhs.len() || values.iter().any(|v| !v.is_finite()) {
            return Err("invalid hair residual correction");
        }
        let mut residual = vec![0.; values.len()];
        for i in self.active.clone() {
            residual[i] = rhs[i] - self.row_product(values, i, rhs).0;
            if !residual[i].is_finite() {
                return Err("hair correction residual overflow");
            }
        }
        Ok(residual)
    }
}
impl HairRod {
    /// Capture the same assembly used by the native solver without changing the rod.
    pub fn linear_system(&self, dt: f64) -> Result<HairLinearSystem, &'static str> {
        if !dt.is_finite() || !(1e-6..=1. / 20.).contains(&dt) {
            return Err("invalid hair linear-system timestep");
        }
        let mut staged = self.clone();
        let (matrix, rhs) = direct::assemble(&mut staged, dt)?;
        if matrix.iter().chain(&rhs).any(|v| !v.is_finite()) {
            return Err("hair linear-system overflow");
        }
        let count = rhs.len();
        Ok(HairLinearSystem {
            band_width: direct::BAND,
            matrix,
            rhs,
            active: 6..count - 3,
        })
    }
}
/// Coupled guide solve. Segment contacts run inside the structural iterations.
#[path = "phase_diagnostics.rs"]
mod phase_diagnostics;
pub use phase_diagnostics::{HairFrictionDiagnostic, HairPhaseDiagnostic, observe_contact_frame};
mod projection_diagnostics;
use projection_diagnostics::HairProjectionTrace;
pub use projection_diagnostics::{HairContactProjectionDiagnostic, HairProjectionPairDiagnostic};
#[derive(Clone, Debug)]
pub struct HairSystem {
    rods: Vec<HairRod>,
    trace_rod: Option<usize>,
    last_trace: Vec<HairPhaseDiagnostic>,
    last_friction_trace: Vec<HairFrictionDiagnostic>,
    last_projection_trace: Vec<HairContactProjectionDiagnostic>,
    pub iterations: usize,
    /// Independent guide solves run before deterministic shared contacts.
    pub workers: usize,
    pub substeps: usize,
    last_step_substeps: usize,
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
            trace_rod: None,
            last_trace: Vec::new(),
            last_friction_trace: Vec::new(),
            last_projection_trace: Vec::new(),
            iterations: 16,
            workers: 1,
            substeps: 4,
            last_step_substeps: 0,
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
        system.trace_rod = self.trace_rod;
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
    /// Effective substeps of the last admitted step, including contact retries.
    pub fn last_step_substeps(&self) -> usize {
        self.last_step_substeps
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
        self.step_validated(dt, roots, gravity, air_velocity, meshes, None, |_| Ok(()))
    }
    /// The complete step commits only after the accelerator and contacts succeed.
    pub fn step_with_solver(
        &mut self,
        dt: f64,
        roots: &[RootPose],
        gravity: V,
        air_velocity: V,
        meshes: &[TriangleMesh],
        solver: &mut dyn HairLinearSolver,
    ) -> Result<(), &'static str> {
        self.step_validated(
            dt,
            roots,
            gravity,
            air_velocity,
            meshes,
            Some(solver),
            |_| Ok(()),
        )
    }
    /// Apply application-specific admission before publishing a solved state.
    /// Rejection preserves poses, velocities, contact history and diagnostics.
    /// Accelerator side effects are external and cannot be rolled back.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn step_validated(
        &mut self,
        dt: f64,
        roots: &[RootPose],
        gravity: V,
        air_velocity: V,
        meshes: &[TriangleMesh],
        solver: Option<&mut dyn HairLinearSolver>,
        validate: impl FnOnce(&HairSystem) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        *self =
            self.prepare_validated(dt, roots, gravity, air_velocity, meshes, solver, validate)?;
        Ok(())
    }
    /// Prepare an admitted state without publishing it, for coupled transactions.
    /// Accelerator observations remain external to the transaction.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn prepare_validated(
        &self,
        dt: f64,
        roots: &[RootPose],
        gravity: V,
        air_velocity: V,
        meshes: &[TriangleMesh],
        solver: Option<&mut dyn HairLinearSolver>,
        validate: impl FnOnce(&HairSystem) -> Result<(), &'static str>,
    ) -> Result<Self, &'static str> {
        let mut staged = self.clone();
        let result = staged
            .step_impl(dt, roots, gravity, air_velocity, meshes, solver)
            .and_then(|()| {
                let admission = validate(&staged);
                if admission.is_err() {
                    // Only completed steps use the endpoint collider geometry.
                    // Capture before rollback, including prediction/inertia data
                    // needed to reproduce elastic corrections after contact.
                    if let Some(path) = std::env::var_os("VOXY_HAIR_ADMISSION_REPLAY_EXPORT") {
                        let mut capture = || -> Result<(), String> {
                            let substeps = staged.substeps.max((dt * 240.).ceil() as usize);
                            let h = dt / substeps as f64;
                            // Completed rods retain the LAST substep's old_x.
                            // Match its collider interval, not the full frame.
                            let sampled = if staged.sample_collider_motion
                                || staged.continuous_mesh_admission
                            {
                                let start = (substeps - 1) as f64 / substeps as f64;
                                Some(
                                    meshes
                                        .iter()
                                        .map(|mesh| mesh.motion_interval(start, 1.))
                                        .collect::<Result<Vec<_>, _>>()
                                        .map_err(str::to_owned)?,
                                )
                            } else {
                                None
                            };
                            let capture_meshes = sampled.as_deref().unwrap_or(meshes);
                            // Captures rebuild the spatial hierarchy on load.
                            // Cached nearest triangle indices and clearance bounds
                            // belong to the live sampled collider; do not use them
                            // to certify the serialized geometry's contact merit.
                            for rod in &mut staged.rods {
                                rod.nearest_cache.clear();
                                rod.clearance_cache.clear();
                            }
                            let pairs = if staged.self_collision {
                                contact::refresh_strand_responses(
                                    &mut staged.rods,
                                    staged.contact_radius,
                                    &[],
                                )
                            } else {
                                Vec::new()
                            };
                            Self::refresh_mesh_geometry(
                                &mut staged.rods,
                                capture_meshes,
                                staged.contact_radius,
                                staged.continuous_mesh_admission,
                            )
                            .map_err(str::to_owned)?;
                            let merit =
                                Self::contact_merit(&staged.rods, &pairs, staged.contact_radius)
                                    .map_err(str::to_owned)?;
                            contact_replay::save(
                                std::path::Path::new(&path),
                                &staged.rods,
                                capture_meshes,
                                h,
                                staged.contact_radius,
                                staged.self_collision,
                                merit,
                                staged.continuous_mesh_admission,
                            )
                            .map_err(|error| error.to_string())
                        };
                        if let Err(error) = capture() {
                            eprintln!("HAIR ADMISSION REPLAY EXPORT ERROR: {error}");
                        }
                    }
                }
                admission
            });
        if let Err(error) = result {
            // Opt-in observation of rejected staged state; the public state
            // still remains untouched. Export errors never change admission.
            if let Some(path) = std::env::var_os("VOXY_HAIR_FAILURE_TRACE_EXPORT") {
                let trace = format!(
                    "error: {error}\nphases: {:#?}\nprojections: {:#?}\nfriction: {:#?}\n",
                    staged.last_trace, staged.last_projection_trace, staged.last_friction_trace
                );
                if let Err(write_error) = std::fs::write(&path, trace) {
                    eprintln!("HAIR FAILURE TRACE EXPORT ERROR: {write_error}");
                }
            }
            return Err(error);
        }
        Ok(staged)
    }
}

mod reconcile;
mod step;

#[cfg(test)]
mod contact_merit_tests {
    #[test]
    fn feasible_prefix_requires_descent_fixed_roots_and_physical_stretch() {
        // Resolve an inertia-dominated microstep so the small lateral move
        // is an actual elastic-objective descent, not just a geometric move.
        let dt = 1e-6;
        let radius = 40e-6;
        let mut rod = super::HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            Default::default(),
        )
        .unwrap();
        for i in 1..rod.x.len() {
            rod.predicted_x[i][0] = 1e-4;
        }
        let before = vec![(rod.x.clone(), rod.q.clone())];
        assert!(
            !super::HairSystem::feasible_elastic_prefix_admitted(
                &[rod.clone()],
                &[],
                &before,
                dt,
                radius,
                true
            )
            .unwrap()
        );
        for i in 1..rod.x.len() {
            rod.x[i][0] = 5e-5;
        }
        assert!(
            super::HairSystem::feasible_elastic_prefix_admitted(
                &[rod.clone()],
                &[],
                &before,
                dt,
                radius,
                true
            )
            .unwrap()
        );
        let mut moved_root = rod.clone();
        moved_root.x[0][0] = 1e-5;
        assert!(
            !super::HairSystem::feasible_elastic_prefix_admitted(
                &[moved_root],
                &[],
                &before,
                dt,
                radius,
                true
            )
            .unwrap()
        );
        rod.x[2][0] = 0.1;
        assert!(
            !super::HairSystem::feasible_elastic_prefix_admitted(
                &[rod],
                &[],
                &before,
                dt,
                radius,
                true
            )
            .unwrap()
        );
    }
    #[test]
    fn feasible_prefix_rejects_a_clear_endpoint_reached_through_a_surface() {
        let dt = 1e-6;
        let radius = 40e-6;
        let mut rod = super::HairRod::new(
            vec![[-1e-3, 0., 0.], [-1e-3, 0., 0.01], [-1e-3, 0., 0.02]],
            Default::default(),
        )
        .unwrap();
        let before = vec![(rod.x.clone(), rod.q.clone())];
        for i in 1..rod.x.len() {
            rod.predicted_x[i][0] = 2e-3;
            rod.x[i][0] = 1e-3;
        }
        assert!(
            super::HairSystem::feasible_elastic_prefix_admitted(
                &[rod.clone()],
                &[],
                &before,
                dt,
                radius,
                true
            )
            .unwrap(),
            "fixture must decrease energy while preserving roots and stretch"
        );
        let mesh = super::TriangleMesh::new(
            &[[0., -1., 0.012], [0., 1., 0.012], [0., 0., 0.03]],
            &[[0, 1, 2]],
        )
        .unwrap();
        let mut endpoint = rod.clone();
        endpoint.old_x.clone_from(&endpoint.x);
        assert!(
            super::mesh_motion::admit(&[endpoint], std::slice::from_ref(&mesh), radius).is_ok(),
            "the final geometry must be clear independently of the crossing path"
        );
        assert!(
            !super::HairSystem::feasible_elastic_prefix_admitted(
                &[rod],
                &[mesh],
                &before,
                dt,
                radius,
                true
            )
            .unwrap(),
            "a descending clear endpoint cannot admit a path through the triangle"
        );
    }
    use super::*;
    #[test]
    fn feasible_elastic_progress_is_not_zero_contact_stagnation() {
        let merit = vec![0.; 9];
        let energy = vec![Some(-1e-25); 9]; // Hidden by subtraction from an8e-5J total.
        assert!(HairSystem::nonlinear_progress(&merit, &energy));
        assert!(!HairSystem::nonlinear_progress(&merit, &[Some(0.); 9]));
        assert!(!HairSystem::nonlinear_progress(&merit, &[Some(1e-25); 9]));
        assert!(!HairSystem::nonlinear_progress(&merit, &[None; 9]));
        let mut cancelling = vec![Some(0.); 9];
        cancelling[1] = Some(-1.);
        cancelling[2] = Some(-1e-25);
        cancelling[3] = Some(1.);
        assert!(
            HairSystem::nonlinear_progress(&merit, &cancelling),
            "small net work survives large cancelling moves"
        );
        cancelling[4] = Some(f64::NAN);
        assert!(!HairSystem::nonlinear_progress(&merit, &cancelling));
        assert!(!HairSystem::nonlinear_progress(&merit, &[]));
        let mut penetrating = vec![1.; 9];
        penetrating[8] = 0.95;
        assert!(!HairSystem::nonlinear_progress(&penetrating, &energy));
        penetrating[8] = 0.8;
        assert!(HairSystem::nonlinear_progress(&penetrating, &energy));
    }
    #[test]
    fn continuous_joint_step_activates_an_unseen_pair_and_admits_its_entire_path() {
        let radius = 40e-6;
        let mut rods: Vec<_> = [-0.5e-3, 0.5e-3]
            .into_iter()
            .map(|x| {
                HairRod::new(
                    vec![[x, 0., 0.], [x, 0., 0.1], [x, 0., 0.2]],
                    HairMaterial::default(),
                )
                .unwrap()
            })
            .collect();
        let start = rods.clone();
        for (r, rod) in rods.iter_mut().enumerate() {
            for point in &mut rod.predicted_x[1..] {
                point[0] += if r == 0 { 0.55e-3 } else { -0.55e-3 };
            }
        }
        assert!(contact::refresh_strand_responses(&mut rods, radius, &[]).is_empty());
        let mut history = Vec::new();
        // Match the runtime: structural motion owns the free load and discovers
        // swept pairs before the contact-only reconciliation loop.
        contact::advance_swept_strands(
            &mut rods,
            1. / 240.,
            radius,
            None,
            &mut history,
            Some(&start),
        )
        .unwrap();
        HairSystem::reconcile_positions_mode(
            &mut rods,
            &[],
            1. / 240.,
            radius,
            true,
            &mut history,
            None,
            None,
            true,
        )
        .unwrap();
        let pairs = contact::refresh_strand_responses(&mut rods, radius, &[]);
        assert!(contact::strand_geometry_admitted(&rods, &pairs, radius).unwrap());
        let end: Vec<_> = rods.iter().map(|rod| rod.x.clone()).collect();
        assert_eq!(contact::strand_fraction(&start, &end, radius).unwrap(), 1.);
        for (rod, old) in rods.iter().zip(&start) {
            assert_eq!(rod.x[0], old.x[0]);
            assert!(
                (rod.x[2][0] - old.x[2][0]).abs() > 0.3e-3,
                "a frozen guide cannot qualify this motion"
            );
            assert!(rod.max_relative_stretch() < 0.05);
        }
    }
    #[test]
    fn newly_discovered_contact_merit_is_continuous_at_admission_tolerance() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 1.], [0., 0., 2.]],
            HairMaterial::default(),
        )
        .unwrap();
        let empty = HairSystem::contact_merit(&[rod.clone()], &[], 40e-6).unwrap();
        let row = rod.record_contact(1, 0., [1., 0., 0.], [1e-10, 0., 1.], ContactSource::Mesh(0));
        assert_eq!(
            HairSystem::contact_merit(&[rod.clone()], &[], 40e-6).unwrap(),
            empty
        );
        rod.contacts[row].target[0] += 1e-13;
        let violation = HairSystem::contact_merit(&[rod], &[], 40e-6).unwrap();
        assert!(
            violation > 0. && violation < 2e-26,
            "activation must not add a tolerance-squared discontinuity"
        );
    }
    #[test]
    fn mesh_merit_is_invariant_to_redundant_segment_witnesses() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 1.], [0., 0., 2.]],
            HairMaterial::default(),
        )
        .unwrap();
        rod.record_contact(1, 0., [1., 0., 0.], [0.1, 0., 1.], ContactSource::Mesh(0));
        let initial = HairSystem::contact_merit(&[rod.clone()], &[], 40e-6).unwrap();
        rod.record_contact(1, 0.5, [1., 0., 0.], [0.1, 0., 1.5], ContactSource::Mesh(0));
        assert_eq!(
            initial,
            HairSystem::contact_merit(&[rod.clone()], &[], 40e-6).unwrap()
        );
        rod.record_contact(
            1,
            0.25,
            [1., 0., 0.],
            [0.2, 0., 1.25],
            ContactSource::Mesh(0),
        );
        assert!(
            (HairSystem::contact_merit(&[rod], &[], 40e-6).unwrap() - (0.2f64 - 1e-10).powi(2))
                .abs()
                < 1e-15
        );
    }
}

#[path = "mesh_motion.rs"]
mod mesh_motion;

#[cfg(test)]
#[path = "substep_clock_tests.rs"]
mod substep_clock_tests;

mod contact_expanded;
pub use contact_expanded::HairResponseCoefficient;

/// Both native coefficient components remain owned by the immutable physical operator.
#[derive(Debug)]
pub struct HairExpandedCoordinateRequest<'a> {
    pub columns: &'a [Vec<HairResponseCoefficient>],
    pub bounds: &'a [f64],
    pub tolerance: f64,
}

/// Evaluates the discrete Kirchhoff / Cosserat elastic rod bending and twist strain energy
/// $E = \frac{1}{2} \ell \left[ B_1 (\kappa_1 - \bar{\kappa}_1)^2 + B_2 (\kappa_2 - \bar{\kappa}_2)^2 + C (\omega - \bar{\omega})^2 \right]$.
#[must_use]
pub fn cosserat_rod_bending_twist_energy(
    curvature_kappa: [f64; 2],
    rest_kappa: [f64; 2],
    twist_omega: f64,
    rest_omega: f64,
    bend_stiffness: [f64; 2],
    twist_stiffness: f64,
    segment_len: f64,
) -> Option<f64> {
    if !curvature_kappa[0].is_finite()
        || !curvature_kappa[1].is_finite()
        || !rest_kappa[0].is_finite()
        || !rest_kappa[1].is_finite()
        || !twist_omega.is_finite()
        || !rest_omega.is_finite()
        || !bend_stiffness[0].is_finite()
        || !bend_stiffness[1].is_finite()
        || !twist_stiffness.is_finite()
        || !segment_len.is_finite()
        || bend_stiffness[0] < 0.0
        || bend_stiffness[1] < 0.0
        || twist_stiffness < 0.0
        || segment_len <= 0.0
    {
        return None;
    }
    let dk0 = curvature_kappa[0] - rest_kappa[0];
    let dk1 = curvature_kappa[1] - rest_kappa[1];
    let dw = twist_omega - rest_omega;
    let energy = 0.5
        * segment_len
        * (bend_stiffness[0] * dk0 * dk0
            + bend_stiffness[1] * dk1 * dk1
            + twist_stiffness * dw * dw);
    energy.is_finite().then_some(energy)
}

#[cfg(test)]
mod cosserat_energy_tests {
    use super::*;

    #[test]
    fn cosserat_bending_twist_energy_is_zero_at_rest_and_quadratic_in_strain() {
        let e0 = cosserat_rod_bending_twist_energy(
            [0.5, -0.2],
            [0.5, -0.2],
            0.1,
            0.1,
            [1e-6, 1e-6],
            5e-7,
            0.01,
        )
        .unwrap();
        assert!(e0.abs() < 1e-18);
        let e1 = cosserat_rod_bending_twist_energy(
            [1.5, -0.2],
            [0.5, -0.2],
            0.1,
            0.1,
            [1e-6, 1e-6],
            5e-7,
            0.01,
        )
        .unwrap();
        let e2 = cosserat_rod_bending_twist_energy(
            [2.5, -0.2],
            [0.5, -0.2],
            0.1,
            0.1,
            [1e-6, 1e-6],
            5e-7,
            0.01,
        )
        .unwrap();
        assert!((e2 / e1 - 4.0).abs() < 1e-12);
    }
}
