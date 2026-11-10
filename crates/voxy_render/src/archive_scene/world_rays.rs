use super::{ArchiveSceneRenderer, compute_gi};
use std::sync::Arc;

pub(super) fn validate_street_ray_env_config(
    device: &wgpu::Device,
) -> Result<(), crate::ComputeError> {
    if std::env::var_os("VOXY_STREET_RAY_PERSISTENT").is_some() {
        if !device.features().contains(wgpu::Features::SUBGROUP)
            || [
                "VOXY_STREET_RAY_QUEUE",
                "VOXY_STREET_RAY_STAGES",
                "VOXY_STREET_PAIRED_DDA",
                "VOXY_STREET_COMPUTE_GI",
            ]
            .iter()
            .any(|key| std::env::var_os(key).is_none())
            || std::env::var_os("VOXY_STREET_RAY_HIERARCHY").is_some()
        {
            return Err(crate::ComputeError::Validation(
                "persistent exact DDA requires subgroup support and queued staged paired compute; select either persistent DDA or pixel-stack traversal".into(),
            ));
        }
    }
    if std::env::var_os("VOXY_STREET_RAY_FORCE_FALLBACK").is_some()
        && std::env::var_os("VOXY_STREET_GI_PRECISION_PROOF").is_none()
    {
        return Err(crate::ComputeError::Validation(
            "forced fallback is reserved for precision qualification".into(),
        ));
    }
    if std::env::var_os("VOXY_STREET_RAY_CAPTURE_DIR").is_some()
        && [
            "VOXY_STREET_RAY_QUEUE",
            "VOXY_STREET_RAY_STAGES",
            "VOXY_STREET_PAIRED_DDA",
            "VOXY_STREET_COMPUTE_GI",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_none())
    {
        return Err(crate::ComputeError::Validation(
            "ray capture requires queued staged paired compute queries".into(),
        ));
    }
    if std::env::var_os("VOXY_STREET_RAY_HIERARCHY_NEAR_FIRST").is_some()
        && std::env::var_os("VOXY_STREET_RAY_HIERARCHY").is_none()
    {
        return Err(crate::ComputeError::Validation(
            "near-first scheduling requires the pixel ray hierarchy".into(),
        ));
    }
    if std::env::var_os("VOXY_STREET_RAY_HIERARCHY_RECIPROCAL").is_some()
        && std::env::var_os("VOXY_STREET_RAY_HIERARCHY").is_none()
    {
        return Err(crate::ComputeError::Validation(
            "reciprocal intervals require the pixel ray hierarchy".into(),
        ));
    }
    if std::env::var_os("VOXY_STREET_RAY_HIERARCHY").is_some()
        && [
            "VOXY_STREET_RAY_QUEUE",
            "VOXY_STREET_RAY_STAGES",
            "VOXY_STREET_PAIRED_DDA",
            "VOXY_STREET_COMPUTE_GI",
            "VOXY_STREET_DDA_HIERARCHY",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_none())
    {
        return Err(crate::ComputeError::Validation(
            "pixel ray hierarchy requires queued staged paired compute and depth hierarchy".into(),
        ));
    }
    if std::env::var_os("VOXY_STREET_RAY_SPATIAL_KEY").is_some()
        && std::env::var_os("VOXY_STREET_RAY_QUEUE").is_none()
    {
        return Err(crate::ComputeError::Validation(
            "spatial ray key requires the global queue".into(),
        ));
    }
    if std::env::var_os("VOXY_STREET_RAY_QUEUE").is_some() {
        if [
            "VOXY_STREET_RAY_STAGES",
            "VOXY_STREET_PAIRED_DDA",
            "VOXY_STREET_COMPUTE_GI",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_none())
        {
            return Err(crate::ComputeError::Validation(
                "ray queue requires staged paired compute queries".into(),
            ));
        }
        if std::env::var_os("VOXY_STREET_RAY_COHERENT").is_some() {
            return Err(crate::ComputeError::Validation(
                "choose global queue or local ray permutation".into(),
            ));
        }
    }
    Ok(())
}

