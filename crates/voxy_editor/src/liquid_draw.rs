//! Bounded, volume-preserving particle geometry from an immutable accepted fluid state.
use physics::liquid::Liquid;
use voxy_render::{SceneMesh, SceneVertex};

pub(super) fn mesh(liquid: &Liquid) -> Result<Option<SceneMesh>, String> {
    if liquid.particles().is_empty() {
        return Ok(None);
    }
    if liquid.particles().len() > 16384 {
        return Err("liquid draw particle budget exceeded".into());
    }
    let materials = liquid
        .effective_materials()
        .map_err(|e| format!("liquid draw material: {e:?}"))?;
    let mut vertices = Vec::with_capacity(liquid.particles().len() * 6);
    let mut indices = Vec::with_capacity(liquid.particles().len() * 24);
    for (p, m) in liquid.particles().iter().zip(materials) {
        // Octahedron volume = 4r^3/3, exactly matching represented fluid volume.
        let r = (3. * p.mass / (4. * m.rest_density)).cbrt();
        let points: [[f64; 3]; 6] = std::array::from_fn(|i| {
            let mut point = p.position;
            point[i / 2] += if i % 2 == 0 { r } else { -r };
            point
        });
        let color = match p.material % 3 {
            0 => [0.1, 0.65, 1., 1.],
            1 => [1., 0.65, 0.1, 1.],
            _ => [0.7, 0.3, 1., 1.],
        };
        let gpu_points = points.map(|point| point.map(|v| v as f32));
        let gpu_center = p.position.map(|v| v as f32);
        if gpu_points.iter().flatten().any(|v| !v.is_finite())
            || (0..3).any(|axis| {
                gpu_points[2 * axis][axis] <= gpu_center[axis]
                    || gpu_points[2 * axis + 1][axis] >= gpu_center[axis]
            })
        {
            return Err("liquid particle extent is not representable in GPU coordinates".into());
        }
        let base = u32::try_from(vertices.len()).map_err(|_| "liquid draw index overflow")?;
        vertices.extend(gpu_points.map(|position| SceneVertex {
            position,
            uv: [0.; 2],
            color,
        }));
        for face in [
            [0, 2, 4],
            [2, 1, 4],
            [1, 3, 4],
            [3, 0, 4],
            [2, 0, 5],
            [1, 2, 5],
            [3, 1, 5],
            [0, 3, 5],
        ] {
            indices.extend(face.map(|i| base + i as u32));
        }
    }
    SceneMesh::new(vertices, indices)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_and_centroid_follow_physical_particle() {
        let p = physics::liquid::Particle {
            position: [1., 2., 3.],
            velocity: [0.; 3],
            mass: 1.,
            material: 0,
        };
        let liquid = Liquid::new(
            vec![p],
            vec![physics::liquid::Material::WATER],
            physics::liquid::Config::default(),
        )
        .unwrap();
        let mesh = mesh(&liquid).unwrap().unwrap();
        assert_eq!(mesh.vertices().len(), 6);
        assert_eq!(mesh.indices().len(), 24);
        voxy_render::MediumBoundaryMesh::from_scene_mesh(
            &mesh,
            voxy_render::OpticalMediumId(1),
            voxy_render::OpticalMediumId(0),
            1.,
            8,
        )
        .unwrap();
        let mut volume = 0.;
        for indices in mesh.indices().chunks_exact(3) {
            let face = indices.iter().map(|&i| &mesh.vertices()[i as usize]);
            let points: Vec<_> = face
                .map(|v| {
                    glam::DVec3::from_array(v.position.map(f64::from))
                        - glam::DVec3::from_array(p.position)
                })
                .collect();
            volume += points[0].dot(points[1].cross(points[2])).abs() / 6.;
        }
        assert!((volume - 0.001).abs() < 1e-8);
        let center = mesh
            .vertices()
            .iter()
            .map(|v| glam::Vec3::from_array(v.position))
            .sum::<glam::Vec3>()
            / mesh.vertices().len() as f32;
        assert!((center - glam::Vec3::new(1., 2., 3.)).length() < 1e-6);
    }
    #[test]
    fn gpu_coordinate_collapse_rejects_the_whole_particle_mesh() {
        let particle = |position| physics::liquid::Particle {
            position,
            velocity: [0.; 3],
            mass: 1.,
            material: 0,
        };
        for center in [1e8, 1e8 + 4.] {
            let liquid = Liquid::new(
                vec![particle([0.; 3]), particle([center; 3])],
                vec![physics::liquid::Material::WATER],
                physics::liquid::Config::default(),
            )
            .unwrap();
            assert!(mesh(&liquid).unwrap_err().contains("not representable"));
            assert_eq!(liquid.particles().len(), 2);
        }
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    #[test]
    #[ignore = "requires a physical GPU adapter"]
    fn accepted_liquid_mesh_is_visible_on_gpu_and_empty_state_clears() {
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        println!("LIQUID EDITOR GPU {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut renderer =
            voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        pollster::block_on(renderer.reload_shader(&device, include_str!("material.wgsl"))).unwrap();
        let texture = renderer
            .upload_texture(&device, &queue, 1, 1, &[255; 4])
            .unwrap();
        let transform = renderer
            .create_transform(&device, glam::Mat4::IDENTITY)
            .unwrap();
        let extent = wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        };
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * 64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut scene = voxy_scene::SceneGraph::new(1);
        let node = scene
            .spawn(
                None,
                voxy_scene::Transform {
                    translation: glam::Vec3::new(0., 0., 0.5),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                voxy_gameplay::LiquidSource {
                    pulses: vec![voxy_gameplay::LiquidPulse {
                        start_s: 0.,
                        duration_s: 0.001,
                        volume_m3: 0.1,
                        speed_m_s: 0.,
                    }],
                    density_kg_m3: 1000.,
                    particle_volume_m3: 0.1,
                    nozzle_radius_m: 0.,
                    direction: [1., 0., 0.],
                    material_asset: "water".into(),
                },
            )
            .unwrap();
        let mut runtime = voxy_gameplay::SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), physics::liquid::Material::WATER)],
            physics::liquid::Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            1,
        )
        .unwrap();
        assert!(mesh(runtime.liquid()).unwrap().is_none());
        runtime.tick(&scene, 0.001, None).unwrap();
        assert!((runtime.liquid().mass() - 100.).abs() < 1e-12);
        let geometry = renderer
            .upload_mesh(&device, &mesh(runtime.liquid()).unwrap().unwrap())
            .unwrap();
        let mut fluid = voxy_render::ScreenSpaceFluidRenderer::new_with_adapter(
            &device,
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            64,
            64,
            16,
        )
        .unwrap();
        let camera = voxy_render::SceneCamera {
            eye: glam::Vec3::new(0., 0., 2.),
            target: glam::Vec3::new(0., 0., 0.5),
            up: glam::Vec3::Y,
            projection: voxy_render::SceneProjection::Perspective {
                vertical_fov: 1.,
                aspect: 1.,
                near: 0.1,
                far: 10.,
            },
        };
        let optical = optical_particles(runtime.liquid(), &[Some([3., 0.5, 0.2, 1.333])])
            .unwrap()
            .unwrap();
        fluid
            .update(&queue, camera, &optical, 1., Default::default())
            .unwrap();
        let background_mesh = SceneMesh::new(
            vec![
                SceneVertex {
                    position: [-2., -2., 0.],
                    uv: [0., 1.],
                    color: [1.; 4],
                },
                SceneVertex {
                    position: [2., -2., 0.],
                    uv: [1., 1.],
                    color: [1.; 4],
                },
                SceneVertex {
                    position: [2., 2., 0.],
                    uv: [1., 0.],
                    color: [1.; 4],
                },
                SceneVertex {
                    position: [-2., 2., 0.],
                    uv: [0., 0.],
                    color: [1.; 4],
                },
            ],
            vec![0, 1, 2, 0, 2, 3],
        )
        .unwrap();
        let background_geometry = renderer.upload_mesh(&device, &background_mesh).unwrap();
        let background_texture = renderer
            .upload_texture(
                &device,
                &queue,
                2,
                2,
                &[
                    220, 180, 120, 255, 120, 200, 120, 255, 120, 200, 120, 255, 220, 180, 120, 255,
                ],
            )
            .unwrap();
        let background_transform = renderer
            .create_transform(&device, camera.view_projection().unwrap())
            .unwrap();
        let mut baseline = Vec::new();
        for (visible, optical_view) in [
            (false, false),
            (true, false),
            (false, false),
            (false, true),
            (true, true),
            (false, true),
        ] {
            let draw = voxy_render::SceneDraw {
                geometry: &geometry,
                texture: &texture,
                transform: &transform,
                overlay: false,
            };
            let draws = if visible {
                std::slice::from_ref(&draw)
            } else {
                &[]
            };
            let views = [voxy_render::SceneView {
                viewport: [0, 0, 64, 64],
                draws,
            }];
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            if optical_view {
                fluid
                    .update(
                        &queue,
                        camera,
                        if visible { &optical } else { &[] },
                        1.,
                        Default::default(),
                    )
                    .unwrap();
                let background = voxy_render::SceneDraw {
                    geometry: &background_geometry,
                    texture: &background_texture,
                    transform: &background_transform,
                    overlay: false,
                };
                fluid.encode(
                    &renderer,
                    &mut encoder,
                    &color.create_view(&Default::default()),
                    wgpu::Color::BLACK,
                    &[background],
                );
            } else {
                renderer
                    .encode_view_frame(
                        &mut encoder,
                        &color.create_view(&Default::default()),
                        &depth.create_view(&Default::default()),
                        wgpu::Color::BLACK,
                        &views,
                        &[],
                    )
                    .unwrap();
            }
            encoder.copy_texture_to_buffer(
                color.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(64),
                    },
                },
                extent,
            );
            let submitted = queue.submit([encoder.finish()]);
            let (send, receive) = std::sync::mpsc::channel();
            readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| send.send(r).unwrap());
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submitted),
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .unwrap();
            receive
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap()
                .unwrap();
            let data = readback.slice(..).get_mapped_range().unwrap();
            let blue = data
                .chunks_exact(4)
                .filter(|p| p[2] > 150 && p[1] > 50 && p[0] < 100)
                .count();
            println!("visible={visible} blue_pixels={blue}");
            if visible {
                if optical_view {
                    let changed = data
                        .chunks_exact(4)
                        .zip(baseline.chunks_exact(4))
                        .filter(|(p, b)| p[..3] != b[..3])
                        .count();
                    println!("optical_changed_pixels={changed}");
                    assert!(changed > 100);
                } else {
                    assert!(blue > 100);
                }
                if let Some(path) = std::env::var_os(if optical_view {
                    "VOXY_LIQUID_OPTICAL_PPM"
                } else {
                    "VOXY_LIQUID_DRAW_PPM"
                }) {
                    let mut ppm = b"P6\n64 64\n255\n".to_vec();
                    for pixel in data.chunks_exact(4) {
                        ppm.extend_from_slice(&pixel[..3]);
                    }
                    std::fs::write(path, ppm).unwrap();
                }
            } else {
                assert_eq!(blue, 0);
                if optical_view {
                    if baseline.is_empty() {
                        baseline = data.to_vec();
                    } else {
                        assert_eq!(&data[..], &baseline[..]);
                    }
                }
            }
            drop(data);
            readback.unmap();
        }
        fluid
            .update(&queue, camera, &optical, 1., Default::default())
            .unwrap();
        let wide_extent = wgpu::Extent3d {
            width: 128,
            height: 64,
            depth_or_array_layers: 1,
        };
        let wide = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wide_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let wide_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wide_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let wide_read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 512 * 64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        let color_view = wide.create_view(&Default::default());
        let depth_view = wide_depth.create_view(&Default::default());
        let views = [voxy_render::SceneView {
            viewport: [0, 0, 128, 64],
            draws: &[],
        }];
        renderer
            .encode_view_frame(
                &mut encoder,
                &color_view,
                &depth_view,
                wgpu::Color::RED,
                &views,
                &[],
            )
            .unwrap();
        let background = voxy_render::SceneDraw {
            geometry: &background_geometry,
            texture: &background_texture,
            transform: &background_transform,
            overlay: false,
        };
        assert!(
            fluid
                .encode_viewport(
                    &renderer,
                    &mut encoder,
                    &color_view,
                    &depth_view,
                    [128, 64],
                    [100, 0, 64, 64],
                    wgpu::Color::BLACK,
                    &[background]
                )
                .is_err()
        );
        let background = voxy_render::SceneDraw {
            geometry: &background_geometry,
            texture: &background_texture,
            transform: &background_transform,
            overlay: false,
        };
        fluid
            .encode_viewport(
                &renderer,
                &mut encoder,
                &color_view,
                &depth_view,
                [128, 64],
                [64, 0, 64, 64],
                wgpu::Color::BLACK,
                &[background],
            )
            .unwrap();
        encoder.copy_texture_to_buffer(
            wide.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &wide_read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(512),
                    rows_per_image: Some(64),
                },
            },
            wide_extent,
        );
        let submitted = queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        wide_read
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| send.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let data = wide_read.slice(..).get_mapped_range().unwrap();
        let mut changed = 0;
        for y in 0..64 {
            for x in 0..128 {
                let p = &data[y * 512 + x * 4..y * 512 + x * 4 + 4];
                if x < 64 {
                    assert_eq!(p, &[255, 0, 0, 255]);
                } else {
                    let index = y * 256 + (x - 64) * 4;
                    if p[..3] != baseline[index..index + 3] {
                        changed += 1;
                    }
                }
            }
        }
        assert!(changed > 100);
        println!("viewport_optical_changed={changed} preserved_neighbor_pixels=4096");
        if let Some(path) = std::env::var_os("VOXY_LIQUID_VIEWPORT_PPM") {
            let mut ppm = b"P6\n128 64\n255\n".to_vec();
            for p in data.chunks_exact(4) {
                ppm.extend_from_slice(&p[..3]);
            }
            std::fs::write(path, ppm).unwrap();
        }
        drop(data);
        wide_read.unmap();
        assert!(pollster::block_on(scope.pop()).is_none());
    }
}

