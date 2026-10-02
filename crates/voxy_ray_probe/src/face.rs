//! Hardware-ray shadows and short-range ambient visibility on the character mesh.
use glam::{Vec2, Vec3};
use voxy_render::{
    RayScene, RaySegment, RayVisibilityPipeline, SceneCamera, SceneDraw, SceneMesh,
    SceneProjection, SceneRenderer, TextureFilter, TextureSampling,
};

pub fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut preview = voxy_app::FacePreview::new()?;
    let arguments: Vec<_> = std::env::args().collect();
    let mut base_parameters = voxy_app::face_parameters::FaceParameters::default();
    if let Some(index) = arguments.iter().position(|a| a == "--face-preset") {
        if arguments.iter().any(|a| a == "--lid-surface") {
            return Err("face presets require the integrated face preview".into());
        }
        let path = arguments
            .get(index + 1)
            .ok_or("--face-preset requires a JSON path")?;
        let parameters =
            voxy_app::face_parameters::FaceParameters::from_json(&std::fs::read_to_string(path)?)?;
        base_parameters = parameters;
        preview.set_parameters(base_parameters.clone());
    }
    let mut renderer = SceneRenderer::new_msaa4(device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mut shader = voxy_app::FacePreview::material_shader().to_owned();
    if std::env::var("VOXY_FACE_DIAGNOSTIC_BACKLIGHT").as_deref() == Ok("1") {
        shader = shader.replace("vec3<f32>(-0.5,0.7,1.0)", "vec3<f32>(0.5,0.3,-1.0)");
    }
    if arguments.iter().any(|a| a == "--skin-macro-light") {
        // Diagnostic: light the posed smooth geometry directly, excluding the
        // atlas relief and baked CPU irradiance. Keep the same key and visibility.
        let original = "return vec4<f32>(texel.rgb*in.color.rgb*diffuse_ratio+vec3<f32>(1.0,0.96,0.92)*highlight*visibility,1.0);";
        if !shader.contains(original) {
            return Err("skin macro-light diagnostic no longer matches material shader".into());
        }
        shader = shader.replace(original,
            "let macro_light=0.2+0.6*max(dot(base_normal,light),0.0)*visibility+0.2*max(dot(base_normal,fill),0.0); return vec4<f32>(vec3<f32>(0.62,0.48,0.40)*macro_light+vec3<f32>(skin_area_highlight(base_normal,view,light,0.6,0.0)*visibility),1.0);");
    }
    pollster::block_on(renderer.reload_shader(device, &shader))?;
    let (width, height, pixels) = preview.material_texture();
    let clay = std::env::args().any(|a| a == "--lid-clay");
    let neutral_texture = clay.then(|| {
        let mut pixels = vec![255; (width * height * 4) as usize];
        for y in 0..height {
            for x in width / 2..width {
                let offset = ((y * width + x) * 4) as usize;
                // sRGB-encoded constant roughness, zero oil, constant height.
                pixels[offset..offset + 4].copy_from_slice(&[243, 0, 188, 255]);
            }
        }
        pixels
    });
    let pixels = neutral_texture.as_deref().unwrap_or(&pixels);
    let mut texture = renderer.upload_texture_with_sampling(
        device,
        queue,
        width,
        height,
        pixels,
        TextureSampling {
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            ..Default::default()
        },
    )?;
    let oral_closeup = std::env::args().any(|a| a == "--oral-closeup");
    let oral_transition = arguments.iter().any(|a| a == "--oral-transition");
    let tongue_transition = arguments.iter().any(|a| a == "--tongue-transition");
    let oral_above = arguments.iter().any(|a| a == "--oral-above");
    let pupil_transition = arguments.iter().any(|a| a == "--pupil-transition");
    let brow_transition = arguments.iter().any(|a| a == "--brow-transition");
    let lid_close_transition = arguments.iter().any(|a| a == "--lid-close-transition");
    let age_strong_transition = arguments.iter().any(|a| a == "--age-strong-transition");
    let age_transition = age_strong_transition || arguments.iter().any(|a| a == "--age-transition");
    let skin_finish_transition = arguments.iter().any(|a| a == "--skin-finish-transition");
    let skin_depth_transition = arguments.iter().any(|a| a == "--skin-depth-transition");
    let lid_surface = std::env::args().any(|a| a == "--lid-surface");
    let eye_closeup = lid_surface || std::env::args().any(|a| a == "--eye-closeup");
    let eye_side = std::env::args().any(|a| a == "--eye-side");
    let skin_closeup = arguments.iter().any(|a| a == "--skin-closeup");
    let camera = SceneCamera {
        eye: if skin_closeup {
            Vec3::new(
                if arguments.iter().any(|a| a == "--skin-side") {
                    0.065
                } else {
                    0.
                },
                0.753,
                0.26,
            )
        } else if eye_closeup {
            Vec3::new(if eye_side { 0.065 } else { 0.033 }, 0.714, 0.20)
        } else if oral_closeup {
            Vec3::new(
                0.,
                if oral_above { 0.678 } else { 0.646 },
                if std::env::var("VOXY_FACE_DIAGNOSTIC_DENTAL_MACRO").as_deref() == Ok("1") {
                    0.205
                } else {
                    0.27
                },
            )
        } else {
            Vec3::new(0., 0.715, if age_transition { 0.37 } else { 0.57 })
        },
        target: if skin_closeup {
            Vec3::new(0., 0.753, 0.13)
        } else if eye_closeup {
            Vec3::new(0.033, 0.712, 0.13)
        } else if oral_closeup {
            Vec3::new(0., 0.642, 0.135)
        } else {
            Vec3::new(0., 0.70, 0.12)
        },
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 45f32.to_radians(),
            aspect: 0.75,
            near: 0.01,
            far: 10.,
        },
    };
    let transform = renderer.create_transform(device, camera.view_projection()?)?;
    transform.update_view_position(queue, camera.eye)?;
    let color = target(
        device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        1,
    );
    let depth = target(
        device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        4,
    );
    let msaa_color = target(
        device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        4,
    );
    let visibility = RayVisibilityPipeline::new(device)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("face PNG"),
        size: 768 * 2304,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mouth = std::env::args().any(|a| a == "--mouth");
    let mut frames = Vec::new();
    let times = if pupil_transition
        || oral_transition
        || tongue_transition
        || brow_transition
        || skin_finish_transition
        || skin_depth_transition
        || lid_close_transition
        || age_transition
    {
        [0.; 3]
    } else if std::env::args().any(|a| a == "--blink-transition") {
        [0.86, 0.9, 0.94]
    } else {
        [0., 0.9, 3.]
    };
    for (frame, time) in times.into_iter().enumerate() {
        if age_transition {
            let amount = [0., 0.5, 1.][frame];
            let young: serde_json::Value = serde_json::from_str(include_str!(
                "../../../assets/characters/face-presets/youthful-soft.json"
            ))?;
            let mature: serde_json::Value = serde_json::from_str(if age_strong_transition {
                include_str!("../../../assets/characters/face-presets/aged-defined.json")
            } else {
                include_str!("../../../assets/characters/face-presets/mature-soft.json")
            })?;
            let mut patch = serde_json::Map::new();
            for (key, value) in mature.as_object().ok_or("invalid age preset")? {
                let a = young[key]
                    .as_f64()
                    .or_else(|| base_parameters.value(key).map(f64::from))
                    .ok_or("invalid youthful value")?;
                let b = value.as_f64().ok_or("invalid age value")?;
                patch.insert(key.clone(), serde_json::json!(a + (b - a) * amount));
            }
            preview.set_parameters(base_parameters.patched(&patch.into())?);
            let (atlas_width, atlas_height, atlas_pixels) = preview.material_texture();
            texture = renderer.upload_texture_with_sampling(
                device,
                queue,
                atlas_width,
                atlas_height,
                &atlas_pixels,
                TextureSampling {
                    min_filter: TextureFilter::Linear,
                    mag_filter: TextureFilter::Linear,
                    ..Default::default()
                },
            )?;
        }
        if pupil_transition {
            preview.set_parameters(
                base_parameters
                    .patched(&serde_json::json!({"pupil_diameter_mm":([2.,4.05,8.][frame])}))?,
            );
        }
        if skin_depth_transition {
            let surface_fraction = [0., 0.35, 1.][frame];
            preview.set_parameters(base_parameters.patched(&serde_json::json!({
                "skin_surface_light": surface_fraction
            }))?);
        }
        if skin_finish_transition {
            let mut finish_pixels = pixels.to_vec();
            if frame != 1 {
                let (roughness, oil): (f32, f32) = if frame == 0 { (0.8, 0.) } else { (0.28, 0.8) };
                let encode = |value: f32| -> u8 {
                    let srgb = if value <= 0.0031308 {
                        12.92 * value
                    } else {
                        1.055 * value.powf(1. / 2.4) - 0.055
                    };
                    (srgb * 255.).round() as u8
                };
                for y in 0..height {
                    for x in width / 2..width {
                        let offset = ((y * width + x) * 4) as usize;
                        finish_pixels[offset] = encode(roughness);
                        finish_pixels[offset + 1] = encode(oil);
                    }
                }
            }
            texture = renderer.upload_texture_with_sampling(
                device,
                queue,
                width,
                height,
                &finish_pixels,
                TextureSampling {
                    min_filter: TextureFilter::Linear,
                    mag_filter: TextureFilter::Linear,
                    ..Default::default()
                },
            )?;
        }
        let tongue = std::env::args().any(|a| a == "--tongue");
        let mesh = if lid_surface {
            if std::env::args().any(|a| a == "--lid-head") {
                voxy_app::FacePreview::lid_surface_with_head([0., 0.5, 1.][frame])?
            } else if std::env::args().any(|a| a == "--lid-globes") {
                voxy_app::FacePreview::lid_surface_with_eyes([0., 0.5, 1.][frame])?
            } else {
                voxy_app::FacePreview::lid_surface([0., 0.5, 1.][frame])?
            }
        } else if age_transition {
            preview.sample_blink(time, camera.eye, 0.)?
        } else if lid_close_transition {
            preview.sample_blink(time, camera.eye, [0., 0.5, 1.][frame])?
        } else if skin_depth_transition {
            preview.sample_brow(time, camera.eye, 1.)?
        } else if skin_finish_transition && skin_closeup {
            preview.sample_brow(time, camera.eye, 1.)?
        } else if brow_transition {
            preview.sample_brow(time, camera.eye, [-1., 0., 1.][frame])?
        } else {
            preview.sample_oral(
                time,
                camera.eye,
                if skin_finish_transition {
                    Some(0.)
                } else if tongue_transition {
                    Some(0.9)
                } else if oral_transition {
                    Some([0., 0.45, 0.9][frame])
                } else if mouth || tongue {
                    Some(0.8)
                } else {
                    None
                },
                if tongue_transition {
                    [-0.4, 0., 0.7][frame]
                } else if tongue {
                    0.7
                } else {
                    0.
                },
                if tongue_transition {
                    [0., 0.35, 0.8][frame]
                } else if tongue {
                    0.65
                } else {
                    0.
                },
            )?
        };
        let mut traced = ray_shade(device, queue, &visibility, &mesh)?;
        if frame == 2 && std::env::var("VOXY_FACE_DIAGNOSTIC_MOUTH_SECTION").as_deref() == Ok("1") {
            let triangles: Vec<_> = mesh
                .indices()
                .chunks_exact(3)
                .filter_map(|ids| {
                    let triangle = std::array::from_fn::<_, 3, _>(|i| {
                        let vertex = mesh.vertices()[ids[i] as usize];
                        (vertex.position, vertex.uv)
                    });
                    let crosses = triangle.iter().any(|v| v.0[0] <= 0.)
                        && triangle.iter().any(|v| v.0[0] >= 0.);
                    let nearby = triangle.iter().all(|v| {
                        (0.625..0.665).contains(&v.0[1]) && (0.075..0.17).contains(&v.0[2])
                    });
                    (crosses && nearby).then_some(triangle)
                })
                .collect();
            std::fs::write(
                "/tmp/voxy-mouth-surface-section.json",
                serde_json::to_vec(&triangles)?,
            )?;
            eprintln!("MOUTH SECTION: {} triangles", triangles.len());
        }
        if std::env::var("VOXY_FACE_DIAGNOSTIC_ORAL_MATERIAL_IDS").as_deref() == Ok("1") {
            if frame == 2 {
                let inverse = camera.view_projection()?.inverse();
                let pixels =
                    if std::env::var("VOXY_FACE_DIAGNOSTIC_DENTAL_MACRO").as_deref() == Ok("1") {
                        [[288., 480.], [240., 478.], [330., 478.]]
                    } else {
                        [[230., 377.], [250., 363.], [346., 377.]]
                    };
                for pixel in pixels {
                    let far = inverse.project_point3(Vec3::new(
                        pixel[0] / 576. * 2. - 1.,
                        1. - pixel[1] / 768. * 2.,
                        1.,
                    ));
                    let direction = (far - camera.eye).normalize();
                    let mut nearest = f32::INFINITY;
                    let mut hit = None;
                    for ids in mesh.indices().chunks_exact(3) {
                        let a = Vec3::from_array(mesh.vertices()[ids[0] as usize].position);
                        let b = Vec3::from_array(mesh.vertices()[ids[1] as usize].position);
                        let c = Vec3::from_array(mesh.vertices()[ids[2] as usize].position);
                        let e1 = b - a;
                        let e2 = c - a;
                        let h = direction.cross(e2);
                        let det = e1.dot(h);
                        if det.abs() < 1e-12 {
                            continue;
                        }
                        let s = camera.eye - a;
                        let u = s.dot(h) / det;
                        let q = s.cross(e1);
                        let v = direction.dot(q) / det;
                        let distance = e2.dot(q) / det;
                        if u >= 0. && v >= 0. && u + v <= 1. && distance > 0. && distance < nearest
                        {
                            nearest = distance;
                            hit = Some((ids.to_vec(), camera.eye + direction * distance));
                        }
                    }
                    eprintln!("ORAL MATERIAL HIT pixel={pixel:?} hit={hit:?}");
                    if let Some((ids, _)) = hit {
                        for id in ids {
                            eprintln!(
                                "ORAL MATERIAL VERTEX id={id} vertex={:?}",
                                mesh.vertices()[id as usize]
                            );
                        }
                    }
                }
            }
            let mut vertices = traced.vertices().to_vec();
            for vertex in &mut vertices {
                let tissue = vertex.uv[0] < -0.5;
                vertex.color = if tissue {
                    [0.9, 0.04, 0.65, 1.]
                } else {
                    [0.04, 0.9, 0.15, 1.]
                };
                vertex.uv = [-1., 0.9];
            }
            traced = SceneMesh::new(vertices, traced.indices().to_vec())?;
        }
        if std::env::args().any(|a| a == "--lid-flat-color") {
            let mut vertices = traced.vertices().to_vec();
            for vertex in &mut vertices {
                let uv = Vec2::from_array(vertex.uv);
                let eye = (0.1..=0.3).contains(&uv.x) && (0.004..=0.044).contains(&uv.y);
                if uv.x >= 0. && !eye {
                    vertex.color[..3].copy_from_slice(&[0.72, 0.46, 0.34]);
                }
            }
            traced = SceneMesh::new(vertices, traced.indices().to_vec())?;
        }
        let geometry = renderer.upload_mesh(device, &traced)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode_msaa4(
            &mut encoder,
            &msaa_color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color {
                r: 0.02,
                g: 0.024,
                b: 0.033,
                a: 1.,
            },
            &[SceneDraw {
                geometry: &geometry,
                transform: &transform,
                texture: &texture,
                overlay: false,
            }],
        )?;
        encoder.copy_texture_to_buffer(
            color.as_image_copy(),
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
        let bytes = submit_read(device, queue, encoder, &readback)?;
        frames.push(bytes);
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let path = save_strip(&frames, mouth)?;
    println!("CHARACTER HARDWARE RAY PASS: 3 poses; {path}");
    Ok(())
}
const KEY_SAMPLES: usize = 16;
const FILL_SAMPLES: usize = 4;
const AO_SAMPLES: usize = 4;
const RAYS_PER_VERTEX: usize = KEY_SAMPLES + FILL_SAMPLES + AO_SAMPLES;

/// Deterministic disk quadrature for a finite angular-size studio light.
fn area_light_directions(axis: Vec3, radius: f32, count: usize) -> Result<Vec<Vec3>, &'static str> {
    let count =
        u16::try_from(count).map_err(|_| "studio light sample count exceeds exact range")?;
    if count == 0 {
        return Err("studio light sample count must be positive");
    }
    let tangent = axis.cross(Vec3::Y).normalize();
    let bitangent = axis.cross(tangent);
    Ok((0..count)
        .map(|sample| {
            let r = radius * ((f32::from(sample) + 0.5) / f32::from(count)).sqrt();
            let angle = f32::from(sample) * 2.399_963_1;
            (axis + tangent * (r * angle.cos()) + bitangent * (r * angle.sin())).normalize()
        })
        .collect())
}

