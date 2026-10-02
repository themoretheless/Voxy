use voxy_render::{
    GgxSurfaceSample, RayScene, RaySceneError, ReconstructionMaterial, SpecularDistancePipeline,
    SpecularRay,
};

pub fn probe(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), Box<dyn std::error::Error>> {
    let scene = RayScene::new(
        device,
        &[
            [-1.0, -1.0, 3.0],
            [1.0, -1.0, 3.0],
            [0.0, 1.0, 3.0],
            [-1.0, -1.0, 5.0],
            [1.0, -1.0, 5.0],
            [0.0, 1.0, 5.0],
        ],
    )?;
    let ray = |x, bias, range| {
        SpecularRay::from_surface([x, 0.0, 1.0], [0.0, 0.0, 1.0], [x, 0.0, 3.0], bias, range)
    };
    let rays = [
        ray(0.0, 0.01, 10.0)?,
        ray(3.0, 0.01, 10.0)?,
        ray(0.0, 0.01, 1.0)?,
        ray(0.0, 2.5, 10.0)?,
    ];
    let pipeline = SpecularDistancePipeline::new(device)?;
    assert!(matches!(
        pipeline.create_job(&scene, 4, 1, &rays[..1]),
        Err(RaySceneError::Capacity)
    ));
    let emission = [[4.0, 1.0, 0.5, 1.0], [0.5, 2.0, 8.0, 1.0]];
    assert!(
        pipeline
            .create_radiance_job(&scene, 4, 1, &rays, &emission[..1])
            .is_err()
    );
    let throughput = [
        mirror_weight([0.5, 0.25, 0.125])?,
        [1.0; 4],
        [1.0; 4],
        mirror_weight([0.0, 0.5, 1.0])?,
    ];
    reject_invalid_weights(&pipeline, &scene, &rays, &emission, &throughput);
    let job =
        pipeline.create_weighted_radiance_job(&scene, [4, 1], &rays, &emission, &throughput)?;
    let hdr_pipeline = SpecularDistancePipeline::new_hdr(device)?;
    let hdr_job = hdr_pipeline.create_weighted_radiance_job(
        &scene,
        [4, 1],
        &rays,
        &[[65504.0, 1.0, 0.5, 1.0], emission[1]],
        &[[2.0; 4]; 4],
    )?;
    assert_eq!(
        hdr_job.incident_radiance().format(),
        wgpu::TextureFormat::Rgba32Float
    );
    let rough = ReconstructionMaterial::new([0.8, 0.4, 0.2], 1.0, 0.5)?;
    let accepted = GgxSurfaceSample::new(
        &rough,
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 3.0],
        [0.25, 0.0],
        [0.01, 10.0],
    )?;
    assert!(accepted.is_some());
    let null = GgxSurfaceSample::new(
        &ReconstructionMaterial::new([1.0; 3], 1.0, 1.0)?,
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 3.0],
        [0.75, 0.3],
        [0.01, 10.0],
    )?;
    assert!(null.is_none());
    let ggx =
        hdr_pipeline.create_ggx_job(&scene, [4, 1], &[accepted, null, None, None], &emission)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("specular distance proof"),
        size: 1536,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    scene.build(&mut encoder);
    job.encode(&mut encoder);
    hdr_job.encode(&mut encoder);
    encoder.copy_texture_to_buffer(
        hdr_job.incident_radiance().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 768,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        hdr_job.incident_radiance().size(),
    );
    for (texture, offset) in [(job.output(), 0), (job.incident_radiance(), 256)] {
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            texture.size(),
        );
    }
    ggx.encode(&mut encoder);
    for (texture, offset) in [(ggx.incident_radiance(), 512), (ggx.output(), 1024)] {
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            texture.size(),
        );
    }
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(index),
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = readback.slice(..).get_mapped_range()?;
    for (bytes, expected) in mapped[..16].chunks_exact(4).zip([2.0_f32, 0.0, 0.0, 4.0]) {
        let actual = f32::from_le_bytes(bytes.try_into()?);
        if !actual.is_finite() || (actual - expected).abs() > 0.0001 {
            return Err(format!("specular GPU distance {actual} != {expected}").into());
        }
    }
    verify_radiance(&mapped[256..288])?;
    for (bytes, expected) in mapped[768..784]
        .chunks_exact(4)
        .zip([131008.0_f32, 2.0, 1.0, 1.0])
    {
        let actual = f32::from_le_bytes(bytes.try_into()?);
        if !actual.is_finite() || (actual - expected).abs() > 0.0001 {
            return Err(format!("wide HDR reflection {actual} != {expected}").into());
        }
    }
    println!("Wide HDR reflection RGBA32: weight 2, radiance 131008 passed");
    // Independently derived for alpha=1/4, u=1/4: tan²(theta)=1/48,
    // reflected z=47/49, plane at z=3 from origin z=1 => distance=98/47.
    let reflected_z = 47.0_f64 / 49.0;
    let half_cosine = (48.0_f64 / 49.0).sqrt();
    let masking = 2.0
        / (1.0
            + (1.0 + (1.0 - reflected_z * reflected_z) / (16.0 * reflected_z * reflected_z))
                .sqrt());
    let expected_rough =
        [0.8, 0.4, 0.2].map(|f0| (f0 + (1.0 - f0) * (1.0 - half_cosine).powi(5)) * masking);
    for (pixel, values) in mapped[512..576].chunks_exact(16).enumerate() {
        let expected = if pixel == 0 {
            [
                4.0 * expected_rough[0],
                expected_rough[1],
                0.5 * expected_rough[2],
                1.0,
            ]
        } else {
            [0.0, 0.0, 0.0, 1.0]
        };
        for (bytes, expected) in values.chunks_exact(4).zip(expected) {
            let actual = f32::from_le_bytes(bytes.try_into()?);
            if !actual.is_finite() || (f64::from(actual) - expected).abs() > 0.0001 {
                return Err(format!("GGX GPU sample {actual} != {expected}").into());
            }
        }
    }
    for (bytes, expected) in mapped[1024..1040]
        .chunks_exact(4)
        .zip([98.0 / 47.0, 0.0, 0.0, 0.0])
    {
        let actual = f32::from_le_bytes(bytes.try_into()?);
        if (actual - expected).abs() > 0.0001 {
            return Err("GGX distance/null mismatch".into());
        }
    }
    println!(
        "GGX GPU ray sample: nonzero rough deflection, analytic distance/BRDF weight, null radiance and zero distance passed"
    );
    drop(mapped);
    readback.unmap();
    println!(
        "Specular R32 closest-hit, miss, range and surface-origin bias distances and BSDF-weighted HDR emissive radiance passed"
    );
    Ok(())
}

