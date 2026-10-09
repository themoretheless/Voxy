//! Real anatomical CC0 mesh with a nonlinear full-body skin shell bound to its rendered surface.
use glam::Vec3;
use physics::secondary_motion::{Config as SecondaryConfig, ContactPlane, SecondaryMotion};
#[path = "skin_light_diffusion.rs"]
mod skin_light_diffusion;
#[path = "surface_normals.rs"]
mod surface_normals;
use physics::skin::{
    Attachment, ContactScene, ContactSphere, Skin, SkinEmbedding, SkinMaterial, SolverConfig,
    SurfaceBinding,
};
use std::collections::HashMap;
use voxy_render::{ObjAsset, ObjLimits, SceneMesh, SceneVertex};
#[path = "model_presentation.rs"]
mod presentation_receipts;
#[path = "tissue_diagnostics.rs"]
mod tissue_diagnostics;
#[path = "view_settings.rs"]
mod view_settings;
use presentation_receipts::record as presentation_record;
const BODY: &str =
    include_str!("../../../assets/characters/blender-female/prepared/body-forehead-refined.obj");
#[derive(Debug, PartialEq)]
struct SurfaceLightInput {
    points: Vec<[f32; 3]>,
    triangles: Vec<[usize; 3]>,
    source: Vec<[f64; 3]>,
}
const EYES: [&str; 2] = [
    include_str!("../../../assets/characters/blender-female/eye-l.obj"),
    include_str!("../../../assets/characters/blender-female/eye-r.obj"),
];
const PATCH: &str = include_str!(
    "../../../assets/characters/blender-female/prepared/body-forehead-refined-skin.json"
);
#[derive(Debug)]
pub(crate) struct FemaleDemo {
    rig: crate::female_rig::FemaleRig,
    grasp_mesh: Option<SceneMesh>,
    grasp_cache:
        std::collections::HashMap<String, (SceneMesh, Vec<Vec3>, crate::female_rig::PreparedGrasp)>,
    grasp_normals: Vec<Vec3>,
    pub(crate) hand_focus: bool,
    features: crate::female_features::FaceFeatures,
    lid_contour: Option<crate::female_face::LidContour>,
    hair: crate::female_hair::FemaleHair,
    pub(crate) film: Option<crate::surface_film_preview::FilmPreview>,
    pub(crate) simulate_hair: bool,
    pub(crate) show_hair: bool,
    pub(crate) surface_diffusion_enabled: bool,
    transmission_cache: std::cell::RefCell<crate::female_transmission::Cache>,
    surface_light_guess: std::cell::RefCell<Option<Vec<[f64; 3]>>>,
    surface_light_input: std::cell::RefCell<Option<SurfaceLightInput>>,
    vertices: Vec<SceneVertex>,
    indices: Vec<u32>,
    body_vertices: usize,
    normals: Vec<Vec3>,
    prepared_normals: surface_normals::PreparedNormals,
    embedding: SkinEmbedding,
    skin_rig: crate::female_rig::FemaleRig,
    targets: Vec<[f64; 3]>,
    last_residual: f64,
    min_area_ratio: f64,
    last_step_ms: f64,
    /// Elapsed solver time in milliseconds, skin then hair (they run concurrently).
    pub(crate) solver_ms: [f64; 2],
    skin: Skin,
    skin_reference_rest: Vec<[f64; 3]>,
    attachments: Vec<Attachment>,
    probe_anchor: usize,
    probe_center: [f64; 3],
    probe_depth: f64,
    probe_force: f64,
    pub(crate) probe_enabled: bool,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) distance: f32,
    pub(crate) pressing: bool,
    pub(crate) show_skin: bool,
    pub(crate) show_complexion: bool,
    pub(crate) preview_camera_eye: Option<Vec3>,
    pub(crate) preview_expression: Option<crate::female_face::FacePose>,
    pub(crate) show_strain: bool,
    pub(crate) parameter_file: Option<std::path::PathBuf>,
    parameter_revision: Option<std::time::SystemTime>,
    film_revision: Option<std::time::SystemTime>,
    view_revision: Option<std::time::SystemTime>,
    last_presentation_receipt: Option<std::time::Instant>,
    pub(crate) face_parameters: crate::face_parameters::FaceParameters,
    pub(crate) face_parameter_file: Option<std::path::PathBuf>,
    face_parameter_revision: Option<std::time::SystemTime>,
    pub(crate) body_parameters: crate::body_parameters::BodyParameters,
    cold_response: Option<crate::body_parameters::ColdResponse>,
    pub(crate) secondary_only: bool,
    pub(crate) rig_pose_only: bool,
    pub(crate) animation_only: bool,
    regions: Vec<crate::volume_regions::Region>,
    secondary: [SecondaryMotion; 4],
    animation_time: f64,
    time: f64,
    accumulator: f64,
    pub(crate) steps: usize,
    max_displacement: f64,
}
fn point(value: &serde_json::Value) -> Result<[f64; 3], Box<dyn std::error::Error>> {
    let a = value.as_array().ok_or("invalid skin vector")?;
    if a.len() != 3 {
        return Err("invalid skin vector size".into());
    }
    Ok([
        a[0].as_f64().ok_or("invalid skin number")?,
        a[1].as_f64().ok_or("invalid skin number")?,
        a[2].as_f64().ok_or("invalid skin number")?,
    ])
}
fn ids(value: &serde_json::Value) -> Result<[usize; 3], Box<dyn std::error::Error>> {
    let a = value.as_array().ok_or("invalid skin indices")?;
    if a.len() != 3 {
        return Err("invalid skin indices size".into());
    }
    Ok([
        usize::try_from(a[0].as_u64().ok_or("invalid index")?)?,
        usize::try_from(a[1].as_u64().ok_or("invalid index")?)?,
        usize::try_from(a[2].as_u64().ok_or("invalid index")?)?,
    ])
}
#[cfg(test)]
mod repaired_pose_export_tests {
    #[test]
    #[ignore = "exports actual repaired render surface for static posed collision audit"]
    fn export_repaired_body_pose_surfaces() {
        use std::io::Write;
        let directory = std::env::var("VOXY_BODY_POSE_EXPORT")
            .unwrap_or_else(|_| "/tmp/voxy-repaired-body-poses".into());
        std::fs::create_dir_all(&directory).unwrap();
        let mut model = super::FemaleDemo::male_from_assets(
            include_str!(
                "../../../assets/characters/blender-male/body-repaired-render-candidate.obj"
            ),
            include_str!(
                "../../../assets/characters/blender-male/body-repaired-skin-candidate.json"
            ),
        )
        .unwrap();
        model.animation_only = true;
        model.surface_diffusion_enabled = false;
        let baseline = model.body_parameters;
        let configurations = if std::env::var_os("VOXY_BODY_MORPH_EXPORT").is_some() {
            vec![
                (1.75, serde_json::json!({"height_cm":190.})),
                (
                    1.75,
                    serde_json::json!({"weight_kg":100.,"leg_length":1.2,"arm_length":1.2}),
                ),
            ]
        } else {
            vec![
                (0., serde_json::json!({})),
                (1.75, serde_json::json!({})),
                (3., serde_json::json!({})),
            ]
        };
        let mut samples = Vec::new();
        for (index, (time, patch)) in configurations.into_iter().enumerate() {
            model
                .set_body_parameters(baseline.patched(&patch).unwrap())
                .unwrap();
            model.preview_pose(time);
            let mesh = model.mesh().unwrap();
            let path = std::path::Path::new(&directory).join(format!("pose-{index}.obj"));
            let mut file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
            for vertex in &mesh.vertices()[..model.body_vertices] {
                let p = vertex.position;
                assert!(p.iter().all(|x| x.is_finite()));
                writeln!(file, "v {} {} {}", p[0], p[1], p[2]).unwrap();
            }
            let mut triangles = 0;
            for t in mesh.indices().chunks_exact(3) {
                if t.iter().all(|&v| (v as usize) < model.body_vertices) {
                    writeln!(file, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1).unwrap();
                    triangles += 1;
                }
            }
            file.flush().unwrap();
            samples.push(serde_json::json!({"path":path,"time_s":time,"parameters":model.body_parameters.to_json(),"vertices":model.body_vertices,"triangles":triangles}));
        }
        let report = serde_json::json!({"samples":samples,"animation_only":true,"scope":"actual body render vertices after rig/face deformation and mouth-seal removal; no XPBD dynamic advance or CCD"});
        std::fs::write(
            std::path::Path::new(&directory).join("samples.json"),
            report.to_string(),
        )
        .unwrap();
        println!("{report}");
    }
}
impl FemaleDemo {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_assets(BODY, EYES, PATCH)
    }
    pub(crate) fn female_from_assets(
        body_source: &str,
        patch: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_assets(body_source, EYES, patch)
    }
    pub(crate) fn new_male() -> Result<Self, Box<dyn std::error::Error>> {
        Self::male_from_assets(
            include_str!(
                "../../../assets/characters/blender-male/body-repaired-render-candidate.obj"
            ),
            include_str!(
                "../../../assets/characters/blender-male/body-repaired-skin-candidate.json"
            ),
        )
    }
    pub(crate) fn male_from_assets(
        body_source: &str,
        patch: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut model = Self::from_assets(
            body_source,
            [
                include_str!("../../../assets/characters/blender-male/eye-l.obj"),
                include_str!("../../../assets/characters/blender-male/eye-r.obj"),
            ],
            patch,
        )?;
        model.body_parameters.body_model = crate::body_parameters::BodyModel::Male;
        model.show_hair = false;
        model.simulate_hair = false;
        Ok(model)
    }
    fn from_assets(
        body_source: &str,
        eyes: [&str; 2],
        patch: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let body = ObjAsset::parse(body_source, ObjLimits::default())?;
        let mut vertices = body.mesh.vertices().to_vec();
        let mut indices = body.mesh.indices().to_vec();
        let body_vertices = vertices.len();
        let mut normals: Vec<Vec3> = body
            .normals
            .into_iter()
            .map(|n| Vec3::from_array(n.unwrap_or([0.0, 1.0, 0.0])))
            .collect();
        let mut lookup: HashMap<[u32; 3], Vec<usize>> = HashMap::new();
        for (i, v) in vertices.iter().enumerate() {
            lookup
                .entry(v.position.map(f32::to_bits))
                .or_default()
                .push(i);
        }
        for eye in eyes {
            let eye = ObjAsset::parse(eye, ObjLimits::default())?;
            let (eye_vertices, eye_indices) = crate::female_eyes::refined_globe(&eye.mesh);
            let offset = u32::try_from(vertices.len())?;
            normals.extend(eye_vertices.iter().map(|v| {
                let p = Vec3::from_array(v.position);
                (p - crate::female_eyes::center(p.x)).normalize()
            }));
            vertices.extend(eye_vertices);
            indices.extend(eye_indices.iter().map(|i| i + offset));
        }
        let data: serde_json::Value = serde_json::from_str(patch)?;
        let positions: Vec<_> = data["positions"]
            .as_array()
            .ok_or("missing skin points")?
            .iter()
            .map(point)
            .collect::<Result<_, _>>()?;
        let triangles: Vec<_> = data["triangles"]
            .as_array()
            .ok_or("missing skin triangles")?
            .iter()
            .map(ids)
            .collect::<Result<_, _>>()?;
        let pins: Vec<_> = data["pins"]
            .as_array()
            .ok_or("missing skin pins")?
            .iter()
            .map(|v| {
                usize::try_from(v.as_u64().ok_or("invalid skin pin")?).map_err(|_| "invalid pin")
            })
            .collect::<Result<_, _>>()?;
        let count = triangles.len();
        let mut skin = Skin::new(
            positions,
            triangles,
            &pins,
            SkinMaterial::default(),
            data["directions"]
                .as_array()
                .ok_or("missing skin directions")?
                .iter()
                .map(point)
                .collect::<Result<_, _>>()?,
        )?;
        skin.set_element_workers(
            std::thread::available_parallelism().map_or(1, |n| n.get().min(8)),
        )?;
        // Illustrative regional stiffness; topology and surface mass density are retained.
        let stiffness: Vec<_> = skin
            .triangles()
            .iter()
            .map(|ids| {
                let p: [f64; 3] = std::array::from_fn(|k| {
                    ids.iter().map(|&i| skin.rest_positions()[i][k] / 3.).sum()
                });
                if p[0].abs() > 0.30 && p[1] < 0.15 {
                    1.6
                } else if (0.24..0.48).contains(&p[1]) && p[0].abs() < 0.18 {
                    0.7
                } else if (0.02..0.24).contains(&p[1]) && p[0].abs() < 0.17 {
                    0.85
                } else {
                    1.0
                }
            })
            .collect();
        skin.set_face_properties(&stiffness, &vec![1.; stiffness.len()])?;
        let mut bindings = Vec::new();
        let mut covered = vec![false; body_vertices];
        for b in data["bindings"].as_array().ok_or("missing skin bindings")? {
            let position = point(&b["position"])?;
            let key = position.map(|v| (v as f32).to_bits());
            let matches = lookup
                .get(&key)
                .ok_or("skin binding has no rendered vertex")?;
            let triangle = usize::try_from(b["triangle"].as_u64().ok_or("invalid binding face")?)?;
            let weights = point(&b["weights"])?;
            for &vertex in matches {
                if covered[vertex] {
                    continue;
                }
                covered[vertex] = true;
                bindings.push(SurfaceBinding {
                    vertex,
                    triangle,
                    weights,
                });
            }
        }
        let embedding = SkinEmbedding::new(
            body_vertices,
            skin.positions().len(),
            skin.triangles().to_vec(),
            bindings,
            true,
        )?;
        let skin_vertices: Vec<_> = skin
            .rest_positions()
            .iter()
            .map(|p| SceneVertex {
                position: p.map(|v| v as f32),
                uv: [0.; 2],
                color: [1.; 4],
            })
            .collect();
        let skin_rig = crate::female_rig::FemaleRig::new(&skin_vertices)?;
        let targets = skin_rig.pose_points(skin.rest_positions(), 0.);
        let attachments = skin
            .rest_positions()
            .iter()
            .enumerate()
            .map(|(i, &target)| {
                let smooth = |a: f64, b: f64, x: f64| {
                    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
                    t * t * (3.0 - 2.0 * t)
                };
                // Thin skin over the forearm/hand is supported more tightly
                // than soft torso tissue. Blend the foundation across the elbow.
                let bony =
                    smooth(0.20, 0.27, target[0].abs()) * (1.0 - smooth(0.24, 0.36, target[1]));
                Attachment {
                    vertex: i,
                    target,
                    velocity: [0.0; 3],
                    stiffness: skin.masses()[i] / skin.material().area_density()
                        * 250_000.0
                        * (1.0 + 3.0 * bony),
                    viscosity: skin.masses()[i] / skin.material().area_density() * 1_000.0,
                }
            })
            .collect();
        println!(
            "BODY MODEL: {} vertices, {} triangles; actual surface skin: {} vertices, {} triangles; {} render bindings",
            body_vertices,
            body.mesh.indices().len() / 3,
            skin.positions().len(),
            count,
            embedding.bindings().len()
        );
        let mut rig = crate::female_rig::FemaleRig::new(&vertices)?;
        rig.bind_hand_surface(&vertices, &indices)?;
        let hair =
            crate::female_hair::FemaleHair::new(&vertices[..body_vertices], body.mesh.indices())?;
        let probe_anchor = skin
            .rest_positions()
            .iter()
            .enumerate()
            .filter(|(_, p)| p[0].abs() < 0.06 && (0.08..0.24).contains(&p[1]))
            .max_by(|(_, a), (_, b)| a[2].total_cmp(&b[2]))
            .map(|(i, _)| i)
            .ok_or("missing anterior probe anchor")?;
        let mut probe_center = targets[probe_anchor];
        probe_center[2] += 0.031;
        let regions = crate::volume_regions::Region::build(&vertices[..body_vertices])?;
        let features =
            crate::female_features::FaceFeatures::new(&vertices[..body_vertices], &indices);
        let lid_contour = (std::env::var("VOXY_EXPERIMENTAL_LID_CONTOUR").as_deref() != Ok("0"))
            .then(|| crate::female_face::LidContour::new(&vertices[..body_vertices], &indices));
        let lip_triangles = crate::female_features::open_mouth_surface(&vertices, &mut indices);
        eprintln!("Facial render mouth opening: {lip_triangles} seal triangles");
        let prepared_normals = surface_normals::PreparedNormals::new(&vertices, &indices);
        Ok(Self {
            prepared_normals,
            grasp_mesh: None,
            grasp_cache: std::collections::HashMap::new(),
            grasp_normals: Vec::new(),
            hand_focus: false,
            rig,
            features,
            lid_contour,
            hair,
            film: None,
            show_hair: true,
            simulate_hair: std::env::var("VOXY_SKIN_ONLY").as_deref() != Ok("1"),
            surface_diffusion_enabled: true,
            transmission_cache: Default::default(),
            surface_light_guess: Default::default(),
            surface_light_input: Default::default(),
            vertices,
            indices,
            body_vertices,
            normals,
            embedding,
            skin_rig,
            targets,
            last_residual: 0.,
            min_area_ratio: 1.,
            last_step_ms: 0.,
            solver_ms: [0.; 2],
            skin_reference_rest: skin.rest_positions().to_vec(),
            skin,
            attachments,
            probe_anchor,
            probe_center,
            probe_depth: 0.,
            probe_force: 0.,
            probe_enabled: false,
            yaw: 0.15,
            pitch: 0.0,
            distance: 2.4,
            pressing: false,
            show_skin: false,
            show_complexion: true,
            preview_camera_eye: None,
            preview_expression: None,
            show_strain: false,
            regions,
            parameter_file: None,
            parameter_revision: None,
            film_revision: None,
            view_revision: None,
            last_presentation_receipt: None,
            body_parameters: Default::default(),
            cold_response: None,
            face_parameters: Default::default(),
            face_parameter_file: None,
            face_parameter_revision: None,
            secondary_only: false,
            rig_pose_only: false,
            animation_only: false,
            secondary: std::array::from_fn(|i| {
                SecondaryMotion::new(SecondaryConfig {
                    frequency: if i < 2 { 3.5 } else { 3.8 },
                    damping_ratio: 0.16,
                })
                .expect("valid secondary material")
            }),
            animation_time: 0.0,
            time: 0.0,
            accumulator: 0.0,
            steps: 0,
            max_displacement: 0.0,
        })
    }
    pub(crate) fn record_presentation(&mut self, frame: u64, size: [u32; 2]) {
        let Some(path) = &self.parameter_file else {
            return;
        };
        if let Some(id) = presentation_receipts::pending_film_state(path) {
            if let Err(error) = presentation_receipts::publish_film_state(
                path,
                &id,
                &self.body_parameters.to_json(),
                self.time,
                frame,
                self.film.as_ref().map(|film| film.state_snapshot()),
            ) {
                eprintln!("Film state capture failed: {error}");
            }
        }
        if self
            .last_presentation_receipt
            .is_some_and(|t| t.elapsed().as_secs_f64() < 0.5)
        {
            return;
        }
        if let Err(error) = presentation_record(
            path,
            &self.body_parameters.to_json(),
            self.time,
            frame,
            size,
            &serde_json::json!({"view":{"mode":if self.show_strain {"strain"} else if self.show_skin {"displacement"} else {"material"}},"skinShell":tissue_diagnostics::measurements(&self.skin,!self.animation_only),"fluid":self.film.as_ref().map(|f| f.measurements()),
                "tissueCageVolumesM3":self.regions.iter().map(|r| r.volume_m3()).collect::<Vec<_>>(),
                "tissueCageMassesKg":self.regions.iter().map(|r| r.mass_kg()).collect::<Vec<_>>(),
                "secondaryDynamics": {
                    "enabled":self.secondary_only && !self.animation_only,
                    "steps":self.steps,
                    "rootDisplacementM":self.root_bob(self.time),
                    "regionLocalDisplacementsM":self.regions.iter().map(|r| r.maximum_local_displacement(self.root_bob(self.time))).collect::<Vec<_>>(),
                    "peakLocalDisplacementM":self.max_displacement
                },
                "tissueCageReferenceDensityKgM3":1000.,
                "tissueCageOrder":["leftBreast","rightBreast","leftButtock","rightButtock","abdomen"],
                "anatomicalVolumeMeasurement":false,
                "coldResponse":{"enabled":self.cold_response.is_some(),
                    "target":self.body_parameters.nipple_cold_response,
                    "current":self.render_body_parameters().nipple_cold_response,
                    "solverCoupled":false}}),
        ) {
            eprintln!("Presentation receipt write failed: {error}");
        }
        self.last_presentation_receipt = Some(std::time::Instant::now());
    }
    /// Enable visual response dynamics without rebasing the tissue solver per frame.
    /// The saved body response remains the stimulus target, not the current response.
    pub(crate) fn enable_cold_response(
        &mut self,
        initial: f64,
        onset: f64,
        recovery: f64,
    ) -> Result<(), &'static str> {
        self.cold_response = Some(crate::body_parameters::ColdResponse::new(
            initial,
            f64::from(self.body_parameters.nipple_cold_response),
            onset,
            recovery,
        )?);
        Ok(())
    }
    fn advance_cold_response(&mut self, dt: f64) -> Result<(), &'static str> {
        if let Some(response) = &mut self.cold_response {
            response.set_target(f64::from(self.body_parameters.nipple_cold_response))?;
            response.advance(dt)?;
        }
        Ok(())
    }
    fn render_body_parameters(&self) -> crate::body_parameters::BodyParameters {
        self.cold_response.map_or(self.body_parameters, |state| {
            state.apply(self.body_parameters)
        })
    }
    pub(crate) fn set_body_parameters(
        &mut self,
        parameters: crate::body_parameters::BodyParameters,
    ) -> Result<(), &'static str> {
        parameters.validate()?;
        if parameters == self.body_parameters {
            return Ok(());
        }
        let mut geometry_parameters = parameters;
        geometry_parameters.areola_radius_mm = self.body_parameters.areola_radius_mm;
        geometry_parameters.areola_pigmentation = self.body_parameters.areola_pigmentation;
        geometry_parameters.left_areola_size = self.body_parameters.left_areola_size;
        geometry_parameters.right_areola_size = self.body_parameters.right_areola_size;
        if geometry_parameters == self.body_parameters {
            self.body_parameters = parameters;
            return Ok(());
        }
        if parameters.body_model != self.body_parameters.body_model || self.film.is_some() {
            let mut replacement = match parameters.body_model {
                crate::body_parameters::BodyModel::Male => Self::new_male(),
                crate::body_parameters::BodyModel::Female => Self::new(),
            }
            .map_err(|_| "cannot load requested body source")?;
            let same_source = parameters.body_model == self.body_parameters.body_model;
            replacement.animation_only = self.animation_only;
            replacement.secondary_only = self.secondary_only;
            replacement.rig_pose_only = self.rig_pose_only;
            if same_source {
                // Force a rebase even when resetting to source defaults at a nonzero pose time.
                replacement.body_parameters = self.body_parameters;
                replacement.time = self.time;
                replacement.animation_time = self.animation_time;
                replacement.accumulator = self.accumulator;
                replacement.steps = self.steps;
                replacement.simulate_hair = self.simulate_hair;
                replacement.show_hair = self.show_hair;
                replacement.pressing = self.pressing;
                replacement.probe_enabled = self.probe_enabled;
            }
            replacement.set_body_parameters(parameters)?;
            if let Some(film) = &self.film {
                let indices: Vec<_> = replacement
                    .indices
                    .chunks_exact(3)
                    .filter(|t| t.iter().all(|&i| (i as usize) < replacement.body_vertices))
                    .flatten()
                    .copied()
                    .collect();
                replacement.film = Some(if same_source {
                    film.rebound_same_body(
                        &replacement.vertices[..replacement.body_vertices],
                        &indices,
                    )?
                } else {
                    film.remapped_body(
                        &replacement.vertices[..replacement.body_vertices],
                        &indices,
                        self.body_parameters.body_model,
                    )?
                });
            }
            replacement.parameter_file = self.parameter_file.clone();
            replacement.parameter_revision = self.parameter_revision;
            replacement.film_revision = self.film_revision;
            replacement.view_revision = self.view_revision;
            replacement.face_parameter_file = self.face_parameter_file.clone();
            replacement.face_parameter_revision = self.face_parameter_revision;
            replacement.face_parameters = self.face_parameters.clone();
            replacement.show_skin = self.show_skin;
            replacement.show_strain = self.show_strain;
            replacement.show_complexion = self.show_complexion;
            replacement.surface_diffusion_enabled = self.surface_diffusion_enabled;
            replacement.animation_only = self.animation_only;
            replacement.secondary_only = self.secondary_only;
            replacement.rig_pose_only = self.rig_pose_only;
            replacement.yaw = self.yaw;
            replacement.pitch = self.pitch;
            replacement.distance = self.distance;
            replacement.preview_camera_eye = self.preview_camera_eye;
            replacement.preview_expression = self.preview_expression;
            replacement.cold_response = self.cold_response;
            replacement.hand_focus = self.hand_focus;
            if replacement.film.is_some() {
                let mesh = replacement
                    .mesh()
                    .map_err(|_| "invalid replacement fluid substrate")?;
                replacement
                    .film
                    .as_mut()
                    .unwrap()
                    .update_substrate(mesh.vertices())?;
            }
            *self = replacement;
            return Ok(());
        }
        let body_indices: Vec<_> = self
            .indices
            .chunks_exact(3)
            .filter(|t| t.iter().all(|&i| (i as usize) < self.body_vertices))
            .flatten()
            .copied()
            .collect();
        parameters.surface_quality(&self.vertices[..self.body_vertices], &body_indices)?;
        let hair = crate::female_hair::FemaleHair::new_parameterized(
            &self.vertices[..self.body_vertices],
            &body_indices,
            parameters,
        )?;
        let regions = crate::volume_regions::Region::build_parameterized(
            &self.vertices[..self.body_vertices],
            parameters,
        )?;
        let rest: Vec<_> = self
            .skin_reference_rest
            .iter()
            .map(|p| {
                if parameters == Default::default() {
                    *p
                } else {
                    parameters.transform(p.map(|x| x as f32)).map(f64::from)
                }
            })
            .collect();
        let mut skin = self.skin.rebased(rest)?;
        let mut targets = if self.secondary_only {
            if self.animation_only {self.skin_reference_rest.clone()}
            else {self.skin_rig.jump_pose_points(&self.skin_reference_rest, self.time)}
        } else {
            self.skin_rig
                .pose_points(&self.skin_reference_rest, self.time as f32)
        };
        for target in &mut targets {
            if !self.animation_only {
                target[1] += self.root_bob(self.time);
            }
            if parameters != Default::default() {
                *target = parameters
                    .transform(target.map(|x| x as f32))
                    .map(f64::from);
            }
        }
        skin.set_state(targets.clone(), vec![[0.; 3]; targets.len()])?;
        let mut attachments = self.attachments.clone();
        for (i, attachment) in attachments.iter_mut().enumerate() {
            let scale = skin.masses()[i] / self.skin.masses()[i];
            attachment.stiffness *= scale;
            attachment.viscosity *= scale;
            attachment.target = targets[i];
            attachment.velocity = [0.; 3];
        }
        // Publish only after the rebuilt shell and all dependent state validate.
        self.skin = skin;
        self.hair = hair;
        self.targets = targets;
        self.attachments = attachments;
        self.probe_center = self.targets[self.probe_anchor];
        self.probe_center[2] += 0.031;
        self.probe_depth = 0.;
        self.probe_force = 0.;
        self.regions = regions;
        self.body_parameters = parameters;
        for state in &mut self.secondary {
            state.reset();
        }
        Ok(())
    }
    pub(crate) fn enable_film(
        &mut self,
        center: [f64; 3],
        radius: f64,
        volume_m3: f64,
    ) -> Result<(), &'static str> {
        self.film = Some(crate::surface_film_preview::FilmPreview::new(
            &self.vertices[..self.body_vertices],
            &self
                .indices
                .chunks_exact(3)
                .filter(|t| t.iter().all(|&i| (i as usize) < self.body_vertices))
                .flatten()
                .copied()
                .collect::<Vec<_>>(),
            center,
            radius,
            volume_m3,
        )?);
        Ok(())
    }
    fn advance_hair(&mut self, h: f64, time: f64, head: glam::Mat4) -> Result<(), &'static str> {
        if !self.simulate_hair {
            return Ok(());
        }
        let posed = self.hair_collider_pose(time);
        self.hair
            .advance(h, time, head, &posed[..self.body_vertices])
    }
    fn hair_collider_pose(&self, time: f64) -> Vec<SceneVertex> {
        let mut posed = self.vertices.clone();
        if self.secondary_only {
            if !self.animation_only {self.rig.deform_jump(&mut posed, time);}
        } else {
            self.rig.deform(&mut posed, time as f32);
        }
        let bob = self.root_bob(time);
        for v in &mut posed {
            v.position[1] += bob as f32;
        }
        self.body_parameters.apply(&mut posed);
        posed
    }
    fn hair_head_matrix(&self, time: f64) -> glam::Mat4 {
        let bob = self.root_bob(time) as f32;
        let head = if self.secondary_only {
            if self.animation_only {glam::Mat4::IDENTITY} else {self.rig.jump_head_matrix(time)}
        } else {
            self.rig.head_matrix(time as f32)
        };
        glam::Mat4::from_translation(Vec3::new(0.0, bob, 0.0)) * head
    }
    // Conjugate the canonical head pose into the edited head's local affine
    // frame. Collider/root positions still come from the full nonlinear morph.
    fn hair_physics_head_matrix(&self, time: f64) -> glam::Mat4 {
        if self.body_parameters == Default::default() {
            return self.hair_head_matrix(time);
        }
        let center = Vec3::new(0., 0.72, 0.06);
        let morph = |p: Vec3| Vec3::from_array(self.body_parameters.transform(p.to_array()));
        let mapped = morph(center);
        let x = (morph(center + Vec3::X * 0.01) - mapped) / 0.01;
        let y = (morph(center + Vec3::Y * 0.01) - mapped) / 0.01;
        let z = (morph(center + Vec3::Z * 0.01) - mapped) / 0.01;
        let translation = mapped - x * center.x - y * center.y - z * center.z;
        let frame = glam::Mat4::from_cols(
            x.extend(0.),
            y.extend(0.),
            z.extend(0.),
            translation.extend(1.),
        );
        frame * self.hair_head_matrix(time) * frame.inverse()
    }
    // One trajectory drives the rendered root, hair attachments and tissue excitation.
    // The returned acceleration is the analytic second derivative of the position.
    fn root_motion(&self, time: f64) -> (f64, f64) {
        if !self.secondary_only || self.animation_only {
            return (0.0, 0.0);
        }
        let motion = crate::jump_motion::sample(time);
        (motion.height, motion.acceleration)
    }
    fn root_bob(&self, time: f64) -> f64 {
        self.root_motion(time).0
    }
    fn face_pose(&self) -> crate::female_face::FacePose {
        let mut pose = self
            .preview_expression
            .unwrap_or_else(|| crate::female_face::FacePose::sample(self.time as f32));
        let camera = self.preview_camera_eye.unwrap_or_else(|| self.eye());
        let local = self
            .hair_head_matrix(self.time)
            .inverse()
            .transform_point3(camera);
        for (index, side) in [1., -1.].into_iter().enumerate() {
            let direction = (local - crate::female_eyes::center(side)).normalize();
            if direction.z > 0.3 {
                pose.gaze_offsets[index] = [
                    direction.x.atan2(direction.z).clamp(-0.22, 0.22),
                    (-direction.y).asin().clamp(-0.15, 0.15),
                ];
            }
        }
        pose
    }
    /// Samples a rig pose for offscreen inspection without advancing the physical solver.
    #[allow(dead_code)] // Also compiled directly into the offscreen render example.
    pub(crate) fn set_grasp_target(
        &mut self,
        shape: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_grasp_target_with_offset(shape, Vec3::ZERO)
    }
    pub(crate) fn set_grasp_target_with_offset(
        &mut self,
        shape: &str,
        offset: Vec3,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::female_rig::GraspObject;
        if !offset.is_finite() {
            return Err("grasp offset must be finite".into());
        }
        let cache_key = if offset == Vec3::ZERO {
            shape.to_owned()
        } else {
            format!("{shape}@{},{},{}", offset.x, offset.y, offset.z)
        };
        if let Some((mesh, normals, prepared)) = self.grasp_cache.get(&cache_key) {
            self.rig.apply_prepared_grasp(prepared);
            self.skin_rig.apply_prepared_grasp(prepared);
            self.grasp_mesh = Some(mesh.clone());
            self.grasp_normals = normals.clone();
            return Ok(());
        }

        let center = Vec3::new(0.350, -0.075, 0.078) + offset;
        let (object, mesh) = if shape == "sphere" || shape == "cylinder" {
            let sphere = shape == "sphere";
            let radius = if sphere { 0.032 } else { 0.024 };
            let half_length = 0.060;
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            for row in 0..=16 {
                let fraction = row as f32 / 16.;
                let angle = fraction * std::f32::consts::PI;
                let r = if sphere { radius * angle.sin() } else { radius };
                let z = if sphere {
                    radius * angle.cos()
                } else {
                    (fraction * 2. - 1.) * half_length
                };
                for column in 0..=32 {
                    let theta = column as f32 / 32. * std::f32::consts::TAU;
                    vertices.push(SceneVertex {
                        position: (center + Vec3::new(r * theta.cos(), r * theta.sin(), z))
                            .to_array(),
                        uv: [0., 0.],
                        color: [0.08, 0.35, 0.70, 1.],
                    });
                }
            }
            for row in 0..16 {
                for column in 0..32 {
                    let a = row * 33 + column;
                    if sphere {
                        indices.extend_from_slice(&[a, a + 33, a + 1, a + 1, a + 33, a + 34]);
                    } else {
                        indices.extend_from_slice(&[a, a + 1, a + 33, a + 1, a + 34, a + 33]);
                    }
                }
            }
            if !sphere {
                for row in [0, 16] {
                    let tip = vertices.len() as u32;
                    vertices.push(SceneVertex {
                        position: (center
                            + Vec3::Z * if row == 0 { -half_length } else { half_length })
                        .to_array(),
                        uv: [0., 0.],
                        color: [0.08, 0.35, 0.70, 1.],
                    });
                    for column in 0..32 {
                        let a = row * 33 + column;
                        indices.extend_from_slice(&if row == 0 {
                            [tip, a + 1, a]
                        } else {
                            [tip, a, a + 1]
                        });
                    }
                }
            }
            let object = if sphere {
                GraspObject::Sphere { center, radius }
            } else {
                GraspObject::Cylinder {
                    center,
                    radius,
                    half_length,
                }
            };
            (object, SceneMesh::new(vertices, indices)?)
        } else {
            // Seat the slender preset closer to the palm than the wider targets.
            let center = if shape == "handle" {
                Vec3::new(0.360, -0.055, 0.078)
            } else {
                Vec3::new(0.350, -0.070, 0.078)
            } + offset;
            let text = if shape == "handle" {
                include_str!("../../../assets/characters/grasp-handle.obj").to_owned()
            } else {
                std::fs::read_to_string(shape)?
            };
            let asset = ObjAsset::parse(&text, ObjLimits::default())?;
            let mut vertices = asset.mesh.vertices().to_vec();
            let low = vertices.iter().fold(Vec3::splat(f32::INFINITY), |p, v| {
                p.min(Vec3::from_array(v.position))
            });
            let high = vertices
                .iter()
                .fold(Vec3::splat(f32::NEG_INFINITY), |p, v| {
                    p.max(Vec3::from_array(v.position))
                });
            let scale = 0.075 / (high - low).max_element();
            if !scale.is_finite() {
                return Err("degenerate grasp object".into());
            }
            for v in &mut vertices {
                v.position = (center
                    + (Vec3::from_array(v.position) - (high + low) * 0.5)
                        * scale
                        * if shape == "handle" {
                            Vec3::new(1., 1., 1.6)
                        } else {
                            Vec3::ONE
                        })
                .to_array();
                v.color = [0.08, 0.35, 0.70, 1.];
            }
            let object = GraspObject::from_mesh(&vertices, asset.mesh.indices())?;
            (
                object,
                SceneMesh::new(vertices, asset.mesh.indices().to_vec())?,
            )
        };
        let object = std::sync::Arc::new(object);
        self.rig.set_grasp_object(Some(object));
        self.skin_rig.copy_grasp_target_from(&self.rig);
        let mut normals = vec![Vec3::ZERO; mesh.vertices().len()];
        for face in mesh.indices().chunks_exact(3) {
            let p: [Vec3; 3] = std::array::from_fn(|j| {
                Vec3::from_array(mesh.vertices()[face[j] as usize].position)
            });
            let normal = (p[1] - p[0]).cross(p[2] - p[0]);
            for &i in face {
                normals[i as usize] += normal;
            }
        }
        self.grasp_normals = normals.into_iter().map(Vec3::normalize_or_zero).collect();
        if matches!(shape, "cylinder" | "sphere" | "handle") {
            self.grasp_cache.insert(
                cache_key,
                (
                    mesh.clone(),
                    self.grasp_normals.clone(),
                    self.rig.prepared_grasp(),
                ),
            );
        }
        self.grasp_mesh = Some(mesh);
        Ok(())
    }
    pub(crate) fn prepare_grasp_presets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for shape in ["cylinder", "sphere", "handle"] {
            self.set_grasp_target(shape)?;
        }
        Ok(())
    }
    pub(crate) fn set_grasp_cycle(&mut self, enabled: bool) {
        self.rig.set_grasp_cycle(enabled);
        self.skin_rig.set_grasp_cycle(enabled);
    }
    pub(crate) fn set_hand_focus(&mut self, enabled: bool) {
        self.hand_focus = enabled;
        self.rig.set_body_motion(!enabled);
        self.skin_rig.set_body_motion(!enabled);
    }
    pub(crate) fn adjust_grasp(&mut self, delta: f32) -> Result<(), &'static str> {
        if !delta.is_finite() {
            return Err("grasp adjustment must be finite");
        }
        let amount = (self.rig.grasp_amount(self.time as f32) + delta).clamp(0., 1.);
        self.rig.set_grasp(amount)?;
        self.skin_rig.set_grasp(amount)?;
        Ok(())
    }
    /// Prescribed engineering strain fixture for offscreen diagnostic validation.
    pub(crate) fn set_strain_fixture(&mut self, x_stretch: f64) -> Result<(), &'static str> {
        if !x_stretch.is_finite() || !(0.5..=1.5).contains(&x_stretch) {
            return Err("invalid diagnostic stretch");
        }
        let positions = self
            .skin
            .rest_positions()
            .iter()
            .map(|p| [p[0] * x_stretch, p[1], p[2]])
            .collect();
        self.skin
            .set_state(positions, vec![[0.; 3]; self.skin.positions().len()])
    }
    pub(crate) fn gpu_hair_surface_frames(&self) -> Vec<voxy_render::FiberSurfaceFrame> {
        self.hair.gpu_surface_frames()
    }
    pub(crate) fn gpu_hair_surface_input(&self) -> Result<voxy_render::FiberSurfaceInput, voxy_render::SceneError> {
        self.hair.gpu_surface_input()
    }
    pub(crate) fn simulation_time(&self) -> f64 {
        self.time
    }
    pub(crate) fn preview_pose(&mut self, time: f64) {
        self.time = time;
    }
    pub(crate) fn title(&self) -> String {
        let mut legend = String::new();
        if self.show_strain {
            legend.push_str("Strain magnitude: blue 0%, green 5%, red >=10% | ");
        } else if self.show_skin {
            legend.push_str("Displacement: blue 0 mm, green 5 mm, red >=10 mm | ");
        }
        if self
            .film
            .as_ref()
            .is_some_and(|film| film.thickness_view_enabled())
        {
            legend.push_str("Film: blue 0 um, green 50 um, red >=100 um; <0.1 um hidden | ");
        }
        format!("{legend}{}", self.base_title())
    }
    pub(crate) fn diagnostic_legend_labels(&self) -> Vec<[&'static str; 3]> {
        let mut rows = Vec::new();
        if self.show_strain {
            rows.push(["0%", "5%", "10%+"]);
        } else if self.show_skin {
            rows.push(["0mm", "5mm", "10mm+"]);
        }
        if self
            .film
            .as_ref()
            .is_some_and(|film| film.thickness_view_enabled())
        {
            rows.push(["0um", "50um", "100um+"]);
        }
        rows
    }
    pub(crate) fn diagnostic_legend_visible(&self) -> bool {
        self.show_strain
            || self.show_skin
            || self
                .film
                .as_ref()
                .is_some_and(|film| film.thickness_view_enabled())
    }
    fn base_title(&self) -> String {
        if self.hand_focus {
            return format!(
                "Voxy hand grasp {:.0}% | 1: cylinder | 2: sphere | 3: handle | [/]: open/close | G: cycle | Space: pause | arrows: orbit/zoom",
                self.rig.grasp_amount(self.time as f32) * 100.
            );
        }
        if self.face_parameter_file.is_some() {
            return "Voxy face constructor | live preset | Space: pause | arrows: orbit/zoom"
                .into();
        }
        if self.secondary_only {
            return format!(
                "Voxy body physics | jumps then settling | steps {} | peak response {:.1} mm | arrows: orbit/zoom | Space: pause",
                self.steps,
                self.max_displacement * 1000.
            );
        }
        if self.animation_only {
            return "Voxy skeletal animation | relaxed arm gesture | 6 s loop | B: skin details | Space: pause | arrows: orbit/zoom".into();
        }
        format!(
            "Voxy {} | full-body skin | hair physics {} | arrows: orbit/zoom | P: pressure {} | C: probe {} | Fz {:.3} N | B: skin details | M: displacement 0-10 mm | K: strain 0-10% | Space: pause | max {:.1} mm | residual {:.2e} | step {:.1} ms",
            self.body_parameters.body_model.as_str(),
            if self.simulate_hair { "on" } else { "off" },
            if self.pressing { "on" } else { "off" },
            if self.probe_enabled { "in" } else { "out" },
            self.probe_force,
            self.max_displacement * 1000.0,
            self.last_residual,
            self.last_step_ms
        )
    }
    pub(crate) fn focus(&self) -> Vec3 {
        if self.hand_focus {
            self.rig
                .hand_matrix(self.time as f32, false)
                .transform_point3(Vec3::new(0.380, -0.075, 0.080))
        } else if self.face_parameter_file.is_some() {
            Vec3::new(0., 0.70, 0.12)
        } else {
            Vec3::ZERO
        }
    }
    pub(crate) fn eye(&self) -> Vec3 {
        let focus = self.focus();
        focus
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos() * self.distance,
                self.pitch.sin() * self.distance,
                self.yaw.cos() * self.pitch.cos() * self.distance,
            )
    }
    pub(crate) fn orbit(&mut self, x: f32, y: f32) {
        self.yaw += x;
        self.pitch = (self.pitch + y).clamp(-1.2, 1.2);
    }
    pub(crate) fn zoom(&mut self, amount: f32) {
        let (minimum, step) = if self.hand_focus {
            (0.18, 0.2)
        } else {
            (0.7, 1.0)
        };
        self.distance = (self.distance + amount * step).clamp(minimum, 5.0);
    }
    fn support_offset(&self, rest: [f64; 3], time: f64) -> f64 {
        if self.animation_only {
            return 0.0;
        }
        let bob = self.root_bob(time);
        if self.rig_pose_only {return bob;}
        let mut offset = bob;
        for (i, state) in self.secondary.iter().enumerate() {
            let center = [
                if i % 2 == 0 { -0.1 } else { 0.1 },
                if i < 2 { 0.36 } else { -0.10 },
                if i < 2 { 0.10 } else { -0.11 },
            ];
            let radius = [0.11_f64, 0.12, 0.10];
            let distance: f64 = (0..3)
                .map(|k| ((rest[k] - center[k]) / radius[k]).powi(2))
                .sum();
            offset += state.offset()[1] * (-distance).exp();
        }
        offset
    }
    pub(crate) fn background_view_replica(&self) -> Result<Self, Box<dyn std::error::Error>> {
        if !self.secondary_only
            || self.animation_only
            || self.rig_pose_only
            || self.parameter_file.is_some()
            || self.face_parameter_file.is_some()
            || self.film.is_some()
            || self.cold_response.is_some()
            || self.body_parameters != Default::default()
        {
            return Err(
                "background full-model mode requires the unparameterized secondary example".into(),
            );
        }
        // This copy is a passive UI/view facade. Only the transferred original is advanced.
        let mut view = Self::new()?;
        view.secondary_only = true;
        view.simulate_hair = self.simulate_hair;
        view.yaw = self.yaw;
        view.pitch = self.pitch;
        view.distance = self.distance;
        view.pressing = self.pressing;
        view.probe_enabled = self.probe_enabled;
        view.show_skin = self.show_skin;
        view.show_complexion = self.show_complexion;
        view.show_strain = self.show_strain;
        view.show_hair = self.show_hair;
        view.face_parameters = self.face_parameters.clone();
        view.preview_expression = self.preview_expression;
        Ok(view)
    }
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), &'static str> {
        if self.rig_pose_only {return Err("pose-only rig preview cannot advance physical state");}
        if let Some(path) = &self.face_parameter_file {
            if let Ok(revision) = std::fs::metadata(path).and_then(|m| m.modified()) {
                if self.face_parameter_revision != Some(revision) {
                    match std::fs::read_to_string(path)
                        .map_err(|e| e.to_string())
                        .and_then(|text| crate::face_parameters::FaceParameters::from_json(&text))
                    {
                        Ok(parameters) => {
                            self.face_parameters = parameters;
                            eprintln!("Face preset applied: {}", path.display());
                        }
                        Err(error) => eprintln!("Face parameter reload rejected: {error}"),
                    }
                    self.face_parameter_revision = Some(revision);
                }
            }
        }
        if let Some(path) = &self.parameter_file {
            if let Ok(revision) = std::fs::metadata(path).and_then(|m| m.modified()) {
                if self.parameter_revision != Some(revision) {
                    match std::fs::read_to_string(path)
                        .map_err(|e| e.to_string())
                        .and_then(|text| {
                            crate::body_parameters::BodyParameters::from_json(&text)
                                .map_err(|e| e.to_string())
                        }) {
                        Ok(parameters) => {
                            if let Err(error) = self.set_body_parameters(parameters) {
                                eprintln!("Body geometry rebuild rejected: {error}");
                            }
                        }
                        Err(error) => eprintln!("Body parameter reload rejected: {error}"),
                    }
                    self.parameter_revision = Some(revision);
                }
            }
        }

        if let Some(body_path) = &self.parameter_file {
            let path = view_settings::path(body_path);
            if let Ok(revision) = std::fs::metadata(&path).and_then(|m| m.modified()) {
                if self.view_revision != Some(revision) {
                    match view_settings::ViewSettings::read(&path) {
                        Ok(settings) => {
                            self.show_strain = settings.mode == "strain";
                            self.show_skin = settings.mode == "displacement";
                        }
                        Err(error) => eprintln!("View reload rejected: {error}"),
                    }
                    self.view_revision = Some(revision);
                }
            }
        }
        if let (Some(body_path), Some(film)) = (&self.parameter_file, &mut self.film) {
            let path = crate::film_settings::path(body_path);
            if let Ok(revision) = std::fs::metadata(&path).and_then(|m| m.modified()) {
                if self.film_revision != Some(revision) {
                    match crate::film_settings::FilmSettings::read(&path) {
                        Ok(settings) => {
                            if let Err(error) = film.configure(settings) {
                                eprintln!("Film settings rejected: {error}");
                            }
                        }
                        Err(error) => eprintln!("Film settings reload rejected: {error}"),
                    }
                    self.film_revision = Some(revision);
                }
            }
        }
        if !dt.is_finite() || dt < 0.0 {
            return Err("invalid animation timestep");
        }
        self.solver_ms = [0.; 2];
        if self.animation_only {
            let previous = (self.time, self.animation_time, self.cold_response);
            let result = (|| {
                self.advance_cold_response(dt)?;
                self.time += dt;
                self.animation_time = self.time;
                if self.film.is_some() && dt > 0. {
                    if dt > 0.1 {
                        return Err("film animation timestep exceeds 0.1 seconds");
                    }
                    let mesh = self.mesh().map_err(|_| "invalid film substrate")?;
                    self.film.as_mut().unwrap().advance(mesh.vertices(), dt)?;
                }
                Ok(())
            })();
            if result.is_err() {
                (self.time, self.animation_time, self.cold_response) = previous;
            }
            return result;
        }
        let previous_time = self.time;
        // Integrate elapsed frame time, with a bounded implicit step and backlog.
        // Hair independently subdivides this interval to at most 1/240 s.
        self.accumulator = (self.accumulator + dt.min(0.1)).min(0.1);
        let h = 1.0 / 120.0;
        let mut count = 0;
        while self.accumulator + 1e-12 >= h && count < 6 {
            let time = self.time + h;
            // Driven damped springs in metres; base excitation is the same root bob
            // rendered below. Separate stiffness for anterior/posterior soft regions.
            let acceleration = self.root_motion(time).1;
            for (index, state) in self.secondary.iter_mut().enumerate() {
                let size = if index < 2 {
                    self.body_parameters.breast_size
                        * if index % 2 == 0 {
                            self.body_parameters.left_breast_size
                        } else {
                            self.body_parameters.right_breast_size
                        }
                } else {
                    self.body_parameters.buttock_size
                        * if index % 2 == 0 {
                            self.body_parameters.left_buttock_size
                        } else {
                            self.body_parameters.right_buttock_size
                        }
                } as f64;
                state.configure(SecondaryConfig {
                    frequency: (if index < 2 { 3.5 } else { 3.8 }) / size.sqrt(),
                    damping_ratio: 0.16,
                })?;
                state.step_nonlinear(
                    h,
                    [0.0, acceleration, 0.0],
                    [0.0; 3],
                    1800.0,
                    &[
                        ContactPlane {
                            normal: [0.0, 1.0, 0.0],
                            limit: -0.04,
                            friction: 0.35,
                        },
                        ContactPlane {
                            normal: [0.0, -1.0, 0.0],
                            limit: -0.04,
                            friction: 0.35,
                        },
                    ],
                )?;
            }
            self.features.advance_lashes(h, [0., acceleration, 0.])?;
            if self.secondary_only {
                let bob = self.root_bob(time);
                for region in &mut self.regions {
                    region.step(h, bob)?;
                    self.max_displacement = self
                        .max_displacement
                        .max(region.maximum_local_displacement(bob));
                }
                // Retain the elastic skin solver in the inertial demo instead of freezing its state.
                let posed_shell = self.skin_rig.jump_pose_points(&self.skin_reference_rest, time);
                let targets: Vec<_> = posed_shell.iter().zip(&self.skin_reference_rest)
                    .map(|(posed, rest)| {
                        let mut p = *posed;
                        p[1] += self.support_offset(*rest, time);
                        self.body_parameters
                            .transform(p.map(|v| v as f32))
                            .map(f64::from)
                    })
                    .collect();
                for (i, attachment) in self.attachments.iter_mut().enumerate() {
                    attachment.target = self.targets[i];
                    attachment.velocity =
                        std::array::from_fn(|k| (targets[i][k] - self.targets[i][k]) / h);
                }
                let head = self.hair_physics_head_matrix(time);
                let hair_pose = self.simulate_hair.then(|| self.hair_collider_pose(time));
                let skin = &mut self.skin;
                let hair = &mut self.hair;
                let attachments = &self.attachments;
                let body_vertices = self.body_vertices;
                // Hair contacts use the independently posed canonical body,
                // so neither solver reads the other solver's mutable state.
                let (skin_ms, hair_ms) = std::thread::scope(|scope| {
                    let hair_step = hair_pose.as_ref().map(|posed| {
                        scope.spawn(move || {
                            let started = std::time::Instant::now();
                            let result = hair.advance(h, time, head, &posed[..body_vertices]);
                            (result, started.elapsed().as_secs_f64() * 1000.)
                        })
                    });
                    let started = std::time::Instant::now();
                    let skin_result = skin.step_adaptive(
                        h,
                        [0., -9.81, 0.],
                        &vec![[0.; 3]; targets.len()],
                        attachments,
                        SolverConfig {
                            force_tolerance: 1e-6,
                            ..Default::default()
                        },
                        6,
                    );
                    let skin_ms = started.elapsed().as_secs_f64() * 1000.;
                    let hair_ms = if let Some(hair_step) = hair_step {
                        let (result, elapsed) = hair_step.join().map_err(|_| "hair worker panicked")?;
                        result?;
                        elapsed
                    } else { 0. };
                    skin_result?;
                    Ok::<_, &'static str>((skin_ms, hair_ms))
                })?;
                self.solver_ms[0] += skin_ms;
                self.solver_ms[1] += hair_ms;
                self.targets = targets;
                self.advance_cold_response(h)?;
                self.time = time;
                self.animation_time = time;
                self.accumulator -= h;
                count += 1;
                self.steps += 1;
                continue;
            }
            let pressure = if self.pressing {
                900.0 * (0.5 - 0.5 * (time * 2.0).cos())
            } else {
                0.0
            };
            let forces: Vec<_> = self
                .skin
                .rest_positions()
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let r = (p[0] / 0.045).powi(2)
                        + ((p[1] - 0.2) / 0.045).powi(2)
                        + ((p[2] - 0.12) / 0.06).powi(2);
                    let area = self.skin.masses()[i] / self.skin.material().area_density();
                    [0.0, 0.0, -pressure * (-r).exp() * area]
                })
                .collect();
            let mut targets = self
                .skin_rig
                .pose_points(&self.skin_reference_rest, time as f32);
            for (target, rest) in targets.iter_mut().zip(&self.skin_reference_rest) {
                target[1] += self.support_offset(*rest, time);
                if self.body_parameters != Default::default() {
                    *target = self
                        .body_parameters
                        .transform(target.map(|x| x as f32))
                        .map(f64::from);
                }
            }
            for (i, attachment) in self.attachments.iter_mut().enumerate() {
                attachment.target = self.targets[i];
                attachment.velocity =
                    std::array::from_fn(|k| (targets[i][k] - self.targets[i][k]) / h);
            }
            let started = std::time::Instant::now();
            let next_depth = if self.probe_enabled {
                (self.probe_depth + h * 0.02).min(0.012)
            } else {
                (self.probe_depth - h * 0.02).max(0.)
            };
            let mut next_center = targets[self.probe_anchor];
            next_center[2] += 0.031 - next_depth;
            let contacts = ContactScene {
                spheres: if self.probe_enabled || self.probe_depth > 0. {
                    vec![ContactSphere {
                        center: self.probe_center,
                        radius: 0.02,
                        velocity: std::array::from_fn(|i| {
                            (next_center[i] - self.probe_center[i]) / h
                        }),
                    }]
                } else {
                    vec![]
                },
                distance: 0.003,
                stiffness: 100_000.,
                ..ContactScene::default()
            };
            let head = self.hair_physics_head_matrix(time);
            let hair_pose = self.simulate_hair.then(|| self.hair_collider_pose(time));
            let skin = &mut self.skin;
            let hair = &mut self.hair;
            let attachments = &self.attachments;
            let body_vertices = self.body_vertices;
            let (report, skin_ms, hair_ms) = std::thread::scope(|scope| {
                let hair_step = hair_pose.as_ref().map(|posed| {
                    scope.spawn(move || {
                        let started = std::time::Instant::now();
                        let result = hair.advance(h, time, head, &posed[..body_vertices]);
                        (result, started.elapsed().as_secs_f64() * 1000.)
                    })
                });
                let started = std::time::Instant::now();
                let report = skin.step_with_contacts(
                    h,
                    [0.0, -9.81, 0.0],
                    &forces,
                    attachments,
                    &contacts,
                    SolverConfig {
                        force_tolerance: 1e-6,
                        ..SolverConfig::default()
                    },
                );
                let skin_ms = started.elapsed().as_secs_f64() * 1000.;
                let hair_ms = if let Some(hair_step) = hair_step {
                    let (result, elapsed) = hair_step.join().map_err(|_| "hair worker panicked")?;
                    result?;
                    elapsed
                } else {
                    0.
                };
                Ok::<_, &'static str>((report?, skin_ms, hair_ms))
            })?;
            self.solver_ms[0] += skin_ms;
            self.solver_ms[1] += hair_ms;
            self.probe_center = next_center;
            self.probe_depth = next_depth;
            let endpoint_contacts = ContactScene {
                spheres: contacts
                    .spheres
                    .iter()
                    .map(|sphere| ContactSphere {
                        center: next_center,
                        velocity: [0.; 3],
                        ..*sphere
                    })
                    .collect(),
                ..contacts
            };
            self.probe_force = if endpoint_contacts.spheres.is_empty() {
                0.
            } else {
                self.skin
                    .contact_forces(&endpoint_contacts)?
                    .iter()
                    .map(|f| f[2])
                    .sum::<f64>()
                    .abs()
            };
            self.animation_time = time;
            self.last_step_ms = started.elapsed().as_secs_f64() * 1000.;
            self.last_residual = report.residual;
            self.min_area_ratio = report.min_area_ratio;
            self.targets = targets;
            self.advance_cold_response(h)?;
            self.time = time;
            self.accumulator -= h;
            count += 1;
            self.steps += 1;
            for (p, r) in self.skin.positions().iter().zip(&self.targets) {
                let d = p
                    .iter()
                    .zip(r)
                    .map(|(p, r)| (p - r).powi(2))
                    .sum::<f64>()
                    .sqrt();
                self.max_displacement = self.max_displacement.max(d);
            }
        }
        let consumed = self.time - previous_time;
        if self.film.is_some() && consumed > 0. {
            let mesh = self.mesh().map_err(|_| "invalid film substrate")?;
            self.film
                .as_mut()
                .unwrap()
                .advance(mesh.vertices(), consumed)?;
        }
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn gpu_secondary_binding(
        &self,
        count: usize,
    ) -> Result<(Vec<u32>, Vec<voxy_render::SurfaceDeformationWeight>, u32), voxy_render::SceneError>
    {
        if !self.secondary_only
            || self.rig_pose_only
            || self.film.is_some()
            || self.simulate_hair
            || self.cold_response.is_some()
            || self.parameter_file.is_some()
            || self.body_parameters != Default::default()
            || self.time != 0.
        {
            return Err(voxy_render::SceneError::InvalidGeometry);
        }
        let mut rows = vec![
            vec![voxy_render::SurfaceDeformationWeight {
                control: 0,
                weight: 1.
            }];
            count
        ];
        for (index, vertex) in self.vertices.iter().enumerate().take(self.body_vertices) {
            for i in 0..4 {
                let center = [
                    if i % 2 == 0 { -0.1 } else { 0.1 },
                    if i < 2 { 0.36 } else { -0.10 },
                    if i < 2 { 0.10 } else { -0.11 },
                ];
                let radius = [0.11_f64, 0.12, 0.10];
                let distance: f64 = (0..3)
                    .map(|k| ((f64::from(vertex.position[k]) - center[k]) / radius[k]).powi(2))
                    .sum();
                rows[index].push(voxy_render::SurfaceDeformationWeight {
                    control: 1 + i as u32,
                    weight: (-distance).exp() as f32,
                });
            }
        }
        for (i, region) in self.regions.iter().enumerate() {
            region.gpu_displacement_weights(5 + i as u32 * 7, &mut rows);
        }
        let mut offsets = vec![0];
        let mut weights = Vec::new();
        for row in rows {
            weights.extend(row);
            offsets.push(weights.len() as u32);
        }
        Ok((offsets, weights, 5 + self.regions.len() as u32 * 7))
    }
    pub(crate) fn gpu_secondary_body_vertices(&self) -> u32 {
        self.body_vertices as u32
    }
    pub(crate) fn gpu_jump_weights(&self) -> Vec<voxy_render::SurfaceRigidSkinWeight> {self.rig.gpu_jump_weights()}
    pub(crate) fn gpu_jump_palette(&self) -> Vec<glam::Mat4> {self.rig.jump_palette(if self.animation_only {5.} else {self.time})}
    pub(crate) fn gpu_secondary_controls(&self) -> Vec<[f32; 4]> {
        let bob = self.root_bob(self.time);
        let mut controls = vec![[0., bob as f32, 0., 0.]];
        controls.extend(
            self.secondary
                .iter()
                .map(|s| [0., s.offset()[1] as f32, 0., 0.]),
        );
        for region in &self.regions {
            controls.extend(region.gpu_displacements(bob));
        }
        controls
    }
    pub(crate) fn mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        self.mesh_with_hair(self.show_hair)
    }
    pub(crate) fn mesh_with_full_hair(&self) -> Result<SceneMesh,voxy_render::SceneError> {
        self.mesh_with_hair(true)
    }
    pub(crate) fn mesh_without_hair(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        self.mesh_with_hair(false)
    }
    fn mesh_with_hair(&self, include_hair: bool) -> Result<SceneMesh, voxy_render::SceneError> {
        let trace = std::env::var_os("VOXY_FACE_MESH_TRACE").is_some();
        let mut checkpoint = std::time::Instant::now();
        let mut mark = |stage: &str| {
            if trace {
                eprintln!(
                    "FACE MESH t={:.3} stage={stage} ms={:.3}",
                    self.time,
                    checkpoint.elapsed().as_secs_f64() * 1000.
                );
                checkpoint = std::time::Instant::now();
            }
        };
        let mut vertices = self.vertices.clone();
        self.face_parameters
            .apply_wrinkles(&mut vertices[..self.body_vertices]);
        if self.show_complexion && !self.show_skin && !self.show_strain {
            crate::female_complexion::apply(
                &mut vertices[..self.body_vertices],
                &self.normals[..self.body_vertices],
            );
        }
        crate::female_face::deform_with_contour(
            &mut vertices,
            self.body_vertices,
            self.face_pose(),
            self.lid_contour.as_ref(),
        );
        if self.secondary_only {
            if !self.animation_only {self.rig.deform_jump(&mut vertices, self.time);}
        } else {
            self.rig.deform(&mut vertices, self.time as f32);
        }
        for (vertex, rest) in vertices.iter_mut().zip(&self.vertices) {
            vertex.position[1] +=
                self.support_offset(rest.position.map(f64::from), self.time) as f32;
        }
        let displacement = if self.rig_pose_only {vec![[0.;3];self.body_vertices]} else {self
            .embedding
            .deform(
                self.skin.positions(),
                &self.targets,
                &vec![[0.; 3]; self.body_vertices],
            )
            .expect("validated finite shell and binding dimensions")};
        let mut normal_geometry = vertices.clone();
        self.render_body_parameters().apply(&mut normal_geometry);
        for (vertex, delta) in normal_geometry.iter_mut().zip(&displacement) {
            for k in 0..3 {
                vertex.position[k] += delta[k] as f32;
            }
        }
        if self.secondary_only && !self.rig_pose_only {
            for region in &self.regions {
                region.apply(
                    &self.vertices,
                    &mut normal_geometry,
                    self.root_bob(self.time),
                );
            }
        }
        mark("deformation");
        let mut normals = self
            .prepared_normals
            .transport(&normal_geometry, &self.normals);
        if std::env::var("VOXY_FACE_DIAGNOSTIC_GEOMETRIC_NORMALS").as_deref() == Ok("1") {
            let direct = self
                .prepared_normals
                .geometric(&normal_geometry, &self.normals);
            let differences: Vec<_> = self
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, v)| {
                    v.position[0].abs() < 0.05
                        && (0.748..0.785).contains(&v.position[1])
                        && v.position[2] > 0.11
                })
                .map(|(i, _)| normals[i].dot(direct[i]).clamp(-1., 1.).acos().to_degrees())
                .collect();
            eprintln!(
                "FOREHEAD NORMALS count={} mean_degrees={:.4} max_degrees={:.4}",
                differences.len(),
                differences.iter().sum::<f32>() / differences.len().max(1) as f32,
                differences.iter().copied().fold(0., f32::max)
            );
            normals = direct;
        }
        mark("normals");
        let key = crate::female_transmission::key_direction();
        let fill = Vec3::new(0.8, 0.2, 0.5).normalize();
        let surface_fraction = self
            .face_parameters
            .value("skin_surface_light")
            .unwrap_or(0.35);
        let surface_light = if self.surface_diffusion_enabled
            && !self.show_skin
            && !self.show_strain
            && surface_fraction < 1.
            && std::env::var("VOXY_FACE_DIAGNOSTIC_NO_DIFFUSION").as_deref() != Ok("1")
        {
            let points: Vec<_> = normal_geometry[..self.body_vertices]
                .iter()
                .map(|v| v.position)
                .collect();
            let triangles: Vec<_> = self
                .indices
                .chunks_exact(3)
                .filter(|t| t.iter().all(|i| (*i as usize) < self.body_vertices))
                .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
                .collect();
            let source: Vec<_> = (0..self.body_vertices)
                .map(|i| {
                    let n = normals[i].try_normalize().unwrap_or(self.normals[i]);
                    [f64::from(0.2 + 0.6 * n.dot(key).max(0.) + 0.2 * n.dot(fill).max(0.)); 3]
                })
                .collect();
            let mut guess = self.surface_light_guess.borrow_mut();
            let input = SurfaceLightInput {
                points,
                triangles,
                source,
            };
            let mut previous = self.surface_light_input.borrow_mut();
            let light = if previous.as_ref() == Some(&input) && guess.is_some() {
                guess.as_ref().unwrap().clone()
            } else {
                skin_light_diffusion::diffuse_with_initial(
                    &input.points,
                    &input.triangles,
                    &input.source,
                    [0.002, 0.001, 0.0005],
                    guess.as_deref(),
                )
                .map_err(|error| {
                    if trace {
                        eprintln!("face mesh diffusion rejected: {error}");
                    }
                    voxy_render::SceneError::InvalidGeometry
                })?
            };
            *previous = Some(input);
            *guess = Some(light.clone());
            Some(light)
        } else {
            None
        };
        mark("diffusion");
        let eye_pose = self.face_pose();
        let head = self.hair_head_matrix(self.time);
        for (i, v) in vertices.iter_mut().enumerate() {
            let n = normals[i].try_normalize().unwrap_or(self.normals[i]);
            let lighting = 0.2 + 0.6 * n.dot(key).max(0.0) + 0.2 * n.dot(fill).max(0.0);
            if i >= self.body_vertices {
                v.uv = crate::female_eyes::uv(Vec3::from_array(self.vertices[i].position));
                if crate::female_eyes::is_eye_uv(v.uv) {
                    // OBJ corner splits must not turn the globe into flat-shaded patches.
                    let bind = Vec3::from_array(self.vertices[i].position);
                    let radial = (bind - crate::female_eyes::center(bind.x)).normalize();
                    let n = head
                        .transform_vector3(eye_pose.eye_rotation(bind.x) * radial)
                        .normalize();
                    v.color = [0.5 + 0.5 * n.x, 0.5 + 0.5 * n.y, 0.5 + 0.5 * n.z, 1.];
                } else {
                    v.color = [0.78 * lighting, 0.79 * lighting, 0.75 * lighting, 1.];
                }
                continue;
            }
            if v.uv == [0.; 2]
                && (self.body_parameters.areola_radius_mm != 18.5
                    || self.body_parameters.areola_pigmentation != 1.
                    || self.body_parameters.left_areola_size != 1.
                    || self.body_parameters.right_areola_size != 1.)
            {
                // Untextured-skin UV metadata; shader decodes radius and strength.
                v.uv = [
                    2. + self
                        .body_parameters
                        .areola_radius_at(self.vertices[i].position[0])
                        / 1000.,
                    self.body_parameters.areola_pigmentation,
                ];
            }
            let base = [0.72, 0.46, 0.34];
            // Artistic two-depth approximation: retain a local surface response
            // alongside the diffused component instead of blurring all creases.
            // The convex blend keeps the same response on uniformly lit skin.
            let irradiance = surface_light.as_ref().map_or([lighting; 3], |light| {
                light[i].map(|v| surface_fraction * lighting + (1. - surface_fraction) * v as f32)
            });
            v.color = [
                base[0] * irradiance[0],
                base[1] * irradiance[1],
                base[2] * irradiance[2],
                1.0,
            ];
        }
        if self.show_skin {
            for b in self.embedding.bindings() {
                let ids = self.skin.triangles()[b.triangle];
                let displacement: [f64; 3] = std::array::from_fn(|axis| {
                    (0..3)
                        .map(|j| {
                            b.weights[j]
                                * (self.skin.positions()[ids[j]][axis] - self.targets[ids[j]][axis])
                        })
                        .sum()
                });
                let amount = (displacement.iter().map(|v| v * v).sum::<f64>().sqrt() / 0.01)
                    .clamp(0., 1.) as f32;
                vertices[b.vertex].color = [amount, 0.25 + 0.5 * (1. - amount), 1. - amount, 1.];
                vertices[b.vertex].uv = [-4., amount];
            }
        }
        if self.show_strain {
            let metrics = self
                .skin
                .surface_metrics()
                .map_err(|_| voxy_render::SceneError::InvalidGeometry)?;
            for binding in self.embedding.bindings() {
                let value = metrics[binding.triangle]
                    .principal_stretches
                    .iter()
                    .map(|stretch| (stretch - 1.).abs())
                    .fold(0., f64::max);
                let amount = (value / 0.1).clamp(0., 1.) as f32;
                vertices[binding.vertex].color =
                    [amount, 0.25 + 0.5 * (1. - amount), 1. - amount, 1.];
                vertices[binding.vertex].uv = [-4., amount];
            }
        }
        let mut indices = self.indices.clone();
        if std::env::var("VOXY_FACE_DIAGNOSTIC_NO_EYES").as_deref() == Ok("1") {
            indices = indices
                .chunks_exact(3)
                .filter(|ids| ids.iter().all(|&i| (i as usize) < self.body_vertices))
                .flatten()
                .copied()
                .collect();
        }
        mark("vertex_lighting");
        let posed_face = vertices[..self.body_vertices].to_vec();
        self.features.append_configured(
            &posed_face,
            &normals[..self.body_vertices],
            &mut vertices,
            &mut indices,
            self.hair_head_matrix(self.time),
            self.face_pose(),
            &self.face_parameters,
        );
        mark("face_features");
        let hair_start = vertices.len();
        let hair_index_start = indices.len();
        let mut hair_normals = Vec::new();
        if include_hair {
            self.hair.append_with_normals(&mut vertices, &mut indices, &mut hair_normals);
        }
        let hair_end = vertices.len();
        let hair_index_end = indices.len();
        if self.animation_only || !self.simulate_hair {
            let head = self.hair_physics_head_matrix(self.time);
            for vertex in &mut vertices[hair_start..] {
                vertex.position = head
                    .transform_point3(Vec3::from_array(vertex.position))
                    .to_array();
            }
            for normal in &mut hair_normals {
                *normal = head.transform_vector3(Vec3::from_array(*normal)).normalize_or_zero().to_array();
            }
        }
        self.face_parameters
            .apply_shape(&mut vertices, self.hair_head_matrix(self.time));
        self.render_body_parameters()
            .apply(&mut vertices[..hair_start]);
        for (vertex, delta) in vertices.iter_mut().zip(&displacement) {
            for k in 0..3 {
                vertex.position[k] += delta[k] as f32;
            }
        }
        if self.secondary_only && !self.rig_pose_only {
            for region in &self.regions {
                region.apply(&self.vertices, &mut vertices, self.root_bob(self.time));
            }
        }

        mark("hair_and_shapes");
        if let Some(film) = &self.film {
            film.apply(&mut vertices, &mut indices, self.eye());
        }
        // Keep GPU capacity and topology stable across interactive probe toggles.
        {
            let center = if self.probe_enabled || self.probe_depth > 0. {
                self.probe_center
            } else {
                [0., 0., 100.]
            }; // Clipped beyond the camera's far plane.
            let base = vertices.len() as u32;
            for row in 0..=12 {
                let phi = row as f32 * std::f32::consts::PI / 12.;
                for column in 0..=24 {
                    let theta = column as f32 * std::f32::consts::TAU / 24.;
                    let normal =
                        Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                    let position = Vec3::from_array(center.map(|v| v as f32)) + normal * 0.02;
                    let light = 0.3 + 0.7 * normal.dot(key).max(0.);
                    vertices.push(SceneVertex {
                        position: position.to_array(),
                        uv: [0.; 2],
                        color: [0.15 * light, 0.65 * light, light, 1.],
                    });
                }
            }
            for row in 0..12 {
                for column in 0..24 {
                    let a = base + row * 25 + column;
                    indices.extend_from_slice(&[a, a + 25, a + 1, a + 1, a + 25, a + 26]);
                }
            }
        }
        if let Some(object) = &self.grasp_mesh {
            for right in [false, true] {
                let matrix = self.rig.hand_matrix(self.time as f32, right)
                    * glam::Mat4::from_scale(Vec3::new(if right { -1. } else { 1. }, 1., 1.));
                let base = vertices.len() as u32;
                vertices.extend(object.vertices().iter().enumerate().map(|(i, v)| {
                    let light = 0.3
                        + 0.7
                            * matrix
                                .transform_vector3(self.grasp_normals[i])
                                .dot(key)
                                .max(0.);
                    SceneVertex {
                        position: matrix
                            .transform_point3(Vec3::from_array(v.position))
                            .to_array(),
                        color: [
                            v.color[0] * light,
                            v.color[1] * light,
                            v.color[2] * light,
                            v.color[3],
                        ],
                        ..*v
                    }
                }));
                for face in object.indices().chunks_exact(3) {
                    let ids = if right {
                        [face[0], face[2], face[1]]
                    } else {
                        [face[0], face[1], face[2]]
                    };
                    indices.extend(ids.map(|i| base + i));
                }
            }
        }
        mark("film_and_probe");
        crate::female_transmission::apply(
            &mut vertices,
            &indices,
            &self.vertices,
            self.body_vertices,
            self.hair_head_matrix(self.time),
            &mut self.transmission_cache.borrow_mut(),
        );
        mark("transmission");
        let mut coordinates = vec![[0.; 3]; vertices.len()];
        for (coordinate, vertex) in coordinates
            .iter_mut()
            .zip(&self.vertices[..self.body_vertices])
        {
            *coordinate = vertex.position;
        }
        // Eye-only metadata: pigment/relief annulus radius in metres.
        let pupil_radius = self
            .face_parameters
            .value("pupil_diameter_mm")
            .unwrap_or(4.05)
            * 0.0005;
        for (coordinate, vertex) in coordinates.iter_mut().zip(&vertices) {
            if crate::female_eyes::is_eye_uv(vertex.uv) {
                *coordinate = [0., 0., pupil_radius];
            }
        }
        // Hair radial normals come directly from the solved Cosserat frame.
        // Weld only the much smaller non-hair surfaces. Film retains its existing
        // complete geometric normal path because it can add overlapping layers.
        // Face morphs also retain it until their Jacobian transports strand normals.
        let authored_normals = if self.film.is_none() && include_hair
            && self.face_parameters == crate::face_parameters::FaceParameters::default()
        {
            let prefix = SceneMesh::new(vertices[..hair_start].to_vec(), indices[..hair_index_start].to_vec())?
                .with_prepared_upload_streams();
            let suffix_indices = indices[hair_index_end..].iter()
                .map(|i| i.checked_sub(hair_end as u32).ok_or(voxy_render::SceneError::InvalidGeometry))
                .collect::<Result<Vec<_>, _>>()?;
            let suffix = SceneMesh::new(vertices[hair_end..].to_vec(), suffix_indices)?
                .with_prepared_upload_streams();
            let mut combined = prefix.authored_normals().unwrap().to_vec();
            combined.append(&mut hair_normals);
            combined.extend_from_slice(suffix.authored_normals().unwrap());
            Some(combined)
        } else { None };
        let mesh = SceneMesh::new(vertices, indices)?.with_material_coordinates(coordinates)?;
        let mesh = if let Some(normals) = authored_normals { mesh.with_normals(normals)? } else { mesh };
        mark("mesh_validation");
        if let Some(film) = &self.film {
            mesh.with_material_parameters(film.optical_parameters())
        } else {
            Ok(mesh)
        }
    }
    pub(crate) fn verify(&self) -> Result<(), &'static str> {
        if self.steps < 8
            || self.max_displacement < 1e-6
            || self.embedding.bindings().len() != self.body_vertices
        {
            return Err("female skin smoke did not observe deformation");
        }
        if self.simulate_hair {
            self.hair.verify(self.hair_head_matrix(self.time))?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "exports actual lash centerlines and strict skin crossings"]
    fn audit_lash_skin_crossings() {
        let mut demo = super::FemaleDemo::new().unwrap();
        if let Ok(path) = std::env::var("VOXY_FACE_DIAGNOSTIC_PRESET") {
            demo.face_parameters = crate::face_parameters::FaceParameters::from_json(
                &std::fs::read_to_string(path).unwrap(),
            )
            .unwrap();
        }
        demo.animation_only = true;
        demo.show_hair = false;
        let mut frames = Vec::new();
        let tube_audit = std::env::var("VOXY_FACE_DIAGNOSTIC_LASH_TUBE").as_deref() == Ok("1");
        let closures: Vec<_> = if tube_audit {
            vec![0., 0.5, 1.]
        } else if std::env::var("VOXY_FACE_DIAGNOSTIC_LASH_CONTACT_TRACE").as_deref() == Ok("1") {
            vec![0.996, 0.997]
        } else if std::env::var("VOXY_FACE_DIAGNOSTIC_LASH_CLOSE_SWEEP").as_deref() == Ok("1") {
            (0..=50).map(|step| 0.95 + step as f32 * 0.001).collect()
        } else {
            (0..=20).map(|step| step as f32 / 20.).collect()
        };
        for closure in closures {
            if std::env::var("VOXY_FACE_DIAGNOSTIC_LASH_CONTACT_TRACE").as_deref() == Ok("1") {
                eprintln!("LASH CONTACT FRAME closure={closure}");
            }
            demo.preview_expression = Some(crate::female_face::FacePose {
                blink: closure,
                ..Default::default()
            });
            let mesh = demo.mesh().unwrap();
            let triangle_sources: Vec<[u32; 3]> = mesh
                .indices()
                .chunks_exact(3)
                .filter(|ids| {
                    ids.iter().all(|&i| (i as usize) < demo.body_vertices)
                        && ids.iter().any(|&i| {
                            let p = demo.vertices[i as usize].position;
                            p[0].abs() < 0.06 && (0.685..0.74).contains(&p[1]) && p[2] > 0.10
                        })
                })
                .map(|ids| [ids[0], ids[1], ids[2]])
                .collect();
            let triangles: Vec<_> = triangle_sources
                .iter()
                .map(|ids| {
                    std::array::from_fn::<_, 3, _>(|k| {
                        glam::DVec3::from_array(
                            mesh.vertices()[ids[k] as usize].position.map(f64::from),
                        )
                    })
                })
                .collect();
            let lashes: Vec<_> = mesh
                .vertices()
                .iter()
                .filter(|v| v.uv == [-0.25, 0.45])
                .collect();
            assert_eq!(lashes.len(), 144 * 102);
            let mut curves = Vec::new();
            let mut distal_crossings = 0;
            let mut root_crossings = 0;
            let mut tube_crossings = 0;
            for (index, strand) in lashes.chunks_exact(102).enumerate() {
                let centers: Vec<_> = strand
                    .chunks_exact(6)
                    .map(|ring| {
                        ring.iter()
                            .map(|v| glam::DVec3::from_array(v.position.map(f64::from)))
                            .sum::<glam::DVec3>()
                            / 6.
                    })
                    .collect();
                let mut hits = Vec::new();
                for (span, pair) in centers.windows(2).enumerate() {
                    let a = pair[0];
                    let b = pair[1];
                    if triangles.iter().any(|tri| {
                        (0..3).all(|k| {
                            a[k].max(b[k]) >= tri.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)
                                && a[k].min(b[k])
                                    <= tri.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max)
                        }) && crate::female_face::tests::segment_crosses_triangle(a, b, *tri)
                    }) {
                        hits.push(span);
                    }
                }
                root_crossings += usize::from(hits.contains(&0));
                distal_crossings += usize::from(hits.iter().any(|&span| span > 0));
                let mut tube_hits = Vec::new();
                if tube_audit {
                    for row in 0..17 {
                        let mut edges = Vec::new();
                        for side in 0..6 {
                            edges.push((row * 6 + side, row * 6 + (side + 1) % 6));
                            if row < 16 {
                                edges.push((row * 6 + side, (row + 1) * 6 + side));
                                edges.push((row * 6 + (side + 1) % 6, (row + 1) * 6 + side));
                            }
                        }
                        if edges.iter().any(|&(a,b)| {
                            let a=glam::DVec3::from_array(strand[a].position.map(f64::from));
                            let b=glam::DVec3::from_array(strand[b].position.map(f64::from));
                            let hit = triangles.iter().enumerate().find(|(_,tri)| (0..3).all(|k|
                                a[k].max(b[k]) >= tri.iter().map(|p|p[k]).fold(f64::INFINITY,f64::min)
                                && a[k].min(b[k]) <= tri.iter().map(|p|p[k]).fold(f64::NEG_INFINITY,f64::max))
                                && crate::female_face::tests::segment_crosses_triangle(a,b,**tri));
                            if let Some((triangle,_)) = hit {
                                let ids = triangle_sources[triangle];
                                let p=ids.map(|i|glam::Vec3::from_array(demo.vertices[i as usize].position));
                                let center=(p[0]+p[1]+p[2])/3.;
                                let normal=(p[1]-p[0]).cross(p[2]-p[0]);
                                let included=center.y>0.685 && center.y<0.74 && center.z>0.105
                                    && center.x.abs()<0.09 && normal.z>0.;
                                eprintln!("LASH TUBE TRI closure={closure} strand={index} row={row} source={ids:?} bind_center={center:?} bind_normal={normal:?} collider_filter={included}");
                                eprintln!("LASH TUBE DETAIL {}",serde_json::json!({
                                    "closure":closure,"strand":index,"row":row,"edge":[a.to_array(),b.to_array()],
                                    "triangle":triangles[triangle].map(|p|p.to_array()),
                                    "centers":centers.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
                                    "rings":strand.iter().map(|v|v.position).collect::<Vec<_>>()
                                }));
                            }
                            hit.is_some()
                        }) {tube_hits.push(row);}
                    }
                    tube_crossings += usize::from(!tube_hits.is_empty());
                }
                curves.push(serde_json::json!({"strand":index,"upper":index%72<48,
                    "points":centers.iter().map(|p|p.to_array()).collect::<Vec<_>>(),"crossingSpans":hits,"tubeCrossingSpans":tube_hits}));
            }
            eprintln!(
                "LASH SKIN closure={closure} root_crossing_strands={root_crossings} distal_crossing_strands={distal_crossings}"
            );
            if tube_audit {
                eprintln!("LASH TUBE closure={closure} crossing_strands={tube_crossings}");
            }
            frames.push(serde_json::json!({"closure":closure,"skinTriangles":triangles.len(),
                "rootCrossingStrands":root_crossings,"distalCrossingStrands":distal_crossings,"tubeCrossingStrands":tube_crossings,"curves":curves}));
        }
        std::fs::write(
            "/tmp/voxy-lash-skin-crossings.json",
            serde_json::to_string(&frames).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn surface_light_control_changes_irradiance_without_rebuilding_diffusion_or_geometry() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.show_hair = false;
        demo.preview_expression = Some(crate::female_face::FacePose {
            brow: 1.,
            ..Default::default()
        });
        demo.face_parameters = demo
            .face_parameters
            .patched(&serde_json::json!({"skin_surface_light":0.}))
            .unwrap();
        let scattered = demo.mesh().unwrap();
        let light = demo.surface_light_guess.borrow().clone().unwrap();
        let signature = demo.face_parameters.material_signature();
        demo.face_parameters = demo
            .face_parameters
            .patched(&serde_json::json!({"skin_surface_light":1.}))
            .unwrap();
        let local = demo.mesh().unwrap();
        assert_eq!(signature, demo.face_parameters.material_signature());
        assert_eq!(light, *demo.surface_light_guess.borrow().as_ref().unwrap());
        assert_eq!(scattered.indices(), local.indices());
        let mut changed = 0;
        for (a, b) in scattered.vertices().iter().zip(local.vertices()) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.uv, b.uv);
            if a.color != b.color {
                changed += 1;
            }
        }
        assert!(
            changed > 100,
            "surface-light control did not change skin irradiance"
        );
    }
    #[test]
    fn forehead_wrinkle_relief_follows_brow_pose_without_reapplying_mask() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.show_complexion = false;
        demo.preview_expression = Some(crate::female_face::FacePose::default());
        let parameters = crate::face_parameters::FaceParameters::default()
            .patched(&serde_json::json!({"wrinkles_forehead":0.8}))
            .unwrap();
        let neutral = demo.mesh().unwrap();
        demo.face_parameters = parameters.clone();
        let creased = demo.mesh().unwrap();
        let index = (0..demo.body_vertices)
            .filter(|&i| {
                let p = demo.vertices[i].position;
                p[1] > 0.750 && p[1] < 0.790 && p[0].abs() < 0.04
            })
            .max_by(|&a, &b| {
                let distance = |i: usize| {
                    glam::Vec3::from_array(neutral.vertices()[i].position)
                        .distance(glam::Vec3::from_array(creased.vertices()[i].position))
                };
                distance(a).total_cmp(&distance(b))
            })
            .unwrap();
        let depth = |a: &voxy_render::SceneMesh, b: &voxy_render::SceneMesh| {
            glam::Vec3::from_array(a.vertices()[index].position)
                .distance(glam::Vec3::from_array(b.vertices()[index].position))
        };
        let neutral_depth = depth(&neutral, &creased);
        assert!(neutral_depth > 0.0001);
        demo.face_parameters = Default::default();
        demo.preview_expression = Some(crate::female_face::FacePose {
            brow: 1.,
            ..Default::default()
        });
        let raised = demo.mesh().unwrap();
        assert!(
            depth(&neutral, &raised) > 0.0003,
            "test vertex did not move with the brow"
        );
        demo.face_parameters = parameters;
        let raised_creased = demo.mesh().unwrap();
        assert!(
            depth(&raised, &raised_creased) > neutral_depth * 0.8,
            "brow motion lost the wrinkle attached to its original vertex"
        );
        assert_eq!(raised.indices(), raised_creased.indices());
    }
    #[test]
    fn camera_gaze_converges_from_both_eye_centers() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.preview_expression = Some(Default::default());
        let head = demo.hair_head_matrix(demo.time);
        let local_target = glam::Vec3::new(0., 0.712416, 0.45);
        demo.preview_camera_eye = Some(head.transform_point3(local_target));
        let pose = demo.face_pose();
        for side in [-1., 1.] {
            let desired = (local_target - crate::female_eyes::center(side)).normalize();
            let actual = pose.eye_rotation(side) * glam::Vec3::Z;
            assert!(actual.distance(desired) < 0.0001);
        }
    }
    use super::*;
    #[test]
    fn grasp_preset_switch_preserves_pose_and_failed_load_keeps_target() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.animation_only = true;
        let cold = std::time::Instant::now();
        demo.prepare_grasp_presets().unwrap();
        let cold_ms = cold.elapsed().as_secs_f64() * 1000.;
        demo.set_grasp_target("cylinder").unwrap();
        demo.set_grasp_cycle(true);
        demo.preview_pose(1.75);
        let before = demo.mesh().unwrap();
        let light_before = demo
            .surface_light_input
            .borrow()
            .as_ref()
            .map(|v| (v.points.clone(), v.triangles.clone(), v.source.clone()));
        let original_distance = demo.distance;
        demo.set_hand_focus(true);
        demo.distance = 0.32;
        let focus = demo.focus();
        demo.zoom(-0.15);
        assert!((demo.distance - 0.29).abs() < 1e-6, "hand zoom jumps away");
        assert_eq!(demo.focus(), focus, "zoom moved the tracked hand focus");
        for _ in 0..20 {
            demo.zoom(-0.15);
        }
        assert!((demo.distance - 0.18).abs() < 1e-6);
        demo.set_hand_focus(false);
        demo.distance = original_distance;
        demo.preview_pose(0.0);
        let open = demo.mesh().unwrap();
        assert!(
            before.vertices().iter().zip(open.vertices()).any(|(a, b)| {
                Vec3::from_array(a.position).distance(Vec3::from_array(b.position)) > 0.005
            }),
            "grasp timeline does not change the rendered mesh"
        );
        demo.preview_pose(1.75);
        let hot = std::time::Instant::now();
        for index in 0..60 {
            demo.set_grasp_target(["cylinder", "sphere", "handle"][index % 3])
                .unwrap();
        }
        let hot_us = hot.elapsed().as_secs_f64() * 1e6 / 60.;
        demo.set_grasp_target("cylinder").unwrap();
        assert!(demo.set_grasp_target("\0").is_err());
        let after = demo.mesh().unwrap();
        if let (Some((p, t, s)), Some(now)) =
            (light_before, demo.surface_light_input.borrow().as_ref())
        {
            println!(
                "lighting inputs: points_equal={} triangles_equal={} source_equal={}",
                p == now.points,
                t == now.triangles,
                s == now.source
            );
        }
        assert_eq!(before.indices(), after.indices());
        assert_eq!(before.vertices().len(), after.vertices().len());
        for (index, (a, b)) in before.vertices().iter().zip(after.vertices()).enumerate() {
            assert_eq!(
                a.position, b.position,
                "reselecting target changed animation pose"
            );
            assert_eq!(
                a.color, b.color,
                "reselecting target changed its shading at vertex {index}"
            );
        }
        println!("preset preparation {cold_ms:.2} ms; cached selection mean {hot_us:.2} us");
        let cycling = demo.rig.grasp_amount(demo.time as f32);
        demo.adjust_grasp(-0.1).unwrap();
        let manual = (cycling - 0.1).clamp(0., 1.);
        assert!((demo.rig.grasp_amount(5.) - manual).abs() < 1e-6);
        assert_eq!(demo.rig.grasp_amount(5.), demo.skin_rig.grasp_amount(5.));
        assert!(demo.adjust_grasp(f32::NAN).is_err());
        assert!((demo.rig.grasp_amount(5.) - manual).abs() < 1e-6);
        demo.adjust_grasp(2.).unwrap();
        assert_eq!(demo.rig.grasp_amount(0.), 1.);
        demo.adjust_grasp(-2.).unwrap();
        assert_eq!(demo.rig.grasp_amount(5.), 0.);
        demo.set_grasp_cycle(true);
        assert!(demo.rig.grasp_amount(3.) > 0.99);
        assert_eq!(demo.rig.grasp_amount(0.), 0.);
    }

    #[test]
    fn full_character_follows_elapsed_frame_time() {
        for frames in [180, 120] {
            let mut demo = FemaleDemo::new().unwrap();
            demo.simulate_hair = true;
            for dt in [f64::NAN, f64::INFINITY, -0.01] {
                assert!(demo.advance(dt).is_err());
                assert_eq!(demo.time, 0.0);
                assert_eq!(demo.steps, 0);
            }
            for _ in 0..frames {
                demo.advance(6.0 / f64::from(u32::try_from(frames).unwrap()))
                    .unwrap();
                assert!(demo.min_area_ratio > 0.5);
                demo.verify()
                    .or_else(|error| if demo.steps < 8 { Ok(()) } else { Err(error) })
                    .unwrap();
            }
            assert!(
                (demo.time - 6.0).abs() < 1e-10,
                "timeline drift: {}",
                demo.time
            );
            assert_eq!(demo.steps, frames);
            assert!(demo.min_area_ratio > 0.5);
            println!(
                "FULL FRAME CADENCE: {frames} elapsed frames reached {:.3} simulated seconds",
                demo.time
            );
        }
    }

    #[test]
    fn physical_skin_preserves_forearm_surface() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.simulate_hair = false;
        demo.show_complexion = false;
        let mut minimum_area = f32::INFINITY;
        let mut maximum_stretch = 0f32;
        let mut minimum_orientation = 1f32;
        let mut worst = Vec3::ZERO;
        for step in 1..=120 {
            demo.advance(0.05).unwrap();
            if step % 5 != 0 {
                continue;
            }
            let rendered = demo.mesh().unwrap();
            let mut reference = demo.vertices.clone();
            demo.rig.deform(&mut reference, demo.time as f32);
            for triangle in demo.indices.chunks_exact(3) {
                if !triangle.iter().all(|&i| {
                    let p = Vec3::from_array(demo.vertices[i as usize].position);
                    p.x.abs() > 0.24 && (-0.12..0.32).contains(&p.y)
                }) {
                    continue;
                }
                let before = [triangle[0], triangle[1], triangle[2]]
                    .map(|i| Vec3::from_array(reference[i as usize].position));
                let after = [triangle[0], triangle[1], triangle[2]]
                    .map(|i| Vec3::from_array(rendered.vertices()[i as usize].position));
                let a = (before[1] - before[0]).cross(before[2] - before[0]);
                let b = (after[1] - after[0]).cross(after[2] - after[0]);
                if a.length() > 1e-10 {
                    minimum_area = minimum_area.min(b.length() / a.length());
                    minimum_orientation =
                        minimum_orientation.min(a.normalize().dot(b.normalize_or_zero()));
                }
                for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                    let ratio = after[i].distance(after[j]) / before[i].distance(before[j]);
                    if ratio > maximum_stretch {
                        maximum_stretch = ratio;
                        worst = before[i];
                    }
                }
            }
        }
        println!(
            "PHYSICAL FOREARM: area {minimum_area}, edge {maximum_stretch}, normal {minimum_orientation}, worst {worst:?}"
        );
        assert!(minimum_area > 0.5);
        assert!(maximum_stretch < 1.5);
        assert!(minimum_orientation > 0.0);
    }

    #[test]
    fn probe_contacts_actual_surface_and_retracts() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.simulate_hair = false;
        demo.pressing = false;
        let before = demo.mesh().unwrap();
        demo.probe_enabled = true;
        let mut maximum_force: f64 = 0.;
        for _ in 0..80 {
            demo.advance(1. / 120.).unwrap();
            maximum_force = maximum_force.max(demo.probe_force);
        }
        assert!(
            maximum_force > 1e-5,
            "probe did not contact: {maximum_force}"
        );
        assert!(demo.min_area_ratio > 0.5);
        let loaded = demo.mesh().unwrap();
        assert_eq!(before.indices(), loaded.indices());
        assert_eq!(before.vertices().len(), loaded.vertices().len());
        demo.probe_enabled = false;
        for _ in 0..80 {
            demo.advance(1. / 120.).unwrap();
        }
        assert_eq!(demo.probe_depth, 0.);
        assert_eq!(demo.probe_force, 0.);
        assert_eq!(demo.mesh().unwrap().indices(), before.indices());
        assert!(
            demo.skin
                .positions()
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
        println!("SKIN PROBE: peak axial reaction {maximum_force:.6} N; retracts without overlap");
    }
    #[test]
    fn complexion_follows_face_animation_and_leaves_eyes_untinted() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.preview_pose(0.);
        let rest = demo.mesh().unwrap();
        demo.preview_pose(3.);
        let posed = demo.mesh().unwrap();
        assert_eq!(rest.indices(), posed.indices());
        for (index, (a, b)) in rest.vertices().iter().zip(posed.vertices()).enumerate() {
            assert_eq!(
                a.uv, b.uv,
                "material coordinates changed at vertex {index}, position {:?} -> {:?}",
                a.position, b.position
            );
        }
        let mapped = posed.vertices()[..demo.body_vertices]
            .iter()
            .filter(|v| v.uv != [0.; 2])
            .count();
        assert!(mapped > 1000, "facial atlas missed the mesh: {mapped}");
        assert!(
            posed.vertices()[demo.body_vertices..demo.vertices.len()]
                .iter()
                .all(|v| v.uv == [0.; 2] || crate::female_eyes::is_eye_uv(v.uv))
        );
        demo.show_complexion = false;
        let bare = demo.mesh().unwrap();
        assert!(
            bare.vertices()[..demo.body_vertices]
                .iter()
                .all(|v| v.uv == [0.; 2])
        );
        assert!(
            posed.vertices()[demo.body_vertices..demo.vertices.len()]
                .iter()
                .zip(&bare.vertices()[demo.body_vertices..demo.vertices.len()])
                .all(|(a, b)| a.uv == b.uv)
        );
        assert!(
            posed.vertices()[demo.body_vertices..demo.vertices.len()]
                .iter()
                .zip(&bare.vertices()[demo.body_vertices..demo.vertices.len()])
                .all(|(a, b)| a.position == b.position)
        );
        let relieved = posed.vertices()[..demo.body_vertices]
            .iter()
            .zip(&bare.vertices()[..demo.body_vertices])
            .filter(|(a, b)| {
                Vec3::from_array(a.position).distance(Vec3::from_array(b.position)) > 1e-6
            })
            .count();
        assert!(
            relieved > 10,
            "blemish relief missed render vertices: {relieved}"
        );
    }
    #[test]
    fn animation_preview_uses_elapsed_time_without_skin_solver() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.animation_only = true;
        for _ in 0..60 {
            demo.advance(1.0 / 60.0).unwrap();
        }
        assert!((demo.time - 1.).abs() < 1e-9);
        assert_eq!(demo.steps, 0);
        assert_eq!(demo.skin.positions(), demo.skin.rest_positions());
    }
    #[test]
    fn ordinary_gesture_keeps_rendered_feet_at_their_support_height() {
        let mut demo = FemaleDemo::new().unwrap();
        let rest = demo.mesh().unwrap();
        let feet: Vec<_> = rest.vertices()[..demo.body_vertices]
            .iter()
            .enumerate()
            .filter(|(_, vertex)| vertex.position[1] < -0.74)
            .map(|(index, vertex)| (index, vertex.position))
            .collect();
        assert!(feet.len() > 100);
        for sample in 0..=24 {
            demo.preview_pose(sample as f64 * 0.25);
            let posed = demo.mesh().unwrap();
            for &(index, position) in &feet {
                assert!(
                    Vec3::from_array(position)
                        .distance(Vec3::from_array(posed.vertices()[index].position))
                        < 1e-6,
                    "foot drift at sample {sample}, vertex {index}"
                );
            }
        }
    }
    #[test]
    fn secondary_face_and_hair_colliders_share_the_rendered_head_pose() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        for time in [0.0, 0.5, 1.25, 2.75, 4.5] {
            let head = demo.hair_head_matrix(time);
            let posed = demo.hair_collider_pose(time);
            let bob = demo.root_bob(time) as f32;
            for point in [Vec3::new(-0.025, 0.646, 0.148), Vec3::new(0.025, 0.646, 0.148)] {
                assert!(head.transform_point3(point).distance(demo.rig.jump_head_matrix(time).transform_point3(point) + Vec3::Y*bob) < 1e-7);
            }
            for (rest, posed) in demo.vertices[..demo.body_vertices].iter().zip(&posed) {
                let point = Vec3::from_array(rest.position);
                if point.y > 0.62 {
                    assert!(Vec3::from_array(posed.position).distance(head.transform_point3(point)) < 1e-7);
                }
            }
        }
        demo.secondary_only = false;
        assert!(demo.hair_head_matrix(1.25).abs_diff_eq(
            glam::Mat4::from_translation(Vec3::Y*demo.root_bob(1.25) as f32)*demo.rig.head_matrix(1.25), 1e-7));
    }
    #[test]
    fn jump_excitation_matches_visible_root_and_stops_during_settling() {
        let mut demo = FemaleDemo::new().unwrap();
        // A stationary root must not receive an invisible jump force.
        assert_eq!(demo.root_motion(0.25), (0.0, 0.0));
        demo.secondary_only = true;
        let epsilon = 1e-5;
        for time in [0.07, 0.25, 0.43, 1.27, 3.73] {
            let (position, acceleration) = demo.root_motion(time);
            let numerical = (demo.root_bob(time + epsilon) - 2.0 * position
                + demo.root_bob(time - epsilon)) / (epsilon * epsilon);
            assert!((numerical - acceleration).abs() < 1e-5);
        }
        for time in [4.0, 4.5, 5.99] {
            assert_eq!(demo.root_motion(time), (0.0, 0.0));
        }
        demo.animation_only = true;
        assert_eq!(demo.root_motion(0.25), (0.0, 0.0));
    }
    #[test]
    #[ignore = "export full-model native systems for GPU qualification"]
    fn export_hair_linear_systems() {
        let mut demo = FemaleDemo::new().unwrap(); demo.secondary_only = true;
        let times: &[f64] = if std::env::var_os("VOXY_HAIR_QUALIFY_CYCLE").is_some() {
            &[1./120.,0.25,0.5,1.,2.,4.,5.95]
        } else { &[1./120.] };
        let mut rows = Vec::new();
        for &target in times {
            while demo.time + 1e-10 < target { demo.advance(1./120.).unwrap(); }
            let systems = demo.hair.qualification_systems(1./240.).unwrap();
            assert_eq!(systems.len(),469);
            rows.extend(systems.iter().map(|s| serde_json::json!({"simulation_seconds":demo.time,"band":s.band_width,"first":s.active.start,"end":s.active.end,"matrix":s.matrix,"rhs":s.rhs})));
        }
        let path=std::env::var("VOXY_HAIR_SYSTEM_FILE").expect("absolute qualification output path");
        assert!(std::path::Path::new(&path).is_absolute());
        std::fs::write(path,serde_json::to_vec(&rows).unwrap()).unwrap();
    }
    #[test]
    fn delayed_full_model_frame_matches_six_fixed_physics_steps() {
        let mut delayed=FemaleDemo::new().unwrap();
        let mut fixed=FemaleDemo::new().unwrap();
        delayed.secondary_only=true;fixed.secondary_only=true;
        let started = std::time::Instant::now();
        delayed.advance(0.05).unwrap();
        eprintln!("FULL SIX-STEP POSE wall_ms={:.3} skin_ms={:.3} hair_ms={:.3}",
            started.elapsed().as_secs_f64() * 1000., delayed.solver_ms[0], delayed.solver_ms[1]);
        let mut summed = [0.; 2];
        for _ in 0..6 {
            fixed.advance(1./120.).unwrap();
            for (total, elapsed) in summed.iter_mut().zip(fixed.solver_ms) { *total += elapsed; }
        }
        assert!(delayed.solver_ms.iter().all(|ms| ms.is_finite() && *ms > 0.));
        assert!(summed.iter().all(|ms| *ms > 0.));
        assert_eq!(delayed.steps,6);assert_eq!(delayed.time,fixed.time);
        assert_eq!(delayed.skin.positions(),fixed.skin.positions());
        assert_eq!(delayed.skin.velocities(),fixed.skin.velocities());
        let a=delayed.gpu_hair_surface_frames();let b=fixed.gpu_hair_surface_frames();
        assert_eq!(a.len(),b.len());
        for (a,b) in a.iter().zip(&b) {
            assert_eq!(a.position_arc,b.position_arc);assert_eq!(a.u_red,b.u_red);
            assert_eq!(a.v_green,b.v_green);assert_eq!(a.w_blue,b.w_blue);
        }
        delayed.hair.verify(delayed.hair_physics_head_matrix(delayed.time)).unwrap();
        delayed.advance(0.).unwrap();
        assert_eq!(delayed.solver_ms, [0.; 2], "a frame without substeps must not report stale solver costs");
    }
    #[test]
    fn gpu_body_stream_preserves_every_non_hair_vertex() {
        let mut demo=FemaleDemo::new().unwrap();demo.secondary_only=true;
        demo.advance(1./120.).unwrap();
        let full=demo.mesh().unwrap();let body=demo.mesh_without_hair().unwrap();
        let non_hair:Vec<_>=full.vertices().iter().filter(|v| v.uv[0]!=-7.).collect();
        assert_eq!(body.vertices().len(),non_hair.len());
        assert!(body.vertices().iter().all(|v| v.uv[0]!=-7.));
        assert!(body.vertices().iter().zip(non_hair).all(|(a,b)| a.position==b.position && a.uv==b.uv && a.color==b.color));
        assert_eq!(body.indices().len(),full.indices().chunks_exact(3).filter(|ids| full.vertices()[ids[0] as usize].uv[0]!=-7.).count()*3);
        assert!(demo.show_hair,"separate body publication must preserve the full model setting");
    }
    #[test]
    fn parallel_full_secondary_preserves_skin_and_hair_contacts() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = true;
        let before = demo.skin.positions().to_vec();
        for _ in 0..4 {
            demo.advance(1. / 120.).unwrap();
            demo.hair.verify(demo.hair_physics_head_matrix(demo.time)).unwrap();
            assert!(demo.solver_ms.iter().all(|ms| ms.is_finite() && *ms > 0.));
            eprintln!("FULL SECONDARY SOLVERS step={} skin_ms={:.3} hair_ms={:.3}",
                demo.steps, demo.solver_ms[0], demo.solver_ms[1]);
            assert!(demo.skin.positions().iter().flatten().all(|v| v.is_finite()));
        }
        assert!(demo.skin.positions().iter().zip(&before)
            .any(|(a, b)| (a[1] - b[1]).abs() > 1e-6));
        assert_eq!(demo.steps, 4);
    }
    #[test]
    fn full_secondary_demo_advances_skin_and_all_volume_regions() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false; // This regression isolates the skin and volume owners.
        assert_eq!(demo.regions.len(), 5, "paired chest/hip cages plus abdomen");
        let before = demo.skin.positions().to_vec();
        for _ in 0..4 {
            demo.advance(1. / 120.).unwrap();
        }
        let mut abdomen_surface = demo.vertices.clone();
        demo.regions[4].apply(&demo.vertices, &mut abdomen_surface, demo.root_bob(demo.time));
        assert!(abdomen_surface.iter().zip(&demo.vertices).any(|(posed, rest)| {
            let p = Vec3::from_array(rest.position);
            p.x.abs() < 0.10 && (0.0..0.25).contains(&p.y) && p.z > 0.04
                && Vec3::from_array(posed.position).distance(p) > 1e-6
        }), "the abdomen cage must move the visible abdominal surface");
        assert!(
            demo.skin
                .positions()
                .iter()
                .zip(&before)
                .any(|(a, b)| (a[1] - b[1]).abs() > 1e-6)
        );
        assert!(
            demo.regions
                .iter()
                .all(|r| r.maximum_local_displacement(demo.root_bob(demo.time)) > 1e-6)
        );
        assert!(
            demo.skin
                .positions()
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }
    #[test]
    fn preview_has_visible_soft_region_motion_and_settles() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false;
        let mut peak = [0.0_f64; 4];
        let mut rendered_peak = [0.0_f64; 4];
        for frame in 0..696 {
            demo.advance(1.0 / 120.0).unwrap();
            if frame % 12 == 0 {
                let mesh = demo.mesh().unwrap();
                for (v, r) in mesh
                    .vertices()
                    .iter()
                    .zip(&demo.vertices)
                    .take(demo.body_vertices)
                {
                    for i in 0..4 {
                        let center = [
                            if i % 2 == 0 { -0.10 } else { 0.10 },
                            if i < 2 { 0.36 } else { -0.10 },
                            if i < 2 { 0.10 } else { -0.11 },
                        ];
                        let d: f64 = (0..3)
                            .map(|k| ((f64::from(r.position[k]) - center[k]) / 0.11).powi(2))
                            .sum();
                        if d < 1.0 {
                            rendered_peak[i] = rendered_peak[i].max(
                                (f64::from(v.position[1] - r.position[1])
                                    - demo.root_bob(demo.time))
                                .abs(),
                            );
                        }
                    }
                }

                for (i, state) in demo.secondary.iter().enumerate() {
                    peak[i] = peak[i].max(state.offset()[1].abs());
                }
            }
        }
        eprintln!("region support peak mm: {:?}", peak.map(|x| x * 1000.0));
        assert!(peak.iter().all(|x| *x > 0.01));
        eprintln!("rendered peak mm: {:?}", rendered_peak.map(|x| x * 1000.0));
        assert!(rendered_peak.iter().all(|x| *x > 0.005));
        assert!(demo.secondary.iter().all(|s| s.velocity()[1].abs() < 0.003));
    }
    #[test]
    fn secondary_motion_changes_actual_model_vertices() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.simulate_hair = false;
        demo.secondary_only = true;
        for _ in 0..12 {
            demo.advance(1.0 / 120.0).unwrap();
        }
        let moving = demo.mesh().unwrap();
        let states = demo.secondary.clone();
        assert!(states.iter().all(|s| s.offset()[1].abs() > 1e-5));
        for state in &mut demo.secondary {
            state.reset();
        }
        let reference = demo.mesh().unwrap();
        let changed = moving
            .vertices()
            .iter()
            .zip(reference.vertices())
            .filter(|(a, b)| (a.position[1] - b.position[1]).abs() > 1e-5)
            .count();
        assert!(
            changed > 100,
            "secondary motion did not reach model: {changed}"
        );
    }
    #[test]
    fn real_mesh_loads_and_skin_deforms_rendered_surface() {
        let mut demo = FemaleDemo::new().unwrap();
        demo.simulate_hair = false;
        let before = demo.mesh().unwrap();
        for _ in 0..30 {
            demo.advance(1.0 / 60.0).unwrap();
        }
        demo.verify().unwrap();
        let after = demo.mesh().unwrap();
        assert_eq!(before.indices(), after.indices());
        assert_eq!(demo.embedding.bindings().len(), demo.body_vertices);
        assert!(demo.min_area_ratio > 0.5);
        let transfer = demo
            .embedding
            .deform(
                demo.skin.positions(),
                &demo.targets,
                &vec![[0.; 3]; demo.body_vertices],
            )
            .unwrap();
        for (name, predicate) in [("head", 0), ("arms", 1), ("torso", 2), ("legs", 3)] {
            let count = demo.vertices[..demo.body_vertices]
                .iter()
                .zip(&transfer)
                .filter(|(v, d)| {
                    let p = v.position;
                    let region = match predicate {
                        0 => p[1] > 0.65,
                        1 => p[0].abs() > 0.25,
                        2 => p[1] > -0.1 && p[1] < 0.45 && p[0].abs() < 0.2,
                        _ => p[1] < -0.25,
                    };
                    region && d.iter().any(|v| v.abs() > 1e-6)
                })
                .count();
            assert!(
                count > 10,
                "skin displacement did not cover {name}: {count}"
            );
        }
        assert!(
            before
                .vertices()
                .iter()
                .zip(after.vertices())
                .any(|(a, b)| (a.position[2] - b.position[2]).abs() > 1e-6)
        );
        assert!(
            demo.skin
                .thicknesses()
                .unwrap()
                .iter()
                .all(|v| v.is_finite() && *v > 0.0)
        );
    }
}

