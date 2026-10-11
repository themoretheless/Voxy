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
    fn same_input_audit_preserves_outputs_and_exports_complete_original_operator() {
        let mut request=fixture();request.system.matrix.fill(0.);
        for i in 0..request.system.rhs.len() {request.system.matrix[i*direct::BAND]=1.;}
        request.loads=vec![vec![0.;18]];request.loads[0][6]=1.;
        let requests=[request];let bounds=[1.];
        let prepared=PreparedNativeJoint::new(&requests,&bounds,1e-14).unwrap();
        let (mut responses,reactions)=prepared.solve(&bounds,1e-14).unwrap();
        let path=std::env::temp_dir().join(format!("voxy-same-input-audit-{}.vqc",std::process::id()));
        assert!(!path.exists(),"preserve previous diagnostic evidence");
        prepared.capture_accelerator_difference(&path,&bounds,1e-14,&responses,&reactions);
        assert!(!path.exists(),"identical solve exported a divergence");
        responses[0][6]+=1e-6;
        let before=(responses.clone(),reactions.clone(),requests[0].system.matrix.clone());
        prepared.capture_accelerator_difference(&path,&bounds,1e-14,&responses,&reactions);
        assert_eq!((&responses,&reactions,&requests[0].system.matrix),(&before.0,&before.1,&before.2));
        let bytes=std::fs::read(&path).unwrap();assert_eq!(&bytes[..4],b"VQC1");
        let mut report_path=path.as_os_str().to_os_string();report_path.push(".comparison.txt");
        let report=std::fs::read_to_string(&report_path).unwrap();
        assert!(report.contains("native_responses="));assert!(report.contains("accelerated_responses="));
        // Preserve prior evidence, including when a caller requests overwrite.
        assert!(!square_root_diagnostics::export_input_to(path.as_os_str(),&requests,
            &prepared.columns,&bounds,&bounds,1e-14,0,true));
        assert_eq!(std::fs::read(&path).unwrap(),bytes);
        std::fs::remove_file(path).unwrap();std::fs::remove_file(report_path).unwrap();
    }
    #[test]
    fn cooperative_whitening_refinement_keeps_original_bounds_and_drops_finished_owners() {
        fn candidate(columns:&[Vec<f64>],bounds:&[f64])->(Vec<f64>,Vec<f64>) {
            let c=columns[0][6];let mut x=vec![0.;columns[0].len()];x[6]=bounds[0]/c;
            let reaction=x[6]/c;(x,vec![reaction])
        }
        struct Backend {rounds:Vec<usize>}
        impl crate::hair::HairLinearSolver for Backend {
            fn solve(&mut self,_:&[HairLinearSystem])->Result<Vec<Vec<f64>>,&'static str> {panic!("unexpected structural solve")}
            fn joint_contact_hints_enabled(&self)->bool {true}
            fn solve_joint_coordinates(&mut self,_:&[Vec<f64>],_:&[f64],_:f64)->Option<(Vec<f64>,Vec<f64>)> {
                panic!("whitening correction escaped cooperative batch")
            }
            fn solve_joint_coordinates_batch(&mut self,requests:&[HairContactCoordinateRequest<'_>])
                ->Option<Vec<(Vec<f64>,Vec<f64>)>> {
                self.rounds.push(requests.len());
                if self.rounds.len()>1 {assert!(requests.iter().all(|r|r.seeds.iter().any(|&v|v>0.)));}
                Some(requests.iter().map(|r|candidate(r.columns,r.bounds)).collect())
            }
        }
        let request=|diagonal,load| {
            let mut r=fixture();r.system.matrix.fill(0.);
            for i in 0..18 {r.system.matrix[i*direct::BAND]=1.;}
            r.system.matrix[6*direct::BAND]=diagonal;r.loads=vec![vec![0.;18]];r.loads[0][6]=load;r
        };
        let a=[request(3.,2e-7)];let b=[request(1.,1.)];let ba=[1.1];let bb=[1.];let tolerance=1e-17;
        let ja=PreparedNativeJoint::new(&a,&ba,tolerance).unwrap();
        let jb=PreparedNativeJoint::new(&b,&bb,tolerance).unwrap();
        let mut serial_calls=0;
        let expected_a=ja.solve_accelerated(&ba,tolerance,|c,b,_| {serial_calls+=1;Some(candidate(c,b))}).unwrap();
        assert!(serial_calls>1,"fixture missed original-load defect correction");
        let expected_b=jb.solve_accelerated(&bb,tolerance,|c,b,_|Some(candidate(c,b))).unwrap();
        let mut seeds=vec![vec![],vec![]];let mut backend=Backend {rounds:Vec::new()};
        let actual=PreparedNativeJoint::solve_accelerated_batch(&[&ja,&jb],&[&ba,&bb],tolerance,&mut seeds,&mut backend).unwrap();
        assert_eq!(actual,vec![expected_a,expected_b]);assert_eq!(backend.rounds[0],2);
        assert!(backend.rounds.len()>1);assert!(backend.rounds[1..].iter().all(|&count|count==1));
        for ((requests,bounds),(responses,reactions,accelerated)) in [(&a[..],&ba[..]),(&b[..],&bb[..])].into_iter().zip(&actual) {
            assert!(*accelerated);HairResponseSystem::validate_joint_solution(requests,bounds,responses,reactions,tolerance).unwrap();
        }
        assert_eq!(ba,[1.1]);assert_eq!(bb,[1.]);
    }
    #[test]
    fn joint_accelerator_owner_admits_original_physics_and_preserves_native_fallback() {
        let mut request=fixture();request.system.matrix.fill(0.);
        for i in 0..request.system.rhs.len() {request.system.matrix[i*direct::BAND]=1.;}
        request.loads=vec![vec![0.;request.system.rhs.len()]];request.loads[0][6]=1.;
        let requests=[request];let bounds=[1.];
        let native=HairResponseSystem::solve_joint_load_inequalities_native(&requests,&bounds,1e-14).unwrap();
        let mut calls=0;
        let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,1e-14,
            |columns,bounds,tolerance| {
                calls+=1;assert_eq!(tolerance,1e-14);assert_eq!(columns[0][6],1.);
                let mut x=vec![0.;18];x[6]=bounds[0];Some((x,vec![bounds[0]]))
            }).unwrap();
        assert!(accelerated);assert_eq!(calls,1);
        HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,1e-14).unwrap();
        for invalid in 0..6 {
            let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,1e-14,
                |_,_,_| {
                    let mut x=vec![0.;18];x[6]=1.;let mut r=vec![1.];
                    match invalid {
                        0=>{x.pop();},1=>{r.clear();},2=>{x[6]=f64::INFINITY;},
                        3=>{r[0]=-1.;},4=>{x[6]=0.;},5=>{x[6]=0.;r[0]=0.;},_=>unreachable!()
                    }
                    Some((x,r))
                }).unwrap();
            assert!(!accelerated,"invalid candidate {invalid} admitted");
            assert_eq!(responses,native.0);assert_eq!(reactions,native.1);
        }
        let unavailable=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,1e-14,|_,_,_|None).unwrap();
        assert!(!unavailable.2);assert_eq!(unavailable.0,native.0);assert_eq!(unavailable.1,native.1);
    }
    #[test]
    fn joint_accelerator_admission_rejects_partial_and_nonphysical_outputs() {
        let mut request=fixture();request.system.matrix.fill(0.);
        for i in 0..request.system.rhs.len() {request.system.matrix[i*direct::BAND]=1.;}
        request.loads=vec![vec![0.;request.system.rhs.len()]];request.loads[0][6]=1.;
        let requests=[request];let bounds=[1.];
        let (response,reactions)=HairResponseSystem::solve_joint_load_inequalities_native(&requests,&bounds,1e-14).unwrap();
        let admit=|x:&[Vec<f64>],r:&[f64]|HairResponseSystem::validate_joint_solution(&requests,&bounds,x,r,1e-14);
        admit(&response,&reactions).unwrap();
        assert!(admit(&[],&reactions).is_err());assert!(admit(&response,&[]).is_err());
        assert!(admit(&response,&[-1.]).is_err());assert!(admit(&response,&[f64::NAN]).is_err());
        let mut bad=response.clone();bad[0].pop();assert!(admit(&bad,&reactions).is_err());
        let mut bad=response.clone();bad[0][6]=f64::INFINITY;assert!(admit(&bad,&reactions).is_err());
        let mut bad=response.clone();bad[0][6]=0.;assert!(admit(&bad,&reactions).is_err());
        let zero=vec![vec![0.;response[0].len()]];
        assert!(admit(&zero,&[0.]).is_err(),"force balance alone cannot admit an unmet bound");
        assert!(HairResponseSystem::validate_joint_solution(&requests,&[-1.],&zero,&[0.],1e-14).is_ok());
    }
    #[test]
    fn exact_load_cache_preserves_bits_bounds_and_operator_identity() {
        let mut request = fixture();
        let mut cache = NativeLoadCache::new(&request).unwrap();
        let check = |cache: &mut NativeLoadCache, request: &HairResponseSystem| {
            let (factor, local) = cache.prepare(request).unwrap();
            let fresh_factor = request.factor_native().unwrap();
            let fresh = request.whiten_with_factor(&fresh_factor).unwrap();
            assert_eq!(factor.len(), fresh_factor.len());
            assert_eq!(local.len(), fresh.len());
            for (a,b) in factor.iter().chain(local.iter().flatten())
                .zip(fresh_factor.iter().chain(fresh.iter().flatten())) {
                assert_eq!(a.to_bits(), b.to_bits());
            }
        };
        check(&mut cache, &request);
        let count = cache.columns.len();
        request.loads.reverse();
        check(&mut cache, &request);
        assert_eq!(cache.columns.len(), count);
        request.loads[0][8] = -0.;
        check(&mut cache, &request);
        assert_eq!(cache.columns.len(), count + 1, "signed zero is an exact cache miss");
        request.loads[0][8] = f64::from_bits(1);
        check(&mut cache, &request);
        assert_eq!(cache.columns.len(), count + 2, "subnormal must not match zero");
        let mut changed = request.clone();
        changed.system.matrix[6 * direct::BAND] *= 1.1;
        assert!(cache.prepare(&changed).is_err());
        changed = request.clone(); changed.loads[0][6] = f64::NAN;
        assert!(cache.prepare(&changed).is_err());
        request.loads = (0..140).map(|i| {
            let mut load = vec![0.; request.system.rhs.len()];
            load[6] = (i as f64 + 1.) * 1e-7; load
        }).collect();
        check(&mut cache, &request);
        assert_eq!(cache.columns.len(), 128);
        assert!(cache.payload_bytes <= 256 * 1024);
        check(&mut cache, &request);
        assert_eq!(cache.columns.len(), 128, "full cache still computes every uncached load");
    }
    #[test]
    fn prepared_joint_reuses_immutable_operator_for_changed_bounds_bitwise() {
        let mut a = fixture();
        a.system.matrix.fill(0.);
        for i in 0..a.system.rhs.len() { a.system.matrix[i * direct::BAND] = 1.; }
        a.loads = vec![vec![0.; a.system.rhs.len()]];
        a.loads[0][6] = 1.;
        let mut b = a.clone(); b.loads[0][6] = -1.;
        let requests = [a, b];
        let prepared = PreparedNativeJoint::new(&requests, &[1.], 1e-14).unwrap();
        let factors = prepared.factors.clone();
        let columns = prepared.columns.clone();
        for bound in [1., -1., 1e-8, 0., 2., -0.5, 0.75] {
            let reused = prepared.solve(&[bound], 1e-14).unwrap();
            let fresh = HairResponseSystem::solve_joint_load_inequalities_native(&requests, &[bound], 1e-14).unwrap();
            for (left, right) in reused.0.iter().flatten().chain(&reused.1)
                .zip(fresh.0.iter().flatten().chain(&fresh.1)) {
                assert_eq!(left.to_bits(), right.to_bits());
            }
            assert_eq!(prepared.factors, factors);
            assert_eq!(prepared.columns, columns);
        }
        assert!(PreparedNativeJoint::new_with_preparation(&requests, &[1.], 1e-14,
            |_, request| Ok((request.factor_native()?, Vec::new()))).is_err());
        assert!(PreparedNativeJoint::new_with_preparation(&requests, &[1.], 1e-14,
            |_, _| Ok((Vec::new(), vec![Vec::new()]))).is_err());
        assert!(prepared.solve(&[f64::NAN], 1e-14).is_err());
        assert!(prepared.solve(&[1., 2.], 1e-14).is_err());
        assert!(prepared.solve(&[1.], 0.).is_err());
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

/// An equality backend may supply a dual direction solely to release an
/// active constraint. Such a direction is never an admitted equality solution.
#[derive(Debug)]
pub enum HairContactEqualityProposal {
    Solution(Vec<f64>,Vec<f64>),
    ReleaseDirection(Vec<f64>),
}

/// One immutable coordinate operator. Hints belong to these exact columns.
/// Returned coordinates still require admission by the original physical owner.
#[derive(Debug)]
pub struct HairContactCoordinateRequest<'a> {
    pub columns: &'a [Vec<f64>],
    pub bounds: &'a [f64],
    pub tolerance: f64,
    pub seeds: &'a [f64],
}

