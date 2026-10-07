//! Physical GPU extinction readback against the native f64 cell reference.
use physics::liquid::*;
use voxy_render::{
    ComputeProgram, DROPLET_EXTINCTION_SHADER, DropletExtinctionComputeInput, ExtinctionGridView,
    GraphicsOptions,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut liquid = Liquid::new(
        vec![
            Particle {
                position: [0.25, 0.5, 0.5],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            },
            Particle {
                position: [1.25, 0.5, 0.5],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config::default(),
    )?;
    liquid.configure_droplet_population(Some(vec![true, true]))?;
    let gas = FiniteDropletGasGrid::new(
        [0.; 3],
        [1.; 3],
        [2, 1, 1],
        vec![
            VaporCell {
                mass: 0.1,
                volume: 1.,
                temperature: 300.,
                velocity: [0.; 3],
                specific_heat_cv: 718.
            };
            2
        ],
    )?;
    let field = liquid.droplet_extinction_grid(&gas, &[0.1, 0.2], 2.)?;
    let cases = [
        ([-1., 0.5, 0.5], [3., 0.5, 0.5]),
        ([3., 0.5, 0.5], [-1., 0.5, 0.5]),
        ([0., 0., 0.5], [2., 1., 0.5]),
        ([1., 0., 0.5], [1., 1., 0.5]),
        ([0., 2., 0.], [2., 2., 0.]),
        ([0.5; 3], [0.5; 3]),
    ];
    let rays = (0..131).map(|i| cases[i % cases.len()]).collect::<Vec<_>>();
    let analytic_ray_count = rays.len();
    let view = || ExtinctionGridView {
        origin: field.origin(),
        spacing: field.spacing(),
        shape: field.shape(),
        extinction_m_inverse: field.extinction_m_inverse(),
    };
    let input = DropletExtinctionComputeInput::new(view(), &rays, 65536)?;
    assert!(input.decode(input.bytes()).is_err());
    assert!(DropletExtinctionComputeInput::new(view(), &rays, input.bytes().len() - 1).is_err());
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("EXTINCTION GPU {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let program = pollster::block_on(ComputeProgram::new(&device, DROPLET_EXTINCTION_SHADER))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let job = program.create_job(&device, input.bytes())?;
    let dispatch = job.encode(&mut encoder, input.workgroups())?;
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes = pending.try_read()?.ok_or("GPU readback pending")?;
    let values = input.decode(&bytes)?;
    let mut max_tau = 0_f64;
    let mut max_transmission = 0_f64;
    for (&(a, b), result) in rays.iter().zip(&values) {
        max_tau = max_tau.max((result[0] as f64 - field.optical_depth_segment(a, b)?).abs());
        max_transmission =
            max_transmission.max((result[1] as f64 - field.transmittance_segment(a, b)?).abs());
    }
    assert!(max_tau < 1e-6 && max_transmission < 1e-6);
    let mut corrupted = bytes.clone();
    let output = (12 + field.extinction_m_inverse().len() + 6) * 4;
    corrupted[output..output + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(input.decode(&corrupted).is_err());
    assert!(input.decode(&bytes[..bytes.len() - 1]).is_err());
    // A heterogeneous 3D field qualifies cell traversal beyond the two-cell analytic cases.
    let shape = [7, 5, 3];
    let origin = [-1., -1., -0.5];
    let spacing = [0.25, 0.5, 0.25];
    let mut particles = Vec::new();
    let mut radii = Vec::new();
    for z in 0..shape[2] {
        for y in 0..shape[1] {
            for x in 0..shape[0] {
                let radius = 0.03 + 0.005 * ((x + 2 * y + 3 * z) % 7) as f64;
                particles.push(Particle {
                    position: [
                        origin[0] + (x as f64 + 0.5) * spacing[0],
                        origin[1] + (y as f64 + 0.5) * spacing[1],
                        origin[2] + (z as f64 + 0.5) * spacing[2],
                    ],
                    velocity: [0.; 3],
                    mass: 1000. * 4. * std::f64::consts::PI * radius * radius * radius / 3.,
                    material: 0,
                });
                radii.push(radius);
            }
        }
    }
    let mut cloud = Liquid::new(particles, vec![Material::WATER], Config::default())?;
    cloud.configure_droplet_population(Some(vec![true; radii.len()]))?;
    let gas = FiniteDropletGasGrid::new(
        origin,
        spacing,
        shape,
        vec![
            VaporCell {
                mass: 0.01,
                volume: spacing.iter().product(),
                temperature: 300.,
                velocity: [0.; 3],
                specific_heat_cv: 718.
            };
            radii.len()
        ],
    )?;
    let cloud_field = cloud.droplet_extinction_grid(&gas, &radii, 2.)?;
    let mut seed = 19u32;
    let mut random_point = || {
        std::array::from_fn(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed % 193) as f64 / 32. - 3.
        })
    };
    let mut rays = (0..1021)
        .map(|_| (random_point(), random_point()))
        .collect::<Vec<_>>();
    rays.extend([
        ([-2., -2., -1.], [2., 3., 1.]),
        ([2., 3., 1.], [-2., -2., -1.]),
        ([-1., -1., -0.5], [0.75, 1.5, 0.25]),
        ([0., -2., 0.], [0., 3., 0.]),
        ([-2., 0., 0.], [2., 0., 0.]),
        ([0.; 3], [0.; 3]),
    ]);
    let input = DropletExtinctionComputeInput::new(
        ExtinctionGridView {
            origin,
            spacing,
            shape,
            extinction_m_inverse: cloud_field.extinction_m_inverse(),
        },
        &rays,
        65536,
    )?;
    let mut outputs = Vec::new();
    for shader in [
        DROPLET_EXTINCTION_SHADER,
        voxy_render::DROPLET_EXTINCTION_REFERENCE_SHADER,
    ] {
        let program = pollster::block_on(ComputeProgram::new(&device, shader))?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let dispatch = program
            .create_job(&device, input.bytes())?
            .encode(&mut encoder, input.workgroups())?;
        queue.submit([encoder.finish()]);
        let mut read = dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely())?;
        outputs.push(input.decode(&read.try_read()?.ok_or("cloud readback pending")?)?);
    }
    let mut max_cloud_error = 0_f64;
    let mut max_dense_error = 0_f64;
    for (i, &(a, b)) in rays.iter().enumerate() {
        let expected = cloud_field.optical_depth_segment(a, b)?;
        max_cloud_error = max_cloud_error.max((outputs[0][i][0] as f64 - expected).abs());
        max_dense_error = max_dense_error.max((outputs[0][i][0] - outputs[1][i][0]).abs() as f64);
    }
    assert!(max_cloud_error < 1e-5 && max_dense_error < 1e-5);
    println!(
        "EXTINCTION TRAVERSAL PASS cells={} rays={} maximum_native_tau_error={max_cloud_error:.17e} maximum_dense_gpu_tau_error={max_dense_error:.17e}",
        radii.len(),
        rays.len()
    );
    let camera = voxy_render::SceneCamera {
        eye: glam::Vec3::new(-2., 0.5, 0.5),
        target: glam::Vec3::new(0., 0.5, 0.5),
        up: glam::Vec3::Y,
        projection: voxy_render::SceneProjection::Perspective {
            vertical_fov: 60_f32.to_radians(),
            aspect: 1.,
            near: 0.1,
            far: 10.,
        },
    };
    let project = camera.view_projection()?;
    let mut depth_rays = Vec::new();
    for x in [-0.5, 3.] {
        let depth = project.project_point3(glam::Vec3::new(x, 0.5, 0.5)).z;
        depth_rays.extend(voxy_render::extinction_segments_from_depth(
            camera,
            1,
            1,
            &[depth],
        )?);
    }
    let colors = [[0.8, 0.4, 0.2, 0.5]; 2];
    let composite = voxy_render::DropletExtinctionCompositeInput::new(
        ExtinctionGridView {
            origin: field.origin(),
            spacing: field.spacing(),
            shape: field.shape(),
            extinction_m_inverse: field.extinction_m_inverse(),
        },
        &depth_rays,
        &colors,
        65536,
    )?;
    let program = pollster::block_on(ComputeProgram::new(
        &device,
        voxy_render::DROPLET_EXTINCTION_COMPOSITE_SHADER,
    ))?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = program
        .create_job(&device, composite.bytes())?
        .encode(&mut encoder, composite.workgroups())?;
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes = pending.try_read()?.ok_or("composite readback pending")?;
    let output = composite.decode(&bytes)?;
    assert_eq!(
        output[0], colors[0],
        "opaque foreground must hide all downstream extinction"
    );
    let expected = field.transmittance_segment(depth_rays[1].0, depth_rays[1].1)?;
    assert!(expected < 1.);
    for k in 0..3 {
        assert!((output[1][k] as f64 - colors[1][k] as f64 * expected).abs() < 1e-6);
    }
    assert_eq!(output[1][3], colors[1][3]);
    println!(
        "EXTINCTION COMPOSITE PASS foreground_unattenuated=true background_transmission={expected:.17e} alpha_preserved=true"
    );
    verify_rendered_scene(&device, &queue, camera, &field)?;
    verify_hdr_storage_presentation(&device, &queue)?;
    verify_single_scattering(&device, &queue)?;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "EXTINCTION PASS rays={} maximum_tau_error={max_tau:.17e} maximum_transmission_error={max_transmission:.17e}",
        analytic_ray_count
    );
    Ok(())
}

