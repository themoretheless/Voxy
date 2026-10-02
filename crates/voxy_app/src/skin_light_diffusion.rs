//! Surface-only screened diffusion of irradiance, not volumetric tissue optics.
use std::collections::BTreeMap;

#[derive(Debug)]
struct Timings {
    assembly_ms: f64,
    solve_ms: f64,
    iterations: [usize; 3],
}

pub(super) fn diffuse(
    points: &[[f32; 3]],
    triangles: &[[usize; 3]],
    source: &[[f64; 3]],
    radii_m: [f64; 3],
) -> Result<Vec<[f64; 3]>, &'static str> {
    diffuse_measured(points, triangles, source, radii_m).map(|(result, _)| result)
}

pub(super) fn diffuse_with_initial(
    points: &[[f32; 3]],
    triangles: &[[usize; 3]],
    source: &[[f64; 3]],
    radii_m: [f64; 3],
    initial: Option<&[[f64; 3]]>,
) -> Result<Vec<[f64; 3]>, &'static str> {
    if points.len() != source.len()
        || points.iter().flatten().any(|x| !x.is_finite())
        || source.iter().flatten().any(|x| !x.is_finite() || *x < 0.)
    {
        return Err("invalid surface diffusion input");
    }
    let mut used = vec![false; points.len()];
    for &index in triangles.iter().flatten() {
        *used.get_mut(index).ok_or("invalid diffusion triangle")? = true;
    }
    if used.iter().any(|x| !x) {
        let active: Vec<_> = (0..points.len()).filter(|&i| used[i]).collect();
        let mut remap = vec![usize::MAX; points.len()];
        for (compact, &original) in active.iter().enumerate() {
            remap[original] = compact;
        }
        let compact_points: Vec<_> = active.iter().map(|&i| points[i]).collect();
        let compact_source: Vec<_> = active.iter().map(|&i| source[i]).collect();
        let compact_triangles: Vec<_> = triangles.iter().map(|t| t.map(|i| remap[i])).collect();
        let compact_initial = initial
            .filter(|v| {
                v.len() == points.len() && v.iter().flatten().all(|x| x.is_finite() && *x >= 0.)
            })
            .map(|v| active.iter().map(|&i| v[i]).collect::<Vec<_>>());
        let (solved, _) = diffuse_measured_initial(
            &compact_points,
            &compact_triangles,
            &compact_source,
            radii_m,
            true,
            compact_initial.as_deref(),
        )?;
        // Vertices outside the triangle domain have no surface equation.
        // Preserve their source value while solving the actual surface only.
        let mut result = source.to_vec();
        for (&original, value) in active.iter().zip(solved) {
            result[original] = value;
        }
        return Ok(result);
    }
    diffuse_measured_initial(points, triangles, source, radii_m, true, initial).map(|(v, _)| v)
}