// Keep the existing f32 quantization without saturating distinct positions into i32 limits.
fn position_key(position: [f32; 3]) -> Result<[u32; 3], &'static str> {
    let rounded = position.map(|p| (p * 10_000_000.).round());
    if rounded.iter().any(|value| !value.is_finite()) {
        return Err("skin normal position exceeds quantization range");
    }
    Ok(rounded.map(|value| if value == 0.0 { 0 } else { value.to_bits() }))
}

fn ray_shade(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &RayVisibilityPipeline,
    mesh: &SceneMesh,
) -> Result<SceneMesh, Box<dyn std::error::Error>> {
    let key = if std::env::var("VOXY_FACE_DIAGNOSTIC_BACKLIGHT").as_deref() == Ok("1") {
        Vec3::new(0.5, 0.3, -1.).normalize()
    } else {
        Vec3::new(-0.5, 0.7, 1.).normalize()
    };
    let fill = Vec3::new(0.8, 0.2, 0.5).normalize();
    let mut triangles = Vec::new();
    let mut normals = vec![Vec3::ZERO; mesh.vertices().len()];
    let smooth_lids = std::env::args().any(|a| a == "--lid-surface");
    let mut skin_normals = std::collections::HashMap::<[u32; 3], Vec3>::new();
    let skin = |v: &voxy_render::SceneVertex| {
        v.uv[0] >= 0. && !((0.1..=0.3).contains(&v.uv[0]) && (0.004..=0.044).contains(&v.uv[1]))
    };
    for face in mesh.indices().chunks_exact(3) {
        if face
            .iter()
            .all(|&i| mesh.vertices()[i as usize].color[3] <= 0.)
        {
            continue;
        }
        let p: [Vec3; 3] =
            std::array::from_fn(|i| Vec3::from_array(mesh.vertices()[face[i] as usize].position));
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        if n.length_squared() < 1e-20 {
            continue;
        }
        triangles.extend(p.map(|point| point.to_array()));
        for &i in face {
            normals[i as usize] += n;
        }
        if smooth_lids {
            let p = p.map(|p| p.as_dvec3());
            if let Some(unit) = (p[1] - p[0]).cross(p[2] - p[0]).try_normalize() {
                for (corner, &i) in face.iter().enumerate() {
                    let vertex = &mesh.vertices()[i as usize];
                    if !skin(vertex) {
                        continue;
                    }
                    let a = p[(corner + 1) % 3] - p[corner];
                    let b = p[(corner + 2) % 3] - p[corner];
                    let denominator = a.length() * b.length();
                    if denominator > 0. {
                        let angle = (a.dot(b) / denominator).clamp(-1., 1.).acos();
                        *skin_normals
                            .entry(position_key(vertex.position)?)
                            .or_default() += (unit * angle).as_vec3();
                    }
                }
            }
        }
    }
    if smooth_lids {
        for (vertex, normal) in mesh.vertices().iter().zip(&mut normals) {
            if skin(vertex) {
                if let Some(&smooth) = skin_normals.get(&position_key(vertex.position)?) {
                    *normal = smooth;
                }
            }
        }
    }
    let scene = RayScene::new(device, &triangles)?;
    let key_directions = area_light_directions(key, 0.22, KEY_SAMPLES)?;
    let fill_directions = area_light_directions(fill, 0.30, FILL_SAMPLES)?;
    let mut segments = Vec::with_capacity(mesh.vertices().len() * RAYS_PER_VERTEX);
    for (vertex, n) in mesh.vertices().iter().zip(&normals) {
        let normal = n.try_normalize().unwrap_or(Vec3::Z);
        let p = Vec3::from_array(vertex.position) + normal * 0.00015;
        for direction in key_directions.iter().chain(&fill_directions) {
            segments.push(RaySegment::new(
                p.to_array(),
                (p + *direction * 3.).to_array(),
                0.0001,
            )?);
        }
        let tangent = normal
            .cross(if normal.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            })
            .normalize();
        let bitangent = normal.cross(tangent);
        for direction in [tangent, -tangent, bitangent, -bitangent] {
            segments.push(RaySegment::new(
                p.to_array(),
                (p + (normal * 0.6 + direction * 0.8) * 0.025).to_array(),
                0.0001,
            )?);
        }
    }

    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("character ray visibility"),
        size: u64::try_from(segments.len())? * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    scene.build(&mut encoder);
    // Bound each dispatch/upload independently of the character's growing mesh.
    // All batches query the same acceleration structure built once in this encoder.
    let mut jobs = Vec::new();
    let mut offset = 0u64;
    for batch in segments.chunks(400_000) {
        let job = pipeline.create_job(&scene, batch)?;
        job.encode(&mut encoder);
        let size = u64::try_from(batch.len())? * 4;
        encoder.copy_buffer_to_buffer(job.output(), 0, &output, offset, size);
        offset += size;
        jobs.push(job);
    }
    let bytes = submit_read(device, queue, encoder, &output)?;
    let (visible, blocked) = decode_visibility(&bytes)?;
    let mut vertices = mesh.vertices().to_vec();
    let unshadowed_teeth =
        std::env::var("VOXY_FACE_DIAGNOSTIC_UNSHADOWED_TEETH").as_deref() == Ok("1");
    for (index, vertex) in vertices.iter_mut().enumerate() {
        if vertex.color[3] <= 0. {
            continue;
        }
        if unshadowed_teeth && vertex.uv[0] == -1. && vertex.color[0] > 0.6 {
            vertex.color[3] = 1.;
            continue;
        }
        let n = normals[index].try_normalize().unwrap_or(Vec3::Z);
        let result = &visible[index * RAYS_PER_VERTEX..(index + 1) * RAYS_PER_VERTEX];
        let key_visibility =
            result[..KEY_SAMPLES].iter().sum::<f32>() / f32::from(u16::try_from(KEY_SAMPLES)?);
        let fill_visibility = result[KEY_SAMPLES..KEY_SAMPLES + FILL_SAMPLES]
            .iter()
            .sum::<f32>()
            / f32::from(u16::try_from(FILL_SAMPLES)?);
        let key_term = 0.6 * n.dot(key).max(0.);
        let fill_term = 0.2 * n.dot(fill).max(0.);
        let ambient = 0.25
            + 0.75 * result[KEY_SAMPLES + FILL_SAMPLES..].iter().sum::<f32>()
                / f32::from(u16::try_from(AO_SAMPLES)?);
        let ratio = (0.2 * ambient + key_term * key_visibility + fill_term * fill_visibility)
            / (0.2 + key_term + fill_term);
        let uv = Vec2::from_array(vertex.uv);
        if (0.1..=0.3).contains(&uv.x) && (0.004..=0.044).contains(&uv.y) {
            // RGB is a packed smooth eye normal, not irradiance or baked pigment.
            vertex.color[3] = 0.5 + 0.5 * key_visibility;
        } else {
            for value in &mut vertex.color[..3] {
                *value *= ratio;
            }
            if uv.x < -0.1 || ((0.025..=0.475).contains(&uv.x) && (0.05..=0.95).contains(&uv.y)) {
                vertex.color[3] = 0.5 + 0.5 * key_visibility;
            }
            if uv.x == -1. {
                // Oral U is otherwise constant. Keep fill visibility in a
                // linearly interpolated channel; V remains roughness.
                vertex.uv[0] = -0.75 - 0.25 * fill_visibility;
            }
        }
    }
    println!(
        "CHARACTER RAYS: {} triangles, {} segments, {blocked} occluded",
        scene.triangle_count(),
        segments.len()
    );
    let mut result = SceneMesh::new(vertices, mesh.indices().to_vec())?;
    if let Some(coordinates) = mesh.explicit_material_coordinates() {
        result = result.with_material_coordinates(coordinates.to_vec())?;
    }
    Ok(result)
}
fn submit_read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let submission = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(std::time::Duration::from_secs(30)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let bytes = mapped.to_vec();
    drop(mapped);
    buffer.unmap();
    Ok(bytes)
}
fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    samples: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("character target"),
        size: wgpu::Extent3d {
            width: 576,
            height: 768,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn save_strip(frames: &[Vec<u8>], mouth: bool) -> Result<&'static str, Box<dyn std::error::Error>> {
    let mut strip = Vec::new();
    for row in 0..768 {
        for frame in frames {
            strip.extend_from_slice(&frame[row * 2304..(row + 1) * 2304]);
        }
    }
    let path = if std::env::args().any(|a| a == "--lid-surface") {
        if std::env::args().any(|a| a == "--lid-head") {
            if std::env::args().any(|a| a == "--lid-flat-color") {
                "/tmp/voxy-lid-head-flat-color.png"
            } else if std::env::args().any(|a| a == "--lid-clay") {
                "/tmp/voxy-lid-head-clay.png"
            } else {
                "/tmp/voxy-lid-head.png"
            }
        } else if std::env::args().any(|a| a == "--lid-globes") {
            if std::env::args().any(|a| a == "--eye-side") {
                "/tmp/voxy-lid-globes-side.png"
            } else {
                "/tmp/voxy-lid-globes.png"
            }
        } else {
            "/tmp/voxy-lid-surface.png"
        }
    } else if std::env::args().any(|a| a == "--age-strong-transition") {
        "/tmp/voxy-age-strong-transition.png"
    } else if std::env::args().any(|a| a == "--age-transition") {
        "/tmp/voxy-age-transition.png"
    } else if std::env::args().any(|a| a == "--lid-close-transition") {
        "/tmp/voxy-lid-close-transition.png"
    } else if std::env::args().any(|a| a == "--skin-depth-transition") {
        "/tmp/voxy-skin-depth-transition.png"
    } else if std::env::args().any(|a| a == "--skin-finish-transition") {
        "/tmp/voxy-skin-finish-transition.png"
    } else if std::env::args().any(|a| a == "--pupil-transition") {
        "/tmp/voxy-pupil-transition.png"
    } else if std::env::args().any(|a| a == "--brow-transition") {
        "/tmp/voxy-brow-transition.png"
    } else if std::env::args().any(|a| a == "--tongue-transition") {
        "/tmp/voxy-tongue-transition.png"
    } else if std::env::args().any(|a| a == "--oral-transition") {
        "/tmp/voxy-oral-transition.png"
    } else if std::env::args().any(|a| a == "--skin-closeup") {
        "/tmp/voxy-skin-closeup.png"
    } else if std::env::args().any(|a| a == "--face-preset") {
        "/tmp/voxy-face-preset-ray.png"
    } else if std::env::args().any(|a| a == "--blink-transition") {
        "/tmp/voxy-blink-transition.png"
    } else if std::env::args().any(|a| a == "--eye-closeup") {
        if std::env::args().any(|a| a == "--eye-side") {
            "/tmp/voxy-eye-side.png"
        } else {
            "/tmp/voxy-eye-closeup.png"
        }
    } else if std::env::args().any(|a| a == "--oral-closeup") {
        "/tmp/voxy-oral-closeup.png"
    } else if mouth {
        "/tmp/voxy-face-ray-mouth.png"
    } else {
        "/tmp/voxy-face-ray-preview.png"
    };
    image::save_buffer(path, &strip, 1728, 768, image::ColorType::Rgba8)?;
    Ok(path)
}