fn verify_rendered_scene(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    camera: voxy_render::SceneCamera,
    field: &physics::liquid::DropletExtinctionGrid,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_render::{SceneDraw, SceneMesh, SceneRenderer, SceneVertex};
    let size = 128u32;
    let renderer = SceneRenderer::new(device, wgpu::TextureFormat::Rgba8Unorm);
    let quad = |x: f32, y: [f32; 2], z: [f32; 2], color: [f32; 4]| {
        SceneMesh::new(
            [
                [x, y[0], z[0]],
                [x, y[1], z[0]],
                [x, y[1], z[1]],
                [x, y[0], z[1]],
            ]
            .into_iter()
            .zip([[0., 0.], [0., 1.], [1., 1.], [1., 0.]])
            .map(|(position, uv)| SceneVertex {
                position,
                uv,
                color,
            })
            .collect(),
            vec![0, 2, 1, 0, 3, 2],
        )
    };
    let front = renderer.upload_mesh(
        device,
        &quad(-0.5, [-0.2, 1.2], [-0.2, 0.45], [0.1, 0.8, 0.2, 1.])?,
    )?;
    let back = renderer.upload_mesh(device, &quad(3., [-3., 4.], [-3., 4.], [1.; 4])?)?;
    let white = renderer.upload_texture(device, queue, 1, 1, &[255; 4])?;
    let checker = renderer.upload_texture(
        device,
        queue,
        2,
        2,
        &[
            240, 240, 240, 255, 80, 120, 180, 255, 80, 120, 180, 255, 240, 240, 240, 255,
        ],
    )?;
    let transform = renderer.create_transform(device, camera.view_projection()?)?;
    let texture = |format| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("extinction opaque scene"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    };
    let color = texture(wgpu::TextureFormat::Rgba8Unorm);
    let depth = texture(wgpu::TextureFormat::Depth32Float);
    let buffer = || {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("extinction scene readback"),
            size: (size * size * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    };
    let color_read = buffer();
    let depth_read = buffer();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    renderer.encode(
        &mut encoder,
        &color.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        wgpu::Color::BLACK,
        &[
            SceneDraw {
                geometry: &back,
                texture: &checker,
                transform: &transform,
                overlay: false,
            },
            SceneDraw {
                geometry: &front,
                texture: &white,
                transform: &transform,
                overlay: false,
            },
        ],
    );
    for (texture, buffer, aspect) in [
        (&color, &color_read, wgpu::TextureAspect::All),
        (&depth, &depth_read, wgpu::TextureAspect::DepthOnly),
    ] {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect,
            },
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size * 4),
                    rows_per_image: Some(size),
                },
            },
            texture.size(),
        );
    }
    queue.submit([encoder.finish()]);
    let read = |buffer: &wgpu::Buffer| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let bytes = buffer.slice(..).get_mapped_range()?.to_vec();
        buffer.unmap();
        Ok(bytes)
    };
    // Sample the rendered textures directly before any CPU scene readback.
    let resident = voxy_render::DropletExtinctionSceneInput::new(
        ExtinctionGridView {
            origin: field.origin(),
            spacing: field.spacing(),
            shape: field.shape(),
            extinction_m_inverse: field.extinction_m_inverse(),
        },
        camera,
        size,
        size,
        2_000_000,
    )?;
    assert!(resident.decode(resident.bytes()).is_err());
    let mut scene_program = pollster::block_on(ComputeProgram::with_scene_textures(
        device,
        voxy_render::DROPLET_EXTINCTION_SCENE_SHADER,
    ))?;
    assert!(scene_program.create_job(device, resident.bytes()).is_err());
    assert!(!pollster::block_on(
        scene_program.reload_shader(voxy_render::DROPLET_EXTINCTION_SCENE_SHADER)
    )?);
    assert!(pollster::block_on(scene_program.reload_shader("invalid WGSL")).is_err());
    assert!(pollster::block_on(scene_program.reload_shader(&format!(
        "{}\n// validated reload",
        voxy_render::DROPLET_EXTINCTION_SCENE_SHADER
    )))?);
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    assert!(
        pollster::block_on(scene_program.create_scene_job(
            device,
            resident.bytes(),
            &depth_view,
            &color_view
        ))
        .is_err()
    );
    let scene_job = pollster::block_on(scene_program.create_scene_job(
        device,
        resident.bytes(),
        &color_view,
        &depth_view,
    ))?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    scene_job.encode_step(&mut encoder, resident.workgroups())?;
    let presented = texture(wgpu::TextureFormat::Rgba8Unorm);
    let presented_srgb = texture(wgpu::TextureFormat::Rgba8UnormSrgb);
    let presented_read = buffer();
    let presented_srgb_read = buffer();
    for (format, target, staging) in [
        (wgpu::TextureFormat::Rgba8Unorm, &presented, &presented_read),
        (
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &presented_srgb,
            &presented_srgb_read,
        ),
    ] {
        let blit = pollster::block_on(voxy_render::StorageColorBlit::new(device, format))?;
        let target_view = target.create_view(&Default::default());
        let output = resident.color_output(scene_job.buffer());
        for invalid in [
            voxy_render::StorageColorView { width: 0, ..output },
            voxy_render::StorageColorView {
                height: size + 1,
                ..output
            },
            voxy_render::StorageColorView {
                offset_words: u32::MAX,
                ..output
            },
        ] {
            assert!(
                pollster::block_on(blit.encode_checked(
                    device,
                    &mut encoder,
                    invalid,
                    &target_view
                ))
                .is_err()
            );
        }
        pollster::block_on(blit.encode_checked(device, &mut encoder, output, &target_view))?;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size * 4),
                    rows_per_image: Some(size),
                },
            },
            target.size(),
        );
    }
    // Qualification copies follow both presentations. No mapped data feeds either pass.
    let dispatch = scene_job.encode_snapshot(&mut encoder)?;
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let scene_outputs = resident.decode(
        &pending
            .try_read()?
            .ok_or("resident scene readback pending")?,
    )?;
    let presented_pixels = read(&presented_read)?;
    let presented_srgb_pixels = read(&presented_srgb_read)?;
    let mut maximum_presentation_byte_error = 0_u8;
    for (index, rgba) in scene_outputs.iter().enumerate() {
        for component in 0..4 {
            let linear = rgba[component].clamp(0., 1.);
            let srgb = if component == 3 {
                linear
            } else if linear <= 0.0031308 {
                12.92 * linear
            } else {
                1.055 * linear.powf(1. / 2.4) - 0.055
            };
            for (actual, value) in [
                (presented_pixels[index * 4 + component], linear),
                (presented_srgb_pixels[index * 4 + component], srgb),
            ] {
                let error = actual.abs_diff((value * 255.).round() as u8);
                maximum_presentation_byte_error = maximum_presentation_byte_error.max(error);
                assert!(
                    error <= 1,
                    "GPU presentation pixel={index} channel={component} error={error}"
                );
            }
        }
    }
    println!(
        "EXTINCTION PRESENTATION PASS pixels={} linear_and_srgb=true maximum_byte_error={maximum_presentation_byte_error} cpu_pixel_upload=false",
        scene_outputs.len()
    );
    let pixels = read(&color_read)?;
    let depth_bytes = read(&depth_read)?;
    let depths = depth_bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    let rays = voxy_render::extinction_segments_from_depth(camera, size, size, &depths)?;
    let colors = pixels
        .chunks_exact(4)
        .map(|p| std::array::from_fn(|k| p[k] as f32 / 255.))
        .collect::<Vec<[f32; 4]>>();
    let input = voxy_render::DropletExtinctionCompositeInput::new(
        ExtinctionGridView {
            origin: field.origin(),
            spacing: field.spacing(),
            shape: field.shape(),
            extinction_m_inverse: field.extinction_m_inverse(),
        },
        &rays,
        &colors,
        2_000_000,
    )?;
    let program = pollster::block_on(ComputeProgram::new(
        device,
        voxy_render::DROPLET_EXTINCTION_COMPOSITE_SHADER,
    ))?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = program
        .create_job(device, input.bytes())?
        .encode(&mut encoder, input.workgroups())?;
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let outputs = input.decode(
        &pending
            .try_read()?
            .ok_or("scene composite readback pending")?,
    )?;
    let mut foreground = 0usize;
    let mut attenuated = 0usize;
    let mut maximum_error = 0_f64;
    for (i, output) in outputs.iter().enumerate() {
        let transmittance = field.transmittance_segment(rays[i].0, rays[i].1)?;
        for k in 0..3 {
            maximum_error =
                maximum_error.max((output[k] as f64 - colors[i][k] as f64 * transmittance).abs());
        }
        if colors[i][1] > colors[i][0] * 2. && colors[i][1] > colors[i][2] * 2. {
            assert_eq!(
                *output, colors[i],
                "rendered opaque foreground must hide the cloud"
            );
            foreground += 1;
        } else if transmittance < 0.99 {
            attenuated += 1;
        }
    }
    assert!(foreground > 100 && attenuated > 100 && maximum_error < 1e-5);
    let mut maximum_resident_error = 0_f64;
    for (gpu, reference) in scene_outputs.iter().zip(&outputs) {
        for k in 0..4 {
            maximum_resident_error =
                maximum_resident_error.max((gpu[k] - reference[k]).abs() as f64);
        }
    }
    assert!(maximum_resident_error < 1e-5);
    println!(
        "EXTINCTION RESIDENT SCENE PASS pixels={} maximum_cpu_upload_path_difference={maximum_resident_error:.17e} color_depth_cpu_upload=false",
        scene_outputs.len()
    );
    let lighting = voxy_render::DirectionalScatteringOptions {
        direction_to_light: [-1., 0.2, 0.1],
        irradiance_rgb: [10., 11., 12.],
        albedo: 0.9,
        asymmetry: 0.,
        samples: 32,
    };
    let illuminated = voxy_render::DropletExtinctionSceneInput::new(
        ExtinctionGridView {
            origin: field.origin(),
            spacing: field.spacing(),
            shape: field.shape(),
            extinction_m_inverse: field.extinction_m_inverse(),
        },
        camera,
        size,
        size,
        2_000_000,
    )?
    .with_directional_scattering(lighting, 2_000_000, 12_000_000)?;
    let illuminated_job = pollster::block_on(scene_program.create_scene_job(
        device,
        illuminated.bytes(),
        &color_view,
        &depth_view,
    ))?;
    let illuminated_target = texture(wgpu::TextureFormat::Rgba8Unorm);
    let illuminated_read = buffer();
    let blit = pollster::block_on(voxy_render::StorageColorBlit::new(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
    ))?;
    let mut encoder = device.create_command_encoder(&Default::default());
    illuminated_job.encode_step(&mut encoder, illuminated.workgroups())?;
    pollster::block_on(blit.encode_checked(
        device,
        &mut encoder,
        illuminated.color_output(illuminated_job.buffer()),
        &illuminated_target.create_view(&Default::default()),
    ))?;
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &illuminated_target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &illuminated_read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(size),
            },
        },
        illuminated_target.size(),
    );
    let illuminated_dispatch = illuminated_job.encode_snapshot(&mut encoder)?;
    queue.submit([encoder.finish()]);
    let mut pending = illuminated_dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let illuminated_colors =
        illuminated.decode(&pending.try_read()?.ok_or("illuminated readback pending")?)?;
    let illuminated_pixels = read(&illuminated_read)?;
    let physical_light = DirectionalScatteringLight {
        direction_to_light: lighting.direction_to_light.map(f64::from),
        irradiance_rgb: lighting.irradiance_rgb.map(f64::from),
        single_scattering_albedo: f64::from(lighting.albedo),
        asymmetry: f64::from(lighting.asymmetry),
    };
    let mut maximum_scattering_error = 0_f64;
    let mut pixels_receiving_light = 0;
    for (i, &(start, end)) in rays.iter().enumerate() {
        let scatter = field.directional_scattered_radiance_segment(
            start,
            end,
            physical_light,
            lighting.samples,
            192,
        )?;
        let t = field.transmittance_segment(start, end)?;
        for k in 0..3 {
            let expected = f64::from(colors[i][k]) * t + scatter[k];
            maximum_scattering_error = maximum_scattering_error
                .max((f64::from(illuminated_colors[i][k]) - expected).abs());
            assert!(illuminated_colors[i][k] >= scene_outputs[i][k]);
            assert!(
                illuminated_pixels[i * 4 + k]
                    .abs_diff((illuminated_colors[i][k].clamp(0., 1.) * 255.).round() as u8)
                    <= 1
            );
        }
        assert_eq!(illuminated_colors[i][3], colors[i][3]);
        if scatter.iter().any(|v| *v > 1e-5) {
            pixels_receiving_light += 1;
        }
        if t == 1. {
            assert_eq!(illuminated_colors[i], scene_outputs[i]);
        }
    }
    assert!(maximum_scattering_error < 2e-5 && pixels_receiving_light > 1000);
    println!(
        "SINGLE SCATTERING RENDERED SCENE PASS pixels={} illuminated_pixels={pixels_receiving_light} maximum_native_rgb_error={maximum_scattering_error:.17e} foreground_unchanged=true",
        illuminated_colors.len()
    );
    if let Ok(path) = std::env::var("VOXY_SCATTERING_CAPTURE") {
        let mut comparison = Vec::with_capacity(pixels.len() * 2);
        for y in 0..size as usize {
            let row = y * size as usize * 4..(y + 1) * size as usize * 4;
            comparison.extend_from_slice(&pixels[row.clone()]);
            comparison.extend_from_slice(&illuminated_pixels[row]);
        }
        image::save_buffer(path, &comparison, size * 2, size, image::ColorType::Rgba8)?;
    }
    if let Ok(path) = std::env::var("VOXY_EXTINCTION_CAPTURE") {
        let mut comparison = Vec::with_capacity(pixels.len() * 2);
        for y in 0..size as usize {
            comparison
                .extend_from_slice(&pixels[y * size as usize * 4..(y + 1) * size as usize * 4]);
            comparison.extend_from_slice(
                &presented_pixels[y * size as usize * 4..(y + 1) * size as usize * 4],
            );
        }
        image::save_buffer(path, &comparison, size * 2, size, image::ColorType::Rgba8)?;
    }
    println!(
        "EXTINCTION RENDERED SCENE PASS pixels={} unchanged_foreground_pixels={foreground} attenuated_background_pixels={attenuated} maximum_rgb_error={maximum_error:.17e}",
        outputs.len()
    );
    Ok(())
}