#[cfg(test)]
mod morphology_tests {
    #[test]
    fn face_live_reload_keeps_last_valid_preset() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        let path =
            std::env::temp_dir().join(format!("voxy-face-reload-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"nose_width":1.2}"#).unwrap();
        demo.face_parameter_file = Some(path.clone());
        demo.advance(0.).unwrap();
        assert_eq!(demo.face_parameters.value("nose_width"), Some(1.2));
        let good = demo.face_parameters.clone();
        std::fs::write(&path, r#"{"eyes_width":9}"#).unwrap();
        demo.face_parameter_revision = None;
        demo.advance(0.).unwrap();
        assert_eq!(demo.face_parameters, good);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn face_constructor_preserves_animated_topology_and_attachment_density() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.preview_pose(0.9);
        let base = demo.mesh().unwrap();
        demo.face_parameters=crate::face_parameters::FaceParameters::default().patched(&serde_json::json!({"nose_width":1.3,"eyes_spacing":2.,"lips_height":1.2,"lashes_length":1.8,"brow_hair_density":0.})).unwrap();
        let changed = demo.mesh().unwrap();
        assert_eq!(base.indices(), changed.indices());
        assert_eq!(base.vertices().len(), changed.vertices().len());
        assert!(
            base.vertices()
                .iter()
                .zip(changed.vertices())
                .filter(|(a, b)| glam::Vec3::from_array(a.position)
                    .distance(glam::Vec3::from_array(b.position))
                    > 0.0001)
                .count()
                > 100
        );
        assert!(
            changed
                .vertices()
                .iter()
                .filter(|v| v.color[3] == 0.)
                .count()
                > base.vertices().iter().filter(|v| v.color[3] == 0.).count()
        );
    }

    #[test]
    fn parameters_preserve_real_model_topology_and_motion() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false;
        let baseline = demo.mesh().unwrap();
        demo.body_parameters = crate::body_parameters::BodyParameters::from_json(include_str!(
            "../../../assets/characters/blender-female/presets/tall.json"
        ))
        .unwrap();
        let changed = demo.mesh().unwrap();
        assert_eq!(baseline.indices(), changed.indices());
        assert_eq!(baseline.vertices().len(), changed.vertices().len());
        assert!(
            baseline
                .vertices()
                .iter()
                .zip(changed.vertices())
                .any(|(a, b)| (a.position[1] - b.position[1]).abs() > 0.1)
        );
        for _ in 0..120 {
            demo.advance(1. / 120.).unwrap();
        }
        let moving = demo.mesh().unwrap();
        assert!(
            moving
                .vertices()
                .iter()
                .all(|v| v.position.iter().all(|p| p.is_finite()))
        );
        assert!(
            changed
                .vertices()
                .iter()
                .zip(moving.vertices())
                .any(|(a, b)| (a.position[1] - b.position[1]).abs() > 0.005)
        );
    }
}

#[cfg(test)]
mod parameter_reload_tests {
    #[test]
    fn watched_file_updates_and_invalid_json_keeps_last_body() {
        let path = std::env::temp_dir().join(format!("voxy-reload-{}.json", std::process::id()));
        std::fs::write(&path, "{\"height_cm\":182,\"breast_size\":1.3}").unwrap();
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false;
        demo.parameter_file = Some(path.clone());
        demo.advance(1. / 120.).unwrap();
        assert_eq!(demo.body_parameters.height_cm, 182.);
        let valid = demo.body_parameters;
        std::fs::write(&path, "{\"weight_kg\":0}").unwrap();
        demo.parameter_revision = None;
        demo.advance(1. / 120.).unwrap();
        assert_eq!(demo.body_parameters, valid);
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod film_tests {
    #[test]
    fn actual_model_accepts_film_and_advances_a_frame() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false;
        demo.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        demo.advance(1. / 120.).unwrap();
        assert!(
            demo.mesh()
                .unwrap()
                .vertices()
                .iter()
                .all(|v| v.color.iter().all(|c| c.is_finite()))
        );
    }
}

#[cfg(test)]
mod parameter_skin_rebase_tests {
    #[test]
    fn pupil_parameter_changes_only_eye_metadata_not_geometry_or_skin() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.simulate_hair = false;
        let small = demo
            .face_parameters
            .patched(&serde_json::json!({"pupil_diameter_mm":2.}))
            .unwrap();
        demo.face_parameters = small;
        let a = demo.mesh().unwrap();
        demo.face_parameters = demo
            .face_parameters
            .patched(&serde_json::json!({"pupil_diameter_mm":8.}))
            .unwrap();
        let b = demo.mesh().unwrap();
        assert_eq!(a.vertices(), b.vertices());
        let mut eyes = 0;
        for ((vertex, x), y) in a
            .vertices()
            .iter()
            .zip(a.explicit_material_coordinates().unwrap())
            .zip(b.explicit_material_coordinates().unwrap())
        {
            if crate::female_eyes::is_eye_uv(vertex.uv) {
                eyes += 1;
                assert!((x[2] - 0.001).abs() < 1e-7 && (y[2] - 0.004).abs() < 1e-7);
            } else {
                assert_eq!(x, y);
            }
        }
        assert!(eyes > 100);
        assert!(
            demo.face_parameters
                .patched(&serde_json::json!({"pupil_diameter_mm":9.}))
                .is_err()
        );
    }
    #[test]
    fn skin_material_coordinates_stay_attached_across_morph_and_animation() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.simulate_hair = false;
        let reference = demo.mesh().unwrap();
        let count = demo.body_vertices;
        let coordinates = reference.explicit_material_coordinates().unwrap()[..count].to_vec();
        let positions: Vec<_> = reference.vertices()[..count]
            .iter()
            .map(|v| v.position)
            .collect();
        let mut parameters = demo.body_parameters;
        parameters.height_cm = 190.;
        parameters.torso_depth = 1.3;
        demo.set_body_parameters(parameters).unwrap();
        demo.advance(0.5).unwrap();
        let changed = demo.mesh().unwrap();
        assert_eq!(
            &changed.explicit_material_coordinates().unwrap()[..count],
            coordinates.as_slice()
        );
        let maximum_displacement = changed.vertices()[..count]
            .iter()
            .zip(&positions)
            .map(|(v, p)| {
                (0..3)
                    .map(|k| f64::from(v.position[k] - p[k]).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
            .fold(0.0_f64, f64::max);
        assert!(
            maximum_displacement > 0.1,
            "fixture must actually deform geometry"
        );
        demo.set_body_parameters(Default::default()).unwrap();
        demo.advance(0.5).unwrap();
        let restored = demo.mesh().unwrap();
        assert_eq!(
            &restored.explicit_material_coordinates().unwrap()[..count],
            coordinates.as_slice()
        );
    }

    #[test]
    fn parameter_edit_rebases_shell_without_accumulating_morphs() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false;
        let original_rest = demo.skin.rest_positions().to_vec();
        let original_mass: f64 = demo.skin.masses().iter().sum();
        let original_attachments = demo.attachments.clone();
        let mut parameters = demo.body_parameters;
        parameters.height_cm = 190.;
        parameters.torso_depth = 1.3;
        demo.set_body_parameters(parameters).unwrap();
        assert!(demo.skin.masses().iter().sum::<f64>() > original_mass);
        let changed_rest = demo.skin.rest_positions().to_vec();
        assert_eq!(demo.skin.positions(), demo.targets);
        demo.set_body_parameters(parameters).unwrap();
        assert_eq!(demo.skin.rest_positions(), changed_rest);
        demo.set_body_parameters(Default::default()).unwrap();
        assert_eq!(demo.skin.rest_positions(), original_rest);
        assert!((demo.skin.masses().iter().sum::<f64>() - original_mass).abs() < 1e-10);
        for (a, b) in demo.attachments.iter().zip(original_attachments) {
            assert!((a.stiffness - b.stiffness).abs() < 1e-8);
            assert!((a.viscosity - b.viscosity).abs() < 1e-8);
        }
        let before = demo.skin.rest_positions().to_vec();
        parameters.height_cm = f32::NAN;
        assert!(demo.set_body_parameters(parameters).is_err());
        assert_eq!(demo.skin.rest_positions(), before);
        parameters.height_cm = 190.;
        demo.secondary_only = false;
        demo.set_body_parameters(parameters).unwrap();
        demo.advance(1. / 240.).unwrap();
        assert!(
            demo.skin
                .positions()
                .iter()
                .flatten()
                .all(|x| x.is_finite())
        );
        assert!(
            demo.mesh()
                .unwrap()
                .vertices()
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite()))
        );
    }
}

