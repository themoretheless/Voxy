//! Reusable GPU-to-GPU exact original-source intersection. No CPU ray selector.
use crate::{RayScene, RaySceneError, archive_scene::ArchiveMaterialSource};
use std::sync::Arc;
mod encode;
use encode::pack_materials;
#[path = "world_geometry_bounds.rs"]
mod geometry_bounds;
use geometry_bounds::WorldGeometryBounds;
#[path = "world_exact_vertex_cache.rs"]
mod exact_vertex_cache;
use exact_vertex_cache::WorldExactVertexCache;
#[path = "world_cached_relative.rs"]
mod cached_relative;
#[path = "world_exact_fused_affine.rs"]
mod fused_affine;
#[path = "world_exact_winner_workspace.rs"]
mod winner_workspace;
#[path = "world_exact_zero_prefix.rs"]
mod zero_prefix;

/// Nearest surface queries and closed-segment definite opaque visibility share source ownership.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldRayQueryKind {
    #[default]
    Nearest,
    /// Nearest material surface: discard proved back faces of one-sided source
    /// materials. Ambiguous first events remain unresolved; unknown events proved
    /// strictly behind a known nearer hit cannot change its identity. Uses the
    /// existing exact source predicate and conservative sided certificates.
    MaterialSurface,
    OpaqueVisibility,
    /// Rendering-only nearest admission. A proved hit that the archive history
    /// consumer cannot use returns unresolved (reason 1024 side, 2048 optical,
    /// 4096 non-dyadic t, 8192 existence witness when every material forces
    /// consumer fallback). Ordinary nearest/physics outputs remain unchanged.
    HistoryAdmission,
}
impl WorldRayQueryKind {
    pub const fn supports_surface_nearest(self) -> bool {
        matches!(
            self,
            Self::Nearest | Self::MaterialSurface | Self::HistoryAdmission
        )
    }
}

#[path = "world_traversal_pair.rs"]
mod traversal_pair;
use traversal_pair::WorldTraversalPair;

/// Immutable query choice and conservative certificates. Defaults preserve nearest queries.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExactWorldOptions {
    pub query_kind: WorldRayQueryKind,
    /// Admit any proved hit only when every material forces history fallback.
    pub history_any_hit: bool,
    /// Prove visibility triangles with outward intervals before exact fallback.
    pub certify_visibility: bool,
    /// Separate certificate-only pass with a GPU queue for exact fallback.
    pub defer_visibility: bool,
    /// Prove nearest misses/unique winners; queue uncertain cases for full traversal.
    pub defer_nearest: bool,
    /// Native primitive ID hint, always source-certified and supplemented.
    pub native_seed: bool,
    /// Isolate hardware hint traversal from conservative source traversal.
    pub separate_native_seed: bool,
    /// Rank optional hardware IDs; never substitutes for original source proof.
    pub native_triangle_hint: bool,
    /// Prove render-consumer existence directly from hardware candidate IDs.
    pub native_source_witness: bool,
    /// Reuse an outward-proved reciprocal per ray in software node traversal.
    pub node_reciprocal: bool,
    /// Candidate eight-child scheduling; original bounds and exact fallback retained.
    pub wide_source_nodes: bool,
    /// Scheduling-only child ordering; no bounds or source predicates change.
    pub wide_source_near_first: bool,
    /// Diagnostic only: original then wide on one actual GPU ray batch, with a
    /// native-word comparison. Rejects previous-frame seeds and performance profiles.
    pub compare_wide_same_producer: bool,
    /// Re-certify the previous matching ray ID hit against current source words.
    pub previous_hit_seed: bool,
    /// Bounded queued exact worker pool; absent preserves 16 workers.
    pub exact_worker_limit: Option<std::num::NonZeroU32>,
    /// Queued exact workgroup width; absent preserves width 1.
    pub exact_group_size: Option<std::num::NonZeroU32>,
    /// Reference scheduling for deferred nearest queue comparisons.
    pub single_ray_queue_claim: bool,
    /// Diagnostic software certificate counts, excludes first ray per encode.
    pub traversal_work_stats: bool,
    /// Immutable original-source interval edges and normals, prepared on GPU.
    /// Only deferred ordinary/material nearest queries; exact predicates unchanged.
    pub geometry_bounds_cache: bool,
    /// Recompute/check every prepared record before making the cache ready.
    pub geometry_bounds_check: bool,
    /// GPU-prepared exact affine world coordinates; all records checked before ready.
    /// Non-admitted records use complete original source arithmetic.
    pub exact_vertex_cache: bool,
    /// Experimental exact sum of source transforms and both ray-origin parts.
    /// Default off; retains every digit and the existing triangle predicates.
    pub fused_source_affine: bool,
    /// Experimental exact multiplication: skip only zero low words, retaining
    /// all 2048 bits, original scratch words and noncanonical overflow behavior.
    pub skip_zero_prefix_multiply: bool,
    /// Experimental outward relative bounds; requires both same-owner caches.
    /// Failed whole-vertex admission retains the complete source calculation.
    pub cached_relative_interval: bool,
    /// Experimental winner-only temporary slots; 2048-bit integers stay intact.
    /// Exhausted/non-hit winners retry the unchanged complete 512-slot pass.
    pub exact_winner_arena_slots: Option<std::num::NonZeroU32>,
    /// Separate winner-only worker cap, bounded by the existing scratch allocation.
    /// Absent preserves the ordinary exact worker count.
    pub exact_winner_worker_limit: Option<std::num::NonZeroU32>,
    /// Quotient estimates accepted only after exact RN-even midpoint proof.
    /// Retains the original full division on any failed/unavailable proof.
    /// Never participates in source predicates or unrounded hit ordering.
    pub certified_ratio_seed: bool,
    pub certify_order: bool,
    /// Require the explicit GPU producer admission word at input index 3.
    pub guarded_input: bool,
    /// Append canonical UV pairs from unrounded source weights, with residual flags.
    pub hit_uv: bool,
    /// Append world-position pairs formed from unrounded exact t, with residual flags.
    pub hit_position: bool,
    /// Opaque-history consumer only. After proving the nearest material event,
    /// return unresolved reason 16384 when an exact odd-divisor test proves a
    /// non-dyadic world coordinate. The consumer must use its complete fallback;
    /// no geometric miss, rounded position, or incomplete shading is published.
    /// Requires MaterialSurface plus hit_position; ordinary queries reject it.
    /// Valid only while the history consumer rejects every nonzero coordinate
    /// residual. Requalify or disable it if that acceptance policy changes.
    pub history_position_admission: bool,
}

