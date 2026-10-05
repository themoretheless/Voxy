//! GPU snapshots of the existing neutral skeleton and volumetric tissue demo.
#[path = "../src/biomechanics_demo.rs"]
mod biomechanics_demo;
#[path = "../src/tissue_demo.rs"]
mod tissue_demo;
use glam::{DMat4, Mat4, Vec3};
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};
use voxy_render::{ModelAsset, ModelLimits, SceneMesh};
fn displayed_meshes(
    demo: &tissue_demo::TissueDemo,
    model: Option<&ModelAsset>,
    phase: f64,
) -> Result<Vec<SceneMesh>, Box<dyn std::error::Error>> {
    let Some(model) = model else {
        return Ok(vec![demo.mesh()?]);
    };
    let pose = model.sample_pose_phase(Some(0), phase)?;
    let mut meshes = Vec::new();
    for mesh in model.scene_meshes(&pose)? {
        let mut normals = vec![Vec3::ZERO; mesh.vertices().len()];
        for triangle in mesh.indices().chunks_exact(3) {
            let [a, b, c] = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            let point = |index: usize| Vec3::from_array(mesh.vertices()[index].position);
            let normal = (point(b) - point(a)).cross(point(c) - point(a));
            for node in [a, b, c] {
                normals[node] += normal;
            }
        }
        let vertices = mesh
            .vertices()
            .iter()
            .zip(normals)
            .map(|(vertex, normal)| {
                let light = 0.25
                    + 0.75
                        * normal
                            .normalize_or_zero()
                            .dot(Vec3::new(-0.5, 0.8, 1.).normalize())
                            .max(0.);
                let mut vertex = *vertex;
                for channel in &mut vertex.color[..3] {
                    *channel *= light;
                }
                vertex
            })
            .collect();
        meshes.push(SceneMesh::new(vertices, mesh.indices().to_vec())?);
    }
    meshes.push(demo.tissue_mesh()?);
    Ok(meshes)
}
fn contact_positions(
    model: &ModelAsset,
    phase: f64,
) -> Result<(Vec<[f64; 3]>, Vec<[usize; 3]>), Box<dyn std::error::Error>> {
    let pose = model.sample_pose_phase(Some(0), phase)?;
    let mut positions = Vec::new();
    let mut faces = Vec::new();
    for mesh in model.scene_meshes(&pose)? {
        let base = positions.len();
        positions.extend(mesh.vertices().iter().map(|v| v.position.map(f64::from)));
        faces.extend(mesh.indices().chunks_exact(3).map(|f| {
            [
                base + f[0] as usize,
                base + f[1] as usize,
                base + f[2] as usize,
            ]
        }));
    }
    Ok((positions, faces))
}
fn contact_positions64(
    model: &ModelAsset,
    phase: f64,
) -> Result<(Vec<[f64; 3]>, Vec<[usize; 3]>), Box<dyn std::error::Error>> {
    let pose = model.sample_pose_phase64(Some(0), phase)?;
    contact_positions_from_pose64(model, &pose)
}
fn contact_positions_from_pose64(
    model: &ModelAsset,
    pose: &voxy_animation::Pose64,
) -> Result<(Vec<[f64; 3]>, Vec<[usize; 3]>), Box<dyn std::error::Error>> {
    let mut positions = Vec::new();
    let mut faces = Vec::new();
    for mesh in model.scene_surfaces64(pose)? {
        let base = positions.len();
        positions.extend(mesh.positions);
        faces.extend(mesh.indices.chunks_exact(3).map(|f| {
            [
                base + f[0] as usize,
                base + f[1] as usize,
                base + f[2] as usize,
            ]
        }));
    }
    Ok((positions, faces))
}
/// Both moving boundaries derive from one pose; used by runtime and tests.
fn imported_contact_sample64(
    model: &ModelAsset,
    reference: &[DMat4],
    domains: &[std::sync::Arc<physics::biomechanics::PrescribedTriangleSurface>],
    phase: f64,
) -> Result<
    (
        Vec<DMat4>,
        Vec<std::sync::Arc<physics::biomechanics::PrescribedTriangleSurface>>,
    ),
    &'static str,
