use std::sync::Arc;

use super::{WorldRayQueryJob, WorldRayQueryKind};
use crate::{RaySceneError, archive_scene::ArchiveMaterialSource};

impl WorldRayQueryJob {
    /// Conservative admission property for the immutable packed source table.
    /// This enables bounded nearest traversal only; false always keeps all nodes.
    #[doc(hidden)]
    pub fn packed_material_table_known(words: &[u32]) -> bool {
        if words.len() < 4 || words[2] != 24 {
            return false;
        }
        if words[1] == 0 {
            return true;
        }
        let Some(end) = (words[0] as usize)
            .checked_mul(24)
            .and_then(|n| n.checked_add(4))
        else {
            return false;
        };
        if end > words.len() {
            return false;
        }
        let nonnegative =
            |v: u32| v & 0x7fff_ffff < 0x7f80_0000 && (v >> 31 == 0 || v.trailing_zeros() >= 31);
        words[4..end].chunks_exact(24).all(|m| {
            if !nonnegative(m[3])
                || m[3] & 0x7fff_ffff > 0x3f80_0000
                || !nonnegative(m[14])
                || m[15] & 0x7fff_ffff >= 0x7f80_0000
            {
                return false;
            }
            let image = m[20] != u32::MAX;
            let negative = m[15] >> 31 != 0 && m[15] & 0x7fff_ffff != 0;
            if !negative && image {
                return false;
            }
            if m[15] == 0xc000_0000 && image {
                return nonnegative(m[17]) && m[17] & 0x7fff_ffff > 0x3fc0_0000;
            }
            true
        })
    }
    /// This is a consumer policy proof, never geometric or opacity admission.
    /// Every possible accepted winner must select the same complete legacy path.
    pub fn packed_materials_force_history_fallback(words: &[u32]) -> bool {
        if words.len() < 4 || words[0] == 0 || words[1] != 1 || words[2] != 24 {
            return false;
        }
        let Some(end) = (words[0] as usize)
            .checked_mul(24)
            .and_then(|n| n.checked_add(4))
        else {
            return false;
        };
        if end > words.len() {
            return false;
        }
        words[4..end]
            .chunks_exact(24)
            .all(|m| m[21] == 0 || m[15] == 0xc0000000 || (m[14] & 0x7fffffff) != 0)
    }
    pub const fn node_reciprocal_enabled(&self) -> bool {
        self.node_reciprocal
    }
    pub const fn native_source_witness_enabled(&self) -> bool {
        self.native_source_witness
    }
    #[doc(hidden)]
    /// Actual admitted scheduling tail, not merely the requested option.
    pub const fn wide_source_near_first_enabled(&self) -> bool {
        self.wide_source_near_first
    }
    pub const fn wide_source_nodes_enabled(&self) -> bool {
        self.wide_source_base != 0
    }
    pub const fn wide_source_digest(&self) -> [u8; 32] {
        self.wide_source_digest
    }
    pub const fn skip_zero_prefix_multiply_enabled(&self) -> bool {
        self.skip_zero_prefix_multiply
    }
    #[doc(hidden)]
    pub const fn exact_integer_source_digest(&self) -> [u8; 32] {
        self.exact_integer_digest
    }
    pub const fn history_any_hit_enabled(&self) -> bool {
        self.history_any_hit
    }
    /// Diagnostic derived-table header and storage; original geometry stays authoritative.
    pub fn geometry_bounds_buffer(&self) -> Option<&wgpu::Buffer> {
        self.geometry_bounds.as_ref().map(|c| &*c.buffer)
    }
    pub fn exact_vertex_cache_buffer(&self) -> Option<&wgpu::Buffer> {
        self.vertex_cache.as_ref().map(|cache| &*cache.buffer)
    }
    pub fn exact_vertex_cache_header(&self) -> Option<u32> {
        self.vertex_cache.as_ref().map(|cache| cache.header)
    }
    /// Actual per-stage resource shape; full traversal keeps its 512-slot pool.
    pub fn exact_winner_workspace(&self) -> Option<(u32, u32)> {
        self.winner
            .as_ref()
            .map(|_| (self.winner_slots, self.winner_workers))
    }
    pub fn geometry_bounds_bytes(&self) -> u64 {
        self.geometry_bounds.as_ref().map_or(0, |c| c.buffer.size())
    }
    /// Build retained acceleration structures and optional derived bounds in order.
    /// Encode once for a new owner; subsequent queries reuse the completed build.
    pub fn build_scene(&self, encoder: &mut wgpu::CommandEncoder) {
        self.scene.build(encoder);
        if let Some(cache) = &self.geometry_bounds {
            cache.encode(encoder);
        }
        if let Some(cache) = &self.vertex_cache {
            cache.encode(encoder);
        }
    }
    pub(crate) fn producer_input(
        &self,
        device: &wgpu::Device,
    ) -> Result<Arc<crate::ComputeStorage>, RaySceneError> {
        self.scene.validate_device(device)?;
        if !self.guarded_input {
            return Err(RaySceneError::InvalidGeometry);
        }
        Ok(self.input.clone())
    }
    pub(crate) fn source_material_words(&self) -> &wgpu::Buffer {
        &self._hierarchy
    }
    pub fn input(&self) -> &wgpu::Buffer {
        &self.input
    }
    /// Record width is 20 words, or 28 with exact UV output enabled.
    pub const fn output_record_words(&self) -> u32 {
        self.output_words
    }
    /// Visibility hits identify a definite blocker, not the nearest surface; no t/u/v is returned.
    pub const fn query_kind(&self) -> WorldRayQueryKind {
        self.query_kind
    }
    /// Optional aligned tail in `output`: eight words per ray: X hi/lo, Y hi/lo,
    /// Z hi/lo, XYZ validity mask, XYZ nonzero rational residual mask.
    /// Position is evaluated before rounding t. A remaining residual is explicit.
    /// Misses, inactive and unresolved queries have an empty position record.
    pub const fn world_position_offset_bytes(&self) -> Option<u64> {
        self.position_offset
    }
    pub fn world_position_binding(&self) -> Option<wgpu::BufferBinding<'_>> {
        self.position_offset.map(|offset| wgpu::BufferBinding {
            buffer: &self.output,
            offset,
            size: std::num::NonZeroU64::new(u64::from(self.capacity) * 32),
        })
    }
    /// Primary records retain `output_record_words`; optional position tail follows.
    pub fn output(&self) -> &wgpu::Buffer {
        &self.output
    }
    /// Allocated exact workspace; opaque visibility caps this at 4 MiB.
    pub fn workspace_bytes(&self) -> u64 {
        self._scratch.size()
    }
    pub const fn deferred_visibility_enabled(&self) -> bool {
        self.fast.is_some() && matches!(self.query_kind, WorldRayQueryKind::OpaqueVisibility)
    }
    pub const fn exact_worker_count(&self) -> u32 {
        self.exact_workers
    }
    pub const fn exact_workgroup_size(&self) -> u32 {
        self.exact_group_size
    }
    pub const fn capacity(&self) -> u32 {
        self.capacity
    }
    /// Order producer -> this pass -> consumer before reusing this job's storage.
    /// No submit, poll, map, filesystem access or CPU decision occurs here.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        count: u32,
    ) -> Result<(), RaySceneError> {
        self.encode_profiled(encoder, count, None)
    }
    pub(crate) fn nearest_profile_stage_count(&self) -> u32 {
        4 + u32::from(self.history_filter.is_some()) + u32::from(self.native_hint.is_some())
    }
    pub(crate) fn nearest_profile_has_native_witness(&self) -> bool {
        self.native_source_witness
    }
    pub(crate) fn nearest_profile_has_native_hint(&self) -> bool {
        self.native_hint.is_some()
    }
    pub(crate) fn can_profile_nearest(&self) -> bool {
        self.traversal_pair.is_none()
            && self.fast.is_some()
            && self.winner.is_some()
            && self.candidates.is_some()
    }
    pub(crate) fn encode_profiled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        count: u32,
        profile: Option<(&wgpu::QuerySet, u32)>,
    ) -> Result<(), RaySceneError> {
        if let Some(pair) = &self.traversal_pair {
            if profile.is_some() {
                return Err(RaySceneError::Unsupported);
            }
            self.encode_once(encoder, count, None, Some(&pair.original_fast))?;
            if count == 0 {
                return Ok(());
            }
            pair.snapshot(encoder, &self.input, &self.output);
            self.encode_once(encoder, count, None, self.fast.as_ref())?;
            pair.compare(&self.device, encoder, &self.input, &self.output, count);
            return Ok(());
        }
        self.encode_once(encoder, count, profile, self.fast.as_ref())
    }
    /// Cumulative diagnostic record count and first protected-word failure.
    /// No CPU readback occurs during encoding; the caller must fence before reading.
    pub(crate) fn same_producer_original_buffers(&self) -> Option<(&wgpu::Buffer, &wgpu::Buffer)> {
        self.traversal_pair
            .as_ref()
            .map(super::traversal_pair::WorldTraversalPair::original_buffers)
    }
    pub fn same_producer_comparison_buffer(&self) -> Option<&wgpu::Buffer> {
        self.traversal_pair.as_ref().map(|pair| &*pair.stats)
    }
    pub fn read_same_producer_comparison(
        &self,
        queue: &wgpu::Queue,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
        self.traversal_pair
            .as_ref()
            .map(|pair| {
                let mut proof = pair.read_proof(&self.device, queue)?;
                proof["output_record_words"] = self.output_words.into();
                proof["explicit_producer_admission_required"] = self.guarded_input.into();
                proof["world_position_extension_checked"] = self.position_offset.is_some().into();
                proof["source_geometry_revision"] = self.scene.geometry_revision().into();
                proof["same_exact_integer_pipeline"] = true.into();
                proof["expected_query_results_uploaded"] = false.into();
                proof["exact_integer_module_blake3"] = serde_json::json!(self.exact_integer_digest);
                proof["wide_source_near_first"] = self.wide_source_near_first.into();
                proof["wide_source_block_blake3"] = serde_json::json!(self.wide_source_digest);
                Ok(proof)
            })
            .transpose()
    }
    /// Number of distinct restricted input bindings retained by this query.
    #[doc(hidden)]
    pub fn cached_partial_bind_group_count(&self) -> usize {
        self.partial_groups.lock().unwrap().len()
    }

    fn partial_group(
        &self,
        pipeline: &wgpu::ComputePipeline,
        count: u32,
        create: impl FnOnce() -> Result<wgpu::BindGroup, RaySceneError>,
    ) -> Result<wgpu::BindGroup, RaySceneError> {
        // The job owns immutable resources. Each pipeline and record count
        // determines one restricted view, including the comparison pipeline.
        const MAX_CACHED_GROUPS: usize = 32;
        let mut groups = self.partial_groups.lock().unwrap();
        if let Some((_, _, group)) = groups
            .iter()
            .find(|(owner, size, _)| owner == pipeline && *size == count)
        {
            return Ok(group.clone());
        }
        let group = create()?;
        if groups.len() < MAX_CACHED_GROUPS {
            groups.push((pipeline.clone(), count, group.clone()));
        }
        Ok(group)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn encode_once(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        count: u32,
        profile: Option<(&wgpu::QuerySet, u32)>,
        fast: Option<&(wgpu::ComputePipeline, wgpu::BindGroup)>,
    ) -> Result<(), RaySceneError> {
        if profile.is_some() && !self.can_profile_nearest() {
            return Err(RaySceneError::Unsupported);
        }
        let timestamps = |stage: u32| {
            profile.map(|(set, base)| wgpu::ComputePassTimestampWrites {
                query_set: set,
                beginning_of_pass_write_index: Some(base + stage * 2),
                end_of_pass_write_index: Some(base + stage * 2 + 1),
            })
        };
        if count > self.capacity {
            return Err(RaySceneError::Capacity);
        }
        if count == 0 {
            return Ok(());
        }
        // A restricted view conveys the actual batch size without CPU-side ray
        // selection or a mutable shared uniform; successive encodes can differ.
        let partial_group = if (self.query_kind == WorldRayQueryKind::OpaqueVisibility
            || self.fast.is_some())
            && count != self.capacity
        {
            Some(self.partial_group(&self.pipeline, count, || {
                Self::make_group(
                    &self.device,
                    &self.scene,
                    &self.pipeline,
                    &self.input,
                    &self.output,
                    &self._scratch,
                    &self._counts,
                    &self._hierarchy,
                    self.query_kind == WorldRayQueryKind::OpaqueVisibility,
                    count,
                    false,
                    None,
                    self.vertex_cache.as_ref(),
                )
            })?)
        } else {
            None
        };
        let native_stage = u32::from(self.native_hint.is_some());
        if let Some((pipeline, group)) = &self.native_hint {
            let partial = if count != self.capacity {
                Some(self.partial_group(pipeline, count, || {
                    if self.native_source_witness {
                        Self::make_group(
                            &self.device,
                            &self.scene,
                            pipeline,
                            &self.input,
                            &self.output,
                            &self._scratch,
                            &self._counts,
                            &self._hierarchy,
                            false,
                            count,
                            true,
                            None,
                            self.vertex_cache.as_ref(),
                        )
                    } else {
                        Self::make_hint_group(
                            &self.device,
                            &self.scene,
                            pipeline,
                            &self.input,
                            &self.output,
                            &self._counts,
                            count,
                            self.native_triangle_hint,
                        )
                    }
                })?)
            } else {
                None
            };
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("isolated hardware hint before source coverage"),
                timestamp_writes: timestamps(0),
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, partial.as_ref().unwrap_or(group), &[]);
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        if let Some((pipeline, group)) = fast {
            // Reset only reserved padding. Coefficients, admission and IDs are intact.
            encoder.clear_buffer(&self.input, 28, Some(4));
            encoder.clear_buffer(&self.input, 44, Some(4));
            let partial = if count != self.capacity {
                Some(self.partial_group(pipeline, count, || {
                    Self::make_group(
                        &self.device,
                        &self.scene,
                        pipeline,
                        &self.input,
                        &self.output,
                        &self._scratch,
                        &self._counts,
                        &self._hierarchy,
                        !self.native_seed,
                        count,
                        true,
                        self.geometry_bounds.as_ref(),
                        self.vertex_cache.as_ref(),
                    )
                })?)
            } else {
                None
            };
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("certificate-only visibility before exact queue"),
                timestamp_writes: timestamps(native_stage),
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, partial.as_ref().unwrap_or(group), &[]);
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        if let Some((pipeline, group)) = &self.history_filter {
            // Rebuild only the padding queue after the exact render-only filter.
            // No coefficient, admission, source identity or unrequested tail changes.
            encoder.clear_buffer(&self.input, 28, Some(4));
            let partial = if count != self.capacity {
                Some(self.partial_group(pipeline, count, || {
                    Self::make_group(
                        &self.device,
                        &self.scene,
                        pipeline,
                        &self.input,
                        &self.output,
                        &self._scratch,
                        &self._counts,
                        &self._hierarchy,
                        true,
                        count,
                        true,
                        None,
                        self.vertex_cache.as_ref(),
                    )
                })?)
            } else {
                None
            };
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("source-history admission and queue compaction"),
                timestamp_writes: timestamps(1 + native_stage),
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, partial.as_ref().unwrap_or(group), &[]);
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        let admission_stage = u32::from(self.history_filter.is_some()) + native_stage;
        for (index, (stage, label)) in [
            (&self.winner, "exact certified nearest primitives"),
            (&self.candidates, "exact bounded nearest candidates"),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some((pipeline, group)) = stage {
                let partial = if count != self.capacity {
                    Some(self.partial_group(pipeline, count, || {
                        Self::make_group(
                            &self.device,
                            &self.scene,
                            pipeline,
                            &self.input,
                            &self.output,
                            &self._scratch,
                            &self._counts,
                            &self._hierarchy,
                            true,
                            count,
                            false,
                            None,
                            self.vertex_cache.as_ref(),
                        )
                    })?)
                } else {
                    None
                };
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some(label),
                        timestamp_writes: timestamps(index as u32 + 1 + admission_stage),
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, partial.as_ref().unwrap_or(group), &[]);
                    pass.dispatch_workgroups(
                        count
                            .min(if index == 0 {
                                self.winner_workers
                            } else {
                                self.exact_workers
                            })
                            .div_ceil(self.exact_group_size),
                        1,
                        1,
                    );
                }
                encoder.clear_buffer(&self.input, 44, Some(4));
            }
        }
        if let Some((pipeline, group, stats)) = &self.work_stats {
            let partial = if count != self.capacity {
                Some(self.partial_group(pipeline, count, || {
                    Ok(Self::make_work_stats_group(
                        &self.device,
                        pipeline,
                        &self.input,
                        &self.output,
                        stats,
                        count,
                    ))
                })?)
            } else {
                None
            };
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("diagnostic software traversal work"),
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, partial.as_ref().unwrap_or(group), &[]);
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("exact source ray traversal"),
            timestamp_writes: timestamps(3 + admission_stage),
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, partial_group.as_ref().unwrap_or(&self.group), &[]);
        pass.dispatch_workgroups(
            if self.query_kind.supports_surface_nearest() && self.fast.is_some() {
                count
                    .min(self.exact_workers)
                    .div_ceil(self.exact_group_size)
            } else if self.query_kind == WorldRayQueryKind::OpaqueVisibility {
                count.min(16)
            } else {
                count
            },
            1,
            1,
        );
        drop(pass);
        Ok(())
    }
}