#[cfg(test)]
mod parameterized_hair_tests {
    #[test]
    fn edited_groom_and_collider_advance_in_the_same_space() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        let mut parameters = demo.body_parameters;
        parameters.height_cm = 182.;
        parameters.head_size = 1.2;
        demo.set_body_parameters(parameters).unwrap();
        let posed = demo.hair_collider_pose(0.);
        assert!(
            posed[..demo.body_vertices]
                .iter()
                .zip(&demo.vertices)
                .any(|(a, b)| (a.position[1] - b.position[1]).abs() > 0.02)
        );
        demo.advance(1. / 240.).unwrap();
        demo.hair
            .verify(demo.hair_physics_head_matrix(demo.time))
            .unwrap();
        assert!(
            demo.mesh()
                .unwrap()
                .vertices()
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite()))
        );
    }
}

#[cfg(test)]
mod nipple_refinement_tests {
    #[test]
    fn response_advances_with_animation_time_and_survives_target_changes() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.surface_diffusion_enabled = false;
        let target = demo
            .body_parameters
            .patched(&serde_json::json!({"nipple_type":"projecting","nipple_cold_response":1.}))
            .unwrap();
        demo.set_body_parameters(target).unwrap();
        demo.enable_cold_response(0., 2., 8.).unwrap();
        let before = demo.mesh().unwrap();
        let shell = demo.skin.positions().to_vec();
        let bindings = demo.embedding.bindings().len();
        demo.advance(2.).unwrap();
        assert!((demo.cold_response.unwrap().response() - (1. - (-1_f64).exp())).abs() < 1e-12);
        assert_eq!(demo.skin.positions(), shell);
        assert_eq!(demo.embedding.bindings().len(), bindings);
        // Compare at the same animation pose to isolate response geometry.
        demo.preview_pose(0.);
        let after = demo.mesh().unwrap();
        assert!(
            before.vertices()[..demo.body_vertices]
                .iter()
                .zip(&after.vertices()[..demo.body_vertices])
                .filter(|(a, b)| (a.position[2] - b.position[2]).abs() > 0.0001)
                .count()
                > 30
        );
        let state = demo.cold_response.unwrap();
        assert!(demo.enable_cold_response(0., 0., 8.).is_err());
        assert_eq!(demo.cold_response, Some(state));
        assert!(demo.advance(f64::NAN).is_err());
        assert_eq!(demo.cold_response, Some(state));
        demo.set_body_parameters(
            target
                .patched(&serde_json::json!({"nipple_cold_response":0.}))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(demo.cold_response, Some(state));
        demo.advance(8.).unwrap();
        assert!(
            (demo.cold_response.unwrap().response() - state.response() * (-1_f64).exp()).abs()
                < 1e-12
        );
    }
    #[test]
    fn response_uses_integrated_solver_time_instead_of_unconsumed_frame_time() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false;
        demo.set_body_parameters(
            demo.body_parameters
                .patched(&serde_json::json!({"nipple_cold_response":1.}))
                .unwrap(),
        )
        .unwrap();
        demo.enable_cold_response(0., 2., 8.).unwrap();
        demo.advance(0.001).unwrap();
        assert_eq!(demo.time, 0.);
        assert_eq!(demo.cold_response.unwrap().response(), 0.);
        demo.advance(0.1).unwrap();
        assert!(demo.time > 0. && demo.time <= 0.05);
        assert!(
            (demo.cold_response.unwrap().response() - (1. - (-demo.time / 2.).exp())).abs() < 1e-12
        );
    }
    #[test]
    fn refined_landmarks_have_complete_bindings_and_visible_cold_deformation() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.surface_diffusion_enabled = false;
        demo.simulate_hair = false;
        demo.animation_only = true;
        assert_eq!(demo.body_vertices, 60_866);
        assert_eq!(demo.embedding.bindings().len(), demo.body_vertices);
        let neutral = demo
            .body_parameters
            .patched(&serde_json::json!({"nipple_type":"projecting"}))
            .unwrap();
        demo.set_body_parameters(neutral).unwrap();
        let warm = demo.mesh().unwrap();
        let cold = neutral
            .patched(&serde_json::json!({"nipple_cold_response":1.}))
            .unwrap();
        demo.set_body_parameters(cold).unwrap();
        let chilled = demo.mesh().unwrap();
        let changed = warm.vertices()[..demo.body_vertices]
            .iter()
            .zip(&chilled.vertices()[..demo.body_vertices])
            .filter(|(a, b)| (a.position[2] - b.position[2]).abs() > 0.0001)
            .count();
        assert!(
            changed > 30,
            "too few vertices resolve the nipple response: {changed}"
        );
        assert!(
            chilled
                .vertices()
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite()))
        );
        let render: Vec<_> = demo.vertices[..demo.body_vertices]
            .iter()
            .map(|v| v.position.map(f64::from))
            .collect();
        let unchanged = demo
            .embedding
            .deform(
                demo.skin.rest_positions(),
                demo.skin.rest_positions(),
                &render,
            )
            .unwrap();
        assert_eq!(unchanged, render);
    }
}

