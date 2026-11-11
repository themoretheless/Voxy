//! Lymphatic vessel wall constitutive laws, series Poiseuille attachments, and Starling filtration.
use super::Exchange;

/// Fixed-length cylindrical lymphangion with a passive collapse/stiffening law.
/// Parameters and active circumferential tension are explicit, not fitted defaults.
#[derive(Clone, Copy, Debug)]
pub struct LymphaticWallLaw {
    pub reference_volume_m3: f64,
    pub length_m: f64,
    pub passive_pressure_scale_pa: f64,
    pub stiffening: f64,
    pub external_pressure_pa: f64,
    pub active_tension_n_per_m: f64,
}
impl LymphaticWallLaw {
    /// Radius from actual fluid volume at fixed length.
    pub fn radius_m(self, volume_m3: f64) -> Result<f64, &'static str> {
        if [
            self.reference_volume_m3,
            self.length_m,
            self.passive_pressure_scale_pa,
            self.stiffening,
            volume_m3,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
            || !self.external_pressure_pa.is_finite()
            || !self.active_tension_n_per_m.is_finite()
            || self.active_tension_n_per_m < 0.
        {
            return Err("invalid lymphatic wall state");
        }
        let radius = (volume_m3 / self.length_m / std::f64::consts::PI).sqrt();
        if !radius.is_finite() || radius <= 0. {
            return Err("lymphatic radius overflow");
        }
        Ok(radius)
    }
    /// P = Pext + scale*(exp(k*(r/r0-1))-(r0/r)^3) + Tactive/r.
    /// No clipping: collapse singularity or exponential overflow rejects the step.
    pub fn pressure_pa(self, volume_m3: f64) -> Result<f64, &'static str> {
        let radius = self.radius_m(volume_m3)?;
        let stretch = (volume_m3 / self.reference_volume_m3).sqrt();
        let passive = self.passive_pressure_scale_pa
            * ((self.stiffening * (stretch - 1.)).exp() - stretch.powi(-3));
        let p = self.external_pressure_pa + passive + self.active_tension_n_per_m / radius;
        if !p.is_finite() {
            return Err("lymphatic wall pressure overflow");
        }
        Ok(p)
    }
}
/// Poiseuille tube segment in series with an exchange edge's fixed resistance.
/// Radius follows an endpoint wall compartment; segment length is explicit.
#[derive(Clone, Copy, Debug)]
pub struct LymphaticHydraulicAttachment {
    pub edge: usize,
    pub compartment: usize,
    pub segment_length_m: f64,
    pub viscosity_pa_s: f64,
}
/// Circular Newtonian segment diagnostics; not a validity guarantee or fitted physiology.
#[derive(Clone, Copy, Debug)]
pub struct LymphaticTubeDiagnostics {
    pub radius_m: f64,
    pub mean_velocity_m_per_s: f64,
    pub reynolds: f64,
    pub womersley: f64,
    pub tube_resistance_pa_s_per_m3: f64,
    pub inertance_pa_s2_per_m3: f64,
    pub relaxation_seconds: f64,
    pub inertial_to_resistive_ratio: f64,
}
impl LymphaticHydraulicAttachment {
    /// Backward-Euler segment momentum response at a supplied trial geometry.
    /// Returns signed flow and its pressure tangent. Pressure drive must include
    /// osmosis and active/external wall pressure before calling this method.
    /// An ideal valve may retain forward flow against reversed pressure while
    /// inertia decays. This response alone does not advance compartment inventory.
    /// # Errors
    /// Rejects invalid coefficients, geometry, time step or previous valve flow.
    pub fn momentum_response(
        self,
        wall: LymphaticWallLaw,
        volume_m3: f64,
        density_kg_per_m3: f64,
        fixed_resistance_pa_s_per_m3: f64,
        pressure_drive_pa: f64,
        previous_flow_m3_per_s: f64,
        seconds: f64,
        valve: bool,
    ) -> Result<(f64, f64), &'static str> {
        if !fixed_resistance_pa_s_per_m3.is_finite() || fixed_resistance_pa_s_per_m3 < 0. {
            return Err("invalid lymphatic series resistance");
        }
        let mut pipe = crate::circulation::Vessel::rigid_pipe(
            0,
            1,
            self.segment_length_m,
            wall.radius_m(volume_m3)?,
            self.viscosity_pa_s,
            density_kg_per_m3,
            0.,
        )?;
        pipe.resistance += fixed_resistance_pa_s_per_m3;
        pipe.valve = valve;
        pipe.flow(pressure_drive_pa, previous_flow_m3_per_s, seconds)
    }

    /// Diagnose supplied signed flow and forcing period with explicit density.
    /// Uses the same rigid-pipe coefficients as the inertial circulation solver.
    /// # Errors
    /// Invalid geometry, density, period, fixed resistance or derived overflow.
    pub fn diagnostics(
        self,
        wall: LymphaticWallLaw,
        volume_m3: f64,
        density_kg_per_m3: f64,
        flow_m3_per_s: f64,
        fixed_resistance_pa_s_per_m3: f64,
        forcing_period_seconds: f64,
    ) -> Result<LymphaticTubeDiagnostics, &'static str> {
        if !flow_m3_per_s.is_finite()
            || !fixed_resistance_pa_s_per_m3.is_finite()
            || fixed_resistance_pa_s_per_m3 < 0.
            || !forcing_period_seconds.is_finite()
            || forcing_period_seconds <= 0.
        {
            return Err("invalid lymphatic diagnostic parameters");
        }
        let radius = wall.radius_m(volume_m3)?;
        let pipe = crate::circulation::Vessel::rigid_pipe(
            0,
            1,
            self.segment_length_m,
            radius,
            self.viscosity_pa_s,
            density_kg_per_m3,
            0.,
        )?;
        let area = std::f64::consts::PI * radius * radius;
        let velocity = flow_m3_per_s / area;
        let reynolds = 2. * density_kg_per_m3 * radius * velocity.abs() / self.viscosity_pa_s;
        let omega = 2. * std::f64::consts::PI / forcing_period_seconds;
        let womersley = radius * (omega * density_kg_per_m3 / self.viscosity_pa_s).sqrt();
        let relaxation = pipe.inertance / (pipe.resistance + fixed_resistance_pa_s_per_m3);
        let ratio = omega * relaxation;
        if [velocity, reynolds, womersley, relaxation, ratio]
            .iter()
            .any(|v| !v.is_finite())
            || !(pipe.resistance + fixed_resistance_pa_s_per_m3).is_finite()
        {
            return Err("lymphatic diagnostic overflow");
        }
        Ok(LymphaticTubeDiagnostics {
            radius_m: radius,
            mean_velocity_m_per_s: velocity,
            reynolds,
            womersley,
            tube_resistance_pa_s_per_m3: pipe.resistance,
            inertance_pa_s2_per_m3: pipe.inertance,
            relaxation_seconds: relaxation,
            inertial_to_resistive_ratio: ratio,
        })
    }
}