// Synthetic storage fixture independently verifies HDR/alpha, row order and
// a non-square extent. This input upload is not used by scene composition.
fn verify_hdr_storage_presentation(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use wgpu::util::DeviceExt;
    let width = 13_u32;
    let height = 3_u32;
    let colors = (0..width * height)
        .map(|p| {
            [
                p as f32 / 8.,
                (p % width) as f32 / 4.,
                (p / width) as f32 + 0.125,
                (p % 5) as f32 / 4.,
            ]
        })
        .collect::<Vec<_>>();
    let mut words = vec![0x7fc00000_u32; 5];
    words.extend(colors.iter().flatten().map(|v| v.to_bits()));
    let source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("HDR row/alpha fixture"),
        contents: bytemuck::cast_slice(&words),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let target = voxy_render::ProcessedColorTarget::new(device, width, height, true)?;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR presentation qualification"),
        size: u64::from(height) * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    assert!(
        pollster::block_on(voxy_render::StorageColorBlit::new(
            device,
            wgpu::TextureFormat::Depth32Float
        ))
        .is_err()
    );
    let blit = pollster::block_on(voxy_render::StorageColorBlit::new(
        device,
        wgpu::TextureFormat::Rgba16Float,
    ))?;
    let mut encoder = device.create_command_encoder(&Default::default());
    pollster::block_on(blit.encode_checked(
        device,
        &mut encoder,
        voxy_render::StorageColorView {
            buffer: &source,
            width,
            height,
            offset_words: 5,
        },
        target.view(),
    ))?;
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: target.texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(height),
            },
        },
        target.texture().size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let mapped = staging.slice(..).get_mapped_range()?;
    for y in 0..height as usize {
        for x in 0..width as usize {
            for k in 0..4 {
                let offset = y * 256 + x * 8 + k * 2;
                let actual = u16::from_le_bytes(mapped[offset..offset + 2].try_into().unwrap());
                let expected = half::f16::from_f32(colors[y * width as usize + x][k]).to_bits();
                assert_eq!(
                    actual, expected,
                    "HDR presentation row={y} column={x} channel={k}"
                );
            }
        }
    }
    drop(mapped);
    staging.unmap();
    println!(
        "EXTINCTION HDR PRESENTATION PASS pixels={} width={width} height={height} offset_words=5 rgba16_exact=true alpha_varies=true unclamped_rgb=true",
        width * height
    );
    Ok(())
}

