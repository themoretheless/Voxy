//! Closed-loop incompressible blood-volume circuit in SI units.
//! Time-varying chamber elastance, resistive/inertial flow and ideal one-way valves.
//! No patient calibration, 3D wall coupling, blood cells or oxygen transport.
#[derive(Clone, Copy, Debug)]
pub struct Compartment {
    pub unstressed_volume_m3: f64,
    pub initial_volume_m3: f64,
    pub initial_elastance_pa_per_m3: f64,
    pub initial_external_pressure_pa: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Vessel {
    pub from: usize,
    pub to: usize,
    /// Linear resistance, Pa s/m³.
    pub resistance: f64,
    /// Quadratic pressure-loss coefficient, Pa s²/m⁶.
    pub quadratic_resistance: f64,
    /// Flow inertance, Pa s²/m³.
    pub inertance: f64,
    /// An ideal check valve: forward flow only; no leaflet state/leakage.
    pub valve: bool,
}
impl Vessel {
    /// Rigid circular vessel with laminar Newtonian resistance, plug-flow inertance
    /// and optional local loss coefficient. Density and viscosity must be supplied;
    /// no assumed patient blood properties or shear-dependent rheology.
    /// # Errors
    /// Rejects nonpositive geometry/fluid properties and coefficient overflow.
    pub fn rigid_pipe(
        from: usize,
        to: usize,
        length_m: f64,
        radius_m: f64,
        viscosity_pa_s: f64,
        density_kg_per_m3: f64,
        local_loss: f64,
    ) -> Result<Self, &'static str> {
        if [length_m, radius_m, viscosity_pa_s, density_kg_per_m3]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
            || !local_loss.is_finite()
            || local_loss < 0.
        {
            return Err("invalid vascular pipe parameters");
        }
        let area = std::f64::consts::PI * radius_m * radius_m;
        let resistance = 8. * viscosity_pa_s * length_m / (area * radius_m * radius_m);
        let inertance = density_kg_per_m3 * length_m / area;
        let quadratic_resistance = 0.5 * density_kg_per_m3 * local_loss / (area * area);
        if !resistance.is_finite()
            || resistance <= 0.
            || !resistance.recip().is_finite()
            || !inertance.is_finite()
            || inertance <= 0.
            || !quadratic_resistance.is_finite()
        {
            return Err("vascular pipe coefficient overflow");
        }
        Ok(Self {
            from,
            to,
            resistance,
            quadratic_resistance,
            inertance,
            valve: false,
        })
    }
    pub(crate) fn flow(
        self,
        pressure_difference: f64,
        old_flow: f64,
        dt: f64,
    ) -> Result<(f64, f64), &'static str> {
        if !dt.is_finite()
            || dt <= 0.
            || !pressure_difference.is_finite()
            || !old_flow.is_finite()
            || !self.resistance.is_finite()
            || self.resistance <= 0.
            || !self.inertance.is_finite()
            || self.inertance < 0.
            || !self.quadratic_resistance.is_finite()
            || self.quadratic_resistance < 0.
            || (self.valve && old_flow < 0.)
        {
            return Err("invalid vascular momentum state");
        }
        let inertia = self.inertance / dt;
        let drive = pressure_difference + inertia * old_flow;
        let resistance = self.resistance + inertia;
        if !drive.is_finite() || !resistance.is_finite() {
            return Err("vascular flow overflow");
        }
        if self.valve && drive <= 0. {
            return Ok((0., 0.));
        }
        let magnitude = drive.abs();
        let root = resistance.hypot(2. * self.quadratic_resistance.sqrt() * magnitude.sqrt());
        let flow = drive / (0.5 * resistance + 0.5 * root);
        let derivative = (resistance + 2. * self.quadratic_resistance * flow.abs()).recip();
        if !flow.is_finite() || !derivative.is_finite() || derivative <= 0. {
            return Err("invalid vascular flow response");
        }
        Ok((flow, derivative))
    }
}
#[derive(Clone, Debug)]
pub struct Circulation {
    compartments: Vec<Compartment>,
    vessels: Vec<Vessel>,
    volumes: Vec<f64>,
    pressures: Vec<f64>,
    flows: Vec<f64>,
    elastance: Vec<f64>,
    external: Vec<f64>,
    custom_volume: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct CirculationStep {
    pub iterations: usize,
    pub residual_m3: f64,
    pub volume_drift_m3: f64,
    pub resistive_loss_j: f64,
    pub flow_kinetic_energy_j: f64,
}
/// Constitutive chamber volume and tangent at a trial transmural pressure.
#[derive(Clone, Copy, Debug)]
pub struct PressureVolume {
    pub volume_m3: f64,
    pub compliance_m3_per_pa: f64,
}
struct Assembly {
    custom_volume: bool,
    residual: Vec<f64>,
    jacobian: Vec<f64>,
    volumes: Vec<f64>,
    flows: Vec<f64>,
}
impl Circulation {
    /// Construct a connected closed graph with nonzero compliant storage at every
    /// node. Initial flows are zero; initial pressures follow each chamber law.
    /// # Errors
    /// Rejects invalid material parameters, volumes and disconnected topology.
    pub fn new(compartments: Vec<Compartment>, vessels: Vec<Vessel>) -> Result<Self, &'static str> {
        let count = compartments.len();
        if !(2..=256).contains(&count) || vessels.is_empty() || vessels.len() > 4096 {
            return Err("invalid circulation size");
        }
        for c in &compartments {
            if !c.unstressed_volume_m3.is_finite()
                || c.unstressed_volume_m3 < 0.
                || !c.initial_volume_m3.is_finite()
                || c.initial_volume_m3 <= 0.
                || !c.initial_elastance_pa_per_m3.is_finite()
                || c.initial_elastance_pa_per_m3 <= 0.
                || !c.initial_elastance_pa_per_m3.recip().is_finite()
                || !c.initial_external_pressure_pa.is_finite()
            {
                return Err("invalid circulatory compartment");
            }
        }
        for e in &vessels {
            if e.from >= count
                || e.to >= count
                || e.from == e.to
                || !e.resistance.is_finite()
                || e.resistance <= 0.
                || !e.resistance.recip().is_finite()
                || !e.quadratic_resistance.is_finite()
                || e.quadratic_resistance < 0.
                || !e.inertance.is_finite()
                || e.inertance < 0.
            {
                return Err("invalid vascular edge");
            }
        }
        let mut reached = vec![false; count];
        reached[0] = true;
        for _ in 0..count {
            for e in &vessels {
                if reached[e.from] || reached[e.to] {
                    reached[e.from] = true;
                    reached[e.to] = true;
                }
            }
        }
        if reached.iter().any(|v| !*v) {
            return Err("disconnected circulation");
        }
        let volumes: Vec<_> = compartments.iter().map(|c| c.initial_volume_m3).collect();
        let pressures: Vec<_> = compartments
            .iter()
            .map(|c| {
                c.initial_external_pressure_pa
                    + c.initial_elastance_pa_per_m3 * (c.initial_volume_m3 - c.unstressed_volume_m3)
            })
            .collect();
        if pressures.iter().any(|p| !p.is_finite()) || !volumes.iter().sum::<f64>().is_finite() {
            return Err("initial circulation overflow");
        }
        Ok(Self {
            flows: vec![0.; vessels.len()],
            volumes,
            pressures,
            elastance: compartments
                .iter()
                .map(|c| c.initial_elastance_pa_per_m3)
                .collect(),
            external: compartments
                .iter()
                .map(|c| c.initial_external_pressure_pa)
                .collect(),
            compartments,
            vessels,
            custom_volume: false,
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
    #[must_use]
    pub fn total_volume(&self) -> f64 {
        self.volumes.iter().sum()
    }
    /// Pressure across each chamber wall relative to its external pressure.
    #[must_use]
    pub fn transmural_pressures(&self) -> Vec<f64> {
        self.pressures
            .iter()
            .zip(&self.external)
            .map(|(p, e)| p - e)
            .collect()
    }
    /// Elastic chamber energy relative to each unstressed volume, J.
    /// # Errors
    /// A custom chamber volume law requires its own potential; the nominal
    /// linear elastance energy must not be substituted for that tissue energy.
    pub fn elastic_energy(&self) -> Result<f64, &'static str> {
        if self.custom_volume {
            return Err("custom chamber energy requires its constitutive potential");
        }
        Ok(self
            .compartments
            .iter()
            .zip(&self.volumes)
            .zip(&self.elastance)
            .map(|((c, v), e)| 0.5 * e * (v - c.unstressed_volume_m3).powi(2))
            .sum())
    }
    fn assemble<F>(
        &self,
        p: &[f64],
        elastance: &[f64],
        external: &[f64],
        dt: f64,
        volume_response: &mut F,
    ) -> Result<Assembly, &'static str>
    where
        F: FnMut(usize, f64) -> Result<Option<PressureVolume>, &'static str>,
    {
        let n = p.len();
        let mut residual = vec![0.; n];
        let mut jacobian = vec![0.; n * n];
        let mut volumes = vec![0.; n];
        let mut custom_volume = false;
        for i in 0..n {
            let (volume, compliance) = if let Some(response) =
                volume_response(i, p[i] - external[i])?
            {
                custom_volume = true;
                (response.volume_m3, response.compliance_m3_per_pa)
            } else {
                (
                    self.compartments[i].unstressed_volume_m3 + (p[i] - external[i]) / elastance[i],
                    elastance[i].recip(),
                )
            };
            if !volume.is_finite() || volume <= 0. || !compliance.is_finite() || compliance <= 0. {
                return Err("invalid constitutive blood volume or compliance");
            }
            volumes[i] = volume;
            residual[i] = volume - self.volumes[i];
            jacobian[i * n + i] = compliance;
        }
        let mut flows = Vec::with_capacity(self.vessels.len());
        for (e, old) in self.vessels.iter().zip(&self.flows) {
            let (flow, derivative) = e.flow(p[e.from] - p[e.to], *old, dt)?;
            flows.push(flow);
            let transfer = dt * flow;
            let conductance = dt * derivative;
            residual[e.from] += transfer;
            residual[e.to] -= transfer;
            jacobian[e.from * n + e.from] += conductance;
            jacobian[e.to * n + e.to] += conductance;
            jacobian[e.from * n + e.to] -= conductance;
            jacobian[e.to * n + e.from] -= conductance;
        }
        if residual.iter().chain(&jacobian).any(|v| !v.is_finite()) {
            return Err("vascular assembly overflow");
        }
        Ok(Assembly {
            custom_volume,
            residual,
            jacobian,
            volumes,
            flows,
        })
    }
    /// Backward-Euler closed-loop flow step. Chamber elastance/external pressures
    /// are prescribed at the end of this time interval. Failure is transactional.
    /// # Errors
    /// Rejects invalid input, negative volumes, overflow and nonlinear failure.
    pub fn step(
        &mut self,
        dt: f64,
        elastance: &[f64],
        external: &[f64],
        max_iterations: usize,
        tolerance_m3: f64,
    ) -> Result<CirculationStep, &'static str> {
        self.step_with_volume_response(
            dt,
            elastance,
            external,
            max_iterations,
            tolerance_m3,
            |_, _| Ok(None),
        )
    }
    /// Backward-Euler vessel flow with prescribed signed external fluid transfers.
    /// Positive `exchange_m3[i]` adds fluid to chamber i during this interval;
    /// negative values remove it. This is an integrated volume, not a flow rate.
    /// Pressure is recomputed from the supplied end-of-step chamber laws.
    /// The report's volume drift excludes the prescribed net external transfer.
    /// # Errors
    /// Rejects nonfinite/wrong-sized transfers, depleted staged inventories and
    /// failed circulation solves. Every failure preserves the original circuit.
    /// Exchange pressures and solute transport must be coupled by the caller;
    /// this API alone does not solve simultaneous tissue/circuit exchange.
    pub fn step_with_exchange(
        &mut self,
        dt: f64,
        elastance: &[f64],
        external: &[f64],
        exchange_m3: &[f64],
        max_iterations: usize,
        tolerance_m3: f64,
    ) -> Result<CirculationStep, &'static str> {
        if exchange_m3.len() != self.volumes.len() || exchange_m3.iter().any(|v| !v.is_finite()) {
            return Err("invalid external circulation exchange");
        }
        let mut trial = self.clone();
        for (volume, transfer) in trial.volumes.iter_mut().zip(exchange_m3) {
            *volume += transfer;
            if !volume.is_finite() || *volume <= 0. {
                return Err("external circulation exchange depletes inventory");
            }
        }
        if !trial.total_volume().is_finite() {
            return Err("external circulation exchange overflow");
        }
        let report = trial.step(dt, elastance, external, max_iterations, tolerance_m3)?;
        *self = trial;
        Ok(report)
    }
    /// Joint closed-circuit flow and conservative mixed-compartment protein step.
    /// Protein inventories are kilograms; concentrations use accepted blood volumes.
    /// Uses backward-Euler donor advection, including reverse vessel flow.
    /// # Errors
    /// Invalid inventories, failed flow or transport preserve circuit and protein.
    /// No protein production, binding, diffusion or red-cell/plasma partition is
    /// inferred; this treats each blood compartment as a homogeneous fluid.
    pub fn step_with_protein(
        &mut self,
        protein_kg: &mut [f64],
        dt: f64,
        elastance: &[f64],
        external: &[f64],
        max_iterations: usize,
        tolerance_m3: f64,
    ) -> Result<CirculationStep, &'static str> {
        let mut trial = self.clone();
        let report = trial.step(dt, elastance, external, max_iterations, tolerance_m3)?;
        let mass = trial.protein_at_accepted_flow(protein_kg, dt)?;
        protein_kg.copy_from_slice(&mass);
        *self = trial;
        Ok(report)
    }
    /// Joint prescribed external volume/protein exchange and vascular transport.
    /// Transfers are signed integrated m³ and kg, positive into each chamber.
    /// External donors/reflection must supply their own matching mass ledger;
    /// this method does not infer concentration from fluid transfer.
    /// # Errors
    /// Invalid/depleting exchange or any flow/transport error preserves all inputs.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn step_with_protein_exchange(
        &mut self,
        protein_kg: &mut [f64],
        dt: f64,
        elastance: &[f64],
        external: &[f64],
        exchange_m3: &[f64],
        exchange_kg: &[f64],
        max_iterations: usize,
        tolerance_m3: f64,
    ) -> Result<CirculationStep, &'static str> {
        let n = self.volumes.len();
        if protein_kg.len() != n
            || exchange_kg.len() != n
            || protein_kg.iter().any(|m| !m.is_finite() || *m < 0.)
            || exchange_kg.iter().any(|m| !m.is_finite())
        {
            return Err("invalid external circulating protein exchange");
        }
        let staged: Vec<_> = protein_kg
            .iter()
            .zip(exchange_kg)
            .map(|(m, d)| m + d)
            .collect();
        if staged.iter().any(|m| !m.is_finite() || *m < 0.) {
            return Err("external protein exchange depletes inventory");
        }
        let mut trial = self.clone();
        let report = trial.step_with_exchange(
            dt,
            elastance,
            external,
            exchange_m3,
            max_iterations,
            tolerance_m3,
        )?;
        let mass = trial.protein_at_accepted_flow(&staged, dt)?;
        protein_kg.copy_from_slice(&mass);
        *self = trial;
        Ok(report)
    }
    fn protein_at_accepted_flow(
        &self,
        protein_kg: &[f64],
        dt: f64,
    ) -> Result<Vec<f64>, &'static str> {
        let n = self.volumes.len();
        if protein_kg.len() != n
            || protein_kg.iter().any(|m| !m.is_finite() || *m < 0.)
            || !protein_kg.iter().sum::<f64>().is_finite()
        {
            return Err("invalid circulating protein inventory");
        }
        let mut diagonal = vec![1.; n];
        let mut incoming = vec![Vec::new(); n];
        for (vessel, flow) in self.vessels.iter().zip(&self.flows) {
            let (donor, receiver) = if *flow >= 0. {
                (vessel.from, vessel.to)
            } else {
                (vessel.to, vessel.from)
            };
            let coefficient = dt * flow.abs() / self.volumes[donor];
            diagonal[donor] += coefficient;
            if !coefficient.is_finite() || !diagonal[donor].is_finite() {
                return Err("circulating protein transport overflow");
            }
            incoming[receiver].push((donor, coefficient));
        }
        crate::positive_transport::solve_protein(protein_kg, &diagonal, &incoming, &vec![0.; n])
    }
    /// Closed-loop solve with selected chamber laws supplied by a volume/tangent
    /// callback. The callback receives index and trial transmural pressure.
    /// It must evaluate a frozen constitutive state without committing history.
    /// # Errors
    /// Failure leaves circuit state unchanged; callback-owned side effects are
    /// outside this transaction and must be managed by the caller.
    pub fn step_with_volume_response<F>(
        &mut self,
        dt: f64,
        elastance: &[f64],
        external: &[f64],
        max_iterations: usize,
        tolerance_m3: f64,
        mut volume_response: F,
    ) -> Result<CirculationStep, &'static str>
    where
        F: FnMut(usize, f64) -> Result<Option<PressureVolume>, &'static str>,
    {
        let n = self.compartments.len();
        if !dt.is_finite()
            || dt <= 0.
            || elastance.len() != n
            || external.len() != n
            || elastance
                .iter()
                .any(|v| !v.is_finite() || *v <= 0. || !v.recip().is_finite())
            || external.iter().any(|v| !v.is_finite())
            || !(1..=128).contains(&max_iterations)
            || !tolerance_m3.is_finite()
            || tolerance_m3 <= 0.
        {
            return Err("invalid circulation step");
        }
        let mut pressure: Vec<_> = self
            .compartments
            .iter()
            .zip(&self.volumes)
            .enumerate()
            .map(|(i, (c, v))| {
                if self.custom_volume {
                    external[i] + self.pressures[i] - self.external[i]
                } else {
                    external[i] + elastance[i] * (v - c.unstressed_volume_m3)
                }
            })
            .collect();
        let mut iterations = 0;
        loop {
            let Assembly {
                residual, jacobian, ..
            } = self.assemble(&pressure, elastance, external, dt, &mut volume_response)?;
            let norm = residual.iter().fold(0_f64, |a, b| a.max(b.abs()));
            if norm <= tolerance_m3 {
                break;
            }
            if iterations >= max_iterations {
                return Err("circulation did not converge");
            }
            let direction = solve_spd(jacobian, residual.iter().map(|v| -v).collect())?;
            let mut fraction = 1.;
            let mut accepted = false;
            for _ in 0..40 {
                let trial: Vec<_> = pressure
                    .iter()
                    .zip(&direction)
                    .map(|(p, d)| p + fraction * d)
                    .collect();
                if let Ok(a) = self.assemble(&trial, elastance, external, dt, &mut volume_response)
                {
                    let new_norm = a.residual.iter().fold(0_f64, |a, b| a.max(b.abs()));
                    if new_norm <= norm * (1. - 1e-4 * fraction) || new_norm <= tolerance_m3 {
                        pressure = trial;
                        accepted = true;
                        break;
                    }
                }
                fraction *= 0.5;
            }
            if !accepted {
                return Err("circulation line search failed");
            }
            iterations += 1;
        }
        let a = self.assemble(&pressure, elastance, external, dt, &mut volume_response)?;
        let volume_drift_m3 = a.volumes.iter().sum::<f64>() - self.total_volume();
        let mut resistive_loss_j = 0.;
        let mut flow_kinetic_energy_j = 0.;
        for (e, q) in self.vessels.iter().zip(&a.flows) {
            resistive_loss_j +=
                dt * (e.resistance * q * q + e.quadratic_resistance * q.abs().powi(3));
            flow_kinetic_energy_j += 0.5 * e.inertance * q * q;
        }
        if !volume_drift_m3.is_finite()
            || !resistive_loss_j.is_finite()
            || !flow_kinetic_energy_j.is_finite()
        {
            return Err("circulation diagnostic overflow");
        }
        let report = CirculationStep {
            iterations,
            volume_drift_m3,
            resistive_loss_j,
            flow_kinetic_energy_j,
            residual_m3: a.residual.iter().fold(0_f64, |v, r| v.max(r.abs())),
        };
        self.custom_volume = a.custom_volume;
        self.volumes = a.volumes;
        self.pressures = pressure;
        self.flows = a.flows;
        self.elastance = elastance.to_vec();
        self.external = external.to_vec();
        Ok(report)
    }
}
fn solve_spd(mut a: Vec<f64>, mut rhs: Vec<f64>) -> Result<Vec<f64>, &'static str> {
    let n = rhs.len();
    for i in 0..n {
        for j in 0..=i {
            let mut v = a[i * n + j];
            for k in 0..j {
                v -= a[i * n + k] * a[j * n + k];
            }
            if i == j {
                if !v.is_finite() || v <= 0. {
                    return Err("singular circulatory Jacobian");
                }
                a[i * n + j] = v.sqrt();
            } else {
                a[i * n + j] = v / a[j * n + j];
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
        return Err("circulatory solve overflow");
    }
    Ok(rhs)
}

/// Computes laminar Hagen–Poiseuille endothelial wall shear stress
/// $\tau_w = \frac{4 \mu Q}{\pi r^3}$ in Pascals (`Pa`) for a cylindrical vessel of radius
/// `radius_m` carrying volumetric flow `flow_m3_s`.
///
/// # Errors
///
/// Rejects non-finite inputs, non-positive viscosity or radius, or numerical overflow.
pub fn wall_shear_stress_pa(
    viscosity_pa_s: f64,
    flow_m3_s: f64,
    radius_m: f64,
) -> Result<f64, &'static str> {
    if !viscosity_pa_s.is_finite()
        || !flow_m3_s.is_finite()
        || !radius_m.is_finite()
        || viscosity_pa_s <= 0.0
        || radius_m <= 0.0
    {
        return Err("invalid wall shear stress input");
    }
    let denom = std::f64::consts::PI * radius_m * radius_m * radius_m;
    let tau = 4.0 * viscosity_pa_s * flow_m3_s / denom;
    if !tau.is_finite() {
        return Err("wall shear stress overflow");
    }
    Ok(tau)
}