#[cfg(test)]
mod film_state_capture_tests {
    #[test]
    fn moving_body_film_conserves_source_adjusted_mass_over_two_seconds() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = true;
        demo.simulate_hair = false;
        demo.surface_diffusion_enabled = false;
        demo.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        let settings = crate::film_settings::FilmSettings::default()
            .patched(&serde_json::json!({"source_rate_m3_s":1e-9,"self_contact_enabled":true}))
            .unwrap();
        demo.film.as_mut().unwrap().configure(settings).unwrap();
        let mut maximum_error = 0_f64;
        for _ in 0..120 {
            demo.advance(1. / 60.).unwrap();
            maximum_error = maximum_error.max(
                demo.film
                    .as_ref()
                    .unwrap()
                    .verify_mass_balance(1e-9)
                    .unwrap(),
            );
        }
        assert!((demo.time - 2.).abs() < 1e-10);
        assert!(demo.max_displacement > 1e-6);
        let mesh = demo.mesh().unwrap();
        let state = demo.film.as_ref().unwrap().state_snapshot();
        let representatives: Vec<usize> =
            serde_json::from_value(state["representatives"].clone()).unwrap();
        let points: Vec<[f64; 3]> =
            serde_json::from_value(state["physics"]["pointsM"].clone()).unwrap();
        let triangles: Vec<[usize; 3]> =
            serde_json::from_value(state["physics"]["triangles"].clone()).unwrap();
        for index in triangles
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
        {
            assert_eq!(
                points[index],
                mesh.vertices()[representatives[index]]
                    .position
                    .map(f64::from)
            );
        }
        assert!(
            (state["measurements"]["sourceAddedMassKg"].as_f64().unwrap() - 2e-6).abs() < 1e-15
        );
        println!(
            "MOVING FILM MASS: 120 frames, time={} s, max relative error={maximum_error:.9e}, contact transfer={} m3",
            demo.time, state["measurements"]["selfContact"]["grossTransferredVolumeM3"]
        );
    }
    fn check_physical_film_clock(secondary_only: bool) {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.secondary_only = secondary_only;
        demo.simulate_hair = false;
        demo.surface_diffusion_enabled = false;
        demo.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        let settings = crate::film_settings::FilmSettings::default()
            .patched(&serde_json::json!({"source_rate_m3_s":1e-9}))
            .unwrap();
        demo.film.as_mut().unwrap().configure(settings).unwrap();
        let before = demo.film.as_ref().unwrap().state_snapshot();
        demo.advance(0.001).unwrap();
        assert_eq!(demo.time, 0.);
        assert_eq!(demo.film.as_ref().unwrap().state_snapshot(), before);
        demo.advance(0.1).unwrap();
        assert_eq!(demo.time, 0.05);
        let mesh = demo.mesh().unwrap();
        let state = demo.film.as_ref().unwrap().state_snapshot();
        let representatives: Vec<usize> =
            serde_json::from_value(state["representatives"].clone()).unwrap();
        let points: Vec<[f64; 3]> =
            serde_json::from_value(state["physics"]["pointsM"].clone()).unwrap();
        let triangles: Vec<[usize; 3]> =
            serde_json::from_value(state["physics"]["triangles"].clone()).unwrap();
        for index in triangles
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
        {
            assert_eq!(
                points[index],
                mesh.vertices()[representatives[index]]
                    .position
                    .map(f64::from)
            );
        }
        let initial = before["measurements"]["massKg"].as_f64().unwrap();
        let actual = state["measurements"]["massKg"].as_f64().unwrap();
        let density = state["physics"]["material"]["density"].as_f64().unwrap();
        assert!((actual - initial - demo.time * 1e-9 * density).abs() < initial * 1e-12);
    }
    #[test]
    fn physical_film_uses_consumed_solver_time_and_current_surface() {
        check_physical_film_clock(true);
    }
    #[test]
    fn full_skin_solver_film_uses_consumed_time_and_current_surface() {
        check_physical_film_clock(false);
    }
    #[test]
    fn animated_film_uses_current_pose_and_rejects_truncated_time() {
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.simulate_hair = false;
        demo.surface_diffusion_enabled = false;
        demo.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        let settings = crate::film_settings::FilmSettings::default()
            .patched(&serde_json::json!({"source_rate_m3_s":1e-9}))
            .unwrap();
        demo.film.as_mut().unwrap().configure(settings).unwrap();
        let before = demo.film.as_ref().unwrap().state_snapshot();
        demo.advance(0.05).unwrap();
        let mesh = demo.mesh().unwrap();
        let state = demo.film.as_ref().unwrap().state_snapshot();
        let representatives: Vec<usize> =
            serde_json::from_value(state["representatives"].clone()).unwrap();
        let points: Vec<[f64; 3]> =
            serde_json::from_value(state["physics"]["pointsM"].clone()).unwrap();
        let previous: Vec<[f64; 3]> =
            serde_json::from_value(before["physics"]["pointsM"].clone()).unwrap();
        assert!(
            points
                .iter()
                .zip(previous)
                .any(|(a, b)| a.iter().zip(b).any(|(x, y)| (x - y).abs() > 1e-5))
        );
        let triangles: Vec<[usize; 3]> =
            serde_json::from_value(state["physics"]["triangles"].clone()).unwrap();
        let used: std::collections::BTreeSet<_> = triangles.iter().flatten().copied().collect();
        assert_eq!(points.len(), representatives.len());
        // Numerical snapshots store coordinates for referenced film vertices;
        // unused OBJ slots are zero and do not belong to the substrate.
        for index in used {
            assert_eq!(
                points[index],
                mesh.vertices()[representatives[index]]
                    .position
                    .map(f64::from)
            );
        }
        let initial = before["measurements"]["massKg"].as_f64().unwrap();
        let actual = state["measurements"]["massKg"].as_f64().unwrap();
        let density = state["physics"]["material"]["density"].as_f64().unwrap();
        assert!((actual - initial - 0.05 * 1e-9 * density).abs() < initial * 1e-12);
        let time = demo.time;
        let animation_time = demo.animation_time;
        let cold = demo.cold_response;
        assert!(demo.advance(0.101).is_err());
        assert_eq!(demo.time, time);
        assert_eq!(demo.animation_time, animation_time);
        assert_eq!(demo.cold_response, cold);
        assert_eq!(demo.film.as_ref().unwrap().state_snapshot(), state);
    }
    #[test]
    fn viewer_callback_exports_actual_film_once_for_the_requested_frame() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-viewer-film-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("body.json");
        let mut demo = super::FemaleDemo::new().unwrap();
        demo.animation_only = true;
        demo.simulate_hair = false;
        demo.surface_diffusion_enabled = false;
        std::fs::write(&path, demo.body_parameters.to_json().to_string()).unwrap();
        demo.parameter_file = Some(path.clone());
        demo.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        demo.advance(0.01).unwrap();
        let mass = demo.film.as_ref().unwrap().measurements()["massKg"]
            .as_f64()
            .unwrap();
        let request = crate::model_presentation::request_film_state(&path).unwrap();
        let id = request["requestId"].as_str().unwrap();
        demo.record_presentation(42, [800, 600]);
        let result = crate::model_presentation::film_state_status(&path, id).unwrap();
        assert_eq!(result["status"], "ready");
        assert_eq!(result["frame"], 42);
        assert_eq!(result["filmEnabled"], true);
        let snapshot_path = std::path::Path::new(result["path"].as_str().unwrap());
        let bytes = std::fs::read(snapshot_path).unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            saved["state"]["measurements"]["massKg"].as_f64().unwrap(),
            mass
        );
        assert_eq!(saved["state"]["format"], "voxy.surface-film-state.v1");
        demo.record_presentation(43, [800, 600]);
        assert_eq!(std::fs::read(snapshot_path).unwrap(), bytes);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(test)]