> {
    let pose = model
        .sample_pose_phase64(Some(0), phase)
        .map_err(|_| "imported physical pose sampling failed")?;
    let current = pose
        .skin_matrices(&model.skeleton)
        .map_err(|_| "imported physical palette failed")?;
    if current.len() != reference.len() || domains.is_empty() {
        return Err("imported contact binding mismatch");
    }
    if reference.iter().any(|matrix| {
        let determinant = matrix.determinant();
        !matrix.is_finite() || !determinant.is_finite() || determinant == 0.
    }) {
        return Err("invalid imported physical reference");
    }
    let palette: Vec<_> = current
        .iter()
        .zip(reference)
        .map(|(a, b)| *a * b.inverse())
        .collect();
    if palette.iter().any(|m| !m.is_finite()) {
        return Err("nonfinite imported physical palette");
    }
    let (positions, faces) = contact_positions_from_pose64(model, &pose)
        .map_err(|_| "imported physical surface failed")?;
    if domains.iter().any(|domain| domain.faces() != faces) {
        return Err("imported collision topology changed");
    }
    let surfaces = domains
        .iter()
        .map(|domain| {
            domain
                .with_positions(positions.clone())
                .map(std::sync::Arc::new)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((palette, surfaces))
}
fn imported_attachments(
    model: &ModelAsset,
    reference: &[Mat4],
) -> Result<([[f64; 3]; 4], [usize; 4]), Box<dyn std::error::Error>> {
    let names = [
        "Skeleton_torso_joint_2",
        "Skeleton_torso_joint_2",
        "leg_joint_R_1",
        "leg_joint_L_1",
    ];
    let mut joints = [0; 4];
    let mut centers = [[0.; 3]; 4];
    for (i, name) in names.into_iter().enumerate() {
        joints[i] = usize::from(model.resolve_joint_name(name)?);
        let joint = &model.skeleton.joints()[joints[i]];
        let global = reference[joints[i]] * joint.inverse_bind.inverse();
        let offset = if i < 2 {
            Vec3::new(if i == 0 { -0.08 } else { 0.08 }, 0., 0.12)
        } else {
            Vec3::new(0., -0.1, -0.08)
        };
        centers[i] = (global.transform_point3(Vec3::ZERO) + offset)
            .to_array()
            .map(f64::from);
    }
    println!("IMPORTED support centers: {centers:?}");
    Ok((centers, joints))
}
fn imported_tissues(
    model: &ModelAsset,
    reference: &[Mat4],
) -> Result<tissue_demo::TissueDemo, Box<dyn std::error::Error>> {
    let (centers, joints) = imported_attachments(model, reference)?;
    Ok(tissue_demo::TissueDemo::body_at_centers(centers, joints))
}
fn attachment_domains(
    surface: &physics::biomechanics::PrescribedTriangleSurface,
    centers: [[f64; 3]; 4],
) -> Result<Vec<std::sync::Arc<physics::biomechanics::PrescribedTriangleSurface>>, &'static str> {
    centers
        .iter()
        .enumerate()
        .map(|(region, center)| {
            // Authored reference-space attachment volumes, in metres. They cover
            // each illustrative pad and its shared base with the original skin.
            // Never recomputed from current intersections or animated poses.
            let half = if region < 2 {
                [0.11, 0.10, 0.10]
            } else {
                [0.12, 0.12, 0.11]
            };
            let enabled = surface
                .faces()
                .iter()
                .map(|face| {
                    let points = face.map(|node| surface.positions()[node]);
                    let overlaps = (0..3).all(|axis| {
                        let lo = points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
                        let hi = points
                            .iter()
                            .map(|p| p[axis])
                            .fold(f64::NEG_INFINITY, f64::max);
                        lo <= center[axis] + half[axis] && hi >= center[axis] - half[axis]
                    });
                    !overlaps
                })
                .collect();
            surface.with_contact_faces(enabled).map(std::sync::Arc::new)
        })
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: body_motion_snapshot OUTPUT.png [FRAME_DIRECTORY]")?;
    if path == "--contact-motion-bench" {
        let model = ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            ModelLimits::default(),
        )?;
        let time = 0.275;
        println!(
            "precision,time_s,dt_s,maximum_vertex_motion_m,maximum_speed_m_s,unchanged_source_vertices"
        );
        for precision in ["render_f32", "physical_f64"] {
            let positions = |time: f64| -> Result<Vec<[f64; 3]>, Box<dyn std::error::Error>> {
                if precision == "render_f32" {
                    return Ok(contact_positions(&model, time / 2.)?.0);
                }
                let pose = model.sample_pose_phase64(Some(0), time / 2.)?;
                Ok(model
                    .scene_surfaces64(&pose)?
                    .into_iter()
                    .flat_map(|m| m.positions)
                    .collect())
            };
            let baseline = positions(time)?;
            for dt in [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9] {
                let next = positions(time + dt)?;
                let mut maximum = 0.0_f64;
                let mut unchanged = 0;
                for (a, b) in baseline.iter().zip(&next) {
                    if a == b {
                        unchanged += 1;
                    }
                    let distance = (0..3)
                        .map(|axis| (b[axis] - a[axis]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    maximum = maximum.max(distance);
                }
                println!(
                    "{precision},{time:.17e},{dt:.17e},{maximum:.17e},{:.17e},{unchanged}",
                    maximum / dt
                );
            }
        }
        return Ok(());
    }
    if path == "--cpu-bench" {
        let mut demo = tissue_demo::TissueDemo::body();
        let start = std::time::Instant::now();
        let trace_path = std::env::args().nth(2);
        let mut trace = String::from("time_s,sample,offset_x_m,offset_y_m,offset_z_m,volume_m3\n");
        for step in 0..=2880 {
            if step > 0 {
                demo.advance(1. / 240.)?;
            }
            if trace_path.is_some() && step % 12 == 0 {
                use std::fmt::Write;
                let volumes = demo.body_volumes_m3();
                for (sample, offset) in demo.body_secondary_offsets().iter().enumerate() {
                    writeln!(
                        trace,
                        "{:.6},{sample},{:.12},{:.12},{:.12},{:.12}",
                        f64::from(step) / 240.,
                        offset[0],
                        offset[1],
                        offset[2],
                        volumes[sample]
                    )?;
                }
            }
        }
        if let Some(path) = trace_path {
            std::fs::write(path, trace)?;
        }
        println!(
            "CPU physics 12 seconds: {:.9} wall seconds",
            start.elapsed().as_secs_f64()
        );
        println!(
            "step counts (accepted, rejected, depth): {:?}",
            demo.body_step_counts()
        );
        println!("energy receipts: {:?}", demo.body_energy_receipts()?);
        println!(
            "thermal cells (min K, max K, stored J): {:?}",
            demo.body_thermal_diagnostics()?
        );
        println!("secondary offsets: {:?}", demo.body_secondary_offsets());
        return Ok(());
    }
    let remaining: Vec<_> = std::env::args().skip(2).collect();
    let close_up = remaining.iter().any(|arg| arg == "--close-up");
    let cesium = remaining.iter().any(|arg| arg == "--cesium");
    let contact = remaining.iter().any(|arg| arg == "--contact");
    let wide_contact = remaining.iter().any(|arg| arg == "--wide-contact");
    if wide_contact && !contact {
        return Err("--wide-contact requires --contact".into());
    }
    if contact && !cesium {
        return Err("--contact requires --cesium".into());
    }
    let imported = cesium
        .then(|| {
            ModelAsset::parse(
                include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
                &[],
                ModelLimits::default(),
            )
        })
        .transpose()?;
    let duration = imported
        .as_ref()
        .map_or(12., |model| f64::from(model.animations[0].duration()));
    let reference = imported
        .as_ref()
        .map(|model| {
            model
                .sample_pose_phase(Some(0), 0.)?
                .skin_matrices(&model.skeleton)
                .map_err(|error| voxy_render::ModelError(error.to_string()))
        })
        .transpose()?;
    let palette_at = |time: f64| -> Result<Vec<Mat4>, &'static str> {
        let model = imported.as_ref().ok_or("missing imported rig")?;
        let current = model
            .sample_pose_phase(Some(0), (time / duration).min(1.))
            .map_err(|_| "imported pose sampling failed")?
            .skin_matrices(&model.skeleton)
            .map_err(|_| "imported palette failed")?;
        Ok(current
            .iter()
            .zip(reference.as_ref().unwrap())
            .map(|(current, reference)| *current * reference.inverse())
            .collect())
    };
    let reference64 = imported
        .as_ref()
        .map(|model| {
            model
                .sample_pose_phase64(Some(0), 0.)?
                .skin_matrices(&model.skeleton)
                .map_err(|error| voxy_render::ModelError(error.to_string()))
        })
        .transpose()?;
    if remaining.iter().any(|arg| {
        arg.starts_with("--")
            && arg != "--close-up"
            && arg != "--cesium"
            && arg != "--contact"
            && arg != "--wide-contact"
    }) {
        return Err(
            "usage: body_motion_snapshot OUTPUT.png [FRAME_DIRECTORY] [--close-up] [--cesium] [--contact] [--wide-contact]"
                .into(),
        );
    }
    let directories: Vec<_> = remaining
        .iter()
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    if directories.len() > 1 {
        return Err("only one frame directory is supported".into());
    }
    let frame_directory = directories.first().map(std::path::PathBuf::from);
    if let Some(directory) = &frame_directory {
        std::fs::create_dir_all(directory)?;
    }
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("BODY GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut demo = if let Some(model) = &imported {
        imported_tissues(model, reference.as_ref().unwrap())?
    } else {
        tissue_demo::TissueDemo::body()
    };
    let contact_reference = if contact {
        let sample = if wide_contact {
            contact_positions64
        } else {
            contact_positions
        };
        let (positions, faces) =
            sample(imported.as_ref().ok_or("missing imported character")?, 0.)?;
        println!(
            "CONTACT imported mesh: {} vertices, {} triangles",
            positions.len(),
            faces.len()
        );
        let surface = std::sync::Arc::new(physics::biomechanics::PrescribedTriangleSurface::new(
            positions, faces, 0.0001, 0.003, 100.,
        )?);
        let (centers, _) =
            imported_attachments(imported.as_ref().unwrap(), reference.as_ref().unwrap())?;
        let domains = attachment_domains(&surface, centers)?;
        for (region, domain) in domains.iter().enumerate() {
            println!(
                "CONTACT region {region}: {} enabled, {} excluded source triangles",
                domain.contact_faces().iter().filter(|&&v| v).count(),
                domain.contact_faces().iter().filter(|&&v| !v).count()
            );
        }
        demo.bind_contact_surfaces(&domains)?;
        Some(domains)
    } else {
        None
    };
    let meshes = displayed_meshes(&demo, imported.as_ref(), 0.)?;
    let mut geometries = meshes
        .iter()
        .map(|mesh| renderer.reserve_geometry(&device, mesh.vertices().len(), mesh.indices().len()))
        .collect::<Result<Vec<_>, _>>()?;
    let images = if cesium {
        ModelAsset::decode_embedded_images(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            voxy_render::ImageLimits::default(),
            16,
        )?
    } else {
        Vec::new()
    };
    let mut textures = Vec::new();
    for index in 0..meshes.len() {
        let material = imported
            .as_ref()
            .and_then(|model| model.primitives.get(index))
            .and_then(|primitive| primitive.base_color_texture);
        let texture = if let Some(material) = material {
            let image = images
                .get(material.image)
                .ok_or("missing imported material image")?;
            if material.use_mips {
                renderer.upload_image_mips(
                    &device,
                    &queue,
                    &image.mip_chain(),
                    material.sampling,
                )?
            } else {
                renderer.upload_image(&device, &queue, image, material.sampling)?
            }
        } else {
            renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?
        };
        textures.push(texture);
    }
    let camera = SceneCamera {
        eye: if cesium && close_up {
            Vec3::new(0.85, 0.9, 1.15)
        } else if cesium {
            Vec3::new(2.2, 1., 3.)
        } else if close_up {
            Vec3::new(0., 0.65, 4.6)
        } else {
            Vec3::new(2.8, 0.4, 6.5)
        },
        target: if cesium && close_up {
            Vec3::new(0., 0.7, 0.)
        } else if cesium {
            Vec3::new(0., 0.8, 0.)
        } else if close_up {
            Vec3::new(0., 0.25, 0.)
        } else {
            Vec3::ZERO
        },
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 45_f32.to_radians(),
            aspect: 1.,
            near: 0.1,
            far: 30.,
        },
    };
    let transform = renderer.create_transform(&device, camera.view_projection()?)?;
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("neutral body snapshot"),
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("body pixels"),
        size: 512 * 512 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut frames = Vec::new();
    let mut motion = String::from(
        "time_s,sample,offset_x_m,offset_y_m,offset_z_m,volume_m3,mechanical_change_j,support_work_j,viscous_heat_j,numerical_defect_j\n",
    );
    let mut elapsed = 0;
    let snapshots = if cesium {
        [0, 120, 240, 480]
    } else {
        [0, 168, 1080, 2160]
    };
    let steps: Vec<usize> = if cesium && frame_directory.is_some() {
        (0..=40).map(|frame| frame * 12).collect()
    } else if frame_directory.is_some() {
        (0..=240).map(|frame| frame * 12).collect()
    } else {
        snapshots.to_vec()
    };
    let render_start = std::time::Instant::now();
    for step in steps {
        while elapsed < step {
            if cesium {
                if let Some(reference) = &contact_reference {
                    demo.advance_with_palette64_and_surfaces(1. / 240., |time| {
                        if wide_contact {
                            return imported_contact_sample64(
                                imported.as_ref().ok_or("missing imported character")?,
                                reference64.as_ref().ok_or("missing imported physical reference")?,
                                reference,
                                (time / duration).min(1.),
                            );
                        }
                        let sample = contact_positions;
                        let (positions, faces) = sample(
                            imported.as_ref().ok_or("missing imported character")?,
                            (time / duration).min(1.),
                        )
                        .map_err(|_| "imported collision pose failed")?;
                        if faces != reference[0].faces() {
                            return Err("imported collision topology changed");
                        }
                        let surfaces = reference
                            .iter()
                            .map(|domain| {
                                domain
                                    .with_positions(positions.clone())
                                    .map(std::sync::Arc::new)
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let palette = palette_at(time)?.iter().map(|m| DMat4::from_cols_array(&m.to_cols_array().map(f64::from))).collect();
                        Ok((palette, surfaces))
                    }).map_err(|error| {
                        format!("{error}; rejected frame step={} time_s={:.9}; last committed energy receipts={:?}; last committed refinement={:?}", elapsed + 1, (elapsed + 1) as f64 / 240., demo.body_energy_receipts(), demo.body_step_counts())
                    })?;
                } else {
                    demo.advance_with_palette(1. / 240., &palette_at)?;
                }
            } else {
                demo.advance(1. / 240.)?;
            }
            elapsed += 1;
        }
        let volumes = demo.body_volumes_m3();
        let energy = demo.body_energy_receipts()?;
        let offsets = if cesium {
            demo.secondary_offsets_for_palette(&palette_at(step as f64 / 240.)?)
        } else {
            demo.body_secondary_offsets()
        };
        for (sample, offset) in offsets.iter().enumerate() {
            use std::fmt::Write;
            writeln!(
                motion,
                "{:.6},{sample},{:.9},{:.9},{:.9},{:.12},{:.12},{:.12},{:.12},{:.12}",
                step as f64 / 240.,
                offset[0],
                offset[1],
                offset[2],
                volumes[sample],
                energy[sample][0],
                energy[sample][1],
                energy[sample][2],
                energy[sample][3]
            )?;
        }
        let meshes = displayed_meshes(
            &demo,
            imported.as_ref(),
            (step as f64 / (240. * duration)).min(1.),
        )?;
        if meshes.len() != geometries.len() {
            return Err("display primitive count changed".into());
        }
        for (geometry, mesh) in geometries.iter_mut().zip(&meshes) {
            geometry.update(&queue, mesh)?;
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&Default::default()),
            &depth.create_view(&Default::default()),
            wgpu::Color {
                r: 0.025,
                g: 0.04,
                b: 0.065,
                a: 1.,
            },
            &geometries
                .iter()
                .zip(&textures)
                .map(|(geometry, texture)| SceneDraw {
                    geometry,
                    texture,
                    transform: &transform,
                    overlay: false,
                })
                .collect::<Vec<_>>(),
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(2048),
                    rows_per_image: Some(512),
                },
            },
            color.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let pixels = readback.slice(..).get_mapped_range()?;
        if let Some(directory) = &frame_directory {
            image::save_buffer(
                directory.join(format!("frame-{:04}.png", step / 12)),
                &pixels,
                512,
                512,
                image::ColorType::Rgba8,
            )?;
        }
        if snapshots.contains(&step) {
            frames.push(pixels.to_vec());
        }
        drop(pixels);
        readback.unmap();
        if snapshots.contains(&step) {
            println!("frame t={:.2}", step as f64 / 240.);
        }
    }
    println!(
        "snapshot simulation + render seconds={:.9}; step receipts={:?}",
        render_start.elapsed().as_secs_f64(),
        demo.body_step_counts()
    );
    if frames.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("body frames did not change".into());
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let mut strip = Vec::with_capacity(4 * 512 * 512 * 4);
    for row in 0..512 {
        for frame in &frames {
            strip.extend_from_slice(&frame[row * 2048..(row + 1) * 2048]);
        }
    }
    image::save_buffer(path, &strip, 2048, 512, image::ColorType::Rgba8)?;
    if let Some(directory) = &frame_directory {
        std::fs::write(directory.join("secondary-motion.csv"), motion)?;
    }
    Ok(())
}

