//! GPU host initialization, device-loss recovery and fallback primitive mesh builders.
use super::*;

impl SceneApp {
    pub(super) fn use_gpu_secondary(&self) -> bool {
        if std::env::var_os("VOXY_GPU_SECONDARY").is_some() {
            return true;
        }
        !self.smoke
            && std::env::var_os("VOXY_DISABLE_GPU_SECONDARY").is_none()
            && std::env::var_os("VOXY_CPU_SECONDARY").is_none()
            && std::env::var_os("VOXY_ASYNC_FULL").is_none()
            && std::env::var_os("VOXY_GPU_BODY").is_none()
            && std::env::var_os("VOXY_GPU_SURFACE_LIGHT").is_none()
            && std::env::var_os("VOXY_GPU_HAIR_PHYSICS").is_none()
            && std::env::var_os("VOXY_RESIDENT_HAIR").is_none()
            && self
                .female
                .as_ref()
                .is_some_and(voxy_demos::female_demo::FemaleDemo::can_use_gpu_secondary)
    }

    pub(super) fn initialize(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.resources.is_some() {
            return Ok(());
        }
        let use_gpu_secondary = self.use_gpu_secondary();
        if use_gpu_secondary && let Some(female) = &mut self.female {
            if std::env::var_os("VOXY_GPU_SECONDARY").is_none() {
                female.simulate_hair = false;
            }
            if !female.simulate_hair {
                female.gpu_secondary_fast = true;
            }
        }
        let window = if let Some(window) = &self.window {
            Arc::clone(window)
        } else {
            Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy 2D/3D | Space: pause | Esc: exit")
                        .with_visible(false)
                        .with_inner_size(if use_gpu_secondary {
                            winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(1280, 900))
                        } else {
                            winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(900., 650.))
                        }),
                )?,
            )
        };
        let size = window.inner_size();
        let options = self.graphics;
        let instance = options.create_instance_with_display(event_loop.owned_display_handle());
        let mut host = pollster::block_on(Renderer::new_with_instance(
            Arc::clone(&window),
            size.width,
            size.height,
            options,
            instance,
        ))?;
        if use_gpu_secondary || std::env::var_os("VOXY_ASYNC_FULL").is_some() {
            match host.enable_unthrottled_presentation() {
                Ok(mode) => eprintln!("GPU SECONDARY presentation={mode:?}"),
                Err(error) => eprintln!(
                    "GPU SECONDARY non-vsync unavailable: {error}; retaining existing presentation"
                ),
            }
        }
        if self.motion_vectors == MotionMode::Enabled {
            pollster::block_on(host.enable_motion_vectors())?;
        }
        if self.female.is_some()
            && (!use_gpu_secondary || std::env::var_os("VOXY_GPU_SECONDARY_MSAA").is_some())
        {
            host.enable_msaa4()?;
        }
        if use_gpu_secondary {
            eprintln!(
                "GPU SECONDARY resolution={}x{} MSAA4={}",
                size.width,
                size.height,
                std::env::var_os("VOXY_GPU_SECONDARY_MSAA").is_some()
            );
        }
        // The render host owns the device; physics receives its existing backend.
        // Keep qualification explicit until coupled dynamic GPU admission passes.
        if self.female.is_some() && std::env::var_os("VOXY_GPU_HAIR_PHYSICS").is_some() {
            self.hair_solver = Some(pollster::block_on(
                voxy_demos::gpu_hair_solver::GpuHairLinearSolver::new_coupled(
                    host.device(),
                    host.queue(),
                ),
            )?);
        }
        let mut scene = host.create_scene_renderer();
        if self.female.is_some() {
            pollster::block_on(
                scene.reload_shader(host.device(), voxy_demos::female_eyes::MATERIAL_SHADER),
            )?;
        }
        let use_gpu_body = std::env::var_os("VOXY_GPU_BODY").is_some();
        if use_gpu_body
            && (self.female.is_none()
                || std::env::var_os("VOXY_ASYNC_FULL").is_some()
                || use_gpu_secondary
                || std::env::var_os("VOXY_GPU_SURFACE_LIGHT").is_some())
        {
            return Err(
                "GPU body currently requires synchronous physical stance with CPU surface lighting"
                    .into(),
            );
        }
        let use_gpu_surface_light = std::env::var_os("VOXY_GPU_SURFACE_LIGHT").is_some();
        if use_gpu_surface_light && self.female.is_none() {
            return Err("GPU surface lighting requires a character model".into());
        }
        if use_gpu_surface_light
            && (std::env::var_os("VOXY_ASYNC_FULL").is_some() || use_gpu_secondary)
        {
            return Err("authored light-normal frame ownership for async/GPU-secondary combinations is not qualified yet".into());
        }
        if use_gpu_surface_light
            && std::env::var("VOXY_FACE_EXPERIMENTAL_TRANSMISSION").as_deref() == Ok("1")
        {
            return Err(
                "GPU surface lighting additive experimental transmission is not qualified yet"
                    .into(),
            );
        }
        let use_gpu_hair = (use_gpu_body
            || std::env::var_os("VOXY_ASYNC_FULL").is_some()
            || std::env::var_os("VOXY_RESIDENT_HAIR").is_some())
            && std::env::var_os("VOXY_DISABLE_GPU_HAIR").is_none()
            && self.female.as_ref().is_some_and(|f| {
                f.face_parameters == voxy_demos::face_parameters::FaceParameters::default()
                    && f.film.is_none()
                    && f.simulate_hair
            });
        let mut inherited_colors = Vec::new();
        let mesh = if let Some(female) = &self.female {
            if use_gpu_surface_light {
                let (mesh, bindings) =
                    female.mesh_with_gpu_light_bindings(female.show_hair || use_gpu_hair)?;
                inherited_colors = bindings;
                mesh
            } else if use_gpu_hair {
                female.mesh_with_full_hair()?
            } else {
                female.mesh()?
            }
        } else if let Some(xray) = &self.xray {
            xray.shell(Vec3::new(0.0, 0.2, 3.0))?
        } else if let Some(fem) = &self.fem {
            fem.mesh()?
        } else if let Some(wear) = &self.wear {
            wear.mesh()?
        } else if let Some(liquids) = &self.liquids {
            liquids.mesh()?
        } else if let Some(tissues) = &self.tissues {
            tissues.mesh()?
        } else if let Some(gravity) = &self.gravity {
            gravity.mesh()?
        } else if let Some(strands) = &self.strands {
            strands.mesh()?
        } else if self.rush.is_some() {
            rush_box_mesh([0.2, 0.65, 0.95, 1.0])?
        } else {
            cube_mesh()?
        };
        let mut hair_geometry = None;
        let mut gpu_hair = None;
        let cube = if use_gpu_hair {
            let female = self
                .female
                .as_ref()
                .ok_or("resident hair requires full model")?;
            let (_, indices) = voxy_demos::full_model_worker::partition_hair(&mesh)
                .map_err(std::io::Error::other)?;
            let mut hair = scene.upload_compute_tangent_mesh(
                host.device(),
                &mesh,
                &female.gpu_hair_vertex_tangents(&mesh)?,
            )?;
            hair.update_index_partition(
                host.queue(),
                if female.show_hair { &indices } else { &[] },
            )?;
            pollster::block_on(scene.set_geometry_opaque_shader(
                host.device(),
                &mut hair,
                include_str!("../../../voxy_demos/src/female_hair_material_tangent.wgsl"),
            ))?;
            let input = female.gpu_hair_surface_input()?;
            let base = mesh
                .vertices()
                .iter()
                .position(|v| v.uv[0] == -7.)
                .ok_or("missing full hair surface")?;
            let program = pollster::block_on(voxy_render::ComputeProgram::new(
                host.device(),
                voxy_render::FIBER_SURFACE_SHADER,
            ))?;
            let job = program.create_job(host.device(), input.bytes())?;
            let transfer =
                pollster::block_on(voxy_render::FiberSurfaceTransfer::new_with_tangents(
                    host.device(),
                    &hair,
                    job.buffer(),
                    &input,
                    base as u32,
                ))?;
            eprintln!(
                "FULL MODEL GPU HAIR initialized vertices={} resident=true",
                input.vertex_count()
            );
            gpu_hair = Some(ResidentHair {
                input,
                job,
                transfer,
                indices,
                dirty: true,
            });
            hair_geometry = Some(hair);
            if use_gpu_surface_light || use_gpu_body {
                scene.upload_compute_mesh(host.device(), &female.mesh_without_hair()?)?
            } else {
                scene.upload_mesh(host.device(), &female.mesh_without_hair()?)?
            }
        } else if std::env::var_os("VOXY_ASYNC_FULL").is_some() {
            let (body, hair) = voxy_demos::full_model_worker::partition_hair(&mesh)
                .map_err(std::io::Error::other)?;
            let mut partitions = if use_gpu_surface_light {
                scene.upload_compute_shared_mesh_partitions(host.device(), &mesh)?
            } else {
                scene.upload_shared_mesh_partitions(host.device(), &mesh)?
            };
            let mut hair_owner = partitions.pop().unwrap();
            let mut body_owner = partitions.pop().unwrap();
            body_owner.update_index_partition(host.queue(), &body)?;
            hair_owner.update_index_partition(host.queue(), &hair)?;
            pollster::block_on(scene.set_geometry_opaque_shader(
                host.device(),
                &mut hair_owner,
                voxy_demos::FEMALE_HAIR_MATERIAL_SHADER,
            ))?;
            hair_geometry = Some(hair_owner);
            body_owner
        } else if use_gpu_secondary {
            scene.upload_mesh(host.device(), &SceneMesh::quad([1.; 4]))?
        } else if self.liquids.is_some() || self.wear.is_some() || self.fem.is_some() {
            let mut geometry = scene.reserve_geometry(host.device(), 60_024, 60_024)?;
            geometry.update(host.queue(), &mesh)?;
            geometry
        } else if use_gpu_surface_light || use_gpu_body {
            scene.upload_compute_mesh(host.device(), &mesh)?
        } else {
            scene.upload_mesh(host.device(), &mesh)?
        };
        let gpu_body = if use_gpu_body {
            let demo = self.female.as_ref().ok_or("GPU body requires character")?;
            let separated = if use_gpu_hair {
                Some(demo.mesh_without_hair()?)
            } else {
                None
            };
            let body = voxy_demos::female_demo::resident_body::ResidentBody::new(
                host.device(),
                host.queue(),
                &scene,
                demo,
                separated.as_ref().unwrap_or(&mesh),
                &cube,
            )?;
            eprintln!(
                "FULL MODEL GPU BODY initialized: resident rig + shell; CPU face details and physics retained"
            );
            Some(body)
        } else {
            None
        };
        let (gpu_secondary, gpu_secondary_reference) = if use_gpu_secondary {
            let female = self
                .female
                .as_ref()
                .ok_or("GPU secondary mode requires the body preview")?;
            let (offsets, weights, count) = female.gpu_secondary_binding(mesh.vertices().len())?;
            eprintln!(
                "GPU SECONDARY preview: {} vertices; neutral initial facial pose and baked initial irradiance; same nonlinear tissue simulation",
                mesh.vertices().len()
            );
            let mut gpu = scene.prepare_surface_deformation(
                host.device(),
                &mesh,
                &offsets,
                &weights,
                count,
            )?;
            gpu.set_deforming_normal_prefix(female.gpu_secondary_body_vertices())?;
            let palette = female.gpu_jump_palette();
            gpu.enable_rigid_skinning(
                host.device(),
                mesh.authored_normals()
                    .ok_or("jump GPU source lacks normals")?,
                &female.gpu_secondary_jump_weights(mesh.vertices().len()),
                palette.len() as u32,
                Some(3),
            )?;
            gpu.upload_rigid_pose(host.queue(), &palette)?;
            let reference = female.gpu_secondary_controls();
            gpu.update_controls(host.queue(), &vec![[0.; 4]; reference.len()])?;
            (Some(gpu), reference)
        } else {
            (None, Vec::new())
        };
        let gpu_surface_light = if use_gpu_surface_light {
            let female = self
                .female
                .as_ref()
                .ok_or("GPU surface lighting requires a character")?;
            let count = female.gpu_secondary_body_vertices() as usize;
            let triangles: Vec<_> = mesh
                .indices()
                .chunks_exact(3)
                .filter(|t| t.iter().all(|i| (*i as usize) < count))
                .map(|t| [t[0], t[1], t[2]])
                .collect();
            let settings = voxy_render::SurfaceLightFrameSettings {
                key_direction: voxy_demos::female_transmission::key_direction().to_array(),
                key_weight: 0.6,
                fill_direction: [0.8, 0.2, 0.5],
                fill_weight: 0.2,
                ambient: 0.2,
                albedo: [0.72, 0.46, 0.34, 1.],
                surface_fraction: female
                    .face_parameters
                    .value("skin_surface_light")
                    .unwrap_or(0.35),
                radii: [0.002, 0.001, 0.0005],
            };
            let target = gpu_secondary.as_ref().map_or(&cube, |gpu| gpu.geometry());
            let light =
                pollster::block_on(voxy_render::SurfaceLightFrame::new_with_reference_normals(
                    host.device(),
                    target,
                    &triangles,
                    settings,
                    &female.surface_light_normals(),
                ))?;
            eprintln!(
                "GPU SURFACE LIGHT initialized vertices={count} triangles={} CPU_authored_normals=true CPU_reference_retained={}",
                triangles.len(),
                std::env::var_os("VOXY_GPU_SURFACE_LIGHT_CPU_REFERENCE").is_some()
            );
            Some(light)
        } else {
            None
        };
        let gpu_surface_colors = if use_gpu_surface_light && !inherited_colors.is_empty() {
            let female = self
                .female
                .as_ref()
                .ok_or("missing inherited-color source model")?;
            let transfer = pollster::block_on(voxy_render::SurfaceColorTransfer::new(
                host.device(),
                &cube,
                female.gpu_secondary_body_vertices(),
                &inherited_colors,
            ))?;
            eprintln!(
                "GPU SURFACE COLORS inherited_bindings={}",
                inherited_colors.len()
            );
            Some(transfer)
        } else {
            None
        };
        let gpu_surface_light_audit = if gpu_surface_light.is_some()
            && std::env::var_os("VOXY_GPU_SURFACE_LIGHT_AUDIT").is_some()
        {
            Some(SurfaceLightAudit {
                buffer: host.device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some("surface light tail audit"),
                    size: 32,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                pending: None,
            })
        } else {
            None
        };
        let rush_floor = self
            .rush
            .as_ref()
            .map(|_| scene.upload_mesh(host.device(), &rush_box_mesh([0.15, 0.19, 0.25, 1.0])?))
            .transpose()?;
        let rush_crate = self
            .rush
            .as_ref()
            .map(|_| scene.upload_mesh(host.device(), &rush_box_mesh([0.95, 0.55, 0.15, 1.0])?))
            .transpose()?;
        let internal = self
            .xray
            .as_ref()
            .map(|xray| scene.upload_mesh(host.device(), &xray.internal()?))
            .transpose()?;
        let overlay_mesh = if let Some(female) = &self.female {
            voxy_demos::diagnostic_legend::mesh(&female.diagnostic_legend_labels())?
        } else {
            SceneMesh::quad([1.0, 1.0, 1.0, 0.8])
        };
        let overlay = scene.upload_mesh(host.device(), &overlay_mesh)?;
        let texture = scene.upload_texture(
            host.device(),
            host.queue(),
            2,
            2,
            &[
                255, 180, 50, 255, 50, 180, 255, 255, 50, 180, 255, 255, 255, 180, 50, 255,
            ],
        )?;
        let texture = if let Some(female) = &self.female {
            scene.upload_texture_with_sampling(
                host.device(),
                host.queue(),
                voxy_demos::female_complexion::WIDTH,
                voxy_demos::female_complexion::SIZE,
                &voxy_demos::female_complexion::atlas_for_parameters(&female.face_parameters),
                voxy_render::TextureSampling {
                    min_filter: voxy_render::TextureFilter::Linear,
                    mag_filter: voxy_render::TextureFilter::Linear,
                    ..Default::default()
                },
            )?
        } else if self.liquids.is_some()
            || self.wear.is_some()
            || self.fem.is_some()
            || self.rush.is_some()
            || self.xray.is_some()
            || self.strands.is_some()
            || self.gravity.is_some()
            || self.tissues.is_some()
        {
            scene.upload_texture(host.device(), host.queue(), 1, 1, &[255; 4])?
        } else {
            texture
        };
        let world_transform = scene.create_transform(host.device(), Mat4::IDENTITY)?;
        let overlay_transform = scene.create_transform(host.device(), Mat4::IDENTITY)?;
        println!("SCENE GPU: {:?}", host.adapter_info());
        // Fresh resources (first start or GPU recovery): forget the cached title so it is re-sent.
        self.last_title.clear();
        if self.liquids.is_some() || self.wear.is_some() || self.fem.is_some() {
            window.set_title(
                if self.fem.is_some() { "Voxy FEM | dynamic fracture | Space: pause | R: reset | Esc: exit" } else if self.wear.is_some() { "Voxy wear | receding solid and emitted grains | Space: pause | R: reset | Esc: exit" } else if self.liquid_optics { "Voxy liquid optics | water (left), oil (right) | Space: pause | R: reset | Esc: exit" } else { "Voxy liquids | water (blue), oil (gold) | Space: pause | R: reset | Esc: exit" },
            );
        }
        if self.strands.is_some() {
            window.set_title("Voxy grass & hair | Space: pause | Esc: exit");
        }
        if let Some(gravity) = &self.gravity {
            set_title_if_changed(&window, &mut self.last_title, gravity.title());
        }
        if self
            .tissues
            .as_ref()
            .is_some_and(voxy_demos::tissue_demo::TissueDemo::is_body)
        {
            window.set_title(
                "Voxy skeleton + soft tissues | front / rear | walk, jump, rest | Space: pause",
            );
        } else if self.tissues.is_some() {
            window.set_title(
                "Voxy tissues: skin | buttock | breast | lip | sphincter | penis — Space: pause",
            );
        }
        if let Some(female) = &self.female {
            let title = if gpu_secondary.is_some() {
                format!(
                    "GPU preview | neutral face / initial irradiance | {}",
                    female.title()
                )
            } else {
                female.title()
            };
            self.female_window_title = if self.paused {
                format!("Paused | {title}")
            } else {
                format!("Playing | {title}")
            };
            window.set_title(&self.female_window_title);
        }
        self.motion.reset();
        self.resources = Some(Resources {
            host,
            liquid_renderer: None,
            scene,
            cube,
            hair_geometry,
            gpu_hair,
            gpu_secondary,
            gpu_body,
            gpu_surface_light,
            gpu_surface_colors,
            gpu_surface_light_audit,
            gpu_secondary_last_controls: vec![[0.; 4]; gpu_secondary_reference.len()],
            gpu_jump_last_palette: Vec::new(),
            gpu_secondary_reference,
            film_layer: None,
            optical_frames: 0,
            film_mass_checks: 0,
            film_mass_max_relative_error: 0.,
            overlay,
            legend_labels: self
                .female
                .as_ref()
                .map(voxy_demos::female_demo::FemaleDemo::diagnostic_legend_labels)
                .unwrap_or_default(),
            internal,
            texture,
            material_creases: self
                .female
                .as_ref()
                .map(|f| f.face_parameters.material_signature())
                .unwrap_or_default(),
            atlas_update: Default::default(),
            world_transform,
            last_view_position: None,
            last_world_motion: None,
            last_overlay_mvp: None,
            overlay_transform,
            motion_frames: 0,
            rush_floor,
            rush_crate,
            rush_panel: None,
            script_transforms: std::collections::HashMap::new(),
        });
        window.set_visible(true);
        if self.rush.is_some()
            || self.smoke
            || self.xray.is_some()
            || self.female.is_some()
            || self.strands.is_some()
            || self.gravity.is_some()
        {
            window.focus_window();
        }
        self.window = Some(window);
        if self.full_model_worker.is_none() && std::env::var_os("VOXY_ASYNC_FULL").is_some() {
            let simulation = self
                .female
                .take()
                .ok_or("background full model requires female example")?;
            let view = simulation.background_view_replica()?;
            let controls = voxy_demos::full_model_worker::Controls::capture(&view, self.paused);
            self.full_model_worker = Some(voxy_demos::full_model_worker::FullModelWorker::new(
                simulation,
                controls,
                self.resources
                    .as_ref()
                    .is_some_and(|r| r.gpu_hair.is_some()),
                self.hair_solver.take(),
            )?);
            self.female = Some(view);
        }
        self.last = Instant::now();
        Ok(())
    }
    pub(super) fn recreate_gpu(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.recovering_gpu = true;
        self.resources = None;
        self.motion.reset();
        self.frame_loop.reset();
        self.last = Instant::now();
        self.initialize(event_loop)?;
        eprintln!("SCENE GPU RECOVERED: resources recreated, history reset");
        Ok(())
    }
}

