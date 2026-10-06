//! Finite-deformation elastic dynamics with velocity Verlet and prescribed supports.
use super::{Body, PrescribedTriangleSurface, Vec3, cross, dot};
use std::sync::Arc;
mod film_binding;
mod implicit;
mod supports;
mod thermal;
pub use film_binding::SolidFilmBinding;
mod viscous;
pub use supports::{DrivenSupportStep, SupportTarget};
pub use viscous::ViscoelasticDynamicStep;
#[derive(Clone, Debug)]
pub struct InertialBody {
    body: Body,
    masses: Vec<f64>,
    cell_masses: Vec<f64>,
    thermal: Option<thermal::CellThermalState>,
    velocities: Vec<Vec3>,
    acceleration: Vec3,
    plane: Option<PlaneContact>,
    prescribed_surface: Option<Arc<PrescribedTriangleSurface>>,
}
struct PotentialEvaluation {
    potential_j: f64,
    gradient: Vec<Vec3>,
    contact_j: f64,
    plane_offset_gradient: f64,
    plane_rotation_gradient: Vec3,
    surface_gradient: Vec<Vec3>,
    embedded_skin_work: super::EmbeddedSkinWork,
}
/// Frictionless stationary halfspace `normal·x >= offset_m` with nodal penalty.
/// Stiffness is N/m per boundary node and must be scaled with mesh refinement.
#[derive(Clone, Copy, Debug)]
pub struct PlaneContact {
    normal: Vec3,
    offset_m: f64,
    stiffness_n_m: f64,
}
impl PlaneContact {
    /// # Errors
    /// Rejects nonunit/nonfinite normals, invalid offset or nonpositive stiffness.
    pub fn new(normal: Vec3, offset_m: f64, stiffness_n_m: f64) -> Result<Self, &'static str> {
        if normal.iter().any(|v| !v.is_finite())
            || (dot(normal, normal) - 1.).abs() > 1e-10
            || !offset_m.is_finite()
            || !stiffness_n_m.is_finite()
            || stiffness_n_m <= 0.
        {
            return Err("invalid elastic plane contact");
        }
        Ok(Self {
            normal,
            offset_m,
            stiffness_n_m,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct InertialDiagnostics {
    pub mass_kg: f64,
    pub momentum_kg_m_s: Vec3,
    /// Angular momentum about the world origin, including orbital motion.
    pub angular_momentum_kg_m2_s: Vec3,
    pub kinetic_j: f64,
    /// Elastic energy plus constant cavity/dead-load and acceleration potentials.
    pub potential_j: f64,
    /// Contact potential, already included in `potential_j`.
    pub contact_j: f64,
}
impl InertialBody {
    /// Reference densities in kg/m³ per cell, nodal velocities in m/s.
    /// Uses the existing objective finite-deformation constitutive responses.
    /// No pins or viscoelastic histories: only time-independent potentials apply.
    /// # Errors
    /// Invalid density/velocity input, pins, history-dependent materials or overflow.
    pub fn new(body: Body, densities: &[f64], velocities: Vec<Vec3>) -> Result<Self, &'static str> {
        Self::new_material_mode(body, densities, velocities, false)
    }
    fn new_material_mode(
        body: Body,
        densities: &[f64],
        velocities: Vec<Vec3>,
        allow_maxwell: bool,
    ) -> Result<Self, &'static str> {
        if densities.len() != body.elements.len()
            || densities.iter().any(|d| !d.is_finite() || *d <= 0.)
            || velocities.len() != body.rest.len()
            || velocities.iter().flatten().any(|v| !v.is_finite())
            || body.pinned.contains(&true)
            || body.elements.iter().any(|e| {
                e.viscoelastic_hgo.is_some()
                    || e.viscoelastic
                        .as_ref()
                        .is_some_and(|law| !allow_maxwell || law.trial_seconds != 0.)
            })
        {
            return Err("invalid finite-deformation inertial input");
        }
        let cell_masses: Vec<_> = body
            .elements
            .iter()
            .zip(densities)
            .map(|(cell, density)| cell.volume * density)
            .collect();
        if cell_masses
            .iter()
            .any(|mass| !mass.is_finite() || *mass <= 0.)
        {
            return Err("invalid finite-deformation cell mass");
        }
        let mut masses = vec![0.; body.rest.len()];
        for (cell, &mass) in body.elements.iter().zip(&cell_masses) {
            for &node in &cell.nodes {
                masses[node] += mass / 4.;
            }
        }
        if masses.iter().any(|m| !m.is_finite() || *m <= 0.) {
            return Err("invalid finite-deformation nodal mass");
        }
        let result = Self {
            body,
            masses,
            cell_masses,
            thermal: None,
            velocities,
            acceleration: [0.; 3],
            plane: None,
            prescribed_surface: None,
        };
        result.diagnostics()?;
        Ok(result)
    }
    /// Construct dynamics with stationary fully fixed nodal supports retained.
    /// Initial velocities on fixed nodes must be exactly zero. Support reactions
    /// do no work while stationary. Use `step_with_support_targets` for prescribed
    /// support motion after construction.
    /// # Errors
    /// Same checks as `new`, except stationary pins are accepted.
    pub fn new_with_fixed_supports(
        body: Body,
        densities: &[f64],
        velocities: Vec<Vec3>,
    ) -> Result<Self, &'static str> {
        Self::new_supported_material_mode(body, densities, velocities, false)
    }
    fn new_supported_material_mode(
        mut body: Body,
        densities: &[f64],
        velocities: Vec<Vec3>,
        allow_maxwell: bool,
    ) -> Result<Self, &'static str> {
        if velocities.len() != body.positions.len()
            || body
                .pinned
                .iter()
                .zip(&velocities)
                .any(|(p, v)| *p && v.iter().any(|x| *x != 0.))
        {
            return Err("invalid fixed support velocity");
        }
        let pins = body.pinned.clone();
        body.pinned.fill(false);
        let mut result = Self::new_material_mode(body, densities, velocities, allow_maxwell)?;
        result.body.pinned = pins;
        Ok(result)
    }
    /// Full stationary support reaction forces, N; unconstrained nodes return zero.
    /// Includes muscle velocity forces, gravity, dead loads and plane contact.
    /// # Errors
    /// Constitutive or force evaluation failure.
    pub fn muscle_support_reactions(
        &self,
        law: super::ActiveFiberVelocityLaw,
    ) -> Result<Vec<Vec3>, &'static str> {
        self.require_time_independent_material()?;
        self.require_stationary_supports()?;
        let gradient = self.evaluate()?.1;
        let correction = self.body.active_velocity_forces(&self.velocities, law)?;
        Ok(gradient
            .iter()
            .enumerate()
            .map(|(i, g)| {
                if !self.body.pinned[i] {
                    return [0.; 3];
                }
                std::array::from_fn(|k| {
                    g[k] - correction.forces_n[i][k] - self.masses[i] * self.acceleration[k]
                })
            })
            .collect())
    }
    #[must_use]
    pub fn body(&self) -> &Body {
        &self.body
    }
    #[must_use]
    pub fn velocities(&self) -> &[Vec3] {
        &self.velocities
    }
    #[must_use]
    pub fn masses(&self) -> &[f64] {
        &self.masses
    }
    /// Set/remove a stationary frictionless penalty plane. Returns the change
    /// in potential energy from changing this parameter at fixed positions.
    /// # Errors
    /// Rejects overflowing contact energy, without modifying state.
    pub fn set_plane_contact(&mut self, plane: Option<PlaneContact>) -> Result<f64, &'static str> {
        let before = self.diagnostics()?.potential_j;
        let mut candidate = self.clone();
        candidate.plane = plane;
        let change = candidate.diagnostics()?.potential_j - before;
        if !change.is_finite() {
            return Err("contact potential overflow");
        }
        *self = candidate;
        Ok(change)
    }
    fn evaluate(&self) -> Result<(f64, Vec<Vec3>, f64), &'static str> {
        self.evaluate_at(&self.body.positions)
    }
    /// Install/remove a prescribed triangle obstacle at the current body pose.
    /// Returns the parameter-induced potential change; this is distinct from
    /// actuator work during a subsequent motion step.
    /// # Errors
    /// A closed contact gap or invalid response leaves the previous owner intact.
    pub fn set_prescribed_surface(
        &mut self,
        surface: Option<Arc<PrescribedTriangleSurface>>,
    ) -> Result<f64, &'static str> {
        let before = self.diagnostics()?.potential_j;
        let mut candidate = self.clone();
        candidate.prescribed_surface = surface;
        let work = candidate.diagnostics()?.potential_j - before;
        if !work.is_finite() {
            return Err("prescribed surface parameter work overflow");
        }
        *self = candidate;
        Ok(work)
    }
    /// Stationary embedded skin contact uses the existing potential/CCD/step pipeline.
    /// Returns parameter work at fixed mechanical state; not finite-time rig motion.
    /// # Errors
    /// Invalid owner, closed gap or nonfinite energy leaves the body unchanged.
    pub fn set_stationary_embedded_contact(
        &mut self,
        contact: Option<super::StationaryEmbeddedContact>,
    ) -> Result<f64, &'static str> {
        self.body.set_stationary_embedded_contact(contact)
    }
    #[must_use]
    pub fn prescribed_surface(&self) -> Option<&PrescribedTriangleSurface> {
        self.prescribed_surface.as_deref()
    }
    // Frozen constitutive history can be evaluated at trial positions without
    // copying topology, materials, or thermal storage.
    fn evaluate_at(&self, positions: &[Vec3]) -> Result<(f64, Vec<Vec3>, f64), &'static str> {
        self.evaluate_at_contacts(positions, self.plane, self.prescribed_surface.as_deref())
            .map(|response| (response.potential_j, response.gradient, response.contact_j))
    }
    // The offset derivative is evaluated beside the same contact forces. It
    // supplies independent actuator work for a translating prescribed obstacle.
    fn evaluate_at_contacts(
        &self,
        positions: &[Vec3],
        plane: Option<PlaneContact>,
        surface: Option<&PrescribedTriangleSurface>,
    ) -> Result<PotentialEvaluation, &'static str> {
        self.evaluate_at_contacts_and_skin(
            positions,
            plane,
            surface,
            self.body.embedded_contact.as_ref(),
        )
    }
    fn evaluate_at_contacts_and_skin(
        &self,
        positions: &[Vec3],
        plane: Option<PlaneContact>,
        surface: Option<&PrescribedTriangleSurface>,
        skin: Option<&super::StationaryEmbeddedContact>,
    ) -> Result<PotentialEvaluation, &'static str> {
        let (mut energy, mut gradient, embedded_contact_j) = self
            .body
            .evaluate_with_embedded_contact_state(positions, skin)?;
        let mut contact = embedded_contact_j;
        let mut offset_gradient = 0.;
        let mut rotation_gradient = [0.; 3];
        if let Some(plane) = plane {
            let mut plane_contact_j = 0.;
            let mut boundary = vec![false; self.masses.len()];
            for face in self.body.surface() {
                for node in face {
                    boundary[node] = true;
                }
            }
            for (node, &on_surface) in boundary.iter().enumerate() {
                if !on_surface {
                    continue;
                }
                let gap = dot(plane.normal, positions[node]) - plane.offset_m;
                if !gap.is_finite() {
                    return Err("plane gap overflow");
                }
                if gap < 0. {
                    plane_contact_j += 0.5 * plane.stiffness_n_m * gap * gap;
                    offset_gradient -= plane.stiffness_n_m * gap;
                    let moment = cross(plane.normal, positions[node]);
                    for axis in 0..3 {
                        rotation_gradient[axis] += plane.stiffness_n_m * gap * moment[axis];
                    }
                    for (axis, value) in gradient[node].iter_mut().enumerate() {
                        *value += plane.stiffness_n_m * gap * plane.normal[axis];
                    }
                }
            }
            energy += plane_contact_j;
            contact += plane_contact_j;
        }
        let mut surface_gradient = Vec::new();
        if let Some(surface) = surface {
            let response = surface.response(positions, &self.body.surface())?;
            energy += response.potential_j;
            contact += response.potential_j;
            for (body_gradient, contact_gradient) in
                gradient.iter_mut().zip(response.body_gradient_n)
            {
                for axis in 0..3 {
                    body_gradient[axis] += contact_gradient[axis];
                }
            }
            surface_gradient = response.obstacle_gradient_n;
        }
        if !energy.is_finite() || gradient.iter().flatten().any(|v| !v.is_finite()) {
            return Err("contact evaluation overflow");
        }
        Ok(PotentialEvaluation {
            potential_j: energy,
            gradient,
            contact_j: contact,
            plane_offset_gradient: offset_gradient,
            plane_rotation_gradient: rotation_gradient,
            surface_gradient,
            embedded_skin_work: super::EmbeddedSkinWork::default(),
        })
    }
    /// Set constant uniform acceleration in m/s², e.g. gravity. It is mass
    /// weighted and adds to existing dead loads. Returns the change in potential
    /// energy caused by changing the acceleration parameter at fixed positions;
    /// account for this separately when comparing energy across parameter changes.
    /// # Errors
    /// Nonfinite acceleration or potential overflow leaves the body untouched.
    pub fn set_uniform_acceleration(&mut self, acceleration: Vec3) -> Result<f64, &'static str> {
        if acceleration.iter().any(|v| !v.is_finite()) {
            return Err("invalid uniform acceleration");
        }
        let before = self.diagnostics()?.potential_j;
        let mut candidate = self.clone();
        candidate.acceleration = acceleration;
        let change = candidate.diagnostics()?.potential_j - before;
        if !change.is_finite() {
            return Err("acceleration potential overflow");
        }
        *self = candidate;
        Ok(change)
    }
    /// # Errors
    /// Constitutive errors or overflowing energy/momentum.
    pub fn diagnostics(&self) -> Result<InertialDiagnostics, &'static str> {
        let (potential, _, contact) = self.evaluate()?;
        self.diagnostics_from_potential(potential, contact)
    }
    // Reuse a force evaluation at this exact position/history. Velocities may
    // have changed since evaluation; kinetic quantities are always read fresh.
    fn diagnostics_from_potential(
        &self,
        potential: f64,
        contact: f64,
    ) -> Result<InertialDiagnostics, &'static str> {
        self.diagnostics_at(potential, contact, &self.body.positions, &self.velocities)
    }
    fn diagnostics_at(
        &self,
        potential: f64,
        contact: f64,
        positions: &[Vec3],
        velocities: &[Vec3],
    ) -> Result<InertialDiagnostics, &'static str> {
        let mut result = InertialDiagnostics {
            mass_kg: self.masses.iter().sum(),
            momentum_kg_m_s: [0.; 3],
            angular_momentum_kg_m2_s: [0.; 3],
            kinetic_j: 0.,
            potential_j: potential,
            contact_j: contact,
        };
        for ((&mass, &position), &rest) in self.masses.iter().zip(positions).zip(&self.body.rest) {
            result.potential_j -= mass * dot(self.acceleration, super::sub(position, rest));
        }
        for ((&mass, &velocity), &position) in self.masses.iter().zip(velocities).zip(positions) {
            let momentum = velocity.map(|v| mass * v);
            let angular = cross(position, momentum);
            result.kinetic_j += 0.5 * mass * dot(velocity, velocity);
            for axis in 0..3 {
                result.momentum_kg_m_s[axis] += momentum[axis];
                result.angular_momentum_kg_m2_s[axis] += angular[axis];
            }
        }
        if [result.mass_kg, result.kinetic_j, result.potential_j]
            .iter()
            .chain(&result.momentum_kg_m_s)
            .chain(&result.angular_momentum_kg_m2_s)
            .any(|v| !v.is_finite())
        {
            return Err("inertial diagnostic overflow");
        }
        Ok(result)
    }
    /// Velocity Verlet with a per-step absolute energy-defect guard in joules.
    /// Returns signed total-energy change; any failure leaves all state untouched.
    /// This is explicit: stiff solids need a small stability timestep, and the
    /// energy guard does not replace temporal convergence/trajectory checks.
    /// Constant dead loads/cavity pressure are already included in the potential.
    /// # Errors
    /// Invalid timestep/tolerance, inversion, constitutive failure or energy defect.
    pub fn step(&mut self, dt: f64, energy_tolerance_j: f64) -> Result<f64, &'static str> {
        self.require_time_independent_material()?;
        self.require_stationary_supports()?;
        Ok(self
            .advance_supports(dt, energy_tolerance_j, None)?
            .energy_defect_j)
    }
}

