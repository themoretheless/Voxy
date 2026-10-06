//! Time-error admission around the existing conservative thermal vapor law.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct ThermalVaporAccuracy {
    pub relative_tolerance: f64,
    pub mass_tolerance_kg: f64,
    pub temperature_tolerance_k: f64,
    pub max_attempts: usize,
}
impl Default for ThermalVaporAccuracy {
    fn default() -> Self {
        Self {
            relative_tolerance: 1e-5,
            mass_tolerance_kg: 1e-10,
            temperature_tolerance_k: 1e-4,
            max_attempts: 10000,
        }
    }
}
/// Receipt for a fully admitted trajectory; accepted intervals use two half steps.
#[derive(Clone, Debug)]
pub struct ThermalVaporStep {
    pub transfer: VaporTransfer,
    pub elapsed_s: f64,
    pub attempts: usize,
    pub accepted_intervals: usize,
    pub rejected_attempts: usize,
    pub minimum_interval_s: f64,
}
impl Body {
    /// Adaptive first-order split exchange with a finite thermal vapor owner.
    /// Step doubling controls local mass/temperature error, not accumulated
    /// global error or constitutive calibration. No condensate model is added.
    /// # Errors
    /// Invalid controls, incompatible links, domain or refinement failure.
    /// Both owners stay unchanged unless the complete elapsed time is admitted.
    pub fn advance_thermal_vapor_adaptive(
        &mut self,
        dt_s: f64,
        vapor: &mut ThermalVapor,
        links: &[VaporLink],
        accuracy: ThermalVaporAccuracy,
    ) -> Result<VaporTransfer, &'static str> {
        self.advance_thermal_vapor_adaptive_with_receipt(dt_s, vapor, links, accuracy)
            .map(|report| report.transfer)
    }
    /// Admit the full trajectory and return refinement diagnostics.
    /// # Errors
    /// Same atomic controls, domain, conservation and budget admission as the
    /// transfer-only API. No receipt is published for an incomplete trajectory.
    pub fn advance_thermal_vapor_adaptive_with_receipt(
        &mut self,
        dt_s: f64,
        vapor: &mut ThermalVapor,
        links: &[VaporLink],
        accuracy: ThermalVaporAccuracy,
    ) -> Result<ThermalVaporStep, &'static str> {
        if !dt_s.is_finite()
            || dt_s <= 0.
            || !accuracy.relative_tolerance.is_finite()
            || accuracy.relative_tolerance < 0.
            || !accuracy.mass_tolerance_kg.is_finite()
            || accuracy.mass_tolerance_kg <= 0.
            || !accuracy.temperature_tolerance_k.is_finite()
            || accuracy.temperature_tolerance_k <= 0.
            || accuracy.max_attempts == 0
        {
            return Err("invalid adaptive thermal vapor controls");
        }
        // Admit the complete configured network before attempting refinement.
        let template = self.prepare_vapor_network(&vapor.reservoir, links)?;
        let normalized = |initial: f64, coarse: f64, fine: f64, absolute: f64| {
            (coarse - fine).abs()
                / (absolute + accuracy.relative_tolerance * initial.abs().max(fine.abs()))
        };
        let ((next, gas), receipt) =
            crate::liquid::evaporation::adaptive_step_doubling_with_receipt(
                (self.clone(), vapor.clone()),
                dt_s,
                accuracy.max_attempts,
                2.,
                |(mut body, mut gas), step| {
                    body.advance_thermal_vapor_prepared(step, &mut gas, template.clone())
                        .map_err(|_| crate::liquid::Error::NumericalFailure)?;
                    Ok((body, gas))
                },
                |initial, coarse, fine| {
                    let mut error = normalized(
                        initial.1.temperature_k(),
                        coarse.1.temperature_k(),
                        fine.1.temperature_k(),
                        accuracy.temperature_tolerance_k,
                    );
                    error = error.max(normalized(
                        initial.1.water_kg(),
                        coarse.1.water_kg(),
                        fine.1.water_kg(),
                        accuracy.mass_tolerance_kg,
                    ));
                    for ((a, b), c) in initial
                        .0
                        .cells
                        .iter()
                        .zip(&coarse.0.cells)
                        .zip(&fine.0.cells)
                    {
                        error = error.max(normalized(
                            a.water_kg,
                            b.water_kg,
                            c.water_kg,
                            accuracy.mass_tolerance_kg,
                        ));
                    }
                    if !error.is_finite() {
                        return Err(crate::liquid::Error::NumericalFailure);
                    }
                    Ok(error)
                },
            )
            .map_err(|_| "adaptive thermal vapor refinement failed")?;
        let changes: Vec<_> = next
            .cells
            .iter()
            .zip(&self.cells)
            .map(|(a, b)| a.water_kg - b.water_kg)
            .collect();
        let vapor_change = gas.water_kg() - vapor.water_kg();
        let report = VaporTransfer {
            mass_defect_kg: changes.iter().sum::<f64>() + vapor_change,
            energy_defect_j: gas.accounted_energy_j() - vapor.accounted_energy_j(),
            latent_exchange_j: vapor.reservoir.latent_j_kg * vapor_change,
            material_water_change_kg: changes,
            vapor_water_change_kg: vapor_change,
        };
        if [
            report.mass_defect_kg,
            report.energy_defect_j,
            report.latent_exchange_j,
            report.vapor_water_change_kg,
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err("adaptive thermal vapor balance overflow");
        }
        let mass_scale = self.cells.iter().map(|c| c.water_kg).sum::<f64>() + vapor.water_kg();
        let energy_scale = vapor.accounted_energy_j();
        if report.mass_defect_kg.abs() > 1e-10 * mass_scale.max(f64::MIN_POSITIVE)
            || report.energy_defect_j.abs() > 1e-10 * energy_scale.max(f64::MIN_POSITIVE)
        {
            return Err("adaptive thermal vapor conservation failed");
        }
        *self = next;
        *vapor = gas;
        Ok(ThermalVaporStep {
            transfer: report,
            elapsed_s: dt_s,
            attempts: receipt.attempts,
            accepted_intervals: receipt.accepted_intervals,
            rejected_attempts: receipt.rejected_attempts,
            minimum_interval_s: receipt.minimum_interval_s,
        })
    }
}