/// A borrowed equality request from an independent paused active-set owner.
/// Replies must retain this request order; operator indices remain stable
/// when other operators finish or retry without hints.
#[derive(Debug)]
pub struct HairContactEqualityRequest<'a> {
    pub operator_index: usize,
    pub columns: &'a [Vec<f64>],
    pub bounds: &'a [f64],
    pub tolerance: f64,
}

impl HairResponseSystem {
    /// Canonical active-set selection with an alternate equality backend.
    /// Returns whitened coordinates only; original physical admission is still
    /// required through solve_joint_load_inequalities_accelerated.
    pub fn solve_contact_coordinates_with_equality_accelerator(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,
        mut equality:impl FnMut(&[Vec<f64>],&[f64],f64)->Option<(Vec<f64>,Vec<f64>)>)->Option<(Vec<f64>,Vec<f64>)> {
        square_root_qr::unilateral_accelerated(columns,bounds,tolerance,&mut equality)
    }
    /// Seed canonical active-set selection with nonnegative dual hints.
    /// Current columns reconstruct the associated primal. Invalid or unusable
    /// hints retry zero-dual selection; all original constraints remain checked.
    /// Like the unseeded coordinate API, ORIGINAL physical admission is required.
    pub fn solve_contact_coordinates_seeded_with_equality_accelerator(
        columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,seeds:&[f64],
        mut equality:impl FnMut(&[Vec<f64>],&[f64],f64)->Option<(Vec<f64>,Vec<f64>)>,
    )->Option<(Vec<f64>,Vec<f64>)> {
        square_root_qr::unilateral_seeded_accelerated(columns,bounds,tolerance,seeds,&mut equality)
    }
    /// Typed release directions can change the active set only. The owner
    /// reconstructs the associated primal from original columns; final KKT
    /// and original physical admission are mandatory before publication.
    pub fn solve_contact_coordinates_with_proposals(
        columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,seeds:&[f64],
        mut equality:impl FnMut(&[Vec<f64>],&[f64],f64)->Option<HairContactEqualityProposal>,
    )->Option<(Vec<f64>,Vec<f64>)> {
        square_root_qr::unilateral_with_proposals(columns,bounds,tolerance,seeds,&mut equality)
    }
    /// Advance independent coordinate operators in cooperative equality rounds.
    /// All inputs are validated before the first backend call. Each round
    /// exposes all ready requests to one backend owner, without worker threads.
    /// A missing per-request proposal retries valid hints once from zero duals;
    /// a failed batch or cold solve rejects the entire output. No partial result
    /// is published. This does not perform original physical admission.
    pub fn solve_contact_coordinate_batch_with_proposals(
        requests: &[HairContactCoordinateRequest<'_>],
        mut equality: impl FnMut(&[HairContactEqualityRequest<'_>])
            -> Option<Vec<Option<HairContactEqualityProposal>>>,
    ) -> Option<Vec<(Vec<f64>, Vec<f64>)>> {
        square_root_qr::unilateral_batch_with_proposals(requests, &mut equality)
    }
    /// Try an accelerator in whitened coordinates through the original physical
    /// owner, including defect correction. Any rejected candidate falls back to
    /// the canonical native solver from the unchanged operator and bounds.
    /// The final bool is true only when accelerator output was physically admitted.
    pub fn solve_joint_load_inequalities_accelerated(
        requests:&[Self],bounds:&[f64],absolute_tolerance:f64,
        accelerator:impl FnMut(&[Vec<f64>],&[f64],f64)->Option<(Vec<f64>,Vec<f64>)>,
    )->Result<(Vec<Vec<f64>>,Vec<f64>,bool),&'static str> {
        let prepared=PreparedNativeJoint::new(requests,bounds,absolute_tolerance)?;
        prepared.solve_accelerated(bounds,absolute_tolerance,accelerator)
    }
    /// Admit a complete joint accelerator candidate against ORIGINAL physical
    /// loads and bounds. Backend precision never changes admission tolerance.
    pub fn validate_joint_solution(
        requests: &[Self], bounds: &[f64], responses: &[Vec<f64>],
        reactions: &[f64], absolute_tolerance: f64,
    ) -> Result<(), &'static str> {
        if !absolute_tolerance.is_finite() || absolute_tolerance <= 0.
            || bounds.iter().any(|v|!v.is_finite()) {
            return Err("invalid joint square-root bounds");
        }
        if responses.len() != requests.len() || reactions.len() != bounds.len()
            || (requests.is_empty() && !bounds.is_empty()) {
            return Err("incomplete joint hair solution");
        }
        if reactions.iter().any(|v|!v.is_finite() || *v < 0.) {
            return Err("invalid joint hair reaction");
        }
        for (request, response) in requests.iter().zip(responses) {
            request.validate()?;
            if request.loads.len() != bounds.len() || response.len() != request.system.rhs.len() {
                return Err("joint hair solution shape mismatch");
            }
            let mut force=vec![0.;response.len()];
            for (load, &reaction) in request.loads.iter().zip(reactions) {
                for (value, &axis) in force.iter_mut().zip(load) { *value += reaction * axis; }
            }
            request.system.validate_load_correction(response, &force)?;
        }
        for i in 0..bounds.len() {
            let actual=square_root_qr::accurate_products(requests.iter().zip(responses)
                .flat_map(|(request,response)|request.loads[i].iter().copied().zip(response.iter().copied())));
            let gap=actual-bounds[i];
            if !gap.is_finite() || if reactions[i]>0. {gap.abs()>absolute_tolerance}
                else {gap < -absolute_tolerance} {
                return Err("joint hair original inequality residual failed");
            }
        }
        Ok(())
    }
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
        let prepared = PreparedNativeJoint::new(requests, bounds, absolute_tolerance)?;
        prepared.solve_with_coordinates(bounds, absolute_tolerance, retry_sorted, solve_coordinates)
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
            let (responses,failed)=Self::admit_prepared_joint_trial(requests,bounds,absolute_tolerance,
                factors,columns,&coordinates,&reactions,&mut effective_bounds)?;
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
    // One physical candidate gate, independent of coordinate scheduling.
    // Retain the exact scalar arithmetic and original immutable load bounds.
    fn admit_prepared_joint_trial(
        requests:&[Self],bounds:&[f64],absolute_tolerance:f64,factors:&[Vec<f64>],columns:&[Vec<f64>],
        coordinates:&[f64],reactions:&[f64],effective_bounds:&mut [f64],
    )->Result<(Vec<Vec<f64>>,Option<usize>),&'static str> {
            let mut responses = Vec::with_capacity(requests.len());
            let mut offset = 0;
            for (request, factor) in requests.iter().zip(factors) {
                let end = offset + request.system.rhs.len();
                let mut response = coordinates[offset..end].to_vec();
                direct::solve_upper_factored(factor, &mut response, request.system.active.clone());
                let mut force = vec![0.; response.len()];
                for (load, &reaction) in request.loads.iter().zip(reactions) {
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
            Ok((responses,failed))
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

// Borrowing requests keeps matrices, DOF ownership and exact original loads
// immutable for every reuse. Each solve still admits fresh original bounds.
pub(super) struct PreparedNativeJoint<'a> {
    requests: &'a [HairResponseSystem],
    factors: Vec<Vec<f64>>,
    columns: Vec<Vec<f64>>,
}
impl<'a> PreparedNativeJoint<'a> {
    pub(super) fn new(requests: &'a [HairResponseSystem], bounds: &[f64], absolute_tolerance: f64)
        -> Result<Self, &'static str> {
        Self::new_with_preparation(requests, bounds, absolute_tolerance, |_, request| {
            let factor = request.factor_native()?;
            let local = request.whiten_with_factor(&factor)?;
            Ok((factor, local))
        })
    }
    pub(super) fn new_with_preparation(requests: &'a [HairResponseSystem], bounds: &[f64], absolute_tolerance: f64,
        mut prepare: impl FnMut(usize, &HairResponseSystem) -> Result<(Vec<f64>, Vec<Vec<f64>>), &'static str>,
    ) -> Result<Self, &'static str> {
        if !absolute_tolerance.is_finite() || absolute_tolerance <= 0.
            || bounds.iter().any(|v| !v.is_finite()) { return Err("invalid joint square-root bounds"); }
        if requests.is_empty() && !bounds.is_empty() {
            return Err("joint square-root bounds have no systems");
        }
        let rows = bounds.len();
        let mut factors = Vec::with_capacity(requests.len());
        let mut columns = vec![Vec::new(); rows];
        for (index, request) in requests.iter().enumerate() {
            request.validate()?;
            if request.loads.len() != rows {
                return Err("joint square-root load count mismatch");
            }
            let (factor, local) = prepare(index, request)?;
            if factor.len() != request.system.matrix.len() || local.len() != rows
                || local.iter().any(|column|column.len() != request.system.rhs.len()) {
                return Err("invalid prepared hair load shape");
            }
            for (column, values) in columns.iter_mut().zip(local) {
                column.extend(values);
            }
            factors.push(factor);
        }
        Ok(Self { requests, factors, columns })
    }
    pub(super) fn solve_accelerated(&self,bounds:&[f64],absolute_tolerance:f64,
        accelerator:impl FnMut(&[Vec<f64>],&[f64],f64)->Option<(Vec<f64>,Vec<f64>)>,
    )->Result<(Vec<Vec<f64>>,Vec<f64>,bool),&'static str> {
        let callback=std::cell::RefCell::new(accelerator);
        let used=std::cell::Cell::new(false);
        let width=self.requests.iter().map(|r|r.system.rhs.len()).sum::<usize>();
        let attempt=self.solve_with_coordinates(bounds,absolute_tolerance,false,
            |_,effective,tolerance| {
                let (coordinates,reactions)=callback.borrow_mut()(&self.columns,effective,tolerance)?;
                if coordinates.len()!=width || reactions.len()!=effective.len()
                    || coordinates.iter().any(|v|!v.is_finite())
                    || reactions.iter().any(|v|!v.is_finite() || *v<0.) {return None;}
                used.set(true);
                Some((coordinates,reactions))
            });
        match attempt {
            Ok((responses,reactions))=> {
                // Opt-in same-input audit separates local numerical error from
                // trajectory branching. It never changes admission or output.
                self.observe_accelerator_difference(bounds,absolute_tolerance,&responses,&reactions);
                Ok((responses,reactions,used.get()))
            },
            Err(_)=>self.recover_accelerated(bounds,absolute_tolerance),
        }
    }
    // Cooperative original-load refinement. Only unfinished physical owners
    // request another coordinate solve; matrices and original bounds never move.
    pub(super) fn solve_accelerated_batch(joints:&[&Self],bounds:&[&[f64]],tolerance:f64,
        seeds:&mut [Vec<f64>],backend:&mut dyn super::HairLinearSolver)
        ->Result<Vec<(Vec<Vec<f64>>,Vec<f64>,bool)>,&'static str> {
        if joints.len()!=bounds.len() || joints.len()!=seeds.len() {return Err("joint physical batch input count mismatch");}
        if !tolerance.is_finite() || tolerance<=0. || bounds.iter().any(|b|b.iter().any(|v|!v.is_finite())) {
            return Err("invalid joint square-root bounds");
        }
        if joints.iter().zip(bounds).any(|(j,b)|j.columns.len()!=b.len()) {return Err("joint square-root load count mismatch");}
        let widths:Vec<_>=joints.iter().map(|j|j.requests.iter().map(|r|r.system.rhs.len()).sum::<usize>()).collect();
        if joints.iter().zip(&widths).any(|(joint,&width)|joint.columns.iter().any(|c|
            c.len()!=width || c.iter().any(|v|!v.is_finite()))) {return Err("invalid joint square-root coordinate map");}
        let mut effective:Vec<_>=bounds.iter().map(|b|b.to_vec()).collect();
        let mut finished:Vec<_>=(0..joints.len()).map(|_|None).collect();
        let use_hints=backend.joint_contact_hints_enabled();
        for i in 0..joints.len() {
            if bounds[i].is_empty() {
                let (responses,reactions)=joints[i].solve(bounds[i],tolerance)?;
                finished[i]=Some((responses,reactions,false));
            }
        }
        for refinement in 0..8 {
            let ready:Vec<_>=(0..joints.len()).filter(|&i|finished[i].is_none()).collect();
            if ready.is_empty() {break;}
            let requests:Vec<_>=ready.iter().map(|&i|HairContactCoordinateRequest {
                columns:&joints[i].columns,bounds:&effective[i],tolerance,
                seeds:if use_hints {&seeds[i]} else {&[]}
            }).collect();
            let started=std::env::var_os("VOXY_HAIR_QR_PROFILE").map(|_|std::time::Instant::now());
            let proposals=backend.solve_joint_coordinates_batch(&requests);
            if let Some(started)=started {
                eprintln!("HAIR JOINT PHYSICAL BATCH PROFILE operators={} refinement={refinement} elapsed_ms={}",ready.len(),started.elapsed().as_secs_f64()*1000.);
            }
            let proposals=match proposals {
                Some(proposals)=> {
                    if proposals.len()!=ready.len() {return Err("joint coordinate batch result count mismatch");}
                    proposals.into_iter().map(Some).collect::<Vec<_>>()
                },
                None=>(0..ready.len()).map(|_|None).collect(),
            };
            for (i,proposal) in ready.into_iter().zip(proposals) {
                let Some((coordinates,reactions))=proposal else {
                    square_root_diagnostics::export_input(joints[i].requests,&joints[i].columns,bounds[i],&effective[i],tolerance,refinement);
                    finished[i]=Some(joints[i].recover_accelerated(bounds[i],tolerance)?);continue;
                };
                if use_hints {seeds[i].clone_from(&reactions);}
                if coordinates.len()!=widths[i] || reactions.len()!=bounds[i].len()
                    || coordinates.iter().any(|v|!v.is_finite()) || reactions.iter().any(|v|!v.is_finite() || *v<0.) {
                    square_root_diagnostics::export_input(joints[i].requests,&joints[i].columns,bounds[i],&effective[i],tolerance,refinement);
                    finished[i]=Some(joints[i].recover_accelerated(bounds[i],tolerance)?);continue;
                }
                match HairResponseSystem::admit_prepared_joint_trial(joints[i].requests,bounds[i],tolerance,
                    &joints[i].factors,&joints[i].columns,&coordinates,&reactions,&mut effective[i]) {
                    Ok((responses,None))=> {
                        joints[i].observe_accelerator_difference(bounds[i],tolerance,&responses,&reactions);
                        finished[i]=Some((responses,reactions,true));
                    },
                    Ok((responses,Some(failed)))=> {
                        if refinement==7 || effective[i].iter().any(|v|!v.is_finite()) {
                            square_root_diagnostics::export(joints[i].requests,&joints[i].columns,bounds[i],&coordinates,
                                &reactions,&responses,tolerance,failed);
                            finished[i]=Some(joints[i].recover_accelerated(bounds[i],tolerance)?);
                        }
                    },
                    Err(_)=>finished[i]=Some(joints[i].recover_accelerated(bounds[i],tolerance)?),
                }
            }
        }
        finished.into_iter().map(|r|r.ok_or("joint square-root inequality residual failed")).collect()
    }
    fn recover_accelerated(&self,bounds:&[f64],absolute_tolerance:f64)
        ->Result<(Vec<Vec<f64>>,Vec<f64>,bool),&'static str> {
                if let Some(path)=std::env::var_os("VOXY_HAIR_ACCELERATOR_REJECTION_EXPORT") {
                    use std::sync::atomic::{AtomicBool,Ordering};
                    static CAPTURED:AtomicBool=AtomicBool::new(false);
                    if CAPTURED.compare_exchange(false,true,Ordering::Relaxed,Ordering::Relaxed).is_ok() {
                        self.capture_observed_input(std::path::Path::new(&path),bounds,absolute_tolerance);
                    }
                }
                self.solve(bounds,absolute_tolerance).map(|(responses,reactions)|(responses,reactions,false))
    }

    fn observe_accelerator_difference(&self,bounds:&[f64],tolerance:f64,
        responses:&[Vec<f64>],reactions:&[f64]) {
        let Some(path)=std::env::var_os("VOXY_HAIR_ACCELERATOR_DIFFERENCE_EXPORT") else {return;};
        self.capture_accelerator_difference(std::path::Path::new(&path),bounds,tolerance,responses,reactions);
    }
    fn capture_accelerator_difference(&self,path:&std::path::Path,bounds:&[f64],tolerance:f64,
        responses:&[Vec<f64>],reactions:&[f64]) {
        let native=match self.solve(bounds,tolerance) {
            Ok(native)=>native,
            Err(error)=> {eprintln!("HAIR SAME INPUT AUDIT native_failed={error}");return;},
        };
        let mut translation=0f64;let mut angular=0f64;
        for (a,b) in native.0.iter().zip(responses) {
            for (i,(a,b)) in a.iter().zip(b).enumerate() {
                if i%6<3 {translation=translation.max((a-b).abs());}
                else {angular=angular.max((a-b).abs());}
            }
        }
        let reaction=native.1.iter().zip(reactions).map(|(a,b)|(a-b).abs()).fold(0f64,f64::max);
        // Observation thresholds only: original physical gates are untouched.
        if translation<=1e-9 && angular<=1e-7 {return;}
        use std::sync::atomic::{AtomicBool,Ordering};
        static CAPTURED:AtomicBool=AtomicBool::new(false);
        if CAPTURED.compare_exchange(false,true,Ordering::Relaxed,Ordering::Relaxed).is_err() {return;}
        if !square_root_diagnostics::export_input_to(path.as_os_str(),self.requests,&self.columns,
            bounds,bounds,tolerance,0,true) {return;}
        let report=format!("scope=same immutable physical operator; not whole trajectory\ntranslation_difference_m={translation:.17e}\nangular_increment_difference_rad={angular:.17e}\nreaction_difference={reaction:.17e}\nabsolute_physical_tolerance={tolerance:.17e}\nnative_responses={:?}\naccelerated_responses={responses:?}\nnative_reactions={:?}\naccelerated_reactions={reactions:?}\n",native.0,native.1);
        let mut report_path=path.as_os_str().to_os_string();report_path.push(".comparison.txt");
        if let Err(error)=std::fs::OpenOptions::new().write(true).create_new(true).open(&report_path)
            .and_then(|mut file| {use std::io::Write;file.write_all(report.as_bytes())}) {
            eprintln!("HAIR SAME INPUT AUDIT export_failed={error}");
        }
        eprintln!("HAIR SAME INPUT AUDIT translation_difference_m={translation} angular_increment_difference_rad={angular} reaction_difference={reaction}");
    }
    pub(super) fn capture_observed_input(&self,path:&std::path::Path,bounds:&[f64],tolerance:f64)->bool {
        square_root_diagnostics::export_input_to(path.as_os_str(),self.requests,&self.columns,bounds,bounds,tolerance,0,true)
    }
    pub(super) fn solve(&self, bounds: &[f64], tolerance: f64)
        -> Result<(Vec<Vec<f64>>, Vec<f64>), &'static str> {
        self.solve_with_coordinates(bounds, tolerance, true,
            |prepared, bounds, tolerance| prepared.solve_appended(bounds, tolerance))
    }
    fn solve_with_coordinates(&self, bounds: &[f64], absolute_tolerance: f64, retry_sorted: bool,
        solve_coordinates: impl Fn(&square_root_qr::NonzeroCoordinates<'_>, &[f64], f64)
            -> Option<(Vec<f64>, Vec<f64>)>,
    ) -> Result<(Vec<Vec<f64>>, Vec<f64>), &'static str> {
        if !absolute_tolerance.is_finite() || absolute_tolerance <= 0.
            || bounds.iter().any(|v| !v.is_finite()) { return Err("invalid joint square-root bounds"); }
        if bounds.len() != self.columns.len() { return Err("joint square-root load count mismatch"); }
        if self.requests.is_empty() { return Ok((Vec::new(), Vec::new())); }
        let reduced=if self.columns.is_empty() {None} else {
            Some(square_root_qr::NonzeroCoordinates::new(&self.columns)
                .ok_or("invalid joint square-root coordinate map")?)
        };
        let attempt=HairResponseSystem::solve_prepared_joint_loads(self.requests,bounds,absolute_tolerance,&self.factors,&self.columns,
            reduced.as_ref(),&solve_coordinates);
        if !retry_sorted || attempt.is_ok() {return attempt;}
        if std::env::var_os("VOXY_HAIR_QR_PROFILE").is_some() {
            eprintln!("HAIR QR SORTED RETRY rows={} reason={}",bounds.len(),attempt.as_ref().unwrap_err());
        }
        HairResponseSystem::solve_prepared_joint_loads(self.requests,bounds,absolute_tolerance,&self.factors,&self.columns,
            reduced.as_ref(),&|prepared,bounds,tolerance|prepared.solve(bounds,tolerance))
    }
}