/// Work balance of a held-activation rate-dependent muscle step.
#[derive(Clone, Copy, Debug)]
pub struct MuscleDynamicStep {
    pub correction_work_j: f64,
    pub energy_defect_j: f64,
}
impl InertialBody {
    fn muscle_acceleration(
        &self,
        law: super::ActiveFiberVelocityLaw,
    ) -> Result<(Vec<Vec3>, f64), &'static str> {
        self.require_time_independent_material()?;
        self.require_stationary_supports()?;
        let gradient = self.evaluate()?.1;
        let correction = self.body.active_velocity_forces(&self.velocities, law)?;
        let acceleration = gradient
            .iter()
            .enumerate()
            .map(|(node, g)| {
                if self.body.pinned[node] {
                    return [0.; 3];
                }
                std::array::from_fn(|axis| {
                    self.acceleration[axis]
                        + (correction.forces_n[node][axis] - g[axis]) / self.masses[node]
                })
            })
            .collect();
        Ok((acceleration, correction.correction_power_w))
    }
    /// Explicit midpoint integration of positions and velocities with active
    /// force-velocity correction. Activation and material parameters are held fixed.
    /// Work is midpoint power times dt; the guard checks delta(K+U)-work.
    /// # Errors
    /// Invalid controls, inverted intermediate/final geometry, constitutive error
    /// or excess work-balance defect; all failures preserve the original state.
    pub fn step_muscle(
        &mut self,
        dt: f64,
        energy_tolerance_j: f64,
        law: super::ActiveFiberVelocityLaw,
    ) -> Result<MuscleDynamicStep, &'static str> {
        if !dt.is_finite()
            || dt <= 0.
            || !energy_tolerance_j.is_finite()
            || energy_tolerance_j <= 0.
        {
            return Err("invalid muscle dynamic step");
        }
        let before = self.diagnostics()?;
        let (initial_acceleration, _) = self.muscle_acceleration(law)?;
        let mut midpoint = self.clone();
        for (node, a) in initial_acceleration.iter().enumerate() {
            if self.body.pinned[node] {
                continue;
            }
            for axis in 0..3 {
                midpoint.body.positions[node][axis] += 0.5 * dt * self.velocities[node][axis];
                midpoint.velocities[node][axis] += 0.5 * dt * a[axis];
            }
        }
        if !self
            .body
            .gap_path_is_open(&self.body.positions, &midpoint.body.positions)
        {
            return Err("muscle tissue gap predictor crossing");
        }
        let (mid_acceleration, power) = midpoint.muscle_acceleration(law)?;
        let mut candidate = self.clone();
        for (node, a) in mid_acceleration.iter().enumerate() {
            if self.body.pinned[node] {
                continue;
            }
            for axis in 0..3 {
                candidate.body.positions[node][axis] += dt * midpoint.velocities[node][axis];
                candidate.velocities[node][axis] += dt * a[axis];
            }
        }
        if !self
            .body
            .gap_path_is_open(&self.body.positions, &candidate.body.positions)
        {
            return Err("muscle tissue gap path crossing");
        }
        let after = candidate.diagnostics()?;
        let work = dt * power;
        let defect =
            after.kinetic_j - before.kinetic_j + after.potential_j - before.potential_j - work;
        if !work.is_finite() || !defect.is_finite() || defect.abs() > energy_tolerance_j {
            return Err("muscle dynamic work defect");
        }
        *self = candidate;
        Ok(MuscleDynamicStep {
            correction_work_j: work,
            energy_defect_j: defect,
        })
    }
}

