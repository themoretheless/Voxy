//! Exact diagnostic oracle. Never used by the production solver.
use super::*;
use num_rational::BigRational;
use num_traits::ToPrimitive;

fn exact(v: f64) -> BigRational {
    BigRational::from_float(v).expect("finite fixture scalar")
}
fn upper_exact(factor: &[f64], rhs: &mut [BigRational], active: std::ops::Range<usize>) {
    for i in (active.start..active.end).rev() {
        let mut value = rhs[i].clone();
        for j in i + 1..(i + direct::BAND).min(active.end) {
            value -= exact(factor[j * direct::BAND + j - i]) * &rhs[j];
        }
        rhs[i] = value / exact(factor[i * direct::BAND]);
    }
}
fn lower_exact(factor: &[f64], rhs: &mut [BigRational], active: std::ops::Range<usize>) {
    for i in active.clone() {
        let mut value = rhs[i].clone();
        for j in active.start.max(i.saturating_sub(direct::BAND - 1))..i {
            value -= exact(factor[i * direct::BAND + i - j]) * &rhs[j];
        }
        rhs[i] = value / exact(factor[i * direct::BAND]);
    }
}
#[test]
fn exact_lower_and_upper_preserve_the_physical_duality_identity() {
    let mut factor = vec![0.; 3 * direct::BAND];
    for (i, diagonal) in [1.25, 0.75, 2.5].into_iter().enumerate() {
        factor[i * direct::BAND] = diagonal;
    }
    factor[direct::BAND + 1] = 0.3;
    factor[2 * direct::BAND + 1] = -0.2;
    factor[2 * direct::BAND + 2] = 0.1;
    let load = vec![exact(0.7), exact(-0.4), exact(0.1)];
    let coordinate = vec![exact(1.) + exact(2f64.powi(-80)), exact(-2.), exact(0.2)];
    let mut column = load.clone();
    lower_exact(&factor, &mut column, 0..3);
    let mut physical = coordinate.clone();
    upper_exact(&factor, &mut physical, 0..3);
    let dot = |a: &[BigRational], b: &[BigRational]| {
        a.iter().zip(b).fold(exact(0.), |sum, (a, b)| sum + a * b)
    };
    assert_eq!(dot(&load, &physical), dot(&column, &coordinate));
}
#[test]
fn exact_upper_retains_a_coordinate_tail_lost_before_backtransform() {
    let mut factor = vec![0.; 2 * direct::BAND];
    factor[0] = 1.;
    factor[direct::BAND] = 1.;
    factor[direct::BAND + 1] = 1.;
    let tail = exact(2f64.powi(-80));
    let mut rhs = vec![exact(1.) + &tail, exact(1.)];
    upper_exact(&factor, &mut rhs, 0..2);
    assert_eq!(rhs[0], tail);
    let mut rounded = vec![1., 1.];
    direct::solve_upper_factored(&factor, &mut rounded, 0..2);
    assert_eq!(rounded[0], 0.);
}

