use super::types::{display_mode, effects_off, twilight_sky};
use super::{ArchiveSceneRenderer, Camera};
use glam::{Mat4, Vec3, Vec4};

impl ArchiveSceneRenderer {
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[must_use]
    pub const fn hdr_view(&self) -> &wgpu::TextureView {
        &self.hdr_view
    }

    #[must_use]
    pub const fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Spatially supersampled temporal AA. The renderer dimensions must match
    /// the resolve's render size. Invalid targets leave scene/history untouched.
    pub fn draw_supersampled(
        &mut self,
        resolve: &crate::SupersampleResolve,
        output: &wgpu::TextureView,
        eye: Vec3,
        direction: Vec3,
    ) -> Result<wgpu::SubmissionIndex, crate::ComputeError> {
        resolve.validate(output)?;
        if self.device != resolve.device || (self.width, self.height) != resolve.render_size() {
            return Err(crate::ComputeError::InvalidBuffer);
        }
        let linear = self.linear_output;
        self.set_linear_output(true);
        self.draw(&resolve.view, eye, direction);
        self.set_linear_output(linear);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        resolve.encode(
            &self.queue,
            &mut encoder,
            output,
            if linear {
                3.
            } else {
                display_mode(
                    output.texture().format() == wgpu::TextureFormat::Rgba16Float,
                    self.tone_mapper,
                )
            },
            self.display_headroom,
        );
        Ok(self.queue.submit([encoder.finish()]))
    }

