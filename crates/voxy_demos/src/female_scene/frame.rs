//! Per-frame uploads, draw list and compute encoding of the character scene.
use super::FemaleScene;
use super::gpu::FemaleGpu;
use crate::female_demo::FemaleDemo;
use crate::full_model_worker::{Controls, Frame};
use std::time::Instant;
use voxy_render::{RendererError, SceneDraw, SceneError, SceneMesh};
use voxy_scene_host::{DemoFrame, DemoResult, FrameParts, Presented, SharedDraw};

impl FemaleGpu {
    /// Replaces the shared texture when the complexion atlas was rebuilt.
    fn poll_atlas(&mut self, female: &FemaleDemo, frame: &mut DemoFrame<'_>) -> DemoResult {
        if let Some((creases, pixels)) = self
            .atlas_update
            .poll(&self.material_creases, &female.face_parameters)?
        {
            *frame.texture = frame.scene.upload_texture_with_sampling(
                frame.device,
                frame.queue,
                crate::female_complexion::WIDTH,
                crate::female_complexion::SIZE,
                &pixels,
                voxy_render::TextureSampling {
                    min_filter: voxy_render::TextureFilter::Linear,
                    mag_filter: voxy_render::TextureFilter::Linear,
                    ..Default::default()
                },
            )?;
            self.material_creases = creases;
        }
        Ok(())
    }

    /// Uploads a frame published by the background full-model worker.
    fn upload_worker_frame(&mut self, frame: &Frame, queue: &wgpu::Queue) -> DemoResult {
        eprintln!(
            "FULL MODEL WORKER work_ms={:.3} physics_ms={:.3} mesh_ms={:.3} partition_ms={:.3} streams_ms={:.3} skin_solver_ms={:.3} hair_solver_ms={:.3} simulated_s={:.6}",
            frame.work_ms,
            frame.phase_ms[0],
            frame.phase_ms[1],
            frame.phase_ms[2],
            frame.phase_ms[3],
            frame.solver_ms[0],
            frame.solver_ms[1],
            frame.simulation_time
        );
        let upload_started = Instant::now();
        if let Some(gpu) = &mut self.gpu_hair {
            let frames = frame
                .hair_frames
                .as_ref()
                .ok_or("missing solved GPU hair frames")?;
            let bytes = gpu.input.replace_frames(frames)?;
            let uploaded = bytes.len();
            self.cube.update(queue, &frame.mesh)?;
            queue.write_buffer(gpu.job.buffer(), 32, bytes);
            let offset = gpu.input.transform_byte_offset();
            let pose = gpu.input.replace_transform(frame.hair_transform)?;
            queue.write_buffer(gpu.job.buffer(), offset, pose);
            gpu.dirty = true;
            if let Some(hair) = &mut self.hair_geometry {
                hair.update_index_partition(
                    queue,
                    if frame.hair_visible {
                        &gpu.indices
                    } else {
                        &[]
                    },
                )?;
            }
            eprintln!(
                "FULL MODEL GPU HAIR frame body_vertices={} hair_vertices={} upload_bytes={uploaded} cpu_hair_vertices=0",
                frame.mesh.vertices().len(),
                gpu.input.vertex_count()
            );
        } else {
            self.cube.update_shared_vertex_streams(queue, &frame.mesh)?;
            self.cube
                .update_index_partition(queue, &frame.body_indices)?;
            if let Some(hair) = &mut self.hair_geometry {
                hair.update_index_partition(queue, &frame.hair_indices)?;
            }
        }
        if std::env::var_os("VOXY_PRESENT_PROFILE").is_some()
            || std::env::var_os("VOXY_FRAME_PROFILE").is_some()
        {
            eprintln!(
                "FULL MODEL CPU UPLOAD stage_ms={:.3}",
                upload_started.elapsed().as_secs_f64() * 1000.
            );
        }
        Ok(())
    }

    /// Uploads the resident hair inputs and returns the body mesh without hair.
    fn upload_resident_hair(
        &mut self,
        female: &FemaleDemo,
        queue: &wgpu::Queue,
    ) -> DemoResult<Option<SceneMesh>> {
        let Some(gpu) = &mut self.gpu_hair else {
            return Ok(None);
        };
        let frames = female.gpu_hair_surface_frames();
        let bytes = gpu.input.replace_frames(&frames)?;
        queue.write_buffer(gpu.job.buffer(), 32, bytes);
        let offset = gpu.input.transform_byte_offset();
        let pose = gpu
            .input
            .replace_transform(female.gpu_hair_surface_transform())?;
        queue.write_buffer(gpu.job.buffer(), offset, pose);
        gpu.dirty = true;
        if let Some(hair) = &mut self.hair_geometry {
            hair.update_index_partition(queue, if female.show_hair { &gpu.indices } else { &[] })?;
        }
        Ok(Some(female.mesh_without_hair()?))
    }