pub(super) fn pack_materials(
    materials: &[ArchiveMaterialSource],
    image_count: usize,
) -> Result<Vec<u32>, RaySceneError> {
    let count = u32::try_from(materials.len()).map_err(|_| RaySceneError::Capacity)?;
    let mut words = vec![count, 1, 24, 0];
    for material in materials {
        if bytemuck::cast_slice::<_, f32>(std::slice::from_ref(&material.parameters))
            .iter()
            .any(|v| !v.is_finite())
            || material
                .images
                .iter()
                .flatten()
                .any(|&i| i >= image_count || u32::try_from(i).is_err())
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        words.extend_from_slice(bytemuck::cast_slice(std::slice::from_ref(
            &material.parameters,
        )));
        words.extend([
            material.images[0].map_or(u32::MAX, |i| i as u32),
            u32::from(material.double_sided),
            0,
            0,
        ]);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive_scene::ArchiveMaterial;
    #[test]
    fn source_material_transport_preserves_low_bits_and_signed_zero() {
        let mut p = ArchiveMaterial::metallic_roughness([1.; 4], 0., 0.1, 0., -1.);
        p.diffuse[0] = f32::from_bits(1);
        p.diffuse[1] = -0.;
        p.diffuse[2] = f32::from_bits(0x3f800001);
        let source = ArchiveMaterialSource {
            parameters: p,
            images: [Some(0), None, None, None],
            double_sided: true,
        };
        let words = pack_materials(&[source], 1).unwrap();
        assert_eq!(&words[..4], &[1, 1, 24, 0]);
        assert_eq!(&words[4..7], &[1, 0x80000000, 0x3f800001]);
        assert_eq!(&words[24..28], &[0, 1, 0, 0]);
    }
    #[test]
    fn nearest_bound_requires_every_opacity_policy_to_be_known() {
        let mut words = vec![2, 1, 24, 0];
        for _ in 0..2 {
            let mut material = [0u32; 24];
            material[3] = 1f32.to_bits();
            material[15] = (-1f32).to_bits();
            material[20] = u32::MAX;
            words.extend(material);
        }
        assert!(WorldRayQueryJob::packed_material_table_known(&words));
        // Even a far-away unknown mask must disable global distance pruning.
        words[28 + 15] = 0.5f32.to_bits();
        words[28 + 20] = 0;
        assert!(!WorldRayQueryJob::packed_material_table_known(&words));
        words[28 + 15] = (-2f32).to_bits();
        words[28 + 17] = 1.5f32.to_bits();
        assert!(!WorldRayQueryJob::packed_material_table_known(&words));
        words[28 + 17] += 1;
        assert!(WorldRayQueryJob::packed_material_table_known(&words));
        words[28 + 14] = f32::INFINITY.to_bits();
        assert!(!WorldRayQueryJob::packed_material_table_known(&words));
        assert!(!WorldRayQueryJob::packed_material_table_known(&words[..30]));
    }
    #[test]
    fn history_existence_requires_every_material_to_force_consumer_fallback() {
        let mut a = ArchiveMaterialSource {
            parameters: ArchiveMaterial::MIRROR,
            images: [None; 4],
            double_sided: false,
        };
        let mut b = ArchiveMaterialSource {
            parameters: a.parameters,
            images: a.images,
            double_sided: a.double_sided,
        };
        let copy = |a: &ArchiveMaterialSource| ArchiveMaterialSource {
            parameters: a.parameters,
            images: a.images,
            double_sided: a.double_sided,
        };
        let check = |a: &ArchiveMaterialSource, b: &ArchiveMaterialSource| {
            WorldRayQueryJob::packed_materials_force_history_fallback(
                &pack_materials(&[copy(a), copy(b)], 0).unwrap(),
            )
        };
        assert!(check(&a, &b));
        b.double_sided = true;
        assert!(!check(&a, &b));
        b.parameters.surface[2] = 0.5;
        assert!(check(&a, &b));
        a.double_sided = true;
        assert!(!check(&a, &b));
        a.parameters.surface[3] = -2.;
        assert!(check(&a, &b));
        assert!(!WorldRayQueryJob::packed_materials_force_history_fallback(
            &[]
        ));
        assert!(!WorldRayQueryJob::packed_materials_force_history_fallback(
            &[0, 1, 24, 0]
        ));
        assert!(!WorldRayQueryJob::packed_materials_force_history_fallback(
            &[2, 1, 24, 0]
        ));
    }
    #[test]
    fn invalid_source_material_is_not_substituted() {
        let mut source = ArchiveMaterialSource {
            parameters: ArchiveMaterial::GLASS,
            images: [Some(1), None, None, None],
            double_sided: true,
        };
        assert!(pack_materials(std::slice::from_ref(&source), 1).is_err());
        source.images = [None; 4];
        source.parameters.diffuse[0] = f32::NAN;
        assert!(pack_materials(&[source], 0).is_err());
    }
}
