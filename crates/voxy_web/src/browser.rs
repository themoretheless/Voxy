use glam::{Mat4, Quat, Vec3};
use voxy_render::{
    SceneCamera, SceneDraw, SceneGeometry, SceneMesh, SceneProjection, SceneRenderer, SceneTexture,
    SceneTransform, SceneVertex,
};
use wasm_bindgen::prelude::*;

pub(crate) fn error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[derive(Debug)]
struct BrowserGravity {
    job: voxy_gpu::GravityJob,
    drawing: voxy_gpu::GravityView,
    seconds: f32,
    accumulator: f32,
}

impl BrowserGravity {
    fn encode(
        &mut self,
        seconds: f32,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), JsValue> {
        self.accumulator += (seconds - self.seconds).clamp(0.0, 0.1);
        self.seconds = seconds;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (self.accumulator * 240.0).floor().min(24.0) as u32;
        if steps > 0 {
            self.job.encode_steps(encoder, steps).map_err(error)?;
            #[allow(clippy::cast_precision_loss)]
            {
                self.accumulator = (self.accumulator - steps as f32 / 240.0).max(0.0);
            }
        }
        self.drawing.encode(encoder, view);
        Ok(())
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct WebEngine {
    canvas: web_sys::HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    scene: SceneRenderer,
    mesh: SceneGeometry,
    overlay: SceneGeometry,
    texture: SceneTexture,
    transform: SceneTransform,
    overlay_transform: SceneTransform,
    depth: wgpu::TextureView,
    backend: String,
    terrain: Option<voxy_gpu::TerrainProgram>,
    gravity: Option<BrowserGravity>,
    terrain_cpu: voxy_world::ProceduralTerrainGenerator,
    voxel_world: Option<voxy_runtime::BootstrapScene>,
    voxel_epoch: u64,
    voxel_water: Option<super::voxel_water::BrowserWater>,
    voxel_game: Option<super::voxel_game::VoxelGame>,
    animated: Option<voxy_render::SkinnedMotionHistory>,
    animated_presentations: u32,
    animated_hdr: Option<super::animated_scene::HdrScene>,
    animated_light_intensity: f32,
    animated_shadows: bool,
    animated_shadow_filter: voxy_render::ShadowFilter,
    pause_started: Option<f64>,
    device_failure: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

#[wasm_bindgen]
impl WebEngine {
    /// Creates a strict WebGPU or WebGL renderer, or automatically detects WebGPU.
    /// # Errors
    /// Returns initialization diagnostics for unavailable APIs or invalid GPU resources.
    #[allow(clippy::too_many_lines)]
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        backend: String,
    ) -> Result<WebEngine, JsValue> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = match backend.as_str() {
            "webgl" => wgpu::Backends::GL,
            "webgpu" => wgpu::Backends::BROWSER_WEBGPU,
            "auto" => wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            _ => return Err(error("expected auto, webgl or webgpu backend")),
        };
        let instance = if backend == "auto" {
            wgpu::util::new_instance_with_webgpu_detection(descriptor).await
        } else {
            wgpu::Instance::new(descriptor)
        };
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(error)?;
        let backend = format!("{:?}", adapter.get_info().backend);
        let limits = if adapter.get_info().backend == wgpu::Backend::BrowserWebGpu {
            wgpu::Limits::default().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: limits,
                ..Default::default()
            })
            .await
            .map_err(error)?;
        let device_failure = std::sync::Arc::new(std::sync::Mutex::new(None));
        let failure_callback = device_failure.clone();
        device.set_device_lost_callback(move |reason, message| {
            if let Ok(mut failure) = failure_callback.lock() {
                *failure = Some(format!("GPU device lost ({reason:?}): {message}"));
            }
        });
        let mut config = surface
            .get_default_config(&adapter, canvas.width().max(1), canvas.height().max(1))
            .ok_or_else(|| error("surface has no supported format"))?;
        let render_format = config.format.add_srgb_suffix();
        if render_format != config.format {
            config.view_formats = vec![render_format];
        }
        surface.configure(&device, &config);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let scene = SceneRenderer::new(&device, render_format);
        let vertices = [
            [-1.0, -0.8, 0.0],
            [1.0, -0.8, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.2],
        ]
        .into_iter()
        .enumerate()
        .map(|(index, position)| SceneVertex {
            position,
            uv: match index {
                0 => [0.0, 1.0],
                1 => [1.0, 1.0],
                2 => [0.5, 0.0],
                _ => [0.5, 0.5],
            },
            color: match index {
                0 => [1.0, 0.3, 0.15, 1.0],
                1 => [0.2, 0.8, 1.0, 1.0],
                2 => [0.4, 1.0, 0.3, 1.0],
                _ => [1.0, 0.8, 0.2, 1.0],
            },
        })
        .collect();
        let mesh = scene
            .upload_mesh(
                &device,
                &SceneMesh::new(vertices, vec![0, 1, 2, 0, 3, 1, 1, 3, 2, 2, 3, 0])
                    .map_err(error)?,
            )
            .map_err(error)?;
        let overlay = scene
            .upload_mesh(&device, &SceneMesh::quad([0.2, 0.7, 1.0, 0.65]))
            .map_err(error)?;
        let material_image = voxy_render::ImageAsset::decode(
            include_bytes!("../assets/tint.png"),
            voxy_render::ImageLimits::default(),
        )
        .map_err(error)?;
        let texture = scene
            .upload_image_mips(
                &device,
                &queue,
                &material_image.mip_chain(),
                voxy_render::TextureSampling {
                    min_filter: voxy_render::TextureFilter::Linear,
                    mag_filter: voxy_render::TextureFilter::Linear,
                    anisotropy: 4,
                    ..voxy_render::TextureSampling::default()
                },
            )
            .map_err(error)?;
        let transform = scene
            .create_transform(&device, Mat4::IDENTITY)
            .map_err(error)?;
        let overlay_transform = scene
            .create_transform(&device, Mat4::IDENTITY)
            .map_err(error)?;
        let depth = create_depth(&device, config.width, config.height);
        if let Some(validation) = scope.pop().await {
            return Err(error(validation));
        }
        let (terrain_palette, terrain_water) = terrain_palette().map_err(error)?;
        let terrain_cpu =
            voxy_world::ProceduralTerrainGenerator::new(terrain_palette, terrain_water);
        let terrain = if device.limits().max_compute_workgroups_per_dimension > 0 {
            Some(
                voxy_gpu::TerrainProgram::new(&device, terrain_palette, terrain_water)
                    .await
                    .map_err(error)?,
            )
        } else {
            None
        };
        Ok(Self {
            canvas,
            surface,
            device,
            queue,
            config,
            scene,
            mesh,
            overlay,
            texture,
            transform,
            overlay_transform,
            depth,
            backend,
            terrain,
            gravity: None,
            terrain_cpu,
            voxel_world: None,
            voxel_epoch: 0,
            voxel_water: None,
            voxel_game: None,
            animated: None,
            animated_presentations: 0,
            animated_hdr: None,
            animated_light_intensity: 35.0,
            animated_shadows: true,
            animated_shadow_filter: voxy_render::ShadowFilter::Pcf3x3,
            pause_started: None,
            device_failure,
        })
    }