pub(super) fn rush_box_mesh(color: [f32; 4]) -> Result<SceneMesh, voxy_render::SceneError> {
    let cube = cube_mesh()?;
    let vertices = cube
        .vertices()
        .iter()
        .map(|vertex| {
            let mut v = *vertex;
            let shade = 0.65
                + if v.position[1] > 0.0 { 0.25 } else { 0.0 }
                + if v.position[0] < 0.0 { 0.1 } else { 0.0 };
            v.position = v.position.map(|coordinate| coordinate * 0.5);
            v.color = [
                color[0] * shade,
                color[1] * shade,
                color[2] * shade,
                color[3],
            ];
            v
        })
        .collect();
    SceneMesh::new(vertices, cube.indices().to_vec())
}

pub(super) fn cube_mesh() -> Result<SceneMesh, voxy_render::SceneError> {
    let positions = [
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let vertices = positions
        .into_iter()
        .enumerate()
        .map(|(i, position)| SceneVertex {
            position,
            uv: [
                if i % 2 == 0 { 0.0 } else { 1.0 },
                if i % 4 < 2 { 1.0 } else { 0.0 },
            ],
            color: [1.0; 4],
        })
        .collect();
    SceneMesh::new(
        vertices,
        vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 3, 7, 6, 3, 6, 2, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    )
}