/// Missing explicit optical materials selects the diagnostic mesh; invalid data rejects.
pub(super) fn optical_particles(
    liquid: &Liquid,
    optics: &[Option<[f32; 4]>],
) -> Result<Option<Vec<voxy_render::FluidRenderParticle>>, String> {
    if optics.is_empty() || optics.iter().any(Option::is_none) {
        return Ok(None);
    }
    if liquid
        .particles()
        .iter()
        .any(|p| optics.get(p.material).is_none_or(Option::is_none))
    {
        return Ok(None);
    }
    if liquid.particles().len() > 16384 {
        return Err("optical liquid particle budget exceeded".into());
    }
    let radii = liquid
        .equivalent_sphere_radii()
        .map_err(|e| format!("liquid optical volume: {e:?}"))?;
    let mut particles = Vec::with_capacity(radii.len());
    for (p, r) in liquid.particles().iter().zip(radii) {
        let position_radius = [
            p.position[0] as f32,
            p.position[1] as f32,
            p.position[2] as f32,
            r as f32,
        ];
        let material = optics[p.material].unwrap();
        if position_radius.iter().any(|v| !v.is_finite())
            || position_radius[3] <= 0.
            || material.iter().any(|v| !v.is_finite())
            || material[..3].iter().any(|v| *v < 0.)
            || material[3] < 1.
        {
            return Err("invalid liquid optical snapshot".into());
        }
        particles.push(voxy_render::FluidRenderParticle {
            position_radius,
            absorption_ior: material,
        });
    }
    Ok(Some(particles))
}

#[cfg(test)]
mod optical_tests {
    use super::*;
    #[test]
    fn explicit_material_and_volume_admission() {
        let liquid = Liquid::new(
            vec![physics::liquid::Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            }],
            vec![physics::liquid::Material::WATER],
            Default::default(),
        )
        .unwrap();
        assert!(optical_particles(&liquid, &[None]).unwrap().is_none());
        let optics = [Some([0.2, 0.1, 0.05, 1.333])];
        let particles = optical_particles(&liquid, &optics).unwrap().unwrap();
        assert_eq!(particles[0].absorption_ior, optics[0].unwrap());
        let r = f64::from(particles[0].position_radius[3]);
        assert!((4. * std::f64::consts::PI * r * r * r / 3. - 0.001).abs() < 1e-9);
        assert!(optical_particles(&liquid, &[Some([-1., 0., 0., 1.333])]).is_err());
    }
}