fn diffuse_measured(
    points: &[[f32; 3]],
    triangles: &[[usize; 3]],
    source: &[[f64; 3]],
    radii_m: [f64; 3],
) -> Result<(Vec<[f64; 3]>, Timings), &'static str> {
    diffuse_measured_with_parallel(points, triangles, source, radii_m, true)
}
fn diffuse_measured_with_parallel(
    points: &[[f32; 3]],
    triangles: &[[usize; 3]],
    source: &[[f64; 3]],
    radii_m: [f64; 3],
    parallel: bool,
) -> Result<(Vec<[f64; 3]>, Timings), &'static str> {
    diffuse_measured_initial(points, triangles, source, radii_m, parallel, None)
}
fn diffuse_measured_initial(
    points: &[[f32; 3]],
    triangles: &[[usize; 3]],
    source: &[[f64; 3]],
    radii_m: [f64; 3],
    parallel: bool,
    initial: Option<&[[f64; 3]]>,
) -> Result<(Vec<[f64; 3]>, Timings), &'static str> {
    let initial = initial.filter(|v| {
        v.len() == points.len() && v.iter().flatten().all(|x| x.is_finite() && *x >= 0.)
    });
    if points.len() != source.len()
        || points.iter().flatten().any(|x| !x.is_finite())
        || source.iter().flatten().any(|x| !x.is_finite() || *x < 0.)
        || radii_m.iter().any(|x| !x.is_finite() || *x < 0.)
    {
        return Err("invalid surface diffusion input");
    }
    let started = std::time::Instant::now();
    let mut mass = vec![0.; points.len()];
    let mut weights = BTreeMap::<(usize, usize), f64>::new();
    for t in triangles {
        if t.iter().any(|i| *i >= points.len()) {
            return Err("invalid diffusion triangle");
        }
        let p = t.map(|i| points[i].map(f64::from));
        let ab: [f64; 3] = std::array::from_fn(|k| p[1][k] - p[0][k]);
        let ac: [f64; 3] = std::array::from_fn(|k| p[2][k] - p[0][k]);
        let n = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let area = n.iter().map(|x| x * x).sum::<f64>().sqrt() / 2.;
        if area <= 0. {
            return Err("degenerate diffusion triangle");
        }
        for i in t {
            mass[*i] += area / 3.;
        }
        for k in 0..3 {
            let a = t[k];
            let b = t[(k + 1) % 3];
            let length2 = (0..3)
                .map(|j| (f64::from(points[a][j]) - f64::from(points[b][j])).powi(2))
                .sum::<f64>();
            if length2 <= 0. {
                return Err("zero diffusion edge");
            }
            *weights.entry((a.min(b), a.max(b))).or_default() += 2. * area / (3. * length2);
        }
    }
    if mass.iter().any(|x| *x <= 0.) {
        return Err("unsupported diffusion vertex");
    }
    let edges: Vec<_> = weights.into_iter().map(|((a, b), w)| (a, b, w)).collect();
    let mut timings = Timings {
        assembly_ms: started.elapsed().as_secs_f64() * 1000.,
        solve_ms: 0.,
        iterations: [0; 3],
    };
    let solved = std::time::Instant::now();
    let mut result = source.to_vec();
    let solve_channel = |channel: usize| -> Result<(Vec<f64>, usize), &'static str> {
        let mass = &mass;
        let edges = &edges;
        let mut iterations = 0;
        let lambda = radii_m[channel].powi(2);
        if lambda == 0. {
            return Ok((source.iter().map(|s| s[channel]).collect(), 0));
        }
        let apply = |x: &[f64], y: &mut [f64]| {
            for (value, (m, v)) in y.iter_mut().zip(mass.iter().zip(x)) {
                *value = m * v;
            }
            for &(a, b, w) in edges {
                let flux = lambda * w * (x[a] - x[b]);
                y[a] += flux;
                y[b] -= flux;
            }
        };
        let mut diagonal = mass.clone();
        for &(a, b, w) in edges {
            diagonal[a] += lambda * w;
            diagonal[b] += lambda * w;
        }
        let rhs: Vec<_> = source
            .iter()
            .zip(mass)
            .map(|(s, m)| s[channel] * m)
            .collect();
        let mut x: Vec<_> = initial
            .unwrap_or(source)
            .iter()
            .map(|s| s[channel])
            .collect();
        let mut ax = vec![0.; mass.len()];
        apply(&x, &mut ax);
        let mut residual: Vec<_> = rhs.iter().zip(&ax).map(|(b, a)| b - a).collect();
        let mut z: Vec<_> = residual.iter().zip(&diagonal).map(|(r, d)| r / d).collect();
        let mut direction = z.clone();
        let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        // Resolve well below f32 colour precision even when the warm-start
        // history differs; a loose residual can straddle a rounding boundary.
        let tolerance = dot(&rhs, &rhs) * 1e-28;
        let mut rz = dot(&residual, &z);
        for _ in 0..256 {
            if dot(&residual, &residual) <= tolerance {
                break;
            }
            iterations += 1;
            apply(&direction, &mut ax);
            let ad = &ax;
            let denominator = dot(&direction, ad);
            if denominator <= 0. || !denominator.is_finite() {
                return Err("invalid diffusion system");
            }
            let alpha = rz / denominator;
            for i in 0..x.len() {
                x[i] += alpha * direction[i];
                residual[i] -= alpha * ad[i];
                z[i] = residual[i] / diagonal[i];
            }
            let next = dot(&residual, &z);
            let beta = next / rz;
            for i in 0..x.len() {
                direction[i] = z[i] + beta * direction[i];
            }
            rz = next;
        }
        apply(&x, &mut ax);
        let error: Vec<_> = rhs.iter().zip(&ax).map(|(b, a)| b - a).collect();
        if dot(&error, &error) > tolerance * 4. {
            return Err("surface diffusion did not converge");
        }
        let input: f64 = rhs.iter().sum();
        let output: f64 = x.iter().zip(mass).map(|(v, m)| v * m).sum();
        if (input - output).abs() > input.abs().max(1e-20) * 1e-8
            || x.iter().any(|v| !v.is_finite() || *v < -1e-10)
        {
            return Err("invalid diffusion energy budget");
        }
        Ok((x, iterations))
    };
    let channels = if parallel {
        std::thread::scope(|scope| {
            let solve = &solve_channel;
            let handles: Vec<_> = (0..3)
                .map(|channel| scope.spawn(move || solve(channel)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().map_err(|_| "diffusion worker panicked")?)
                .collect::<Result<Vec<_>, &'static str>>()
        })?
    } else {
        (0..3).map(solve_channel).collect::<Result<Vec<_>, _>>()?
    };
    for (channel, (values, iterations)) in channels.into_iter().enumerate() {
        timings.iterations[channel] = iterations;
        for (r, v) in result.iter_mut().zip(values) {
            r[channel] = v.max(0.);
        }
    }
    timings.solve_ms = solved.elapsed().as_secs_f64() * 1000.;
    Ok((result, timings))
}

