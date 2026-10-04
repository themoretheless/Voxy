//! Browser raster version of the native two-joint ray-demo geometry.
pub(crate) fn mesh() -> Result<voxy_render::SkinnedMesh, voxy_render::SkinnedMeshError> {
    let vertices = [
        [-1.0_f32, -1.0, 0.5],
        [1.0, -1.0, 0.5],
        [1.0, 1.0, 0.5],
        [-1.0, 1.0, 0.5],
    ]
    .map(|position| voxy_render::SkinnedVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        uv: [(position[0] + 1.0) * 0.5, (position[1] + 1.0) * 0.5],
        joints: [0, 1, 0, 0],
        weights: if position[1] < 0.0 {
            [65535, 0, 0, 0]
        } else {
            [0, 65535, 0, 0]
        },
    });
    voxy_render::SkinnedMesh::new(vertices.to_vec(), vec![0, 1, 2, 0, 2, 3], 2)
}

type SceneEnvironment = (
    voxy_render::GgxEnvironmentPrefilter,
    voxy_render::GgxDfgLut,
    voxy_render::DiffuseEnvironmentConvolution,
);

/// Retained HDR scene pipeline and tone mapper; color targets follow canvas size.
#[derive(Debug)]
pub(crate) struct HdrScene {
    environment: SceneEnvironment,
    temporal_guides_enabled: bool,
    guide_probe: Option<super::temporal_guides_probe::GuideProbe>,
    color_pipeline: Option<wgpu::ComputePipeline>,
    color_probe: Option<super::temporal_color_probe::ColorProbe>,
    pub(crate) color_checks: u32,
    pub(crate) depth_rejections: u32,
    pub(crate) color_blends: u32,
    guide_frames: u32,
    guide_checks: u32,
    pub(crate) guide_moving: u32,
    temporal: Option<(voxy_render::TemporalResolve, voxy_render::TemporalHistory)>,
    camera: std::cell::Cell<glam::Mat4>,
    previous_camera: Option<glam::Mat4>,
    motion: Option<voxy_render::RasterMotionPass>,
    previous_depth: Option<voxy_render::PreviousDepthPass>,
    pub(crate) probe: Option<voxy_render::HdrPixelProbe>,
    pub(crate) temporal_probe: Option<voxy_render::HdrPixelProbe>,
    renderer: voxy_render::SceneRenderer,
    target: voxy_render::ProcessedColorTarget,
    display: voxy_render::TextureBlit,
    shadow: voxy_render::ShadowMap,
    shadow_camera: glam::Mat4,
    receiver: voxy_render::SceneGeometry,
    receiver_transform: voxy_render::SceneTransform,
    reflection: voxy_render::PlanarReflectionCapture,
    reflection_transform: voxy_render::SceneTransform,
    surface_renderer: voxy_render::SceneRenderer,
    surface_geometry: voxy_render::SceneGeometry,
    surface_transform: voxy_render::SceneTransform,
    surface_material: voxy_render::SceneTexture,
    pub(crate) material: [f32; 2],
    pub(crate) reflector_material: [f32; 2],
}
impl HdrScene {
    pub(crate) fn enable_temporal_guides(&mut self, enabled: bool) {
        self.temporal_guides_enabled = enabled;
        self.previous_camera = None;
        if !enabled {
            self.motion = None;
            self.previous_depth = None;
            self.temporal = None;
        }
    }
    pub(crate) fn begin_guide_read(&mut self) {
        if let Some(probe) = &mut self.color_probe {
            probe.begin_read();
        }
        if let Some(probe) = &mut self.guide_probe {
            probe.begin_read();
        }
    }
    pub(crate) fn poll_guide_checks(&mut self) -> Result<u32, wasm_bindgen::JsValue> {
        if let Some(probe) = &mut self.color_probe {
            if let Some((checked, rejected, blended)) = probe.poll()? {
                self.color_checks += checked;
                self.depth_rejections += rejected;
                self.color_blends += blended;
                self.color_probe = None;
            }
        }
        if let Some(probe) = &mut self.guide_probe {
            if let Some((checked, moving)) = probe.poll()? {
                self.guide_checks += checked;
                self.guide_moving += moving;
                self.guide_probe = None;
            }
        }
        Ok(self.guide_checks)
    }
    pub(crate) fn invalidate_temporal(&mut self) {
        self.previous_camera = None;
        if let Some((_, history)) = &mut self.temporal {
            history.reset();
        }
    }
    pub(crate) fn presented(&mut self) {
        if self.temporal_guides_enabled {
            self.previous_camera = Some(self.camera.get());
            self.guide_frames = self.guide_frames.saturating_add(1);
            if let Some((_, history)) = &mut self.temporal {
                history.presented();
            }
        }
    }
    pub(crate) fn environment_intensity(
        &mut self,
        queue: &wgpu::Queue,
        intensity: f32,
    ) -> Result<(), JsError> {
        self.renderer
            .set_environment_intensity(queue, intensity)
            .map_err(|e| JsError(e.to_string()))?;
        self.reflection
            .set_environment_intensity(queue, intensity)
            .map_err(|e| JsError(e.to_string()))?;
        self.invalidate_temporal();
        Ok(())
    }
    pub(crate) async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<Self, JsError> {
        let mut renderer =
            voxy_render::SceneRenderer::new(device, wgpu::TextureFormat::Rgba16Float);
        let shadow =
            voxy_render::ShadowMap::new(device, 1024, 1024).map_err(|e| JsError(e.to_string()))?;
        let shadow_camera = voxy_render::SceneCamera {
            eye: glam::Vec3::new(1.5, 1.0, 2.5),
            target: glam::Vec3::ZERO,
            up: glam::Vec3::Y,
            projection: voxy_render::SceneProjection::Perspective {
                vertical_fov: 90_f32.to_radians(),
                aspect: 1.0,
                near: 0.1,
                far: 15.0,
            },
        }
        .view_projection()
        .map_err(|e| JsError(e.to_string()))?;
        renderer
            .enable_shadowed_point_light(
                device,
                &shadow,
                voxy_render::ShadowSettings {
                    light_from_world: shadow_camera,
                    bias: 0.001,
                    enabled: true,
                    filter: voxy_render::ShadowFilter::Pcf3x3,
                },
            )
            .await
            .map_err(|e| JsError(e.to_string()))?;
        let (environment, dfg, diffuse) = initialize_environment(device, queue)?;
        renderer
            .enable_full_environment_lighting(device, &environment, &dfg, &diffuse)
            .await
            .map_err(|e| JsError(e.to_string()))?;
        let receiver = renderer
            .upload_mesh(device, &plane_mesh([0.65, 0.65, 0.65, 1.0], -0.5)?)
            .map_err(|e| JsError(e.to_string()))?;
        let receiver_transform = renderer
            .create_transform(device, glam::Mat4::IDENTITY)
            .map_err(|e| JsError(e.to_string()))?;
        let mut reflection = voxy_render::PlanarReflectionCapture::new(device, width, height)
            .map_err(|e| JsError(e.to_string()))?;
        reflection
            .enable_full_environment_lighting(&environment, &dfg, &diffuse)
            .await
            .map_err(|e| JsError(e.to_string()))?;
        let reflection_transform = reflection
            .renderer()
            .create_transform(device, glam::Mat4::IDENTITY)
            .map_err(|e| JsError(e.to_string()))?;
        let mut surface_renderer =
            voxy_render::SceneRenderer::new(device, wgpu::TextureFormat::Rgba16Float);
        surface_renderer
            .reload_shader(device, voxy_render::PLANAR_REFLECTION_ROUGH_SHADER)
            .await
            .map_err(|e| JsError(e.to_string()))?;
        let surface_geometry = surface_renderer
            .upload_mesh(device, &plane_mesh([1.0; 4], -0.49)?)
            .map_err(|e| JsError(e.to_string()))?;
        let surface_transform = surface_renderer
            .create_transform(device, glam::Mat4::IDENTITY)
            .map_err(|e| JsError(e.to_string()))?;
        let surface_material = reflection
            .mip_material_binding(&surface_renderer, reflection_sampling())
            .map_err(|e| JsError(e.to_string()))?;
        Ok(Self {
            environment: (environment, dfg, diffuse),
            temporal_guides_enabled: false,
            guide_probe: None,
            color_pipeline: None,
            color_probe: None,
            color_checks: 0,
            depth_rejections: 0,
            color_blends: 0,
            guide_frames: 0,
            guide_checks: 0,
            guide_moving: 0,
            temporal: None,
            camera: std::cell::Cell::new(glam::Mat4::IDENTITY),
            previous_camera: None,
            motion: None,
            previous_depth: None,
            probe: None,
            temporal_probe: None,
            reflection,
            reflection_transform,
            surface_renderer,
            surface_geometry,
            surface_transform,
            surface_material,
            renderer,
            shadow,
            shadow_camera,
            receiver,
            receiver_transform,
            material: [0.35, 0.5],
            reflector_material: [0.35, 0.5],
            target: voxy_render::ProcessedColorTarget::new(device, width, height, true)
                .map_err(|e| JsError(e.to_string()))?,
            display: voxy_render::TextureBlit::tone_mapped(device, format, 1.0)
                .ok_or_else(|| JsError("invalid exposure".into()))?,
        })
    }
    pub(crate) async fn load_environment(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> Result<(), JsError> {
        let panorama =
            voxy_render::HdrImageAsset::decode(bytes, voxy_render::ImageLimits::default())
                .map_err(|e| JsError(e.to_string()))?;
        let candidate = voxy_render::ImportedEnvironment::from_panorama(
            device,
            queue,
            &panorama,
            32,
            voxy_render::ImageLimits::default(),
        )
        .map_err(|e| JsError(e.to_string()))?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        candidate.encode(&mut encoder);
        queue.submit([encoder.finish()]);
        candidate
            .attach(&mut self.renderer)
            .await
            .map_err(|e| JsError(e.to_string()))?;
        if let Err(error) = self
            .reflection
            .enable_full_environment_lighting(
                candidate.specular(),
                candidate.dfg(),
                candidate.diffuse(),
            )
            .await
        {
            self.renderer
                .enable_full_environment_lighting(
                    device,
                    &self.environment.0,
                    &self.environment.1,
                    &self.environment.2,
                )
                .await
                .map_err(|rollback| {
                    JsError(format!("{error}; restoring environment failed: {rollback}"))
                })?;
            return Err(JsError(error.to_string()));
        }
        self.environment = candidate.into_parts();
        self.invalidate_temporal();
        self.probe = Some(voxy_render::HdrPixelProbe::new(device));
        Ok(())
    }

    pub(crate) fn update_camera(
        &self,
        queue: &wgpu::Queue,
        camera: voxy_render::SceneCamera,
        intensity: f32,
        shadows: bool,
        filter: voxy_render::ShadowFilter,
    ) -> Result<(), JsError> {
        let update = || -> Result<(), voxy_render::SceneError> {
            let vp = camera
                .view_projection()
                .map_err(|_| voxy_render::SceneError::InvalidTransform)?;
            self.camera.set(vp);
            let reflected = camera
                .reflected(glam::Vec3::new(0.0, 0.0, -0.5), glam::Vec3::Z)
                .map_err(|_| voxy_render::SceneError::InvalidTransform)?;
            let reflected_vp = reflected
                .view_projection()
                .map_err(|_| voxy_render::SceneError::InvalidTransform)?;
            self.reflection_transform.update(queue, reflected_vp)?;
            self.reflection_transform
                .update_view_position(queue, reflected.eye)?;
            self.reflection_transform.update_scene_material(
                queue,
                glam::Mat4::IDENTITY,
                [1.0; 4],
                [1.5, 1.0, 2.5, intensity],
            )?;
            self.reflection_transform.update_pbr_material(
                queue,
                self.material[0],
                self.material[1],
            )?;
            self.reflection_transform.update_pbr_capture_plane(
                queue,
                glam::Vec3::new(0.0, 0.0, -0.5),
                glam::Vec3::Z,
            )?;
            self.surface_transform
                .update_planar_projection(queue, vp, reflected_vp)?;
            self.surface_transform.update_scene_material(
                queue,
                glam::Mat4::IDENTITY,
                [0.65, 0.65, 0.65, 1.0],
                [0.0; 4],
            )?;
            self.surface_transform
                .update_view_position(queue, camera.eye)?;
            self.surface_transform.update_pbr_material(
                queue,
                self.reflector_material[0],
                self.reflector_material[1],
            )?;
            self.receiver_transform.update(queue, vp)?;
            self.receiver_transform
                .update_view_position(queue, camera.eye)?;
            self.receiver_transform.update_scene_material(
                queue,
                glam::Mat4::IDENTITY,
                [1.0; 4],
                [1.5, 1.0, 2.5, intensity],
            )?;
            self.renderer.update_shadow_settings(
                queue,
                voxy_render::ShadowSettings {
                    light_from_world: self.shadow_camera,
                    bias: 0.001,
                    enabled: shadows,
                    filter,
                },
            )
        };
        update().map_err(|e| JsError(e.to_string()))
    }
    pub(crate) fn exposure(&mut self, device: &wgpu::Device, value: f32) -> Result<(), JsError> {
        self.display = self
            .display
            .with_exposure(device, value)
            .ok_or_else(|| JsError("exposure must be finite and positive".into()))?;
        Ok(())
    }
    pub(crate) fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        views: (&wgpu::TextureView, &wgpu::TextureView),
        size: [u32; 2],
        draw: &voxy_render::SceneDraw<'_>,
        pose: Option<&voxy_render::PreparedSkinnedFrame>,
    ) -> Result<(), JsError> {
        let (output, depth) = views;
        if self.target.texture().width() != size[0] || self.target.texture().height() != size[1] {
            self.previous_camera = None;
            self.target = voxy_render::ProcessedColorTarget::new(device, size[0], size[1], true)
                .map_err(|e| JsError(e.to_string()))?;
        }
        if self
            .reflection
            .resize(size[0], size[1])
            .map_err(|e| JsError(e.to_string()))?
        {
            self.surface_material = self
                .reflection
                .mip_material_binding(&self.surface_renderer, reflection_sampling())
                .map_err(|e| JsError(e.to_string()))?;
        }
        self.reflection
            .encode(
                encoder,
                wgpu::Color::BLACK,
                &[voxy_render::SceneDraw {
                    geometry: draw.geometry,
                    texture: draw.texture,
                    transform: &self.reflection_transform,
                    overlay: false,
                }],
            )
            .map_err(|e| JsError(e.to_string()))?;
        self.shadow
            .encode(
                encoder,
                &[self
                    .shadow
                    .prepare(draw.geometry, self.shadow_camera)
                    .map_err(|e| JsError(e.to_string()))?],
            )
            .map_err(|e| JsError(e.to_string()))?;
        let receiver = voxy_render::SceneDraw {
            geometry: &self.receiver,
            texture: draw.texture,
            transform: &self.receiver_transform,
            overlay: false,
        };
        self.renderer.encode(
            encoder,
            self.target.view(),
            depth,
            wgpu::Color::BLACK,
            &[
                receiver,
                voxy_render::SceneDraw {
                    geometry: draw.geometry,
                    texture: draw.texture,
                    transform: draw.transform,
                    overlay: false,
                },
            ],
        );
        self.surface_renderer.encode_transparent_over(
            encoder,
            self.target.view(),
            depth,
            &[voxy_render::SceneDraw {
                geometry: &self.surface_geometry,
                texture: &self.surface_material,
                transform: &self.surface_transform,
                overlay: false,
            }],
        );
        let mut resolved = None;
        if self.temporal_guides_enabled {
            let pose = pose.ok_or_else(|| JsError("missing temporal skeletal pose".into()))?;
            let mut vertices = pose.motion().vertices.clone();
            let receiver = plane_mesh([1.0; 4], -0.5)?;
            for index in receiver.indices() {
                let current = receiver.vertices()[*index as usize].position;
                vertices.push(voxy_render::PreviousPositionVertex {
                    current,
                    previous: current,
                });
            }
            let cameras = [
                self.camera.get(),
                self.previous_camera.unwrap_or(self.camera.get()),
            ];
            let reset = self.previous_camera.is_none() || !pose.motion().history_valid;
            let motion = match self.motion.take() {
                Some(previous) => {
                    previous.next_frame_reusing(device, depth.texture(), cameras, &vertices, reset)
                }
                None => voxy_render::RasterMotionPass::new(
                    device,
                    depth.texture(),
                    cameras,
                    &vertices,
                    reset,
                ),
            }
            .map_err(|e| JsError(e.to_string()))?;
            let previous_depth = match self.previous_depth.take() {
                Some(previous) => {
                    previous.next_frame_reusing(device, depth.texture(), cameras, &vertices)
                }
                None => {
                    voxy_render::PreviousDepthPass::new(device, depth.texture(), cameras, &vertices)
                }
            }
            .map_err(|e| JsError(e.to_string()))?;
            motion.encode(encoder);
            previous_depth.encode(encoder);
            if self.guide_probe.is_none() && [0, 30, 60, 120].contains(&self.guide_frames) {
                self.guide_probe = Some(
                    super::temporal_guides_probe::GuideProbe::encode(
                        device,
                        encoder,
                        motion.output(),
                        previous_depth.output(),
                        cameras,
                        &vertices,
                        reset,
                    )
                    .map_err(|e| JsError(format!("{e:?}")))?,
                );
            }
            if self.temporal.is_none() {
                self.probe = Some(voxy_render::HdrPixelProbe::new(device));
                self.temporal_probe = Some(voxy_render::HdrPixelProbe::new(device));
                self.temporal = Some((
                    voxy_render::TemporalResolve::new(device)
                        .map_err(|e| JsError(e.to_string()))?,
                    voxy_render::TemporalHistory::new(device, size[0], size[1])
                        .map_err(|e| JsError(e.to_string()))?,
                ));
            }
            let (resolver, history) = self
                .temporal
                .as_mut()
                .ok_or_else(|| JsError("missing temporal history".into()))?;
            history
                .resize(size[0], size[1])
                .map_err(|e| JsError(e.to_string()))?;
            history
                .encode_depth_attachment(encoder, depth.texture())
                .map_err(|e| JsError(e.to_string()))?;
            let frame = history
                .prepare_resolve(
                    resolver,
                    self.target.texture(),
                    motion.output(),
                    previous_depth.output(),
                    voxy_render::TemporalResolveOptions {
                        history_weight: 0.85,
                        depth_tolerance: 0.001,
                        reset_history: reset,
                    },
                    true,
                )
                .map_err(|e| JsError(e.to_string()))?;
            frame.encode(encoder);
            if self.color_probe.is_none() && [0, 30, 60, 120].contains(&self.guide_frames) {
                let pipeline = self
                    .color_pipeline
                    .get_or_insert_with(|| super::temporal_color_probe::pipeline(device));
                let pixels =
                    super::temporal_guides_probe::sample_pixels(size, cameras[0], &vertices, 0.5);
                self.color_probe = Some(
                    super::temporal_color_probe::ColorProbe::encode(
                        device,
                        encoder,
                        pipeline,
                        [
                            self.target.texture(),
                            motion.output(),
                            previous_depth.output(),
                            history.color(),
                            history.depth(),
                            frame.output(),
                        ],
                        &pixels,
                        reset || !history.valid(),
                    )
                    .map_err(|e| JsError(format!("{e:?}")))?,
                );
            }
            if let Some(probe) = &mut self.temporal_probe {
                probe
                    .encode(encoder, frame.output(), size[0] / 2, size[1] / 2)
                    .map_err(|e| JsError(e.to_string()))?;
            }
            resolved = Some(frame);
            self.motion = Some(motion);
            self.previous_depth = Some(previous_depth);
        }
        if let Some(probe) = &mut self.probe {
            probe
                .encode(encoder, self.target.texture(), size[0] / 2, size[1] / 2)
                .map_err(|e| JsError(e.to_string()))?;
        }
        let resolved_view = resolved
            .as_ref()
            .map(|frame| frame.output().create_view(&Default::default()));
        self.display
            .encode_checked(
                device,
                encoder,
                resolved_view.as_ref().unwrap_or(self.target.view()),
                output,
            )
            .map_err(|e| JsError(e.to_string()))?;
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) struct JsError(String);
impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn plane_mesh(color: [f32; 4], z: f32) -> Result<voxy_render::SceneMesh, JsError> {
    let mesh = voxy_render::SceneMesh::quad(color);
    let vertices = mesh
        .vertices()
        .iter()
        .map(|vertex| {
            let mut vertex = *vertex;
            vertex.position = [vertex.position[0] * 6.0, vertex.position[1] * 6.0, z];
            vertex
        })
        .collect();
    voxy_render::SceneMesh::new(vertices, mesh.indices().to_vec())
        .map_err(|e| JsError(e.to_string()))
}

fn reflection_sampling() -> voxy_render::TextureSampling {
    voxy_render::TextureSampling {
        min_filter: voxy_render::TextureFilter::Linear,
        mag_filter: voxy_render::TextureFilter::Linear,
        mipmap_filter: Some(voxy_render::TextureFilter::Linear),
        ..voxy_render::TextureSampling::default()
    }
}

/// Small deterministic studio environment; integer half-float radiance avoids asset decoding.
fn initialize_environment(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<SceneEnvironment, JsError> {
    let environment = voxy_render::GgxEnvironmentPrefilter::new(device, 16)
        .map_err(|e| JsError(e.to_string()))?;
    let diffuse = voxy_render::DiffuseEnvironmentConvolution::new(device, 16)
        .map_err(|e| JsError(e.to_string()))?;
    let dfg = voxy_render::GgxDfgLut::new(device, 64).map_err(|e| JsError(e.to_string()))?;
    // Six distinct studio walls: warm key, cool fill, ceiling, floor, front, back.
    let faces: [[u16; 4]; 6] = [
        [0x4000, 0x3c00, 0x3800, 0x3c00],
        [0x3800, 0x3c00, 0x4000, 0x3c00],
        [0x4000, 0x4000, 0x4000, 0x3c00],
        [0x3000, 0x3000, 0x3000, 0x3c00],
        [0x3c00, 0x3800, 0x3400, 0x3c00],
        [0x3400, 0x3800, 0x3c00, 0x3c00],
    ];
    for destination in [environment.input(), diffuse.input()] {
        for (face, color) in faces.into_iter().enumerate() {
            let pixel: Vec<u8> = color.into_iter().flat_map(u16::to_le_bytes).collect();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: destination,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: u32::try_from(face).map_err(|e| JsError(e.to_string()))?,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &pixel.repeat(16 * 16),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(16 * 8),
                    rows_per_image: Some(16),
                },
                wgpu::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    environment.encode(&mut encoder);
    diffuse.encode(&mut encoder);
    dfg.encode(&mut encoder);
    queue.submit([encoder.finish()]);
    Ok((environment, dfg, diffuse))
}