fn decode_visibility(bytes: &[u8]) -> Result<(Vec<f32>, usize), Box<dyn std::error::Error>> {
    let bits: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    if bits.iter().any(|&value| value > 1) {
        return Err("invalid GPU visibility flag".into());
    }
    let blocked = bits.iter().filter(|&&value| value == 0).count();
    let visible: Vec<f32> = bits
        .iter()
        .map(|&value| if value == 0 { 0.0 } else { 1.0 })
        .collect();
    assert!(blocked > 100, "character rays did not hit geometry");
    Ok((visible, blocked))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn studio_disk_samples_are_finite_deterministic_and_cover_source() {
        let axis = Vec3::new(-0.5, 0.7, 1.).normalize();
        let directions = area_light_directions(axis, 0.22, KEY_SAMPLES).unwrap();
        assert_eq!(
            directions,
            area_light_directions(axis, 0.22, KEY_SAMPLES).unwrap()
        );
        assert_eq!(directions.len(), KEY_SAMPLES);
        assert!(
            directions
                .iter()
                .all(|d| d.is_finite() && (d.length() - 1.).abs() < 1e-6 && d.dot(axis) > 0.97)
        );
        let tangent = axis.cross(Vec3::Y).normalize();
        assert!(directions.iter().any(|d| d.dot(tangent) > 0.1));
        assert!(directions.iter().any(|d| d.dot(tangent) < -0.1));
    }
    #[test]
    fn studio_sampling_rejects_zero_and_inexact_count_range() {
        let axis = Vec3::new(-0.5, 0.7, 1.).normalize();
        assert!(area_light_directions(axis, 0.22, 0).is_err());
        assert!(area_light_directions(axis, 0.22, usize::from(u16::MAX) + 1).is_err());
        let samples = area_light_directions(axis, 0.22, usize::from(u16::MAX)).unwrap();
        assert_eq!(samples.len(), usize::from(u16::MAX));
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }
    #[test]
    fn position_keys_do_not_saturate_or_split_signed_zero() {
        assert_ne!(
            position_key([1000.0, 0.0, 0.0]).unwrap(),
            position_key([2000.0, 0.0, 0.0]).unwrap()
        );
        assert_eq!(
            position_key([0.0; 3]).unwrap(),
            position_key([-0.0; 3]).unwrap()
        );
        assert_eq!(
            position_key([1e-9; 3]).unwrap(),
            position_key([-1e-9; 3]).unwrap()
        );
        assert!(position_key([f32::MAX, 0.0, 0.0]).is_err());
        assert!(position_key([f32::NAN, 0.0, 0.0]).is_err());
    }
}