#[cfg(test)]
mod tests {
    #[test]
    fn unused_render_vertices_do_not_change_the_surface_solution() {
        let p = [[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.]];
        let s = [[1., 0.2, 0.1], [0.1, 0.5, 0.8], [0.3, 0.4, 0.2]];
        let radii = [0.002, 0.001, 0.0005];
        let reference = super::diffuse_with_initial(&p, &[[0, 1, 2]], &s, radii, None).unwrap();
        let points = [p[0], [9., 9., 9.], p[1], p[2]];
        let source = [s[0], [0.7, 0.8, 0.9], s[1], s[2]];
        let actual =
            super::diffuse_with_initial(&points, &[[0, 2, 3]], &source, radii, Some(&source))
                .unwrap();
        assert_eq!(actual[1], source[1]);
        for (i, j) in [(0, 0), (2, 1), (3, 2)] {
            for c in 0..3 {
                assert!((actual[i][c] - reference[j][c]).abs() < 1e-9);
            }
        }
        assert!(super::diffuse_with_initial(&points, &[[0, 2, 4]], &source, radii, None).is_err());
    }
    use super::*;
    #[test]
    fn warm_start_reconverges_for_changed_shape_and_light() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let triangles = [[0, 1, 2]];
        let initial = diffuse(&points, &triangles, &[[1.; 3], [0.; 3], [0.; 3]], [1.; 3]).unwrap();
        let moved = [[0., 0., 0.], [1.2, 0., 0.], [0., 0.8, 0.]];
        let source = [[0.4; 3], [0.8; 3], [0.1; 3]];
        let cold = diffuse(&moved, &triangles, &source, [0.8; 3]).unwrap();
        let warm =
            diffuse_with_initial(&moved, &triangles, &source, [0.8; 3], Some(&initial)).unwrap();
        for (a, b) in cold.iter().flatten().zip(warm.iter().flatten()) {
            assert!((a - b).abs() < 1e-9);
        }
        assert_eq!(
            cold,
            diffuse_with_initial(
                &moved,
                &triangles,
                &source,
                [0.8; 3],
                Some(&[[f64::NAN; 3]])
            )
            .unwrap()
        );
    }
    #[test]
    fn nonlocal_transport_preserves_energy_and_channel_order() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let source = [[1.; 3], [0.; 3], [0.; 3]];
        let output = diffuse(&points, &[[0, 1, 2]], &source, [1., 0.5, 0.]).unwrap();
        for c in 0..3 {
            assert!((output.iter().map(|v| v[c]).sum::<f64>() - 1.).abs() < 1e-10);
        }
        assert!(output[1][0] > output[1][1] && output[1][1] > output[1][2]);
        assert_eq!(output[0][2], 1.);
        assert!(output.iter().flatten().all(|v| *v >= 0. && *v <= 1.));
    }
    #[test]
    fn constant_light_and_invalid_inputs() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let source = [[0.7; 3]; 3];
        assert_eq!(
            diffuse(&points, &[[0, 1, 2]], &source, [1.; 3]).unwrap(),
            source
        );
        assert!(diffuse(&points, &[[0, 1, 3]], &source, [1.; 3]).is_err());
        assert!(diffuse(&points, &[[0, 1, 2]], &source, [f64::NAN; 3]).is_err());
    }
    #[test]
    fn spatial_units_and_area_weighted_energy_are_consistent() {
        let points = [[0., 0., 0.], [2., 0., 0.], [0., 1., 0.], [2., 1., 0.]];
        let triangles = [[0, 1, 2], [1, 3, 2]];
        let source = [[1.; 3], [0.; 3], [0.; 3], [0.; 3]];
        let reference = diffuse(&points, &triangles, &source, [0.4, 0.2, 0.1]).unwrap();
        for channel in 0..3 {
            let energy = (reference[0][channel]
                + 2. * reference[1][channel]
                + 2. * reference[2][channel]
                + reference[3][channel])
                / 3.;
            assert!((energy - 1. / 3.).abs() < 1e-10);
        }
        for scale in [0.001, 1000.] {
            let scaled: Vec<_> = points.iter().map(|p| p.map(|v| v * scale as f32)).collect();
            let result = diffuse(
                &scaled,
                &triangles,
                &source,
                [0.4 * scale, 0.2 * scale, 0.1 * scale],
            )
            .unwrap();
            for (a, b) in result.iter().flatten().zip(reference.iter().flatten()) {
                assert!((a - b).abs() < 1e-7);
            }
        }
    }
}