#[cfg(test)]
mod preparation_benchmark {
    use super::*;
    #[test]
    #[ignore = "manual prepared-network comparison; no timing threshold"]
    fn prepared_network_benchmark() {
        let curve = crate::liquid::SaturationCurve {
            reference_temperature: 300.,
            reference_pressure: 3500.,
            latent_heat: 2.4e6,
            vapor_gas_constant: 461.,
            min_temperature: 280.,
            max_temperature: 320.,
        };
        let body = Body::new(
            vec![
                Cell {
                    capacity_kg: 0.01,
                    water_kg: 0.005
                };
                32
            ],
            (0..31)
                .map(|i| Link {
                    cells: [i, i + 1],
                    conductance_kg_s: 1e-5,
                })
                .collect(),
        )
        .unwrap();
        let gas = ThermalVapor::new(300., 1000., 10., 0.001, curve).unwrap();
        let links: Vec<_> = (0..32)
            .map(|i| VaporLink {
                material_cell: i,
                conductance_kg_s: 1e-5,
            })
            .collect();
        let (mut reconstructed, mut reconstructed_gas) = (body.clone(), gas.clone());
        let begin = std::time::Instant::now();
        for _ in 0..100 {
            reconstructed
                .advance_thermal_vapor(0.001, &mut reconstructed_gas, &links)
                .unwrap();
        }
        let reconstruct_time = begin.elapsed();
        let (mut prepared, mut prepared_gas) = (body, gas);
        let begin = std::time::Instant::now();
        let template = prepared
            .prepare_vapor_network(&prepared_gas.reservoir, &links)
            .unwrap();
        for _ in 0..100 {
            prepared
                .advance_thermal_vapor_prepared(0.001, &mut prepared_gas, template.clone())
                .unwrap();
        }
        let prepare_time = begin.elapsed();
        assert_eq!(
            format!("{reconstructed:?}{reconstructed_gas:?}"),
            format!("{prepared:?}{prepared_gas:?}")
        );
        eprintln!(
            "PREPARED_NETWORK_BENCH material_cells=32 steps=100 reconstruct_ns={} prepared_ns={}",
            reconstruct_time.as_nanos(),
            prepare_time.as_nanos()
        );
    }
}