/// Adjust base constitutive rates once; base edge metadata is the fixed series element.
/// # Errors
/// Invalid mapping/state, nonpositive resistance or overflow leaves rates unchanged.
pub fn apply_wall_hydraulic_resistances(
    volumes: &[f64],
    proteins: &[f64],
    edges: &[Exchange],
    walls: &[Option<LymphaticWallLaw>],
    attachments: &[LymphaticHydraulicAttachment],
    rates: &mut [(f64, f64)],
) -> Result<(), &'static str> {
    if rates.len() != edges.len() || proteins.len() != volumes.len() || walls.len() != volumes.len()
    {
        return Err("invalid wall hydraulic dimensions");
    }
    if volumes.iter().any(|v| !v.is_finite() || *v <= 0.)
        || proteins.iter().any(|m| !m.is_finite() || *m < 0.)
        || rates.iter().any(|(q, j)| !q.is_finite() || !j.is_finite())
    {
        return Err("invalid wall hydraulic state");
    }
    let mut linked = std::collections::BTreeSet::new();
    let mut added = std::collections::BTreeMap::<usize, f64>::new();
    for a in attachments {
        let e = edges.get(a.edge).ok_or("invalid wall hydraulic edge")?;
        let wall = walls
            .get(a.compartment)
            .and_then(|w| *w)
            .ok_or("missing hydraulic wall")?;
        if (e.from != a.compartment && e.to != a.compartment)
            || !a.segment_length_m.is_finite()
            || a.segment_length_m <= 0.
            || !a.viscosity_pa_s.is_finite()
            || a.viscosity_pa_s <= 0.
            || !e.hydraulic_m3_per_pa_s.is_finite()
            || e.hydraulic_m3_per_pa_s <= 0.
            || !e.reflection.is_finite()
            || !(0. ..=1.).contains(&e.reflection)
            || !e.protein_permeability_m3_per_s.is_finite()
            || e.protein_permeability_m3_per_s < 0.
            || !linked.insert((a.edge, a.compartment))
        {
            return Err("invalid wall hydraulic attachment");
        }
        let radius = wall.radius_m(volumes[a.compartment])?;
        let resistance =
            8. * a.viscosity_pa_s * a.segment_length_m / (std::f64::consts::PI * radius.powi(4));
        if !resistance.is_finite() || resistance <= 0. {
            return Err("wall hydraulic resistance overflow");
        }
        *added.entry(a.edge).or_default() += resistance;
    }
    // Stage every result: an invalid later edge must not partially change rates.
    let mut replacements = Vec::new();
    for (index, resistance) in added {
        let e = &edges[index];
        if e.from >= volumes.len() || e.to >= volumes.len() {
            return Err("invalid wall hydraulic endpoint");
        }
        let base = 1. / e.hydraulic_m3_per_pa_s;
        let total = base + resistance;
        if !total.is_finite() || total <= 0. {
            return Err("wall hydraulic resistance overflow");
        }
        let (old_q, _) = rates[index];
        let q = old_q * (base / total);
        let c0 = proteins[e.from] / volumes[e.from];
        let c1 = proteins[e.to] / volumes[e.to];
        if !c0.is_finite() || !c1.is_finite() {
            return Err("wall hydraulic concentration overflow");
        }
        if e.valve && old_q < 0. {
            return Err("invalid reverse valve flux");
        }
        let j = if e.valve && old_q == 0. {
            0.
        } else {
            (1. - e.reflection) * q * if q >= 0. { c0 } else { c1 }
                + e.protein_permeability_m3_per_s * (c0 - c1)
        };
        if !q.is_finite() || !j.is_finite() {
            return Err("wall hydraulic flux overflow");
        }
        replacements.push((index, (q, j)));
    }
    for (index, rate) in replacements {
        rates[index] = rate;
    }
    Ok(())
}