#[cfg(test)]
mod benchmark {
    use super::*;
    #[test]
    #[ignore = "release profiling on the imported body"]
    fn actual_body_diffusion_profile() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let points: Vec<_> = body.mesh.vertices().iter().map(|v| v.position).collect();
        let triangles: Vec<_> = body
            .mesh
            .indices()
            .chunks_exact(3)
            .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
            .collect();
        let key = glam::Vec3::new(-0.5, 0.7, 1.0).normalize();
        let fill = glam::Vec3::new(0.8, 0.2, 0.5).normalize();
        let source: Vec<_> = body
            .normals
            .iter()
            .map(|n| {
                let n = glam::Vec3::from_array(n.unwrap_or([0., 1., 0.])).normalize();
                [f64::from(0.2 + 0.6 * n.dot(key).max(0.) + 0.2 * n.dot(fill).max(0.)); 3]
            })
            .collect();
        let mut samples = Vec::new();
        let (serial, serial_timing) = diffuse_measured_with_parallel(
            &points,
            &triangles,
            &source,
            [0.002, 0.001, 0.0005],
            false,
        )
        .unwrap();
        let mut reference = Some(serial);
        for run in 0..8 {
            let (output, t) =
                diffuse_measured(&points, &triangles, &source, [0.002, 0.001, 0.0005]).unwrap();
            if let Some(before) = &reference {
                assert_eq!(before, &output);
            } else {
                reference = Some(output);
            }
            if run > 0 {
                samples.push(serde_json::json!({"assemblyMs":t.assembly_ms,"solveMs":t.solve_ms,"iterations":t.iterations}));
            }
        }
        let report = serde_json::json!({"vertices":points.len(),"triangles":triangles.len(),"samples":samples,"serialSolveMs":serial_timing.solve_ms,"serialAssemblyMs":serial_timing.assembly_ms,"scope":"Static imported-body kernel profile, seven samples after warmup; not GPU or full renderer timing."});
        if let Ok(path) = std::env::var("VOXY_LIGHT_DIFFUSION_REFERENCE") {
            let bits: Vec<u8> = reference
                .unwrap()
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            std::fs::write(path, bits).unwrap();
        }
        if let Ok(path) = std::env::var("VOXY_LIGHT_DIFFUSION_REPORT") {
            std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        }
        println!("{report}");
    }
}
