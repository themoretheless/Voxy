//! Grasp presets, camera controls, GPU binding exports and mesh assembly for `FemaleDemo`.
use super::*;

#[allow(dead_code)]
impl FemaleDemo {
    pub fn record_presentation(&mut self, frame: u64, size: [u32; 2]) {
        let Some(path) = &self.parameter_file else {
            return;
        };
        if let Some(id) = presentation_receipts::pending_film_state(path)
            && let Err(error) = presentation_receipts::publish_film_state(
                path,
                &id,
                &self.body_parameters.to_json(),
                self.time,
                frame,
                self.film
                    .as_ref()
                    .map(crate::surface_film_preview::FilmPreview::state_snapshot),
            )
        {
            eprintln!("Film state capture failed: {error}");
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
            &serde_json::json!({"view":{"mode":if self.show_strain {"strain"} else if self.show_skin {"displacement"} else {"material"}},"skinShell":tissue_diagnostics::measurements(&self.skin,!self.animation_only),"fluid":self.film.as_ref().map(crate::surface_film_preview::FilmPreview::measurements),
                "tissueCageVolumesM3":self.regions.iter().map(crate::volume_regions::Region::volume_m3).collect::<Vec<_>>(),
                "tissueCageMassesKg":self.regions.iter().map(crate::volume_regions::Region::mass_kg).collect::<Vec<_>>(),
                "secondaryDynamics": {
                    "enabled":self.secondary_only && !self.animation_only,
                    "authoredJump":self.rig.authored_jump_source().is_some(),
                    "authoredGait":self.rig.authored_gait_source().map(|(_,clip)|format!("{clip:?}")),
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
    pub fn enable_cold_response(
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
    pub(crate) fn advance_cold_response(&mut self, dt: f64) -> Result<(), &'static str> {
        self.cold_response = self.prepare_cold_response(dt)?;
        Ok(())
    }
    pub(crate) fn prepare_cold_response(
        &self,
        dt: f64,
    ) -> Result<Option<crate::body_parameters::ColdResponse>, &'static str> {
        let mut next = self.cold_response;
        if let Some(response) = &mut next {
            response.set_target(f64::from(self.body_parameters.nipple_cold_response))?;
            response.advance(dt)?;
        }
        Ok(next)
    }
    pub(crate) fn render_body_parameters(&self) -> crate::body_parameters::BodyParameters {
        self.cold_response.map_or(self.body_parameters, |state| {
            state.apply(self.body_parameters)
        })
    }
    /// Samples a rig pose for offscreen inspection without advancing the physical solver.
    #[allow(dead_code)] // Also compiled directly into the offscreen render example.
    pub fn set_grasp_target(&mut self, shape: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.set_grasp_target_with_offset(shape, Vec3::ZERO)
    }
    pub fn set_grasp_target_with_offset(
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
                include_str!("../../../../assets/characters/grasp-handle.obj").to_owned()
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
    pub fn prepare_grasp_presets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for shape in ["cylinder", "sphere", "handle"] {
            self.set_grasp_target(shape)?;
        }
        Ok(())
    }
    pub fn set_grasp_cycle(&mut self, enabled: bool) {
        self.rig.set_grasp_cycle(enabled);
        self.skin_rig.set_grasp_cycle(enabled);
    }
    pub fn set_hand_focus(&mut self, enabled: bool) {
        self.hand_focus = enabled;
        self.rig.set_body_motion(!enabled);
        self.skin_rig.set_body_motion(!enabled);
    }
    pub fn adjust_grasp(&mut self, delta: f32) -> Result<(), &'static str> {
        if !delta.is_finite() {
            return Err("grasp adjustment must be finite");
        }
        let amount = (self.rig.grasp_amount(self.time as f32) + delta).clamp(0., 1.);
        self.rig.set_grasp(amount)?;
        self.skin_rig.set_grasp(amount)?;
        Ok(())
    }
    /// Prescribed engineering strain fixture for offscreen diagnostic validation.
    pub fn set_strain_fixture(&mut self, x_stretch: f64) -> Result<(), &'static str> {
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
    pub fn gpu_hair_surface_frames(&self) -> Vec<voxy_render::FiberSurfaceFrame> {
        self.hair.gpu_surface_frames()
    }
    pub fn gpu_hair_surface_transform(&self) -> glam::Mat4 {
        if self.animation_only || !self.simulate_hair {
            self.hair_physics_head_matrix(self.time)
        } else {
            glam::Mat4::IDENTITY
        }
    }
    /// CPU reference/init only. Live resident directors are written by GPU scatter.
    pub fn gpu_hair_vertex_tangents(
        &self,
        mesh: &voxy_render::SceneMesh,
    ) -> Result<Vec<[f32; 4]>, voxy_render::SceneError> {
        let frames = self.gpu_hair_surface_frames();
        let count = mesh.vertices().iter().filter(|v| v.uv[0] == -7.).count();
        if frames.is_empty()
            || count == 0
            || !count.is_multiple_of(frames.len())
            || !(count / frames.len()).is_multiple_of(4)
        {
            return Err(voxy_render::SceneError::InvalidGeometry);
        }
        let per_frame = count / frames.len();
        let pose = self.gpu_hair_surface_transform();
        let mut tangents = vec![[0., 0., 1., 1.]; mesh.vertices().len()];
        let mut hair_index = 0;
        for (vertex, tangent) in mesh.vertices().iter().zip(&mut tangents) {
            if vertex.uv[0] != -7. {
                continue;
            }
            let director = pose.transform_vector3(glam::Vec3::from_slice(
                &frames[hair_index / per_frame].w_blue,
            ));
            if !director.is_finite() || director.length_squared() < 1e-12 {
                return Err(voxy_render::SceneError::InvalidGeometry);
            }
            let director = director.normalize();
            *tangent = [director.x, director.y, director.z, 1.];
            hair_index += 1;
        }
        Ok(tangents)
    }
    pub fn gpu_hair_surface_input(
        &self,
    ) -> Result<voxy_render::FiberSurfaceInput, voxy_render::SceneError> {
        let mut input = self.hair.gpu_surface_input()?;
        input.replace_transform(self.gpu_hair_surface_transform())?;
        Ok(input)
    }
    pub fn simulation_time(&self) -> f64 {
        self.time
    }
    pub fn preview_pose(&mut self, time: f64) {
        self.time = time;
    }
    pub fn title(&self) -> String {
        let mut legend = String::new();
        if self.show_strain {
            legend.push_str("Strain magnitude: blue 0%, green 5%, red >=10% | ");
        } else if self.show_skin {
            legend.push_str("Displacement: blue 0 mm, green 5 mm, red >=10 mm | ");
        }
        if self
            .film
            .as_ref()
            .is_some_and(crate::surface_film_preview::FilmPreview::thickness_view_enabled)
        {
            legend.push_str("Film: blue 0 um, green 50 um, red >=100 um; <0.1 um hidden | ");
        }
        format!("{legend}{}", self.base_title())
    }
    pub fn diagnostic_legend_labels(&self) -> Vec<[&'static str; 3]> {
        let mut rows = Vec::new();
        if self.show_strain {
            rows.push(["0%", "5%", "10%+"]);
        } else if self.show_skin {
            rows.push(["0mm", "5mm", "10mm+"]);
        }
        if self
            .film
            .as_ref()
            .is_some_and(crate::surface_film_preview::FilmPreview::thickness_view_enabled)
        {
            rows.push(["0um", "50um", "100um+"]);
        }
        rows
    }
    pub fn diagnostic_legend_visible(&self) -> bool {
        self.show_strain
            || self.show_skin
            || self
                .film
                .as_ref()
                .is_some_and(crate::surface_film_preview::FilmPreview::thickness_view_enabled)
    }
    pub(crate) fn base_title(&self) -> String {
        if self.physical_stance.is_some() {
            return "Voxy Physical Stance | foot support, squat and roll | Space: pause | arrows: orbit/zoom".into();
        }
        if let Some((_, clip)) = self.rig.authored_gait_source() {
            return format!(
                "Voxy {clip:?} | in-place skeletal preview | 1 idle / 2 walk / 3 jog / 4 sprint | Space: pause | arrows: orbit/zoom"
            );
        }
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
    pub fn focus(&self) -> Vec3 {
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
    pub fn eye(&self) -> Vec3 {
        let focus = self.focus();
        focus
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos() * self.distance,
                self.pitch.sin() * self.distance,
                self.yaw.cos() * self.pitch.cos() * self.distance,
            )
    }
    pub fn orbit(&mut self, x: f32, y: f32) {
        use crate::orbit::ORBIT_PITCH_LIMIT;
        self.yaw += x;
        self.pitch = (self.pitch + y).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
    }
    pub fn zoom(&mut self, amount: f32) {
        let (minimum, step) = crate::orbit::zoom_range(self.hand_focus);
        self.distance =
            (self.distance + amount * step).clamp(minimum, crate::orbit::ORBIT_MAX_DISTANCE);
    }
    pub(crate) fn support_offset(&self, rest: [f64; 3], time: f64) -> f64 {
        self.support_offset_with_secondary(rest, time, &self.secondary)
    }
    pub(crate) fn support_offset_with_secondary(
        &self,
        rest: [f64; 3],
        time: f64,
        secondary: &[SecondaryMotion; 4],
    ) -> f64 {
        if self.animation_only {
            return 0.0;
        }
        let bob = self.root_bob(time);
        if self.rig_pose_only {
            return bob;
        }
        let mut offset = bob;
        for (i, state) in secondary.iter().enumerate() {
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
    pub fn background_view_replica(&self) -> Result<Self, Box<dyn std::error::Error>> {
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
        if let Some(source) = self.rig.authored_jump_source() {
            view.install_authored_jump_source(source)?;
        }
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
    pub fn can_use_gpu_secondary(&self) -> bool {
        self.secondary_only
            && !self.rig_pose_only
            && !self.animation_only
            && self.film.is_none()
            && self.cold_response.is_none()
            && self.parameter_file.is_none()
            && self.face_parameter_file.is_none()
            && self.body_parameters == Default::default()
            && self.time == 0.
    }
    pub fn gpu_secondary_binding(
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
        let base_controls = 5 + self.regions.len() as u32 * 7;
        let [pelvic_start, _pelvic_end] = self.last_pelvic_vertex_range.get();
        if pelvic_start > 0 && !self.pelvic.is_empty() {
            self.pelvic.append_gpu_secondary_weights(
                pelvic_start,
                base_controls,
                base_controls + 1,
                &mut rows,
            );
        }
        let mut offsets = vec![0];
        let mut weights = Vec::new();
        for row in rows {
            weights.extend(row);
            offsets.push(weights.len() as u32);
        }
        Ok((offsets, weights, base_controls + 2))
    }
    pub fn can_use_resident_surface_light(&self, fraction: f32) -> bool {
        !self.show_skin
            && !self.show_strain
            && self.surface_diffusion_enabled
            && self.film.is_none()
            && self.face_parameters == crate::face_parameters::FaceParameters::default()
            && self
                .face_parameters
                .value("skin_surface_light")
                .unwrap_or(0.35)
                == fraction
    }
    pub fn surface_light_normals(&self) -> std::cell::Ref<'_, Vec<[f32; 3]>> {
        self.surface_light_normals.borrow()
    }
    pub fn gpu_secondary_body_vertices(&self) -> u32 {
        self.body_vertices as u32
    }
    pub fn gpu_jump_weights(&self) -> Vec<voxy_render::SurfaceRigidSkinWeight> {
        self.rig.gpu_jump_weights()
    }
    pub fn gpu_secondary_jump_weights(
        &self,
        total_vertices: usize,
    ) -> Vec<voxy_render::SurfaceRigidSkinWeight> {
        let mut weights = self.rig.gpu_jump_weights();
        let [pelvic_start, pelvic_end] = self.last_pelvic_vertex_range.get();
        if pelvic_start >= weights.len()
            && pelvic_end > pelvic_start
            && pelvic_end <= total_vertices
            && !self.pelvic.is_empty()
        {
            let head_weight = voxy_render::SurfaceRigidSkinWeight {
                joints: [3, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            };
            weights.resize(pelvic_start, head_weight);
            let base_weights = self.rig.gpu_jump_weights();
            self.pelvic
                .append_gpu_jump_weights(&base_weights, &mut weights);
        }
        weights
    }
    pub fn gpu_jump_palette(&self) -> Vec<glam::Mat4> {
        if let Some(stance) = &self.physical_stance {
            let actor = glam::Mat4::from_translation(stance.offset.as_vec3());
            return stance
                .palette
                .iter()
                .map(|matrix| actor * *matrix)
                .collect();
        }
        self.rig
            .jump_palette(if self.animation_only { 5. } else { self.time })
    }
    pub fn gpu_secondary_controls(&self) -> Vec<[f32; 4]> {
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
        let motion = self.pelvic_motion_state();
        let pulse_delta = if self.time == 0.0 && self.steps == 0 {
            0.0
        } else {
            0.5 * (motion.time * 2.6).sin() * 0.30
                + 0.35 * motion.prolapse_pulse.clamp(-1.0, 1.0)
                + 0.25 * motion.sphincter_tone.clamp(-1.0, 1.0)
        };
        let squeeze_delta = motion.gluteal_compression.clamp(-1.0, 1.0);
        controls.push([0.0, -pulse_delta, 0.0, 0.0]);
        controls.push([0.0, -squeeze_delta, 0.0, 0.0]);
        controls
    }
    pub fn mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        self.mesh_with_hair(self.show_hair)
    }
    pub fn mesh_with_full_hair(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        self.mesh_with_hair(true)
    }
    pub fn mesh_without_hair(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        self.mesh_with_hair(false)
    }
    pub(crate) fn pre_skin_surface_vertices(&self) -> Vec<SceneVertex> {
        let mut checkpoint =
            std::env::var_os("VOXY_FACE_MESH_TRACE").map(|_| std::time::Instant::now());
        let mut mark = |stage: &str| {
            if let Some(start) = checkpoint.as_mut() {
                eprintln!(
                    "FACE DEFORMATION t={:.3} stage={stage} ms={:.3}",
                    self.time,
                    start.elapsed().as_secs_f64() * 1000.
                );
                *start = std::time::Instant::now();
            }
        };
        let mut vertices = self.vertices.clone();
        self.face_parameters
            .apply_wrinkles(&mut vertices[..self.body_vertices]);
        mark("clone_and_wrinkles");
        if self.show_complexion && !self.show_skin && !self.show_strain {
            crate::female_complexion::apply(
                &mut vertices[..self.body_vertices],
                &self.normals[..self.body_vertices],
            );
        }
        mark("complexion");
        crate::female_face::deform_with_contour(
            &mut vertices,
            self.body_vertices,
            self.face_pose(),
            self.lid_contour.as_ref(),
        );
        mark("expression");
        vertices
    }
    pub(crate) fn posed_surface_vertices(&self) -> Vec<SceneVertex> {
        let mut vertices = self.pre_skin_surface_vertices();
        let mut checkpoint =
            std::env::var_os("VOXY_FACE_MESH_TRACE").map(|_| std::time::Instant::now());
        let mut mark = |stage: &str| {
            if let Some(start) = checkpoint.as_mut() {
                eprintln!(
                    "FACE DEFORMATION t={:.3} stage={stage} ms={:.3}",
                    self.time,
                    start.elapsed().as_secs_f64() * 1000.
                );
                *start = std::time::Instant::now();
            }
        };
        if let Some(stance) = &self.physical_stance {
            self.rig
                .deform_pose(&mut vertices, self.time as f32, &stance.pose);
            for vertex in &mut vertices {
                let position = Vec3::from_array(vertex.position).as_dvec3() + stance.offset;
                vertex.position = position.as_vec3().to_array();
            }
        } else if self.secondary_only {
            if !self.animation_only {
                self.rig.deform_jump(&mut vertices, self.time);
            }
        } else {
            self.rig.deform(&mut vertices, self.time as f32);
        }
        mark("rig");
        for (vertex, rest) in vertices.iter_mut().zip(&self.vertices) {
            vertex.position[1] +=
                self.support_offset(rest.position.map(f64::from), self.time) as f32;
        }
        mark("support");
        vertices
    }
    /// Sparse physical-shell transfer only. This does not replace skeletal,
    /// facial, or region deformation and must follow an independently posed base.
    pub fn gpu_shell_binding(
        &self,
        count: usize,
    ) -> Result<(Vec<u32>, Vec<voxy_render::SurfaceDeformationWeight>, u32), voxy_render::SceneError>
    {
        use voxy_render::{SceneError, SurfaceDeformationWeight};
        if count < self.body_vertices
            || count > u32::MAX as usize
            || self.skin.positions().len() > u32::MAX as usize
        {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        let mut rows = vec![Vec::new(); count];
        for binding in self.embedding.bindings() {
            let ids = self.embedding.shell_triangles()[binding.triangle];
            for (control, weight) in ids.into_iter().zip(binding.weights) {
                rows[binding.vertex].push(SurfaceDeformationWeight {
                    control: control as u32,
                    weight: weight as f32,
                });
            }
        }
        let mut offsets = Vec::with_capacity(count + 1);
        let mut weights = Vec::new();
        offsets.push(0);
        for row in rows {
            weights.extend(row);
            offsets.push(
                u32::try_from(weights.len()).map_err(|_| SceneError::GeometryCapacityExceeded)?,
            );
        }
        Ok((offsets, weights, self.skin.positions().len() as u32))
    }
    pub fn gpu_shell_controls(&self) -> Result<Vec<[f32; 4]>, voxy_render::SceneError> {
        use voxy_render::SceneError;
        if self.skin.positions().len() != self.targets.len() {
            return Err(SceneError::InvalidGeometry);
        }
        self.skin
            .positions()
            .iter()
            .zip(&self.targets)
            .map(|(shell, reference)| {
                // Subtract in the physical solver's precision before conversion:
                // uploading absolute f32 poses would cancel small displacements.
                let delta = if self.rig_pose_only {
                    [0.; 3]
                } else {
                    std::array::from_fn(|k| (shell[k] - reference[k]) as f32)
                };
                if delta.iter().any(|v| !v.is_finite()) {
                    return Err(SceneError::NonFiniteVertex);
                }
                Ok([delta[0], delta[1], delta[2], 0.])
            })
            .collect()
    }
    pub(crate) fn surface_displacement(&self) -> Result<Vec<[f64; 3]>, &'static str> {
        if self.rig_pose_only {
            Ok(vec![[0.; 3]; self.body_vertices])
        } else {
            self.embedding.deform(
                self.skin.positions(),
                &self.targets,
                &vec![[0.; 3]; self.body_vertices],
            )
        }
    }
    pub(crate) fn pelvic_motion_state(&self) -> crate::female_pelvic::PelvicMotionState {
        let gluteal_compression = if self.animation_only || self.rig_pose_only {
            0.0
        } else {
            ((-0.5 * (self.secondary[2].offset()[1] + self.secondary[3].offset()[1])) as f32 * 45.0)
                .clamp(-0.35, 1.0)
        };
        let t = self.time as f32;
        let prolapse_pulse = if self.time == 0.0 && self.steps == 0 {
            0.0
        } else {
            0.65 * (t * 2.8).sin() + 0.35 * (t * 5.6 + 0.7).cos()
        };
        let sphincter_tone = if self.time == 0.0 && self.steps == 0 {
            0.0
        } else {
            0.60 * (t * 3.2 + 0.4).cos() + 0.40 * gluteal_compression
        };
        crate::female_pelvic::PelvicMotionState {
            time: t,
            gluteal_compression,
            prolapse_pulse,
            sphincter_tone,
        }
    }
    pub(crate) fn finish_surface_vertices(
        &self,
        vertices: &mut [SceneVertex],
        displacement: &[[f64; 3]],
        body_end: usize,
    ) {
        self.face_parameters
            .apply_shape(vertices, self.hair_head_matrix(self.time));
        self.render_body_parameters()
            .apply(&mut vertices[..body_end]);
        for (vertex, delta) in vertices.iter_mut().zip(displacement) {
            for k in 0..3 {
                vertex.position[k] += delta[k] as f32;
            }
        }
        if self.secondary_only && !self.rig_pose_only {
            for region in &self.regions {
                region.apply(&self.vertices, vertices, self.root_bob(self.time));
            }
        }
    }
    pub(crate) fn film_substrate_vertices(&self) -> Result<Vec<SceneVertex>, &'static str> {
        let mut vertices = self.posed_surface_vertices();
        let displacement = self.surface_displacement()?;
        let body_end = vertices.len();
        self.finish_surface_vertices(&mut vertices, &displacement, body_end);
        vertices.truncate(self.body_vertices);
        Ok(vertices)
    }
    pub(crate) fn mesh_with_hair(
        &self,
        include_hair: bool,
    ) -> Result<SceneMesh, voxy_render::SceneError> {
        self.mesh_with_hair_and_bindings(include_hair, None)
    }
    pub fn mesh_with_gpu_light_bindings(
        &self,
        include_hair: bool,
    ) -> Result<(SceneMesh, Vec<voxy_render::SurfaceColorBinding>), voxy_render::SceneError> {
        let mut bindings = Vec::new();
        let mesh = self.mesh_with_hair_and_bindings(include_hair, Some(&mut bindings))?;
        Ok((mesh, bindings))
    }
    pub(crate) fn mesh_with_hair_and_bindings(
        &self,
        include_hair: bool,
        inherited_colors: Option<&mut Vec<voxy_render::SurfaceColorBinding>>,
    ) -> Result<SceneMesh, voxy_render::SceneError> {
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
        let mut vertices = self.posed_surface_vertices();
        let displacement = self
            .surface_displacement()
            .expect("validated finite shell and binding dimensions");
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
        if std::env::var_os("VOXY_GPU_SURFACE_LIGHT").is_some() || cfg!(test) {
            *self.surface_light_normals.borrow_mut() = normals[..self.body_vertices]
                .iter()
                .map(|n| n.to_array())
                .collect();
        }
        mark("normals");
        let key = crate::female_transmission::key_direction();
        let fill = Vec3::new(0.8, 0.2, 0.5).normalize();
        let surface_fraction = self
            .face_parameters
            .value("skin_surface_light")
            .unwrap_or(0.35);
        let surface_light = if self.surface_diffusion_enabled
            && !self.resident_surface_light
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
        self.features.append_configured_with_bindings(
            &posed_face,
            &normals[..self.body_vertices],
            &mut vertices,
            &mut indices,
            self.hair_head_matrix(self.time),
            self.face_pose(),
            &self.face_parameters,
            inherited_colors,
        );
        mark("face_features");
        let face_end = vertices.len();
        let pelvic_index_start = indices.len();
        let mut pelvic_normals = Vec::new();
        self.pelvic.append_with_normals(
            &normal_geometry[..self.body_vertices],
            &normals[..self.body_vertices],
            &mut vertices,
            &mut indices,
            Some(&mut pelvic_normals),
            self.pelvic_motion_state(),
        );
        self.last_pelvic_vertex_range
            .set([face_end, vertices.len()]);
        let hair_start = vertices.len();
        let _hair_index_start = indices.len();
        let mut hair_normals = Vec::new();
        if include_hair {
            self.hair
                .append_with_normals(&mut vertices, &mut indices, &mut hair_normals);
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
                *normal = head
                    .transform_vector3(Vec3::from_array(*normal))
                    .normalize_or_zero()
                    .to_array();
            }
        }
        self.finish_surface_vertices(&mut vertices, &displacement, face_end);
        self.pelvic
            .recess_covered_body_vertices(&normals[..self.body_vertices], &mut vertices);

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
        // Hair radial normals come directly from the solved Cosserat frame, and
        // pelvic manifold normals come directly from the 129x193 grid evaluation.
        // Weld only the much smaller non-hair, non-pelvic surfaces.
        let authored_normals = if self.film.is_none()
            && self.face_parameters == crate::face_parameters::FaceParameters::default()
        {
            let prefix = SceneMesh::new(
                vertices[..face_end].to_vec(),
                indices[..pelvic_index_start].to_vec(),
            )?
            .with_prepared_upload_streams();
            let suffix_indices = indices[hair_index_end..]
                .iter()
                .map(|i| {
                    i.checked_sub(hair_end as u32)
                        .ok_or(voxy_render::SceneError::InvalidGeometry)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let suffix = SceneMesh::new(vertices[hair_end..].to_vec(), suffix_indices)?
                .with_prepared_upload_streams();
            let mut combined = prefix.authored_normals().unwrap().to_vec();
            // The same transported base normals already drive CPU lighting.
            // Keep them in the draw stream whether the groom is separate or not.
            for (output, normal) in combined.iter_mut().zip(&normals) {
                *output = normal.to_array();
            }
            combined.append(&mut pelvic_normals);
            combined.append(&mut hair_normals);
            combined.extend_from_slice(suffix.authored_normals().unwrap());
            Some(combined)
        } else {
            None
        };
        let mesh = SceneMesh::new(vertices, indices)?.with_material_coordinates(coordinates)?;
        let mesh = if let Some(normals) = authored_normals {
            mesh.with_normals(normals)?
        } else {
            mesh
        };
        mark("mesh_validation");
        if let Some(film) = &self.film {
            mesh.with_material_parameters(film.optical_parameters())
        } else {
            Ok(mesh)
        }
    }
    pub fn verify(&self) -> Result<(), &'static str> {
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