    /// Start the same two-joint deforming quad used by the native ray demo.
    /// This raster path works on WebGPU and WebGL; hardware ray queries are absent.
    /// # Errors
    /// Rejects conflicts with voxel/gravity demos or invalid/uploaded geometry.
    pub async fn start_animated_scene(&mut self) -> Result<(), JsValue> {
        if self.voxel_world.is_some() || self.gravity.is_some() {
            return Err(error("animated scene requires a separate demo session"));
        }
        let mesh = super::animated_scene::mesh().map_err(error)?;
        let history = voxy_render::SkinnedMotionHistory::new(mesh);
        let pose = history
            .prepare_frame(&[Mat4::IDENTITY; 2], Mat4::IDENTITY, [0.8, 0.4, 0.2, 1.0])
            .map_err(error)?;
        self.mesh = self
            .scene
            .upload_mesh(&self.device, pose.scene())
            .map_err(error)?;
        self.animated_hdr = if self.backend == "BrowserWebGpu" {
            Some(
                super::animated_scene::HdrScene::new(
                    &self.device,
                    &self.queue,
                    self.config.format.add_srgb_suffix(),
                    self.config.width,
                    self.config.height,
                )
                .await
                .map_err(error)?,
            )
        } else {
            None
        };
        self.animated = Some(history);
        self.animated_presentations = 0;
        Ok(())
    }
    /// Enable deformation-aware primary temporal guides for the animated WebGPU scene.
    /// Includes clipped HDR temporal accumulation before tone mapping.
    /// # Errors
    /// Requires an active animated HDR scene.
    pub fn set_animated_temporal_guides(&mut self, enabled: bool) -> Result<(), JsValue> {
        self.animated_hdr.as_mut().ok_or_else(|| error("temporal guides require animated WebGPU"))?
            .enable_temporal_guides(enabled);
        Ok(())
    }
    /// Load a bounded Radiance HDR panorama into the animated WebGPU scene.
    /// The caller must suspend frame calls until this async operation completes.
    pub async fn load_hdr_environment(&mut self, bytes: Vec<u8>) -> Result<(), JsValue> {
        self.animated_hdr.as_mut().ok_or_else(|| error("HDR environment requires animated WebGPU scene"))?
            .load_environment(&self.device,&self.queue,&bytes).await.map_err(error)
    }

