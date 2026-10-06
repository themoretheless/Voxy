//! GPU snapshots of the existing neutral skeleton and volumetric tissue demo.
#[path = "../src/biomechanics_demo.rs"]
mod biomechanics_demo;
#[path = "../src/tissue_demo.rs"]
mod tissue_demo;
#[path = "body_motion_snapshot/tissue_regions.rs"]
mod tissue_regions;
use glam::{DMat4, Mat4, Vec3};
use std::io::Write;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};
use voxy_render::{ModelAsset, ModelLimits, SceneMesh};
fn displayed_meshes(
    demo: &tissue_demo::TissueDemo,
    model: Option<&ModelAsset>,
    phase: f64,
    skin: Option<(&tissue_demo::TissueSkinBinding, &[DMat4])>,
) -> Result<Vec<SceneMesh>, Box<dyn std::error::Error>> {
    let Some(model) = model else {
        return Ok(vec![demo.mesh()?]);
    };
    let pose = model.sample_pose_phase(Some(0), phase)?;
    let deformed_skin = if let Some((binding, reference)) = skin {
        let pose64 = model.sample_pose_phase64(Some(0), phase)?;
        let current = pose64.skin_matrices(&model.skeleton)?;
        if current.len() != reference.len() {
            return Err("skin palette size changed".into());
        }
        let palette: Vec<_> = current
            .iter()
            .zip(reference)
            .map(|(a, b)| *a * b.inverse())
            .collect();
        let surfaces = model.scene_surfaces64(&pose64)?;
        let points: Vec<_> = surfaces.into_iter().flat_map(|s| s.positions).collect();
        Some(demo.deform_skin(binding, &palette, &points)?)
    } else {
        None
    };
    let mut cursor = 0;
    let mut meshes = Vec::new();
    for mesh in model.scene_meshes(&pose)? {
        let mesh = if let Some(points) = &deformed_skin {
            let end = cursor + mesh.vertices().len();
            let positions = points
                .get(cursor..end)
                .ok_or("skin render topology changed")?;
            let vertices = mesh
                .vertices()
                .iter()
                .zip(positions)
                .map(|(v, p)| {
                    let mut v = *v;
                    v.position = p.map(|x| x as f32);
                    v
                })
                .collect();
            cursor = end;
            SceneMesh::new(vertices, mesh.indices().to_vec())?
        } else {
            mesh
        };

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
    if deformed_skin.as_ref().is_some_and(|p| cursor != p.len()) {
        return Err("skin render vertex count changed".into());
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
/// Replay external-contact geometry only, without GPU, rig sampling or dynamics.
fn replay_contact_geometry(
    path: &std::path::Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if std::fs::metadata(path)?.len() > 16 * 1024 * 1024 {
        return Err("contact fixture exceeds 16 MiB".into());
    }
    let value: serde_json::Value = serde_json::from_reader(std::fs::File::open(path)?)?;
    if value["version"] != 1 || value["scope"] != "external-ccd-geometry" {
        return Err("unsupported contact geometry fixture".into());
    }
    let points = |key: &str| serde_json::from_value::<Vec<[f64; 3]>>(value[key].clone());
    let body = points("body_start")?;
    let predictor = points("body_predictor")?;
    if body.len() != predictor.len() || body.is_empty() {
        return Err("fixture body vertex count mismatch".into());
    }
    let faces: Vec<[usize; 3]> = serde_json::from_value(value["body_faces"].clone())?;
    let surface_faces: Vec<[usize; 3]> = serde_json::from_value(value["surface_faces"].clone())?;
    let parameters = &value["contact_parameters"];
    let scalar = |key: &str| parameters[key].as_f64().ok_or("missing contact parameter");
    let enabled: Vec<bool> = serde_json::from_value(value["contact_faces"].clone())?;
    let start = physics::biomechanics::PrescribedTriangleSurface::new(
        points("surface_start")?,
        surface_faces,
        scalar("minimum_distance_m")?,
        scalar("activation_gap_m")?,
        scalar("pair_stiffness_n_m")?,
    )?
    .with_contact_faces(enabled)?;
    let end = start.with_positions(points("surface_end")?)?;
    let mut samples = Vec::new();
    for index in 0..=32 {
        let t = index as f64 / 32.;
        let position = |a: [f64; 3], b: [f64; 3]| {
            std::array::from_fn(|axis| {
                if t <= 0.5 {
                    t.mul_add(b[axis] - a[axis], a[axis])
                } else {
                    (1. - t).mul_add(a[axis] - b[axis], b[axis])
                }
            })
        };
        let body: Vec<_> = body
            .iter()
            .zip(&predictor)
            .map(|(a, b)| position(*a, *b))
            .collect();
        let surface = start.with_positions(
            start
                .positions()
                .iter()
                .zip(end.positions())
                .map(|(a, b)| position(*a, *b))
                .collect(),
        )?;
        let feature = surface.nearest_active_contact(&body, &faces)?;
        samples.push(serde_json::json!({"time_fraction":t,"nearest":feature.map(|f|serde_json::json!({"gap_m":f.gap_m,"distance_m":f.distance_m,"body_face":f.body_face,"obstacle_face_index":f.obstacle_face_index,"body_weights":f.body_weights,"obstacle_weights":f.obstacle_weights}))}));
    }
    Ok(
        serde_json::json!({"scope":"external-ccd-geometry-only","static_start_open":start.path_is_open(&start,&body,&body,&faces)?,"stationary_body_path_open":start.path_is_open(&end,&body,&body,&faces)?,"predictor_path_open":start.path_is_open(&end,&body,&predictor,&faces)?,"samples":samples,"sampling_note":"Discrete gap samples do not prove continuous path admission."}),
    )
}
/// Evaluate native contact gaps after individual adjacent-f64 perturbations.
/// These are sensitivity observations, not interval error bounds or CCD proofs.
fn contact_precision_report(
    path: &std::path::Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if std::fs::metadata(path)?.len() > 16 * 1024 * 1024 {
        return Err("contact fixture exceeds 16 MiB".into());
    }
    let value: serde_json::Value = serde_json::from_reader(std::fs::File::open(path)?)?;
    if value["version"] != 1 || value["scope"] != "last-iterate-contact-precision" {
        return Err("unsupported contact precision fixture".into());
    }
    let parameters = &value["contact_parameters"];
    let scalar = |key: &str| parameters[key].as_f64().ok_or("missing contact parameter");
    let minimum = scalar("minimum_distance_m")?;
    let activation = scalar("activation_gap_m")?;
    let stiffness = scalar("pair_stiffness_n_m")?;
    let mut reports = Vec::new();
    for pose in value["poses"].as_array().ok_or("missing contact poses")? {
        let body: [[f64; 3]; 3] = serde_json::from_value(pose["body_triangle"].clone())?;
        let obstacle: [[f64; 3]; 3] = serde_json::from_value(pose["obstacle_triangle"].clone())?;
        let gap = |body: [[f64; 3]; 3], obstacle: [[f64; 3]; 3]| -> Result<f64, &'static str> {
            let surface = physics::biomechanics::PrescribedTriangleSurface::new(
                obstacle.to_vec(),
                vec![[0, 1, 2]],
                minimum,
                activation,
                stiffness,
            )?;
            surface
                .nearest_active_contact(&body, &[[0, 1, 2]])?
                .map(|feature| feature.gap_m)
                .ok_or("missing nearest contact")
        };
        let baseline = gap(body, obstacle)?;
        let mut trials = Vec::new();
        for side in 0..2 {
            for node in 0..3 {
                for axis in 0..3 {
                    for up in [false, true] {
                        let mut b = body;
                        let mut o = obstacle;
                        let point = if side == 0 {
                            &mut b[node]
                        } else {
                            &mut o[node]
                        };
                        let old = point[axis];
                        point[axis] = if up { old.next_up() } else { old.next_down() };
                        let delta = point[axis] - old;
                        let perturbed = gap(b, o)?;
                        trials.push(serde_json::json!({"side":if side==0 {"body"} else {"obstacle"},"node":node,"axis":axis,"up":up,"coordinate_delta_m":delta,"gap_m":perturbed,"gap_delta_m":perturbed-baseline}));
                    }
                }
            }
        }
        let flips = trials
            .iter()
            .filter(|t| (t["gap_m"].as_f64().unwrap() > 0.) != (baseline > 0.))
            .count();
        let origin = obstacle[0];
        let rebase =
            |triangle: [[f64; 3]; 3]| triangle.map(|p| std::array::from_fn(|i| p[i] - origin[i]));
        reports.push(serde_json::json!({"baseline_gap_m":baseline,"positive_gap_classification_changes":flips,"rebased_gap_m":gap(rebase(body),rebase(obstacle))?,"trials":trials}));
    }
    Ok(
        serde_json::json!({"scope":"native-contact-coordinate-sensitivity","poses":reports,"note":"Adjacent-f64 perturbations and origin translation are diagnostic; they do not bound distance error or certify a continuous path."}),
    )
}
/// A limited capture changes observation duration, never the simulation timestep.
fn capture_schedule(
    cesium: bool,
    frames: bool,
    limit: Option<usize>,
) -> Result<([usize; 4], Vec<usize>), &'static str> {
    if let Some(end) = limit {
        if !cesium || !(3..=480).contains(&end) {
            return Err("capture steps require --cesium and a value in 3..=480");
        }
        let snapshots = [0, end / 3, 2 * end / 3, end];
        let mut steps = snapshots.to_vec();
        if frames {
            steps.extend((0..=end / 12).map(|n| n * 12));
        }
        steps.sort_unstable();
        steps.dedup();
        return Ok((snapshots, steps));
    }
    let snapshots = if cesium {
        [0, 120, 240, 480]
    } else {
        [0, 168, 1080, 2160]
    };
    let steps = if frames {
        (0..=if cesium { 40 } else { 240 })
            .map(|n| n * 12)
            .collect()
    } else {
        snapshots.to_vec()
    };
    Ok((snapshots, steps))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: body_motion_snapshot OUTPUT.png [FRAME_DIRECTORY]")?;
    if path == "--check-volume-json" {
        let input = std::env::args()
            .nth(2)
            .ok_or("--check-volume-json requires a volume path")?;
        if std::env::args().nth(3).is_some() {
            return Err("unexpected volume audit argument".into());
        }
        const MODEL: &[u8] = include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb");
        let model = ModelAsset::parse(MODEL, &[], ModelLimits::default())?;
        let report = tissue_regions::check_volume(std::path::Path::new(&input), &model, MODEL)?;
        println!("{}", serde_json::to_string_pretty(report.value())?);
        if report.value()["complete_skin_binding"] != true {
            return Err("volume does not admit complete source skin binding".into());
        }
        return Ok(());
    }
    if path == "--contact-ulp-json" {
        let input = std::env::args()
            .nth(2)
            .ok_or("--contact-ulp-json requires a fixture path")?;
        if std::env::args().nth(3).is_some() {
            return Err("unexpected precision argument".into());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&contact_precision_report(std::path::Path::new(&input))?)?
        );
        return Ok(());
    }
    if path == "--replay-contact-json" {
        let input = std::env::args()
            .nth(2)
            .ok_or("--replay-contact-json requires a fixture path")?;
        if std::env::args().nth(3).is_some() {
            return Err("unexpected replay argument".into());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&replay_contact_geometry(std::path::Path::new(&input))?)?
        );
        return Ok(());
    }
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
                for (sample, offset) in demo.body_secondary_offsets()?.iter().enumerate() {
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
        println!("secondary offsets: {:?}", demo.body_secondary_offsets()?);
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
    let tissue_paths: Vec<_> = remaining
        .iter()
        .filter_map(|arg| arg.strip_prefix("--tissue-regions="))
        .collect();
    if tissue_paths.len() > 1 {
        return Err("tissue regions specified more than once".into());
    }
    if !tissue_paths.is_empty() && (!cesium || !contact) {
        return Err("--tissue-regions requires --cesium --contact".into());
    }
    let authored = tissue_paths
        .first()
        .map(|path| {
            tissue_regions::load(
                std::path::Path::new(path),
                imported.as_ref().unwrap(),
                include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            )
        })
        .transpose()?;
    let limits: Vec<_> = remaining
        .iter()
        .filter_map(|a| a.strip_prefix("--capture-steps="))
        .collect();
    if limits.len() > 1 {
        return Err("capture steps specified more than once".into());
    }
    let capture_limit = limits.first().map(|s| s.parse::<usize>()).transpose()?;
    capture_schedule(cesium, false, capture_limit)?;
    if remaining.iter().any(|arg| {
        arg.starts_with("--")
            && arg != "--close-up"
            && arg != "--cesium"
            && arg != "--contact"
            && arg != "--wide-contact"
            && !arg.starts_with("--capture-steps=")
            && !arg.starts_with("--tissue-regions=")
    }) {
        return Err(
            "usage: body_motion_snapshot OUTPUT.png [FRAME_DIRECTORY] [--close-up] [--cesium] [--contact] [--wide-contact] [--capture-steps=N] [--tissue-regions=MANIFEST.json]"
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
    let mut demo = if let Some(authored) = &authored {
        authored.value().instantiate()?
    } else if let Some(model) = &imported {
        imported_tissues(model, reference.as_ref().unwrap())?
    } else {
        tissue_demo::TissueDemo::body()
    };
    let contact_reference = if contact {
        let sample = contact_positions64;
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
        let domains = if let Some(authored) = &authored {
            authored.value().domains(&surface)?
        } else {
            attachment_domains(&surface, centers)?
        };
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
    if imported.is_some() {
        demo.assemble_regions()?;
        println!("TISSUE assembled regional dynamics into one global owner");
    }
    let skin_binding = if let Some(model) = &imported {
        let pose = model.sample_pose_phase64(Some(0), 0.)?;
        let points: Vec<_> = model
            .scene_surfaces64(&pose)?
            .into_iter()
            .flat_map(|s| s.positions)
            .collect();
        let binding = demo.bind_skin(&points)?;
        if let Some(authored) = &authored {
            authored.value().validate_skin_coverage(&binding)?;
            println!("TISSUE_COVERAGE {}", authored.value().coverage_report);
        }
        println!(
            "SKIN bound={} total={}",
            binding.bound_vertex_count(),
            points.len()
        );
        Some(binding)
    } else {
        None
    };
    if contact {
        let count = demo.bind_skin_contact(
            skin_binding
                .as_ref()
                .ok_or("missing physical skin binding")?,
        )?;
        println!(
            "SKIN physical contact: {count} responsive triangles; native FEM envelope disabled"
        );
    }
    let skin = skin_binding.as_ref().zip(reference64.as_deref());
    let meshes = displayed_meshes(&demo, imported.as_ref(), 0., skin)?;
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
    let capture_node_state = std::env::var_os("VOXY_CAPTURE_NODE_STATE").is_some();
    let mut node_states = Vec::new();
    // Publish each committed checkpoint independently, including when a later
    // interval fails. The aggregate JSON still appears only on full completion.
    let mut node_stream = capture_node_state
        .then(|| std::fs::File::create(std::path::Path::new(&path).with_extension("nodes.jsonl")))
        .transpose()?;
    let mut motion = String::from(
        "time_s,sample,offset_x_m,offset_y_m,offset_z_m,volume_m3,mechanical_change_j,support_work_j,viscous_heat_j,numerical_defect_j\n",
    );
    let mut elapsed = 0;
    let (snapshots, steps) = capture_schedule(cesium, frame_directory.is_some(), capture_limit)?;
    println!(
        "CAPTURE final_step={} simulated_s={:.9} limited={}",
        steps.last().unwrap(),
        *steps.last().unwrap() as f64 / 240.,
        capture_limit.is_some()
    );
    let diagnose_refinement = std::env::var_os("VOXY_REFINEMENT_DIAGNOSTICS").is_some();
    if diagnose_refinement {
        tissue_demo::start_refinement_diagnostics();
    }
    let render_start = std::time::Instant::now();
    let mut stage_seconds = [0.0_f64; 5];
    for (capture_index, step) in steps.into_iter().enumerate() {
        let stage_start = std::time::Instant::now();
        while elapsed < step {
            if cesium {
                if let Some(reference) = &contact_reference {
                    demo.advance_with_palette64_and_surfaces(1. / 240., |time| {
                        imported_contact_sample64(
                            imported.as_ref().ok_or("missing imported character")?,
                            reference64.as_ref().ok_or("missing imported physical reference")?,
                            reference,
                            (time / duration).min(1.),
                        )
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
        stage_seconds[0] += stage_start.elapsed().as_secs_f64();
        let stage_start = std::time::Instant::now();
        let volumes = demo.body_volumes_m3();
        let energy = demo.body_energy_receipts()?;
        if capture_node_state {
            let checkpoint = serde_json::json!({"time_s":step as f64 / 240., "nodes":demo.continuum_node_state()?, "energy_receipts":energy});
            if let Some(stream) = &mut node_stream {
                serde_json::to_writer(&mut *stream, &checkpoint)?;
                stream.write_all(b"\n")?;
                stream.flush()?;
            }
            node_states.push(checkpoint);
        }
        let offsets = if let Some(domains) = &contact_reference {
            let (palette, _) = imported_contact_sample64(
                imported.as_ref().ok_or("missing imported character")?,
                reference64
                    .as_ref()
                    .ok_or("missing imported physical reference")?,
                domains,
                (step as f64 / 240. / duration).min(1.),
            )?;
            demo.secondary_offsets_for_palette64(&palette)?
        } else if cesium {
            demo.secondary_offsets_for_palette(&palette_at(step as f64 / 240.)?)?
        } else {
            demo.body_secondary_offsets()?
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
        stage_seconds[1] += stage_start.elapsed().as_secs_f64();
        let stage_start = std::time::Instant::now();
        let meshes = displayed_meshes(
            &demo,
            imported.as_ref(),
            (step as f64 / (240. * duration)).min(1.),
            skin,
        )?;
        stage_seconds[2] += stage_start.elapsed().as_secs_f64();
        let stage_start = std::time::Instant::now();
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
        stage_seconds[3] += stage_start.elapsed().as_secs_f64();
        let stage_start = std::time::Instant::now();
        if let Some(directory) = &frame_directory {
            image::save_buffer(
                directory.join(format!("frame-{capture_index:04}.png")),
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
        stage_seconds[4] += stage_start.elapsed().as_secs_f64();
        if snapshots.contains(&step) {
            println!("frame t={:.2}", step as f64 / 240.);
        }
    }
    println!(
        "snapshot simulation + render seconds={:.9}; step receipts={:?}",
        render_start.elapsed().as_secs_f64(),
        demo.body_step_counts()
    );
    println!(
        "STAGE_SECONDS simulation={} receipts={} mesh_deformation={} upload_encode_gpu_readback={} capture_io={}",
        stage_seconds[0], stage_seconds[1], stage_seconds[2], stage_seconds[3], stage_seconds[4]
    );
    if diagnose_refinement {
        for ((reason, depth), count) in tissue_demo::take_refinement_diagnostics() {
            println!("REFINEMENT_REJECTION depth={depth} count={count} reason={reason}");
        }
    }
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
    image::save_buffer(&path, &strip, 2048, 512, image::ColorType::Rgba8)?;
    std::fs::write(std::path::Path::new(&path).with_extension("csv"), &motion)?;
    if capture_node_state {
        std::fs::write(
            std::path::Path::new(&path).with_extension("nodes.json"),
            serde_json::to_vec_pretty(&node_states)?,
        )?;
    }
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
    fn limited_capture_keeps_all_checkpoints_and_rejects_invalid_scope() {
        for end in [3, 8, 12, 479, 480] {
            let (snapshots, steps) = capture_schedule(true, true, Some(end)).unwrap();
            assert_eq!(steps.first(), Some(&0));
            assert_eq!(steps.last(), Some(&end));
            assert!(steps.windows(2).all(|p| p[0] < p[1]));
            assert!(snapshots.windows(2).all(|p| p[0] < p[1]));
            assert!(snapshots.iter().all(|s| steps.contains(s)));
        }
        assert!(capture_schedule(false, true, Some(8)).is_err());
        for end in [0, 1, 2, 481, usize::MAX] {
            assert!(capture_schedule(true, true, Some(end)).is_err());
        }
        assert_eq!(capture_schedule(true, true, None).unwrap().1.len(), 41);
    }
    #[test]
    fn imported_skin_binding_reports_actual_contained_vertices() {
        let model = ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            ModelLimits::default(),
        )
        .unwrap();
        let pose = model.sample_pose_phase64(Some(0), 0.).unwrap();
        let skin: Vec<_> = model
            .scene_surfaces64(&pose)
            .unwrap()
            .into_iter()
            .flat_map(|s| s.positions)
            .collect();
        if let Some(directory) = std::env::var_os("VOXY_SOURCE_SKIN_FIXTURE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let (points, boundary) = contact_positions64(&model, 0.).unwrap();
            assert_eq!(points, skin);
            std::fs::write(
                directory.join("source-surface.json"),
                serde_json::to_vec_pretty(
                    &serde_json::json!({"points":points,"boundary":boundary}),
                )
                .unwrap(),
            )
            .unwrap();
            let mut aliases = std::collections::BTreeMap::<[u64; 3], Vec<usize>>::new();
            for (i, p) in skin.iter().enumerate() {
                aliases
                    .entry(p.map(|x| if x == 0. { 0 } else { x.to_bits() }))
                    .or_default()
                    .push(i);
            }
            let aliases: Vec<_> = aliases.into_values().filter(|g| g.len() > 1).collect();
            let mut maximum_scatter = 0_f64;
            let mut worst = None;
            for step in 0..=480 {
                let phase = step as f64 / 480.;
                let (points, faces) = contact_positions64(&model, phase).unwrap();
                assert_eq!(faces, boundary);
                for group in &aliases {
                    for &node in &group[1..] {
                        let scatter = (0..3)
                            .map(|a| (points[node][a] - points[group[0]][a]).powi(2))
                            .sum::<f64>()
                            .sqrt();
                        assert!(scatter.is_finite());
                        if scatter > maximum_scatter {
                            maximum_scatter = scatter;
                            worst = Some((phase, group[0], node));
                        }
                    }
                }
            }
            std::fs::write(directory.join("animated-aliases.json"), serde_json::to_vec_pretty(
                &serde_json::json!({
                    "scope":"native imported phase preview, 481 evenly spaced phases including endpoints",
                    "duplicate_coordinate_groups":aliases.len(), "maximum_scatter_m":maximum_scatter,
                    "worst_phase_and_vertex_pair":worst,
                    "limits":"Sampled trajectory check; not a continuous seam equivalence certificate. No source vertices welded."
                })
            ).unwrap()).unwrap();
        }
        let reference = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let mut demo = imported_tissues(&model, &reference).unwrap();
        let binding = demo.bind_skin(&skin).unwrap();
        let owned_vertices = binding.tissue_owned_vertices();
        assert_eq!(owned_vertices.len(), binding.bound_vertex_count());
        assert!(owned_vertices.windows(2).all(|p| p[0] < p[1]));
        eprintln!(
            "IMPORTED_SKIN_BOUND vertices={} total={}",
            binding.bound_vertex_count(),
            skin.len()
        );
        let palette = vec![DMat4::IDENTITY; model.skeleton.joints().len()];
        assert_eq!(demo.deform_skin(&binding, &palette, &skin).unwrap(), skin);
        assert_eq!(binding.bound_vertex_count(), 16);
        let reference64 = pose.skin_matrices(&model.skeleton).unwrap();
        let before =
            displayed_meshes(&demo, Some(&model), 0., Some((&binding, &reference64))).unwrap();
        demo.step_body_with_contact64_workers(&palette, 0.5, None, 1)
            .unwrap();
        let after =
            displayed_meshes(&demo, Some(&model), 0., Some((&binding, &reference64))).unwrap();
        let changed = before[..model.primitives.len()]
            .iter()
            .zip(&after)
            .flat_map(|(a, b)| a.vertices().iter().zip(b.vertices()))
            .filter(|(a, b)| a.position != b.position)
            .count();
        assert_eq!(changed, 16);
        let physical_skin = demo.deform_skin(&binding, &palette, &skin).unwrap();
        demo.assemble_regions().unwrap();
        let global_binding = demo.bind_skin(&skin).unwrap();
        assert_eq!(global_binding.tissue_owned_vertices(), owned_vertices);
        assert_eq!(global_binding.bound_vertex_count(), 16);
        assert_eq!(
            demo.deform_skin(&global_binding, &palette, &skin).unwrap(),
            physical_skin
        );
        demo.step_body_with_contact64_workers(&palette, 0.5, None, 4)
            .unwrap();
        let global_meshes = displayed_meshes(
            &demo,
            Some(&model),
            0.,
            Some((&global_binding, &reference64)),
        )
        .unwrap();
        assert_eq!(global_meshes.len(), after.len());
    }
    #[test]
    fn captured_edge_contact_agrees_with_decimal_oracle_but_is_coordinate_sensitive() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/animation/contact-fixtures/last-iterate-3062.json"
        ));
        let report = contact_precision_report(path).unwrap();
        // Independent 90-digit Decimal edge-edge solution on exact binary64
        // inputs; both closest parameters lie strictly inside their edges.
        let oracle_gaps = [3.1222638107997636e-17, 4.234388982451942e-18];
        let poses = report["poses"].as_array().unwrap();
        assert_eq!(poses.len(), oracle_gaps.len());
        for (pose, oracle) in poses.iter().zip(oracle_gaps) {
            let baseline = pose["baseline_gap_m"].as_f64().unwrap();
            assert!((baseline - oracle).abs() < 1e-20);
            assert!(baseline > 0.);
            assert!(
                pose["positive_gap_classification_changes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert_eq!(
                pose["rebased_gap_m"].as_f64().unwrap().to_bits(),
                baseline.to_bits()
            );
            let trials = pose["trials"].as_array().unwrap();
            assert_eq!(trials.len(), 36);
            assert!(
                trials
                    .iter()
                    .all(|t| t["gap_m"].as_f64().unwrap().is_finite())
            );
        }
    }
    #[test]
    fn captured_vertex_contact_rejects_crossing_predictor_and_admits_stationary_body() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/animation/contact-fixtures/vertex-1308-predictor.json"
        ));
        let replay = replay_contact_geometry(path).unwrap();
        assert_eq!(replay["static_start_open"], true);
        assert_eq!(replay["stationary_body_path_open"], true);
        assert_eq!(replay["predictor_path_open"], false);
        let samples = replay["samples"].as_array().unwrap();
        let last = &samples.last().unwrap()["nearest"];
        assert_eq!(last["body_face"], serde_json::json!([10, 7, 11]));
        assert_eq!(last["obstacle_face_index"], 1308);
        assert!(last["gap_m"].as_f64().unwrap() < -4e-8);
    }
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
        // Includes the native-only regression at nominal step 71.
        check_wide_imported_contact(72);
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
    #[test]
    fn physical_render_skin_preserves_imported_path_and_energy() {
        check_imported_contact(72, true);
    }
    #[test]
    #[ignore = "manual full physical render-skin clip qualification"]
    fn physical_render_skin_completes_full_clip() {
        check_imported_contact(480, true);
    }
    fn check_wide_imported_contact(steps: usize) {
        check_imported_contact(steps, false);
    }
    fn check_imported_contact(steps: usize, physical_skin: bool) {
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
        if physical_skin {
            demo.assemble_regions().unwrap();
            let binding = demo.bind_skin(surface.positions()).unwrap();
            assert_eq!(binding.bound_vertex_count(), 16);
            assert_eq!(demo.bind_skin_contact(&binding).unwrap(), 56);
            assert_eq!(demo.body_energy_receipts().unwrap().len(), 1);
        }
        let mut previous = demo.body_energy_receipts().unwrap();
        let mut absolute_frame_error_j = vec![0.; previous.len()];
        let mut rounding_allowance_j = vec![0.; previous.len()];
        // Covers the previously failing initial guesses at steps 65 and 66.
        // The full 480-step clip is qualified separately by the ignored test.
        for step in 1..=steps {
            demo.advance_with_palette64_and_surfaces(1. / 240., |time| {
                imported_contact_sample64(&model, &reference64, &domains, (time / 2.).min(1.))
            })
            .unwrap_or_else(|error| panic!("imported contact rejected step {step}: {error}"));
            let current = demo.body_energy_receipts().unwrap();
            assert_eq!(current.len(), previous.len());
            for (index, (actual, prior)) in current.iter().zip(&previous).enumerate() {
                let stored_delta = actual[0] - prior[0];
                let work_delta = actual[1] - prior[1];
                let heat_delta = actual[2] - prior[2];
                let error = stored_delta - work_delta + heat_delta;
                let rounding = 64.
                    * f64::EPSILON
                    * (1.
                        + actual[..3]
                            .iter()
                            .chain(&prior[..3])
                            .map(|x| x.abs())
                            .sum::<f64>());
                assert!(
                    error.abs() <= 1e-5 + rounding,
                    "frame energy budget exceeded step={step} body={index} error_j={error:.17e}"
                );
                absolute_frame_error_j[index] += error.abs();
                rounding_allowance_j[index] += rounding;
            }
            previous = current;
            if step % 12 == 0 || step == steps {
                eprintln!("WIDE_IMPORTED_CONTACT_COMMITTED step={step}");
            }
        }
        let total_requested_budget_j = steps as f64 * 1e-5;
        for (index, &error) in absolute_frame_error_j.iter().enumerate() {
            assert!(error <= total_requested_budget_j + rounding_allowance_j[index]);
        }
        let audit = serde_json::json!({"scope":"independent nominal-frame mechanical energy audit","contact_boundary":if physical_skin { "assembled physical render skin" } else { "regional native FEM envelope" },"steps":steps,"absolute_frame_error_j":absolute_frame_error_j,"requested_budget_per_body_j":total_requested_budget_j,"rounding_allowance_j":rounding_allowance_j,"final_energy_receipts":previous,"reported_defect_subtracted":false});
        eprintln!("WIDE_IMPORTED_ENERGY_AUDIT {audit}");
        if let Some(path) = std::env::var_os("VOXY_ENERGY_AUDIT_FILE") {
            std::fs::write(path, serde_json::to_vec_pretty(&audit).unwrap()).unwrap();
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