#[test]
#[ignore = "exact rational oracle requires VQC1 physical input and VXE1 captured expansions"]
fn captured_expanded_coordinates_audit_original_physical_gate() {
    let original = std::env::var_os("VOXY_EXPANDED_PHYSICAL_INPUT").expect("VQC1 input");
    let directory = std::env::var_os("VOXY_EXPANDED_FIXTURES").expect("VXE1 directory");
    let (requests, bounds, tolerance) =
        square_root_fixture_tests::read_full_joint_operator(std::path::Path::new(&original));
    let prepared = PreparedNativeJoint::new(&requests, &bounds, tolerance).unwrap();
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "vxe"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    let witness = 2usize;
    assert!(witness < bounds.len());
    let mut exact_column = Vec::new();
    for (request, factor) in requests.iter().zip(&prepared.factors) {
        let mut column: Vec<_> = request.loads[witness].iter().map(|&v| exact(v)).collect();
        lower_exact(factor, &mut column, request.system.active.clone());
        exact_column.extend(column);
    }
    let mut expanded_column = Vec::new();
    for request in &requests {
        let columns = request.whiten_loads_expanded().unwrap();
        expanded_column.extend(columns[witness].iter().map(|coefficient| {
            let [hi, lo] = coefficient.parts();
            exact(hi) + exact(lo)
        }));
    }
    let mut reports = Vec::new();
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"VXE1");
        let u = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        let rows = u(4);
        let width = u(8);
        let sequence = u(12);
        assert_eq!(rows, bounds.len());
        assert_eq!(
            width,
            requests.iter().map(|r| r.system.rhs.len()).sum::<usize>()
        );
        assert_eq!(bytes.len(), 56 + 8 * rows * width + 32 * width + 40 * rows);
        let mut offset = 16;
        let mut scalar = || {
            let v = f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            offset += 8;
            assert!(v.is_finite());
            v
        };
        assert_eq!(scalar().to_bits(), tolerance.to_bits());
        let scales: Vec<_> = (0..4).map(|_| scalar()).collect();
        let effective: Vec<_> = (0..rows).map(|_| scalar()).collect();
        for column in &prepared.columns {
            for &v in column {
                assert_eq!(
                    v.to_bits(),
                    scalar().to_bits(),
                    "canonical whitening changed"
                );
            }
        }
        let coordinate_scale = exact(scales[0]) * exact(scales[1]);
        let coordinates: Vec<_> = (0..width)
            .map(|_| (exact(scalar()) + exact(scalar()) + exact(scalar())) * &coordinate_scale)
            .collect();
        let reaction_scale = exact(scales[2]) * exact(scales[3]);
        let reactions: Vec<_> = (0..rows)
            .map(|_| {
                ((exact(scalar()) + exact(scalar()) + exact(scalar())) * &reaction_scale)
                    .to_f64()
                    .unwrap()
            })
            .collect();
        let legacy: Vec<_> = (0..width).map(|_| scalar()).collect();
        let legacy_reactions: Vec<_> = (0..rows).map(|_| scalar()).collect();
        assert_eq!(offset, bytes.len());
        let mut exact_responses = Vec::new();
        let mut rational_responses = Vec::new();
        let mut rounded_responses = Vec::new();
        let mut start = 0;
        for (request, factor) in requests.iter().zip(&prepared.factors) {
            let end = start + request.system.rhs.len();
            let mut response = coordinates[start..end].to_vec();
            upper_exact(factor, &mut response, request.system.active.clone());
            exact_responses.push(
                response
                    .iter()
                    .map(|v| v.to_f64().expect("finite physical response"))
                    .collect(),
            );
            rational_responses.push(response);
            let mut response = legacy[start..end].to_vec();
            direct::solve_upper_factored(factor, &mut response, request.system.active.clone());
            rounded_responses.push(response);
            start = end;
        }
        // Row 2 is the captured quantization witness. Evaluate its ORIGINAL
        // physical equation before final f64 rounding, without changing the gate.
        assert!(witness < rows);
        let mut exact_gap = -exact(bounds[witness]);
        let mut rounded_gap = -exact(bounds[witness]);
        for ((request, response), rounded) in requests
            .iter()
            .zip(&rational_responses)
            .zip(&exact_responses)
        {
            for ((&load, value), &rounded) in
                request.loads[witness].iter().zip(response).zip(rounded)
            {
                exact_gap += exact(load) * value;
                rounded_gap += exact(load) * exact(rounded);
            }
        }
        let dot = |a: &[BigRational], b: &[BigRational]| {
            a.iter().zip(b).fold(exact(0.), |sum, (a, b)| sum + a * b)
        };
        let stored_column: Vec<_> = prepared.columns[witness]
            .iter()
            .map(|&v| exact(v))
            .collect();
        let exact_coordinate_dot = dot(&exact_column, &coordinates);
        assert_eq!(
            exact_coordinate_dot - exact(bounds[witness]),
            exact_gap,
            "exact lower/upper physical duality"
        );
        let nearest_column: Vec<_> = exact_column
            .iter()
            .map(|v| exact(v.to_f64().unwrap()))
            .collect();
        let nearest_mismatches = nearest_column
            .iter()
            .zip(&stored_column)
            .filter(|(a, b)| a != b)
            .count();
        let unavoidable_rounding =
            dot(&exact_column, &coordinates) - dot(&nearest_column, &coordinates);
        let lower_solve_error =
            dot(&nearest_column, &coordinates) - dot(&stored_column, &coordinates);
        let unavoidable_rounding_view = unavoidable_rounding.to_f64().unwrap();
        let lower_solve_error_view = lower_solve_error.to_f64().unwrap();
        let stored_coordinate_dot = dot(&stored_column, &coordinates);
        let coordinate_residual = &stored_coordinate_dot - exact(effective[witness]);
        let bound_refinement = exact(effective[witness]) - exact(bounds[witness]);
        let whitening_error = dot(&exact_column, &coordinates) - stored_coordinate_dot;
        assert_eq!(
            &coordinate_residual + &bound_refinement + &whitening_error,
            exact_gap,
            "original physical defect decomposition"
        );
        let coordinate_residual_view = coordinate_residual.to_f64().unwrap();
        let bound_refinement_view = bound_refinement.to_f64().unwrap();
        let whitening_error_view = whitening_error.to_f64().unwrap();
        let expanded_whitening_error =
            dot(&exact_column, &coordinates) - dot(&expanded_column, &coordinates);
        let expanded_whitening_error_view = expanded_whitening_error.to_f64().unwrap();
        let precision_gate = exact(tolerance) * exact(2f64.powi(-20));
        assert!(
            expanded_whitening_error >= -precision_gate.clone()
                && expanded_whitening_error <= precision_gate,
            "expanded whitening error exceeds original tolerance / 2^20: {expanded_whitening_error_view:e}"
        );
        eprintln!(
            "EXPANDED WHITENING sequence={sequence} dot_error={expanded_whitening_error_view:e}"
        );
        assert_eq!(unavoidable_rounding + lower_solve_error, whitening_error);
        let before_rounding = exact_gap >= -exact(tolerance)
            && (reactions[witness] == 0. || exact_gap <= exact(tolerance));
        let after_rounding = rounded_gap >= -exact(tolerance)
            && (reactions[witness] == 0. || rounded_gap <= exact(tolerance));
        let exact_gap_view = exact_gap.to_f64().unwrap();
        let rounded_gap_view = rounded_gap.to_f64().unwrap();
        let expanded = HairResponseSystem::validate_joint_solution(
            &requests,
            &bounds,
            &exact_responses,
            &reactions,
            tolerance,
        );
        let rounded = HairResponseSystem::validate_joint_solution(
            &requests,
            &bounds,
            &rounded_responses,
            &legacy_reactions,
            tolerance,
        );
        eprintln!(
            "EXACT BACKTRANSFORM sequence={sequence} expanded={expanded:?} rounded={rounded:?}"
        );
        eprintln!(
            "EXACT PHYSICAL WITNESS row={witness} gap_before_rounding={exact_gap_view:e} gap_after_rounding={rounded_gap_view:e} before_admitted={before_rounding} after_admitted={after_rounding}"
        );
        eprintln!(
            "EXACT DEFECT coordinate_residual={coordinate_residual_view:e} bound_refinement={bound_refinement_view:e} whitening_error={whitening_error_view:e}"
        );
        reports.push(format!("{sequence}\t{:?}\t{:?}\t{exact_gap_view:e}\t{rounded_gap_view:e}\t{before_rounding}\t{after_rounding}\t{coordinate_residual_view:e}\t{bound_refinement_view:e}\t{whitening_error_view:e}\t{nearest_mismatches}\t{unavoidable_rounding_view:e}\t{lower_solve_error_view:e}\t{expanded_whitening_error_view:e}", expanded.err(), rounded.err()));
    }
    if let Some(path) = std::env::var_os("VOXY_EXPANDED_REPORT") {
        std::fs::write(path, reports.join("\n")).unwrap();
    }
}
