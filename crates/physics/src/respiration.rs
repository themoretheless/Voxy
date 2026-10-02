//! Resistive airway network with compliant terminal lung units, in SI units.
//! Incompressible volume flow, negligible airway storage/inertia; prescribed pleural pressure.
//! No patient defaults, tissue geometry, gas exchange or recruitment model.
#[derive(Clone, Copy, Debug)]
pub enum Recoil {
    /// V = V0 + C (Ptp - P0); C is m³/Pa. Steps with V <= 0 are rejected.
    Linear { compliance_m3_per_pa: f64 },
    /// Ptp = P0 + a ln(V/V0); a is Pa, giving positive volume and nonlinear compliance.
    Logarithmic { scale_pa: f64 },
}
#[derive(Clone, Copy, Debug)]
pub struct LungUnit {
    pub reference_volume_m3: f64,
    pub reference_transpulmonary_pa: f64,
    pub recoil: Recoil,
}
impl LungUnit {
    fn volume(self, transpulmonary_pa: f64) -> Result<(f64, f64), &'static str> {
        let pressure = transpulmonary_pa - self.reference_transpulmonary_pa;
        let (volume, compliance) = match self.recoil {
            Recoil::Linear {
                compliance_m3_per_pa,
            } => (
                self.reference_volume_m3 + compliance_m3_per_pa * pressure,
                compliance_m3_per_pa,
            ),
            Recoil::Logarithmic { scale_pa } => {
                let v = self.reference_volume_m3 * (pressure / scale_pa).exp();
                (v, v / scale_pa)
            }
        };
        if !volume.is_finite() || volume <= 0. || !compliance.is_finite() || compliance <= 0. {
            return Err("invalid terminal volume or compliance");
        }
        Ok((volume, compliance))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Airway {
    pub from: usize,
    pub to: usize,
    /// Pa s / m³. Positive flow is from -> to.
    pub resistance_pa_s_per_m3: f64,
}
impl Airway {
    /// Laminar rigid circular-pipe resistance 8 mu L / (pi r^4).
    /// # Errors
    /// Rejects invalid geometry, viscosity and unrepresentable resistance.
    pub fn poiseuille(
        from: usize,
        to: usize,
        length_m: f64,
        radius_m: f64,
        viscosity_pa_s: f64,
    ) -> Result<Self, &'static str> {
        if [length_m, radius_m, viscosity_pa_s]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid airway pipe");
        }
        let resistance = 8. * viscosity_pa_s * length_m / (std::f64::consts::PI * radius_m.powi(4));
        if !resistance.is_finite() || resistance <= 0. || !resistance.recip().is_finite() {
            return Err("airway resistance overflow");
        }
        Ok(Self {
            from,
            to,
            resistance_pa_s_per_m3: resistance,
        })
    }
}
#[derive(Clone, Debug)]
pub struct RespiratoryNetwork {
    units: Vec<Option<LungUnit>>,
    airways: Vec<Airway>,
    pressures: Vec<f64>,
    volumes: Vec<f64>,
    pleural: Vec<f64>,
    flows: Vec<f64>,
}
#[derive(Clone, Copy, Debug)]
pub struct VentilationStep {
    pub iterations: usize,
    /// Sum of terminal volume changes in this step, m³.
    pub volume_change_m3: f64,
    /// Volume delivered through the mouth (node zero), m³.
    pub mouth_volume_m3: f64,
    /// Maximum nodal volume-balance residual, m³.
    pub residual_m3: f64,
    /// Sum R Q² dt, joules; nonnegative resistive dissipation.
    pub airway_loss_j: f64,
}
struct Assembly {
    residual: Vec<f64>,
    jacobian: Vec<f64>,
    volumes: Vec<f64>,
}
impl RespiratoryNetwork {
    /// Node zero is the prescribed mouth pressure. Other nodes are storage-free
    /// junctions (None) or compliant terminal units (Some). Pressure/pleural arrays
    /// initialize volume through the selected recoil law.
    /// # Errors
    /// Rejects invalid topology, disconnected nodes, invalid units and geometry.
    pub fn new(
        units: Vec<Option<LungUnit>>,
        airways: Vec<Airway>,
        pressures: Vec<f64>,
        pleural: Vec<f64>,
    ) -> Result<Self, &'static str> {
        let count = units.len();
        if !(2..=256).contains(&count)
            || units[0].is_some()
            || pressures.len() != count
            || pleural.len() != count
            || pressures.iter().chain(&pleural).any(|v| !v.is_finite())
            || airways.len() > 4096
            || airways.is_empty()
        {
            return Err("invalid respiratory network");
        }
        let mut volumes = vec![0.; count];
        let mut terminal_count = 0;
        for (i, unit) in units.iter().enumerate() {
            if let Some(u) = unit {
                terminal_count += 1;
                if !u.reference_volume_m3.is_finite()
                    || u.reference_volume_m3 <= 0.
                    || !u.reference_transpulmonary_pa.is_finite()
                {
                    return Err("invalid reference lung unit");
                }
                match u.recoil {
                    Recoil::Linear {
                        compliance_m3_per_pa,
                    } if !compliance_m3_per_pa.is_finite() || compliance_m3_per_pa <= 0. => {
                        return Err("invalid linear compliance");
                    }
                    Recoil::Logarithmic { scale_pa } if !scale_pa.is_finite() || scale_pa <= 0. => {
                        return Err("invalid recoil scale");
                    }
                    _ => {}
                }
                volumes[i] = u.volume(pressures[i] - pleural[i])?.0;
            }
        }
        if terminal_count == 0 {
            return Err("network has no compliant lung units");
        }
        for e in &airways {
            if e.from >= count
                || e.to >= count
                || e.from == e.to
                || !e.resistance_pa_s_per_m3.is_finite()
                || e.resistance_pa_s_per_m3 <= 0.
                || !e.resistance_pa_s_per_m3.recip().is_finite()
            {
                return Err("invalid airway edge");
            }
        }
        let mut reached = vec![false; count];
        reached[0] = true;
        for _ in 0..count {
            for e in &airways {
                if reached[e.from] || reached[e.to] {
                    reached[e.from] = true;
                    reached[e.to] = true;
                }
            }
        }
        if reached.iter().any(|v| !*v) {
            return Err("airway node disconnected from mouth");
        }
        Ok(Self {
            flows: vec![0.; airways.len()],
            units,
            airways,
            pressures,
            volumes,
            pleural,
        })
    }
    #[must_use]
    pub fn volumes(&self) -> &[f64] {
        &self.volumes
    }
    #[must_use]
    pub fn pressures(&self) -> &[f64] {
        &self.pressures
    }
    #[must_use]
    pub fn flows(&self) -> &[f64] {
        &self.flows
    }
    /// Gauge alveolar minus pleural pressure, suitable as an organ-coupling input.
    #[must_use]
    pub fn transpulmonary_pressures(&self) -> Vec<f64> {
        self.pressures
            .iter()
            .zip(&self.pleural)
            .map(|(a, b)| a - b)
            .collect()
    }
    fn residual(
        &self,
        pressure: &[f64],
        pleural: &[f64],
        dt: f64,
    ) -> Result<Assembly, &'static str> {
        let n = self.units.len() - 1;
        let mut residual = vec![0.; n];
        let mut jacobian = vec![0.; n * n];
        let mut volumes = vec![0.; n + 1];
        for i in 1..=n {
            if let Some(unit) = self.units[i] {
                let (v, c) = unit.volume(pressure[i] - pleural[i])?;
                volumes[i] = v;
                residual[i - 1] = v - self.volumes[i];
                jacobian[(i - 1) * n + i - 1] = c;
            }
        }
        for e in &self.airways {
            let conductance = dt / e.resistance_pa_s_per_m3;
            let transferred = conductance * (pressure[e.from] - pressure[e.to]);
            for (i, k, sign) in [(e.from, e.to, 1.), (e.to, e.from, -1.)] {
                if i != 0 {
                    residual[i - 1] += sign * transferred;
                    jacobian[(i - 1) * n + i - 1] += conductance;
                    if k != 0 {
                        jacobian[(i - 1) * n + k - 1] -= conductance;
                    }
                }
            }
        }
        if residual.iter().chain(&jacobian).any(|v| !v.is_finite()) {
            return Err("airway assembly overflow");
        }
        Ok(Assembly {
            residual,
            jacobian,
            volumes,
        })
    }
    /// Backward Euler conservation solve; new mouth and pleural pressures are
    /// prescribed for this step. No state is committed on failure.
    /// # Errors
    /// Rejects invalid options, inadmissible volumes, factorization failure and nonconvergence.
    pub fn step(
        &mut self,
        dt: f64,
        mouth_pressure_pa: f64,
        pleural_pa: &[f64],
        max_iterations: usize,
        tolerance_m3: f64,
    ) -> Result<VentilationStep, &'static str> {
        if !dt.is_finite()
            || dt <= 0.
            || !mouth_pressure_pa.is_finite()
            || pleural_pa.len() != self.units.len()
            || pleural_pa.iter().any(|v| !v.is_finite())
            || !(1..=128).contains(&max_iterations)
            || !tolerance_m3.is_finite()
            || tolerance_m3 <= 0.
        {
            return Err("invalid ventilation step");
        }
        let mut pressure = self.pressures.clone();
        pressure[0] = mouth_pressure_pa;
        // Preserve each unit's initial volume under a sudden pleural shift; flow
        // then changes volume during the solve. Also avoids inadmissible first guesses.
        for i in 1..pressure.len() {
            if self.units[i].is_some() {
                pressure[i] += pleural_pa[i] - self.pleural[i];
            }
        }
        let mut iterations = 0;
        loop {
            let Assembly {
                residual, jacobian, ..
            } = self.residual(&pressure, pleural_pa, dt)?;
            let norm = residual.iter().fold(0_f64, |a, b| a.max(b.abs()));
            if norm <= tolerance_m3 {
                break;
            }
            if iterations >= max_iterations {
                return Err("ventilation solve did not converge");
            }
            let direction = cholesky_solve(jacobian, residual.iter().map(|v| -v).collect())?;
            let mut fraction = 1.;
            let mut accepted = false;
            for _ in 0..40 {
                let mut trial = pressure.clone();
                for i in 1..trial.len() {
                    trial[i] += fraction * direction[i - 1];
                }
                if let Ok(Assembly { residual: r, .. }) = self.residual(&trial, pleural_pa, dt) {
                    let new_norm = r.iter().fold(0_f64, |a, b| a.max(b.abs()));
                    if new_norm <= norm * (1. - 1e-4 * fraction) || new_norm <= tolerance_m3 {
                        pressure = trial;
                        accepted = true;
                        break;
                    }
                }
                fraction *= 0.5;
            }
            if !accepted {
                return Err("ventilation line search failed");
            }
            iterations += 1;
        }
        let Assembly {
            residual, volumes, ..
        } = self.residual(&pressure, pleural_pa, dt)?;
        let flows: Vec<_> = self
            .airways
            .iter()
            .map(|e| (pressure[e.from] - pressure[e.to]) / e.resistance_pa_s_per_m3)
            .collect();
        let mouth_volume_m3 = dt
            * self
                .airways
                .iter()
                .zip(&flows)
                .map(|(e, q)| {
                    if e.from == 0 {
                        *q
                    } else if e.to == 0 {
                        -q
                    } else {
                        0.
                    }
                })
                .sum::<f64>();
        let volume_change_m3: f64 = volumes.iter().zip(&self.volumes).map(|(a, b)| a - b).sum();
        let airway_loss_j = dt
            * self
                .airways
                .iter()
                .zip(&flows)
                .map(|(e, q)| e.resistance_pa_s_per_m3 * q * q)
                .sum::<f64>();
        if !mouth_volume_m3.is_finite()
            || !airway_loss_j.is_finite()
            || !volume_change_m3.is_finite()
            || flows.iter().any(|v| !v.is_finite())
        {
            return Err("ventilation result overflow");
        }
        let report = VentilationStep {
            iterations,
            volume_change_m3,
            mouth_volume_m3,
            airway_loss_j,
            residual_m3: residual.iter().fold(0_f64, |a, b| a.max(b.abs())),
        };
        self.pressures = pressure;
        self.volumes = volumes;
        self.pleural = pleural_pa.to_vec();
        self.flows = flows;
        Ok(report)
    }
}
fn cholesky_solve(mut a: Vec<f64>, mut rhs: Vec<f64>) -> Result<Vec<f64>, &'static str> {
    let n = rhs.len();
    for i in 0..n {
        for j in 0..=i {
            let mut value = a[i * n + j];
            for k in 0..j {
                value -= a[i * n + k] * a[j * n + k];
            }
            if i == j {
                if !value.is_finite() || value <= 0. {
                    return Err("singular airway Jacobian");
                }
                a[i * n + j] = value.sqrt();
            } else {
                a[i * n + j] = value / a[j * n + j];
            }
        }
    }
    for i in 0..n {
        for k in 0..i {
            rhs[i] -= a[i * n + k] * rhs[k];
        }
        rhs[i] /= a[i * n + i];
    }
    for i in (0..n).rev() {
        for k in i + 1..n {
            rhs[i] -= a[k * n + i] * rhs[k];
        }
        rhs[i] /= a[i * n + i];
    }
    if rhs.iter().any(|v| !v.is_finite()) {
        return Err("airway solve overflow");
    }
    Ok(rhs)
}