fn verify_radiance(pixels: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let colors = [
        [2.0, 0.25, 0.0625, 1.0],
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 8.0, 1.0],
    ];
    for (pixel, expected) in pixels.chunks_exact(8).zip(colors) {
        for (bytes, expected) in pixel.chunks_exact(2).zip(expected) {
            let actual = half::f16::from_bits(u16::from_le_bytes(bytes.try_into()?)).to_f32();
            if !actual.is_finite() || (actual - expected).abs() > 0.001 {
                return Err(format!("specular GPU radiance {actual} != {expected}").into());
            }
        }
    }
    Ok(())
}

fn reject_invalid_weights(
    pipeline: &SpecularDistancePipeline,
    scene: &RayScene,
    rays: &[SpecularRay],
    emission: &[[f32; 4]],
    throughput: &[[f32; 4]; 4],
) {
    for invalid in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        let mut weights = *throughput;
        weights[0][0] = invalid;
        assert!(
            pipeline
                .create_weighted_radiance_job(scene, [4, 1], rays, emission, &weights)
                .is_err()
        );
    }
    assert!(
        pipeline
            .create_weighted_radiance_job(scene, [4, 1], rays, emission, &throughput[..1])
            .is_err()
    );
}

fn mirror_weight(base: [f32; 3]) -> Result<[f32; 4], voxy_render::ReconstructionMaterialError> {
    ReconstructionMaterial::new(base, 1.0, 0.0)?.mirror_throughput([0.0, 0.0, 1.0], [0.0, 0.0, 2.0])
}
