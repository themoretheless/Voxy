//! Uniform-pressure porous tissue: finite-volume Biot storage approximation.
//! This lumped limit has no spatial Darcy pressure field or measured defaults.
use super::{Body, Matrix, columns, det, inverse, mm, sub, transpose};
#[derive(Clone, Copy, Debug)]
pub struct PoreFluid {
    pub reference_fluid_volume_m3: f64,
    pub fluid_volume_m3: f64,
    pub biot_coefficient: f64,
    /// Total storage at fixed skeleton geometry, m³/Pa; must be strictly positive.
    pub storage_m3_per_pa: f64,
}
impl Body {
    #[must_use]
    pub fn reference_volume(&self) -> f64 {
        self.elements.iter().map(|e| e.volume).sum()
    }
    /// # Errors
    /// Rejects mismatched geometry, inverted cells or volume overflow.
    pub fn volume_at(&self, positions: &[[f64; 3]]) -> Result<f64, &'static str> {
        if positions.len() != self.positions.len()
            || positions.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid porous geometry");
        }
        let mut volume = 0.;
        for e in &self.elements {
            let [a, b, c, d] = e.nodes.map(|i| positions[i]);
            let j = det(mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest));
            if !j.is_finite() || j <= 0. {
                return Err("invalid porous deformation");
            }
            volume += e.volume * j;
        }
        if !volume.is_finite() {
            return Err("porous volume overflow");
        }
        Ok(volume)
    }
    /// Uniform pore storage with energy `(Vf-Vf0-alpha*(V-V0))²/(2*S)`.
    /// Reference fluid volume must not exceed reference tissue volume.
    /// # Errors
    /// Rejects invalid fluid inventory, Biot coefficient, storage or overflow.
    pub fn set_pore_fluid(&mut self, fluid: PoreFluid) -> Result<(), &'static str> {
        super::cell_poroelastic::validate_fluid(fluid, self.reference_volume())?;
        let mut next = self.clone();
        next.pore_fluid = Some(fluid);
        next.cell_pore_fluids.clear();
        next.pore_response_at(next.positions())?;
        *self = next;
        Ok(())
    }
    #[must_use]
    pub fn pore_fluid(&self) -> Option<PoreFluid> {
        self.pore_fluid
    }
    /// Signed pore pressure and stored-fluid energy (Pa, J). Negative pressure
    /// is supported numerically; cavitation and desaturation are not represented.
    /// # Errors
    /// Rejects invalid deformation or overflowing pore energy/pressure.
    pub fn pore_response_at(&self, positions: &[[f64; 3]]) -> Result<(f64, f64), &'static str> {
        if !self.cell_pore_fluids.is_empty() {
            return Err("use cell pore pressure response");
        }
        let Some(fluid) = self.pore_fluid else {
            return Ok((0., 0.));
        };
        let content = fluid.fluid_volume_m3
            - fluid.reference_fluid_volume_m3
            - fluid.biot_coefficient * (self.volume_at(positions)? - self.reference_volume());
        let p = content / fluid.storage_m3_per_pa;
        let energy = 0.5 * p * content;
        if !p.is_finite() || !energy.is_finite() {
            return Err("pore storage overflow");
        }
        Ok((p, energy))
    }
    pub(super) fn pore_piola(f: Matrix, pressure: f64) -> Result<Matrix, &'static str> {
        if pressure == 0. {
            return Ok([[0.; 3]; 3]);
        }
        let s = -pressure * det(f);
        Ok(transpose(inverse(f)?).map(|r| r.map(|v| s * v)))
    }
}

/// Elastic, uniformly pressurized porous FEM specimen mapped to a fluid node.
/// Both network and geometry commit only after all substeps and final solve succeed.
#[derive(Clone, Debug)]
pub struct PoreTissue {
    body: Body,
    fluid_node: usize,
    iterations: usize,
    tolerance_n: f64,
}
impl PoreTissue {
    /// # Errors
    /// Rejects missing pore storage, viscous histories or invalid solver options.
    pub fn new(
        body: Body,
        fluid_node: usize,
        iterations: usize,
        tolerance_n: f64,
    ) -> Result<Self, &'static str> {
        if body.pore_fluid.is_none()
            || body.elements.iter().any(|e| e.viscoelastic.is_some())
            || !(1..=100_000).contains(&iterations)
            || !tolerance_n.is_finite()
            || tolerance_n <= 0.
        {
            return Err("invalid elastic pore tissue");
        }
        Ok(Self {
            body,
            fluid_node,
            iterations,
            tolerance_n,
        })
    }
    #[must_use]
    pub fn body(&self) -> &Body {
        &self.body
    }
    /// # Errors
    /// Rejects inventory mismatch, failed transport or unconverged FEM; all state is preserved.
    pub fn step(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        seconds: f64,
        max_step: f64,
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        let initial = network
            .volumes()
            .get(self.fluid_node)
            .ok_or("invalid pore fluid node")?;
        let pore = self.body.pore_fluid.ok_or("missing pore storage")?;
        if (*initial - pore.fluid_volume_m3).abs() > 1e-10 * initial.abs().max(pore.fluid_volume_m3)
        {
            return Err("pore/network fluid volume mismatch");
        }
        let mut trial_body = self.body.clone();
        let mut trial_network = network.clone();
        let report =
            trial_network.step_with_pressure_law(seconds, max_step, |volumes, linear| {
                let mut pore = trial_body.pore_fluid.ok_or("missing trial pore storage")?;
                pore.fluid_volume_m3 = volumes[self.fluid_node];
                trial_body.set_pore_fluid(pore)?;
                let equilibrium = trial_body.equilibrate(self.iterations, self.tolerance_n)?;
                if !equilibrium.converged {
                    return Err("pore FEM equilibrium did not converge");
                }
                let pressure = trial_body.pore_response_at(trial_body.positions())?.0;
                let mut p = linear.to_vec();
                p[self.fluid_node] = pressure;
                Ok(p)
            })?;
        self.body = trial_body;
        *network = trial_network;
        Ok(report)
    }
}