#[cfg(test)]
mod collision_tests {
    use super::*;
    use crate::tissue_demo::TissueDemo;
    use std::sync::Arc;
    #[test]
    fn imported_contact_passes_previous_contact_initialization_rejections() {
        let model = ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            ModelLimits::default(),
        )
        .unwrap();
        let reference = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let mut demo = imported_tissues(&model, &reference).unwrap();
        let (positions, faces) = contact_positions(&model, 0.).unwrap();
        let surface = physics::biomechanics::PrescribedTriangleSurface::new(
            positions, faces, 0.0001, 0.003, 100.,
        )
        .unwrap();
        let (centers, _) = imported_attachments(&model, &reference).unwrap();
        let domains = attachment_domains(&surface, centers).unwrap();
        demo.bind_contact_surfaces(&domains).unwrap();
        // Covers the previously failing initial guesses at steps 65 and 66.
        // The full 480-step clip is qualified separately by the ignored test.
        for step in 1..=68 {
            demo.advance_with_palette_and_surfaces(1. / 240., |time| {
                let phase = (time / 2.).min(1.);
                let palette = model
                    .sample_pose_phase(Some(0), phase)
                    .map_err(|_| "fixture pose failed")?
                    .skin_matrices(&model.skeleton)
                    .map_err(|_| "fixture palette failed")?;
                let relative = palette
                    .iter()
                    .zip(&reference)
                    .map(|(a, b)| *a * b.inverse())
                    .collect();
                let (positions, faces) =
                    contact_positions(&model, phase).map_err(|_| "fixture contact pose failed")?;
                if faces != domains[0].faces() {
                    return Err("fixture topology changed");
                }
                let surfaces = domains
                    .iter()
                    .map(|domain| {
                        domain
                            .with_positions(positions.clone())
                            .map(std::sync::Arc::new)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((relative, surfaces))
            })
            .unwrap_or_else(|error| panic!("imported contact rejected step {step}: {error}"));
        }
        for receipt in demo.body_energy_receipts().unwrap() {
            assert!((receipt[0] - receipt[1] + receipt[2] - receipt[3]).abs() < 1e-8);
        }
    }
    #[test]
    fn wide_imported_contact_preserves_open_path_and_energy() {
        check_wide_imported_contact(68);
    }
    #[test]
    #[ignore = "full 480-step imported contact qualification; run explicitly"]
    fn wide_imported_contact_completes_full_clip() {
        check_wide_imported_contact(480);
    }
    fn paired_imported_region_steps(
        steps: usize,
    ) -> (
        TissueDemo,
        TissueDemo,
        Vec<(
            Vec<DMat4>,
            Vec<Arc<physics::biomechanics::PrescribedTriangleSurface>>,
        )>,
    ) {
        let model = ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            ModelLimits::default(),
        )
        .unwrap();
        let reference32 = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let reference64 = model
            .sample_pose_phase64(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let mut serial = imported_tissues(&model, &reference32).unwrap();
        let mut parallel = imported_tissues(&model, &reference32).unwrap();
        let (positions, faces) = contact_positions64(&model, 0.).unwrap();
        let surface = physics::biomechanics::PrescribedTriangleSurface::new(
            positions, faces, 0.0001, 0.003, 100.,
        )
        .unwrap();
        let (centers, _) = imported_attachments(&model, &reference32).unwrap();
        let domains = attachment_domains(&surface, centers).unwrap();
        serial.bind_contact_surfaces(&domains).unwrap();
        parallel.bind_contact_surfaces(&domains).unwrap();
        let samples = (1..=steps)
            .map(|step| {
                imported_contact_sample64(&model, &reference64, &domains, step as f64 / 480.)
                    .unwrap()
            })
            .collect();
        (serial, parallel, samples)
    }
    #[test]
    fn imported_parallel_regions_match_serial_complete_state() {
        let (mut serial, mut parallel, samples) = paired_imported_region_steps(8);
        for (palette, surfaces) in samples {
            serial
                .step_body_with_contact64_workers(&palette, 0.5, Some(surfaces.clone()), 1)
                .unwrap();
            parallel
                .step_body_with_contact64_workers(&palette, 0.5, Some(surfaces), 4)
                .unwrap();
            assert_eq!(format!("{serial:?}"), format!("{parallel:?}"));
        }
    }
    #[test]
    #[ignore = "manual imported region parallelism benchmark"]
    fn imported_region_parallelism_microbenchmark() {
        use std::time::Instant;
        for _ in 0..3 {
            let (mut serial, mut parallel, samples) = paired_imported_region_steps(8);
            let start = Instant::now();
            for (palette, surfaces) in &samples {
                serial
                    .step_body_with_contact64_workers(palette, 0.5, Some(surfaces.clone()), 1)
                    .unwrap();
            }
            let sequential = start.elapsed();
            let start = Instant::now();
            for (palette, surfaces) in &samples {
                parallel
                    .step_body_with_contact64_workers(palette, 0.5, Some(surfaces.clone()), 4)
                    .unwrap();
            }
            let concurrent = start.elapsed();
            assert_eq!(format!("{serial:?}"), format!("{parallel:?}"));
            eprintln!(
                "IMPORTED_REGION_PARALLEL_BENCH steps=8 serial_ns={} parallel_ns={}",
                sequential.as_nanos(),
                concurrent.as_nanos()
            );
        }
    }
    #[test]
    fn wide_imported_contact_commits_initial_motion() {
        check_wide_imported_contact(8);
    }
    fn check_wide_imported_contact(steps: usize) {
        let model = ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            ModelLimits::default(),
        )
        .unwrap();
        let reference = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let reference64 = model
            .sample_pose_phase64(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let mut demo = imported_tissues(&model, &reference).unwrap();
        let (positions, faces) = contact_positions64(&model, 0.).unwrap();
        let surface = physics::biomechanics::PrescribedTriangleSurface::new(
            positions, faces, 0.0001, 0.003, 100.,
        )
        .unwrap();
        let (centers, _) = imported_attachments(&model, &reference).unwrap();
        let domains = attachment_domains(&surface, centers).unwrap();
        demo.bind_contact_surfaces(&domains).unwrap();
        // Covers the previously failing initial guesses at steps 65 and 66.
        // The full 480-step clip is qualified separately by the ignored test.
        for step in 1..=steps {
            demo.advance_with_palette64_and_surfaces(1. / 240., |time| {
                imported_contact_sample64(&model, &reference64, &domains, (time / 2.).min(1.))
            })
            .unwrap_or_else(|error| panic!("imported contact rejected step {step}: {error}"));
            if step % 12 == 0 || step == steps {
                eprintln!("WIDE_IMPORTED_CONTACT_COMMITTED step={step}");
            }
        }
        for receipt in demo.body_energy_receipts().unwrap() {
            assert!((receipt[0] - receipt[1] + receipt[2] - receipt[3]).abs() < 1e-8);
        }
    }
    #[test]
    fn shared_contact_provider_rejects_invalid_bindings_without_advancing_tissue() {
        let model = ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            ModelLimits::default(),
        )
        .unwrap();
        let reference32 = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let reference = model
            .sample_pose_phase64(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let (positions, faces) = contact_positions64(&model, 0.).unwrap();
        let surface = physics::biomechanics::PrescribedTriangleSurface::new(
            positions.clone(),
            faces.clone(),
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let (centers, _) = imported_attachments(&model, &reference32).unwrap();
        let domains = attachment_domains(&surface, centers).unwrap();
        assert!(imported_contact_sample64(&model, &reference[1..], &domains, 0.).is_err());
        assert!(imported_contact_sample64(&model, &reference, &[], 0.).is_err());
        let mut invalid = reference.clone();
        invalid[0] = DMat4::ZERO;
        assert!(imported_contact_sample64(&model, &invalid, &domains, 0.).is_err());
        invalid[0] = DMat4::from_cols_array(&[f64::NAN; 16]);
        assert!(imported_contact_sample64(&model, &invalid, &domains, 0.).is_err());
        let mut changed_faces = faces;
        changed_faces.reverse();
        let foreign = std::sync::Arc::new(
            physics::biomechanics::PrescribedTriangleSurface::new(
                positions,
                changed_faces,
                0.0001,
                0.003,
                100.,
            )
            .unwrap(),
        );
        assert!(imported_contact_sample64(&model, &reference, &[foreign], 0.).is_err());
        for phase in [0., 0.1375, 0.1375 + 0.5e-9, 0.275, 0.5, 0.75, 1.] {
            let (palette, surfaces) =
                imported_contact_sample64(&model, &reference, &domains, phase).unwrap();
            let separate = model
                .sample_pose_phase64(Some(0), phase)
                .unwrap()
                .skin_matrices(&model.skeleton)
                .unwrap();
            for (actual, (a, b)) in palette.iter().zip(separate.iter().zip(&reference)) {
                assert_eq!(
                    actual.to_cols_array().map(f64::to_bits),
                    (*a * b.inverse()).to_cols_array().map(f64::to_bits)
                );
            }
            let (positions, _) = contact_positions64(&model, phase).unwrap();
            for surface in &surfaces {
                for (actual, expected) in surface.positions().iter().zip(&positions) {
                    assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
                }
            }
        }
        let mut demo = imported_tissues(&model, &reference32).unwrap();
        demo.bind_contact_surfaces(&domains).unwrap();
        let before = format!("{demo:?}");
        for phase in [f64::NAN, -0.1, 1.1] {
            assert!(
                demo.advance_with_palette64_and_surfaces(1. / 240., |_| imported_contact_sample64(
                    &model, &reference, &domains, phase
                ))
                .is_err()
            );
            assert_eq!(format!("{demo:?}"), before);
        }
    }
    #[test]
    fn collision_extraction_preserves_imported_topology_and_world_pose() {
        let model = ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            ModelLimits::default(),
        )
        .unwrap();
        let (initial, faces) = contact_positions(&model, 0.).unwrap();
        assert_eq!(initial.len(), 3273);
        assert_eq!(faces.len(), 4672);
        let reference = physics::biomechanics::PrescribedTriangleSurface::new(
            initial.clone(),
            faces.clone(),
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let palette = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let mut demo = imported_tissues(&model, &palette).unwrap();
        let before = demo.body_energy_receipts().unwrap();
        assert_eq!(
            demo.bind_contact_surface(std::sync::Arc::new(reference.clone())),
            Err("closed surface contact gap")
        );
        assert_eq!(demo.body_energy_receipts().unwrap(), before);
        let (centers, _) = imported_attachments(&model, &palette).unwrap();
        let domains = attachment_domains(&reference, centers).unwrap();
        assert_eq!(
            domains
                .iter()
                .map(|domain| domain.contact_faces().iter().filter(|&&v| !v).count())
                .collect::<Vec<_>>(),
            vec![32, 115, 132, 157]
        );
        demo.bind_contact_surfaces(&domains).unwrap();
        let mut changed = false;
        for phase in [0.25, 0.5, 0.75, 1.] {
            let (positions, next_faces) = contact_positions(&model, phase).unwrap();
            assert_eq!(next_faces, faces);
            changed |= positions != initial;
            reference.with_positions(positions.clone()).unwrap();
            for domain in &domains {
                let moved = domain.with_positions(positions.clone()).unwrap();
                assert_eq!(moved.contact_faces(), domain.contact_faces());
                assert_eq!(moved.faces(), faces);
            }
            let meshes = model
                .scene_meshes(&model.sample_pose_phase(Some(0), phase).unwrap())
                .unwrap();
            let displayed: Vec<_> = meshes
                .iter()
                .flat_map(|mesh| mesh.vertices().iter().map(|v| v.position.map(f64::from)))
                .collect();
            assert_eq!(positions, displayed);
        }
        assert!(changed);
    }
}