    pub fn draw(
        &mut self,
        output: &wgpu::TextureView,
        eye: Vec3,
        direction: Vec3,
    ) -> wgpu::SubmissionIndex {
        self.draw_with_tail(output, eye, direction, |_, _| {})
    }

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub(super) fn draw_with_tail(
        &mut self,
        output: &wgpu::TextureView,
        eye: Vec3,
        direction: Vec3,
        encode_tail: impl FnOnce(&Self, &mut wgpu::CommandEncoder),
    ) -> wgpu::SubmissionIndex {
        self.collect_gpu_time_sample();
        let time_this_frame = self.timing.as_ref().is_some_and(|t| !t.pending);
        #[allow(deprecated)]
        let vp = Mat4::perspective_rh(
            self.vertical_fov,
            self.width as f32 / self.height as f32,
            0.05,
            1000.,
        ) * Mat4::look_at_rh(eye, eye + direction, self.camera_up);
        let rows = vp.transpose().to_cols_array_2d().map(Vec4::from_array);
        // Keep the entire temporal-jitter footprint inside the conservative
        // culling volume; otherwise edge leaves may disappear between AA samples.
        let margin = if self.temporal.is_some() { 2. } else { 0. };
        let horizontal = rows[3] * (1. + margin / self.width as f32);
        let vertical = rows[3] * (1. + margin / self.height as f32);
        let planes = [
            horizontal + rows[0],
            horizontal - rows[0],
            vertical + rows[1],
            vertical - rows[1],
            rows[2],
            rows[3] - rows[2],
        ];
        let raw_planes = planes;
        let planes = planes.map(|p| (p, p.truncate().length()));
        for mesh in &mut self.meshes {
            if let Some(culling) = &mesh.culling {
                culling.prepare(&self.queue, raw_planes);
                mesh.visible_count = mesh.copies;
                continue;
            }
            mesh.frustum.collect(&planes, &mut mesh.scratch_ids);
            let ids = &mut mesh.scratch_ids;
            if self.materials[mesh.material].transparent {
                ids.sort_by(|&a, &b| {
                    mesh.spheres[b]
                        .0
                        .distance_squared(eye)
                        .total_cmp(&mesh.spheres[a].0.distance_squared(eye))
                });
            }
            if *ids != mesh.visible_ids {
                if !ids.is_empty() {
                    mesh.scratch_instances.clear();
                    mesh.scratch_instances
                        .extend(ids.iter().map(|&i| mesh.source_instances[i]));
                    self.queue.write_buffer(
                        &mesh.visible_instances,
                        0,
                        bytemuck::cast_slice(&mesh.scratch_instances),
                    );
                }
                mesh.visible_count = ids.len() as u32;
                std::mem::swap(&mut mesh.visible_ids, &mut mesh.scratch_ids);
            }
        }
        let mut jittered_vp = vp;
        let output_mode = self.output_mode();
        if let Some(temporal) = &mut self.temporal {
            let jitter = temporal.jitter();
            let offset = glam::Vec2::new(
                2. * jitter.x / self.width as f32,
                -2. * jitter.y / self.height as f32,
            );
            jittered_vp = Mat4::from_translation(Vec3::new(offset.x, offset.y, 0.)) * vp;
            temporal.prepare(
                &self.queue,
                vp,
                jittered_vp,
                eye,
                direction,
                output_mode,
                self.display_headroom,
            );
        }
        self.queue.write_buffer(
            &self.camera,
            0,
            bytemuck::bytes_of(&Camera {
                vp: jittered_vp.to_cols_array_2d(),
                shadow: self.shadow_matrix.to_cols_array_2d(),
                inverse_vp: jittered_vp.inverse().to_cols_array_2d(),
                eye: [eye.x, eye.y, eye.z, f32::from(self.environment_enabled)],
                sun: [self.sun.x, self.sun.y, self.sun.z, 0.],
                sun_colour: {
                    let mut c = self.sun_colour;
                    c[3] *= (1. - self.weather.coverage * 0.75) * (1. - 0.9 * self.weather.rain);
                    c
                },
                weather: [
                    self.weather.coverage,
                    self.weather.density,
                    self.weather.rain,
                    self.weather.snow,
                ],
                atmosphere: [
                    self.weather_time,
                    self.weather.cloud_base,
                    self.weather.cloud_thickness,
                    0.,
                ],
                wind: [
                    self.weather.wind[0],
                    self.weather.wind[1],
                    self.weather.wind_gust,
                    effects_off(),
                ],
                previous_vp: self.previous_vp.unwrap_or(jittered_vp).to_cols_array_2d(),
                previous_inverse_vp: self
                    .previous_vp
                    .unwrap_or(jittered_vp)
                    .inverse()
                    .to_cols_array_2d(),
                surface: [
                    self.surface_wetness(),
                    self.puddle_coverage,
                    f32::from(u8::from(self.previous_vp.is_some())),
                    2. * (self.vertical_fov * 0.5).tan() / self.height as f32,
                ],
                viewport: [
                    self.width as f32,
                    self.height as f32,
                    self.exposure * self.adapted_exposure_scale * self.exposure_bias_ev.exp2(),
                    self.sky_strength * twilight_sky(self.sun.y),
                ],
            }),
        );
        // `background` receives this frame's opaque radiance before transparents;
        // the next frame's opaque reflections reproject into it.
        let ray_history_available = self.previous_vp.is_some();
        if ray_history_available && self.opaque_ray_queue_enabled() {
            self.ray_queue_frame_count += 1;
        }
        if time_this_frame && let Some(timing) = &mut self.timing {
            timing.sample_chunks = if ray_history_available
                && !self
                    .compute_gi
                    .as_ref()
                    .is_some_and(super::compute_gi::ArchiveComputeGi::world_frame_enabled)
            {
                timing.phase_chunks
            } else {
                0
            };
        }
        self.previous_vp = Some(jittered_vp);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for mesh in &self.meshes {
            if let Some(culling) = &mesh.culling {
                culling.encode(&mut encoder);
            }
        }
        if !self.shadow_ready {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("static authored sun shadows"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.camera_bind, &[]);
            for mesh in &self.meshes {
                if self.materials[mesh.material].transparent {
                    continue;
                }
                pass.set_bind_group(1, &self.materials[mesh.material].bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.count, 0, 0..mesh.copies);
            }
            drop(pass);
            self.shadow_ready = true;
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("exact opaque visibility prepass"),
                color_attachments: &[],
                timestamp_writes: if time_this_frame {
                    self.timing
                        .as_ref()
                        .map(|t| wgpu::RenderPassTimestampWrites {
                            query_set: &t.set,
                            beginning_of_pass_write_index: Some(0),
                            end_of_pass_write_index: None,
                        })
                } else {
                    None
                },
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.camera_bind, &[]);
            for mesh in &self.meshes {
                let material = &self.materials[mesh.material];
                if material.transparent || mesh.visible_count == 0 {
                    continue;
                }
                pass.set_pipeline(if material.double_sided {
                    &self.depth_double
                } else {
                    &self.depth_pipeline
                });
                pass.set_bind_group(1, &material.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.visible_instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(culling) = &mesh.culling {
                    pass.draw_indexed_indirect(culling.indirect(), 0);
                } else {
                    pass.draw_indexed(0..mesh.count, 0, 0..mesh.visible_count);
                }
            }
        }
        self.depth_bounds.encode(&mut encoder);
        if let Some(bounds) = &self.pixel_depth_bounds {
            bounds.encode(&mut encoder);
        }
        let compute_active = self.compute_gi_active_for_frame();
        if let Some(gi) = self.compute_gi.as_ref().filter(|_| compute_active) {
            gi.encode(
                &mut encoder,
                (&self.device, &self.queue),
                &self.depth_view,
                &self.camera_bind,
                &self.frame_bind,
                &self.environment_bindings.group,
                &self.materials,
                &self.meshes,
                if time_this_frame {
                    self.timing.as_ref().map(|t| &t.set)
                } else {
                    None
                },
                ray_history_available,
            );
        }
        // Keep opaque scene depth immutable: refraction and GI still sample it.
        if let Some(texture) = &self.transmission_depth {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.depth,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::DepthOnly,
                },
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::DepthOnly,
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let has_transmission_view = self.transmission_depth_view.is_some();
        if let Some(view) = &self.transmission_depth_view {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fully covering transmission visibility"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.camera_bind, &[]);
            for mesh in &self.meshes {
                let material = &self.materials[mesh.material];
                if !material.fully_covering || mesh.visible_count == 0 {
                    continue;
                }
                pass.set_pipeline(if material.double_sided {
                    &self.depth_double
                } else {
                    &self.depth_pipeline
                });
                pass.set_bind_group(1, &material.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.visible_instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(culling) = &mesh.culling {
                    pass.draw_indexed_indirect(culling.indirect(), 0);
                } else {
                    pass.draw_indexed(0..mesh.count, 0, 0..mesh.visible_count);
                }
            }
        }
        for transparent in [false, true] {
            if transparent {
                if let Some(water) = &mut self.water {
                    water.encode(
                        &self.queue,
                        &mut encoder,
                        &crate::archive_water::WaterFrame {
                            hdr: &self.hdr,
                            background: &self.background,
                            depth: &self.depth,
                            camera: &self.camera_bind,
                            environment: &self.environment_bindings.group,
                            eye,
                            planes: raw_planes,
                        },
                    );
                }
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.hdr,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.background,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: self.width,
                        height: self.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            self.draw_order.clear();
            self.draw_order
                .extend(self.meshes.iter().enumerate().filter_map(|(i, m)| {
                    (m.visible_count > 0 && self.materials[m.material].transparent == transparent)
                        .then_some(i)
                }));
            if transparent {
                let meshes = &self.meshes;
                let materials = &self.materials;
                self.draw_order.sort_by(|&ia, &ib| {
                    let a = &meshes[ia];
                    let b = &meshes[ib];
                    let coverage_order = if has_transmission_view {
                        materials[b.material]
                            .fully_covering
                            .cmp(&materials[a.material].fully_covering)
                    } else {
                        std::cmp::Ordering::Equal
                    };
                    coverage_order.then_with(|| {
                        b.center
                            .distance_squared(eye)
                            .total_cmp(&a.center.distance_squared(eye))
                    })
                });
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if transparent {
                    "authored transparent surfaces"
                } else {
                    "authored opaque surfaces"
                }),
                timestamp_writes: if time_this_frame {
                    self.timing
                        .as_ref()
                        .map(|t| wgpu::RenderPassTimestampWrites {
                            query_set: &t.set,
                            beginning_of_pass_write_index: Some(if transparent { 3 } else { 1 }),
                            end_of_pass_write_index: Some(if transparent { 4 } else { 2 }),
                        })
                } else {
                    None
                },
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if transparent {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.22,
                                g: 0.35,
                                b: 0.55,
                                a: 1.,
                            })
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: if transparent {
                        self.transmission_depth_view
                            .as_ref()
                            .unwrap_or(&self.depth_view)
                    } else {
                        &self.depth_view
                    },
                    depth_ops: None,
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.camera_bind, &[]);
            pass.set_bind_group(2, &self.frame_bind, &[]);
            pass.set_bind_group(3, &self.environment_bindings.group, &[]);
            let alternate = self
                .refine_crossover
                .is_some_and(|mode| mode.label_b(self.benchmark_frame));
            for &mesh_index in &self.draw_order {
                let mesh = &self.meshes[mesh_index];
                let material = &self.materials[mesh.material];
                let ordinary_pipeline =
                    if transparent && material.fully_covering && has_transmission_view {
                        if material.double_sided {
                            &self.covered_transparent_double
                        } else {
                            &self.covered_transparent
                        }
                    } else {
                        match (transparent, material.double_sided, material.masked) {
                            (false, false, false) => &self.opaque,
                            (false, false, true) => &self.masked,
                            (false, true, false) => &self.opaque_double,
                            (false, true, true) => &self.masked_double,
                            (true, false, _) => &self.transparent,
                            (true, true, _) => &self.transparent_double,
                        }
                    };
                let index = if transparent && material.fully_covering && has_transmission_view {
                    6 + usize::from(material.double_sided)
                } else if transparent {
                    4 + usize::from(material.double_sided)
                } else {
                    2 * usize::from(material.double_sided) + usize::from(material.masked)
                };
                pass.set_pipeline(if alternate {
                    &self
                        .refine_alternates
                        .as_ref()
                        .expect("crossover pipelines")[index]
                } else {
                    ordinary_pipeline
                });
                pass.set_bind_group(1, &material.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.visible_instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(culling) = &mesh.culling {
                    pass.draw_indexed_indirect(culling.indirect(), 0);
                } else {
                    pass.draw_indexed(0..mesh.count, 0, 0..mesh.visible_count);
                }
            }
        }
        let temporal_view = self
            .temporal
            .as_ref()
            .map(|t| t.current.create_view(&Default::default()));
        // Without temporal AA the display pass applies bloom itself; the
        // temporal display pass has its own chain over the resolved image.
        if self.temporal.is_none() {
            self.bloom
                .encode(&self.device, &mut encoder, &self.hdr_view);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("street HDR display"),
                timestamp_writes: if time_this_frame && self.temporal.is_none() {
                    self.timing
                        .as_ref()
                        .map(|t| wgpu::RenderPassTimestampWrites {
                            query_set: &t.set,
                            beginning_of_pass_write_index: None,
                            end_of_pass_write_index: Some(5),
                        })
                } else {
                    None
                },
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: temporal_view.as_ref().unwrap_or(output),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(if self.temporal.is_some() || self.linear_output {
                &self.post_linear
            } else {
                &self.post
            });
            pass.set_bind_group(0, &self.post_bind, &[]);
            pass.set_bind_group(1, &self.environment_bindings.group, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut preceding_commands = Vec::new();
        if let Some(temporal) = &mut self.temporal {
            let timestamps = if time_this_frame {
                self.timing
                    .as_ref()
                    .map(|t| wgpu::RenderPassTimestampWrites {
                        query_set: &t.set,
                        beginning_of_pass_write_index: None,
                        end_of_pass_write_index: Some(5),
                    })
            } else {
                None
            };
            temporal.encode_guides(&self.device, &mut encoder, &self.depth);
            #[cfg(target_os = "macos")]
            if temporal.native() {
                preceding_commands.push(encoder.finish());
                let mut native =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("native MetalFX only"),
                        });
                temporal.encode_native(&mut native, &self.depth);
                preceding_commands.push(native.finish());
                encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("MetalFX final HDR display"),
                    });
            }
            temporal.encode_display(&self.device, &mut encoder, output, timestamps);
        }
        // Save depth only after all old-radiance reads. Both histories then
        // describe the same opaque frame and use the same previous camera.
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.depth,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.history_depth,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        encode_tail(self, &mut encoder);
        preceding_commands.push(encoder.finish());
        let submission = self.queue.submit(preceding_commands);
        if time_this_frame {
            let timing = self.timing.as_mut().unwrap();
            timing.pending = true;
            let label = self
                .refine_crossover
                .is_some_and(|mode| mode.label_b(self.benchmark_frame));
            let cache = self
                .refine_crossover
                .map_or(self.refine_cache_base, |mode| {
                    mode.cache_for_frame(self.benchmark_frame)
                });
            timing.frame_stamp = (self.benchmark_frame, label, cache);
            let ready = timing.ready.clone();
            // Resolve only after counter-producing commands have completed;
            // subsequent rendering continues while the readback is pending.
            self.queue.on_submitted_work_done(move || {
                ready.store(1, std::sync::atomic::Ordering::Release);
            });
        }
        submission
    }
}
