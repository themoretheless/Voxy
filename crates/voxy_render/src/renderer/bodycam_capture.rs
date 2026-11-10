use glam::Vec3;

use super::camera_math::{
    chunk_intersects_frustum, perspective_camera_view_projection, photoreal_camera_uniform,
};
use super::{RenderOutcome, Renderer, RendererError, SurfaceState};
use crate::{BodycamRig, SkinnedUploadError};

impl Renderer {
    pub(super) fn encode_bodycam_passes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        frame_id: u64,
    ) {
        self.encode_bodycam_passes_with_scene(encoder, output_view, frame_id, None);
    }

    pub(super) fn encode_bodycam_passes_with_scene(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        frame_id: u64,
        scene_overlay: Option<(&crate::SceneRenderer, &[crate::SceneDraw<'_>])>,
    ) {
        self.bodycam_post
            .update_from_rig(&self.queue, &self.bodycam_rig, frame_id);
        let mut uniform = photoreal_camera_uniform(
            self.config.width,
            self.config.height,
            self.camera,
            &self.bodycam_rig,
            self.bodycam_flashlight_pos,
            self.bodycam_flashlight_dir,
            self.bodycam_wetness,
        );
        uniform.subject_spheres = self.bodycam_subject_spheres;
        let camera_group = if let Some(shared) = &self.shared_photoreal_camera {
            shared.select(
                encoder,
                &self.photoreal_camera_buffer,
                &self.photoreal_camera_bind_group,
                bytemuck::bytes_of(&uniform),
            )
        } else {
            self.queue.write_buffer(
                &self.photoreal_camera_buffer,
                0,
                bytemuck::bytes_of(&uniform),
            );
            self.photoreal_camera_bind_group.clone()
        };
        {
            // CIE Overcast Sky linear HDR zenith/horizon skybox clear color
            let hdr_sky_clear = wgpu::Color {
                r: 1.32,
                g: 1.44,
                b: 1.62,
                a: 1.0,
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Voxy photoreal HDR voxel pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.bodycam_post.hdr_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(hdr_sky_clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.frame_timing.as_ref().and_then(|t| t.writes()),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if self.quad_count != 0 {
                pass.set_pipeline(&self.photoreal_pipeline);
                pass.set_bind_group(0, &camera_group, &[]);
                pass.set_bind_group(1, &self.material_bind_group, &[]);
                pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
                let view_projection = perspective_camera_view_projection(
                    self.config.width,
                    self.config.height,
                    self.camera,
                );
                for (origin, instances) in &self.chunk_draws {
                    if chunk_intersects_frustum(view_projection, *origin) {
                        pass.draw(0..6, instances.clone());
                    }
                }
            }
            if let Some(skinned) = &self.skinned {
                pass.set_pipeline(&self.photoreal_skinned_pipeline);
                pass.set_bind_group(0, &camera_group, &[]);
                pass.set_bind_group(1, &self.material_bind_group, &[]);
                pass.set_bind_group(2, &skinned.bind_group, &[]);
                pass.set_vertex_buffer(0, skinned.vertex.slice(..));
                pass.set_index_buffer(skinned.index.slice(..), wgpu::IndexFormat::Uint32);
                for range in &skinned.opaque_ranges {
                    pass.draw_indexed(range.clone(), 0, 0..1);
                }
            }
            if let Some(indirect) = &self.indirect_mesh {
                pass.set_pipeline(&self.photoreal_indirect_pipeline);
                pass.set_bind_group(0, &camera_group, &[]);
                pass.set_bind_group(1, &self.material_bind_group, &[]);
                pass.set_vertex_buffer(0, indirect.vertex_buffer.slice(..));
                pass.draw_indirect(&indirect.indirect_buffer, 0);
            }
            if let Some(skinned) = &self.skinned {
                if !skinned.fibre_ranges.is_empty() {
                    pass.set_pipeline(&self.photoreal_fibre_pipeline);
                    pass.set_bind_group(0, &camera_group, &[]);
                    pass.set_bind_group(1, &self.material_bind_group, &[]);
                    pass.set_bind_group(2, &skinned.bind_group, &[]);
                    pass.set_vertex_buffer(0, skinned.vertex.slice(..));
                    pass.set_index_buffer(skinned.index.slice(..), wgpu::IndexFormat::Uint32);
                    for range in &skinned.fibre_ranges {
                        pass.draw_indexed(range.clone(), 0, 0..1);
                    }
                }
            }
        }
        if let Some(plants) = &self.native_vegetation {
            plants.encode(
                &self.device,
                &self.queue,
                encoder,
                self.bodycam_post.hdr_view(),
                &self.depth_view,
                perspective_camera_view_projection(
                    self.config.width,
                    self.config.height,
                    self.camera,
                ),
                self.camera.eye,
                true,
            );
        }
        if let Some((scene, draws)) = scene_overlay {
            scene.encode_over(
                encoder,
                self.bodycam_post.hdr_view(),
                &self.depth_view,
                draws,
            );
        }
        self.bodycam_post.encode(encoder, output_view);
    }

    /// Enable or disable the photorealistic *Bodycam* / *Unrecord* rendering mode
    /// (Perspective HDR PBR + CIE Overcast GI + GTAO Multi-Bounce + Physical Lens/CMOS/ISP).
    pub fn set_bodycam_enabled(&mut self, enabled: bool) {
        if self.bodycam_enabled != enabled {
            self.bodycam_enabled = enabled;
            self.reset_skinned_motion();
        }
    }

    #[must_use]
    pub const fn bodycam_enabled(&self) -> bool {
        self.bodycam_enabled
    }

    #[must_use]
    pub const fn bodycam_rig(&self) -> &BodycamRig {
        &self.bodycam_rig
    }

    pub fn bodycam_rig_mut(&mut self) -> &mut BodycamRig {
        &mut self.bodycam_rig
    }

    /// Updates the tactical flashlight world pose and surface wetness (`0.0..=1.0`).
    /// Analytic sphere proxies used for soft contact occlusion on voxel surfaces.
    /// # Errors
    /// Rejects invalid centers/radii before changing the active proxy set.
    pub fn set_subject_shadow_spheres(
        &mut self,
        spheres: &[[f32; 4]],
    ) -> Result<(), RendererError> {
        if spheres.len() > 12
            || spheres
                .iter()
                .any(|s| s.iter().any(|x| !x.is_finite()) || s[3] < 0.0)
        {
            return Err(RendererError::Skinned(SkinnedUploadError::NonFiniteMatrix));
        }
        let mut next = [[0.0; 4]; 12];
        next[..spheres.len()].copy_from_slice(spheres);
        self.bodycam_subject_spheres = next;
        Ok(())
    }

    pub fn set_bodycam_flashlight_and_wetness(
        &mut self,
        origin: Vec3,
        direction: Vec3,
        wetness: f32,
    ) {
        if origin.is_finite() {
            self.bodycam_flashlight_pos = origin;
        }
        if direction.is_finite() && direction.length_squared() > f32::EPSILON {
            self.bodycam_flashlight_dir = direction.normalize();
        }
        if wetness.is_finite() {
            self.bodycam_wetness = wetness.clamp(0.0, 1.0);
        }
    }

    /// Encodes the full linear HDR PBR + Bodycam Optical/CMOS/ISP post-process frame
    /// into an arbitrary target texture view (matching `surface_format()`), enabling
    /// deterministic headless or offscreen PNG snapshot capture.
    pub fn encode_bodycam_frame_to_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        frame_id: u64,
    ) {
        self.encode_bodycam_passes(encoder, output_view, frame_id);
    }

    #[must_use]
    pub const fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Renders a photorealistic Bodycam HDR PBR + Optical/CMOS/ISP frame into an
    /// offscreen texture and reads back the RGBA8 sRGB bytes `(width, height, rgba8)`.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::DeviceLost`] if the GPU buffer mapping fails.
    pub fn capture_bodycam_rgba8(
        &mut self,
        frame_id: u64,
    ) -> Result<(u32, u32, Vec<u8>), RendererError> {
        self.capture_bodycam_with_scene_rgba8(frame_id, None)
    }

    /// Renders a photorealistic Bodycam HDR PBR voxel pass plus optional 3D `SceneRenderer`
    /// draws (such as the anatomical female character) into the linear HDR target, applies
    /// the physical Lens/CMOS/ISP post-process pass, and reads back RGBA8 sRGB bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::DeviceLost`] if the GPU buffer mapping fails.
    pub fn capture_bodycam_with_scene_rgba8(
        &mut self,
        frame_id: u64,
        scene_overlay: Option<(&crate::SceneRenderer, &[crate::SceneDraw<'_>])>,
    ) -> Result<(u32, u32, Vec<u8>), RendererError> {
        self.capture_world_snapshot(frame_id, scene_overlay, true)
    }

    /// Read back the current world, including vegetation, through its active projection/render mode.
    /// # Errors
    /// Returns GPU or readback failures.
    pub fn capture_world_rgba8(
        &mut self,
        frame_id: u64,
    ) -> Result<(u32, u32, Vec<u8>), RendererError> {
        self.capture_world_snapshot(frame_id, None, self.bodycam_enabled)
    }

    /// Validation capture with the same world/camera and the native vegetation layer temporarily omitted.
    /// # Errors
    /// Returns GPU/readback failures, restoring the vegetation owner on either result.
    pub fn capture_world_without_vegetation_rgba8(
        &mut self,
        frame_id: u64,
    ) -> Result<(u32, u32, Vec<u8>), RendererError> {
        let plants = self.native_vegetation.take();
        let result = self.capture_world_rgba8(frame_id);
        self.native_vegetation = plants;
        result
    }

    fn capture_world_snapshot(
        &mut self,
        frame_id: u64,
        scene_overlay: Option<(&crate::SceneRenderer, &[crate::SceneDraw<'_>])>,
        bodycam: bool,
    ) -> Result<(u32, u32, Vec<u8>), RendererError> {
        self.check_device()?;
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let format = self.config.format;
        let offscreen = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Voxy bodycam snapshot target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = offscreen.create_view(&wgpu::TextureViewDescriptor::default());
        let unpadded_bytes_per_row = width * 4;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Voxy bodycam snapshot readback"),
            size: u64::from(padded_bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Voxy bodycam snapshot encoder"),
            });
        if bodycam {
            self.encode_bodycam_passes_with_scene(&mut encoder, &view, frame_id, scene_overlay);
        } else {
            self.encode_resident_voxels(&mut encoder, &view);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &offscreen,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mapped = slice
            .get_mapped_range()
            .map_err(|err| RendererError::DeviceLost(err.to_string()))?;
        let is_bgra = matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height as usize {
            let start = row * padded_bytes_per_row as usize;
            let row_bytes = &mapped[start..start + unpadded_bytes_per_row as usize];
            if is_bgra {
                for px in row_bytes.chunks_exact(4) {
                    rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                }
            } else {
                rgba.extend_from_slice(row_bytes);
            }
        }
        drop(mapped);
        readback.unmap();
        Ok((width, height, rgba))
    }

    /// Presents a photorealistic Bodycam HDR PBR voxel pass plus optional 3D `SceneRenderer`
    /// draws into the active window surface.
    ///
    /// # Errors
    ///
    /// Returns surface or device-loss errors if presentation fails.
    pub fn render_bodycam_with_scene(
        &mut self,
        scene: &crate::SceneRenderer,
        draws: &[crate::SceneDraw<'_>],
    ) -> Result<RenderOutcome, RendererError> {
        self.check_device()?;
        if self.state == SurfaceState::Suspended {
            return Ok(RenderOutcome::Suspended);
        }
        let frame_id = self
            .resident_presented_frames
            .checked_add(1)
            .ok_or(RendererError::FrameIdExhausted)?;
        let (frame, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderOutcome::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Lost => return Err(RendererError::SurfaceLost),
            wgpu::CurrentSurfaceTexture::Timeout => return Ok(RenderOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(RenderOutcome::SkippedValidation);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Voxy bodycam + scene frame encoder"),
            });
        self.encode_bodycam_passes_with_scene(&mut encoder, &view, frame_id, Some((scene, draws)));
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        self.resident_presented_frames = frame_id;
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
            Ok(RenderOutcome::Reconfigured)
        } else {
            Ok(RenderOutcome::Presented)
        }
    }
}