    /// Uploads the CPU-prepared body mesh, split into body and film layer when a film exists.
    fn upload_body_mesh(
        &mut self,
        female: &FemaleDemo,
        mesh: &SceneMesh,
        frame: &DemoFrame<'_>,
    ) -> DemoResult {
        if let Some((body, film)) = if female.film.is_some() {
            mesh.split_material_layer(-2., -6.)?
        } else {
            None
        } {
            self.cube.update(frame.queue, &body)?;
            if let Some(layer) = &mut self.film_layer {
                layer.update(frame.queue, &film)?;
            } else {
                self.film_layer = Some(frame.scene.upload_mesh(frame.device, &film)?);
            }
        } else {
            self.film_layer = None;
            if let Some(body) = &mut self.gpu_body {
                self.cube
                    .update_after_deforming_prefix(frame.queue, mesh, body.base_vertices())?;
                body.refresh(frame.queue, female, mesh)?;
            } else {
                self.cube.update(frame.queue, mesh)?;
            }
        }
        Ok(())
    }
}

impl FemaleScene {
    pub(super) fn prepare_frame(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult {
        let Self {
            female,
            full_model_worker,
            gpu,
            ..
        } = self;
        let r = gpu.as_mut().ok_or("character resources are not created")?;
        female.resident_surface_light =
            r.gpu_surface_light.as_ref().is_some_and(|light| {
                female.can_use_resident_surface_light(light.surface_fraction())
            }) && std::env::var_os("VOXY_GPU_SURFACE_LIGHT_CPU_REFERENCE").is_none();
        if let Some(audit) = &mut r.gpu_surface_light_audit {
            audit.poll(frame.device)?;
        }
        r.poll_atlas(female, frame)?;
        let view_position = female.eye();
        if r.last_view_position != Some(view_position) {
            frame
                .world_transform
                .update_view_position(frame.queue, view_position)?;
            r.last_view_position = Some(view_position);
        }
        let prepared_mesh = if let Some(worker) = full_model_worker {
            match worker.poll(Controls::capture(female, frame.paused)) {
                Ok(Some(published)) => r.upload_worker_frame(&published, frame.queue)?,
                Ok(None) => {}
                Err(error) => eprintln!(
                    "Full-model preparation failed; retaining last uploaded pose: {error}"
                ),
            }
            None
        } else if r.gpu_hair.is_some() {
            r.upload_resident_hair(female, frame.queue)?
        } else if r.gpu_secondary.is_none() {
            Some(female.mesh()?)
        } else {
            None
        };
        if let Some(mesh) = prepared_mesh {
            r.upload_body_mesh(female, &mesh, frame)?;
        }
        if let Some(light) = &r.gpu_surface_light {
            light.upload_reference_normals(frame.queue, &female.surface_light_normals())?;
        }
        if let Some(started) = frame.trace_started {
            eprintln!(
                "FACE FRAME {} mesh_ready_ms={:.3}",
                frame.frame,
                started.elapsed().as_secs_f64() * 1000.
            );
        }
        let labels = female.diagnostic_legend_labels();
        if labels != r.legend_labels {
            r.overlay
                .update(frame.queue, &crate::diagnostic_legend::mesh(&labels)?)?;
            r.legend_labels = labels;
        }
        Ok(())
    }

    pub(super) fn frame_parts<'a>(
        &'a mut self,
        shared: SharedDraw<'a>,
    ) -> DemoResult<FrameParts<'a>> {
        let female = &self.female;
        let r = self
            .gpu
            .as_mut()
            .ok_or("character resources are not created")?;
        r.gpu_controls = r.gpu_secondary.is_some().then(|| {
            female
                .gpu_secondary_controls()
                .iter()
                .zip(&r.gpu_secondary_reference)
                .map(|(p, r)| std::array::from_fn(|k| p[k] - r[k]))
                .collect::<Vec<[f32; 4]>>()
        });
        r.gpu_jump_palette = r.gpu_secondary.is_some().then(|| female.gpu_jump_palette());
        r.surface_light_audit_encoded = false;
        let update_gpu = r
            .gpu_controls
            .as_ref()
            .is_some_and(|controls| controls != &r.gpu_secondary_last_controls)
            || r.gpu_jump_palette
                .as_ref()
                .is_some_and(|palette| palette != &r.gpu_jump_last_palette);
        let light_enabled = r
            .gpu_surface_light
            .as_ref()
            .is_some_and(|light| female.can_use_resident_surface_light(light.surface_fraction()));
        let world = |geometry| SceneDraw {
            geometry,
            texture: shared.texture,
            transform: shared.world_transform,
            overlay: false,
        };
        let mut draws = Vec::with_capacity(3);
        draws.push(world(
            r.gpu_secondary
                .as_ref()
                .map_or(&r.cube, |gpu| gpu.geometry()),
        ));
        if let Some(hair) = &r.hair_geometry {
            draws.push(world(hair));
        }
        if female.diagnostic_legend_visible() {
            draws.push(SceneDraw {
                geometry: &r.overlay,
                texture: shared.texture,
                transform: shared.overlay_transform,
                overlay: true,
            });
        }
        let refractive = r.film_layer.iter().map(world).collect();
        let encode = ComputeEncode {
            gpu_body: &mut r.gpu_body,
            gpu_hair: &mut r.gpu_hair,
            gpu_secondary: &r.gpu_secondary,
            gpu_controls: &r.gpu_controls,
            gpu_jump_palette: &r.gpu_jump_palette,
            gpu_surface_light: &mut r.gpu_surface_light,
            gpu_surface_colors: &r.gpu_surface_colors,
            gpu_surface_light_audit: &r.gpu_surface_light_audit,
            surface_light_audit_encoded: &mut r.surface_light_audit_encoded,
            update_gpu,
            light_enabled,
        };
        Ok(FrameParts {
            draws,
            refractive,
            prepare: Some(Box::new(move |queue, encoder| encode.run(queue, encoder))),
            custom_pass: None,
        })
    }

    pub(super) fn frame_presented(&mut self, info: Presented) -> DemoResult {
        let r = self
            .gpu
            .as_mut()
            .ok_or("character resources are not created")?;
        if let Some(controls) = r.gpu_controls.take() {
            r.gpu_secondary_last_controls = controls;
            if let Some(palette) = r.gpu_jump_palette.take() {
                r.gpu_jump_last_palette = palette;
            }
        }
        if r.surface_light_audit_encoded
            && let Some(audit) = &mut r.gpu_surface_light_audit
        {
            audit.begin();
        }
        if r.film_layer.is_some() && info.object_active {
            r.optical_frames += 1;
        }
        if info.smoke
            && let Some(film) = self.female.film.as_ref()
        {
            r.film_mass_max_relative_error = r
                .film_mass_max_relative_error
                .max(film.verify_mass_balance(1e-9)?);
            r.film_mass_checks += 1;
        }
        if info.object_active {
            self.female
                .record_presentation(u64::from(info.frame), info.size);
        }
        Ok(())
    }
}

