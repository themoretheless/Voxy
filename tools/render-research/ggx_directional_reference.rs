//! Native independent slope-space GGX loss reference. Same f64 formulas and
//! tolerances as ggx_directional_reference.py; no GPU/VNDF sampler dependency.
use serde_json::{Value, json};
use std::{
    f64::consts::PI,
    io::{self, Write},
};
fn quadrature(f: &impl Fn(f64) -> f64, l: f64, r: f64, tol: f64) -> (f64, f64) {
    fn refine(
        f: &impl Fn(f64) -> f64,
        l: f64,
        r: f64,
        a: f64,
        b: f64,
        c: f64,
        previous: f64,
        tol: f64,
        depth: u32,
    ) -> (f64, f64) {
        let m = (l + r) / 2.;
        let d = f((l + m) / 2.);
        let e = f((m + r) / 2.);
        let first = (m - l) * (a + 4. * d + b) / 6.;
        let second = (r - m) * (b + 4. * e + c) / 6.;
        let delta = first + second - previous;
        let roundoff = 64. * f64::EPSILON * (first.abs() + second.abs() + previous.abs());
        if delta.abs() <= (15. * tol).max(roundoff) {
            return (first + second + delta / 15., delta.abs() / 15. + roundoff);
        }
        assert!(
            depth > 0,
            "quadrature failed at [{l},{r}] delta={delta} tolerance={tol}"
        );
        let (x, ex) = refine(f, l, m, a, d, b, first, tol / 2., depth - 1);
        let (y, ey) = refine(f, m, r, b, e, c, second, tol / 2., depth - 1);
        (x + y, ex + ey)
    }
    let m = (l + r) / 2.;
    let a = f(l);
    let b = f(m);
    let c = f(r);
    let whole = (r - l) * (a + 4. * b + c) / 6.;
    refine(f, l, r, a, b, c, whole, tol, 56)
}
fn add(sum: &mut f64, correction: &mut f64, value: f64) {
    let increment = value - *correction;
    let next = *sum + increment;
    *correction = (next - *sum) - increment;
    *sum = next;
}
// Gauss-Legendre nodes on [-1,1], generated independently in f64.
fn gauss_legendre(count: usize) -> Vec<(f64, f64)> {
    let mut nodes = vec![(0., 0.); count];
    for i in 0..count.div_ceil(2) {
        let mut z = (PI * (i as f64 + 0.75) / (count as f64 + 0.5)).cos();
        let evaluate = |x: f64| {
            let (mut previous, mut current) = (1., x);
            for k in 2..=count {
                let next =
                    ((2 * k - 1) as f64 * x * current - (k - 1) as f64 * previous) / k as f64;
                previous = current;
                current = next;
            }
            let derivative = count as f64 * (x * current - previous) / (x * x - 1.);
            (current, derivative)
        };
        let mut converged = false;
        for _ in 0..32 {
            let (polynomial, derivative) = evaluate(z);
            let next = z - polynomial / derivative;
            if (next - z).abs() <= 4. * f64::EPSILON {
                z = next;
                converged = true;
                break;
            }
            z = next;
        }
        assert!(converged, "Gauss-Legendre root did not converge");
        let (_, derivative) = evaluate(z);
        let weight = 2. / ((1. - z * z) * derivative * derivative);
        nodes[i] = (-z, weight);
        nodes[count - 1 - i] = (z, weight);
    }
    for degree in 0..=12.min(2 * count - 1) {
        let actual = nodes
            .iter()
            .map(|(x, w)| w * x.powi(degree as i32))
            .sum::<f64>();
        let expected = if degree % 2 == 0 {
            2. / (degree + 1) as f64
        } else {
            0.
        };
        assert!(
            (actual - expected).abs() < 1e-12,
            "Gauss-Legendre polynomial moment failed at degree {degree}"
        );
    }
    nodes
}
fn directional(nv: f64, roughness: f64, angles: usize, warped: bool) -> Value {
    assert!(nv > 0. && nv <= 1. && roughness >= 0. && roughness <= 1. && angles > 0);
    if roughness == 0. {
        return json!({"loss":0.,"null_mass":0.,"masking_loss":0.,"radial_estimated_error":0.,"A":1.-(1.-nv).powi(5),"B":(1.-nv).powi(5),"bias_radial_estimated_error":0.});
    }
    let alpha = roughness * roughness;
    let a2 = alpha * alpha;
    let sv = ((1. - nv) * (1. + nv)).sqrt();
    let x = a2 * (sv / nv).powi(2);
    let lambda_v = x / (2. * ((1. + x).sqrt() + 1.));
    let factor = a2 / (PI * nv * (1. + lambda_v));
    let (mut null_sum, mut null_correction, mut mask_sum, mut mask_correction, mut radial_error) =
        (0., 0., 0., 0., 0.);
    let (mut bias_sum, mut bias_correction, mut bias_error) = (0., 0., 0.);
    // The integrand has no azimuth dependence at normal incidence; reuse it exactly.
    let count = if nv == 1. { 1 } else { angles };
    let gauss = if warped && nv != 1. {
        gauss_legendre(count)
    } else {
        Vec::new()
    };
    for i in 0..count {
        // Gauss-Legendre quadrature is independent of the GPU Simpson rule. The
        // sinh map resolves the narrow feature at phi=pi/2; reflect [0,pi]
        // to the full azimuth because the integrand is even in sin(phi).
        let (cosine, sine, angular_weight) = if warped && nv != 1. {
            let upper = (PI / (2. * nv)).asinh();
            let (node, weight) = gauss[i];
            let u = upper * node;
            let distance = nv * u.sinh();
            (
                -distance.sin(),
                distance.cos(),
                weight * count as f64 * upper * nv * u.cosh() / PI,
            )
        } else {
            let phi = (i as f64 + 0.5) * 2. * PI / count as f64;
            (phi.cos(), phi.sin(), 1.)
        };
        let k = sv * cosine;
        let root = (nv * nv + k * k).sqrt();
        let rc = if k >= 0. {
            (k + root) / nv
        } else {
            nv / (root - k)
        };
        let tail1 = |r: f64| 1. / (2. * (a2 + r * r));
        let tail2 = |r: f64| (alpha / r).atan() / (2. * alpha) + r / (2. * (a2 + r * r));
        let (mut first, mut second) = (tail1(rc), tail2(rc));
        if k < 0. {
            let maximum = -nv / k;
            first -= tail1(maximum);
            second -= tail2(maximum);
        }
        add(
            &mut null_sum,
            &mut null_correction,
            factor * (nv * first + k * second) * angular_weight,
        );
        let upper = (rc / alpha).ln();
        let integrand = |distance: f64| {
            let r = rc * (-distance).exp();
            let vh_numerator = nv + k * r;
            let horizon_distance = -rc * (-distance).exp_m1();
            let light_z = nv * horizon_distance * (r + 1. / rc) / (1. + r * r);
            let loss = if light_z <= 0. {
                1.
            } else {
                let tx = 2. * vh_numerator * r * cosine / (1. + r * r) - sv;
                let ty = 2. * vh_numerator * r * sine / (1. + r * r);
                let y = a2 * (tx * tx + ty * ty) / (light_z * light_z);
                let lam = y / (2. * ((1. + y).sqrt() + 1.));
                lam / (1. + lambda_v + lam)
            };
            factor * vh_numerator * r * r / (a2 + r * r).powi(2) * loss
        };
        // Resolve the arbitrarily thin horizon layer in log(distance).
        // Independent adaptive f64 quadrature; GPU uses fixed Simpson nodes.
        let log_lower = -80.;
        let log_upper = (upper + 20.).ln();
        let partitions = ((log_upper - log_lower) / 2.).ceil() as usize;
        let log_integrand = |t: f64| {
            let d = t.exp();
            integrand(d) * d
        };
        let tolerance = 1e-34f64.max(a2 * 1e-12) / partitions as f64;
        let (mut value, mut error, mut correction) = (0., 0., 0.);
        for j in 0..partitions {
            let l = log_lower + (log_upper - log_lower) * j as f64 / partitions as f64;
            let r = log_lower + (log_upper - log_lower) * (j + 1) as f64 / partitions as f64;
            let (v, e) = quadrature(&log_integrand, l, r, tolerance);
            add(&mut value, &mut correction, v);
            error += e;
        }
        add(&mut mask_sum, &mut mask_correction, value * angular_weight);
        radial_error += error * angular_weight;
        let bias_integrand = |distance: f64| {
            let r = rc * (-distance).exp();
            let vh_numerator = nv + k * r;
            let horizon_distance = -rc * (-distance).exp_m1();
            let light_z = nv * horizon_distance * (r + 1. / rc) / (1. + r * r);
            if light_z <= 0. {
                return 0.;
            }
            let tx = 2. * vh_numerator * r * cosine / (1. + r * r) - sv;
            let ty = 2. * vh_numerator * r * sine / (1. + r * r);
            let y = a2 * (tx * tx + ty * ty) / (light_z * light_z);
            let lambda = y / (2. * ((1. + y).sqrt() + 1.));
            let weight = (1. + lambda_v) / (1. + lambda_v + lambda);
            let norm = (1. + r * r).sqrt();
            let one_minus_vh = ((1. - nv) + r * r / (norm + 1.) - k * r) / norm;
            factor * vh_numerator * r * r / (a2 + r * r).powi(2) * one_minus_vh.powi(5) * weight
        };
        // Fresnel integrand can vanish at both endpoints and the first
        // midpoint. Partition around the interior peak before adapting.
        let mut boundaries = vec![0., upper + 20.];
        for d in [
            0.03125,
            0.0625,
            0.125,
            0.25,
            0.5,
            1.,
            2.,
            4.,
            8.,
            16.,
            upper - 4.,
            upper - 2.,
            upper - 1.,
            upper,
            upper + 1.,
            upper + 2.,
            upper + 4.,
        ] {
            if d > 0. && d < upper + 20. {
                boundaries.push(d);
            }
        }
        boundaries.sort_by(f64::total_cmp);
        boundaries.dedup();
        let tolerance = 1e-34f64.max(a2 * 1e-12) / (boundaries.len() - 1) as f64;
        let (mut bias, mut error, mut correction) = (0., 0., 0.);
        for limits in boundaries.windows(2) {
            let (value, estimate) = quadrature(&bias_integrand, limits[0], limits[1], tolerance);
            add(&mut bias, &mut correction, value);
            error += estimate;
        }
        add(&mut bias_sum, &mut bias_correction, bias * angular_weight);
        bias_error += error * angular_weight;
    }
    let scale = 2. * PI / count as f64;
    let null_mass = null_sum * scale;
    let masking = mask_sum * scale;
    // Bound omitted d in [0,exp(-80)] using r in [rc/2,rc] and the
    // global extrema of rc; the visibility-loss kernel is at most one.
    let rc_min = nv / (1. + sv);
    let rc_max = (1. + sv) / nv;
    let horizon_bound =
        2. * PI * factor * (nv + sv * rc_max) * 4. / (rc_min * rc_min) * (-80f64).exp();
    json!({"omitted_horizon_loss_bound":horizon_bound,"A":1.-null_mass-masking-bias_sum*scale,"B":bias_sum*scale,"bias_radial_estimated_error":bias_error*scale,"loss":null_mass+masking,"null_mass":null_mass,"masking_loss":masking,"radial_estimated_error":radial_error*scale,"omitted_radius_probability_bound":((-40f64).exp()/(1.+(-40f64).exp()))*(nv+sv*alpha*(-20f64).exp())/(nv*(1.+lambda_v))})
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert!(
        args.len() >= 2,
        "provide GPU JSONL; optional existing Python reference JSONL"
    );
    let input = std::fs::read_to_string(&args[1]).unwrap();
    let rows = input
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect::<Vec<_>>();
    for nv in [0.05f64, 0.25, 0.7, 1.] {
        let r = directional(nv, 1., 256, false);
        assert!((r["loss"].as_f64().unwrap() - nv * (1. / nv).ln_1p()).abs() < 1e-8);
    }
    let (normal_b, _) = quadrature(
        &|c: f64| c / (1. + c) * (1. - ((1. + c) / 2.).sqrt()).powi(5),
        0.,
        1.,
        1e-18,
    );
    let normal = directional(1., 1., 16, false);
    assert!(
        (normal["B"].as_f64().unwrap() - normal_b).abs() < 1e-12,
        "independent normal roughness-one B mismatch"
    );
    if let Some(path) = args.get(2) {
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let previous: Value = serde_json::from_str(line).unwrap();
            let Some(nv) = previous["nv"].as_f64() else {
                continue;
            };
            let roughness = previous["roughness"].as_f64().unwrap();
            let current = directional(nv, roughness, 512, false);
            for channel in ["loss", "null_mass", "masking_loss"] {
                let a = current[channel].as_f64().unwrap();
                let b = previous["reference"][channel].as_f64().unwrap();
                assert!(
                    (a - b).abs() <= b.abs() * 1e-12 + 1e-25,
                    "Python/native differential mismatch {nv} {roughness} {channel}: {a} vs {b}"
                );
            }
        }
    }
    let warped = std::env::var_os("VOXY_GGX_REFERENCE_WARPED").is_some();
    let budget = rows
        .iter()
        .filter_map(|r| r["samples"].as_u64())
        .max()
        .unwrap();
    let (mut bias_failures, mut max_bias_relative) = (0usize, 0f64);
    let (mut failures, mut max_relative, mut max_refinement) = (0usize, 0f64, 0f64);
    let mut out = io::BufWriter::new(io::stdout().lock());
    for row in rows {
        if row["samples"].as_u64() != Some(budget) {
            continue;
        }
        let nv = f64::from(row["nv"].as_f64().unwrap() as f32);
        let roughness = f64::from(row["roughness"].as_f64().unwrap() as f32);
        let mut angles = if warped { 64usize } else { 512usize };
        let mut coarse = directional(nv, roughness, angles / 2, warped);
        let mut fine = directional(nv, roughness, angles, warped);
        loop {
            let converged = ["loss", "B"].into_iter().all(|c| {
                let a = fine[c].as_f64().unwrap();
                let b = coarse[c].as_f64().unwrap();
                (a - b).abs() <= 1e-24f64.max(a.abs() * 1e-7)
            });
            if converged {
                break;
            }
            assert!(
                angles < 32768,
                "azimuth reference failed to converge nv={nv} roughness={roughness}"
            );
            angles *= 2;
            coarse = fine;
            fine = directional(nv, roughness, angles, warped);
        }
        let reference = fine["loss"].as_f64().unwrap();
        let loss = row["unit_loss"].as_f64().unwrap();
        let relative = if reference == 0. {
            loss.abs()
        } else {
            (loss - reference).abs() / reference
        };
        let delta = (reference - coarse["loss"].as_f64().unwrap()).abs();
        max_relative = max_relative.max(relative);
        max_refinement = max_refinement.max(delta);
        if relative >= 1e-4 {
            failures += 1;
        }
        let bias = fine["B"].as_f64().unwrap();
        let gpu_bias = row["B"].as_f64().unwrap();
        let bias_relative = if bias == 0. {
            gpu_bias.abs()
        } else {
            (bias - gpu_bias).abs() / bias
        };
        let bias_delta = (bias - coarse["B"].as_f64().unwrap()).abs();
        if bias_relative >= 1e-4 {
            bias_failures += 1;
        }
        max_bias_relative = max_bias_relative.max(bias_relative);
        writeln!(out,"{}",json!({"reference_azimuth_angles":angles,"reference_azimuth_warped":warped,"reference_azimuth_rule":if warped {"Gauss-Legendre"} else {"midpoint"},"bias_azimuth_refinement_difference":bias_delta,"relative_bias_error":bias_relative,"gpu_B":gpu_bias,"nv":nv,"roughness":roughness,"samples":budget,"gpu_loss":loss,"reference":fine,"azimuth_refinement_difference":delta,"relative_loss_error":relative})).unwrap();
        out.flush().unwrap();
    }
    writeln!(out,"{}",json!({"status":if failures>0{"accuracy_gate_failed"}else{"directional_loss_gate_passed"},"failures_at_relative_gate_1e_4":failures,"bias_failures_at_relative_gate_1e_4":bias_failures,"max_relative_bias_error":max_bias_relative,"max_relative_loss_error":max_relative,"max_azimuth_refinement_difference":max_refinement,"scope":"Unit-Fresnel loss and Schlick B; estimated quadrature error is not a certificate; full layered BRDF and wider grid still required"})).unwrap();
    out.flush().unwrap();
    if failures > 0 || bias_failures > 0 {
        std::process::exit(1);
    }
}
