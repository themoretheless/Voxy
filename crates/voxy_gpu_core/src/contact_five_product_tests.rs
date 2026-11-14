fn source() -> String {
    format!(
        "{}\n{}",
        include_str!("contact_five_product.wgsl"),
        include_str!("contact_five_product_probe.wgsl")
    )
}
#[test]
fn five_product_shader_validates() {
    let module = naga::front::wgsl::parse_str(&source()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}
#[test]
#[ignore = "requires physical GPU and real five-component coefficient artifact; inner multiplication only"]
fn gpu_five_product_normalization_matches_exact_oracle_input() {
    let bytes = std::fs::read(std::env::var_os("VOXY_FIVE_PRODUCT_INPUT").unwrap()).unwrap();
    assert!(bytes.len() >= 24 && &bytes[..4] == b"VPA2");
    let rows = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let width = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert!(rows > 0 && rows <= 1024 && width > 0 && width <= 65536);
    assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 5);
    assert_eq!(bytes.len(), 24 + rows * width * 36);
    let mut values = bytes[24..]
        .chunks_exact(36)
        .map(|record| {
            std::array::from_fn::<_, 5, _>(|i| {
                u32::from_le_bytes(record[16 + i * 4..20 + i * 4].try_into().unwrap())
            })
        })
        .filter(|v| v.iter().any(|&w| w & 0x7fffffff != 0))
        .collect::<Vec<_>>();
    // Cover the smallest products first, plus the spread across the operator.
    values.sort_by(|a, b| {
        f32::from_bits(a[0])
            .abs()
            .total_cmp(&f32::from_bits(b[0]).abs())
    });
    let mut pairs = Vec::new();
    for i in 0..64.min(values.len()) {
        pairs.push((values[i], values[(i * 17) % 64.min(values.len())]));
    }
    for i in 0..64 {
        let j = i * (values.len() - 1) / 63;
        pairs.push((values[j], values[values.len() - 1 - j]));
    }
    pairs.push(([0; 5], values[0]));
    pairs.push((values[0], [0x80000000, 0, 0, 0, 0]));
    pairs.push((
        [1f32.to_bits(), 2f32.powi(-100).to_bits(), 0, 0, 0],
        [(-1f32).to_bits(), 0, 0, 0, 0],
    ));
    pairs.push((values[0], values[0].map(|word| word ^ 0x80000000)));
    let negative_radius_index = pairs.len();
    pairs.push((values[0], values[0])); // Reject a negative input uncertainty.
    let denominator_crossing_index = pairs.len();
    pairs.push((values[0], values[0])); // Reject a denominator interval crossing zero.
    let sqrt_crossing_index = pairs.len();
    pairs.push(([1f32.to_bits(), 0, 0, 0, 0], [1f32.to_bits(), 0, 0, 0, 0]));
    let sqrt_boundary_index = pairs.len();
    pairs.push(([1f32.to_bits(), 0, 0, 0, 0], [1f32.to_bits(), 0, 0, 0, 0]));
    let sqrt_negative_index = pairs.len();
    pairs.push((
        [(-1f32).to_bits(), 0, 0, 0, 0],
        [1f32.to_bits(), 0, 0, 0, 0],
    ));
    let sqrt_uncertain_zero_index = pairs.len();
    pairs.push(([0; 5], [1f32.to_bits(), 0, 0, 0, 0]));
    let mut words = vec![0x31413556u32, 9, pairs.len() as u32, 200];
    let radii = [0., 2f32.powi(-110), 2f32.powi(-80), 2f32.powi(-20)];
    for (i, (a, b)) in pairs.iter().enumerate() {
        words.extend(a);
        words.extend(b);
        words.extend([0; 144]);
        let radius_a = if i == negative_radius_index {
            -1.
        } else if i == denominator_crossing_index || i == sqrt_negative_index {
            0.
        } else if i == sqrt_crossing_index {
            2.
        } else if i == sqrt_boundary_index {
            1.
        } else if i == sqrt_uncertain_zero_index {
            2f32.powi(-110)
        } else {
            radii[i % 4]
        };
        let radius_b = if i == denominator_crossing_index {
            2.
        } else if i >= sqrt_crossing_index {
            0.
        } else {
            radii[(i / 4) % 4]
        };
        words.push(radius_a.to_bits());
        words.push(radius_b.to_bits());
        words.extend([0; 44]);
    }
    let original = words.clone();
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(crate::ComputeProgram::new(&device, &source())).unwrap();
    let job = program
        .create_job(&device, bytemuck::cast_slice(&words))
        .unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    job.encode_step(&mut encoder, [(pairs.len() as u32).div_ceil(64), 1, 1])
        .unwrap();
    let snapshot = job.encode_snapshot(&mut encoder).unwrap();
    let submission = queue.submit([encoder.finish()]);
    let mut read = snapshot.begin_read();
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    let output = read.try_read().unwrap().unwrap();
    if let Some(path) = std::env::var_os("VOXY_FIVE_PRODUCT_READBACK") {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap()
            .write_all(&output)
            .unwrap();
    }
    assert_eq!(output.len(), original.len() * 4);
    let words = output
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(words[..4], original[..4]);
    let mut records = Vec::new();
    for (i, record) in words[4..].chunks_exact(200).enumerate() {
        assert_eq!(record[..10], original[4 + i * 200..14 + i * 200]);
        assert_eq!(record[112], 1, "normalization rejected case {i}");
        assert_eq!(record[113], 1, "raw multiplication rejected case {i}");
        assert_eq!(
            record[121], 1,
            "five-component compression rejected case {i}"
        );
        assert_eq!(record[129], 1, "addition rejected case {i}");
        assert_eq!(record[137], 1, "subtraction rejected case {i}");
        assert_eq!(record[145], 1, "propagated addition rejected case {i}");
        assert_eq!(
            record[154..156],
            original[4 + i * 200 + 154..4 + i * 200 + 156]
        );
        assert_eq!(
            record[153],
            u32::from(i != negative_radius_index),
            "input-error validation failed case {i}"
        );
        let zero_denominator = pairs[i].1.iter().all(|w| w.trailing_zeros() >= 31);
        assert_eq!(
            record[163],
            u32::from(!zero_denominator),
            "exact denominator validation failed case {i}"
        );
        assert_eq!(
            record[171],
            u32::from(
                !zero_denominator && i != negative_radius_index && i != denominator_crossing_index
            ),
            "interval denominator validation failed case {i}"
        );
        let nonnegative = f32::from_bits(pairs[i].0[0]) >= 0.;
        assert_eq!(
            record[181],
            u32::from(nonnegative),
            "sqrt domain validation failed case {i} stage={}",
            record[190]
        );
        assert_eq!(
            record[189],
            u32::from(
                nonnegative
                    && i != negative_radius_index
                    && i != sqrt_crossing_index
                    && i != sqrt_uncertain_zero_index
            ),
            "sqrt interval validation failed case {i} stage={}",
            record[191]
        );
        let mut row = serde_json::Map::new();
        row.insert(
            "input_a_words".into(),
            serde_json::to_value(&record[..5]).unwrap(),
        );
        row.insert(
            "input_b_words".into(),
            serde_json::to_value(&record[5..10]).unwrap(),
        );
        row.insert(
            "raw_product_words".into(),
            serde_json::to_value(&record[10..60]).unwrap(),
        );
        row.insert(
            "normalized_product_words".into(),
            serde_json::to_value(&record[60..110]).unwrap(),
        );
        row.insert(
            "normalized_exponent".into(),
            serde_json::to_value(record[110] as i32).unwrap(),
        );
        row.insert(
            "normalized_error_bound_word".into(),
            serde_json::to_value(record[111]).unwrap(),
        );
        row.insert(
            "rounded_product_words".into(),
            serde_json::to_value(&record[114..119]).unwrap(),
        );
        row.insert(
            "rounded_exponent".into(),
            serde_json::to_value(record[119] as i32).unwrap(),
        );
        row.insert(
            "rounded_error_bound_word".into(),
            serde_json::to_value(record[120]).unwrap(),
        );
        row.insert(
            "addition_words".into(),
            serde_json::to_value(&record[122..127]).unwrap(),
        );
        row.insert(
            "addition_exponent".into(),
            serde_json::to_value(record[127] as i32).unwrap(),
        );
        row.insert(
            "addition_error_bound_word".into(),
            serde_json::to_value(record[128]).unwrap(),
        );
        row.insert(
            "subtraction_words".into(),
            serde_json::to_value(&record[130..135]).unwrap(),
        );
        row.insert(
            "subtraction_exponent".into(),
            serde_json::to_value(record[135] as i32).unwrap(),
        );
        row.insert(
            "subtraction_error_bound_word".into(),
            serde_json::to_value(record[136]).unwrap(),
        );
        row.insert(
            "propagated_words".into(),
            serde_json::to_value(&record[138..143]).unwrap(),
        );
        row.insert(
            "propagated_exponent".into(),
            serde_json::to_value(record[143] as i32).unwrap(),
        );
        row.insert(
            "propagated_error_bound_word".into(),
            serde_json::to_value(record[144]).unwrap(),
        );
        row.insert(
            "bounded_product_words".into(),
            serde_json::to_value(&record[146..151]).unwrap(),
        );
        row.insert(
            "bounded_product_exponent".into(),
            serde_json::to_value(record[151] as i32).unwrap(),
        );
        row.insert(
            "bounded_product_error_bound_word".into(),
            serde_json::to_value(record[152]).unwrap(),
        );
        row.insert(
            "bounded_product_valid".into(),
            serde_json::to_value(record[153]).unwrap(),
        );
        row.insert(
            "input_error_a_word".into(),
            serde_json::to_value(record[154]).unwrap(),
        );
        row.insert(
            "input_error_b_word".into(),
            serde_json::to_value(record[155]).unwrap(),
        );
        row.insert(
            "exact_division_words".into(),
            serde_json::to_value(&record[156..161]).unwrap(),
        );
        row.insert(
            "exact_division_exponent".into(),
            serde_json::to_value(record[161] as i32).unwrap(),
        );
        row.insert(
            "exact_division_error_bound_word".into(),
            serde_json::to_value(record[162]).unwrap(),
        );
        row.insert(
            "exact_division_valid".into(),
            serde_json::to_value(record[163]).unwrap(),
        );
        row.insert(
            "bounded_division_words".into(),
            serde_json::to_value(&record[164..169]).unwrap(),
        );
        row.insert(
            "bounded_division_exponent".into(),
            serde_json::to_value(record[169] as i32).unwrap(),
        );
        row.insert(
            "bounded_division_error_bound_word".into(),
            serde_json::to_value(record[170]).unwrap(),
        );
        row.insert(
            "bounded_division_valid".into(),
            serde_json::to_value(record[171]).unwrap(),
        );
        row.insert(
            "exact_division_failure_stage".into(),
            serde_json::to_value(record[172]).unwrap(),
        );
        row.insert(
            "bounded_division_failure_stage".into(),
            serde_json::to_value(record[173]).unwrap(),
        );
        row.insert(
            "exact_sqrt_words".into(),
            serde_json::to_value(&record[174..179]).unwrap(),
        );
        row.insert(
            "exact_sqrt_exponent".into(),
            serde_json::to_value(record[179] as i32).unwrap(),
        );
        row.insert(
            "exact_sqrt_error_bound_word".into(),
            serde_json::to_value(record[180]).unwrap(),
        );
        row.insert(
            "exact_sqrt_valid".into(),
            serde_json::to_value(record[181]).unwrap(),
        );
        row.insert(
            "bounded_sqrt_words".into(),
            serde_json::to_value(&record[182..187]).unwrap(),
        );
        row.insert(
            "bounded_sqrt_exponent".into(),
            serde_json::to_value(record[187] as i32).unwrap(),
        );
        row.insert(
            "bounded_sqrt_error_bound_word".into(),
            serde_json::to_value(record[188]).unwrap(),
        );
        row.insert(
            "bounded_sqrt_valid".into(),
            serde_json::to_value(record[189]).unwrap(),
        );
        row.insert(
            "exact_sqrt_failure_stage".into(),
            serde_json::to_value(record[190]).unwrap(),
        );
        row.insert(
            "bounded_sqrt_failure_stage".into(),
            serde_json::to_value(record[191]).unwrap(),
        );
        row.insert(
            "exact_sqrt_arithmetic_failure".into(),
            serde_json::to_value(&record[192..196]).unwrap(),
        );
        row.insert(
            "bounded_sqrt_arithmetic_failure".into(),
            serde_json::to_value(&record[196..200]).unwrap(),
        );
        records.push(serde_json::Value::Object(row));
    }
    let path = std::env::var_os("VOXY_FIVE_PRODUCT_EXPORT").unwrap();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    serde_json::to_writer_pretty(file,&serde_json::json!({"scope":"GPU FMA product/error pairs and five-term compression; not complete arithmetic or contact solve","records":records})).unwrap();
}
