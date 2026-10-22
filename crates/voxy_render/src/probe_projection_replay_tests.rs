//! Replay complete captured faces through the production projector. Expected
//! coefficients remain CPU-only; no result or reduced sampling grid is uploaded.
use super::*;

#[test]
#[ignore = "exclusive Metal GPU; requires VOXY_PROBE_REPLAY_INPUT and new VOXY_PROBE_REPLAY_OUTPUT"]
#[cfg(target_os = "macos")]
fn captured_faces_replay_through_original_projector() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::path::PathBuf::from(
        std::env::var_os("VOXY_PROBE_REPLAY_INPUT").ok_or("input required")?,
    );
    let output = std::path::PathBuf::from(
        std::env::var_os("VOXY_PROBE_REPLAY_OUTPUT").ok_or("new output required")?,
    );
    std::fs::create_dir(&output)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let projector = ProbeProjector::new(&device);
    let probes = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("original capacity probe replay"),
        size: MAX_PROBES as u64 * PROBE_BYTES,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut faces = Vec::new();
    let mut restored_words = 0usize;
    for face in 0..6 {
        let folder = input.join(format!("probe-0000-face-{face}"));
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(folder.join("stages.json"))?)?;
        assert_eq!(metadata["probe_index"], 0);
        assert_eq!(metadata["face_index"], face);
        let stage = metadata["stages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["stage"] == "probe-face-radiance")
            .unwrap();
        assert_eq!(stage["source_format"], "Rgba16Float");
        assert_eq!(stage["nonfinite_channels"], 0);
        let width = stage["size"][0].as_u64().unwrap() as u32;
        let height = stage["size"][1].as_u64().unwrap() as u32;
        let raw = std::fs::read(folder.join("probe-face-radiance.rgba32.bin"))?;
        assert_eq!(
            blake3::hash(&raw).to_hex().as_str(),
            stage["blake3"].as_str().unwrap()
        );
        assert_eq!(raw.len(), width as usize * height as usize * 16);
        let mut native = Vec::with_capacity(raw.len() / 2);
        for word in raw.chunks_exact(4) {
            let bits = u32::from_le_bytes(word.try_into().unwrap());
            let half = half::f16::from_f32(f32::from_bits(bits));
            assert_eq!(
                half.to_f32().to_bits(),
                bits,
                "captured source not losslessly reconstructible"
            );
            native.extend_from_slice(&half.to_bits().to_le_bytes());
            restored_words += 1;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("complete original captured probe face"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &native,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 8),
                rows_per_image: Some(height),
            },
            texture.size(),
        );
        let uniform = std::fs::read(folder.join("projection-uniform.bin"))?;
        assert_eq!(uniform.len(), 64);
        assert_eq!(
            blake3::hash(&uniform).to_hex().as_str(),
            metadata["projection_uniform"]["blake3"].as_str().unwrap()
        );
        let words: Vec<f32> = uniform
            .chunks_exact(4)
            .map(|x| f32::from_le_bytes(x.try_into().unwrap()))
            .collect();
        assert_eq!(words[3], 0.);
        assert_eq!(words[7], face as f32);
        let expected = std::fs::read(folder.join("current-probe-sh.native.bin"))?;
        assert_eq!(expected.len(), 64);
        assert_eq!(
            blake3::hash(&expected).to_hex().as_str(),
            metadata["current_probe_coefficients_after_face"]["blake3"]
                .as_str()
                .unwrap()
        );
        faces.push((texture, uniform, words, expected));
    }
    let mut comparisons = Vec::new();
    for trial in 0..20 {
        queue.write_buffer(&probes, 0, &vec![0u8; MAX_PROBES * PROBE_BYTES as usize]);
        for (face, (texture, uniform, words, expected)) in faces.iter().enumerate() {
            let basis = [0, 4, 8].map(|i| glam::Vec3::new(words[i], words[i + 1], words[i + 2]));
            let submission = projector.project(
                &device,
                &queue,
                &texture.create_view(&Default::default()),
                (texture.width(), texture.height()),
                &probes,
                0,
                face,
                basis,
                words[11],
            );
            device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })?;
            let path = output.join(format!("trial-{trial:02}-face-{face}"));
            std::fs::create_dir(&path)?;
            crate::archive_hdr_diagnostics::capture_uniform(
                &device,
                &queue,
                projector.diagnostic_uniform(),
                &path.join("uniform.bin"),
            )?;
            assert_eq!(std::fs::read(path.join("uniform.bin"))?, *uniform);
            crate::archive_hdr_diagnostics::capture_storage(
                &device,
                &queue,
                &probes,
                &path.join("coefficients.bin"),
                true,
            )?;
            let actual = std::fs::read(path.join("coefficients.bin"))?;
            let identical = &actual[..64] == expected;
            comparisons.push(serde_json::json!({"trial":trial,"face":face,"original_result_identical":identical}));
        }
    }
    let passed = comparisons
        .iter()
        .all(|x| x["original_result_identical"] == true);
    std::fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "all_original_projection_results_identical":passed,"comparisons":comparisons,
            "complete_source_channels_losslessly_restored":restored_words,
            "production_projector_used":true,"expected_coefficients_uploaded":false,
            "full_scene_cause_proved":false,"performance_qualified":false
        }))?,
    )?;
    assert!(
        passed,
        "production projector replay differs; inspect retained actual results"
    );
    Ok(())
}