fn verify_single_scattering(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_render::{
        DirectionalScatteringOptions, SceneDraw, SceneMesh, SceneRenderer, SceneVertex,
    };
    let renderer = SceneRenderer::new(device, wgpu::TextureFormat::Rgba16Float);
    let camera = voxy_render::SceneCamera {
        eye: glam::Vec3::new(0.5, 0.5, -1.),
        target: glam::Vec3::new(0.5, 0.5, 0.),
        up: glam::Vec3::Y,
        projection: voxy_render::SceneProjection::Orthographic {
            left: -0.5,
            right: 0.5,
            bottom: -0.5,
            top: 0.5,
            near: 0.,
            far: 3.,
        },
    };
    let mesh = SceneMesh::new(
        [[0., 0., 2.], [1., 0., 2.], [1., 1., 2.], [0., 1., 2.]]
            .into_iter()
            .map(|position| SceneVertex {
                position,
                uv: [0.; 2],
                color: [0., 0., 0., 1.],
            })
            .collect(),
        vec![0, 2, 1, 0, 3, 2],
    )?;
    let geometry = renderer.upload_mesh(device, &mesh)?;
    let white = renderer.upload_texture(device, queue, 1, 1, &[255; 4])?;
    let transform = renderer.create_transform(device, camera.view_projection()?)?;
    let texture = |format| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("single scattering analytic scene"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    };
    let color = texture(wgpu::TextureFormat::Rgba16Float);
    let depth = texture(wgpu::TextureFormat::Depth32Float);
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.encode(
        &mut encoder,
        &color_view,
        &depth_view,
        wgpu::Color::BLACK,
        &[SceneDraw {
            geometry: &geometry,
            texture: &white,
            transform: &transform,
            overlay: false,
        }],
    );
    queue.submit([encoder.finish()]);
    let program = pollster::block_on(ComputeProgram::with_scene_textures(
        device,
        voxy_render::DROPLET_EXTINCTION_SCENE_SHADER,
    ))?;
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.5; 3],
            velocity: [0.; 3],
            mass: 1.,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )?;
    liquid.configure_droplet_population(Some(vec![true]))?;
    let gas = FiniteDropletGasGrid::new(
        [0.; 3],
        [1.; 3],
        [1; 3],
        vec![VaporCell {
            mass: 0.1,
            volume: 1.,
            temperature: 300.,
            velocity: [0.; 3],
            specific_heat_cv: 718.,
        }],
    )?;
    let mut maximum_error = 0_f64;
    let mut cases = 0;
    for sigma in [0.5_f64, 1e-6, 10., 100., 10000.] {
        let field = liquid.droplet_extinction_grid(
            &gas,
            &[(sigma / (2. * std::f64::consts::PI)).sqrt()],
            2.,
        )?;
        for sign in [-1_f32, 1.] {
            for g in [0_f32, 0.5] {
                let lighting = DirectionalScatteringOptions {
                    direction_to_light: [0., 0., sign],
                    irradiance_rgb: [4., 2., 1.],
                    albedo: 0.8,
                    asymmetry: g,
                    samples: 128,
                };
                let new_input = || {
                    voxy_render::DropletExtinctionSceneInput::new(
                        ExtinctionGridView {
                            origin: field.origin(),
                            spacing: field.spacing(),
                            shape: field.shape(),
                            extinction_m_inverse: field.extinction_m_inverse(),
                        },
                        camera,
                        1,
                        1,
                        4096,
                    )
                };
                assert!(matches!(
                    new_input()?.with_directional_scattering(lighting, 4096, 0),
                    Err(voxy_render::ComputeError::WorkBudget)
                ));
                assert!(
                    new_input()?
                        .with_directional_scattering(
                            DirectionalScatteringOptions {
                                samples: 0,
                                ..lighting
                            },
                            4096,
                            10000
                        )
                        .is_err()
                );
                assert!(
                    new_input()?
                        .with_directional_scattering(
                            DirectionalScatteringOptions {
                                asymmetry: 1.,
                                ..lighting
                            },
                            4096,
                            10000
                        )
                        .is_err()
                );
                assert!(
                    new_input()?
                        .with_directional_scattering(
                            DirectionalScatteringOptions {
                                direction_to_light: [0.; 3],
                                ..lighting
                            },
                            4096,
                            10000
                        )
                        .is_err()
                );
                let input = new_input()?.with_directional_scattering(lighting, 4096, 10000)?;
                assert_eq!(input.workgroups(), [1, 1, 1]);
                let job = pollster::block_on(program.create_scene_job(
                    device,
                    input.bytes(),
                    &color_view,
                    &depth_view,
                ))?;
                let mut encoder = device.create_command_encoder(&Default::default());
                let dispatch = job.encode(&mut encoder, input.workgroups())?;
                queue.submit([encoder.finish()]);
                let mut pending = dispatch.begin_read();
                device.poll(wgpu::PollType::wait_indefinitely())?;
                let actual =
                    input.decode(&pending.try_read()?.ok_or("scattering readback pending")?)?[0];
                // Independent homogeneous-slab integrals; no ray-marching reference here.
                let integral = if sign < 0. {
                    0.5 * (1. - (-2. * sigma).exp())
                } else {
                    sigma * (-sigma).exp()
                };
                let phase = (1. - f64::from(g).powi(2))
                    / (4.
                        * std::f64::consts::PI
                        * (1. + f64::from(g).powi(2) - 2. * f64::from(g * sign)).powf(1.5));
                for k in 0..3 {
                    let expected = f64::from(lighting.irradiance_rgb[k])
                        * f64::from(lighting.albedo)
                        * phase
                        * integral;
                    let error = (f64::from(actual[k]) - expected).abs();
                    maximum_error = maximum_error.max(error);
                    assert!(
                        error < 2e-6 && error < expected * 1e-4 + 1e-37,
                        "scattering sigma={sigma} sign={sign} g={g} channel={k} actual={} expected={expected} error={error}",
                        actual[k]
                    );
                }
                assert_eq!(actual[3], 1.);
                cases += 1;
                println!(
                    "SINGLE SCATTERING GPU SLAB sigma={sigma:.17e} toward_light_z={sign} g={g} red={:.17e}",
                    actual[0]
                );
            }
        }
    }
    println!(
        "SINGLE SCATTERING GPU PASS cases={cases} maximum_analytic_error={maximum_error:.17e} optically_thin_nonzero=true alpha_preserved=true"
    );
    Ok(())
}
