//! Finite-volume-style thermal surface sampling for a translating rectangular plane patch.
use super::{Error, Liquid, ThermalTranslatingBody, finite, positive};
/// Axis-aligned surface patch in body-local coordinates; no rotation or mesh inference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalPlanePatch {
    pub center: [f64; 3],
    pub axis: usize,
    pub side: i8,
    /// Tangential half extents in increasing coordinate-axis order.
    pub half_extents: [f64; 2],
    /// Maximum gap from the particle's inferred cube face; gap is liquid-filled.
    pub max_gap: f64,
    pub body_thickness: f64,
    pub body_conductivity: f64,
    /// Areal contact resistance, m² K/W in SI.
    pub contact_resistance: f64,
}
impl Liquid {
    /// Geometry-generated conduction before and after mechanics/impact heating.
    /// Each half exchange freezes its own stencil; the second stencil uses the
    /// updated fluid and body positions. No continuum accuracy is implied.
    /// # Errors
    /// Any geometry, thermal or mechanical error preserves both complete states.
    pub fn step_symmetric_with_thermal_plane(
        &mut self,
        dt: f64,
        body: &mut ThermalTranslatingBody,
        world: &impl crate::CollisionWorld,
        config: super::DynamicWorldConfig,
        fluid_heat_fraction: f64,
        patch: ThermalPlanePatch,
    ) -> Result<super::ThermalBodyStep, Error> {
        let mut candidate = self.clone();
        let mut body_candidate = *body;
        let initial = candidate.body_plane_conductances(&body_candidate, patch)?;
        let first =
            candidate.exchange_body_heat_symmetric(0.5 * dt, &mut body_candidate, &initial)?;
        let impacts = candidate.step_symmetric_with_thermal_body(
            dt,
            &mut body_candidate,
            world,
            config,
            fluid_heat_fraction,
        )?;
        let updated = candidate.body_plane_conductances(&body_candidate, patch)?;
        let last =
            candidate.exchange_body_heat_symmetric(0.5 * dt, &mut body_candidate, &updated)?;
        let conductive_heat = first + last;
        if !conductive_heat.is_finite() {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        *body = body_candidate;
        Ok(super::ThermalBodyStep {
            impacts,
            conductive_heat,
        })
    }
    /// Builds per-particle W/K from area overlap and series thermal resistance.
    /// Particle cells are cubes of volume mass/effective rest density. This is a
    /// discretization assumption, not resolved SPH surface/wetting geometry.
    /// Fluid conductivity supplies the center-to-interface path, including the
    /// declared liquid-filled gap. Zero conductivity disconnects the corresponding path.
    /// # Errors
    /// Invalid patch/body/transport, unrepresentable geometry/volume or conductance.
    /// Read-only: no fluid/body state is changed.
    pub fn body_plane_conductances(
        &self,
        body: &ThermalTranslatingBody,
        patch: ThermalPlanePatch,
    ) -> Result<Vec<f64>, Error> {
        body.thermal_energy()?;
        if !finite(body.mechanics.position)
            || !finite(patch.center)
            || patch.axis >= 3
            || !matches!(patch.side, -1 | 1)
            || patch.half_extents.iter().any(|v| !positive(*v))
            || [
                patch.max_gap,
                patch.body_thickness,
                patch.body_conductivity,
                patch.contact_resistance,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(Error::InvalidTransport);
        }
        let center: [f64; 3] =
            std::array::from_fn(|a| patch.center[a] + body.mechanics.position[a]);
        if !finite(center) {
            return Err(Error::NumericalFailure);
        }
        let tangents: Vec<_> = (0..3).filter(|a| *a != patch.axis).collect();
        let fields = self.transport.as_ref().ok_or(Error::InvalidTransport)?;
        let properties = self.effective_materials()?;
        let mut result = vec![0.0; self.particles.len()];
        for (i, particle) in self.particles.iter().enumerate() {
            let volume = particle.mass / properties[i].rest_density;
            if !positive(volume) {
                return Err(Error::NumericalFailure);
            }
            let half = 0.5 * volume.cbrt();
            let distance =
                f64::from(patch.side) * (particle.position[patch.axis] - center[patch.axis]);
            if !distance.is_finite() {
                return Err(Error::NumericalFailure);
            }
            let tolerance = 64.0 * f64::EPSILON * distance.abs().max(half);
            let gap = distance - half;
            if gap < -tolerance || gap > patch.max_gap + tolerance {
                continue;
            }
            let mut area = 1.0;
            for (j, axis) in tangents.iter().enumerate() {
                let bounds = [
                    particle.position[*axis] - half,
                    particle.position[*axis] + half,
                    center[*axis] - patch.half_extents[j],
                    center[*axis] + patch.half_extents[j],
                ];
                if bounds.iter().any(|v| !v.is_finite()) {
                    return Err(Error::NumericalFailure);
                }
                area *= (bounds[1].min(bounds[3]) - bounds[0].max(bounds[2])).max(0.0);
            }
            let conductivity = fields.materials[particle.material].conductivity;
            if area <= 0.0
                || conductivity <= 0.0
                || (patch.body_thickness > 0.0 && patch.body_conductivity <= 0.0)
            {
                continue;
            }
            let body_resistance = if patch.body_thickness > 0.0 {
                patch.body_thickness / patch.body_conductivity
            } else {
                0.0
            };
            let resistance =
                distance.max(half) / conductivity + body_resistance + patch.contact_resistance;
            let conductance = area / resistance;
            if !area.is_finite() || !positive(resistance) || !conductance.is_finite() {
                return Err(Error::NumericalFailure);
            }
            result[i] = conductance;
        }
        Ok(result)
    }
}
