//! Physical ownership of one compliance matrix and its contact load columns.
use super::{HairLinearSystem, direct};
#[path = "contact_square_root_qr.rs"]
mod square_root_qr;
#[path = "contact_square_root_diagnostics.rs"]
mod square_root_diagnostics;
#[cfg(test)]
#[path = "contact_square_root_fixture_tests.rs"]
mod square_root_fixture_tests;

#[derive(Clone, Debug)]
pub struct HairResponseSystem {
    pub system: HairLinearSystem,
    pub loads: Vec<Vec<f64>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hair::{HairLinearSolver, HairRod};
    fn fixture() -> HairResponseSystem {
        let mut system = HairRod::new(
            vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
            Default::default(),
        )
        .unwrap()
        .linear_system(1. / 240.)
        .unwrap();
        for value in &mut system.matrix {
            *value *= (1. / 240.) * (1. / 240.);
        }
        system.rhs.fill(0.);
        let mut loads = vec![vec![0.; 18]; 3];
        loads[0][6] = 1e-7;
        loads[1][14] = -0.3e-7;
        loads[2][6] = 0.2e-7;
        loads[2][14] = 0.7e-7;
        HairResponseSystem { system, loads }
    }
    #[test]
    fn shared_factor_native_responses_match_independent_solves_exactly() {
        let request = fixture();
        let shared = request.solve_native().unwrap();
        for (load, response) in request.loads.iter().zip(shared) {
            let mut independent = request.system.clone();
            independent.rhs = load.clone();
            assert_eq!(response, independent.solve_native().unwrap());
        }
    }
    #[test]
    fn joint_square_root_contact_preserves_paired_reactions_and_fixed_roots() {
        let mut a = fixture();
        a.system.matrix.fill(0.);
        for i in 0..a.system.rhs.len() {
            a.system.matrix[i * direct::BAND] = 1.;
        }
        a.loads = vec![vec![0.; a.system.rhs.len()]];
        a.loads[0][6] = 1.;
        let mut b = a.clone();
        b.loads[0][6] = -1.;
        let requests = [a, b];
        let (response, reactions) =
            HairResponseSystem::solve_joint_load_inequalities_native(&requests, &[1.], 1e-14)
                .unwrap();
        assert!((reactions[0] - 0.5).abs() < 1e-14);
        assert!((response[0][6] - 0.5).abs() < 1e-14);
        assert!((response[1][6] + 0.5).abs() < 1e-14);
        assert!((response[0][6] + response[1][6]).abs() < 1e-14);
        for (request, values) in requests.iter().zip(&response) {
            for fixed in
                (0..request.system.active.start).chain(request.system.active.end..values.len())
            {
                assert_eq!(values[fixed], 0.);
            }
        }
        assert!(
            HairResponseSystem::solve_joint_load_inequalities_native(&requests, &[1., 2.], 1e-14)
                .is_err()
        );
        let (released, reactions) =
            HairResponseSystem::solve_joint_load_inequalities_native(&requests, &[-1.], 1e-14)
                .unwrap();
        assert!(released.iter().flatten().all(|v| *v == 0.));
        assert_eq!(reactions, vec![0.]);
    }
    #[test]
    fn square_root_columns_preserve_compliance_energy_and_fixed_dofs() {
        let request = fixture();
        let columns = request.whiten_loads_native().unwrap();
        let responses = request.solve_native().unwrap();
        for i in 0..columns.len() {
            for fixed in
                (0..request.system.active.start).chain(request.system.active.end..columns[i].len())
            {
                assert_eq!(columns[i][fixed], 0.);
            }
            for j in 0..columns.len() {
                let energy = columns[i]
                    .iter()
                    .zip(&columns[j])
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                let compliance = request.loads[i]
                    .iter()
                    .zip(&responses[j])
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                assert!(
                    (energy - compliance).abs()
                        <= 1e-12 * energy.abs().max(compliance.abs()).max(1e-30)
                );
            }
        }
    }
    #[test]
    fn square_root_retains_a_direction_lost_in_the_rounded_gram() {
        let mut request = fixture();
        request.system.matrix.fill(0.);
        for i in 0..request.system.rhs.len() {
            request.system.matrix[i * direct::BAND] = 1.;
        }
        request.loads = vec![vec![0.; request.system.rhs.len()]; 2];
        request.loads[0][6] = 1.;
        request.loads[1][6] = 1.;
        request.loads[1][7] = 1e-10;
        let columns = request.whiten_loads_native().unwrap();
        let gram = |i: usize, j: usize| {
            columns[i]
                .iter()
                .zip(&columns[j])
                .map(|(a, b)| a * b)
                .sum::<f64>()
        };
        assert_eq!(gram(0, 0) * gram(1, 1) - gram(0, 1) * gram(1, 0), 0.);
        let independent = columns[1]
            .iter()
            .zip(&columns[0])
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>();
        assert!((independent - 1e-20).abs() < 1e-34);
        assert_eq!(columns[1][7], 1e-10);
        let bounds = [1., 1. + 1e-10];
        let response = request
            .solve_load_equalities_native(&bounds, 1e-14)
            .unwrap();
        assert!((response[6] - 1.).abs() < 1e-14);
        assert!((response[7] - (bounds[1] - bounds[0]) / 1e-10).abs() < 1e-12);
        for fixed in
            (0..request.system.active.start).chain(request.system.active.end..response.len())
        {
            assert_eq!(response[fixed], 0.);
        }
    }
    #[test]
    fn square_root_equalities_match_the_physical_minimum_energy_response() {
        let mut request = fixture();
        request.loads.truncate(2);
        let responses = request.solve_native().unwrap();
        let expected: Vec<_> = responses[0]
            .iter()
            .zip(&responses[1])
            .map(|(a, b)| 0.3 * a + 0.7 * b)
            .collect();
        let bounds: Vec<_> = request
            .loads
            .iter()
            .map(|load| load.iter().zip(&expected).map(|(a, b)| a * b).sum::<f64>())
            .collect();
        let actual = request
            .solve_load_equalities_native(&bounds, 1e-14)
            .unwrap();
        let scale = expected.iter().map(|v| v.abs()).fold(0., f64::max);
        for (index, (&a, &b)) in actual.iter().zip(&expected).enumerate() {
            assert!(
                (a - b).abs() <= 1e-12 * scale,
                "index={index} actual={a} expected={b} error={}",
                (a - b).abs()
            );
        }
        let (unilateral, reactions) = request
            .solve_load_inequalities_native(&bounds, 1e-14)
            .unwrap();
        for (&a, &b) in unilateral.iter().zip(&expected) {
            assert!((a - b).abs() <= 1e-12 * scale);
        }
        assert!((reactions[0] - 0.3).abs() < 1e-10);
        assert!((reactions[1] - 0.7).abs() < 1e-10);
        let (released, reactions) = request
            .solve_load_inequalities_native(&[-1., -1.], 1e-14)
            .unwrap();
        assert_eq!(released, vec![0.; request.system.rhs.len()]);
        assert_eq!(reactions, vec![0.; 2]);
        request.loads[1] = request.loads[0].clone();
        assert!(
            request
                .solve_load_equalities_native(&[1., 2.], 1e-14)
                .is_err()
        );
        assert!(request.solve_load_equalities_native(&[1., 2.], 0.).is_err());
    }
    #[test]
    fn response_loads_reject_bad_dimensions_nonfinite_values_and_fixed_forces() {
        let request = fixture();
        let mut bad = request.clone();
        bad.loads[0].pop();
        assert!(bad.validate().is_err());
        assert!(bad.whiten_loads_native().is_err());
        let mut bad = request.clone();
        bad.loads[0][6] = f64::NAN;
        assert!(bad.validate().is_err());
        let mut bad = request.clone();
        bad.loads[0][0] = 1.;
        assert!(bad.validate().is_err());
        assert!(bad.whiten_loads_native().is_err());
        let mut bad = request.clone();
        bad.system.rhs[6] = 1.;
        assert!(bad.validate().is_err());
        assert!(
            request
                .system
                .validate_load_correction(&[0.; 18], &request.loads[0])
                .is_err()
        );
    }
    #[test]
    fn legacy_backend_gets_native_response_default_without_structural_calls() {
        struct Legacy;
        impl HairLinearSolver for Legacy {
            fn solve(&mut self, _: &[HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str> {
                panic!("response default must reuse native factors");
            }
        }
        let request = fixture();
        let expected = request.solve_native().unwrap();
        assert_eq!(Legacy.solve_responses(&[request]).unwrap(), vec![expected]);
    }
}

impl HairResponseSystem {
    /// Joint native projection. Every system supplies one load per global
    /// inequality (zero where uninvolved); factors and DOF ownership stay local.
    pub fn solve_joint_load_inequalities_native(
        requests: &[Self],
        bounds: &[f64],
        absolute_tolerance: f64,
    ) -> Result<(Vec<Vec<f64>>, Vec<f64>), &'static str> {
        Self::solve_joint_load_inequalities_with_coordinates(requests,bounds,absolute_tolerance,true,
            |prepared,bounds,tolerance|prepared.solve_appended(bounds,tolerance))
    }

    // Shared physical owner: alternate QR ordering cannot bypass original
    // whitening refinement, load/force balance or inequality admission.
    fn solve_joint_load_inequalities_with_coordinates(
        requests:&[Self],bounds:&[f64],absolute_tolerance:f64,retry_sorted:bool,
        solve_coordinates:impl Fn(&square_root_qr::NonzeroCoordinates<'_>,&[f64],f64)->Option<(Vec<f64>,Vec<f64>)>,
    )->Result<(Vec<Vec<f64>>,Vec<f64>), &'static str> {
        if !absolute_tolerance.is_finite()
            || absolute_tolerance <= 0.
            || bounds.iter().any(|v| !v.is_finite())
        {
            return Err("invalid joint square-root bounds");
        }
        if requests.is_empty() {
            return if bounds.is_empty() {
                Ok((Vec::new(), Vec::new()))
            } else {
                Err("joint square-root bounds have no systems")
            };
        }
        let mut factors = Vec::with_capacity(requests.len());
        let mut columns = vec![Vec::new(); bounds.len()];
        for request in requests {
            request.validate()?;
            if request.loads.len() != bounds.len() {
                return Err("joint square-root load count mismatch");
            }
            let factor = request.factor_native()?;
            let local = request.whiten_with_factor(&factor)?;
            for (column, values) in columns.iter_mut().zip(local) {
                column.extend(values);
            }
            factors.push(factor);
        }
        let reduced=if columns.is_empty() {None} else {
            Some(square_root_qr::NonzeroCoordinates::new(&columns)
                .ok_or("invalid joint square-root coordinate map")?)
        };
        let attempt=Self::solve_prepared_joint_loads(requests,bounds,absolute_tolerance,&factors,&columns,
            reduced.as_ref(),&solve_coordinates);
        if !retry_sorted || attempt.is_ok() {return attempt;}
        if std::env::var_os("VOXY_HAIR_QR_PROFILE").is_some() {
            eprintln!("HAIR QR SORTED RETRY rows={} reason={}",bounds.len(),attempt.as_ref().unwrap_err());
        }
        Self::solve_prepared_joint_loads(requests,bounds,absolute_tolerance,&factors,&columns,
            reduced.as_ref(),&|prepared,bounds,tolerance|prepared.solve(bounds,tolerance))
    }

    // Prepared factors/columns are immutable and scoped to this exact operator.
    // Each trial has fresh defect bounds and must admit original physical loads.
    fn solve_prepared_joint_loads(
        requests:&[Self],bounds:&[f64],absolute_tolerance:f64,
        factors:&[Vec<f64>],columns:&[Vec<f64>],reduced:Option<&square_root_qr::NonzeroCoordinates<'_>>,
        solve_coordinates:&impl Fn(&square_root_qr::NonzeroCoordinates<'_>,&[f64],f64)->Option<(Vec<f64>,Vec<f64>)>,
    )->Result<(Vec<Vec<f64>>,Vec<f64>), &'static str> {
        let mut effective_bounds = bounds.to_vec();
        // Numerical defect correction for whitening/back-transformation only.
        // Every trial must still pass the ORIGINAL inequalities, nonnegative
        // reactions and original per-system force-balance admission.
        for refinement in 0..8 {
            let (coordinates, reactions) = if bounds.is_empty() {
                (vec![0.; requests.iter().map(|r| r.system.rhs.len()).sum()], Vec::new())
            } else {
                let reduced=reduced.expect("nonempty load columns");
                let started=std::env::var_os("VOXY_HAIR_QR_PROFILE").map(|_|std::time::Instant::now());
                let solution=solve_coordinates(reduced,&effective_bounds, absolute_tolerance);
                if let Some(started)=started {
                    let milliseconds=started.elapsed().as_secs_f64()*1000.;
                    if milliseconds>=10. {
                        square_root_diagnostics::export_profile_input(requests,&columns,bounds,
                            &effective_bounds,absolute_tolerance,refinement);
                        eprintln!("HAIR QR PROFILE systems={} rows={} coordinates={} compact_coordinates={} refinement={refinement} elapsed_ms={milliseconds} admitted_coordinates={}",
                            requests.len(),bounds.len(),columns.first().map_or(0,Vec::len),reduced.coordinate_count(),solution.is_some());
                    }
                }
                match solution {
                    Some(solution)=>solution,
                    None=> {
                        square_root_diagnostics::export_input(requests,&columns,bounds,
                            &effective_bounds,absolute_tolerance,refinement);
                        return Err("joint square-root active contacts did not converge");
                    }
                }
            };
            let mut responses = Vec::with_capacity(requests.len());
            let mut offset = 0;
            for (request, factor) in requests.iter().zip(factors) {
                let end = offset + request.system.rhs.len();
                let mut response = coordinates[offset..end].to_vec();
                direct::solve_upper_factored(factor, &mut response, request.system.active.clone());
                let mut force = vec![0.; response.len()];
                for (load, &reaction) in request.loads.iter().zip(&reactions) {
                    for (value, &axis) in force.iter_mut().zip(load) {
                        *value += reaction * axis;
                    }
                }
                request.system.validate_load_correction(&response, &force)?;
                responses.push(response);
                offset = end;
            }
            let mut failed = None;
            for i in 0..bounds.len() {
                let actual = square_root_qr::accurate_products(requests.iter().zip(&responses)
                    .flat_map(|(r,x)| r.loads[i].iter().copied().zip(x.iter().copied())));
                let gap = actual - bounds[i];
                let reaction = reactions[i];
                if !gap.is_finite() || !reaction.is_finite() || reaction < 0.
                    || if reaction > 0. { gap.abs() > absolute_tolerance }
                        else { gap < -absolute_tolerance } {
                    failed.get_or_insert(i);
                }
                let whitened = square_root_qr::accurate_dot(&columns[i], &coordinates);
                effective_bounds[i] = bounds[i] - (actual - whitened);
            }
            if let Some(i) = failed {
                if refinement == 7 || effective_bounds.iter().any(|v| !v.is_finite()) {
                    square_root_diagnostics::export(requests,&columns,bounds,&coordinates,
                        &reactions,&responses,absolute_tolerance,i);
                    return Err("joint square-root inequality residual failed");
                }
            } else {
                return Ok((responses, reactions));
            }
        }
        unreachable!("bounded refinement returns on its final trial")
    }
    /// Minimum-energy native response to load_i dot displacement >= bound_i.
    /// Returns nonnegative reactions; released inequalities remain admitted.
    pub fn solve_load_inequalities_native(
        &self,
        bounds: &[f64],
        absolute_tolerance: f64,
    ) -> Result<(Vec<f64>, Vec<f64>), &'static str> {
        let (mut responses, reactions) = Self::solve_joint_load_inequalities_native(
            std::slice::from_ref(self),
            bounds,
            absolute_tolerance,
        )?;
        Ok((
            responses
                .pop()
                .ok_or("missing single square-root response")?,
            reactions,
        ))
    }
    /// Minimum-energy displacement satisfying load_i dot displacement=bound_i.
    /// This equality primitive does not choose unilateral active contacts.
    pub fn solve_load_equalities_native(
        &self,
        bounds: &[f64],
        absolute_tolerance: f64,
    ) -> Result<Vec<f64>, &'static str> {
        if bounds.len() != self.loads.len()
            || bounds.iter().any(|v| !v.is_finite())
            || !absolute_tolerance.is_finite()
            || absolute_tolerance <= 0.
        {
            return Err("invalid square-root equality bounds");
        }
        self.validate()?;
        if self.loads.is_empty() {
            return Ok(vec![0.; self.system.rhs.len()]);
        }
        let factor = self.factor_native()?;
        let columns = self.whiten_with_factor(&factor)?;
        let mut result = square_root_qr::minimum_norm(&columns, bounds, absolute_tolerance)
            .ok_or("square-root equalities lack an independent finite basis")?;
        direct::solve_upper_factored(&factor, &mut result, self.system.active.clone());
        if result.iter().any(|v| !v.is_finite()) {
            return Err("square-root equality response overflow");
        }
        for (load, &bound) in self.loads.iter().zip(bounds) {
            let actual = load.iter().zip(&result).map(|(a, b)| a * b).sum::<f64>();
            if !actual.is_finite() || (actual - bound).abs() > absolute_tolerance {
                return Err("square-root equality residual failed");
            }
        }
        Ok(result)
    }

    fn factor_native(&self) -> Result<Vec<f64>, &'static str> {
        let mut factor = self.system.matrix.clone();
        let mut zero = self.system.rhs.clone();
        direct::cholesky(&mut factor, &mut zero, self.system.active.clone());
        if factor.iter().any(|value| !value.is_finite()) {
            return Err("hair compliance factor overflow");
        }
        Ok(factor)
    }

    fn whiten_with_factor(&self, factor: &[f64]) -> Result<Vec<Vec<f64>>, &'static str> {
        self.loads
            .iter()
            .map(|load| {
                let mut column = load.clone();
                direct::solve_lower_factored(factor, &mut column, self.system.active.clone());
                if column.iter().any(|value| !value.is_finite()) {
                    return Err("hair square-root response overflow");
                }
                Ok(column)
            })
            .collect()
    }
    /// Square-root compliance columns L^-1*load for H=L*L^T. Retaining
    /// these columns lets a contact QR solve avoid squaring its condition
    /// number through J*H^-1*J^T. This prepares coordinates, not poses.
    pub fn whiten_loads_native(&self) -> Result<Vec<Vec<f64>>, &'static str> {
        self.validate()?;
        if self.loads.is_empty() {
            return Ok(Vec::new());
        }
        let factor = self.factor_native()?;
        self.whiten_with_factor(&factor)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        self.system.validate_shape()?;
        if self.system.rhs.iter().any(|v| *v != 0.) {
            return Err("hair compliance base RHS must be zero");
        }
        for load in &self.loads {
            if load.len() != self.system.rhs.len() || load.iter().any(|v| !v.is_finite()) {
                return Err("invalid hair contact load");
            }
            if (0..self.system.active.start)
                .chain(self.system.active.end..load.len())
                .any(|i| load[i] != 0.)
            {
                return Err("hair contact load acts on fixed DOFs");
            }
        }
        Ok(())
    }

    /// Factor once and solve every load with the existing physical solver.
    pub fn solve_native(&self) -> Result<Vec<Vec<f64>>, &'static str> {
        self.validate()?;
        if self.loads.is_empty() {
            return Ok(Vec::new());
        }
        let factor = self.factor_native()?;
        self.loads
            .iter()
            .map(|load| {
                let mut response = load.clone();
                direct::solve_factored(&factor, &mut response, self.system.active.clone());
                self.system.validate_load_correction(&response, load)?;
                Ok(response)
            })
            .collect()
    }
}
