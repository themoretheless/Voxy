//! Transactional single-cavity 3D FEM / closed-loop circulation coupling.
use super::{Body, cross, dot, sub};
use crate::circulation::{Circulation, CirculationStep, PressureVolume};
#[derive(Clone, Copy, Debug)]
pub struct CouplingConfig {
    pub fem_iterations: usize,
    pub force_tolerance_n: f64,
    /// Finite-difference pressure increment used to measure chamber compliance.
    pub pressure_increment_pa: f64,
    pub volume_tolerance_m3: f64,
}
impl Default for CouplingConfig {
    fn default() -> Self {
        Self {
            fem_iterations: 4000,
            force_tolerance_n: 1e-7,
            pressure_increment_pa: 1.,
            volume_tolerance_m3: 1e-11,
        }
    }
}
#[derive(Clone, Debug)]
pub struct FemChamber {
    body: Body,
    circuit_node: usize,
    config: CouplingConfig,
}
impl Body {
    /// Actual volume enclosed by the deformed oriented cavity boundary, m³.
    /// Translation-relative coordinates reduce cancellation far from the origin.
    /// # Errors
    /// Rejects invalid cavity index and nonpositive/nonfinite enclosed volume.
    pub fn cavity_volume(&self, index: usize) -> Result<f64, &'static str> {
        let cavity = self
            .cavities
            .get(index)
            .ok_or("invalid cavity volume index")?;
        let origin = self.positions[cavity.faces[0][0]];
        let volume = cavity
            .faces
            .iter()
            .map(|face| {
                let [a, b, c] = face.map(|i| sub(self.positions[i], origin));
                dot(a, cross(b, c)) / 6.
            })
            .sum::<f64>();
        if !volume.is_finite() || volume <= 0. {
            return Err("invalid deformed cavity volume");
        }
        Ok(volume)
    }
}
impl FemChamber {
    /// One independent deformable cavity mapped to one circuit compartment.
    /// Shared multi-cavity walls require off-diagonal compliance and are rejected.
    /// # Errors
    /// Rejects unsupported geometry and invalid convergence/tangent parameters.
    pub fn new(
        body: Body,
        circuit_node: usize,
        config: CouplingConfig,
    ) -> Result<Self, &'static str> {
        if body.cavities().len() != 1
            || !(1..=100_000).contains(&config.fem_iterations)
            || !config.force_tolerance_n.is_finite()
            || config.force_tolerance_n <= 0.
            || !config.pressure_increment_pa.is_finite()
            || config.pressure_increment_pa <= 0.
            || !config.volume_tolerance_m3.is_finite()
            || config.volume_tolerance_m3 <= 0.
        {
            return Err("invalid independent FEM chamber");
        }
        body.cavity_volume(0)?;
        Ok(Self {
            body,
            circuit_node,
            config,
        })
    }
    #[must_use]
    pub fn body(&self) -> &Body {
        &self.body
    }
    /// Changes loads/activation before the next coupled increment. Committed
    /// geometry must remain volume-consistent with the circuit compartment.
    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }
    fn solve_at(&self, pressure: f64, dt: f64) -> Result<(f64, Body), &'static str> {
        self.solve_with_guess(pressure, dt, None)
    }
    fn solve_with_guess(
        &self,
        pressure: f64,
        dt: f64,
        geometry: Option<&[super::Vec3]>,
    ) -> Result<(f64, Body), &'static str> {
        let mut body = self.body.clone();
        if let Some(geometry) = geometry {
            body.positions = geometry.to_vec();
        }
        body.set_pressure(0, pressure)?;
        body.relax_step(
            dt,
            self.config.fem_iterations,
            self.config.force_tolerance_n,
        )?;
        Ok((body.cavity_volume(0)?, body))
    }
    fn response(&self, pressure: f64, dt: f64) -> Result<PressureVolume, &'static str> {
        let (volume, center_body) = self.solve_at(pressure, dt)?;
        let h = self.config.pressure_increment_pa;
        let high = self
            .solve_with_guess(pressure + h, dt, Some(center_body.positions()))?
            .0;
        let compliance = if pressure >= h {
            (high
                - self
                    .solve_with_guess(pressure - h, dt, Some(center_body.positions()))?
                    .0)
                / (2. * h)
        } else {
            (high - volume) / h
        };
        if !compliance.is_finite() || compliance <= 0. {
            return Err("nonpositive FEM cavity compliance");
        }
        Ok(PressureVolume {
            volume_m3: volume,
            compliance_m3_per_pa: compliance,
        })
    }
    /// Solve flow and cavity deformation together using an outer pressure Newton
    /// solve and converged inner FEM solves. Viscous history starts from the same
    /// old body in every trial; both circuit and body commit only after success.
    /// Elastance at the mapped compartment supplies only the first initial guess;
    /// later steps start from the previously accepted coupled pressure.
    /// that compartment's actual pressure-volume law comes from the FEM wall.
    /// # Errors
    /// Rejects mismatched initial volume, negative wall pressure, unstable tangent,
    /// failed inner/outer solve or final geometry/flow volume inconsistency.
    pub fn step(
        &mut self,
        circuit: &mut Circulation,
        dt: f64,
        elastance: &[f64],
        external: &[f64],
        max_iterations: usize,
    ) -> Result<CirculationStep, &'static str> {
        let old_volume = circuit
            .volumes()
            .get(self.circuit_node)
            .ok_or("invalid FEM circuit node")?;
        if (self.body.cavity_volume(0)? - old_volume).abs() > self.config.volume_tolerance_m3 {
            return Err("FEM/circuit initial volumes differ");
        }
        let mut trial = circuit.clone();
        let report = trial.step_with_volume_response(
            dt,
            elastance,
            external,
            max_iterations,
            self.config.volume_tolerance_m3,
            |node, pressure| {
                if node == self.circuit_node {
                    Ok(Some(self.response(pressure, dt)?))
                } else {
                    Ok(None)
                }
            },
        )?;
        let pressure = trial.transmural_pressures()[self.circuit_node];
        let (volume, body) = self.solve_at(pressure, dt)?;
        if (volume - trial.volumes()[self.circuit_node]).abs() > self.config.volume_tolerance_m3 {
            return Err("FEM/circuit accepted volumes differ");
        }
        self.body = body;
        *circuit = trial;
        Ok(report)
    }
}
