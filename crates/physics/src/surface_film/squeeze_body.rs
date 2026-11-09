//! Normal-only rigid-plane feedback with frozen-gap Reynolds viscous relaxation.
use super::{SqueezePressureControl, SqueezeTransportReport, SurfaceFilm, dot, sub};
use crate::liquid::ThermalTranslatingBody;
#[derive(Clone, Debug)]
pub struct SqueezeBodyReport {
    pub transport: SqueezeTransportReport,
    /// Reaction on the prescribed stationary lower substrate, kg m/s.
    pub substrate_impulse: [f64; 3],
}
impl SurfaceFilm {
    /// A planar body covers the entire vented mesh and initially matches every
    /// filled gap. Velocity must be normal and closing or zero. Frozen-gap drag
    /// is obtained from the Reynolds solve and integrated exponentially per step.
    /// Its exact mean speed drives extrusion; lost body kinetic energy heats body.
    /// No gravity, tangential shear, opening/cavitation or body rotation is applied.
    pub fn advance_squeeze_body(
        &mut self,
        dt: f64,
        normal: [f64; 3],
        body: &mut ThermalTranslatingBody,
        control: SqueezePressureControl,
        max_substep: f64,
    ) -> Result<SqueezeBodyReport, &'static str> {
        self.advance_squeeze_body_components(dt, normal, body, control, max_substep, None, None)
    }
    pub(super) fn advance_squeeze_body_components(
        &mut self,
        dt: f64,
        normal: [f64; 3],
        body: &mut ThermalTranslatingBody,
        control: SqueezePressureControl,
        max_substep: f64,
        components: Option<&mut Vec<Vec<f64>>>,
        viscosities: Option<&[f64]>,
    ) -> Result<SqueezeBodyReport, &'static str> {
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 0.1
            || !max_substep.is_finite()
            || max_substep <= 0.0
            || max_substep > 0.001
            || normal.iter().any(|v| !v.is_finite())
            || (dot(normal, normal) - 1.0).abs() > 1e-12
        {
            return Err("invalid squeeze body controls");
        }
        let normal = normal.map(|v| v / dot(normal, normal).sqrt());
        let count = (dt / max_substep).ceil() as usize;
        if count > 100000 {
            return Err("squeeze body substep budget");
        }
        body.thermal_energy()
            .map_err(|_| "invalid squeeze body thermal state")?;
        let step = dt / count as f64;
        let original = self.volume.clone();
        let mut inventory = components.as_deref().cloned();
        let mut candidate = *body;
        let mut report = SqueezeBodyReport {
            transport: SqueezeTransportReport {
                vented_volume: 0.0,
                vented_mass: 0.0,
                vented_component_masses: inventory.as_ref().map(|rows| vec![0.0; rows[0].len()]),
                normal_impulse: 0.0,
                dissipated_energy: 0.0,
                substeps: count,
            },
            substrate_impulse: [0.0; 3],
        };
        let result = (|| -> Result<(), &'static str> {
            for _ in 0..count {
                if candidate
                    .mechanics
                    .position
                    .iter()
                    .chain(&candidate.mechanics.velocity)
                    .any(|v| !v.is_finite())
                {
                    return Err("invalid squeeze body pose");
                }
                let speed = -dot(candidate.mechanics.velocity, normal);
                let tangent = sub(candidate.mechanics.velocity, normal.map(|n| -speed * n));
                if speed < 0.0 || dot(tangent, tangent) > 1e-24 * speed * speed {
                    return Err("squeeze body requires normal closing motion");
                }
                let gaps = self.thickness();
                for (i, h) in gaps.iter().enumerate() {
                    let gap = dot(sub(candidate.mechanics.position, self.center[i]), normal);
                    if dot(self.normals[i], normal).abs() < 1.0 - 1e-12
                        || gap <= 0.0
                        || (gap - h).abs() > 1e-9 * h
                    {
                        return Err("squeeze plane and filled film gaps differ");
                    }
                }
                let field = match (&inventory, viscosities) {
                    (Some(rows), Some(values)) => Some(super::mixture::blend_viscosities(
                        rows,
                        values,
                        self.material.viscosity,
                    )),
                    _ => None,
                };
                let drag = self
                    .solve_squeeze_pressure(
                        &gaps,
                        &vec![1.0; gaps.len()],
                        field.as_deref(),
                        control,
                    )?
                    .normal_load;
                let exponent = drag / candidate.mechanics.mass * step;
                if !exponent.is_finite() || exponent <= 0.0 {
                    return Err("invalid squeeze body drag");
                }
                let loss = -(-exponent).exp_m1();
                let mean = speed * loss / exponent;
                let heat = 0.5 * candidate.mechanics.mass * speed * speed * loss * (2.0 - loss);
                let temperature = candidate.temperature
                    + heat / (candidate.mechanics.mass * candidate.specific_heat);
                if !heat.is_finite()
                    || !temperature.is_finite()
                    || (heat > 0.0 && temperature == candidate.temperature)
                {
                    return Err("unrepresentable squeeze body heat");
                }
                let flux = self.step_squeeze_components(
                    step,
                    &vec![mean; gaps.len()],
                    control,
                    step,
                    inventory.as_mut(),
                    viscosities,
                )?;
                let impulse = candidate.mechanics.mass * speed * loss;
                if (flux.normal_impulse - impulse).abs() > 1e-8 * impulse.max(f64::MIN_POSITIVE) {
                    return Err("squeeze body pressure impulse mismatch");
                }
                for k in 0..3 {
                    candidate.mechanics.position[k] -= step * mean * normal[k];
                    candidate.mechanics.velocity[k] += speed * loss * normal[k];
                    report.substrate_impulse[k] -= impulse * normal[k];
                }
                candidate.temperature = temperature;
                candidate
                    .thermal_energy()
                    .map_err(|_| "squeeze body thermal overflow")?;
                report.transport.vented_volume += flux.vented_volume;
                report.transport.vented_mass += flux.vented_mass;
                if let (Some(total), Some(vented)) = (
                    &mut report.transport.vented_component_masses,
                    flux.vented_component_masses,
                ) {
                    for (a, b) in total.iter_mut().zip(vented) {
                        *a += b;
                    }
                }
                report.transport.normal_impulse += impulse;
                // Actual exponential-speed pressure work, rather than the lower
                // constant-mean-speed surrogate used by the volume flux query.
                report.transport.dissipated_energy += heat;
                if candidate
                    .mechanics
                    .position
                    .iter()
                    .chain(&report.substrate_impulse)
                    .any(|v| !v.is_finite())
                    || [
                        report.transport.vented_volume,
                        report.transport.vented_mass,
                        report.transport.normal_impulse,
                        report.transport.dissipated_energy,
                    ]
                    .iter()
                    .any(|v| !v.is_finite())
                    || report
                        .transport
                        .vented_component_masses
                        .as_ref()
                        .is_some_and(|row| row.iter().any(|v| !v.is_finite()))
                {
                    return Err("squeeze body ledger overflow");
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.volume = original;
            return Err(error);
        }
        if let (Some(target), Some(next)) = (components, inventory) {
            *target = next;
        }
        *body = candidate;
        Ok(report)
    }
}