    /// Return a completed imported-environment HDR center-pixel diagnostic once.
    pub fn take_hdr_probe(&self) -> Result<Vec<f32>,JsValue> {
        self.animated_hdr.as_ref().and_then(|hdr|hdr.probe.as_ref()).and_then(voxy_render::HdrPixelProbe::take_result)
            .transpose().map(|value|value.map_or_else(Vec::new,|v|v.to_vec())).map_err(error)
    }
    /// Whether the animated scene uses a floating-point HDR intermediate.
    #[must_use]
    pub fn animated_hdr(&self) -> bool {
        self.animated_hdr.is_some()
    }
    /// Change animated HDR exposure without recompiling the display pipeline.
    /// # Errors
    /// Rejects unavailable HDR mode or nonfinite/nonpositive exposure.
    pub fn set_animated_exposure(&mut self, exposure: f32) -> Result<(), JsValue> {
        self.animated_hdr
            .as_mut()
            .ok_or_else(|| error("HDR unavailable in this scene/backend"))?
            .exposure(&self.device, exposure)
            .map_err(error)
    }
    /// Poll numerical checks of production motion/depth at sampled scene pixels.
    /// # Errors
    /// Reports GPU readback or CPU reference mismatches.
    pub fn temporal_guide_checks(&mut self) -> Result<u32, JsValue> {
        self.animated_hdr.as_mut().map_or(Ok(0), |hdr| hdr.poll_guide_checks())
    }
    /// Number of sampled moving surfaces checked against CPU reprojection.
    pub fn temporal_moving_checks(&self) -> u32 {
        self.animated_hdr.as_ref().map_or(0, |hdr| hdr.guide_moving)
    }
    /// Production HDR channels checked and sampled depth-based history rejections.
    pub fn temporal_color_checks(&self) -> Vec<u32> {
        self.animated_hdr.as_ref().map_or_else(|| vec![0,0,0], |hdr| vec![hdr.color_checks,hdr.depth_rejections,hdr.color_blends])
    }
    /// Retrieve the first temporal HDR output diagnostic pixel once mapping completes.
    /// # Errors
    /// Returns asynchronous readback errors.
    pub fn take_temporal_probe(&self) -> Result<Vec<f32>, JsValue> {
        self.animated_hdr.as_ref().and_then(|hdr| hdr.temporal_probe.as_ref())
            .and_then(voxy_render::HdrPixelProbe::take_result)
            .transpose().map(|value| value.map_or_else(Vec::new, |pixel| pixel.to_vec())).map_err(error)
    }
    /// Set the animated point light's scalar radiant intensity.
    /// # Errors
    /// Rejects missing HDR mode, nonfinite values or intensity outside [0,100].
    pub fn set_animated_light(&mut self, intensity: f32) -> Result<(), JsValue> {
        if self.animated_hdr.is_none()
            || !intensity.is_finite()
            || !(0.0..=100.0).contains(&intensity)
        {
            return Err(error(
                "animated light requires HDR and intensity in [0,100]",
            ));
        }
        if let Some(hdr) = &mut self.animated_hdr { hdr.invalidate_temporal(); }
        self.animated_light_intensity = intensity;
        Ok(())
    }
    /// Scale animated HDR environment lighting in the main and reflection passes.
    /// # Errors
    /// Requires HDR and finite nonnegative intensity.
    pub fn set_animated_environment_intensity(&mut self, intensity: f32) -> Result<(), JsValue> {
        self.animated_hdr.as_mut().ok_or_else(||error("HDR unavailable in this scene/backend"))?
            .environment_intensity(&self.queue,intensity).map_err(error)
    }
    /// Toggle projective raster shadow visibility without replacing resources.
    /// # Errors
    /// Rejects a scene/backend without the HDR shadow renderer.
    pub fn set_animated_shadows(&mut self, enabled: bool) -> Result<(), JsValue> {
        if self.animated_hdr.is_none() {
            return Err(error("shadow rendering unavailable"));
        }
        if let Some(hdr) = &mut self.animated_hdr { hdr.invalidate_temporal(); }
        self.animated_shadows = enabled;
        Ok(())
    }
    /// Select hard shadows (0), 3x3 PCF (1), or 5x5 PCF (2).
    /// # Errors
    /// Rejects unavailable shadow rendering or unknown kernel identifiers.
    pub fn set_animated_shadow_filter(&mut self, filter: u32) -> Result<(), JsValue> {
        if self.animated_hdr.is_none() {
            return Err(error("shadow rendering unavailable"));
        }
        self.animated_shadow_filter = match filter {
            0 => voxy_render::ShadowFilter::Hard,
            1 => voxy_render::ShadowFilter::Pcf3x3,
            2 => voxy_render::ShadowFilter::Pcf5x5,
            _ => return Err(error("unknown shadow filter")),
        };
        if let Some(hdr) = &mut self.animated_hdr { hdr.invalidate_temporal(); }
        Ok(())
    }
    /// Set the animated mesh's GGX roughness and metallic weight.
    /// # Errors
    /// Requires the HDR scene and finite parameters in [0,1].
    pub fn set_animated_material(&mut self, roughness: f32, metallic: f32) -> Result<(), JsValue> {
        if !roughness.is_finite()
            || !metallic.is_finite()
            || !(0.0..=1.0).contains(&roughness)
            || !(0.0..=1.0).contains(&metallic)
        {
            return Err(error("material parameters must be finite and in [0,1]"));
        }
        let hdr = self
            .animated_hdr
            .as_mut()
            .ok_or_else(|| error("HDR material unavailable"))?;
        hdr.material = [roughness, metallic];
        hdr.invalidate_temporal();
        Ok(())
    }
    /// Set the planar receiver material independently from the animated mesh.
    /// # Errors
    /// Requires HDR and finite parameters in [0,1].
    pub fn set_animated_reflector_material(&mut self, roughness: f32, metallic: f32) -> Result<(), JsValue> {
        if !roughness.is_finite() || !metallic.is_finite()
            || !(0.0..=1.0).contains(&roughness) || !(0.0..=1.0).contains(&metallic) {
            return Err(error("reflector parameters must be finite and in [0,1]"));
        }
        let hdr = self.animated_hdr.as_mut().ok_or_else(||error("HDR reflector unavailable"))?;
        hdr.reflector_material = [roughness,metallic];
        hdr.invalidate_temporal();
        Ok(())
    }
    /// Number of animated poses committed after presentation, excluding skipped frames.
    #[must_use]
    pub fn animated_presentations(&self) -> u32 {
        self.animated_presentations
    }

    /// Displays a generated voxel chunk using its actual greedy mesh.
    /// # Errors
    /// Reports world generation, mesh conversion and GPU upload errors.
    pub async fn load_voxel_scene(&mut self) -> Result<u32, JsValue> {
        let world = self.build_voxel_world().await?;
        let count = world
            .chunks
            .iter()
            .map(|chunk| chunk.mesh.quad_count())
            .sum::<usize>();
        let mesh = super::voxel_scene::mesh(&world).map_err(error)?;
        let geometry = self.scene.upload_mesh(&self.device, &mesh).map_err(error)?;
        let game = super::voxel_game::VoxelGame::new(&self.device).await?;
        if self.voxel_world.is_none() {
            self.overlay = std::mem::replace(&mut self.mesh, geometry);
        } else {
            self.mesh = geometry;
        }
        self.voxel_game = Some(game);
        self.voxel_water = None;
        self.voxel_world = Some(world);
        self.voxel_epoch = 1;
        u32::try_from(count).map_err(error)
    }

    /// Advances one fixed character tick, returning false while GPU contacts are pending.
    /// # Errors
    /// Rejects invalid input, unavailable worlds and controller/device failures.
    pub fn step_voxel_character(&mut self, x: f64, z: f64, jump: bool) -> Result<bool, JsValue> {
        if !x.is_finite() || !z.is_finite() || x.abs() > 1.0 || z.abs() > 1.0 {
            return Err(error("invalid browser movement"));
        }
        let world = self
            .voxel_world
            .as_ref()
            .ok_or_else(|| error("load voxel world first"))?;
        let game = self
            .voxel_game
            .as_mut()
            .ok_or_else(|| error("load voxel character first"))?;
        game.step(
            &self.queue,
            &world.world,
            physics::CharacterInput {
                planar_velocity: {
                    let scale = 5.0 / x.hypot(z).max(1.0);
                    [x * scale, z * scale]
                },
                jump_pressed: jump,
            },
        )
    }