/// Split borrows of the compute state encoded before the scene pass.
struct ComputeEncode<'a> {
    gpu_body: &'a mut Option<crate::female_demo::resident_body::ResidentBody>,
    gpu_hair: &'a mut Option<super::gpu::ResidentHair>,
    gpu_secondary: &'a Option<voxy_render::SurfaceDeformation>,
    gpu_controls: &'a Option<Vec<[f32; 4]>>,
    gpu_jump_palette: &'a Option<Vec<glam::Mat4>>,
    gpu_surface_light: &'a mut Option<voxy_render::SurfaceLightFrame>,
    gpu_surface_colors: &'a Option<voxy_render::SurfaceColorTransfer>,
    gpu_surface_light_audit: &'a Option<super::gpu::SurfaceLightAudit>,
    surface_light_audit_encoded: &'a mut bool,
    update_gpu: bool,
    light_enabled: bool,
}

impl ComputeEncode<'_> {
    fn run(
        self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RendererError> {
        if let Some(body) = self.gpu_body {
            body.encode(encoder).map_err(RendererError::Scene)?;
        }
        if let Some(hair) = self.gpu_hair
            && hair.dirty
        {
            hair.job
                .encode_step(encoder, [hair.input.vertex_count().div_ceil(64), 1, 1])
                .map_err(|_| RendererError::Scene(SceneError::InvalidGeometry))?;
            hair.transfer.encode(encoder);
            hair.dirty = false;
        }
        if self.update_gpu
            && let (Some(gpu), Some(controls)) = (self.gpu_secondary, self.gpu_controls)
        {
            if let Some(palette) = self.gpu_jump_palette {
                gpu.upload_rigid_pose(queue, palette)
                    .map_err(RendererError::Scene)?;
            }
            gpu.upload_controls(queue, controls)
                .map_err(RendererError::Scene)?;
            gpu.encode(encoder).map_err(RendererError::Scene)?;
        }
        if let Some(light) = self.gpu_surface_light
            && self.light_enabled
        {
            light.encode(encoder, 64).map_err(RendererError::Scene)?;
            if let Some(colors) = self.gpu_surface_colors {
                colors.encode(encoder);
            }
            if let Some(audit) = self.gpu_surface_light_audit
                && audit.pending.is_none()
            {
                encoder.copy_buffer_to_buffer(
                    light.admission().header_buffer(),
                    0,
                    &audit.buffer,
                    0,
                    32,
                );
                *self.surface_light_audit_encoded = true;
            }
        }
        Ok(())
    }
}
