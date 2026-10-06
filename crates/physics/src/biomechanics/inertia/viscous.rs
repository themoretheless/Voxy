//! Objective Maxwell relaxation split around frozen-memory inertial mechanics.
use super::super::{columns, mm, sub};
use super::{
    Body, DrivenSupportStep, InertialBody, PlaneContact, PrescribedTriangleSurface, SupportTarget,
    Vec3,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct ViscoelasticDynamicStep {
    pub support: DrivenSupportStep,
    /// Released Maxwell energy. Already deposited internally when cell thermal
    /// storage is enabled; otherwise the caller owns its thermal destination.
    pub viscous_heat_j: f64,
    pub relaxation_energy_defect_j: f64,
    pub thermal_energy_defect_j: f64,
    pub total_energy_defect_j: f64,
    /// Absolute defects of both relaxation/thermal stages and frozen mechanics.
    pub absolute_energy_defect_j: f64,
}
/// Receipt for one full interval admitted as equal implicit substeps.
#[derive(Clone, Copy, Debug)]
pub struct ViscoelasticAdaptiveStep {
    pub step: ViscoelasticDynamicStep,
    pub skin_work: super::super::EmbeddedSkinWork,
    pub substeps: usize,
    /// Sum of absolute mechanical, relaxation and thermal defects; no cancellation.
    pub absolute_energy_defect_j: f64,
}
impl InertialBody {
    /// Creates dynamics with committed Ogden-Maxwell histories and initially
    /// stationary supports. Free-body input with no pins is also accepted.
    /// Experimental HGO stress memory is not a passive Maxwell energy model and
    /// is rejected. Material histories are advanced only by the dedicated step.
    /// # Errors
    /// Invalid geometry/density/velocity, moving initial pins, absent Maxwell
    /// material, noncommitted histories or unsupported constitutive memory.
    pub fn new_viscoelastic_with_supports(
        body: Body,
        densities: &[f64],
        velocities: Vec<Vec3>,
    ) -> Result<Self, &'static str> {
        if !body.elements.iter().any(|e| e.viscoelastic.is_some()) {
            return Err("viscoelastic dynamics requires Ogden-Maxwell material");
        }
        Self::new_supported_material_mode(body, densities, velocities, true)
    }
    pub(super) fn require_time_independent_material(&self) -> Result<(), &'static str> {
        if self
            .body
            .elements
            .iter()
            .any(|e| e.viscoelastic.is_some() || e.viscoelastic_hgo.is_some())
        {
            return Err("material history requires viscoelastic inertial step");
        }
        Ok(())
    }
    /// Exact held-pose relaxation for half a step, frozen-history Verlet with
    /// prescribed or stationary supports, then another exact half relaxation.
    /// Work and released heat are accounted independently of energy change.
    /// Separate quarter/half/quarter defect budgets prevent cancellation between
    /// relaxation error and mechanical integration error from hiding a failure.
    /// # Errors
    /// Invalid controls, unsupported/noncommitted memory, inversion/path collapse,
    /// history failure or excessive energy defect preserves the complete owner.
    pub fn step_viscoelastic(
        &mut self,
        targets: Option<&[SupportTarget]>,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<ViscoelasticDynamicStep, &'static str> {
        self.step_viscoelastic_contacts(targets, None, None, dt, energy_tolerance_j)
    }
    /// Viscoelastic counterpart of `step_with_moving_plane`, with the same
    /// independent plane-work receipt and atomic history/heat ownership.
    /// # Errors
    /// Any failed relaxation, contact, geometry or work guard preserves all state.
    pub fn step_viscoelastic_with_moving_plane(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_offset_m: f64,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<ViscoelasticDynamicStep, &'static str> {
        let plane = self.plane_at_offset(next_offset_m)?;
        self.step_viscoelastic_contacts(targets, Some(plane), None, dt, energy_tolerance_j)
    }
    /// History/thermal counterpart of `step_with_plane_motion`.
    /// # Errors
    /// Invalid plane motion or any failed admission preserves the complete owner.
    pub fn step_viscoelastic_with_plane_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_plane: PlaneContact,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<ViscoelasticDynamicStep, &'static str> {
        self.step_viscoelastic_contacts(targets, Some(next_plane), None, dt, energy_tolerance_j)
    }
    /// Prescribed triangle contact with the same atomic Maxwell/thermal owner.
    /// # Errors
    /// Invalid geometry, changed owner, swept contact or work/heat rejection
    /// preserves both the material state and the admitted obstacle pose.
    pub fn step_viscoelastic_with_surface_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_surface: Arc<PrescribedTriangleSurface>,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<ViscoelasticDynamicStep, &'static str> {
        self.step_viscoelastic_contacts(targets, None, Some(next_surface), dt, energy_tolerance_j)
    }
    fn step_viscoelastic_contacts(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_plane: Option<PlaneContact>,
        next_surface: Option<Arc<PrescribedTriangleSurface>>,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<ViscoelasticDynamicStep, &'static str> {
        self.step_viscoelastic_contacts_impl::<false>(
            targets,
            next_plane,
            next_surface,
            dt,
            energy_tolerance_j,
            None,
        )
        .map(|(report, _)| report)
    }
    /// Transactional Maxwell/thermal split with implicit midpoint contact motion.
    /// # Errors
    /// Invalid material, nonlinear/path/work rejection rolls back all state.
    pub fn step_viscoelastic_implicit_with_surface_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_surface: Arc<PrescribedTriangleSurface>,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<ViscoelasticDynamicStep, &'static str> {
        self.step_viscoelastic_contacts_impl::<true>(
            targets,
            None,
            Some(next_surface),
            dt,
            energy_tolerance_j,
            None,
        )
        .map(|(report, _)| report)
    }
    /// Maxwell/thermal transaction with moving embedded skin using shared Verlet mechanics.
    /// # Errors
    /// Any material, motion, CCD or work/heat failure preserves all state.
    pub fn step_viscoelastic_with_embedded_skin_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next: super::super::StationaryEmbeddedContact,
        dt: f64,
        tolerance_j: f64,
    ) -> Result<(ViscoelasticDynamicStep, super::super::EmbeddedSkinWork), &'static str> {
        self.step_viscoelastic_contacts_impl::<false>(
            targets,
            None,
            None,
            dt,
            tolerance_j,
            Some(next),
        )
    }
    /// Atomic Maxwell/thermal split with shared implicit native and moving-skin contact.
    /// Both contact owners must be installed. Embedded work is returned separately
    /// and already included in the support receipt's surface work.
    /// # Errors
    /// Relaxation, heat, owner, CCD, nonlinear or work rejection preserves all state.
    pub fn step_viscoelastic_implicit_with_surface_and_skin_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_surface: Arc<PrescribedTriangleSurface>,
        next_skin: super::super::StationaryEmbeddedContact,
        dt: f64,
        tolerance_j: f64,
    ) -> Result<(ViscoelasticDynamicStep, super::super::EmbeddedSkinWork), &'static str> {
        self.step_viscoelastic_contacts_impl::<true>(
            targets,
            None,
            Some(next_surface),
            dt,
            tolerance_j,
            Some(next_skin),
        )
    }
    /// Retry the same full linear prescribed trajectory with 1,2,4,... substeps.
    /// Every substep uses the original solver and its share of the original budget.
    /// Only the complete interval commits. This does not fix single-step feature kinks.
    /// # Errors
    /// Invalid owners/controls, crossing or exhausted subdivision preserves all state.
    pub fn step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_surface: Arc<PrescribedTriangleSurface>,
        next_skin: super::super::StationaryEmbeddedContact,
        dt: f64,
        tolerance_j: f64,
        max_substeps: usize,
    ) -> Result<ViscoelasticAdaptiveStep, &'static str> {
        if !max_substeps.is_power_of_two()
            || max_substeps > 256
            || !dt.is_finite()
            || dt <= 0.
            || !tolerance_j.is_finite()
            || tolerance_j <= 0.
        {
            return Err("invalid adaptive implicit controls");
        }
        let start_surface = self
            .prescribed_surface
            .as_ref()
            .ok_or("surface motion requires installed contact")?
            .clone();
        start_surface.same_owner(&next_surface)?;
        let start_skin = self
            .body
            .stationary_embedded_contact()
            .ok_or("skin motion requires installed contact")?
            .clone();
        start_skin.same_owner(&next_skin)?;
        if let Some(targets) = targets {
            let mut seen = vec![false; self.masses.len()];
            if targets.len() != self.body.pinned.iter().filter(|&&p| p).count() {
                return Err("incomplete prescribed support targets");
            }
            for target in targets {
                if target.node >= seen.len()
                    || !self.body.pinned[target.node]
                    || seen[target.node]
                    || target.position_m.iter().any(|v| !v.is_finite())
                {
                    return Err("invalid prescribed support target");
                }
                seen[target.node] = true;
            }
        }
        let initial = self.diagnostics()?;
        if let Some(witness) = next_skin.prescribed_contact_obstruction(self.body.positions())? {
            if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                eprintln!("PRESCRIBED_SKIN_OBSTRUCTION {witness:?}");
            }
            return Err("prescribed skin obstacle gap is closed");
        }
        let mut count = 1;
        loop {
            let mut candidate = self.clone();
            let mut sum: Option<ViscoelasticDynamicStep> = None;
            let mut work = super::super::EmbeddedSkinWork::default();
            let mut absolute = 0.;
            let mut rejection = None;
            for index in 1..=count {
                let phase = index as f64 / count as f64;
                let surface = if index == count {
                    next_surface.clone()
                } else {
                    Arc::new(
                        start_surface.with_positions(
                            start_surface
                                .positions()
                                .iter()
                                .zip(next_surface.positions())
                                .map(|(&a, &b)| {
                                    super::super::surface_distance::trajectory_point(a, b, phase)
                                })
                                .collect(),
                        )?,
                    )
                };
                let skin = start_skin.sample_linear_pose(&next_skin, phase)?;
                let staged_targets = targets.map(|targets| {
                    targets
                        .iter()
                        .map(|target| SupportTarget {
                            node: target.node,
                            position_m: if index == count {
                                target.position_m
                            } else {
                                super::super::surface_distance::trajectory_point(
                                    self.body.positions[target.node],
                                    target.position_m,
                                    phase,
                                )
                            },
                        })
                        .collect::<Vec<_>>()
                });
                match candidate.step_viscoelastic_implicit_with_surface_and_skin_motion(
                    staged_targets.as_deref(),
                    surface,
                    skin,
                    dt / count as f64,
                    tolerance_j / count as f64,
                ) {
                    Ok((report, skin_work)) => {
                        absolute += report.absolute_energy_defect_j;
                        work.rig_work_j += skin_work.rig_work_j;
                        work.obstacle_work_j += skin_work.obstacle_work_j;
                        if let Some(sum) = sum.as_mut() {
                            sum.support.support_work_j += report.support.support_work_j;
                            sum.support.reaction_work_j += report.support.reaction_work_j;
                            sum.support.pin_kinetic_work_j += report.support.pin_kinetic_work_j;
                            sum.support.plane_work_j += report.support.plane_work_j;
                            sum.support.plane_translation_work_j +=
                                report.support.plane_translation_work_j;
                            sum.support.plane_rotation_work_j +=
                                report.support.plane_rotation_work_j;
                            sum.support.surface_work_j += report.support.surface_work_j;
                            sum.support.energy_defect_j += report.support.energy_defect_j;
                            sum.viscous_heat_j += report.viscous_heat_j;
                            sum.relaxation_energy_defect_j += report.relaxation_energy_defect_j;
                            sum.thermal_energy_defect_j += report.thermal_energy_defect_j;
                            sum.total_energy_defect_j += report.total_energy_defect_j;
                            sum.absolute_energy_defect_j += report.absolute_energy_defect_j;
                        } else {
                            sum = Some(report);
                        }
                    }
                    Err(error) => {
                        rejection = Some(error);
                        break;
                    }
                }
            }
            if let Some(error) = rejection {
                match error {
                    // Initial diagnostics above admit the committed state. A closed
                    // barrier in a trial pose can therefore be retried at smaller dt;
                    // every accepted substep still requires the original CCD and budget.
                    "closed surface contact gap"
                    | "implicit contact nonlinear nonconvergence"
                    | "implicit contact line search failed"
                    | "implicit contact quadrature nonconvergence"
                    | "implicit midpoint work defect"
                    | "viscoelastic relaxation energy defect"
                    | "viscoelastic inertial work heat defect" => {
                        if count == max_substeps {
                            return Err(error);
                        }
                        count *= 2;
                        continue;
                    }
                    _ => return Err(error),
                }
            }
            let sum = sum.ok_or("empty adaptive implicit interval")?;
            let final_state = candidate.diagnostics()?;
            let independent = final_state.kinetic_j - initial.kinetic_j + final_state.potential_j
                - initial.potential_j
                + sum.viscous_heat_j
                - sum.support.support_work_j
                - sum.support.plane_work_j
                - sum.support.surface_work_j;
            let finite_receipt = [
                sum.support.support_work_j,
                sum.support.reaction_work_j,
                sum.support.pin_kinetic_work_j,
                sum.support.plane_work_j,
                sum.support.plane_translation_work_j,
                sum.support.plane_rotation_work_j,
                sum.support.surface_work_j,
                sum.support.energy_defect_j,
                sum.viscous_heat_j,
                sum.relaxation_energy_defect_j,
                sum.thermal_energy_defect_j,
                sum.total_energy_defect_j,
                sum.absolute_energy_defect_j,
            ]
            .iter()
            .all(|v| v.is_finite());
            if !finite_receipt
                || !absolute.is_finite()
                || absolute > tolerance_j
                || !independent.is_finite()
                || independent.abs() > tolerance_j
                || !work.rig_work_j.is_finite()
                || !work.obstacle_work_j.is_finite()
            {
                return Err("adaptive implicit interval work heat defect");
            }
            *self = candidate;
            return Ok(ViscoelasticAdaptiveStep {
                step: sum,
                skin_work: work,
                substeps: count,
                absolute_energy_defect_j: absolute,
            });
        }
    }
    fn step_viscoelastic_contacts_impl<const IMPLICIT: bool>(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_plane: Option<PlaneContact>,
        next_surface: Option<Arc<PrescribedTriangleSurface>>,
        dt: f64,
        energy_tolerance_j: f64,
        next_skin: Option<super::super::StationaryEmbeddedContact>,
    ) -> Result<(ViscoelasticDynamicStep, super::super::EmbeddedSkinWork), &'static str> {
        if !dt.is_finite()
            || dt * 0.5 <= 0.
            || !energy_tolerance_j.is_finite()
            || energy_tolerance_j * 0.25 <= 0.
        {
            return Err("invalid viscoelastic inertial step");
        }
        if !self.body.elements.iter().any(|e| e.viscoelastic.is_some())
            || self.body.elements.iter().any(|e| {
                e.viscoelastic_hgo.is_some()
                    || e.viscoelastic
                        .as_ref()
                        .is_some_and(|v| v.trial_seconds != 0.)
            })
        {
            return Err("unsupported viscoelastic inertial material");
        }
        if targets.is_none() {
            self.require_stationary_supports()?;
        }
        let mut candidate = self.clone();
        let (first_heat, first_defect, first_thermal_defect) = candidate.relax_maxwell(0.5 * dt)?;
        if first_defect.abs() + first_thermal_defect.abs() > 0.25 * energy_tolerance_j {
            return Err("viscoelastic relaxation energy defect");
        }
        let (support, skin_work) = if IMPLICIT {
            candidate.advance_implicit_surface_and_skin(
                targets,
                next_surface.ok_or("implicit step requires prescribed surface")?,
                next_skin,
                dt,
                0.5 * energy_tolerance_j,
            )?
        } else {
            candidate.advance_supports_with_contact_motion(
                dt,
                0.5 * energy_tolerance_j,
                targets,
                next_plane,
                next_surface,
                next_skin,
            )?
        };
        let (second_heat, second_defect, second_thermal_defect) =
            candidate.relax_maxwell(0.5 * dt)?;
        if second_defect.abs() + second_thermal_defect.abs() > 0.25 * energy_tolerance_j {
            return Err("viscoelastic relaxation energy defect");
        }
        let heat = first_heat + second_heat;
        let relaxation_defect = first_defect + second_defect;
        let thermal_defect = first_thermal_defect + second_thermal_defect;
        let total_defect = support.energy_defect_j + relaxation_defect + thermal_defect;
        let absolute_defect = support.energy_defect_j.abs()
            + first_defect.abs()
            + first_thermal_defect.abs()
            + second_defect.abs()
            + second_thermal_defect.abs();
        if !heat.is_finite()
            || !total_defect.is_finite()
            || total_defect.abs() > energy_tolerance_j
            || !absolute_defect.is_finite()
            || absolute_defect > energy_tolerance_j
        {
            return Err("viscoelastic inertial work heat defect");
        }
        *self = candidate;
        Ok((
            ViscoelasticDynamicStep {
                support,
                viscous_heat_j: heat,
                relaxation_energy_defect_j: relaxation_defect,
                thermal_energy_defect_j: thermal_defect,
                total_energy_defect_j: total_defect,
                absolute_energy_defect_j: absolute_defect,
            },
            skin_work,
        ))
    }
    fn relax_maxwell(&mut self, dt: f64) -> Result<(f64, f64, f64), &'static str> {
        // Pose, external potentials and equilibrium elasticity are unchanged.
        // Check the only variable energy locally, avoiding full-body force
        // assemblies and cancellation against unrelated large potentials.
        let mut heat = 0.;
        let mut energy_change = 0.;
        let mut thermal_defect = 0.;
        for (cell, element) in self.body.elements.iter_mut().enumerate() {
            if let Some(law) = &mut element.viscoelastic {
                let [a, b, c, d] = element.nodes.map(|i| self.body.positions[i]);
                let f = mm(columns(sub(b, a), sub(c, a), sub(d, a)), element.inv_rest);
                let before = law.maxwell_energy_density(f)?;
                let released = element.volume * law.relax_exact(f, dt)?;
                heat += released;
                if let Some(thermal) = &mut self.thermal {
                    thermal_defect += thermal.deposit(cell, released)?;
                }
                energy_change += element.volume * (law.maxwell_energy_density(f)? - before);
            }
        }
        let defect = energy_change + heat;
        if !heat.is_finite() || heat < 0. || !defect.is_finite() || !thermal_defect.is_finite() {
            return Err("viscoelastic heat overflow");
        }
        Ok((heat, defect, thermal_defect))
    }
}