/// Integrates one exact exponential step of a 2-element arterial Windkessel model:
/// $C \frac{dP}{dt} + \frac{P - P_d}{R} = Q_{\text{in}}$.
#[must_use]
pub fn windkessel_2element_arterial_pressure_step(
    p_prev: f64,
    inflow_m3_s: f64,
    resistance_pa_s_m3: f64,
    compliance_m3_pa: f64,
    diastolic_p: f64,
    dt_s: f64,
) -> Option<f64> {
    if !p_prev.is_finite()
        || !inflow_m3_s.is_finite()
        || !resistance_pa_s_m3.is_finite()
        || !compliance_m3_pa.is_finite()
        || !diastolic_p.is_finite()
        || !dt_s.is_finite()
        || resistance_pa_s_m3 <= 0.0
        || compliance_m3_pa <= 0.0
        || dt_s < 0.0
    {
        return None;
    }
    let tau = resistance_pa_s_m3 * compliance_m3_pa;
    let p_steady = diastolic_p + inflow_m3_s * resistance_pa_s_m3;
    let decay = (-dt_s / tau).exp();
    let p_next = p_steady + (p_prev - p_steady) * decay;
    p_next.is_finite().then_some(p_next)
}

/// Evaluates the non-Newtonian Casson apparent blood viscosity
/// $\mu_{\text{app}}(\dot{\gamma}) = \left(\sqrt{\mu_\infty} + \sqrt{\frac{\tau_y}{\dot{\gamma}}}\right)^2$.
#[must_use]
pub fn casson_blood_apparent_viscosity_pa_s(
    shear_rate_s_inv: f64,
    yield_stress_pa: f64,
    inf_viscosity_pa_s: f64,
) -> Option<f64> {
    if !shear_rate_s_inv.is_finite()
        || !yield_stress_pa.is_finite()
        || !inf_viscosity_pa_s.is_finite()
        || shear_rate_s_inv <= 0.0
        || yield_stress_pa < 0.0
        || inf_viscosity_pa_s <= 0.0
    {
        return None;
    }
    let root = inf_viscosity_pa_s.sqrt() + (yield_stress_pa / shear_rate_s_inv).sqrt();
    let mu = root * root;
    mu.is_finite().then_some(mu)
}