mod male_asset_tests {
    #[test]
    fn male_source_has_complete_bindings_and_advances() {
        let mut male = super::FemaleDemo::new_male().unwrap();
        assert_eq!(male.body_vertices, 44_880);
        assert_eq!(male.embedding.bindings().len(), male.body_vertices);
        assert!(!male.show_hair);
        male.animation_only = true;
        male.surface_diffusion_enabled = false;
        let parameters=male.body_parameters.patched(&serde_json::json!({"nipple_type":"projecting","nipple_radius_mm":4.,"nipple_projection_mm":2.,"nipple_cold_response":1.,"areola_radius_mm":10.})).unwrap();
        male.set_body_parameters(parameters).unwrap();
        male.advance(1. / 120.).unwrap();
        assert!(
            male.mesh()
                .unwrap()
                .vertices()
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite()))
        );
    }
}

#[cfg(test)]
mod model_selection_tests {
    #[test]
    fn source_switch_rebuilds_bindings_and_preserves_view_controls() {
        let mut model = super::FemaleDemo::new().unwrap();
        model.show_strain = true;
        model.animation_only = true;
        model.yaw = 0.7;
        let male = model
            .body_parameters
            .patched(&serde_json::json!({"body_model":"male"}))
            .unwrap();
        model.set_body_parameters(male).unwrap();
        assert_eq!(model.body_vertices, 44_880);
        assert_eq!(model.embedding.bindings().len(), 44_880);
        assert!(model.show_strain && model.animation_only);
        assert_eq!(model.yaw, 0.7);
        let female = model
            .body_parameters
            .patched(&serde_json::json!({"body_model":"female"}))
            .unwrap();
        model.set_body_parameters(female).unwrap();
        assert_eq!(model.body_vertices, 60_866);
        assert_eq!(model.embedding.bindings().len(), 60_866);
        assert!(
            model
                .body_parameters
                .patched(&serde_json::json!({"body_model":"unknown"}))
                .is_err()
        );
    }
    #[test]
    fn source_switch_transfers_existing_fluid_without_mass_loss() {
        let mut model = super::FemaleDemo::new().unwrap();
        model.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        let before = model.body_parameters;
        let male = before
            .patched(&serde_json::json!({"body_model":"male"}))
            .unwrap();
        model.animation_only = true;
        model.surface_diffusion_enabled = false;
        model.simulate_hair = false;
        let settings = crate::film_settings::FilmSettings::default()
            .patched(&serde_json::json!({"source_rate_m3_s":1e-9}))
            .unwrap();
        model.film.as_mut().unwrap().configure(settings).unwrap();
        model.advance(1. / 240.).unwrap();
        let old = model.film.as_ref().unwrap().measurements();
        model.set_body_parameters(male).unwrap();
        let new = model.film.as_ref().unwrap().measurements();
        let mass = old["massKg"].as_f64().unwrap();
        assert!((new["massKg"].as_f64().unwrap() - mass).abs() / mass < 1e-12);
        assert_eq!(new["initialMassKg"], old["initialMassKg"]);
        assert_eq!(new["sourceAddedMassKg"], old["sourceAddedMassKg"]);
        model.set_body_parameters(before).unwrap();
        let restored = model.film.as_ref().unwrap().measurements();
        assert!((restored["massKg"].as_f64().unwrap() - mass).abs() / mass < 1e-12);
        model.advance(1. / 240.).unwrap();
        let advanced = model.film.as_ref().unwrap().measurements();
        let addition = advanced["sourceAddedMassKg"].as_f64().unwrap()
            - restored["sourceAddedMassKg"].as_f64().unwrap();
        assert!((addition - 1000. * 1e-9 / 240.).abs() < 1e-15);
        assert!((advanced["massKg"].as_f64().unwrap() - mass - addition).abs() < 1e-15);
    }
}