/// A shared immutable scene owner keeps TLAS, masks, source layout and hierarchy in
/// one revision while commands are prepared. Build the scene before querying.
///
/// Input: 20 u32 words per ray (origin hi at 0, lo at 4; segment direction hi
/// at 8, lo at 12; stable ray ID at 18/19). Parameters span the closed [0,1].
/// Output: 20 u32 words per ray, with status 0 miss/clear, 1 hit/blocker, 2 unresolved;
/// guarded inputs additionally return status 3 for inactive pixels; unsupported
/// coefficients return unresolved reason 512.
/// Nearest returns stable instance/primitive IDs at 6/7 and t/u/v hi/lo at
/// 8..13, ordered by unrounded rationals. Opaque visibility returns any definite
/// blocker with no t/u/v. Optical/unknown layers remain unresolved unless a
/// definite blocker is found elsewhere on the same closed segment.
/// Material parameters/image bindings are an immutable prepared snapshot;
/// recreate the query after they change.
/// With `hit_uv`, append words 20..23: UV x/y high/low; 24: validity mask
/// (bit 0 x, bit 1 y); 25: nonzero residual mask; 26: attribute failure reason;
/// 27: reserved. Miss/inactive/unresolved records have an empty UV extension.
/// A remaining UV residual is explicit; a pair is not the full rational value.
/// Ordinary nearest/visibility queries remain geometric and two-sided.
/// `MaterialSurface` filters exact back faces using original material sidedness;
/// coplanar/degenerate one-sided hits stay unresolved if they can precede or tie
/// the nearest proved hit. Strictly farther unknown events are irrelevant. Consumers still
/// own normal evaluation, texture-dependent alpha, layered transmission and shading.
/// This job is not a performance guarantee.
/// Opaque visibility reuses 16 exact workspaces; deferred nearest defaults to
/// 16 (4 MiB), with an explicitly bounded configurable worker pool, independent
/// of ray capacity. A short encode binds only its requested input records;
/// remaining output records are untouched. With either deferred option, encoded
/// clears and the GPU queue mutate reserved input padding words 7, 11 and 15.
/// All coefficient, admission, target and stable-ID words remain unchanged.
/// Statuses 4 (full fallback), 5 (unique winner), 6 (bounded candidate list),
/// and 7 (same-encode native source witness)
/// are internal and resolved before the consumer
/// before a consumer following `encode` reads the output.
/// Deferred nearest resolves proved misses without integer work. A strictly
/// separated certified winner runs one exact primitive and optional UV. Uncertain
/// cases use the same exact closed predicate and unrounded tie ordering on
/// the conservative candidate list. Overflow retries the complete source walk.
/// Output word18 bits11/12/13 mark certified miss / unique winner / candidate list.
/// Bit16 marks an admitted source proof from this encode's isolated hardware pass.
#[derive(Debug)]
pub struct WorldRayQueryJob {
    scene: Arc<RayScene>,
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    fast: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    traversal_pair: Option<WorldTraversalPair>,
    work_stats: Option<(
        wgpu::ComputePipeline,
        wgpu::BindGroup,
        crate::ComputeStorage,
    )>,
    geometry_bounds: Option<WorldGeometryBounds>,
    vertex_cache: Option<WorldExactVertexCache>,
    native_hint: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    history_filter: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    winner: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    candidates: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    input: Arc<crate::ComputeStorage>,
    output: crate::ComputeStorage,
    _scratch: crate::ComputeStorage,
    _counts: crate::ComputeStorage,
    _hierarchy: crate::ComputeStorage,
    capacity: u32,
    partial_groups: std::sync::Mutex<Vec<(wgpu::ComputePipeline, u32, wgpu::BindGroup)>>,
    guarded_input: bool,
    output_words: u32,
    position_offset: Option<u64>,
    query_kind: WorldRayQueryKind,
    native_seed: bool,
    history_any_hit: bool,
    native_source_witness: bool,
    native_triangle_hint: bool,
    node_reciprocal: bool,
    exact_workers: u32,
    winner_workers: u32,
    winner_slots: u32,
    exact_group_size: u32,
    skip_zero_prefix_multiply: bool,
    exact_integer_digest: [u8; 32],
    wide_source_near_first: bool,
    wide_source_base: u32,
    wide_source_digest: [u8; 32],
}
/// Existing nearest-call spelling; this is the same resource owner.
pub type ExactWorldNearest = WorldRayQueryJob;
impl WorldRayQueryJob {
    /// Prepare once; producers write rays on GPU into `input()`, consumers use
    /// `output()`. Submission, fences, readback and fallbacks belong to the caller.
    /// # Errors
    /// Rejects a foreign device, missing archive source metadata/materials and
    /// resource limits before encoding work. No source material is made opaque.
    pub fn new(
        device: &wgpu::Device,
        scene: Arc<RayScene>,
        materials: &[ArchiveMaterialSource],
        image_count: usize,
        capacity: u32,
    ) -> Result<Self, RaySceneError> {
        Self::with_options(
            device,
            scene,
            materials,
            image_count,
            capacity,
            ExactWorldOptions::default(),
        )
    }
    /// Prepare with explicit conservative order-certificate selection.
    /// The certificate changes work only; uncertain/tied cases stay exact.
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn with_options(
        device: &wgpu::Device,
        scene: Arc<RayScene>,
        materials: &[ArchiveMaterialSource],
        image_count: usize,
        capacity: u32,
        options: ExactWorldOptions,
    ) -> Result<Self, RaySceneError> {
        scene.validate_device(device)?;
        if options.wide_source_near_first && !options.wide_source_nodes {
            return Err(RaySceneError::Unsupported);
        }
        if options.compare_wide_same_producer
            && (!options.wide_source_nodes
                || !options.defer_nearest
                || options.previous_hit_seed
                || options.traversal_work_stats
                || options.history_any_hit
                || !matches!(
                    options.query_kind,
                    WorldRayQueryKind::Nearest | WorldRayQueryKind::MaterialSurface
                ))
        {
            return Err(RaySceneError::Unsupported);
        }
        let visibility = options.query_kind == WorldRayQueryKind::OpaqueVisibility;
        let history_admission = options.query_kind == WorldRayQueryKind::HistoryAdmission;
        let material_surface = options.query_kind == WorldRayQueryKind::MaterialSurface;
        if options.native_source_witness
            && (!options.separate_native_seed || !options.history_any_hit)
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.history_any_hit && !history_admission {
            return Err(RaySceneError::Unsupported);
        }
        if history_admission
            && (!options.defer_nearest
                || !options.certify_visibility
                || !options.guarded_input
                || options.hit_uv)
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.defer_visibility && (!visibility || !options.certify_visibility) {
            return Err(RaySceneError::Unsupported);
        }
        if options.defer_nearest
            && (visibility || !options.certify_visibility || options.defer_visibility)
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.native_triangle_hint
            && (!options.native_seed
                || !options.separate_native_seed
                || options.native_source_witness
                || options.history_any_hit
                || !options.query_kind.supports_surface_nearest()
                || history_admission)
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.separate_native_seed && !options.native_seed {
            return Err(RaySceneError::Unsupported);
        }
        if options.native_seed && !options.defer_nearest {
            return Err(RaySceneError::Unsupported);
        }
        if options.previous_hit_seed && (!options.defer_nearest || options.native_seed) {
            return Err(RaySceneError::Unsupported);
        }
        if !options.defer_nearest
            && (options.exact_worker_limit.is_some() || options.exact_group_size.is_some())
        {
            return Err(RaySceneError::Unsupported);
        }
        let exact_group_size = options
            .exact_group_size
            .map_or(1, std::num::NonZeroU32::get);
        let worker_limit = options
            .exact_worker_limit
            .map_or(16, std::num::NonZeroU32::get);
        let deferred = options.defer_visibility || options.defer_nearest;
        if options.cached_relative_interval
            && (!options.exact_vertex_cache || !options.geometry_bounds_cache)
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.history_position_admission
            && (!options.hit_position || options.query_kind != WorldRayQueryKind::MaterialSurface)
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.exact_vertex_cache
            && (!options.defer_nearest
                || !options.certify_visibility
                || options.native_source_witness
                || options.history_any_hit
                || !matches!(
                    options.query_kind,
                    WorldRayQueryKind::Nearest | WorldRayQueryKind::MaterialSurface
                ))
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.fused_source_affine
            && (!options.defer_nearest
                || !options.certify_visibility
                || options.native_source_witness
                || options.history_any_hit
                || !matches!(
                    options.query_kind,
                    WorldRayQueryKind::Nearest | WorldRayQueryKind::MaterialSurface
                ))
        {
            return Err(RaySceneError::Unsupported);
        }
        let compact_winner = options.exact_winner_arena_slots.is_some()
            || options.exact_winner_worker_limit.is_some();
        if compact_winner
            && (!options.defer_nearest
                || !options.certify_visibility
                || options.native_source_witness
                || options.history_any_hit
                || !matches!(
                    options.query_kind,
                    WorldRayQueryKind::Nearest | WorldRayQueryKind::MaterialSurface
                ))
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.geometry_bounds_check && !options.geometry_bounds_cache {
            return Err(RaySceneError::Unsupported);
        }
        if options.geometry_bounds_cache
            && (!options.defer_nearest
                || !options.certify_visibility
                || options.native_source_witness
                || options.history_any_hit
                || !matches!(
                    options.query_kind,
                    WorldRayQueryKind::Nearest | WorldRayQueryKind::MaterialSurface
                ))
        {
            return Err(RaySceneError::Unsupported);
        }
        if options.traversal_work_stats
            && (!options.defer_nearest || !options.guarded_input || options.hit_uv)
        {
            return Err(RaySceneError::Unsupported);
        }
        if visibility && options.hit_uv {
            return Err(RaySceneError::Unsupported);
        }
        if options.hit_position && (visibility || history_admission) {
            return Err(RaySceneError::Unsupported);
        }
        let limits = device.limits();
        let workers = if visibility || deferred {
            capacity.min(if options.defer_nearest {
                worker_limit
            } else {
                16
            })
        } else {
            capacity
        };
        let scratch_bytes = u64::from(workers) * 512 * 128 * 4;
        let winner_plan = if compact_winner {
            Some(
                winner_workspace::plan(
                    options
                        .exact_winner_arena_slots
                        .map(std::num::NonZeroU32::get),
                    options
                        .exact_winner_worker_limit
                        .map(std::num::NonZeroU32::get),
                    workers,
                    capacity,
                )
                .ok_or(RaySceneError::Capacity)?,
            )
        } else {
            None
        };
        let winner_workers = winner_plan.map_or(workers, |p| p.workers);
        let winner_slots = winner_plan.map_or(512, |p| p.slots);
        let input_bytes = u64::from(capacity) * 80;
        let output_words = if options.hit_uv { 28 } else { 20 };
        let primary_bytes = u64::from(capacity) * u64::from(output_words) * 4;
        let alignment = u64::from(limits.min_storage_buffer_offset_alignment).max(16);
        let position_offset = options
            .hit_position
            .then(|| primary_bytes.div_ceil(alignment) * alignment);
        let output_bytes =
            position_offset.map_or(primary_bytes, |offset| offset + u64::from(capacity) * 32);
        let position_base = position_offset.map_or(0, |offset| offset / 16);
        if exact_group_size > limits.max_compute_workgroup_size_x
            || exact_group_size > limits.max_compute_invocations_per_workgroup
        {
            return Err(RaySceneError::Capacity);
        }
        if winner_workers.div_ceil(exact_group_size) > limits.max_compute_workgroups_per_dimension {
            return Err(RaySceneError::Capacity);
        }
        if position_base > u64::from(u32::MAX)
            || capacity == 0
            || workers > limits.max_compute_workgroups_per_dimension
            || (deferred && capacity.div_ceil(64) > limits.max_compute_workgroups_per_dimension)
            || input_bytes > limits.max_storage_buffer_binding_size
            || output_bytes > limits.max_storage_buffer_binding_size
            || input_bytes > limits.max_buffer_size
            || output_bytes > limits.max_buffer_size
            || scratch_bytes > limits.max_storage_buffer_binding_size
            || scratch_bytes > limits.max_buffer_size
            || limits.max_storage_buffers_per_shader_stage < 8
            || limits.max_uniform_buffer_binding_size < 65536
        {
            return Err(RaySceneError::Capacity);
        }
        let (mut counts, mut hierarchy) = scene
            .archive_exact_source_metadata()
            .ok_or(RaySceneError::InvalidGeometry)?;
        counts[4095][3] = 3
            | (u32::from(options.certify_order) << 2)
            | (u32::from(options.guarded_input) << 3)
            | (u32::from(options.certify_visibility) << 4); // Outward interval certificates; exact fallback.
        let layout = scene
            .archive_candidate_layout()
            .ok_or(RaySceneError::InvalidGeometry)?;
        if layout
            .iter()
            .any(|entry| entry[4] as usize >= materials.len())
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let material_words = pack_materials(materials, image_count)?;
        // MaterialSurface asks for the first material event. A proved admitted
        // hit bounds later events, including unknown layers. Ordinary nearest
        // preserves its global unknown-material contract.
        let can_bound = material_surface || Self::packed_material_table_known(&material_words);
        let history_any_hit = options.history_any_hit
            && Self::packed_materials_force_history_fallback(&material_words);
        let native_source_witness = options.native_source_witness && history_any_hit;
        hierarchy.extend(material_words);
        // Append a derived scheduling view AFTER the unchanged binary hierarchy
        // and material table. Only the certificate pipeline receives its base.
        let (wide_base, wide_source_digest) = if options.wide_source_nodes && deferred {
            let offset = hierarchy[2] as usize;
            let words = (hierarchy[1] as usize)
                .checked_mul(8)
                .ok_or(RaySceneError::Capacity)?;
            let end = offset.checked_add(words).ok_or(RaySceneError::Capacity)?;
            let source = hierarchy
                .get(offset..end)
                .ok_or(RaySceneError::InvalidGeometry)?;
            let wide =
                crate::world_nearest_bvh::wide::convert(bytemuck::cast_slice(source), hierarchy[3])
                    .map_err(|_| RaySceneError::InvalidGeometry)?;
            // Optional scheduling must not turn a supported original scene
            // into a capacity error. Preflight before allocating/uploading tail.
            let final_bytes = wide
                .packed_word_len()
                .ok()
                .and_then(|n| hierarchy.len().checked_add(n))
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| u64::try_from(n).ok());
            let fits = final_bytes.is_some_and(|bytes| {
                bytes <= limits.max_storage_buffer_binding_size && bytes <= limits.max_buffer_size
            });
            if fits && wide.pending_capacity <= 64 && wide.root.is_some() {
                let base = u32::try_from(hierarchy.len()).map_err(|_| RaySceneError::Capacity)?;
                let tail = wide.to_words().map_err(|_| RaySceneError::Capacity)?;
                let digest = *blake3::hash(bytemuck::cast_slice(&tail)).as_bytes();
                hierarchy.extend(tail);
                (base, digest)
            } else {
                (0, [0; 32])
            }
        } else {
            (0, [0; 32])
        };