// One cache per immutable rod operator. Exact bit keys preserve signed zeros
// and subnormal loads; bounded storage affects reuse only, never admission.
pub(super) struct NativeLoadCache {
    matrix: Vec<u64>, rhs: Vec<u64>, active: std::ops::Range<usize>, band_width: usize,
    factor: Vec<f64>, columns: std::collections::HashMap<Vec<u64>, Vec<f64>>,
    payload_bytes: usize,
    #[cfg(test)] computed_columns: usize,
}
impl NativeLoadCache {
    pub(super) fn new(request: &HairResponseSystem) -> Result<Self, &'static str> {
        request.validate()?;
        Ok(Self { matrix: request.system.matrix.iter().map(|v|v.to_bits()).collect(),
            rhs: request.system.rhs.iter().map(|v|v.to_bits()).collect(),
            active: request.system.active.clone(), band_width: request.system.band_width,
            factor: request.factor_native()?, columns: Default::default(), payload_bytes: 0,
            #[cfg(test)] computed_columns: 0 })
    }
    #[cfg(test)]
    pub(super) fn computed_columns(&self)->usize {self.computed_columns}
    pub(super) fn prepare(&mut self, request: &HairResponseSystem)
        -> Result<(Vec<f64>, Vec<Vec<f64>>), &'static str> {
        request.validate()?;
        if self.active != request.system.active || self.band_width != request.system.band_width
            || !self.matrix.iter().copied().eq(request.system.matrix.iter().map(|v|v.to_bits()))
            || !self.rhs.iter().copied().eq(request.system.rhs.iter().map(|v|v.to_bits())) {
            return Err("hair load cache operator identity changed");
        }
        let mut local = Vec::with_capacity(request.loads.len());
        for load in &request.loads {
            let key: Vec<_> = load.iter().map(|v|v.to_bits()).collect();
            if let Some(column) = self.columns.get(&key) { local.push(column.clone()); continue; }
            #[cfg(test)] { self.computed_columns += 1; }
            let mut column = load.clone();
            direct::solve_lower_factored(&self.factor, &mut column, self.active.clone());
            if column.iter().any(|v|!v.is_finite()) { return Err("hair square-root response overflow"); }
            let bytes = load.len().saturating_mul(16);
            if self.columns.len() < 128 && self.payload_bytes.saturating_add(bytes) <= 256 * 1024 {
                self.columns.insert(key, column.clone()); self.payload_bytes += bytes;
            }
            local.push(column);
        }
        Ok((self.factor.clone(), local))
    }
}
