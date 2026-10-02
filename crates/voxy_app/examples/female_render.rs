#[allow(dead_code)]
#[path = "../src/body_parameters.rs"]
mod body_parameters;
#[path = "../src/diagnostic_legend.rs"]
mod diagnostic_legend;
#[allow(dead_code)]
#[path = "../src/face_parameters.rs"]
mod face_parameters;
#[allow(dead_code)]
#[path = "../src/female_complexion.rs"]
mod female_complexion;
#[allow(dead_code)]
#[path = "../src/female_eyes.rs"]
mod female_eyes;
#[allow(dead_code)]
#[path = "../src/female_face.rs"]
mod female_face;
#[allow(dead_code)]
#[path = "../src/female_features.rs"]
mod female_features;
#[allow(dead_code)]
#[path = "../src/film_settings.rs"]
mod film_settings;
#[path = "../src/rig_skinning.rs"]
mod rig_skinning;
// Renders the imported adult mannequin through the native renderer for visual inspection.
#[allow(dead_code)]
#[path = "../src/female_demo.rs"]
mod female_demo;
#[allow(dead_code)]
#[path = "../src/female_hair.rs"]
mod female_hair;
#[allow(dead_code)]
#[path = "../src/female_rig.rs"]
mod female_rig;
#[allow(dead_code)]
#[path = "../src/surface_film_preview.rs"]
mod surface_film_preview;
#[allow(dead_code)]
#[path = "../src/volume_regions.rs"]
mod volume_regions;
use glam::Vec3;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().collect();
    let option_path = |name: &str| -> Result<Option<&str>, Box<dyn std::error::Error>> {
        arguments
            .iter()
            .position(|a| a == name)
            .map(|i| {
                arguments
                    .get(i + 1)
                    .map(String::as_str)
                    .ok_or_else(|| format!("{name} requires a path").into())
            })
            .transpose()
    };
    let male = arguments.iter().any(|a| a == "--male");
    if option_path("--body-obj")?.is_some() && arguments.iter().any(|a| a == "--switch-body-model")
    {
        return Err("candidate source switching requires rebuilt film correspondence".into());
    }
    let mut model = match (option_path("--body-obj")?, option_path("--skin-bindings")?) {
        (Some(body), Some(skin)) if male => female_demo::FemaleDemo::male_from_assets(
            &std::fs::read_to_string(body)?,
            &std::fs::read_to_string(skin)?,
        )?,
        (Some(body), Some(skin)) => female_demo::FemaleDemo::female_from_assets(
            &std::fs::read_to_string(body)?,
            &std::fs::read_to_string(skin)?,
        )?,
        (None, None) if male => female_demo::FemaleDemo::new_male()?,
        (None, None) => female_demo::FemaleDemo::new()?,
        _ => {
            return Err(
                "candidate preview requires --body-obj and --skin-bindings together".into(),
            );
        }
    };
    model.show_complexion = !std::env::args().any(|a| a == "--bare");
    model.animation_only = true;
    model.surface_diffusion_enabled = !arguments.iter().any(|a| a == "--no-surface-diffusion");
    model.show_strain = arguments.iter().any(|a| a == "--strain");
    model.show_skin = arguments.iter().any(|a| a == "--displacement");
    if let Some(index) = arguments.iter().position(|a| a == "--body-preset") {
        let path = arguments
            .get(index + 1)
            .ok_or("--body-preset requires a JSON path")?;
        let parameters = model
            .body_parameters
            .patched(&serde_json::from_str(&std::fs::read_to_string(path)?)?)?;
        if option_path("--body-obj")?.is_some()
            && parameters.body_model != model.body_parameters.body_model
        {
            return Err(
                "candidate preset cannot change body source without rebuilt correspondence".into(),
            );
        }
        model.set_body_parameters(parameters)?;
    }
    // Explicit transition sampling: initial response, elapsed time, and both
    // time constants are required rather than assuming physiological values.
    if let Some(seconds) = option_path("--cold-transition-seconds")? {
        let number = |name: &str| -> Result<f64, Box<dyn std::error::Error>> {
            Ok(option_path(name)?
                .ok_or_else(|| format!("cold transition requires {name}"))?
                .parse()?)
        };
        let mut response = body_parameters::ColdResponse::new(
            number("--cold-initial-response")?,
            f64::from(model.body_parameters.nipple_cold_response),
            number("--cold-onset-seconds")?,
            number("--cold-recovery-seconds")?,
        )?;
        let elapsed: f64 = seconds.parse()?;
        response.advance(elapsed)?;
        model.set_body_parameters(response.apply(model.body_parameters))?;
        println!(
            "COLD RESPONSE: elapsed {elapsed} s, response {}; illustrative normalized stimulus, no temperature calibration",
            response.response()
        );
    }
    let snapshot = arguments
        .iter()
        .position(|a| a == "--snapshot")
        .map(|index| {
            arguments
                .get(index + 1)
                .ok_or("--snapshot requires a PNG path")
        })
        .transpose()?;
    if let Some(index) = arguments.iter().position(|a| a == "--face-preset") {
        let path = arguments
            .get(index + 1)
            .ok_or("--face-preset requires a JSON path")?;
        model.face_parameters =
            face_parameters::FaceParameters::from_json(&std::fs::read_to_string(path)?)?;
    }
    if let Some(index) = arguments.iter().position(|a| a == "--strain-fixture") {
        let stretch = arguments
            .get(index + 1)
            .ok_or("--strain-fixture requires x stretch")?
            .parse::<f64>()?;
        model.set_strain_fixture(stretch)?;
        model.show_strain = true;
    }
    model.set_grasp_cycle(std::env::args().any(|a| a == "--grasp"));
    if arguments.iter().any(|a| a == "--grasp-free") {
        model.adjust_grasp(1.)?;
    }
    if std::env::args().any(|a| a == "--grasp") {
        let target = arguments
            .windows(2)
            .find(|p| p[0] == "--grasp-object")
            .map(|p| p[1].as_str())
            .unwrap_or("cylinder");
        let offset = if let Some(value) = option_path("--grasp-offset")? {
            let coordinates = value
                .split(',')
                .map(str::parse::<f32>)
                .collect::<Result<Vec<_>, _>>()?;
            if coordinates.len() != 3 {
                return Err("--grasp-offset requires x,y,z in metres".into());
            }
            Vec3::new(coordinates[0], coordinates[1], coordinates[2])
        } else {
            Vec3::ZERO
        };
        model.set_grasp_target_with_offset(target, offset)?;
    }
    if let Some(index) = arguments.iter().position(|a| a == "--grasp-amount") {
        let amount = arguments
            .get(index + 1)
            .ok_or("--grasp-amount requires a number in 0..1")?
            .parse::<f32>()?;
        if !amount.is_finite() || !(0. ..=1.).contains(&amount) {
            return Err("--grasp-amount must be in 0..1".into());
        }
        if arguments.iter().any(|a| a == "--grasp-free") {
            return Err("use either --grasp-free or --grasp-amount".into());
        }
        model.adjust_grasp(amount)?;
    }

    if arguments.iter().any(|a| a == "--film") {
        model.enable_film([0., 0.20, 0.13], 0.04, 5e-8)?;
        if let Some(index) = arguments.iter().position(|a| a == "--film-preset") {
            let path = arguments
                .get(index + 1)
                .ok_or("--film-preset requires a JSON path")?;
            model
                .film
                .as_mut()
                .unwrap()
                .configure(film_settings::FilmSettings::read(std::path::Path::new(
                    path,
                ))?)?;
        }
    }
    if let Some(path) = option_path("--film-state")? {
        let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let state = if saved.get("state").is_some() {
            &saved["state"]
        } else {
            &saved
        };
        model
            .film
            .as_mut()
            .ok_or("--film-state requires --film")?
            .restore_snapshot(state)?;
    }
    let film_state_output = option_path("--film-state-output")?;
    if film_state_output.is_some() && model.film.is_none() {
        return Err("--film-state-output requires --film".into());
    }
    if let Some(index) = arguments.iter().position(|a| a == "--switch-body-model") {
        let target = arguments
            .get(index + 1)
            .ok_or("--switch-body-model requires female or male")?;
        let before = model.film.as_ref().map(|f| f.measurements());
        let source = model.body_parameters.body_model.as_str();
        let parameters = model
            .body_parameters
            .patched(&serde_json::json!({"body_model":target}))?;
        model.set_body_parameters(parameters)?;
        let proof = serde_json::json!({"source":source,"target":target,"before":before,"after":model.film.as_ref().map(|f|f.measurements()),"parameters":model.body_parameters.to_json()});
        if let Some(index) = arguments.iter().position(|a| a == "--switch-proof") {
            std::fs::write(
                arguments
                    .get(index + 1)
                    .ok_or("--switch-proof requires a JSON path")?,
                serde_json::to_string_pretty(&proof)?,
            )?;
        }
        println!("BODY SOURCE SWITCH: {proof}");
    }
    let hands = std::env::args().any(|a| a == "--hands");
    let foot_detail = arguments.iter().any(|a| a == "--foot-detail");
    let foot_view = option_path("--foot-view")?;
    if foot_view.is_some() && !foot_detail {
        return Err("--foot-view requires --foot-detail".into());
    }
    let foot_eye_offset = match foot_view.unwrap_or("top") {
        "top" => Vec3::new(0.025, 0.025, 0.07),
        "sole" => Vec3::new(0.025, -0.045, 0.07),
        "outer" => Vec3::new(0.08, 0.01, 0.025),
        "inner" => Vec3::new(-0.08, 0.01, 0.025),
        _ => return Err("--foot-view must be top, sole, outer or inner".into()),
    };
    let feet = foot_detail || arguments.iter().any(|a| a == "--feet");
    if hands && feet {
        return Err("choose either --hands or --feet".into());
    }
    if hands {
        model.set_hand_focus(true);
    }
    let face = std::env::args().any(|a| a == "--face");
    let torso = arguments.iter().any(|a| a == "--torso");
    let chest = arguments.iter().any(|a| a == "--chest");
    let side = std::env::args().any(|a| a == "--side");
    let framing_scale = model.body_parameters.height_cm / 164.;
    let framing_center = Vec3::new(0., 0.82 * (framing_scale - 1.), 0.);
    let whole_body_eye = framing_center + Vec3::new(0.4, 0.1, 2.4) * framing_scale;
    model.preview_camera_eye = Some(if chest {
        Vec3::new(0.10, 0.36, 0.65)
    } else if torso {
        Vec3::new(0.08, 0.20, 0.55)
    } else if face {
        Vec3::new(0., 0.715, 0.57)
    } else if side {
        Vec3::new(2.4, 0.1, 0.4)
    } else {
        whole_body_eye
    });
    let directory = option_path("--sequence-dir")?.unwrap_or(if hands {
        "/tmp/voxy-finger-frames"
    } else if face {
        "/tmp/voxy-face-frames"
    } else if side {
        "/tmp/voxy-rig-side-frames"
    } else {
        "/tmp/voxy-rig-frames"
    });
    let sequence = std::env::args().any(|a| a == "--sequence");
    if sequence {
        std::fs::create_dir_all(directory)?;
    }
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Model smoke on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let msaa4 = arguments.iter().any(|a| a == "--msaa4");
    let mut renderer = if msaa4 {
        SceneRenderer::new_msaa4(&device, wgpu::TextureFormat::Rgba8UnormSrgb)
    } else {
        SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb)
    };
    let hand_indices: Vec<_> = if hands || feet {
        let source = model.mesh()?;
        source
            .explicit_material_coordinates()
            .ok_or("hand camera requires the loaded model's rest coordinates")?
            .iter()
            .enumerate()
            .filter_map(|(index, p)| {
                (if feet {
                    p[1] < -0.68
                } else {
                    p[0] > 0.32 && p[1] < -0.015 && p[1] > -0.14
                })
                .then_some(index)
            })
            .collect()
    } else {
        Vec::new()
    };
    let shader = if arguments.iter().any(|a| a == "--normals") {
        female_eyes::MATERIAL_SHADER.replace(
            "normal=select(-normal,normal,dot(normal,view)>=0.0);",
            "normal=select(-normal,normal,dot(normal,view)>=0.0); if all(in.uv==vec2<f32>(0.0)) { return vec4<f32>(normal*0.5+vec3<f32>(0.5),1.0); }",
        )
    } else {
        female_eyes::MATERIAL_SHADER.to_owned()
    };
    pollster::block_on(renderer.reload_shader(&device, &shader))?;
    let mesh = model.mesh()?;
    let mut geometry = renderer.upload_mesh(&device, &mesh)?;
    if arguments.iter().any(|a| a == "--geometry-bench") {
        let mut shifted = mesh.vertices().to_vec();
        for vertex in &mut shifted {
            vertex.position[1] += 0.0001;
        }
        let mut shifted = voxy_render::SceneMesh::new(shifted, mesh.indices().to_vec())?;
        if let Some(coordinates) = mesh.explicit_material_coordinates() {
            shifted = shifted.with_material_coordinates(coordinates.to_vec())?;
        }
        for (label, changed) in [("unchanged", false), ("deformed", true)] {
            let mut samples = Vec::new();
            for i in 0..24 {
                let started = std::time::Instant::now();
                geometry.update(
                    &queue,
                    if changed && i % 2 == 0 {
                        &shifted
                    } else {
                        &mesh
                    },
                )?;
                samples.push(started.elapsed().as_secs_f64() * 1000.);
                queue.submit([]);
                device.poll(wgpu::PollType::wait_indefinitely())?;
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "GEOMETRY_BENCH,{label},{},{},{:.4},{:.4}",
                mesh.vertices().len(),
                mesh.indices().len() / 3,
                samples[12],
                samples[23]
            );
        }
    }
    let texture = renderer.upload_texture_with_sampling(
        &device,
        &queue,
        female_complexion::WIDTH,
        female_complexion::SIZE,
        female_complexion::atlas(),
        voxy_render::TextureSampling {
            min_filter: voxy_render::TextureFilter::Linear,
            mag_filter: voxy_render::TextureFilter::Linear,
            ..Default::default()
        },
    )?;
    let camera = SceneCamera {
        eye: if chest {
            Vec3::new(0.10, 0.36, 0.65)
        } else if torso {
            Vec3::new(0.08, 0.20, 0.55)
        } else if face {
            Vec3::new(0., 0.715, 0.57)
        } else if side {
            Vec3::new(2.4, 0.1, 0.4)
        } else {
            whole_body_eye
        },
        target: if chest {
            Vec3::new(0., 0.36, 0.13)
        } else if torso {
            Vec3::new(0., 0.20, 0.13)
        } else if face {
            Vec3::new(0., 0.70, 0.12)
        } else {
            framing_center
        },
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 45f32.to_radians(),
            aspect: 0.75,
            near: 0.01,
            far: 10.,
        },
    };
    let transform = renderer.create_transform(&device, camera.view_projection()?)?;
    transform.update_view_position(&queue, camera.eye)?;
    let color = target(
        &device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
    );
    let opaque_scene = if arguments.iter().any(|a| a == "--film-scene-optics") {
        if model.film.is_none() {
            return Err("--film-scene-optics requires --film".into());
        }
        Some(renderer.create_sampled_color(&device, 576, 768)?)
    } else {
        None
    };
    let no_film_refraction = arguments.iter().any(|a| a == "--film-no-refraction");
    if no_film_refraction && opaque_scene.is_none() {
        return Err("--film-no-refraction requires --film-scene-optics".into());
    }
    let mut scene_film_geometry: Option<voxy_render::SceneGeometry> = None;
    let depth = target(
        &device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let msaa = msaa4.then(|| {
        let attachment = |format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("film optical MSAA attachment"),
                size: wgpu::Extent3d {
                    width: 576,
                    height: 768,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        };
        (
            attachment(wgpu::TextureFormat::Rgba8UnormSrgb),
            attachment(wgpu::TextureFormat::Depth32Float),
        )
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("model readback"),
        size: 768 * 2304,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let legend_geometry = renderer.upload_mesh(
        &device,
        &diagnostic_legend::mesh(&model.diagnostic_legend_labels())?,
    )?;
    let legend_transform = renderer.create_transform(
        &device,
        glam::Mat4::from_scale_rotation_translation(
            Vec3::new(0.55, 0.14, 1.0),
            glam::Quat::IDENTITY,
            Vec3::new(-0.55, 0.86, 0.0),
        ),
    )?;
    let mut frames = Vec::new();
    let mut mesh_ms = Vec::new();
    let sequence_center = if sequence && hands {
        model.preview_pose(3.);
        let reference = model.mesh()?;
        Some(
            hand_indices
                .iter()
                .map(|&index| Vec3::from_array(reference.vertices()[index].position))
                .sum::<Vec3>()
                / hand_indices.len() as f32,
        )
    } else {
        None
    };
    for frame in 0..if snapshot.is_some() {
        1
    } else if sequence {
        120
    } else if face {
        3
    } else {
        2
    } {
        model.preview_pose(if sequence {
            f64::from(frame) * 0.05
        } else if face {
            [0., 0.9, 3.][frame as usize]
        } else {
            f64::from(frame) * 3.
        });
        let started = std::time::Instant::now();
        let mesh = model.mesh()?;
        let mesh = if let Some(path) = option_path("--hand-pose-csv")? {
            if snapshot.is_none() || !hands {
                return Err("--hand-pose-csv requires --hands and --snapshot".into());
            }
            let key = |p: [f32; 3]| p.map(|v| (v * 1e6).round() as i32);
            let mut poses = std::collections::BTreeMap::new();
            for line in std::fs::read_to_string(path)?.lines() {
                let values = line
                    .split(',')
                    .map(str::parse::<f32>)
                    .collect::<Result<Vec<_>, _>>()?;
                if values.len() != 6 || values.iter().any(|v| !v.is_finite()) {
                    return Err("invalid hand pose row".into());
                }
                poses.insert(
                    key([values[0], values[1], values[2]]),
                    [values[3], values[4], values[5]],
                );
            }
            let coordinates = mesh
                .explicit_material_coordinates()
                .ok_or("hand pose needs rest coordinates")?;
            let mut vertices = mesh.vertices().to_vec();
            let mut matched = 0;
            for (vertex, rest) in vertices.iter_mut().zip(coordinates) {
                if let Some(position) = poses.get(&key(*rest)) {
                    vertex.position = *position;
                    matched += 1;
                }
            }
            if matched == 0 {
                return Err("hand pose did not match the loaded model".into());
            }
            println!("HAND POSE: applied {matched} vertices from {path}");
            voxy_render::SceneMesh::new(vertices, mesh.indices().to_vec())?
                .with_material_coordinates(coordinates.to_vec())?
        } else {
            mesh
        };
        let mesh = if hands && arguments.iter().any(|a| a == "--hand-only") {
            let coordinates = mesh
                .explicit_material_coordinates()
                .ok_or("hand view needs rest coordinates")?;
            let indices: Vec<u32> = mesh
                .indices()
                .chunks_exact(3)
                .filter(|face| {
                    face.iter().all(|&index| {
                        let rest = coordinates[index as usize];
                        let p = if rest == [0.; 3] {
                            mesh.vertices()[index as usize].position
                        } else {
                            rest
                        };
                        p[0] > 0.30 && (-0.18..0.12).contains(&p[1])
                    })
                })
                .flatten()
                .copied()
                .collect();
            if indices.is_empty() {
                return Err("hand view has no triangles".into());
            }
            voxy_render::SceneMesh::new(mesh.vertices().to_vec(), indices)?
                .with_material_coordinates(coordinates.to_vec())?
        } else {
            mesh
        };
        if hands || feet {
            if hand_indices.is_empty() {
                return Err("no vertices in close-up region".into());
            }
            let center = sequence_center.unwrap_or_else(|| {
                hand_indices
                    .iter()
                    .map(|&index| Vec3::from_array(mesh.vertices()[index].position))
                    .sum::<Vec3>()
                    / hand_indices.len() as f32
            });
            let center = if foot_detail {
                Vec3::new(0.175, -0.800, 0.100)
            } else {
                center
            };
            let hand_camera = SceneCamera {
                eye: center
                    + if foot_detail {
                        foot_eye_offset
                    } else if feet {
                        Vec3::new(0.12, 0.22, 0.85)
                    } else {
                        Vec3::new(
                            if arguments.iter().any(|a| a == "--palm") {
                                -0.24
                            } else {
                                0.24
                            },
                            0.025,
                            0.20,
                        )
                    },
                target: center,
                up: Vec3::Y,
                projection: SceneProjection::Perspective {
                    vertical_fov: (if foot_detail {
                        35_f32
                    } else if feet {
                        60_f32
                    } else {
                        40_f32
                    })
                    .to_radians(),
                    aspect: 0.75,
                    near: 0.005,
                    far: 10.,
                },
            };
            if feet {
                println!(
                    "FEET CAMERA: {} vertices, center {center:?}",
                    hand_indices.len()
                );
            }
            transform.update(&queue, hand_camera.view_projection()?)?;
            transform.update_view_position(&queue, hand_camera.eye)?;
        }
        let mesh = if opaque_scene.is_some() {
            if let Some((opaque, wet)) =
                mesh.split_material_layer(-2., if no_film_refraction { -7. } else { -6. })?
            {
                if let Some(geometry) = &mut scene_film_geometry {
                    geometry.update(&queue, &wet)?;
                } else {
                    scene_film_geometry = Some(renderer.upload_mesh(&device, &wet)?);
                }
                opaque
            } else {
                scene_film_geometry = None;
                mesh
            }
        } else {
            mesh
        };
        geometry.update(&queue, &mesh)?;
        mesh_ms.push(started.elapsed().as_secs_f64() * 1000.);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let colour_view = opaque_scene
            .as_ref()
            .map_or(&color, |image| image.texture())
            .create_view(&Default::default());
        let depth_view = depth.create_view(&Default::default());
        let clear = wgpu::Color {
            r: 0.08,
            g: 0.10,
            b: 0.14,
            a: 1.,
        };
        let mut draws = vec![SceneDraw {
            geometry: &geometry,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        if model.diagnostic_legend_visible() {
            draws.push(SceneDraw {
                geometry: &legend_geometry,
                texture: &texture,
                transform: &legend_transform,
                overlay: true,
            });
        }
        if let Some(scene) = &opaque_scene {
            let wet: Vec<_> = scene_film_geometry
                .iter()
                .map(|geometry| SceneDraw {
                    geometry,
                    texture: scene,
                    transform: &transform,
                    overlay: false,
                })
                .collect();
            let output = color.create_view(&Default::default());
            let multisampled = msaa.as_ref().map(|(color, depth)| {
                (
                    color.create_view(&Default::default()),
                    depth.create_view(&Default::default()),
                )
            });
            renderer.encode_refractive_layers(
                &mut encoder,
                &output,
                &depth_view,
                multisampled.as_ref().map(|(color, depth)| (color, depth)),
                scene,
                clear,
                &draws,
                &wet,
            )?;
        } else if let Some((multisampled_colour, multisampled_depth)) = &msaa {
            renderer.encode_msaa4(
                &mut encoder,
                &multisampled_colour.create_view(&Default::default()),
                &multisampled_depth.create_view(&Default::default()),
                &colour_view,
                clear,
                &draws,
            )?;
        } else {
            renderer.encode(&mut encoder, &colour_view, &depth_view, clear, &draws);
        }
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
                    bytes_per_row: Some(2304),
                    rows_per_image: Some(768),
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
        assert!(
            pixels.chunks_exact(4).filter(|p| p[0] > 70).count() > 1000,
            "body missing from render"
        );
        if sequence {
            image::save_buffer(
                format!("{directory}/{frame:04}.png"),
                &pixels,
                576,
                768,
                image::ColorType::Rgba8,
            )?;
        } else {
            frames.push(pixels.to_vec());
        }
        drop(pixels);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    if let Some(path) = film_state_output {
        let state = model.film.as_ref().unwrap().state_snapshot();
        std::fs::write(path, serde_json::to_vec_pretty(&state)?)?;
        println!("PASS: numerical film checkpoint saved to {path}; body pose/solver not captured");
    }
    mesh_ms.sort_by(f64::total_cmp);
    println!(
        "Mesh update CPU: median {:.2} ms, max {:.2} ms",
        mesh_ms[mesh_ms.len() / 2],
        mesh_ms.last().unwrap()
    );
    if sequence {
        println!("PASS: rendered 120 samples covering the full six-second loop");
        return Ok(());
    }
    if let Some(path) = snapshot {
        image::save_buffer(path, &frames[0], 576, 768, image::ColorType::Rgba8)?;
        println!("PASS: snapshot saved to {path}");
        return Ok(());
    }
    let mut pixels = Vec::new();
    for y in 0..768 {
        for frame in &frames {
            pixels.extend_from_slice(&frame[y * 2304..(y + 1) * 2304]);
        }
    }
    image::save_buffer(
        if hands {
            "/tmp/voxy-finger-preview.png"
        } else if face {
            "/tmp/voxy-face-preview.png"
        } else if side {
            "/tmp/voxy-female-side-preview.png"
        } else {
            "/tmp/voxy-female-preview.png"
        },
        &pixels,
        if face { 1728 } else { 1152 },
        768,
        image::ColorType::Rgba8,
    )?;
    println!(
        "PASS: imported female model rendered in {} poses",
        frames.len()
    );
    Ok(())
}
fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model target"),
        size: wgpu::Extent3d {
            width: 576,
            height: 768,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}