impl ArchiveSceneRenderer {
    /// Access the render graph's actual lighting outputs after a draw fence.
    /// This does not allocate textures or enable reference/comparison passes.
    #[doc(hidden)]
    pub fn opaque_lighting_outputs(&self) -> Option<(&wgpu::Texture, &wgpu::Texture)> {
        self.compute_gi.as_ref()?;
        Some((&self._gi_output, &self._ssr_output))
    }

    /// Paired actual/reference GI textures require the precision diagnostic.
    #[doc(hidden)]
    pub fn opaque_gi_precision_textures(&self) -> Option<(&wgpu::Texture, &wgpu::Texture)> {
        Some((
            &self._gi_output,
            self.compute_gi.as_ref()?.reference_texture()?,
        ))
    }

    #[doc(hidden)]
    pub fn opaque_ssr_precision_textures(&self) -> Option<(&wgpu::Texture, &wgpu::Texture)> {
        Some((
            &self._ssr_output,
            self.compute_gi.as_ref()?.ssr_reference_texture()?,
        ))
    }

    #[doc(hidden)]
    pub fn opaque_ray_stages_configuration(&self) -> Option<[u64; 3]> {
        self.compute_gi.as_ref()?.stages_configuration()
    }

    #[doc(hidden)]
    pub fn transmission_depth_enabled(&self) -> bool {
        self.transmission_depth.is_some()
    }

    /// Prepare a GPU producer from this renderer's actual camera and full-float guides.
    /// Use a guarded exact job; unsupported inputs remain explicitly unresolved.
    pub fn world_ray_producer(
        &self,
        job: &crate::ExactWorldNearest,
        kind: crate::WorldRayKind,
        offset: [u32; 2],
        extent: [u32; 2],
    ) -> Result<crate::ArchiveWorldRayProducer, crate::RaySceneError> {
        let guides = self
            .compute_gi
            .as_ref()
            .ok_or(crate::RaySceneError::Unsupported)?
            .world_ray_guides();
        crate::ArchiveWorldRayProducer::new(
            &self.device,
            self.camera.clone(),
            guides,
            job,
            kind,
            offset,
            extent,
        )
    }

    /// Connect original-source nearest queries to a live GI/reflection region.
    /// The caller supplies the immutable matching world/material snapshot and must
    /// build its scene before drawing. Retain/recreate regions after scene changes.
    /// This opt-in integration preserves the full legacy query for unknown layers,
    /// unrepresented rational distance tails and unsupported projections. It is
    /// not a speed guarantee: the existing whole-frame pass is still retained.
    pub fn add_world_ray_region(
        &mut self,
        job: Arc<crate::ExactWorldNearest>,
        kind: crate::WorldRayKind,
        offset: [u32; 2],
        extent: [u32; 2],
    ) -> Result<(), crate::RaySceneError> {
        if !job.query_kind().supports_surface_nearest()
            || !self
                .compute_gi
                .as_ref()
                .is_some_and(|gi| gi.supports_world_kind(kind) && !gi.world_frame_enabled())
        {
            return Err(crate::RaySceneError::Unsupported);
        }
        let producer = self.world_ray_producer(&job, kind, offset, extent)?;
        let guides = self
            .compute_gi
            .as_ref()
            .ok_or(crate::RaySceneError::Unsupported)?
            .world_ray_guides();
        let output = if matches!(kind, crate::WorldRayKind::Reflection) {
            &self._ssr_output
        } else {
            &self._gi_output
        };
        let region = compute_gi::ArchiveWorldRayRegion::new(
            &self.device,
            &self.camera,
            &guides,
            producer,
            job,
            kind,
            extent,
            &self.background,
            &self.depth,
            &self.history_depth,
            &self.depth_bounds.texture,
            output,
            self.compute_gi
                .as_ref()
                .ok_or(crate::RaySceneError::Unsupported)?
                .world_resolve_constants(),
            false,
            None,
        )?;
        self.compute_gi
            .as_mut()
            .ok_or(crate::RaySceneError::Unsupported)?
            .add_world_region(region);
        Ok(())
    }