    /// Enables asynchronous GPU water on the retained gameplay world.
    /// # Errors
    /// Rejects WebGL, missing worlds, registry or device failures.
    pub async fn start_voxel_water(&mut self) -> Result<(), JsValue> {
        let world = self
            .voxel_world
            .as_ref()
            .ok_or_else(|| error("load voxel world first"))?;
        self.voxel_water =
            Some(super::voxel_water::BrowserWater::new(&self.device, &world.world).await?);
        Ok(())
    }

    /// Excludes an explicit gameplay pause from pending GPU task deadlines.
    /// Returns the number of pending deadlines adjusted or retained on pause.
    pub fn set_voxel_paused(&mut self, paused: bool) -> u32 {
        let milliseconds = if paused {
            self.pause_started.get_or_insert_with(js_sys::Date::now);
            0.0
        } else {
            self.pause_started
                .take()
                .map_or(0.0, |started| (js_sys::Date::now() - started).max(0.0))
        };
        let mut pending = 0;
        if let Some(game) = &mut self.voxel_game {
            pending += game.extend_pause(milliseconds);
        }
        if let Some(water) = &mut self.voxel_water {
            pending += water.extend_pause(milliseconds);
        }
        pending
    }

    /// Seeds a rendered boundary demo using the real gameplay world and loader.
    /// # Errors
    /// Reports absent simulation/world or an invalid fixture transaction.
    pub fn start_water_boundary_demo(&mut self) -> Result<(), JsValue> {
        let scene = self
            .voxel_world
            .as_mut()
            .ok_or_else(|| error("load voxel world first"))?;
        self.voxel_water
            .as_mut()
            .ok_or_else(|| error("start gameplay water first"))?
            .seed_boundary(&mut scene.world)
    }
    /// Number of neighbors streamed by gameplay water.
    pub fn water_streamed_chunks(&self) -> u32 {
        self.voxel_water
            .as_ref()
            .map_or(0, super::voxel_water::BrowserWater::loaded_chunks)
    }
    /// Number of voxel chunks in the published draw mesh.
    pub fn visible_voxel_chunks(&self) -> usize {
        self.voxel_world
            .as_ref()
            .map_or(0, |scene| scene.chunks.len())
    }

    /// Pours one full water cell above the center of the displayed chunk.
    /// # Errors
    /// Rejects absent GPU simulation, occupied targets and edit/mesh failures.
    pub fn pour_voxel_water(&mut self) -> Result<(), JsValue> {
        use voxy_world::{EditSource, EditTxn, ResourceKey, Sample, VoxelView, VoxelWrite};
        let epoch = self
            .voxel_epoch
            .checked_add(1)
            .ok_or_else(|| error("voxel epoch overflow"))?;
        let scene = self
            .voxel_world
            .as_mut()
            .ok_or_else(|| error("load voxel world first"))?;
        let simulation = self
            .voxel_water
            .as_mut()
            .ok_or_else(|| error("start gameplay water first"))?;
        let pos = voxy_world::VoxelPos {
            x: 16,
            y: 18,
            z: 16,
        };
        let Sample::Loaded(block) = scene.world.sample(pos) else {
            return Err(error("water target unavailable"));
        };
        if block != voxy_world::BlockStateId::AIR {
            return Err(error("water target occupied"));
        }
        let water = scene
            .world
            .registry()
            .find(&ResourceKey::parse("voxy:water_8").map_err(error)?)
            .ok_or_else(|| error("missing water state"))?;
        scene
            .world
            .commit(EditTxn {
                source: EditSource::Player(1),
                expected: vec![],
                writes: vec![VoxelWrite { pos, block: water }],
            })
            .map_err(error)?;
        simulation.wake(&scene.world);
        self.voxel_epoch = epoch;
        scene.chunks = voxy_runtime::rebuild_bootstrap_chunks(
            &scene.world,
            &scene
                .chunks
                .iter()
                .map(|chunk| chunk.pos)
                .collect::<Vec<_>>(),
            epoch,
        )
        .map_err(error)?;
        self.mesh = self
            .scene
            .upload_mesh(
                &self.device,
                &super::voxel_scene::mesh(scene).map_err(error)?,
            )
            .map_err(error)?;
        Ok(())
    }

    /// Polls one GPU water transaction and rebuilds the displayed world after commit.
    /// Returns -1 while pending, 0 when settled, and 1 after a committed step.
    /// # Errors
    /// Reports absent simulation, overflow, GPU, world or mesh failures.
    pub fn poll_voxel_water(&mut self) -> Result<i32, JsValue> {
        let protected = self
            .voxel_game
            .as_ref()
            .map(super::voxel_game::VoxelGame::protected_chunks)
            .transpose()?
            .unwrap_or_default();
        let epoch = self
            .voxel_epoch
            .checked_add(1)
            .ok_or_else(|| error("voxel epoch overflow"))?;
        let scene = self
            .voxel_world
            .as_mut()
            .ok_or_else(|| error("load voxel world first"))?;
        let simulation = self
            .voxel_water
            .as_mut()
            .ok_or_else(|| error("start gameplay water first"))?;
        simulation.protect_chunks(protected);
        let result = simulation.poll(
            &self.queue,
            &self.device,
            self.terrain
                .as_ref()
                .ok_or_else(|| error("terrain compute unsupported"))?,
            &mut scene.world,
        )?;
        let status = match result {
            None => -1,
            Some(false) => 0,
            Some(true) => 1,
        };
        if !simulation.needs_mesh_refresh() {
            return Ok(status);
        }
        self.voxel_epoch = epoch;
        scene.chunks = voxy_runtime::rebuild_bootstrap_chunks(
            &scene.world,
            &simulation.visible_chunks(),
            epoch,
        )
        .map_err(error)?;
        self.mesh = self
            .scene
            .upload_mesh(
                &self.device,
                &super::voxel_scene::mesh(scene).map_err(error)?,
            )
            .map_err(error)?;
        simulation.mark_mesh_refreshed();
        Ok(status)
    }

