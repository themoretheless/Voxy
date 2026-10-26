//! Synthetic records qualify the diagnostic comparator, never scene geometry.
use super::WorldTraversalPair;
#[test]
#[ignore = "exclusive Metal GPU; requires fresh VOXY_TRAVERSAL_PAIR_COMPARATOR_PROOF"]
#[cfg(target_os = "macos")]
fn comparator_detects_protected_low_bits_and_ignores_only_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    use wgpu::util::DeviceExt;
    let root = std::path::PathBuf::from(
        std::env::var_os("VOXY_TRAVERSAL_PAIR_COMPARATOR_PROOF")
            .ok_or("fresh proof directory required")?,
    );
    std::fs::create_dir(&root)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let module=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("comparator test placeholder, no ray query"),source:wgpu::ShaderSource::Wgsl("@group(0) @binding(0) var<storage,read> data:array<u32>; @compute @workgroup_size(1) fn main(){_=data[0];}".into())});
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let upload = |label, words: &[u32], usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(words),
            usage,
        })
    };
    const CAP: usize = 130;
    const COUNT: u32 = 129;
    const STRIDE: usize = 28;
    const POSITION: usize = CAP * STRIDE;
    let mut original: Vec<u32> = (0..CAP * 36)
        .map(|i| 0x10000u32.wrapping_add(i as u32 * 31))
        .collect();
    let mut rays: Vec<u32> = (0..CAP * 20)
        .map(|i| 0x20000u32.wrapping_add(i as u32 * 17))
        .collect();
    rays[10 * 20 + 3] = 1;
    original[10 * STRIDE] = 1;
    rays[11 * 20 + 3] = 1;
    original[11 * STRIDE] = 2;
    let mut results = Vec::new();
    for case in [
        "equal",
        "route_metadata",
        "queue_padding",
        "unrequested_output",
        "unrequested_input",
        "origin_low",
        "direction_low",
        "stable_id",
        "parameter_low",
        "uv_low",
        "position_low",
        "material_kind",
        "tie_count",
    ] {
        let mut candidate = original.clone();
        let mut new_rays = rays.clone();
        let expected_equal = match case {
            "equal" => true,
            "route_metadata" => {
                candidate[7 * STRIDE + 3] ^= 1;
                candidate[7 * STRIDE + 16] ^= 1;
                candidate[7 * STRIDE + 18] ^= 0x1001;
                true
            }
            "queue_padding" => {
                for i in [7, 11, 15] {
                    new_rays[65 * 20 + i] ^= 1;
                }
                true
            }
            "unrequested_output" => {
                candidate[129 * STRIDE + 9] ^= 1;
                true
            }
            "unrequested_input" => {
                new_rays[129 * 20] ^= 1;
                true
            }
            "origin_low" => {
                new_rays[65 * 20 + 4] ^= 1;
                false
            }
            "direction_low" => {
                new_rays[65 * 20 + 12] ^= 1;
                false
            }
            "stable_id" => {
                new_rays[63 * 20 + 18] ^= 1;
                false
            }
            "parameter_low" => {
                candidate[66 * STRIDE + 9] ^= 1;
                false
            }
            "uv_low" => {
                candidate[21] ^= 1;
                false
            }
            "position_low" => {
                candidate[POSITION + 64 * 8 + 1] ^= 1;
                false
            }
            "material_kind" => {
                candidate[67 * STRIDE + 18] ^= 4;
                false
            }
            "tie_count" => {
                candidate[2 * STRIDE + 19] ^= 1;
                false
            }
            _ => unreachable!(),
        };
        let source_output = upload(
            "synthetic original records",
            &original,
            wgpu::BufferUsages::COPY_SRC,
        );
        let source_rays = upload(
            "synthetic original ray records",
            &rays,
            wgpu::BufferUsages::COPY_SRC,
        );
        let candidate_output = upload(
            "synthetic candidate records",
            &candidate,
            wgpu::BufferUsages::COPY_SRC,
        );
        let candidate_rays = upload(
            "synthetic modified ray records",
            &new_rays,
            wgpu::BufferUsages::COPY_SRC,
        );
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: original.len() as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let input = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: rays.len() as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let placeholder = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            }],
        });
        let pair = WorldTraversalPair::new(
            &device,
            &input,
            &output,
            STRIDE as u32,
            POSITION as u64,
            (pipeline.clone(), placeholder),
        )?;
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&source_output, 0, &output, 0, output.size());
        encoder.copy_buffer_to_buffer(&source_rays, 0, &input, 0, input.size());
        pair.snapshot(&mut encoder, &input, &output);
        encoder.copy_buffer_to_buffer(&candidate_output, 0, &output, 0, output.size());
        encoder.copy_buffer_to_buffer(&candidate_rays, 0, &input, 0, input.size());
        pair.compare(&device, &mut encoder, &input, &output, COUNT);
        queue.submit([encoder.finish()]);
        let proof = pair.read_proof(&device, &queue)?;
        let capture_ok = if expected_equal {
            proof["first_failure_capture_u32"].is_null()
        } else {
            let capture = proof["first_failure_capture_u32"]
                .as_array()
                .ok_or("missing first mismatch native records")?;
            let index = proof["raw_words"][4].as_u64().unwrap() as usize;
            capture.len() == 128
                && capture[0] == 1u32
                && capture[1] == index
                && capture[3] == STRIDE as u64
                && capture[8..28]
                    .iter()
                    .zip(&rays[index * 20..index * 20 + 20])
                    .all(|(a, b)| a == b)
                && capture[28..48]
                    .iter()
                    .zip(&new_rays[index * 20..index * 20 + 20])
                    .all(|(a, b)| a == b)
                && capture[48..76]
                    .iter()
                    .zip(&original[index * STRIDE..index * STRIDE + STRIDE])
                    .all(|(a, b)| a == b)
                && capture[76..104]
                    .iter()
                    .zip(&candidate[index * STRIDE..index * STRIDE + STRIDE])
                    .all(|(a, b)| a == b)
                && capture[2] == proof["raw_words"][5]
                && capture[7] == COUNT
                && capture[104..112]
                    .iter()
                    .zip(&original[POSITION + index * 8..POSITION + index * 8 + 8])
                    .all(|(a, b)| a == b)
                && capture[112..120]
                    .iter()
                    .zip(&candidate[POSITION + index * 8..POSITION + index * 8 + 8])
                    .all(|(a, b)| a == b)
        };
        let passed = capture_ok
            && proof["all_protected_words_equal"] == expected_equal
            && proof["compared_records"] == u64::from(COUNT)
            && (!expected_equal
                || (proof["explicitly_admitted_records"] == 2u64
                    && proof["matched_hit_records"] == 1u64
                    && proof["matched_unresolved_records"] == 1u64));
        std::fs::write(
            root.join(format!("{case}.json")),
            serde_json::to_vec_pretty(&proof)?,
        )?;
        results.push(serde_json::json!({"case":case,"expected_equal":expected_equal,"passed":passed,"proof":proof}));
    }
    let passed = results.iter().all(|r| r["passed"] == true);
    std::fs::write(
        root.join("proof.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"all_cases_passed":passed,"cases":results,"scope":"diagnostic_comparator_synthetic_record_unit_fixture","synthetic_query_records_uploaded":true,"renderer_or_source_query_qualified":false,"performance_qualified":false}),
        )?,
    )?;
    assert!(
        passed,
        "comparator failed an actual-Metal positive or negative control"
    );
    Ok(())
}