    /// Replace screen traversal with bounded source queries covering every pixel.
    /// Producer/resolve pipelines and the exact query job are shared across tiles.
    /// Prepare transactionally; an invalid partition leaves the previous mode intact.
    pub fn set_world_ray_frame(
        &mut self,
        job: Arc<crate::ExactWorldNearest>,
        tile: [u32; 2],
    ) -> Result<(), crate::RaySceneError> {
        let gi = self
            .compute_gi
            .as_ref()
            .ok_or(crate::RaySceneError::Unsupported)?;
        if !job.query_kind().supports_surface_nearest()
            || !gi.supports_world_kind(crate::WorldRayKind::Diffuse)
            || gi.capture_enabled()
        {
            return Err(crate::RaySceneError::Unsupported);
        }
        let tiles = compute_gi::frame_tiles([self.width, self.height], tile, job.capacity())?;
        let first = tiles[0];
        let mut templates = Vec::new();
        let cache_requested =
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref() == Ok("1");
        let sorted_requested =
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_SORTED").as_deref() == Ok("1");
        let profile_requested =
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_PROFILE").as_deref() == Ok("1");
        if sorted_requested && profile_requested {
            return Err(crate::RaySceneError::Unsupported);
        }
        let profile_workspace = if sorted_requested {
            if !cache_requested || !job.history_any_hit_enabled() {
                return Err(crate::RaySceneError::Unsupported);
            }
            Some(compute_gi::consumer_sorted_workspace(
                &self.device,
                job.capacity(),
            )?)
        } else if profile_requested {
            if !cache_requested
                || !job.history_any_hit_enabled()
                || std::env::var("VOXY_STREET_WORLD_RAY_STAGE_PROFILE").as_deref() != Ok("1")
            {
                return Err(crate::RaySceneError::Unsupported);
            }
            Some(compute_gi::consumer_profile_workspace(
                &self.device,
                job.capacity(),
            )?)
        } else {
            None
        };
        let mut kinds = vec![crate::WorldRayKind::Diffuse];
        if gi.supports_world_kind(crate::WorldRayKind::Reflection) {
            kinds.push(crate::WorldRayKind::Reflection);
        }
        for kind in kinds {
            let producer =
                self.world_ray_producer(&job, kind, [first[0], first[1]], [first[2], first[3]])?;
            let guides = gi.world_ray_guides();
            let output = if matches!(kind, crate::WorldRayKind::Reflection) {
                &self._ssr_output
            } else {
                &self._gi_output
            };
            templates.push(compute_gi::ArchiveWorldRayRegion::new(
                &self.device,
                &self.camera,
                &guides,
                producer,
                job.clone(),
                kind,
                [first[2], first[3]],
                &self.background,
                &self.depth,
                &self.history_depth,
                &self.depth_bounds.texture,
                output,
                gi.world_resolve_constants(),
                cache_requested,
                profile_workspace.clone(),
            )?);
        }
        let frame = compute_gi::ArchiveWorldRayFrame::new(&self.device, templates, tiles)?;
        self.compute_gi
            .as_mut()
            .ok_or(crate::RaySceneError::Unsupported)?
            .set_world_frame(frame);
        Ok(())
    }