    /// Compares all 27 pristine scene chunks against the CPU terrain reference.
    /// # Errors
    /// Rejects an absent scene, modified terrain or any generation/material mismatch.
    pub async fn validate_voxel_world(&self) -> Result<u32, JsValue> {
        let scene = self
            .voxel_world
            .as_ref()
            .ok_or_else(|| error("load voxel world first"))?;
        super::voxel_terrain::validate_world(&scene.world).await
    }

    /// Switches between walking and driving at the current actor location.
    /// # Errors
    /// Reports an absent voxel character.
    pub fn toggle_voxel_vehicle(&mut self) -> Result<bool, JsValue> {
        Ok(self
            .voxel_game
            .as_mut()
            .ok_or_else(|| error("load voxel character first"))?
            .toggle_vehicle())
    }

    /// Advances one vehicle tick; false means GPU contacts are pending.
    /// # Errors
    /// Reports absent worlds/vehicles, invalid input or controller failures.
    pub fn step_voxel_vehicle(
        &mut self,
        throttle: f64,
        steering: f64,
        brake: bool,
    ) -> Result<bool, JsValue> {
        let world = self
            .voxel_world
            .as_ref()
            .ok_or_else(|| error("load voxel world first"))?;
        let game = self
            .voxel_game
            .as_mut()
            .ok_or_else(|| error("load voxel character first"))?;
        game.vehicle
            .as_mut()
            .ok_or_else(|| error("enter vehicle first"))?
            .step(
                game.program.as_ref(),
                &self.queue,
                &world.world,
                physics_voxel::VehicleInput {
                    throttle,
                    steering,
                    brake,
                },
            )
    }

    /// Returns the character center in local scene coordinates.
    /// # Errors
    /// Reports an absent character or coordinates outside the local render range.
    pub fn voxel_character_position(&self) -> Result<Vec<f32>, JsValue> {
        self.voxel_game
            .as_ref()
            .ok_or_else(|| error("load voxel character first"))?
            .position()
            .map(|position| position.to_array().to_vec())
    }

    /// Applies a real explosion to the retained voxel world and rebuilds its mesh.
    /// # Errors
    /// Reports absent worlds, edit/derivation/upload failures or epoch overflow.
    pub fn destroy_voxel_center(&mut self) -> Result<u32, JsValue> {
        let epoch = self
            .voxel_epoch
            .checked_add(1)
            .ok_or_else(|| error("voxel epoch overflow"))?;
        let world = self
            .voxel_world
            .as_mut()
            .ok_or_else(|| error("load voxel world first"))?;
        let changed = super::voxel_scene::destroy_center(&mut world.world).map_err(error)?;
        if let Some(simulation) = &mut self.voxel_water {
            simulation.wake(&world.world);
        }
        self.voxel_epoch = epoch;
        world.chunks = voxy_runtime::rebuild_bootstrap_chunks(
            &world.world,
            &world
                .chunks
                .iter()
                .map(|chunk| chunk.pos)
                .collect::<Vec<_>>(),
            epoch,
        )
        .map_err(error)?;
        let mesh = super::voxel_scene::mesh(world).map_err(error)?;
        self.mesh = self.scene.upload_mesh(&self.device, &mesh).map_err(error)?;
        Ok(changed)
    }

    /// Selects a resident WebGPU orbit drawn directly from its physics buffer.
    /// Repeated calls reset the orbit. Rendering time controls stepping and pause.
    /// # Errors
    /// WebGL has no compute path; shader/device failures are reported explicitly.
    pub async fn start_gravity(&mut self) -> Result<(), JsValue> {
        use voxy_gpu::{
            GravityBody, GravityBudget, GravityParameters, GravityProgram, GravityView,
        };
        if self.terrain.is_none() {
            return Err(error("gravity compute unsupported on WebGL"));
        }
        let program = GravityProgram::new(&self.device, GravityBudget::default())
            .await
            .map_err(error)?;
        let bodies = [
            GravityBody {
                mass: 1.0,
                position: [-0.4, 0.0, 0.0],
                velocity: [0.0, -0.56, 0.0],
            },
            GravityBody {
                mass: 1.0,
                position: [0.4, 0.0, 0.0],
                velocity: [0.0, 0.56, 0.0],
            },
        ];
        let job = program
            .create_job(
                &self.device,
                &bodies,
                GravityParameters {
                    constant: 0.5,
                    softening: 0.02,
                    uniform_acceleration: [0.0; 3],
                    dt: 1.0 / 240.0,
                },
            )
            .map_err(error)?;
        let drawing = GravityView::new(&self.device, &job, self.config.format.add_srgb_suffix())
            .await
            .map_err(error)?;
        self.gravity = Some(BrowserGravity {
            job,
            drawing,
            seconds: 0.0,
            accumulator: 0.0,
        });
        Ok(())
    }

    /// Validates asynchronous GPU-assisted sweeps against the exact CPU solver.
    /// # Errors
    /// Rejects WebGL, readback failures, stale-world acceptance and parity failures.
    pub async fn validate_collisions(&self) -> Result<u32, JsValue> {
        if self.terrain.is_none() {
            return Err(error("collision compute unsupported on WebGL"));
        }
        super::collision_validation::validate(&self.device, &self.queue).await
    }