/// Mechanical work ledger for coupled excitation, activation and solid motion.
#[derive(Clone, Copy, Debug)]
pub struct DrivenMuscleStep {
    /// Change in active potential due to activation at fixed geometry, in J.
    /// This is parameter work, not ATP expenditure.
    pub activation_work_j: f64,
    pub correction_work_j: f64,
    pub energy_defect_j: f64,
}
impl InertialBody {
    fn activation_half_step(
        &mut self,
        drives: &mut [super::MuscleRegionDrive],
        seconds: f64,
    ) -> Result<f64, &'static str> {
        let before = self.diagnostics()?.potential_j;
        let mut seen = std::collections::BTreeSet::new();
        for drive in drives {
            if !seen.insert(drive.region) {
                return Err("duplicate dynamic muscle region");
            }
            let ids: Vec<_> = self
                .body
                .elements
                .iter()
                .enumerate()
                .filter_map(|(i, e)| (e.region == drive.region).then_some(i))
                .collect();
            if ids.is_empty() {
                return Err("missing dynamic muscle region");
            }
            if ids
                .iter()
                .any(|&i| self.body.elements[i].activation != drive.activation)
            {
                return Err("dynamic muscle activation history mismatch");
            }
            let next = drive
                .kinetics
                .advance(drive.activation, drive.excitation, seconds)?;
            for i in ids {
                self.body.set_activation(i, next)?;
            }
            drive.activation = next;
        }
        let work = self.diagnostics()?.potential_j - before;
        if !work.is_finite() {
            return Err("activation work overflow");
        }
        Ok(work)
    }
    /// Symmetric activation-half / motion-full / activation-half splitting.
    /// Excitation is constant within this step; supplied histories must match the
    /// element activations in their regions. Unspecified regions retain activation.
    /// # Errors
    /// Invalid/missing/duplicate drives, mismatched history, constitutive failure
    /// or work defect rolls back geometry, velocities and caller histories together.
    pub fn step_driven_muscle(
        &mut self,
        drives: &mut [super::MuscleRegionDrive],
        dt: f64,
        energy_tolerance_j: f64,
        law: super::ActiveFiberVelocityLaw,
    ) -> Result<DrivenMuscleStep, &'static str> {
        if drives.is_empty()
            || !dt.is_finite()
            || dt <= 0.
            || !energy_tolerance_j.is_finite()
            || energy_tolerance_j <= 0.
        {
            return Err("invalid driven muscle step");
        }
        let before = self.diagnostics()?;
        let mut candidate = self.clone();
        let mut histories = drives.to_vec();
        let first_work = candidate.activation_half_step(&mut histories, 0.5 * dt)?;
        let motion = candidate.step_muscle(dt, energy_tolerance_j, law)?;
        let activation_work =
            first_work + candidate.activation_half_step(&mut histories, 0.5 * dt)?;
        let after = candidate.diagnostics()?;
        let defect = after.kinetic_j - before.kinetic_j + after.potential_j
            - before.potential_j
            - activation_work
            - motion.correction_work_j;
        if !defect.is_finite() || defect.abs() > energy_tolerance_j {
            return Err("driven muscle work defect");
        }
        *self = candidate;
        drives.copy_from_slice(&histories);
        Ok(DrivenMuscleStep {
            activation_work_j: activation_work,
            correction_work_j: motion.correction_work_j,
            energy_defect_j: defect,
        })
    }
}