/// Evaluates the Pries–Secomb–Gaehtgens in-vitro Fåhræus–Lindqvist relative blood viscosity
/// factor in microvessels of diameter `diameter_um` ($\mu\text{m}$) at discharge hematocrit `hematocrit`.
#[must_use]
pub fn fahraeus_lindqvist_microvessel_viscosity_factor(
    diameter_um: f64,
    hematocrit: f64,
) -> Option<f64> {
    if !diameter_um.is_finite()
        || !hematocrit.is_finite()
        || diameter_um < 3.0
        || !(0.0..=0.85).contains(&hematocrit)
    {
        return None;
    }
    let mu_45 =
        220.0 * (-1.3 * diameter_um).exp() + 3.2 - 2.44 * (-0.06 * diameter_um.powf(0.645)).exp();
    let c = (0.8 + (-0.075 * diameter_um).exp())
        * (-1.0 + 1.0 / (1.0 + 1e-11 * diameter_um.powi(12)))
        + 1.0 / (1.0 + 1e-11 * diameter_um.powi(12));
    let denom = (1.0 - 0.45_f64).powf(c) - 1.0;
    if denom.abs() <= 1e-12 {
        return Some(1.0);
    }
    let rel = 1.0 + (mu_45 - 1.0) * (((1.0 - hematocrit).powf(c) - 1.0) / denom);
    rel.is_finite().then_some(rel.max(1.0))
}