    /// Checks gameplay GPU terrain loading and water retry against CPU state.
    /// # Errors
    /// Reports absent voxel scene/compute, loader failures or CPU mismatches.
    pub async fn validate_water_streaming(&self) -> Result<u32, JsValue> {
        let world = &self
            .voxel_world
            .as_ref()
            .ok_or_else(|| error("load voxel world first"))?
            .world;
        let terrain = self
            .terrain
            .as_ref()
            .ok_or_else(|| error("terrain compute unsupported"))?;
        super::voxel_water::validate_loading(&self.device, &self.queue, terrain, world).await
    }

    /// Numerically verify HDR temporal accumulation, depth rejection and reset.
    /// # Errors
    /// Requires WebGPU; reports readback timeout or numerical mismatch.
    pub async fn validate_temporal(&self) -> Result<u32, JsValue> {
        if self.backend != "BrowserWebGpu" { return Err(error("temporal validation requires WebGPU")); }
        super::temporal_validation::validate(&self.device, &self.queue).await
    }

    /// Verifies browser GPU water transactions and stale revision rejection.
    /// # Errors
    /// Rejects missing compute support, readback timeouts and CPU mismatches.
    pub async fn validate_water(&self) -> Result<u32, JsValue> {
        if self.terrain.is_none() {
            return Err(error("water compute unsupported on WebGL"));
        }
        super::water_validation::validate(&self.device, &self.queue).await
    }

    /// Rejects WebGL, readback failures and trajectory/mass mismatches.
    /// Checks 257 bodies over 128 resident steps against CPU f64, returning max error.
    /// # Errors
    /// Rejects WebGL, readback failures and trajectory/mass mismatches.
    pub async fn validate_gravity(&self) -> Result<f64, JsValue> {
        if self.terrain.is_none() {
            return Err(error("gravity compute unsupported on WebGL"));
        }
        crate::gravity_validation::validate(&self.device, &self.queue).await
    }

    /// Generates a chunk on WebGPU without blocking the browser event loop.
    /// Coordinates and seed use JS `BigInt`, retaining their full 64-bit ranges.
    /// Returned IDs: air=0, surface=1, soil=2, stone=3, water=4; index=x+32*(z+32*y).
    /// # Errors
    /// WebGL reports unsupported compute; also reports coordinate/GPU/readback errors.
    pub async fn generate_terrain(
        &self,
        x: i64,
        y: i64,
        z: i64,
        seed: u64,
    ) -> Result<Vec<u32>, JsValue> {
        let chunk = self
            .terrain_chunk(voxy_core::ChunkPos { x, y, z }, voxy_world::WorldSeed(seed))
            .await?;
        dense_ids(&chunk)
    }

    /// Checks the actual browser compute/readback path against the CPU generator.
    /// # Errors
    /// Returns unsupported compute, GPU failures, or a CPU/GPU mismatch.
    pub async fn validate_terrain(&self) -> Result<u32, JsValue> {
        use voxy_world::ChunkGenerator;
        let mut count = 0;
        for (pos, seed) in [
            (voxy_core::ChunkPos::default(), 42),
            (voxy_core::ChunkPos { x: -7, y: -1, z: 4 }, u64::MAX),
            (
                voxy_core::ChunkPos {
                    x: i64::MIN / 32,
                    y: 0,
                    z: i64::MAX / 32,
                },
                1 << 63,
            ),
        ] {
            let actual = self.terrain_chunk(pos, voxy_world::WorldSeed(seed)).await?;
            let expected = self
                .terrain_cpu
                .generate(
                    pos,
                    voxy_world::WorldSeed(seed),
                    &voxy_core::CancelToken::new(),
                )
                .map_err(error)?;
            if actual.data != expected.data {
                return Err(error("browser GPU/CPU terrain mismatch"));
            }
            count += 32768;
        }
        Ok(count)
    }

    #[must_use]
    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    /// Destroys the real device for explicit device-loss acceptance testing.
    /// Reload the page to recreate resources after this terminal diagnostic.
    pub fn destroy_device_for_validation(&self) {
        self.device.destroy();
    }

    /// Returns the driver's terminal device-loss diagnostic, if reported.
    pub fn device_failure(&self) -> Option<String> {
        self.device_failure
            .lock()
            .ok()
            .and_then(|failure| failure.clone())
    }