        if hierarchy.len() as u64 * 4 > limits.max_storage_buffer_binding_size {
            return Err(RaySceneError::Capacity);
        }
        let count_bytes = bytemuck::cast_slice(&counts);
        let hierarchy_bytes = bytemuck::cast_slice(&hierarchy);
        let [input, output, scratch, counts, hierarchy] =
            crate::ComputeMemoryBudget::for_device(device)
                .allocate_buffers([
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "exact world GPU ray input",
                        size: u64::from(capacity) * 80,
                        contents: None,
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::COPY_DST
                            | wgpu::BufferUsages::COPY_SRC,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "exact world GPU intersections",
                        size: output_bytes,
                        contents: None,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "exact world 2048-bit workspace",
                        size: scratch_bytes,
                        contents: None,
                        usage: wgpu::BufferUsages::STORAGE,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "same-owner exact source bounds",
                        size: count_bytes.len() as u64,
                        contents: Some(count_bytes),
                        usage: wgpu::BufferUsages::UNIFORM,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "same-owner exact hierarchy and material words",
                        size: hierarchy_bytes.len() as u64,
                        contents: Some(hierarchy_bytes),
                        usage: wgpu::BufferUsages::STORAGE,
                    },
                ])
                .map_err(|_| RaySceneError::Capacity)?;
        let input = Arc::new(input);
        let geometry_bounds = if options.geometry_bounds_cache {
            Some(WorldGeometryBounds::new(
                device,
                &scene,
                options.geometry_bounds_check,
            )?)
        } else {
            None
        };
        let vertex_cache = if options.exact_vertex_cache {
            Some(WorldExactVertexCache::new(
                device, &scene, &scratch, workers,
            )?)
        } else {
            None
        };
        let exact_source = vertex_cache.as_ref().map_or_else(
            || include_str!("world_exact.wgsl").to_owned(),
            WorldExactVertexCache::source,
        );
        let exact_source = if options.fused_source_affine {
            fused_affine::source(exact_source, vertex_cache.is_some())
        } else {
            exact_source
        };
        let certificate_source = geometry_bounds.as_ref().map_or_else(
            || include_str!("world_source_certificate.wgsl").to_owned(),
            WorldGeometryBounds::source,
        );
        let certificate_source = if options.cached_relative_interval {
            cached_relative::source(
                &certificate_source,
                vertex_cache
                    .as_ref()
                    .ok_or(RaySceneError::Unsupported)?
                    .relative_layout,
            )
        } else {
            certificate_source
        };
        let original_ratio = include_str!("world_exact_ratio.wgsl");
        let ratio_source = if options.certified_ratio_seed {
            let (support, pair) = original_ratio
                .split_once("fn ex_ratio_pair(")
                .expect("canonical ratio pair");
            assert_eq!(pair.matches("ex_round_ratio(").count(), 2);
            format!(
                "{}\n{}\nfn ex_ratio_pair({}",
                support,
                include_str!("world_exact_ratio_seed.wgsl"),
                pair.replace("ex_round_ratio(", "ex_round_ratio_seed(")
            )
        } else {
            original_ratio.to_owned()
        };
        let integer_source = if options.skip_zero_prefix_multiply {
            zero_prefix::source(include_str!("world_exact_integer.wgsl"))
        } else {
            include_str!("world_exact_integer.wgsl").to_owned()
        };
        let nearest = include_str!("world_nearest.wgsl");
        let nearest = if options.node_reciprocal {
            let init = "fn nearest_make_box(ray:u32,d:ExVec)->bool {";
            let miss = "nearest_node_miss(base,ray)";
            assert_eq!(nearest.matches(init).count(), 1);
            assert_eq!(nearest.matches(miss).count(), 1);
            let (before, rest) = nearest.split_once(init).unwrap();
            let (body, after) = rest.split_once("\n return true;\n}").unwrap();
            std::borrow::Cow::Owned(format!(
                "{}\n{before}{init}{body}\n if (nearest_counts.values[4095u].w&2u)!=0u {{nearest_prepare_reciprocal();}}\n return true;\n}}{}",
                include_str!("world_node_reciprocal.wgsl"),
                after.replace(miss, "nearest_node_miss_cached(base,ray,1.)"),
            ))
        } else {
            std::borrow::Cow::Borrowed(nearest)
        };
        let source = [
            "enable wgpu_ray_query;",
            if deferred {
                include_str!("world_ray_deferred_input.wgsl")
            } else {
                include_str!("world_ray_input.wgsl")
            },
            include_str!("world_ray_source_bindings.wgsl"),
            integer_source.as_str(),
            include_str!("world_exact_history_position.wgsl"),
            ratio_source.as_str(),
            exact_source.as_str(),
            include_str!("archive_projection_pair.wgsl"),
            include_str!("world_triangle_pair.wgsl"),
            include_str!("world_ray_bvh_interval.wgsl"),
            include_str!("world_source_certificate.wgsl"),
            include_str!("world_visibility_common.wgsl"),
            nearest.as_ref(),
            include_str!("world_visibility.wgsl"),
            if options.defer_nearest {
                include_str!("world_nearest_deferred_exact.wgsl")
            } else if options.defer_visibility {
                include_str!("world_visibility_deferred_exact.wgsl")
            } else {
                ""
            },
        ]
        .join("\n");
        // The diagnostic fingerprint describes the integer module actually
        // included in the pipeline source, not merely the requested flag.
        assert!(source.contains(integer_source.as_str()));
        assert_eq!(source.matches("fn ex_mul(").count(), 1);
        assert_eq!(
            source.contains("let first_right=ex_low_word(b)"),
            options.skip_zero_prefix_multiply
        );
        let exact_integer_digest = *blake3::hash(integer_source.as_bytes()).as_bytes();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("original-source world ray query"),
            source: wgpu::ShaderSource::Wgsl(source.as_str().into()),
        });
        let mut exact_constants = Vec::new();
        if options.history_position_admission {
            exact_constants.push(("WORLD_EXACT_HISTORY_POSITION_ADMISSION", 1.));
        }
        if options.hit_position {
            exact_constants.push(("WORLD_HIT_POSITION_BASE", position_base as f64));
        }
        if options.defer_nearest {
            exact_constants.push(("WORLD_EXACT_GROUP", f64::from(exact_group_size)));
            if options.single_ray_queue_claim {
                exact_constants.push(("NEAREST_QUEUE_BATCH", 1.));
            }
        }
        if material_surface {
            exact_constants.push(("WORLD_EXACT_MATERIAL_SIDES", 1.));
            exact_constants.push(("WORLD_SOURCE_MATERIAL_SIDES", 1.));
        }
        let trace_pipelines = std::env::var_os("VOXY_WORLD_RAY_PIPELINE_TIMING").is_some();
        let build_pipeline = |descriptor: &wgpu::ComputePipelineDescriptor<'_>| {
            if !trace_pipelines {
                return device.create_compute_pipeline(descriptor);
            }
            let entry = descriptor.entry_point.unwrap_or("<inferred>");
            let started = std::time::Instant::now();
            eprintln!("WORLD RAY PIPELINE begin entry={entry} exact_group={exact_group_size}");
            let pipeline = device.create_compute_pipeline(descriptor);
            eprintln!(
                "WORLD RAY PIPELINE end entry={entry} elapsed_ms={:.3}",
                started.elapsed().as_secs_f64() * 1000.
            );
            pipeline
        };
        let pipeline = build_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("world ray query GPU job"),
            layout: None,
            module: &shader,
            entry_point: Some(if options.defer_nearest {
                if options.hit_uv {
                    "main_nearest_uv_deferred"
                } else {
                    "main_nearest_deferred"
                }
            } else if options.defer_visibility {
                "main_visibility_deferred_exact"
            } else if visibility {
                "main_visibility_bounded"
            } else if options.hit_uv {
                "main_nearest_uv"
            } else {
                "main_nearest"
            }),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &exact_constants,
                ..Default::default()
            },
            cache: None,
        });
        let group = Self::make_group(
            device,
            &scene,
            &pipeline,
            &input,
            &output,
            &scratch,
            &counts,
            &hierarchy,
            visibility,
            capacity,
            false,
            None,
            vertex_cache.as_ref(),
        )?;
        let winner_shader = winner_plan.map(|plan| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("bounded winner workspace with complete source retry"),
                source: wgpu::ShaderSource::Wgsl(winner_workspace::source(&source, plan).into()),
            })
        });
        let winner = if options.defer_nearest {
            let pipeline = build_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("one certified primitive exact query"),
                layout: None,
                module: winner_shader.as_ref().unwrap_or(&shader),
                entry_point: Some(if options.hit_uv {
                    "main_nearest_uv_winner"
                } else {
                    "main_nearest_winner"
                }),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &exact_constants,
                    ..Default::default()
                },
                cache: None,
            });
            let group = Self::make_group(
                device,
                &scene,
                &pipeline,
                &input,
                &output,
                &scratch,
                &counts,
                &hierarchy,
                true,
                capacity,
                false,
                None,
                vertex_cache.as_ref(),
            )?;
            Some((pipeline, group))
        } else {
            None
        };
        let candidates = if options.defer_nearest {
            let pipeline = build_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("bounded exact nearest candidates"),
                layout: None,
                module: &shader,
                entry_point: Some(if options.hit_uv {
                    "main_nearest_uv_candidates"
                } else {
                    "main_nearest_candidates"
                }),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &exact_constants,
                    ..Default::default()
                },
                cache: None,
            });
            let group = Self::make_group(
                device,
                &scene,
                &pipeline,
                &input,
                &output,
                &scratch,
                &counts,
                &hierarchy,
                true,
                capacity,
                false,
                None,
                vertex_cache.as_ref(),
            )?;
            Some((pipeline, group))
        } else {
            None
        };
        if options.compare_wide_same_producer && wide_base == 0 {
            return Err(RaySceneError::Unsupported);
        }
        let mut original_fast = None;
        let fast = if deferred {
            let source = [
                if options.native_seed && !options.separate_native_seed {
                    "enable wgpu_ray_query;"
                } else {
                    ""
                },
                include_str!("world_ray_deferred_input.wgsl"),
                include_str!("world_ray_source_bindings.wgsl"),
                include_str!("archive_projection_pair.wgsl"),
                include_str!("world_triangle_pair.wgsl"),
                include_str!("world_ray_bvh_interval.wgsl"),
                if options.node_reciprocal {include_str!("world_node_reciprocal.wgsl")}else{"fn nearest_prepare_reciprocal() {} fn nearest_node_miss_cached(base:u32,ray:u32,far:f32)->bool {return nearest_node_miss_before(base,ray,far);}"},
                certificate_source.as_str(),include_str!("world_visibility_common.wgsl"),
                include_str!("world_visibility_fast.wgsl"),
                include_str!("world_visibility_wide.wgsl"),
                if options.native_seed && !options.separate_native_seed {
                    include_str!("world_visibility_native_seed.wgsl")
                } else {
                    "fn visibility_seed(ray:u32) {}"
                },
            ]
            .join("\n");
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("certificate-only opaque visibility"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let build = |wide_override: u32| -> Result<_, RaySceneError> {
                let pipeline = build_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("certificate-only opaque visibility"),
                    layout: None,
                    module: &shader,
                    entry_point: Some("main_visibility_fast"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[
                            (
                                "WORLD_FAST_DEFER_NONCLEAR",
                                if options.defer_nearest { 1. } else { 0. },
                            ),
                            ("WORLD_HIT_POSITION_BASE", position_base as f64),
                            ("WORLD_FAST_STRIDE", if options.hit_uv { 7. } else { 5. }),
                            (
                                "WORLD_FAST_WORK_STATS",
                                if options.traversal_work_stats { 1. } else { 0. },
                            ),
                            ("WORLD_FAST_CAN_BOUND", if can_bound { 1. } else { 0. }),
                            ("WORLD_WIDE_BASE", wide_override as f64),
                            (
                                "WORLD_WIDE_NEAR_FIRST",
                                if options.wide_source_near_first && wide_override != 0 {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "WORLD_SOURCE_MATERIAL_SIDES",
                                if material_surface { 1. } else { 0. },
                            ),
                            (
                                "WORLD_FAST_HISTORY_ANY_HIT",
                                if history_any_hit { 1. } else { 0. },
                            ),
                            (
                                "WORLD_FAST_NATIVE_WITNESS",
                                if native_source_witness { 1. } else { 0. },
                            ),
                            (
                                "WORLD_FAST_NODE_RECIPROCAL",
                                if options.node_reciprocal { 1. } else { 0. },
                            ),
                            (
                                "WORLD_FAST_SEPARATE_SEED",
                                if options.separate_native_seed { 1. } else { 0. },
                            ),
                            (
                                "WORLD_FAST_PREVIOUS_SEED",
                                if options.previous_hit_seed || options.separate_native_seed {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                        ],
                        ..Default::default()
                    },
                    cache: None,
                });
                let group = Self::make_group(
                    device,
                    &scene,
                    &pipeline,
                    &input,
                    &output,
                    &scratch,
                    &counts,
                    &hierarchy,
                    !options.native_seed || options.separate_native_seed,
                    capacity,
                    true,
                    geometry_bounds.as_ref(),
                    vertex_cache.as_ref(),
                )?;
                Ok((pipeline, group))
            };
            if options.compare_wide_same_producer {
                original_fast = Some(build(0)?);
            }
            Some(build(wide_base)?)
        } else {
            None
        };
        let history_filter = if history_admission {
            let source = [
                include_str!("world_ray_deferred_input.wgsl"),
                include_str!("world_ray_source_bindings.wgsl"),
                include_str!("archive_projection_pair.wgsl"),
                include_str!("world_triangle_pair.wgsl"),
                include_str!("world_ray_bvh_interval.wgsl"),
                include_str!("world_modular_words.wgsl"),
                include_str!("world_history_admission.wgsl"),
            ]
            .join("\n");
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("source-history admission and queue compaction"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let pipeline = build_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("render-only source admission"),
                layout: None,
                module: &shader,
                entry_point: Some("main_history_admission"),
                compilation_options: Default::default(),
                cache: None,
            });
            let group = Self::make_group(
                device,
                &scene,
                &pipeline,
                &input,
                &output,
                &scratch,
                &counts,
                &hierarchy,
                true,
                capacity,
                true,
                None,
                vertex_cache.as_ref(),
            )?;
            Some((pipeline, group))
        } else {
            None
        };
        let native_hint = if options.separate_native_seed {
            let source = if native_source_witness {
                [
                    "enable wgpu_ray_query;",
                    include_str!("world_ray_deferred_input.wgsl"),
                    include_str!("world_ray_source_bindings.wgsl"),
                    include_str!("archive_projection_pair.wgsl"),
                    include_str!("world_triangle_pair.wgsl"),
                    include_str!("world_ray_bvh_interval.wgsl"),
                    include_str!("world_source_certificate.wgsl"),
                    include_str!("world_native_source_witness.wgsl"),
                ]
                .join("\n")
            } else if options.native_triangle_hint {
                include_str!("world_native_triangle_hint.wgsl").to_owned()
            } else {
                include_str!("world_native_hint.wgsl").to_owned()
            };
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("isolated hardware primitive hint"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let pipeline = build_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("isolated hardware primitive hint"),
                layout: None,
                module: &shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[("HINT_STRIDE", f64::from(output_words / 4))],
                    ..Default::default()
                },
                cache: None,
            });
            let group = if native_source_witness {
                Self::make_group(
                    device,
                    &scene,
                    &pipeline,
                    &input,
                    &output,
                    &scratch,
                    &counts,
                    &hierarchy,
                    false,
                    capacity,
                    true,
                    None,
                    vertex_cache.as_ref(),
                )?
            } else {
                Self::make_hint_group(
                    device,
                    &scene,
                    &pipeline,
                    &input,
                    &output,
                    &counts,
                    capacity,
                    options.native_triangle_hint,
                )?
            };
            Some((pipeline, group))
        } else {
            None
        };
        let work_stats = if options.traversal_work_stats {
            let [stats] = crate::ComputeMemoryBudget::for_device(device)
                .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                    label: "source ray work counts",
                    size: 56,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                }])
                .map_err(|_| RaySceneError::Capacity)?;
            let source = [
                include_str!("world_ray_deferred_input.wgsl"),
                include_str!("world_traversal_work_stats.wgsl"),
            ]
            .join("\n");
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("source work count reduction"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let pipeline = build_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("diagnostic source work counts"),
                layout: None,
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
            let group =
                Self::make_work_stats_group(device, &pipeline, &input, &output, &stats, capacity);
            Some((pipeline, group, stats))
        } else {
            None
        };
        let traversal_pair = original_fast
            .map(|original| {
                WorldTraversalPair::new(
                    device,
                    &input,
                    &output,
                    output_words,
                    position_offset.map_or(0, |v| v / 4),
                    original,
                )
            })
            .transpose()?;
        Ok(Self {
            traversal_pair,
            geometry_bounds,
            vertex_cache,
            work_stats,
            scene,
            device: device.clone(),
            pipeline,
            group,
            fast,
            history_filter,
            native_hint,
            winner,
            candidates,
            input,
            output,
            _scratch: scratch,
            _counts: counts,
            _hierarchy: hierarchy,
            capacity,
            partial_groups: Default::default(),
            guarded_input: options.guarded_input,
            output_words,
            position_offset,
            query_kind: options.query_kind,
            native_seed: options.native_seed && !options.separate_native_seed,
            history_any_hit,
            native_source_witness,
            native_triangle_hint: options.native_triangle_hint,
            node_reciprocal: options.node_reciprocal && deferred,
            exact_workers: workers,
            winner_workers,
            winner_slots,
            exact_group_size,
            skip_zero_prefix_multiply: options.skip_zero_prefix_multiply,
            exact_integer_digest,
            wide_source_base: wide_base,
            wide_source_near_first: options.wide_source_near_first && wide_base != 0,
            wide_source_digest,
        })
    }
    fn make_work_stats_group(
        device: &wgpu::Device,
        pipeline: &wgpu::ComputePipeline,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
        stats: &crate::ComputeStorage,
        count: u32,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("source work count batch"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: input,
                        offset: 0,
                        size: std::num::NonZeroU64::new(u64::from(count) * 80),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: stats.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output.as_entire_binding(),
                },
            ],
        })
    }
    #[doc(hidden)]
    pub fn traversal_work_stats_buffer(&self) -> Option<&wgpu::Buffer> {
        let (_, _, stats) = self.work_stats.as_ref()?;
        Some(stats)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn make_hint_group(
        device: &wgpu::Device,
        scene: &RayScene,
        pipeline: &wgpu::ComputePipeline,
        input: &wgpu::Buffer,
        output: &crate::ComputeStorage,
        counts: &crate::ComputeStorage,
        count: u32,
        triangle_hint: bool,
    ) -> Result<wgpu::BindGroup, RaySceneError> {
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: input,
                    offset: 0,
                    size: std::num::NonZeroU64::new(u64::from(count) * 80),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: scene.binding(),
            },
        ];
        if triangle_hint {
            let [vertices, indices, instances] = scene
                .archive_candidate_source_bindings()
                .ok_or(RaySceneError::InvalidGeometry)?;
            entries.extend([
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertices,
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indices,
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: instances,
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: scene
                        .archive_candidate_instance_layout()
                        .ok_or(RaySceneError::InvalidGeometry)?,
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: counts.as_entire_binding(),
                },
            ]);
        }
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("isolated hint source owner"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        }))
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn make_group(
        device: &wgpu::Device,
        scene: &RayScene,
        pipeline: &wgpu::ComputePipeline,
        input: &wgpu::Buffer,
        output: &crate::ComputeStorage,
        scratch: &crate::ComputeStorage,
        counts: &crate::ComputeStorage,
        hierarchy: &crate::ComputeStorage,
        visibility: bool,
        count: u32,
        fast: bool,
        geometry_bounds: Option<&WorldGeometryBounds>,
        vertex_cache: Option<&WorldExactVertexCache>,
    ) -> Result<wgpu::BindGroup, RaySceneError> {
        if vertex_cache.is_some_and(|cache| {
            !std::ptr::eq(cache.owner.as_ref(), scene)
                || cache.owner.geometry_revision() != cache.relative_layout.revision
        }) {
            return Err(RaySceneError::InvalidGeometry);
        }
        let [vertices, indices, instances] = scene
            .archive_candidate_source_bindings()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: input,
                    offset: 0,
                    size: std::num::NonZeroU64::new(u64::from(count) * 80),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: vertex_cache.map_or(vertices, |cache| cache.buffer.as_entire_binding()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: indices,
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: instances,
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: scene
                    .archive_candidate_instance_layout()
                    .ok_or(RaySceneError::InvalidGeometry)?,
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: scratch.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: counts.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: hierarchy.as_entire_binding(),
            },
        ];
        if let Some(cache) = geometry_bounds {
            entries.push(wgpu::BindGroupEntry {
                binding: 11,
                resource: cache.buffer.as_entire_binding(),
            });
        }
        if fast {
            entries.retain(|entry| entry.binding != 6);
        }
        if !visibility {
            entries.push(wgpu::BindGroupEntry {
                binding: 8,
                resource: scene.binding(),
            });
        }
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("same-owner world ray bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        }))
    }
}
