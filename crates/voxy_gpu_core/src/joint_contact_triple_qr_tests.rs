use super::*;
fn shader() -> String {
    format!(
        "{}\n{}",
        EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,
        include_str!("joint_contact_triple_qr.wgsl")
    )
}
// Accumulate the bound with product residuals before rounding the cancellation.
// Qualification helper: finite non-underflowing fixtures, not a physical owner.
pub(super) fn compensated_gap(a: &[f64], b: &[f64], bound: f64) -> f64 {
    fn grow(expansion: &mut Vec<f64>, mut value: f64) {
        let mut next = Vec::with_capacity(expansion.len() + 1);
        for &term in expansion.iter() {
            let sum = value + term;
            let virtual_term = sum - value;
            let error = (value - (sum - virtual_term)) + (term - virtual_term);
            if error != 0. {
                next.push(error);
            }
            value = sum;
        }
        if value != 0. || next.is_empty() {
            next.push(value);
        }
        *expansion = next;
    }
    let mut expansion = vec![-bound];
    for (&a, &b) in a.iter().zip(b) {
        let product = a * b;
        grow(&mut expansion, a.mul_add(b, -product));
        grow(&mut expansion, product);
    }
    expansion.iter().copied().sum()
}
#[test]
fn triple_residual_checker_preserves_bound_cancellation() {
    let tiny = 2f64.powi(-54);
    assert_eq!(compensated_gap(&[1., tiny], &[1., 1.], 1.), tiny);
    assert_eq!(compensated_gap(&[1e16, 1., -1e16], &[1., 1., 1.], 1.), 0.);
}
#[test]
fn triple_qr_shader_validates() {
    let module = naga::front::wgsl::parse_str(&shader()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}
fn compensated_dot(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0.;
    let mut correction = 0.;
    for (&a, &b) in a.iter().zip(b) {
        let product = a * b;
        let error = a.mul_add(b, -product);
        let value = product - correction;
        let next = sum + value;
        correction = (next - sum) - value - error;
        sum = next;
    }
    sum - correction
}
#[test]
#[ignore = "requires actual GPU and original rejected operator JSON; experimental QR only"]
fn gpu_triple_qr_original_active_operator() {
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let decode_bits = |v: &serde_json::Value| f64::from_bits(v.as_u64().unwrap());
    let original = fixture["columns_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(decode_bits)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let original_bounds = fixture["bounds_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(decode_bits)
        .collect::<Vec<_>>();
    let active = fixture["reactions_bits"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (decode_bits(v) > 0.).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(active, [2, 26, 27, 48, 53, 60]);
    let columns = active
        .iter()
        .map(|&i| original[i].clone())
        .collect::<Vec<_>>();
    let bounds = active
        .iter()
        .map(|&i| original_bounds[i])
        .collect::<Vec<_>>();
    let tolerance = decode_bits(&fixture["tolerance_bits"]);
    let input = ExperimentalTripleContactInput::new(&columns, &bounds, tolerance).unwrap();
    let count = input.rows;
    let width = input.width;
    let mut words = vec![count as u32, width as u32];
    words.extend(input.columns.iter().flatten().copied());
    words.extend(input.bounds.iter().flatten().copied());
    let q = words.len();
    let r = q + 3 * count * width;
    let state = r + 3 * count * count;
    let reaction = state + 3 * width;
    let status = reaction + 3 * count;
    words.resize(status + 1, 0);
    let output = dispatch(&words);
    assert_eq!(&output[..q], &words[..q]);
    assert_eq!(output[status], 0, "QR candidate rejected");
    let decode = |offset| {
        let a = f32::from_bits(output[offset]) as f64;
        let b = f32::from_bits(output[offset + 1]) as f64;
        let c = f32::from_bits(output[offset + 2]) as f64;
        let sum = a + b;
        let v = sum - a;
        sum + ((a - (sum - v)) + (b - v) + c)
    };
    let basis = (0..count)
        .map(|i| {
            (0..width)
                .map(|j| decode(q + 3 * (i * width + j)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut orthogonal = 0f64;
    let mut reconstruction = 0f64;
    for i in 0..count {
        for j in 0..count {
            orthogonal = orthogonal
                .max((compensated_dot(&basis[i], &basis[j]) - if i == j { 1. } else { 0. }).abs());
        }
        for coord in 0..width {
            let coefficients = (0..count)
                .map(|j| decode(r + 3 * (j * count + i)))
                .collect::<Vec<_>>();
            let values = (0..count).map(|j| basis[j][coord]).collect::<Vec<_>>();
            reconstruction = reconstruction.max(
                (compensated_dot(&coefficients, &values) - columns[i][coord] / input.column_scale)
                    .abs(),
            );
        }
    }
    let motion = (0..width)
        .map(|i| decode(state + 3 * i) * input.bound_scale / input.column_scale)
        .collect::<Vec<_>>();
    let lambda = (0..count)
        .map(|i| {
            (decode(reaction + 3 * i) * input.bound_scale / input.column_scale) / input.column_scale
        })
        .collect::<Vec<_>>();
    let worst_gap = original
        .iter()
        .zip(&original_bounds)
        .map(|(column, bound)| -compensated_gap(column, &motion, *bound))
        .fold(0f64, f64::max);
    let equality = columns
        .iter()
        .zip(&bounds)
        .map(|(column, bound)| (-compensated_gap(column, &motion, *bound)).abs())
        .fold(0f64, f64::max);
    eprintln!(
        "TRIPLE ORIGINAL QR active={active:?} width={width} orthogonality={orthogonal} reconstruction={reconstruction} original_worst_gap={worst_gap} active_equality={equality} min_reaction={} tolerance={tolerance} scope=fixed_active_QR_not_full_active_set_or_physical_admission",
        lambda.iter().copied().fold(f64::INFINITY, f64::min)
    );
    assert!(orthogonal <= 16. * f64::EPSILON);
    assert!(reconstruction <= 16. * f64::EPSILON);
    assert!(lambda.iter().all(|v| v.is_finite() && *v >= 0.));
    assert!(motion.iter().all(|v| v.is_finite()));
    assert!(equality <= tolerance);
    assert!(
        worst_gap <= tolerance,
        "original inequalities reject candidate"
    );
}

fn dispatch(words: &[u32]) -> Vec<u32> {
    dispatch_shader(words, &shader())
}
fn dispatch_shader(words: &[u32], source: &str) -> Vec<u32> {
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(crate::ComputeProgram::new(&device, source)).unwrap();
    let job = program
        .create_job(&device, bytemuck::cast_slice(words))
        .unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    job.encode_step(&mut encoder, [1, 1, 1]).unwrap();
    let snapshot = job.encode_snapshot(&mut encoder).unwrap();
    let submission = queue.submit([encoder.finish()]);
    let mut read = snapshot.begin_read();
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    let bytes = read.try_read().unwrap().unwrap();

    bytes
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect::<Vec<_>>()
}

#[test]
#[ignore = "requires actual GPU; exact dependence and tiny independent directions"]
fn gpu_triple_qr_rejects_exact_dependencies_without_rank_cutoff() {
    let cases = [
        (vec![vec![1., 0.], vec![1., 0.]], 8),
        (vec![vec![1., 0.], vec![-1., 0.]], 8),
        (vec![vec![1., 0.], vec![0., 0.]], 2),
        (vec![vec![1., 0.], vec![1., 1e-12]], 0),
    ];
    for (columns, expected_status) in cases {
        let input = ExperimentalTripleContactInput::new(&columns, &[1., 1.], 1e-14).unwrap();
        let count = input.rows;
        let width = input.width;
        let mut words = vec![count as u32, width as u32];
        words.extend(input.columns.iter().flatten().copied());
        words.extend(input.bounds.iter().flatten().copied());
        let immutable = words.len();
        let status = immutable + 3 * count * width + 3 * count * count + 3 * width + 3 * count;
        words.resize(status + 1, 0);
        let output = dispatch(&words);
        assert_eq!(&output[..immutable], &words[..immutable]);
        assert_eq!(output[status], expected_status, "columns={columns:?}");
    }
    eprintln!(
        "TRIPLE QR DEPENDENCE duplicate=reject opposite=reject zero=reject independent_angle_1e_minus12=accepted no_rank_threshold=true scope=qualification_only"
    );
}

fn active_shader() -> String {
    if std::env::var_os("VOXY_TRIPLE_TEST_PARALLEL").is_some() {
        ExperimentalTripleActiveContactInput::parallel_shader_source()
    } else {
        ExperimentalTripleActiveContactInput::shader_source()
    }
}
#[test]
fn triple_active_shader_validates() {
    let module = naga::front::wgsl::parse_str(&active_shader()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}
#[test]
#[ignore = "requires actual GPU and original rejected operator; experimental automatic active set"]
fn gpu_triple_automatic_original_operator() {
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let decode_bits = |v: &serde_json::Value| f64::from_bits(v.as_u64().unwrap());
    let columns = fixture["columns_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(decode_bits)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let bounds = fixture["bounds_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(decode_bits)
        .collect::<Vec<_>>();
    let tolerance = decode_bits(&fixture["tolerance_bits"]);
    let input = ExperimentalTripleContactInput::new(&columns, &bounds, tolerance).unwrap();
    let count = input.rows;
    let width = input.width;
    let mut words = vec![count as u32, width as u32, 4 * (count * count) as u32, 0];
    words.extend(input.columns.iter().flatten().copied());
    words.extend(input.bounds.iter().flatten().copied());
    words.extend(input.tolerance);
    let immutable = words.len();
    let q = immutable;
    let r = q + 3 * count * width;
    let candidate = r + 3 * count * count;
    let candidate_reactions = candidate + 3 * width;
    let state = candidate_reactions + 3 * count;
    let reactions = state + 3 * width;
    let status = reactions + 3 * count;
    words.resize(status + 1, 0);
    let typed = ExperimentalTripleActiveContactInput::new(&columns, &bounds, tolerance).unwrap();
    words = typed
        .bytes()
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect();
    let output = dispatch_shader(&words, &active_shader());
    let (typed_motion, typed_reactions) = typed.candidate(bytemuck::cast_slice(&output)).unwrap();
    assert_eq!(&output[..3], &words[..3]);
    assert_eq!(&output[4..immutable], &words[4..immutable]);
    eprintln!(
        "TRIPLE AUTOMATIC status={} iterations={} rows={count} width={width}",
        output[status], output[3]
    );
    assert_eq!(output[status], 0, "automatic active set rejected");
    let decode = |offset| {
        let a = f32::from_bits(output[offset]) as f64;
        let b = f32::from_bits(output[offset + 1]) as f64;
        let c = f32::from_bits(output[offset + 2]) as f64;
        let sum = a + b;
        let v = sum - a;
        sum + ((a - (sum - v)) + (b - v) + c)
    };
    let motion = (0..width)
        .map(|i| decode(state + 3 * i) * input.bound_scale / input.column_scale)
        .collect::<Vec<_>>();
    let lambda = (0..count)
        .map(|i| {
            (decode(reactions + 3 * i) * input.bound_scale / input.column_scale)
                / input.column_scale
        })
        .collect::<Vec<_>>();
    assert_eq!(typed_motion, motion);
    assert_eq!(typed_reactions, lambda);
    let mut worst = 0f64;
    let mut equality = 0f64;
    let mut active = Vec::new();
    for row in 0..count {
        let gap = compensated_gap(&columns[row], &motion, bounds[row]);
        worst = worst.max(-gap);
        assert!(lambda[row].is_finite() && lambda[row] >= 0.);
        if lambda[row] > 0. {
            active.push(row);
            equality = equality.max(gap.abs());
        }
    }
    assert!(motion.iter().all(|v| v.is_finite()));
    eprintln!(
        "TRIPLE AUTOMATIC ORIGINAL active={active:?} worst_gap={worst} equality={equality} tolerance={tolerance} scope=GPU_candidate_not_native_physical_admission_or_FPS"
    );
    assert!(
        worst <= tolerance && equality <= tolerance,
        "original-coordinate contact gate rejected"
    );
}

#[test]
#[ignore = "requires actual GPU; automatic solver rejection and release paths"]
fn gpu_triple_automatic_rejections_and_release() {
    let cases = [
        (
            "exhaustion before work",
            vec![vec![1.]],
            vec![1.],
            0u32,
            14u32,
            None,
        ),
        (
            "exhaustion after enter",
            vec![vec![1.]],
            vec![1.],
            1,
            14,
            None,
        ),
        (
            "contradictory opposite",
            vec![vec![1.], vec![-1.]],
            vec![1., 1.],
            16,
            8,
            None,
        ),
        (
            "violated zero row",
            vec![vec![0.], vec![1.]],
            vec![1., 0.],
            16,
            2,
            None,
        ),
        (
            "inactive constraints",
            vec![vec![1.]],
            vec![-1.],
            1,
            0,
            Some(vec![0.]),
        ),
        (
            "redundant stricter row",
            vec![vec![1.], vec![1.]],
            vec![1., 2.],
            16,
            0,
            Some(vec![2.]),
        ),
        (
            "release negative reaction",
            vec![vec![1., 0.], vec![2., 1.]],
            vec![2., 3.],
            16,
            0,
            Some(vec![2., 0.]),
        ),
    ];
    for (name, columns, bounds, limit, expected_status, expected_motion) in cases {
        let input = ExperimentalTripleContactInput::new(&columns, &bounds, 1e-14).unwrap();
        let count = input.rows;
        let width = input.width;
        let mut words = vec![count as u32, width as u32, limit, 0];
        words.extend(input.columns.iter().flatten().copied());
        words.extend(input.bounds.iter().flatten().copied());
        words.extend(input.tolerance);
        let immutable = words.len();
        let state = immutable + 3 * count * width + 3 * count * count + 3 * width + 3 * count;
        let reactions = state + 3 * width;
        let status = reactions + 3 * count;
        words.resize(status + 1, 0);
        let output = dispatch_shader(&words, &active_shader());
        assert_eq!(&output[..3], &words[..3]);
        assert_eq!(&output[4..immutable], &words[4..immutable]);
        let expected_status = if std::env::var_os("VOXY_TRIPLE_TEST_PARALLEL").is_some()
            && matches!(expected_status, 2 | 8)
        {
            0x104
        } else {
            expected_status
        };
        assert_eq!(
            output[status], expected_status,
            "{name}: iterations={}",
            output[3]
        );
        assert!(output[3] <= limit);
        if let Some(expected) = expected_motion {
            let decode = |offset| {
                let a = f32::from_bits(output[offset]) as f64;
                let b = f32::from_bits(output[offset + 1]) as f64;
                let c = f32::from_bits(output[offset + 2]) as f64;
                let sum = a + b;
                let v = sum - a;
                sum + ((a - (sum - v)) + (b - v) + c)
            };
            let motion = (0..width)
                .map(|i| decode(state + 3 * i) * input.bound_scale / input.column_scale)
                .collect::<Vec<_>>();
            assert_eq!(motion, expected, "{name}");
            for row in 0..count {
                let reaction = (decode(reactions + 3 * row) * input.bound_scale
                    / input.column_scale)
                    / input.column_scale;
                assert!(reaction >= 0. && reaction.is_finite(), "{name}");
                let gap = compensated_gap(&columns[row], &motion, bounds[row]);
                assert!(gap >= -1e-14, "{name}");
                if reaction > 0. {
                    assert!(gap.abs() <= 1e-14, "{name}");
                }
            }
            if name == "release negative reaction" {
                assert!(output[3] >= 4, "release path was not exercised");
                assert_eq!(decode(reactions + 3), 0.);
            }
        }
        eprintln!(
            "TRIPLE AUTOMATIC CASE name={name:?} status={} iterations={} scope=qualification_only",
            output[status], output[3]
        );
    }
}

#[test]
fn triple_parallel_shader_validates() {
    let source = ExperimentalTripleActiveContactInput::parallel_shader_source();
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
#[ignore = "requires actual GPU and original operator JSON; warm hints remain candidates"]
fn gpu_triple_seeded_original_operator() {
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let value = |v: &serde_json::Value| f64::from_bits(v.as_u64().unwrap());
    let columns = fixture["columns_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap().iter().map(value).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let bounds = fixture["bounds_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(value)
        .collect::<Vec<_>>();
    let tolerance = value(&fixture["tolerance_bits"]);
    let source = ExperimentalTripleActiveContactInput::parallel_shader_source();
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(crate::ComputeProgram::new(&device, &source)).unwrap();
    let solve = |input: &ExperimentalTripleActiveContactInput| {
        let job = program.create_job(&device, input.bytes()).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder, [1, 1, 1]).unwrap();
        let snapshot = job.encode_snapshot(&mut encoder).unwrap();
        let submission = queue.submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        read.try_read()
            .unwrap()
            .unwrap()
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>()
    };
    let cold = ExperimentalTripleActiveContactInput::new(&columns, &bounds, tolerance).unwrap();
    let cold_output = solve(&cold);
    let (cold_motion, seeds) = cold.candidate(bytemuck::cast_slice(&cold_output)).unwrap();
    let warm =
        ExperimentalTripleActiveContactInput::new_seeded(&columns, &bounds, tolerance, &seeds)
            .unwrap();
    let warm_output = solve(&warm);
    let (motion, reactions) = warm.candidate(bytemuck::cast_slice(&warm_output)).unwrap();
    for row in 0..bounds.len() {
        let gap = compensated_gap(&columns[row], &motion, bounds[row]);
        assert!(gap >= -tolerance);
        if reactions[row] > 0. {
            assert!(gap.abs() <= tolerance);
        }
    }
    assert_eq!(warm_output[3], 1);
    assert!(cold_output[3] > warm_output[3]);
    let completed = cold
        .completed_parallel_factors(bytemuck::cast_slice(&cold_output))
        .unwrap();
    let mut restored =
        ExperimentalTripleActiveContactInput::new_seeded(&columns, &bounds, tolerance, &seeds)
            .unwrap();
    restored
        .reuse_completed_parallel_factors(&completed)
        .unwrap();
    let restored_output = solve(&restored);
    let restored_candidate = restored
        .candidate(bytemuck::cast_slice(&restored_output))
        .unwrap();
    assert_eq!(restored_output[3], 1);
    assert_eq!(
        restored_candidate,
        (cold_motion, seeds.clone()),
        "restored final basis changed the completed cold solution bits"
    );
    for row in 0..bounds.len() {
        let gap = compensated_gap(&columns[row], &restored_candidate.0, bounds[row]);
        assert!(gap >= -tolerance);
        if restored_candidate.1[row] > 0. {
            assert!(gap.abs() <= tolerance);
        }
    }
    assert!(
        restored
            .parallel_factors(bytemuck::cast_slice(&restored_output))
            .unwrap()
            .same_basis(&completed)
    );
    eprintln!(
        "TRIPLE COMPLETED BASIS cold_iterations={} restored_iterations=1 final_order_preserved=true original_coordinate_gate=true scope=renderer_full_readback_not_native_physical_admission",
        cold_output[3]
    );
    let factors = warm
        .parallel_factors(bytemuck::cast_slice(&warm_output))
        .unwrap();
    let changed_bounds = bounds.iter().map(|v| v * 1.000001).collect::<Vec<_>>();
    let mut reused = ExperimentalTripleActiveContactInput::new_seeded(
        &columns,
        &changed_bounds,
        tolerance,
        &seeds,
    )
    .unwrap();
    reused.reuse_parallel_factors(&factors).unwrap();
    let reused_output = solve(&reused);
    let (motion, reactions) = reused
        .candidate(bytemuck::cast_slice(&reused_output))
        .unwrap();
    for row in 0..bounds.len() {
        let gap = compensated_gap(&columns[row], &motion, changed_bounds[row]);
        assert!(gap >= -tolerance);
        if reactions[row] > 0. {
            assert!(
                gap.abs() <= tolerance,
                "factor row={row} gap={gap:e} tolerance={tolerance:e}"
            );
        }
    }
    assert_eq!(reused_output[3], 1);
    let fresh = ExperimentalTripleActiveContactInput::new_seeded(
        &columns,
        &changed_bounds,
        tolerance,
        &seeds,
    )
    .unwrap();
    let fresh_output = solve(&fresh);
    let fresh_candidate = fresh
        .candidate(bytemuck::cast_slice(&fresh_output))
        .unwrap();
    assert_eq!(
        (&motion, &reactions),
        (&fresh_candidate.0, &fresh_candidate.1)
    );
    if std::env::var_os("VOXY_TRIPLE_FACTOR_BENCH").is_some() {
        for pair in 0..8 {
            let mut times = [0.; 2];
            for index in if pair % 2 == 0 { [0, 1] } else { [1, 0] } {
                let input = if index == 0 { &fresh } else { &reused };
                let started = std::time::Instant::now();
                let output = solve(input);
                times[index] = started.elapsed().as_secs_f64() * 1000.;
                assert_eq!(
                    input.candidate(bytemuck::cast_slice(&output)).unwrap(),
                    fresh_candidate
                );
            }
            eprintln!(
                "FACTOR_BENCH pair={pair} fresh_ms={} reused_ms={} scope=allocation_upload_dispatch_readback_wall_time_no_pipeline_creation_not_FPS",
                times[0], times[1]
            );
        }
    }
    let mut extended_columns = columns.clone();
    extended_columns.push(vec![0.; columns[0].len()]);
    let mut extended_bounds = changed_bounds.clone();
    extended_bounds.push(-1.);
    let mut extended_seeds = seeds.clone();
    extended_seeds.push(0.);
    let mut extended = ExperimentalTripleActiveContactInput::new_seeded(
        &extended_columns,
        &extended_bounds,
        tolerance,
        &extended_seeds,
    )
    .unwrap();
    extended.reuse_parallel_factors(&factors).unwrap();
    let extended_output = solve(&extended);
    let (extended_motion, extended_reactions) = extended
        .candidate(bytemuck::cast_slice(&extended_output))
        .unwrap();
    assert_eq!(extended_motion, motion);
    assert_eq!(&extended_reactions[..reactions.len()], &reactions);
    assert_eq!(*extended_reactions.last().unwrap(), 0.);
    for row in 0..extended_bounds.len() {
        let gap = compensated_gap(
            &extended_columns[row],
            &extended_motion,
            extended_bounds[row],
        );
        assert!(gap >= -tolerance);
        if extended_reactions[row] > 0. {
            assert!(gap.abs() <= tolerance);
        }
    }
    eprintln!(
        "TRIPLE FACTOR APPENDED INACTIVE ROW original_coordinate_gate=true changed_R_stride=true original_state_bit_equal=true retained_bytes={}",
        factors.retained_bytes()
    );
    eprintln!(
        "TRIPLE FACTOR REUSE changed_bounds_original_coordinate_gate=true freshly_factored_result_bit_equal=true"
    );
    // An appended row may become active: retained prefix must extend with the
    // current R stride and every new original constraint must still be checked.
    let prefix =
        ExperimentalTripleActiveContactInput::new_seeded(&[vec![1., 0.]], &[1.], 1e-14, &[1.])
            .unwrap();
    let prefix_output = solve(&prefix);
    let prefix_factors = prefix
        .parallel_factors(bytemuck::cast_slice(&prefix_output))
        .unwrap();
    let mut growing = ExperimentalTripleActiveContactInput::new_seeded(
        &[vec![1., 0.], vec![0., 1.]],
        &[1., 2.],
        1e-14,
        &[1., 0.],
    )
    .unwrap();
    growing.reuse_parallel_factors(&prefix_factors).unwrap();
    let growing_output = solve(&growing);
    assert_eq!(
        growing
            .candidate(bytemuck::cast_slice(&growing_output))
            .unwrap(),
        (vec![1., 2.], vec![1., 2.])
    );
    assert_eq!(growing_output[3], 2);
    eprintln!(
        "TRIPLE ACTIVE BASIS appended_row_enters_active_set=true cached_prefix_extends_with_new_stride=true"
    );
    // Arbitrary overcomplete hints may fail. A fresh cold solve must remain usable.
    let bad = ExperimentalTripleActiveContactInput::new_seeded(
        &[vec![1.], vec![-1.]],
        &[1., -1.],
        1e-14,
        &[1., 1.],
    )
    .unwrap();
    assert!(bad.candidate(bytemuck::cast_slice(&solve(&bad))).is_err());
    let retry =
        ExperimentalTripleActiveContactInput::new(&[vec![1.], vec![-1.]], &[1., -1.], 1e-14)
            .unwrap();
    assert!(
        retry
            .candidate(bytemuck::cast_slice(&solve(&retry)))
            .is_ok()
    );
    eprintln!(
        "TRIPLE WARM cold_iterations={} warm_iterations={} original_coordinate_gate=true dependent_hint_rejected=true fresh_cold_retry_valid=true scope=coordinate_candidate_only_not_physical_admission",
        cold_output[3], warm_output[3]
    );
}

#[test]
#[ignore = "requires actual GPU and original operator JSON; audits decoded candidate residuals"]
fn gpu_triple_changed_bounds_decode_audit() {
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let value = |v: &serde_json::Value| f64::from_bits(v.as_u64().unwrap());
    let columns = fixture["columns_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap().iter().map(value).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let bounds = fixture["bounds_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| value(v) * 1.000001)
        .collect::<Vec<_>>();
    let tolerance = value(&fixture["tolerance_bits"]);
    let source = ExperimentalTripleActiveContactInput::parallel_shader_source();
    let solve = |input: &ExperimentalTripleActiveContactInput| {
        let words = input
            .bytes()
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>();
        let output = dispatch_shader(&words, &source);
        input.candidate(bytemuck::cast_slice(&output)).unwrap()
    };
    let cold = ExperimentalTripleActiveContactInput::new(&columns, &bounds, tolerance).unwrap();
    let (cold_state, seeds) = solve(&cold);
    let warm =
        ExperimentalTripleActiveContactInput::new_seeded(&columns, &bounds, tolerance, &seeds)
            .unwrap();
    let (warm_state, warm_reactions) = solve(&warm);
    let residual = |motion: &[f64], reactions: &[f64]| {
        columns
            .iter()
            .zip(&bounds)
            .zip(reactions)
            .map(|((column, bound), reaction)| {
                let gap = compensated_gap(column, motion, *bound);
                if *reaction > 0. {
                    gap.abs()
                } else {
                    (-gap).max(0.)
                }
            })
            .fold(0f64, f64::max)
    };
    assert!(residual(&cold_state, &seeds) <= tolerance);
    assert!(residual(&warm_state, &warm_reactions) <= tolerance);
    eprintln!(
        "CHANGED BOUNDS freshly_factored_cold_residual={:e} freshly_factored_warm_residual={:e} tolerance={tolerance:e} scope=decoded_coordinate_diagnostic_not_physical_admission",
        residual(&cold_state, &seeds),
        residual(&warm_state, &warm_reactions)
    );
    if let Some(path) = std::env::var_os("VOXY_TRIPLE_DECODE_AUDIT_EXPORT") {
        let bits = |v: &[f64]| v.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        let report = serde_json::json!({"columns_bits":columns.iter().map(|v|bits(v)).collect::<Vec<_>>(),"bounds_bits":bits(&bounds),"tolerance_bits":tolerance.to_bits(),"cold_state_bits":bits(&cold_state),"cold_reactions_bits":bits(&seeds),"warm_state_bits":bits(&warm_state),"warm_reactions_bits":bits(&warm_reactions),"scope":"decoded coordinate diagnostic; no original physical admission"});
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

#[test]
fn triple_batched_shader_validates() {
    let source = ExperimentalTripleActiveContactBatch::shader_source();
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
#[ignore = "requires actual GPU; independent triple operators in one dispatch"]
fn gpu_triple_batched_independent_operators() {
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(crate::ComputeProgram::new(
        &device,
        &ExperimentalTripleActiveContactBatch::shader_source(),
    ))
    .unwrap();
    let solve = |batch: &ExperimentalTripleActiveContactBatch| {
        let job = program.create_job(&device, batch.bytes()).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder, batch.workgroups()).unwrap();
        let snapshot = job.encode_snapshot(&mut encoder).unwrap();
        let submission = queue.submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        batch.candidate(&read.try_read().unwrap().unwrap())
    };
    let inputs = vec![
        ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[2.], 1e-14).unwrap(),
        ExperimentalTripleActiveContactInput::new_seeded(
            &[vec![1., 0.], vec![0., 1.]],
            &[3., 4.],
            1e-14,
            &[1., 1.],
        )
        .unwrap(),
        ExperimentalTripleActiveContactInput::new(&[vec![1., 0., 0.]], &[-1.], 1e-14).unwrap(),
    ];
    let batch = ExperimentalTripleActiveContactBatch::new(inputs).unwrap();
    assert_eq!(
        solve(&batch).unwrap(),
        vec![
            (vec![2.], vec![2.]),
            (vec![3., 4.], vec![3., 4.]),
            (vec![0., 0., 0.], vec![0.])
        ]
    );
    let bad = ExperimentalTripleActiveContactBatch::new(vec![
        ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[2.], 1e-14).unwrap(),
        ExperimentalTripleActiveContactInput::new(&[vec![1.], vec![-1.]], &[1., 1.], 1e-14)
            .unwrap(),
    ])
    .unwrap();
    assert!(solve(&bad).is_err());
    // Exercise offset isolation with complete captured coefficients, not only small shapes.
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let value = |v: &serde_json::Value| f64::from_bits(v.as_u64().unwrap());
    let columns = fixture["columns_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap().iter().map(value).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let bounds = fixture["bounds_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(value)
        .collect::<Vec<_>>();
    let tolerance = value(&fixture["tolerance_bits"]);
    let captured = ExperimentalTripleActiveContactBatch::new(vec![
        ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[2.], tolerance).unwrap(),
        ExperimentalTripleActiveContactInput::new(&columns, &bounds, tolerance).unwrap(),
        ExperimentalTripleActiveContactInput::new(&columns, &bounds, tolerance).unwrap(),
    ])
    .unwrap();
    let candidates = solve(&captured).unwrap();
    assert_eq!(candidates[0], (vec![2.], vec![2.]));
    assert_eq!(candidates[1], candidates[2]);
    for (motion, reactions) in &candidates[1..] {
        for row in 0..bounds.len() {
            let gap = compensated_gap(&columns[row], motion, bounds[row]);
            assert!(gap >= -tolerance, "batch row={row} gap={gap:e}");
            if reactions[row] > 0. {
                assert!(gap.abs() <= tolerance);
            }
        }
    }
    let seeds = &candidates[1].1;
    let seeded = ExperimentalTripleActiveContactBatch::new(vec![
        ExperimentalTripleActiveContactInput::new_seeded(&columns, &bounds, tolerance, seeds)
            .unwrap(),
        ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[2.], tolerance).unwrap(),
        ExperimentalTripleActiveContactInput::new_seeded(&columns, &bounds, tolerance, seeds)
            .unwrap(),
    ])
    .unwrap();
    let warm = solve(&seeded).unwrap();
    assert_eq!(warm[0], warm[2]);
    for (motion, reactions) in [&warm[0], &warm[2]] {
        for row in 0..bounds.len() {
            let gap = compensated_gap(&columns[row], motion, bounds[row]);
            assert!(gap >= -tolerance);
            if reactions[row] > 0. {
                assert!(gap.abs() <= tolerance);
            }
        }
    }
    eprintln!(
        "TRIPLE BATCHED CAPTURE rows={} coordinates={} duplicate_outputs_equal=true cold_and_seeded_original_gaps=true scope=coordinate_transport_not_original_physical_admission",
        bounds.len(),
        columns[0].len()
    );
    eprintln!(
        "TRIPLE BATCHED GPU one_dispatch_three_workgroups=true heterogeneous_shapes=true independent_headers=true seeded_operator=true inactive_operator=true late_failure_withholds_prefix=true scope=coordinate_transport_not_original_physical_admission"
    );
}

#[test]
#[ignore = "requires a real GPU; retain a coefficient component below an f64 ulp"]
fn gpu_triple_expanded_input_retains_low_component() {
    let low = 2f64.powi(-60);
    let input = ExperimentalTripleActiveContactInput::new_expanded(
        &[vec![[1., low]]],
        &[1.],
        2f64.powi(-65),
    )
    .unwrap();
    let words: Vec<_> = input
        .bytes()
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let output = dispatch_shader(
        &words,
        &ExperimentalTripleActiveContactInput::parallel_shader_source(),
    );
    let candidate = input
        .expanded_candidate(bytemuck::cast_slice(&output))
        .unwrap();
    let (words, scales) = candidate.coordinates().clone().into_parts();
    let parts = words[0].map(|v| f64::from(f32::from_bits(v)));
    assert_eq!(parts[0] * scales[0] * scales[1], 1.);
    let tail = (parts[1] + parts[2]) * scales[0] * scales[1];
    assert!(
        tail < 0. && (tail + low).abs() < low * 1e-6,
        "coordinate tail was lost: {tail:e}"
    );
}
