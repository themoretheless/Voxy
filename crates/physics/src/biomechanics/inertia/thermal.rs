//! Cell-local sensible energy referenced to an immutable initial temperature.
use super::InertialBody;
#[derive(Clone, Debug)]
pub(super) struct CellThermalState {
    capacity_j_per_k: Vec<f64>,
    reference_kelvin: Vec<f64>,
    excess_j: Vec<f64>,
    correction_j: Vec<f64>,
}
impl CellThermalState {
    pub(super) fn deposit(&mut self, cell: usize, heat: f64) -> Result<f64, &'static str> {
        if !heat.is_finite() {
            return Err("invalid Maxwell cell heat");
        }
        let old = self.excess_j[cell];
        let sum = old + heat;
        let correction = if old.abs() >= heat.abs() {
            (old - sum) + heat
        } else {
            (heat - sum) + old
        };
        let tail = self.correction_j[cell] + correction;
        let temperature = self.reference_kelvin[cell] + (sum + tail) / self.capacity_j_per_k[cell];
        if !sum.is_finite()
            || !tail.is_finite()
            || !temperature.is_finite()
            || temperature <= 0.
            || (heat != 0. && sum == old && tail == self.correction_j[cell])
        {
            return Err("unrepresentable Maxwell thermal inventory");
        }
        let defect = (sum - old) + (tail - self.correction_j[cell]) - heat;
        if !defect.is_finite() {
            return Err("thermal deposit defect overflow");
        }
        self.excess_j[cell] = sum;
        self.correction_j[cell] = tail;
        Ok(defect)
    }
}
impl InertialBody {
    /// Enables cell-local conversion of released Maxwell energy into sensible
    /// heat. Specific heats are J/(kg K); temperatures are Kelvin per cell.
    /// Capacity uses the same reference cell mass as inertial assembly.
    /// Reinitialization is rejected so stored heat cannot silently disappear.
    /// This does not add conduction or temperature-dependent material parameters.
    /// # Errors
    /// Invalid configuration/history/overflow preserves the complete owner.
    pub fn enable_maxwell_thermal(
        &mut self,
        specific_heats: &[f64],
        kelvin: &[f64],
    ) -> Result<(), &'static str> {
        let n = self.cell_masses.len();
        if self.thermal.is_some()
            || specific_heats.len() != n
            || kelvin.len() != n
            || specific_heats.iter().any(|v| !v.is_finite() || *v <= 0.)
            || kelvin.iter().any(|v| !v.is_finite() || *v <= 0.)
            || !self.body.elements.iter().any(|e| e.viscoelastic.is_some())
        {
            return Err("invalid Maxwell thermal configuration");
        }
        let capacities: Vec<_> = self
            .cell_masses
            .iter()
            .zip(specific_heats)
            .map(|(m, c)| m * c)
            .collect();
        if capacities.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err("invalid Maxwell heat capacity");
        }
        self.diagnostics()?;
        self.thermal = Some(CellThermalState {
            capacity_j_per_k: capacities,
            reference_kelvin: kelvin.to_vec(),
            excess_j: vec![0.; n],
            correction_j: vec![0.; n],
        });
        Ok(())
    }
    /// Symmetric closed heat exchange across caller-supplied cell links.
    /// Links are (cell a, cell b, conductance W/K). Conductance must come from
    /// the caller's physical geometry/contact model. This is a lumped thermal
    /// network, not a claim of mesh-independent continuum heat diffusion.
    /// Forward half-steps followed by reverse half-steps preserve closed energy
    /// and use the shared finite-capacity film/solid transfer law.
    /// Returns the signed thermal energy defect, joules.
    /// # Errors
    /// Invalid/duplicate links, numerical loss or excess defect preserves state.
    pub fn conduct_maxwell_heat(
        &mut self,
        links: &[(usize, usize, f64)],
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<f64, &'static str> {
        let thermal = self
            .thermal
            .as_ref()
            .ok_or("missing cell thermal inventory")?;
        if !dt.is_finite() || dt < 0. || !energy_tolerance_j.is_finite() || energy_tolerance_j <= 0.
        {
            return Err("invalid cell heat conduction controls");
        }
        let mut seen = std::collections::BTreeSet::new();
        for &(a, b, g) in links {
            if a >= thermal.excess_j.len()
                || b >= thermal.excess_j.len()
                || a == b
                || !g.is_finite()
                || g < 0.
                || !seen.insert((a.min(b), a.max(b)))
            {
                return Err("invalid cell heat conduction link");
            }
        }
        if dt == 0. {
            return Ok(0.);
        }
        let mut candidate = thermal.clone();
        let mut defect = 0.;
        let mut absolute_defect = 0.;
        for &(a, b, g) in links.iter().chain(links.iter().rev()) {
            let temp = |cell: usize| {
                candidate.reference_kelvin[cell]
                    + (candidate.excess_j[cell] + candidate.correction_j[cell])
                        / candidate.capacity_j_per_k[cell]
            };
            let q = crate::heat_exchange::finite_pair_transfer(
                temp(a),
                candidate.capacity_j_per_k[a],
                temp(b),
                candidate.capacity_j_per_k[b],
                g,
                dt * 0.5,
            )?;
            let da = candidate.deposit(a, -q)?;
            let db = candidate.deposit(b, q)?;
            defect += da + db;
            absolute_defect += da.abs() + db.abs();
        }
        if !defect.is_finite()
            || !absolute_defect.is_finite()
            || absolute_defect > energy_tolerance_j
        {
            return Err("cell heat conduction energy defect");
        }
        self.thermal = Some(candidate);
        Ok(defect)
    }
    /// Closed exchange with an owned liquid film. Links are (solid cell,
    /// film cell, conductance W/K), supplied by the contact model. Dry film cells
    /// insulate. Both thermal owners publish together after all pairs pass.
    /// The solid books the opposite of the film's actual representable increment,
    /// and the requested/actual transfer discrepancy also consumes the tolerance.
    /// Returns the signed numerical energy defect, joules.
    /// # Errors
    /// Invalid links/controls, numerical loss or overflow preserves both owners.
    pub fn exchange_maxwell_film_heat(
        &mut self,
        film: &mut crate::surface_film::ThermalFilmMixture,
        links: &[(usize, usize, f64)],
        dt: f64,
        tolerance_j: f64,
    ) -> Result<f64, &'static str> {
        let thermal = self
            .thermal
            .as_ref()
            .ok_or("missing cell thermal inventory")?;
        if !dt.is_finite()
            || dt < 0.
            || (dt > 0. && dt * 0.5 == 0.)
            || !tolerance_j.is_finite()
            || tolerance_j <= 0.
        {
            return Err("invalid solid film heat controls");
        }
        film.temperatures()?;
        let capacities = film.heat_capacities_j_per_k()?;
        let mut seen = std::collections::BTreeSet::new();
        for &(solid, liquid, g) in links {
            if solid >= thermal.excess_j.len()
                || liquid >= capacities.len()
                || !g.is_finite()
                || g < 0.
                || !seen.insert((solid, liquid))
            {
                return Err("invalid solid film heat link");
            }
        }
        if dt == 0. {
            return Ok(0.);
        }
        let mut candidate = thermal.clone();
        let mut film_candidate = film.clone();
        let mut defect = 0.;
        let mut absolute_error = 0.;
        for &(solid, liquid, g) in links.iter().chain(links.iter().rev()) {
            if capacities[liquid] == 0. {
                continue;
            }
            let tf = film_candidate.energies_j()[liquid] / capacities[liquid];
            let ts = candidate.reference_kelvin[solid]
                + (candidate.excess_j[solid] + candidate.correction_j[solid])
                    / candidate.capacity_j_per_k[solid];
            let requested = crate::heat_exchange::finite_pair_transfer(
                ts,
                candidate.capacity_j_per_k[solid],
                tf,
                capacities[liquid],
                g,
                dt * 0.5,
            )?;
            let actual = film_candidate.add_heat_staged_cell(liquid, requested)?;
            let solid_defect = candidate.deposit(solid, -actual)?;
            defect += solid_defect;
            absolute_error += (actual - requested).abs() + solid_defect.abs();
        }
        if !defect.is_finite() || !absolute_error.is_finite() || absolute_error > tolerance_j {
            return Err("solid film heat transfer defect");
        }
        self.thermal = Some(candidate);
        *film = film_candidate;
        Ok(defect)
    }
    /// Cell sensible-energy increments relative to initial temperature, joules.
    /// The initial thermal reference remains immutable; internal compensation
    /// preserves heat increments smaller than the accumulated energy's ulp.
    pub fn maxwell_sensible_energy_j(&self) -> Option<Vec<f64>> {
        self.thermal.as_ref().map(|state| {
            state
                .excess_j
                .iter()
                .zip(&state.correction_j)
                .map(|(sum, tail)| sum + tail)
                .collect()
        })
    }
    /// Derived cell temperatures, Kelvin. No independent temperature state.
    pub fn maxwell_temperatures_kelvin(&self) -> Option<Vec<f64>> {
        self.thermal.as_ref().map(|state| {
            (0..state.excess_j.len())
                .map(|cell| {
                    state.reference_kelvin[cell]
                        + (state.excess_j[cell] + state.correction_j[cell])
                            / state.capacity_j_per_k[cell]
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thermal_inventory_retains_sub_ulp_heat_and_rejects_complete_loss() {
        let mut state = CellThermalState {
            capacity_j_per_k: vec![1.],
            reference_kelvin: vec![300.],
            excess_j: vec![1e16],
            correction_j: vec![0.],
        };
        for _ in 0..10 {
            assert_eq!(state.deposit(0, 1.).unwrap(), 0.);
        }
        assert_eq!((state.excess_j[0] + state.correction_j[0]) - 1e16, 10.);
        state.correction_j[0] = 0.;
        state.deposit(0, 1e-300).unwrap();
        assert_eq!(state.correction_j[0], 1e-300);
        state.correction_j[0] = 1.;
        let before = format!("{state:?}");
        assert!(state.deposit(0, 1e-300).is_err());
        assert_eq!(format!("{state:?}"), before);
    }
}