pub(crate) fn apply_wall_pressures(
    volumes: &[f64],
    pressures: &mut [f64],
    walls: &[Option<LymphaticWallLaw>],
) -> Result<(), &'static str> {
    if walls.len() != volumes.len() || pressures.len() != volumes.len() {
        return Err("invalid lymphatic wall count");
    }
    for ((v, p), wall) in volumes.iter().zip(pressures).zip(walls) {
        if let Some(wall) = wall {
            *p = wall.pressure_pa(*v)?;
        }
    }
    Ok(())
}

/// Evaluates the classic Starling–Landis transcapillary fluid filtration flux
/// $J_v = K_f \left[(P_c - P_i) - \sigma (\pi_c - \pi_i)\right]$.
#[must_use]
pub fn starling_transcapillary_filtration_flux(
    p_cap: f64,
    p_int: f64,
    pi_cap: f64,
    pi_int: f64,
    kf: f64,
    sigma: f64,
) -> Option<f64> {
    if !p_cap.is_finite()
        || !p_int.is_finite()
        || !pi_cap.is_finite()
        || !pi_int.is_finite()
        || !kf.is_finite()
        || !sigma.is_finite()
        || kf < 0.0
        || !(0.0..=1.0).contains(&sigma)
        || pi_cap < 0.0
        || pi_int < 0.0
    {
        return None;
    }
    let jv = kf * ((p_cap - p_int) - sigma * (pi_cap - pi_int));
    jv.is_finite().then_some(jv)
}

#[cfg(test)]
mod starling_flux_tests {
    use super::*;

    #[test]
    fn starling_filtration_is_outward_at_arteriolar_end_and_inward_at_venular_end() {
        let j_art =
            starling_transcapillary_filtration_flux(35.0, -2.0, 28.0, 3.0, 0.01, 0.9).unwrap();
        let j_ven =
            starling_transcapillary_filtration_flux(15.0, -2.0, 28.0, 3.0, 0.01, 0.9).unwrap();
        assert!(j_art > 0.0);
        assert!(j_ven < 0.0);
    }
}