    /// Draws one animation frame; returns false for a temporary surface event.
    /// # Errors
    /// Returns diagnostics for invalid sizes, matrices or surface validation errors.
    #[allow(clippy::cast_precision_loss)]
    pub fn render(&mut self, seconds: f32, width: u32, height: u32) -> Result<bool, JsValue> {
        if let Some(failure) = self.device_failure() {
            return Err(error(failure));
        }
        if !seconds.is_finite() {
            return Err(error("invalid animation time"));
        }
        if width == 0 || height == 0 {
            return Ok(false);
        }
        if width > self.device.limits().max_texture_dimension_2d
            || height > self.device.limits().max_texture_dimension_2d
        {
            return Err(error("canvas exceeds device texture limits"));
        }
        if width != self.config.width || height != self.config.height {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth = create_depth(&self.device, width, height);
        }
        self.update_scene_camera(seconds, width, height)?;
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(error("surface validation failed"));
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if let Some(gravity) = &mut self.gravity {
            gravity.encode(seconds, &mut encoder, &view)?;
            self.queue.submit([encoder.finish()]);
            self.queue.present(frame);
            return Ok(true);
        }
        let animated_pose = self.prepare_animated_pose(seconds)?;
        if let Some(pose) = &animated_pose {
            self.mesh.update(&self.queue, pose.scene()).map_err(error)?;
        }
        let draws = [
            SceneDraw {
                geometry: &self.mesh,
                texture: &self.texture,
                transform: &self.transform,
                overlay: false,
            },
            SceneDraw {
                geometry: &self.overlay,
                texture: &self.texture,
                transform: &self.overlay_transform,
                overlay: self.voxel_world.is_none(),
            },
        ];
        encode_scene(
            &self.device,
            &self.scene,
            self.animated_hdr.as_mut(),
            &mut encoder,
            (&view, &self.depth),
            &draws,
            [width, height],
            animated_pose.as_ref(),
        )?;
        self.queue.submit([encoder.finish()]);
        if let Some(probe)=self.animated_hdr.as_mut().and_then(|hdr|hdr.probe.as_mut()) {probe.begin_read();}
        if let Some(probe) = self.animated_hdr.as_mut().and_then(|hdr| hdr.temporal_probe.as_mut()) { probe.begin_read(); }
        if let Some(hdr) = &mut self.animated_hdr { hdr.begin_guide_read(); }
        self.queue.present(frame);
        self.commit_animated_pose(animated_pose.as_ref())?;
        if let Some(hdr) = &mut self.animated_hdr { hdr.presented(); }
        Ok(true)
    }
}
fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("browser depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

impl WebEngine {
    fn prepare_animated_pose(
        &self,
        seconds: f32,
    ) -> Result<Option<voxy_render::PreparedSkinnedFrame>, JsValue> {
        self.animated
            .as_ref()
            .map(|history| {
                history
                    .prepare_frame(
                        &[
                            Mat4::IDENTITY,
                            Mat4::from_translation(Vec3::new(
                                0.35 * seconds.sin(),
                                0.0,
                                0.1 * (seconds * 0.5).sin(),
                            )),
                        ],
                        Mat4::IDENTITY,
                        [0.8, 0.4, 0.2, 1.0],
                    )
                    .map_err(error)
            })
            .transpose()
    }
    fn commit_animated_pose(
        &mut self,
        pose: Option<&voxy_render::PreparedSkinnedFrame>,
    ) -> Result<(), JsValue> {
        if let Some(pose) = pose {
            self.animated
                .as_mut()
                .ok_or_else(|| error("missing animated history"))?
                .presented_frame(pose)
                .map_err(error)?;
            self.animated_presentations = self.animated_presentations.saturating_add(1);
        }
        Ok(())
    }
    #[allow(clippy::cast_precision_loss)]
    fn update_scene_camera(&self, seconds: f32, width: u32, height: u32) -> Result<(), JsValue> {
        let aspect = width as f32 / height as f32;
        let (eye, target, far) = voxel_camera(self.voxel_world.as_ref(), aspect)?;
        let camera = SceneCamera {
            eye,
            target,
            up: Vec3::Y,
            projection: SceneProjection::Perspective {
                vertical_fov: 55_f32.to_radians(),
                aspect,
                near: 0.1,
                far,
            },
        };
        let vp = camera.view_projection().map_err(error)?;
        self.transform
            .update(
                &self.queue,
                vp * Mat4::from_quat(Quat::from_rotation_y(
                    if self.voxel_world.is_some() || self.animated.is_some() {
                        0.0
                    } else {
                        seconds
                    },
                )),
            )
            .map_err(error)?;
        if self.animated_hdr.is_some() {
            self.transform
                .update_scene_material(
                    &self.queue,
                    Mat4::IDENTITY,
                    [1.0; 4],
                    [1.5, 1.0, 2.5, self.animated_light_intensity],
                )
                .map_err(error)?;
            self.transform
                .update_view_position(&self.queue, eye)
                .map_err(error)?;
        }
        if let Some(hdr) = &self.animated_hdr {
            self.transform
                .update_pbr_material(&self.queue, hdr.material[0], hdr.material[1])
                .map_err(error)?;
            hdr.update_camera(
                &self.queue,
                camera,
                self.animated_light_intensity,
                self.animated_shadows,
                self.animated_shadow_filter,
            )
            .map_err(error)?;
        }
        self.update_actor(vp)?;
        Ok(())
    }
    fn update_actor(&self, vp: Mat4) -> Result<(), JsValue> {
        if let Some(game) = &self.voxel_game {
            self.overlay_transform
                .update(
                    &self.queue,
                    vp * Mat4::from_scale_rotation_translation(
                        game.vehicle
                            .as_ref()
                            .map_or(Vec3::splat(0.06), |_| Vec3::new(0.075, 0.04, 0.1)),
                        Quat::from_rotation_y(
                            game.vehicle
                                .as_ref()
                                .map_or(0.0, super::voxel_vehicle::BrowserVehicle::heading),
                        ),
                        game.position()?,
                    ),
                )
                .map_err(error)?;
        } else {
            update_overlay(&self.overlay_transform, &self.queue)?;
        }
        Ok(())
    }

    async fn build_voxel_world(&self) -> Result<voxy_runtime::BootstrapScene, JsValue> {
        if self.terrain.is_none() {
            return voxy_runtime::build_procedural_scene(42, 0).map_err(error);
        }
        let seed = voxy_world::WorldSeed(42);
        let mut chunks = Vec::with_capacity(27);
        for y in -1..=1 {
            for z in -1..=1 {
                for x in -1..=1 {
                    chunks.push(
                        self.terrain_chunk(voxy_core::ChunkPos { x, y, z }, seed)
                            .await?,
                    );
                }
            }
        }
        let (palette, water) = terrain_palette().map_err(error)?;
        let source = [
            palette.air,
            palette.surface,
            palette.soil,
            palette.stone,
            water,
        ];
        voxy_runtime::build_generated_scene(42, 0, |target, water| {
            Ok(Box::new(super::voxel_terrain::PreparedTerrain::new(
                chunks, seed, source, target, water,
            )?))
        })
        .map_err(error)
    }

    async fn terrain_chunk(
        &self,
        pos: voxy_core::ChunkPos,
        seed: voxy_world::WorldSeed,
    ) -> Result<voxy_world::GeneratedChunk, JsValue> {
        let program = self
            .terrain
            .as_ref()
            .ok_or_else(|| error("terrain compute unsupported on WebGL"))?;
        let job = program
            .create_job(&self.device, pos, seed, &voxy_core::CancelToken::new())
            .map_err(error)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let dispatch = job.encode(&mut encoder).map_err(error)?;
        self.queue.submit([encoder.finish()]);
        let mut read = dispatch.begin_read();
        let deadline = js_sys::Date::now() + 30000.0;
        loop {
            if let Some(chunk) = read.try_read().map_err(error)? {
                return Ok(chunk);
            }
            if js_sys::Date::now() >= deadline {
                return Err(error("browser terrain readback timed out"));
            }
            yield_browser().await?;
        }
    }
}

pub(crate) async fn yield_browser() -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let result = web_sys::window()
            .ok_or_else(|| error("missing browser window"))
            .and_then(|window| {
                window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
            });
        if let Err(error) = result {
            let _ = reject.call1(&JsValue::UNDEFINED, &error);
        }
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|_| ())
}