#[cfg(test)]
mod wss_tests {
    use super::*;

    #[test]
    fn wall_shear_stress_scales_inversely_with_cube_of_radius() {
        let r1 = wall_shear_stress_pa(0.0035, 1e-6, 0.001).unwrap();
        let r2 = wall_shear_stress_pa(0.0035, 1e-6, 0.002).unwrap();
        assert!((r1 / r2 - 8.0).abs() < 1e-9);
        assert!(wall_shear_stress_pa(0.0, 1e-6, 0.001).is_err());
    }

    #[test]
    fn windkessel_casson_and_fahraeus_lindqvist_models_behave_physiologically() {
        let p_diastole =
            windkessel_2element_arterial_pressure_step(16_000.0, 0.0, 1.0e8, 1.5e-8, 10_600.0, 0.5)
                .unwrap();
        assert!(p_diastole < 16_000.0 && p_diastole > 10_600.0);
        let mu_low = casson_blood_apparent_viscosity_pa_s(1.0, 0.005, 0.0035).unwrap();
        let mu_high = casson_blood_apparent_viscosity_pa_s(200.0, 0.005, 0.0035).unwrap();
        assert!(mu_low > mu_high && mu_high > 0.0035);
        let fl_cap = fahraeus_lindqvist_microvessel_viscosity_factor(10.0, 0.45).unwrap();
        let fl_art = fahraeus_lindqvist_microvessel_viscosity_factor(300.0, 0.45).unwrap();
        assert!(fl_cap < fl_art);
    }
}