#[cfg(test)]
mod source_switch_geometry_tests {
    #[test]
    fn switched_film_uses_morphed_geometry_in_the_first_frame() {
        let mut model = super::FemaleDemo::new().unwrap();
        model.animation_only = true;
        model.simulate_hair = false;
        model.surface_diffusion_enabled = false;
        model.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        let mass = model.film.as_ref().unwrap().measurements()["massKg"]
            .as_f64()
            .unwrap();
        let parameters = model
            .body_parameters
            .patched(&serde_json::json!({"body_model":"male","height_cm":190.,"torso_depth":1.2}))
            .unwrap();
        model.set_body_parameters(parameters).unwrap();
        let first = model.film.as_ref().unwrap().measurements();
        let mesh = model.mesh().unwrap();
        model
            .film
            .as_mut()
            .unwrap()
            .update_substrate(mesh.vertices())
            .unwrap();
        let refreshed = model.film.as_ref().unwrap().measurements();
        assert_eq!(
            first["maximumCellThicknessM"],
            refreshed["maximumCellThicknessM"]
        );
        assert!((first["massKg"].as_f64().unwrap() - mass).abs() / mass < 1e-12);
        assert_eq!(first["sourceAddedMassKg"], refreshed["sourceAddedMassKg"]);
    }
}