fn dense_ids(chunk: &voxy_world::GeneratedChunk) -> Result<Vec<u32>, JsValue> {
    let mut ids = Vec::with_capacity(voxy_core::CHUNK_VOLUME);
    for y in 0..32 {
        for z in 0..32 {
            for x in 0..32 {
                let local = voxy_core::LocalPos::new(x, y, z).map_err(error)?;
                ids.push(chunk.data.blocks.get(local.index()).get());
            }
        }
    }
    Ok(ids)
}

pub(super) fn terrain_palette()
-> Result<(voxy_world::TerrainPalette, voxy_world::BlockStateId), voxy_world::RegistryError> {
    use voxy_world::{
        BlockDef, BlockRegistry, CollisionShape, MaterialId, Occlusion, RenderKind, ResourceKey,
        TerrainPalette,
    };
    let definitions = ["air", "surface", "soil", "stone", "water"]
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            Ok(BlockDef {
                key: ResourceKey::parse(format!("voxy:{name}"))?,
                render: if i == 0 {
                    RenderKind::Invisible
                } else {
                    RenderKind::Opaque
                },
                occlusion: if i == 0 {
                    Occlusion::None
                } else {
                    Occlusion::FullCube
                },
                collision: if i == 0 {
                    CollisionShape::Empty
                } else {
                    CollisionShape::FullCube
                },
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let registry = BlockRegistry::new(definitions)?;
    let find = |name| {
        registry
            .find(&ResourceKey::parse(format!("voxy:{name}"))?)
            .ok_or(voxy_world::RegistryError::MissingAir)
    };
    Ok((
        TerrainPalette {
            air: find("air")?,
            surface: find("surface")?,
            soil: find("soil")?,
            stone: find("stone")?,
        },
        find("water")?,
    ))
}

fn update_overlay(transform: &SceneTransform, queue: &wgpu::Queue) -> Result<(), JsValue> {
    transform
        .update(
            queue,
            Mat4::from_scale_rotation_translation(
                Vec3::new(0.6, 0.15, 1.0),
                Quat::IDENTITY,
                Vec3::new(-0.6, 0.75, 0.0),
            ),
        )
        .map_err(error)?;
    Ok(())
}

pub(super) fn voxel_camera(
    scene: Option<&voxy_runtime::BootstrapScene>,
    aspect: f32,
) -> Result<(Vec3, Vec3, f32), JsValue> {
    let Some(scene) = scene else {
        return Ok((Vec3::new(0.0, 1.0, 4.0), Vec3::ZERO, 100.0));
    };
    if scene.chunks.len() <= 1 {
        return Ok((Vec3::new(1.8, 2.2, 2.6), Vec3::ZERO, 100.0));
    }
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for chunk in &scene.chunks {
        let relative = |value: i64, anchor: i64| -> Result<f32, JsValue> {
            let delta = value
                .checked_sub(anchor)
                .ok_or_else(|| error("camera offset overflow"))?;
            Ok(f32::from(i16::try_from(delta).map_err(error)?) * 2.0)
        };
        let origin = Vec3::new(
            relative(chunk.pos.x, scene.anchor.x)?,
            relative(chunk.pos.y, scene.anchor.y)?,
            relative(chunk.pos.z, scene.anchor.z)?,
        );
        let origin = origin - Vec3::new(1.0, 0.625, 1.0);
        min = min.min(origin);
        max = max.max(origin + Vec3::splat(2.0));
    }
    let target = (min + max) * 0.5;
    if !aspect.is_finite() || aspect <= 0.0 {
        return Err(error("invalid camera aspect"));
    }
    let radius = (max - min).length() * 0.5;
    let half_fov = 27.5_f32.to_radians();
    let limiting_fov = half_fov.min((half_fov.tan() * aspect).atan());
    let distance = radius / limiting_fov.sin() * 1.05;
    let far = (distance + radius * 1.2).max(100.0);
    if !distance.is_finite() || !far.is_finite() {
        return Err(error("camera range overflow"));
    }
    Ok((
        target + Vec3::new(1.8, 2.2, 2.6).normalize() * distance,
        target,
        far,
    ))
}

fn encode_scene(
    device: &wgpu::Device,
    scene: &SceneRenderer,
    hdr: Option<&mut super::animated_scene::HdrScene>,
    encoder: &mut wgpu::CommandEncoder,
    views: (&wgpu::TextureView, &wgpu::TextureView),
    draws: &[SceneDraw<'_>; 2],
    size: [u32; 2],
    pose: Option<&voxy_render::PreparedSkinnedFrame>,
) -> Result<(), JsValue> {
    let (view, depth) = views;
    if let Some(hdr) = hdr {
        hdr.encode(device, encoder, (view, depth), size, &draws[0], pose)
            .map_err(error)?;
        scene.encode_overlays(encoder, view, depth, &draws[1..]);
    } else {
        scene.encode(encoder, view, depth, wgpu::Color::BLACK, draws);
    }
    Ok(())
}
