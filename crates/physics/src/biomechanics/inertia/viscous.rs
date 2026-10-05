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
        )
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
        )
    }
    fn step_viscoelastic_contacts_impl<const IMPLICIT: bool>(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_plane: Option<PlaneContact>,
        next_surface: Option<Arc<PrescribedTriangleSurface>>,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<ViscoelasticDynamicStep, &'static str> {
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
        let support = if IMPLICIT {
            candidate.advance_implicit_surface(
                targets,
                next_surface.ok_or("implicit step requires prescribed surface")?,
                dt,
                0.5 * energy_tolerance_j,
            )?
        } else {
            candidate.advance_supports_with_contacts(
                dt,
                0.5 * energy_tolerance_j,
                targets,
                next_plane,
                next_surface,
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
        if !heat.is_finite() || !total_defect.is_finite() || total_defect.abs() > energy_tolerance_j
        {
            return Err("viscoelastic inertial work heat defect");
        }
        *self = candidate;
        Ok(ViscoelasticDynamicStep {
            support,
            viscous_heat_j: heat,
            relaxation_energy_defect_j: relaxation_defect,
            thermal_energy_defect_j: thermal_defect,
            total_energy_defect_j: total_defect,
        })
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