#[cfg(test)]
mod same_source_film_morph_tests {
    #[test]
    fn morphed_film_updates_immediately_without_resetting_time_or_mass() {
        let mut model = super::FemaleDemo::new().unwrap();
        model.animation_only = true;
        model.simulate_hair = false;
        model.surface_diffusion_enabled = false;
        model.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        model.advance(1. / 120.).unwrap();
        let before = model.film.as_ref().unwrap().measurements();
        let time = model.time;
        let parameters = model
            .body_parameters
            .patched(&serde_json::json!({"height_cm":190.,"torso_depth":1.2}))
            .unwrap();
        model.set_body_parameters(parameters).unwrap();
        assert_eq!(model.time, time);
        assert!(!model.simulate_hair);
        let first = model.film.as_ref().unwrap().measurements();
        let mesh = model.mesh().unwrap();
        model
            .film
            .as_mut()
            .unwrap()
            .update_substrate(mesh.vertices())
            .unwrap();
        let refreshed = model.film.as_ref().unwrap().measurements();
        assert_eq!(
            first["maximumCellThicknessM"],
            refreshed["maximumCellThicknessM"]
        );
        let mass = before["massKg"].as_f64().unwrap();
        assert!((first["massKg"].as_f64().unwrap() - mass).abs() / mass < 1e-12);
        assert_eq!(first["sourceAddedMassKg"], before["sourceAddedMassKg"]);
        assert_ne!(
            first["maximumCellThicknessM"],
            before["maximumCellThicknessM"]
        );
        model
            .set_body_parameters(super::super::body_parameters::BodyParameters::default())
            .unwrap();
        assert_eq!(model.time, time);
        let reset = model.film.as_ref().unwrap().measurements();
        assert!((reset["massKg"].as_f64().unwrap() - mass).abs() / mass < 1e-12);
    }
    #[test]
    fn appearance_only_edit_preserves_live_physics_and_film() {
        let mut model = super::FemaleDemo::new().unwrap();
        model.animation_only = true;
        model.simulate_hair = false;
        model.surface_diffusion_enabled = false;
        model.enable_film([0., 0.20, 0.13], 0.04, 5e-8).unwrap();
        model.advance(1. / 120.).unwrap();
        let before = model.film.as_ref().unwrap().measurements();
        let positions = model.skin.positions().to_vec();
        let time = model.time;
        let params = model
            .body_parameters
            .patched(&serde_json::json!({"areola_radius_mm":25.,"areola_pigmentation":0.4}))
            .unwrap();
        model.set_body_parameters(params).unwrap();
        assert_eq!(model.time, time);
        assert_eq!(model.skin.positions(), positions);
        assert_eq!(model.film.as_ref().unwrap().measurements(), before);
        assert_eq!(model.body_parameters, params);
    }
}

#[cfg(test)]
mod warm_light_tests {
    #[test]
    #[ignore = "checks warm/cold lighting on real posed render geometry"]
    fn actual_blink_warm_light_matches_cold() {
        let mut model = super::FemaleDemo::new().unwrap();
        model.animation_only = true;
        model.simulate_hair = false;
        model.show_hair = false;
        model.preview_pose(0.);
        model.mesh().unwrap();
        model.preview_pose(0.9);
        let warm = model.mesh().unwrap();
        *model.surface_light_guess.borrow_mut() = None;
        let cold = model.mesh().unwrap();
        assert_eq!(warm.indices(), cold.indices());
        let mut maximum = 0_f32;
        for (a, b) in warm.vertices().iter().zip(cold.vertices()) {
            assert_eq!(a.position, b.position);
            for channel in 0..3 {
                maximum = maximum.max((a.color[channel] - b.color[channel]).abs());
            }
        }
        assert!(maximum <= 1e-6, "lighting error {maximum}");
        println!("ACTUAL BLINK WARM/COLD max_rgb_error={maximum}");
    }
}
