//! Vented, fully filled, stationary-gap Newtonian Reynolds pressure query.
use super::{SurfaceFilm, norm, sub};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug)]
pub struct SqueezePressureControl {
    pub relative_tolerance: f64,
    pub max_iterations: usize,
}
impl Default for SqueezePressureControl {
    fn default() -> Self {
        Self {
            relative_tolerance: 1e-10,
            max_iterations: 4096,
        }
    }
}
#[derive(Clone, Debug)]
pub struct SqueezePressureReport {
    /// Gauge pressure in Pa. Every open mesh boundary is vented to zero gauge.
    pub pressure: Vec<f64>,
    /// Integral of pressure over cell areas, N; caller supplies force direction.
    pub normal_load: f64,
    /// Outward volume flow through all vents, m³/s.
    pub vented_volume_rate: f64,
    /// Pressure-driven viscous dissipation, W, including boundary half-cells.
    pub dissipated_power: f64,
    pub iterations: usize,
    pub relative_residual: f64,
    /// Oriented internal volume flux (a -> b positive), m³/s.
    pub internal_volume_flux: Vec<(usize, usize, f64)>,
    /// Outward vent flux per cell, m³/s.
    pub vent_volume_flux: Vec<f64>,
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn magnitude(v: &[f64]) -> f64 {
    let scale = v.iter().fold(0.0_f64, |s, x| s.max(x.abs()));
    if scale == 0.0 {
        0.0
    } else {
        scale * v.iter().map(|x| (x / scale).powi(2)).sum::<f64>().sqrt()
    }
}
impl SurfaceFilm {
    /// Solves -div(h³/(12 mu) grad(p)) = closing_speed on filled film cells.
    /// Gap and normal closing speed are prescribed per cell, held fixed. Every
    /// exposed mesh edge is an ambient-pressure vent; every connected component
    /// must have a vent. Closing speeds must be nonnegative. No volume or body
    /// motion is applied. Couette transport, cavitation and elastic load are absent.
    /// The two-point normal-distance flux requires orthogonal duals for consistency;
    /// skew-mesh cross-diffusion reconstruction is not included.
    pub fn solve_squeeze_pressure(
        &self,
        gaps: &[f64],
        closing_speed: &[f64],
        viscosities: Option<&[f64]>,
        control: SqueezePressureControl,
    ) -> Result<SqueezePressureReport, &'static str> {
        let n = self.volume.len();
        if gaps.len() != n
            || closing_speed.len() != n
            || gaps.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || closing_speed.iter().any(|v| !v.is_finite() || *v < 0.0)
            || viscosities
                .is_some_and(|v| v.len() != n || v.iter().any(|x| !x.is_finite() || *x <= 0.0))
            || !control.relative_tolerance.is_finite()
            || control.relative_tolerance <= 0.0
            || control.relative_tolerance > 0.1
            || control.max_iterations == 0
            || control.max_iterations > 100000
        {
            return Err("invalid squeeze pressure controls");
        }
        let mut mobility = Vec::with_capacity(n);
        let mut rhs = Vec::with_capacity(n);
        for i in 0..n {
            if self.volume[i] / self.area[i] < gaps[i] * (1.0 - 1e-12) {
                return Err("squeeze gap is not fully filled");
            }
            let mu = viscosities.map_or(self.material.viscosity, |v| v[i]);
            let m = gaps[i].powi(3) / (12.0 * mu);
            let b = self.area[i] * closing_speed[i];
            if !m.is_finite() || m <= 0.0 || !b.is_finite() {
                return Err("squeeze mobility/source overflow");
            }
            mobility.push(m);
            rhs.push(b);
        }
        let mut edges = Vec::with_capacity(self.edges.len());
        let mut adjacent = vec![Vec::new(); n];
        let mut diagonal = vec![0.0; n];
        for &(a, b, length, _) in &self.edges {
            let resistance = 2.0 * self.area[a] / (3.0 * length * mobility[a])
                + 2.0 * self.area[b] / (3.0 * length * mobility[b]);
            let coefficient = length / resistance;
            if !coefficient.is_finite() || coefficient <= 0.0 {
                return Err("squeeze edge conductance overflow");
            }
            edges.push((a, b, coefficient));
            diagonal[a] += coefficient;
            diagonal[b] += coefficient;
            adjacent[a].push(b);
            adjacent[b].push(a);
        }
        let mut owners = BTreeMap::<(usize, usize), Vec<(usize, f64)>>::new();
        for (cell, t) in self.triangles.iter().enumerate() {
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                let key = (t[i].min(t[j]), t[i].max(t[j]));
                owners.entry(key).or_default().push((
                    cell,
                    norm(sub(self.geometry[cell][i], self.geometry[cell][j])),
                ));
            }
        }
        let mut boundary = vec![0.0; n];
        for cells in owners.values() {
            if let [(cell, length)] = cells.as_slice() {
                let distance = 2.0 * self.area[*cell] / (3.0 * length);
                let coefficient = mobility[*cell] * length / distance;
                if !coefficient.is_finite() || coefficient <= 0.0 {
                    return Err("squeeze vent conductance overflow");
                }
                boundary[*cell] += coefficient;
                diagonal[*cell] += coefficient;
            }
        }
        if diagonal.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err("invalid squeeze pressure diagonal");
        }
        let mut seen = vec![false; n];
        for seed in 0..n {
            if seen[seed] {
                continue;
            }
            let mut stack = vec![seed];
            seen[seed] = true;
            let mut vented = false;
            while let Some(i) = stack.pop() {
                vented |= boundary[i] > 0.0;
                for &j in &adjacent[i] {
                    if !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
            if !vented {
                return Err("sealed squeeze component has no pressure reference");
            }
        }
        let apply = |x: &[f64]| {
            let mut result: Vec<_> = x.iter().zip(&boundary).map(|(x, b)| x * b).collect();
            for &(a, b, c) in &edges {
                let f = c * (x[a] - x[b]);
                result[a] += f;
                result[b] -= f;
            }
            result
        };
        let source_norm = magnitude(&rhs);
        if !source_norm.is_finite() {
            return Err("squeeze source norm overflow");
        }
        let mut pressure = vec![0.0; n];
        let mut residual = rhs.clone();
        let mut z: Vec<_> = residual.iter().zip(&diagonal).map(|(r, d)| r / d).collect();
        let mut direction = z.clone();
        let mut rz = dot(&residual, &z);
        let mut iterations = 0;
        if source_norm > 0.0 {
            for _ in 0..control.max_iterations {
                let product = apply(&direction);
                let denominator = dot(&direction, &product);
                if !rz.is_finite() || rz <= 0.0 || !denominator.is_finite() || denominator <= 0.0 {
                    return Err("squeeze pressure CG breakdown");
                }
                let alpha = rz / denominator;
                for i in 0..n {
                    pressure[i] += alpha * direction[i];
                    residual[i] -= alpha * product[i];
                }
                iterations += 1;
                if pressure.iter().chain(&residual).any(|v| !v.is_finite()) {
                    return Err("squeeze pressure iterate overflow");
                }
                let mut restart = false;
                if magnitude(&residual) / source_norm <= control.relative_tolerance {
                    residual = rhs
                        .iter()
                        .zip(apply(&pressure))
                        .map(|(b, a)| b - a)
                        .collect();
                    if magnitude(&residual) / source_norm <= control.relative_tolerance {
                        break;
                    }
                    restart = true;
                }
                z = residual.iter().zip(&diagonal).map(|(r, d)| r / d).collect();
                let next_rz = dot(&residual, &z);
                let beta = if restart { 0.0 } else { next_rz / rz };
                for i in 0..n {
                    direction[i] = z[i] + beta * direction[i];
                }
                rz = next_rz;
            }
        }
        let residual: Vec<_> = rhs
            .iter()
            .zip(apply(&pressure))
            .map(|(b, a)| b - a)
            .collect();
        let relative_residual = if source_norm == 0.0 {
            0.0
        } else {
            magnitude(&residual) / source_norm
        };
        if !relative_residual.is_finite() || relative_residual > control.relative_tolerance {
            return Err("squeeze pressure did not converge");
        }
        let normal_load = dot(&pressure, &self.area);
        let vented_volume_rate = dot(&pressure, &boundary);
        let dissipated_power = pressure
            .iter()
            .zip(&boundary)
            .map(|(p, b)| p * p * b)
            .sum::<f64>()
            + edges
                .iter()
                .map(|&(a, b, c)| c * (pressure[a] - pressure[b]).powi(2))
                .sum::<f64>();
        if [normal_load, vented_volume_rate, dissipated_power]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err("squeeze pressure ledger overflow");
        }
        let internal_volume_flux = edges
            .iter()
            .map(|&(a, b, c)| (a, b, c * (pressure[a] - pressure[b])))
            .collect();
        let vent_volume_flux = pressure.iter().zip(&boundary).map(|(p, b)| p * b).collect();
        Ok(SqueezePressureReport {
            internal_volume_flux,
            vent_volume_flux,
            pressure,
            normal_load,
            vented_volume_rate,
            dissipated_power,
            iterations,
            relative_residual,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SqueezeTransportReport {
    pub vented_volume: f64,
    pub vented_mass: f64,
    /// Present for a composed film, in the configured component order, kg.
    pub vented_component_masses: Option<Vec<f64>>,
    /// Scalar time-integrated pressure load, N s; no body force is applied here.
    pub normal_impulse: f64,
    pub dissipated_energy: f64,
    pub substeps: usize,
}
impl SurfaceFilm {
    /// Prescribed normal squeezing of a fully filled vented film. Current film
    /// heights define the gap on every interval. Pressure fluxes transport volume
    /// internally and expel it through vents; pressure is recomputed as gaps shrink.
    /// Outgoing donor volume must fit within one interval (otherwise refine dt).
    /// No donor limiter is applied because it would invalidate the Reynolds balance.
    /// Vented liquid and dissipated energy belong to the caller's external ledger.
    pub fn step_squeeze(
        &mut self,
        dt: f64,
        closing_speed: &[f64],
        control: SqueezePressureControl,
        max_substep: f64,
    ) -> Result<SqueezeTransportReport, &'static str> {
        self.step_squeeze_components(dt, closing_speed, control, max_substep, None, None)
    }
    pub(super) fn step_squeeze_components(
        &mut self,
        dt: f64,
        closing_speed: &[f64],
        control: SqueezePressureControl,
        max_substep: f64,
        components: Option<&mut Vec<Vec<f64>>>,
        viscosities: Option<&[f64]>,
    ) -> Result<SqueezeTransportReport, &'static str> {
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 0.1
            || !max_substep.is_finite()
            || max_substep <= 0.0
            || max_substep > 0.001
            || closing_speed.len() != self.volume.len()
            || closing_speed.iter().any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err("invalid squeeze transport controls");
        }
        let count = (dt / max_substep).ceil() as usize;
        if count > 100000 {
            return Err("squeeze transport substep budget");
        }
        let step = dt / count as f64;
        let original = self.volume.clone();
        let mut inventory = components.as_deref().cloned();
        let mut report = SqueezeTransportReport {
            vented_volume: 0.0,
            vented_mass: 0.0,
            vented_component_masses: inventory.as_ref().map(|rows| vec![0.0; rows[0].len()]),
            normal_impulse: 0.0,
            dissipated_energy: 0.0,
            substeps: count,
        };
        let result = (|| -> Result<(), &'static str> {
            for _ in 0..count {
                let gaps = self.thickness();
                if gaps.iter().zip(closing_speed).any(|(h, w)| step * w >= *h) {
                    return Err("squeeze interval closes the gap");
                }
                let field = match (&inventory, viscosities) {
                    (Some(rows), Some(values)) => Some(super::mixture::blend_viscosities(
                        rows,
                        values,
                        self.material.viscosity,
                    )),
                    _ => None,
                };
                let pressure =
                    self.solve_squeeze_pressure(&gaps, closing_speed, field.as_deref(), control)?;
                let mut outgoing = pressure
                    .vent_volume_flux
                    .iter()
                    .map(|q| step * q)
                    .collect::<Vec<_>>();
                if outgoing.iter().any(|v| !v.is_finite() || *v < 0.0) {
                    return Err("squeeze vent inflow/overflow");
                }
                for &(a, b, q) in &pressure.internal_volume_flux {
                    outgoing[if q >= 0.0 { a } else { b }] += step * q.abs();
                }
                if outgoing
                    .iter()
                    .zip(&self.volume)
                    .any(|(q, v)| !q.is_finite() || *q > *v)
                {
                    return Err("squeeze outgoing CFL limit exceeded");
                }
                let mut delta = vec![0.0; self.volume.len()];
                let mut component_delta = inventory.as_ref().map(|rows| {
                    rows.iter()
                        .map(|row| vec![0.0; row.len()])
                        .collect::<Vec<_>>()
                });
                for &(a, b, q) in &pressure.internal_volume_flux {
                    let moved = step * q;
                    let donor = if moved >= 0.0 { a } else { b };
                    delta[a] -= moved;
                    delta[b] += moved;
                    if let (Some(rows), Some(change)) = (&inventory, &mut component_delta) {
                        for k in 0..rows[donor].len() {
                            let carried = (moved / self.volume[donor]) * rows[donor][k];
                            change[a][k] -= carried;
                            change[b][k] += carried;
                        }
                    }
                }
                for (i, q) in pressure.vent_volume_flux.iter().enumerate() {
                    let moved = step * q;
                    delta[i] -= moved;
                    report.vented_volume += moved;
                    if let (Some(rows), Some(change), Some(vented)) = (
                        &inventory,
                        &mut component_delta,
                        &mut report.vented_component_masses,
                    ) {
                        for k in 0..rows[i].len() {
                            let carried = (moved / self.volume[i]) * rows[i][k];
                            change[i][k] -= carried;
                            vented[k] += carried;
                        }
                    }
                }
                if let (Some(rows), Some(change)) = (&mut inventory, component_delta) {
                    for (row, change) in rows.iter_mut().zip(change) {
                        for (value, d) in row.iter_mut().zip(change) {
                            *value = (*value + d).max(0.0);
                            if !value.is_finite() {
                                return Err("squeeze component overflow");
                            }
                        }
                    }
                }
                for (v, d) in self.volume.iter_mut().zip(delta) {
                    *v = (*v + d).max(0.0);
                    if !v.is_finite() {
                        return Err("squeeze volume overflow");
                    }
                }
                report.normal_impulse += step * pressure.normal_load;
                report.dissipated_energy += step * pressure.dissipated_power;
                report.vented_mass = report
                    .vented_component_masses
                    .as_ref()
                    .map_or(report.vented_volume * self.material.density, |masses| {
                        masses.iter().sum()
                    });
                if [
                    report.vented_volume,
                    report.vented_mass,
                    report.normal_impulse,
                    report.dissipated_energy,
                ]
                .iter()
                .any(|v| !v.is_finite())
                    || report
                        .vented_component_masses
                        .as_ref()
                        .is_some_and(|row| row.iter().any(|v| !v.is_finite()))
                {
                    return Err("squeeze transport ledger overflow");
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
        Ok(report)
    }
}