    /// Prepared compute traversal constant; independent of the current environment.
    #[doc(hidden)]
    pub fn paired_dda_tile_edge_cache_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|g| g.tile_edge_cache_enabled())
    }

    /// True only when the prepared frame owns the projected-span queue.
    #[doc(hidden)]
    pub fn world_ray_consumer_sorted(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|g| g.world_consumer_sorted())
    }

    /// CPU-encoded source-query frames, excluding proved-inactive clears.
    pub fn world_ray_query_encoded_frames(&self) -> u64 {
        self.compute_gi
            .as_ref()
            .map_or(0, |gi| gi.world_query_encoded_frames())
    }

    /// Optional complete last-source-frame stage snapshot, resolved after its GPU fence.
    #[doc(hidden)]
    pub fn world_ray_same_producer_comparison(
        &self,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
        match &self.compute_gi {
            Some(gi) => gi.world_same_producer_comparison(&self.queue),
            None => Ok(None),
        }
    }

    pub fn world_ray_work_stats_buffer(&self) -> Option<&wgpu::Buffer> {
        self.compute_gi.as_ref()?.world_work_stats_buffer()
    }

    pub fn world_ray_stage_profile(
        &self,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
        if let Some(gi) = &self.compute_gi {
            gi.world_profile_snapshot(&self.device, &self.queue)
        } else {
            Ok(None)
        }
    }

    pub fn world_ray_frame_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|gi| gi.world_frame_enabled())
    }

    /// One complete non-overlapping partition, shared by GI and optional SSR.
    pub fn world_ray_frame_tiles(&self) -> Option<&[[u32; 4]]> {
        self.compute_gi.as_ref()?.world_frame_tiles()
    }

    /// Qualification counters: inactive, miss, source hit, rational-tail fallback,
    /// optical fallback, unresolved fallback, stale-ID fallback, other fallback.
    /// Enable VOXY_STREET_WORLD_RAY_RESOLVE_AUDIT before preparing regions.
    #[doc(hidden)]
    pub fn world_ray_region_audit_buffer(&self, index: usize) -> Option<&wgpu::Buffer> {
        self.compute_gi.as_ref()?.world_region_audit_buffer(index)
    }

    /// Drop prepared source owners before replacing the caller's world snapshot.
    pub fn clear_world_ray_regions(&mut self) {
        if let Some(gi) = self.compute_gi.as_mut() {
            gi.clear_world_regions();
        }
    }

    /// Number of opt-in source regions currently connected to opaque lighting.
    pub fn world_ray_region_count(&self) -> usize {
        self.compute_gi
            .as_ref()
            .map_or(0, |gi| gi.world_region_count())
    }

    pub fn opaque_ray_persistent_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|g| g.ray_persistent_enabled())
    }

    pub fn opaque_ray_capture_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|g| g.capture_enabled())
    }

    #[doc(hidden)]
    pub fn save_opaque_ray_capture(
        &self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.compute_gi
            .as_ref()
            .ok_or("compute ray owner is not active")?
            .export_capture(&self.device, &self.queue, path)
    }

    #[doc(hidden)]
    pub fn opaque_ray_hierarchy_near_first_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|g| g.hierarchy_near_first_enabled())
    }

    #[doc(hidden)]
    pub fn opaque_ray_hierarchy_reciprocal_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|g| g.hierarchy_reciprocal_enabled())
    }

    #[doc(hidden)]
    pub fn opaque_ray_hierarchy_enabled(&self) -> bool {
        self.pixel_depth_bounds.is_some()
    }

    #[doc(hidden)]
    pub fn opaque_ray_hierarchy_bounds_bytes(&self) -> u64 {
        self.pixel_depth_bounds.as_ref().map_or(0, |b| {
            (0..b.texture.mip_level_count())
                .map(|m| {
                    u64::from((b.texture.width() >> m).max(1))
                        * u64::from((b.texture.height() >> m).max(1))
                        * 8
                })
                .sum()
        })
    }

    pub fn opaque_ray_queue_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|gi| gi.ray_queue_enabled())
    }

    #[doc(hidden)]
    pub fn opaque_ray_queue_encoded_frames(&self) -> u64 {
        self.ray_queue_frame_count
    }

    #[doc(hidden)]
    pub fn opaque_ray_queue_proof_buffer(&self) -> Option<&wgpu::Buffer> {
        self.compute_gi.as_ref()?.ray_queue_proof_buffer()
    }

    pub fn opaque_gi_raster_math_enabled(&self) -> bool {
        self.compute_gi
            .as_ref()
            .is_some_and(|gi| gi.raster_math_enabled())
    }

    pub(super) fn compute_gi_active_for_frame(&self) -> bool {
        self.compute_gi.is_some()
            && (!self.gi_stage_crossover
                || self
                    .refine_crossover
                    .is_some_and(|mode| mode.cache_for_frame(self.benchmark_frame)))
    }
}
