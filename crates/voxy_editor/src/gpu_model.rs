//! One GPU publication per immutable model revision, shared across its scene owners.
use crate::{ModelGraphics, PartGraphics, import::EditorAsset, selection_outline};
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};
use voxy_render::{
    SceneError, SceneGeometry, SceneRenderer, SceneSurface, SceneTexture, TextureSampling,
};
/// Weak references do not retain images after their last model revision is gone.
#[derive(Debug)]
pub(super) struct ResidencyCache {
    images: BTreeMap<[u8; 32], (Weak<SceneTexture>, u64)>,
    pub(super) budget: u64,
    epoch: u64,
    pub(super) geometry_live: u64,
    pub(super) geometry_budget: u64,
}
impl Default for ResidencyCache {
    fn default() -> Self {
        Self::with_budget(256 * 1024 * 1024)
    }
}
impl ResidencyCache {
    pub(super) fn with_budget(budget: u64) -> Self {
        Self {
            images: BTreeMap::new(),
            budget,
            epoch: 0,
            geometry_live: 0,
            geometry_budget: 256 * 1024 * 1024,
        }
    }
    pub(super) fn state(&self) -> (u64, u64, u64, u64) {
        (
            self.epoch,
            self.budget,
            self.geometry_live,
            self.geometry_budget,
        )
    }
    pub(super) fn live_bytes(&self) -> u64 {
        self.images
            .values()
            .filter(|(image, _)| image.strong_count() > 0)
            .map(|(_, bytes)| bytes)
            .sum()
    }
    fn admit(&self, additional: u64) -> Result<(), Box<dyn std::error::Error>> {
        let live = self.live_bytes();
        if additional > self.budget.saturating_sub(live) {
            return Err(format!(
                "GPU image budget exceeded: live={live} additional={additional} limit={}",
                self.budget
            )
            .into());
        }
        Ok(())
    }
    fn preflight(&mut self, asset: &EditorAsset) -> Result<(), Box<dyn std::error::Error>> {
        self.prune();
        let mut incoming = BTreeMap::new();
        let indices: std::collections::BTreeSet<_> =
            asset.nodes.iter().filter_map(|node| node.image).collect();
        for index in indices {
            let image = asset.images.get(index).ok_or(SceneError::InvalidTexture)?;
            let key = image_key(image);
            if !self.images.contains_key(&key) {
                incoming.insert(key, mip_bytes(image.width(), image.height()));
            }
        }
        self.admit(incoming.values().sum())
    }

    pub(super) fn preflight_images(
        &mut self,
        images: &[voxy_render::ImageAsset],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.prune();
        let incoming: BTreeMap<_, _> = images
            .iter()
            .filter_map(|image| {
                let key = image_key(image);
                (!self.images.contains_key(&key))
                    .then_some((key, mip_bytes(image.width(), image.height())))
            })
            .collect();
        self.admit(incoming.values().sum())
    }
    pub(super) fn prune(&mut self) {
        let before = self.images.len();
        self.images.retain(|_, (image, _)| image.strong_count() > 0);
        if before != self.images.len() {
            self.epoch = self.epoch.wrapping_add(1);
        }
    }
    pub(super) fn get_or_upload(
        &mut self,
        renderer: &SceneRenderer,
        host: &SceneSurface,
        image: &voxy_render::ImageAsset,
    ) -> Result<Arc<SceneTexture>, SceneError> {
        self.get_or_upload_on(renderer, host.device(), host.queue(), image)
    }
    pub(super) fn get_or_upload_on(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &voxy_render::ImageAsset,
    ) -> Result<Arc<SceneTexture>, SceneError> {
        self.prune();
        let key = image_key(image);
        if let Some(image) = self
            .images
            .get(&key)
            .and_then(|(image, _)| Weak::upgrade(image))
        {
            return Ok(image);
        }
        // Full storage lets later base-only and mip-filtered resources share it.
        let texture = Arc::new(renderer.upload_image_mips(
            device,
            queue,
            &image.mip_chain(),
            TextureSampling::default(),
        )?);
        self.images.insert(
            key,
            (
                Arc::downgrade(&texture),
                mip_bytes(image.width(), image.height()),
            ),
        );
        self.epoch = self.epoch.wrapping_add(1);
        Ok(texture)
    }
}
#[derive(Debug)]
pub(super) struct DeferredUpload {
    pub(super) message: String,
    source: Weak<voxy_assets::ImportedAsset<EditorAsset>>,
    state: (u64, u64, u64, u64),
}
impl DeferredUpload {
    pub(super) fn new(
        source: &Arc<voxy_assets::ImportedAsset<EditorAsset>>,
        cache: &ResidencyCache,
        message: String,
    ) -> Self {
        Self {
            message,
            source: Arc::downgrade(source),
            state: cache.state(),
        }
    }
    pub(super) fn blocks(
        &self,
        source: &Arc<voxy_assets::ImportedAsset<EditorAsset>>,
        cache: &ResidencyCache,
    ) -> bool {
        self.state == cache.state() && Weak::ptr_eq(&self.source, &Arc::downgrade(source))
    }
}
fn mip_bytes(mut width: u32, mut height: u32) -> u64 {
    let mut bytes = 0;
    loop {
        bytes += u64::from(width) * u64::from(height) * 4;
        if width == 1 && height == 1 {
            return bytes;
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
}
fn image_key(image: &voxy_render::ImageAsset) -> [u8; 32] {
    let mut hash = blake3::Hasher::new_derive_key("voxy.editor.gpu.rgba8-srgb-mips.v1");
    hash.update(&image.width().to_le_bytes());
    hash.update(&image.height().to_le_bytes());
    hash.update(image.rgba());
    *hash.finalize().as_bytes()
}
/// A model owns either a single geometry or one immutable shared-stream LOD bundle.
#[derive(Debug)]
pub(super) enum ModelGeometry {
    Single(SceneGeometry),
    Lod {
        bundle: voxy_render::SceneLodGeometry,
        min: glam::Vec3,
        max: glam::Vec3,
    },
}
impl ModelGeometry {
    fn upload(
        renderer: &SceneRenderer,
        host: &SceneSurface,
        asset: &EditorAsset,
    ) -> Result<Self, SceneError> {
        Ok(if let Some(certificate) = &asset.lod {
            let bundle = renderer.upload_streaming_certified_lod_mesh(
                host.device(),
                &asset.mesh,
                Arc::clone(certificate),
            )?;
            let mut min = glam::Vec3::splat(f32::INFINITY);
            let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
            for vertex in asset.mesh.vertices() {
                let position = glam::Vec3::from_array(vertex.position);
                min = min.min(position);
                max = max.max(position);
            }
            ModelGeometry::Lod { bundle, min, max }
        } else {
            ModelGeometry::Single(renderer.upload_mesh(host.device(), &asset.mesh)?)
        })
    }
    pub(super) fn base(&self) -> &SceneGeometry {
        match self {
            Self::Single(geometry) => geometry,
            Self::Lod { bundle, .. } => bundle.level(0).expect("validated LOD base"),
        }
    }
    pub(super) fn desired_for_view(
        &self,
        camera: Option<voxy_render::SceneCamera>,
        world: glam::Mat4,
        viewport: [u32; 2],
        previous: Option<usize>,
    ) -> Option<usize> {
        let Self::Lod { bundle, min, max } = self else {
            return None;
        };
        Some(
            camera
                .and_then(|camera| {
                    camera
                        .lod_bounds(world, *min, *max)
                        .ok()
                        .and_then(|bounds| {
                            bundle
                                .desired_level_for_camera(
                                    camera,
                                    bounds,
                                    viewport,
                                    voxy_render::LodPolicy {
                                        target_pixels: 1.0,
                                        hysteresis: 0.15,
                                    },
                                    previous,
                                )
                                .ok()
                        })
                })
                .unwrap_or(0),
        )
    }
    pub(super) fn evict_unrequested(&mut self, requested: &std::collections::BTreeSet<usize>) {
        if let Self::Lod { bundle, .. } = self {
            // Editor imports currently publish at most 16 certified levels.
            for index in 1..bundle.level_count() {
                if !requested.contains(&index) {
                    let _ = bundle.evict_level(index);
                }
            }
        }
    }
    pub(super) fn ensure_level(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        index: usize,
        budget: u64,
    ) -> Result<bool, SceneError> {
        match self {
            Self::Lod { bundle, .. } => renderer.ensure_lod_level(device, bundle, index, budget),
            Self::Single(_) => Ok(false),
        }
    }
    pub(super) fn allocation_bytes(&self) -> u64 {
        match self {
            Self::Single(geometry) => geometry.allocation_bytes(),
            Self::Lod { bundle, .. } => bundle.allocation_bytes(),
        }
    }
    pub(super) fn for_view(
        &self,
        camera: Option<voxy_render::SceneCamera>,
        world: glam::Mat4,
        viewport: [u32; 2],
        history: &mut voxy_render::SceneLodHistory,
    ) -> &SceneGeometry {
        if let (Self::Lod { bundle, min, max }, Some(camera)) = (self, camera)
            && let Ok(bounds) = camera.lod_bounds(world, *min, *max)
            && let Ok((_, geometry)) = bundle.select_with_history(
                camera,
                bounds,
                viewport,
                voxy_render::LodPolicy {
                    target_pixels: 1.0,
                    hysteresis: 0.15,
                },
                history,
            )
        {
            return geometry;
        }
        history.reset();
        self.base()
    }
}
type SharedGeometry = (Arc<SceneGeometry>, Option<Arc<SceneGeometry>>);
struct CachedTexture {
    image: usize,
    sampling: TextureSampling,
    mips: bool,
    texture: Arc<SceneTexture>,
}
impl ModelGraphics {
    pub(super) fn geometry_residency_valid(&self, asset: &EditorAsset) -> bool {
        let minimum = Self::geometry_estimate(asset);
        let optional = match (&self.geometry, &asset.lod) {
            (ModelGeometry::Lod { bundle, .. }, Some(source)) => source
                .indices()
                .levels()
                .iter()
                .enumerate()
                .skip(1)
                .filter(|(index, _)| bundle.level(*index).is_some())
                .map(|(_, level)| u64::from(level.index_count) * 4)
                .sum(),
            _ => 0,
        };
        self.geometry_bytes() == minimum.saturating_add(optional)
    }
    pub(super) fn geometry_estimate(asset: &EditorAsset) -> u64 {
        let bytes = |mesh: &voxy_render::SceneMesh| {
            SceneRenderer::mesh_allocation_bytes(mesh)
                + selection_outline(mesh)
                    .ok()
                    .as_ref()
                    .map_or(0, SceneRenderer::mesh_allocation_bytes)
        };
        let mut total = asset.lod.as_ref().map_or_else(
            || bytes(&asset.mesh),
            |certificate| {
                SceneRenderer::certified_lod_mesh_allocation_bytes(&asset.mesh, certificate)
                    .map(|_| SceneRenderer::mesh_allocation_bytes(&asset.mesh))
                    .unwrap_or(u64::MAX)
                    .saturating_add(
                        selection_outline(&asset.mesh)
                            .ok()
                            .as_ref()
                            .map_or(0, SceneRenderer::mesh_allocation_bytes),
                    )
            },
        );
        let mut seen = std::collections::BTreeSet::new();
        for node in &asset.nodes {
            if let Some(mesh) = &node.mesh
                && seen.insert(node.geometry_key)
            {
                total += bytes(mesh);
            }
        }
        total
    }

    /// Counts shared primitive/outline allocations once, plus fallback geometry.
    pub(super) fn geometry_bytes(&self) -> u64 {
        let mut bytes = self.geometry.allocation_bytes()
            + self
                .outline
                .as_ref()
                .map_or(0, SceneGeometry::allocation_bytes);
        let mut seen = std::collections::BTreeSet::new();
        for part in self.parts.values() {
            for geometry in std::iter::once(&part.geometry).chain(part.outline.iter()) {
                if seen.insert(Arc::as_ptr(geometry)) {
                    bytes += geometry.allocation_bytes();
                }
            }
        }
        bytes
    }
    pub(super) fn upload(
        renderer: &SceneRenderer,
        host: &SceneSurface,
        asset: &EditorAsset,
        residency_cache: &mut ResidencyCache,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let additional = Self::geometry_estimate(asset);
        if additional
            > residency_cache
                .geometry_budget
                .saturating_sub(residency_cache.geometry_live)
        {
            return Err(format!(
                "GPU geometry budget exceeded: live={} additional={additional} limit={}",
                residency_cache.geometry_live, residency_cache.geometry_budget
            )
            .into());
        }
        residency_cache.preflight(asset)?;
        let geometry = ModelGeometry::upload(renderer, host, asset)?;
        let outline = selection_outline(&asset.mesh)
            .ok()
            .map(|mesh| renderer.upload_mesh(host.device(), &mesh))
            .transpose()?;
        let mut parts = BTreeMap::new();
        let mut geometry_cache: BTreeMap<(usize, usize), SharedGeometry> = BTreeMap::new();
        let mut textures: Vec<CachedTexture> = Vec::new();
        let mut images: BTreeMap<usize, Arc<SceneTexture>> = BTreeMap::new();
        for (index, node) in asset.nodes.iter().enumerate() {
            let Some(mesh) = &node.mesh else {
                continue;
            };
            let key = node.geometry_key.ok_or(SceneError::InvalidGeometry)?;
            let (geometry, outline) = if let Some(geometry) = geometry_cache.get(&key) {
                geometry.clone()
            } else {
                let geometry = Arc::new(renderer.upload_mesh(host.device(), mesh)?);
                let outline = selection_outline(mesh)
                    .ok()
                    .map(|mesh| renderer.upload_mesh(host.device(), &mesh).map(Arc::new))
                    .transpose()?;
                geometry_cache.insert(key, (Arc::clone(&geometry), outline.clone()));
                (geometry, outline)
            };
            let texture = if let Some(image_index) = node.image {
                if let Some(cached) = textures.iter().find(|entry| {
                    entry.image == image_index
                        && entry.sampling == node.sampling
                        && entry.mips == node.use_mips
                }) {
                    Some(Arc::clone(&cached.texture))
                } else {
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        images.entry(image_index)
                    {
                        let image = asset
                            .images
                            .get(image_index)
                            .ok_or(SceneError::InvalidTexture)?;
                        let storage = residency_cache.get_or_upload(renderer, host, image)?;
                        entry.insert(storage);
                    }
                    let image = &images[&image_index];
                    let levels = if node.use_mips {
                        image.texture().mip_level_count()
                    } else {
                        1
                    };
                    let texture =
                        renderer.texture_binding(host.device(), image, node.sampling, levels)?;
                    let texture = Arc::new(texture);
                    textures.push(CachedTexture {
                        image: image_index,
                        sampling: node.sampling,
                        mips: node.use_mips,
                        texture: Arc::clone(&texture),
                    });
                    Some(texture)
                }
            } else {
                None
            };
            parts.insert(
                u32::try_from(index).map_err(|_| SceneError::GeometryCapacityExceeded)?,
                PartGraphics {
                    geometry,
                    outline,
                    texture,
                },
            );
        }
        Ok(Self {
            geometry,
            outline,
            parts,
            _images: images.into_values().collect(),
        })
    }
}

/// One instance in one view. The caller supplies that view's own history;
/// gathering never mutates residency or publishes selection history.
pub(super) struct LodViewRequest<'a> {
    pub asset: &'a voxy_assets::AssetId,
    pub camera: Option<voxy_render::SceneCamera>,
    pub world: glam::Mat4,
    pub viewport: [u32; 2],
    pub previous: Option<usize>,
}
/// Union all views before the exclusive reconciliation step. Iteration order
/// cannot evict or replace another view's requested level.
pub(super) fn collect_lod_requests<'a>(
    models: &BTreeMap<voxy_assets::AssetId, ModelGraphics>,
    views: impl IntoIterator<Item = LodViewRequest<'a>>,
) -> BTreeMap<voxy_assets::AssetId, std::collections::BTreeSet<usize>> {
    let mut requested = BTreeMap::<_, std::collections::BTreeSet<_>>::new();
    for view in views {
        if let Some(model) = models.get(view.asset)
            && let Some(level) = model.geometry.desired_for_view(
                view.camera,
                view.world,
                view.viewport,
                view.previous,
            )
        {
            requested
                .entry(view.asset.clone())
                .or_default()
                .insert(level);
        }
    }
    requested
}

pub(super) fn reconcile_lod_residency(
    renderer: &SceneRenderer,
    device: &wgpu::Device,
    models: &mut BTreeMap<voxy_assets::AssetId, ModelGraphics>,
    requested: &BTreeMap<voxy_assets::AssetId, std::collections::BTreeSet<usize>>,
    other_bytes: u64,
    budget: u64,
) -> (
    u64,
    std::collections::BTreeSet<(voxy_assets::AssetId, usize)>,
) {
    let mandatory = other_bytes
        + models
            .values()
            .map(|model| {
                model.geometry_bytes() - model.geometry.allocation_bytes()
                    + model.geometry.base().allocation_bytes()
            })
            .sum::<u64>();
    let mut candidates = Vec::new();
    for (asset, levels) in requested {
        if let Some(ModelGraphics {
            geometry: ModelGeometry::Lod { bundle, .. },
            ..
        }) = models.get(asset)
        {
            for (round, &level) in levels.iter().rev().filter(|&&level| level != 0).enumerate() {
                if let Some(metadata) = bundle.level_metadata(level) {
                    let bytes = u64::from(metadata.index_count) * 4;
                    let saved = u64::from(
                        bundle.level_metadata(0).unwrap().index_count - metadata.index_count,
                    );
                    candidates.push((
                        round,
                        asset.clone(),
                        level,
                        bytes,
                        saved,
                        bundle.level(level).is_some(),
                    ));
                }
            }
        }
    }
    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| {
                (u128::from(right.4) * u128::from(left.3))
                    .cmp(&(u128::from(left.4) * u128::from(right.3)))
            })
            .then_with(|| right.5.cmp(&left.5))
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| right.2.cmp(&left.2))
    });
    let mut available = budget.saturating_sub(mandatory);
    let mut admitted = BTreeMap::<voxy_assets::AssetId, std::collections::BTreeSet<usize>>::new();
    let mut pending = std::collections::BTreeSet::new();
    for (_, asset, level, bytes, _, _) in candidates {
        if bytes <= available {
            available -= bytes;
            admitted.entry(asset).or_default().insert(level);
        } else {
            pending.insert((asset, level));
        }
    }
    let empty = std::collections::BTreeSet::new();
    for (asset, model) in models.iter_mut() {
        model
            .geometry
            .evict_unrequested(admitted.get(asset).unwrap_or(&empty));
    }
    let mut live = other_bytes
        + models
            .values()
            .map(ModelGraphics::geometry_bytes)
            .sum::<u64>();
    for (asset, levels) in &admitted {
        if let Some(model) = models.get_mut(asset) {
            for &level in levels {
                let before = model.geometry.allocation_bytes();
                let available = budget.saturating_sub(live.saturating_sub(before));
                if model
                    .geometry
                    .ensure_level(renderer, device, level, available)
                    .is_err()
                {
                    pending.insert((asset.clone(), level));
                }
                live += model.geometry.allocation_bytes() - before;
            }
        }
    }
    (live, pending)
}

#[cfg(test)]
mod tests {
    #[test]
    fn global_lod_admission_retains_all_requests_and_recovers_after_eviction() {
        use super::*;
        use std::collections::BTreeSet;
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let mesh = voxy_render::SceneMesh::new(
            positions
                .map(|position| voxy_render::SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
                .to_vec(),
            vec![0, 1, 2],
        )
        .unwrap();
        let d = voxy_render::LOD_BARYCENTRIC_DENOMINATOR;
        let witness = voxy_render::LodTriangleWitness {
            target_triangle: 0,
            weights: [[d, 0, 0], [0, d, 0], [0, 0, d]],
        };
        let variant = || voxy_render::CertifiedLodVariant {
            indices: vec![0, 1, 2],
            source_to_variant: vec![witness],
            variant_to_source: vec![witness],
        };
        let source = Arc::new(
            voxy_render::CertifiedLodIndexSet::new(
                positions.to_vec(),
                vec![0, 1, 2],
                vec![variant(), variant()],
            )
            .unwrap(),
        );
        let model = || ModelGraphics {
            geometry: ModelGeometry::Lod {
                bundle: renderer
                    .upload_streaming_certified_lod_mesh(&device, &mesh, source.clone())
                    .unwrap(),
                min: glam::Vec3::ZERO,
                max: glam::Vec3::ONE,
            },
            outline: None,
            parts: BTreeMap::new(),
            _images: vec![],
        };
        let a = voxy_assets::AssetId("a".into());
        let b = voxy_assets::AssetId("b".into());
        let mut models = BTreeMap::from([(a.clone(), model()), (b.clone(), model())]);
        let base = models
            .values()
            .map(ModelGraphics::geometry_bytes)
            .sum::<u64>();
        // Residency wins equal-quality ties even when its asset sorts later.
        let only_b = BTreeMap::from([(b.clone(), BTreeSet::from([1]))]);
        let (_, pending) =
            reconcile_lod_residency(&renderer, &device, &mut models, &only_b, 64, base + 64 + 12);
        assert!(pending.is_empty());
        let both = BTreeMap::from([
            (a.clone(), BTreeSet::from([1])),
            (b.clone(), BTreeSet::from([1])),
        ]);
        let (live, pending) =
            reconcile_lod_residency(&renderer, &device, &mut models, &both, 64, base + 64 + 12);
        assert_eq!(live, base + 64 + 12);
        assert_eq!(pending, BTreeSet::from([(a.clone(), 1)]));
        assert_eq!(models[&a].geometry.allocation_bytes(), base / 2);
        assert_eq!(models[&b].geometry.allocation_bytes(), base / 2 + 12);
        // Mandatory bases survive even when the configured budget is too small.
        let (live, pending) =
            reconcile_lod_residency(&renderer, &device, &mut models, &both, 64, 0);
        assert_eq!(live, base + 64);
        assert_eq!(pending, BTreeSet::from([(a.clone(), 1), (b.clone(), 1)]));
        let mut requests = BTreeMap::from([
            (a.clone(), BTreeSet::from([1, 2])),
            (b.clone(), BTreeSet::from([1])),
        ]);
        let (live, pending) = reconcile_lod_residency(
            &renderer,
            &device,
            &mut models,
            &requests,
            64,
            base + 64 + 24,
        );
        assert_eq!(live, base + 64 + 24);
        assert_eq!(pending, BTreeSet::from([(a.clone(), 1)]));
        assert_eq!(models[&a].geometry.allocation_bytes(), base / 2 + 12);
        assert_eq!(models[&b].geometry.allocation_bytes(), base / 2 + 12);
        requests.get_mut(&a).unwrap().remove(&2);
        let (live, pending) = reconcile_lod_residency(
            &renderer,
            &device,
            &mut models,
            &requests,
            64,
            base + 64 + 24,
        );
        assert_eq!(live, base + 64 + 24);
        assert!(pending.is_empty());
        assert_eq!(models[&a].geometry.allocation_bytes(), base / 2 + 12);
        assert_eq!(models[&b].geometry.allocation_bytes(), base / 2 + 12);
        let (live, pending) = reconcile_lod_residency(
            &renderer,
            &device,
            &mut models,
            &BTreeMap::new(),
            64,
            base + 64,
        );
        assert_eq!(live, base + 64);
        assert!(pending.is_empty());
        assert_eq!(models[&a].geometry.base().index_count(), 3);
        requests.get_mut(&a).unwrap().insert(2);
        let (live, pending) = reconcile_lod_residency(
            &renderer,
            &device,
            &mut models,
            &requests,
            64,
            base + 64 + 36,
        );
        assert_eq!(live, base + 64 + 36);
        assert!(pending.is_empty());
        // Contracting a populated budget still gives each model its first level.
        // Repeating reconciliation must preserve the admitted set and byte count.
        for _ in 0..2 {
            let (live, pending) = reconcile_lod_residency(
                &renderer,
                &device,
                &mut models,
                &requests,
                64,
                base + 64 + 24,
            );
            assert_eq!(live, base + 64 + 24);
            assert_eq!(pending, BTreeSet::from([(a.clone(), 1)]));
            let ModelGeometry::Lod {
                bundle: a_bundle, ..
            } = &models[&a].geometry
            else {
                panic!("expected LOD model");
            };
            let ModelGeometry::Lod {
                bundle: b_bundle, ..
            } = &models[&b].geometry
            else {
                panic!("expected LOD model");
            };
            assert!(a_bundle.level(1).is_none());
            assert!(a_bundle.level(2).is_some());
            assert!(b_bundle.level(1).is_some());
        }
        let (live, pending) = reconcile_lod_residency(
            &renderer,
            &device,
            &mut models,
            &requests,
            64,
            base + 64 + 36,
        );
        assert_eq!(live, base + 64 + 36);
        assert!(pending.is_empty());
        // A certified reduction outranks a zero-saving level even if its asset
        // sorts later and the zero-saving level is already resident.
        let quad_positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]];
        let quad = voxy_render::SceneMesh::new(
            quad_positions
                .map(|position| voxy_render::SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
                .to_vec(),
            vec![0, 1, 3, 0, 3, 2],
        )
        .unwrap();
        let quad_source = Arc::new(
            voxy_render::CertifiedLodIndexSet::new(
                quad_positions.to_vec(),
                vec![0, 1, 3, 0, 3, 2],
                vec![voxy_render::CertifiedLodVariant {
                    indices: vec![0, 1, 2],
                    source_to_variant: vec![
                        voxy_render::LodTriangleWitness {
                            target_triangle: 0,
                            weights: [[d, 0, 0], [0, d, 0], [0, d / 2, d / 2]],
                        },
                        voxy_render::LodTriangleWitness {
                            target_triangle: 0,
                            weights: [[d, 0, 0], [0, d / 2, d / 2], [0, 0, d]],
                        },
                    ],
                    variant_to_source: vec![voxy_render::LodTriangleWitness {
                        target_triangle: 0,
                        weights: [[d, 0, 0], [0, d, 0], [d / 2, 0, d / 2]],
                    }],
                }],
            )
            .unwrap(),
        );
        let mut efficient = BTreeMap::from([
            (a.clone(), model()),
            (
                b.clone(),
                ModelGraphics {
                    geometry: ModelGeometry::Lod {
                        bundle: renderer
                            .upload_streaming_certified_lod_mesh(&device, &quad, quad_source)
                            .unwrap(),
                        min: glam::Vec3::ZERO,
                        max: glam::Vec3::ONE,
                    },
                    outline: None,
                    parts: BTreeMap::new(),
                    _images: vec![],
                },
            ),
        ]);
        let mandatory = efficient
            .values()
            .map(ModelGraphics::geometry_bytes)
            .sum::<u64>();
        let only_a = BTreeMap::from([(a.clone(), BTreeSet::from([1]))]);
        let (_, pending) = reconcile_lod_residency(
            &renderer,
            &device,
            &mut efficient,
            &only_a,
            0,
            mandatory + 12,
        );
        assert!(pending.is_empty());
        let (live, pending) =
            reconcile_lod_residency(&renderer, &device, &mut efficient, &both, 0, mandatory + 12);
        assert_eq!(live, mandatory + 12);
        assert_eq!(pending, BTreeSet::from([(a.clone(), 1)]));
        let ModelGeometry::Lod { bundle, .. } = &efficient[&b].geometry else {
            panic!("expected LOD");
        };
        assert!(bundle.level(1).is_some());
        let camera = |half: f32| voxy_render::SceneCamera {
            eye: glam::Vec3::new(0.5, 0.5, 5.),
            target: glam::Vec3::new(0.5, 0.5, 0.),
            up: glam::Vec3::Y,
            projection: voxy_render::SceneProjection::Orthographic {
                left: -half,
                right: half,
                bottom: -half,
                top: half,
                near: 0.1,
                far: 100.,
            },
        };
        let views = || {
            [
                LodViewRequest {
                    asset: &b,
                    camera: Some(camera(1.)),
                    world: glam::Mat4::IDENTITY,
                    viewport: [512, 512],
                    previous: None,
                },
                LodViewRequest {
                    asset: &b,
                    camera: Some(camera(1000.)),
                    world: glam::Mat4::IDENTITY,
                    viewport: [512, 512],
                    previous: None,
                },
            ]
        };
        let union = collect_lod_requests(&efficient, views());
        assert_eq!(union, BTreeMap::from([(b.clone(), BTreeSet::from([0, 1]))]));
        assert_eq!(
            union,
            collect_lod_requests(&efficient, views().into_iter().rev())
        );
        let (live, pending) = reconcile_lod_residency(
            &renderer,
            &device,
            &mut efficient,
            &union,
            0,
            mandatory + 12,
        );
        assert_eq!(live, mandatory + 12);
        assert!(pending.is_empty());
        let mut near_history = voxy_render::SceneLodHistory::default();
        let mut far_history = voxy_render::SceneLodHistory::default();
        for _ in 0..2 {
            assert_eq!(
                efficient[&b]
                    .geometry
                    .for_view(
                        Some(camera(1.)),
                        glam::Mat4::IDENTITY,
                        [512, 512],
                        &mut near_history,
                    )
                    .index_count(),
                6
            );
            assert_eq!(
                efficient[&b]
                    .geometry
                    .for_view(
                        Some(camera(1000.)),
                        glam::Mat4::IDENTITY,
                        [512, 512],
                        &mut far_history,
                    )
                    .index_count(),
                3
            );
            assert_eq!(near_history.previous(), Some(0));
            assert_eq!(far_history.previous(), Some(1));
        }
        let outline = selection_outline(&mesh).unwrap();
        models.get_mut(&a).unwrap().outline =
            Some(renderer.upload_mesh(&device, &outline).unwrap());
        let asset = EditorAsset {
            mesh,
            lod: Some(source),
            animated: None,
            skinned_lod: None,
            nodes: vec![],
            images: vec![],
        };
        assert!(models[&a].geometry_residency_valid(&asset));
        models.get_mut(&a).unwrap().outline = None;
        assert!(!models[&a].geometry_residency_valid(&asset));
    }
    use super::*;
    #[test]
    fn deferral_wakes_on_source_budget_or_residency_change() {
        let source = || {
            Arc::new(
                voxy_assets::ImportInputs::new(1, 1)
                    .finish(
                        EditorAsset {
                            lod: None,
                            animated: None,
                            skinned_lod: None,
                            mesh: voxy_render::SceneMesh::quad([1.; 4]),
                            nodes: vec![],
                            images: vec![],
                        },
                        |_, _| Ok(vec![]),
                    )
                    .unwrap(),
            )
        };
        let first = source();
        let other = source();
        let mut cache = ResidencyCache::with_budget(4);
        let deferred = DeferredUpload::new(&first, &cache, "pressure".into());
        assert!(deferred.blocks(&Arc::clone(&first), &cache));
        assert!(!deferred.blocks(&other, &cache));
        assert_eq!(Arc::strong_count(&first), 1);
        cache.budget = 8;
        assert!(!deferred.blocks(&first, &cache));
        cache.budget = 4;
        cache.geometry_live = 1;
        assert!(!deferred.blocks(&first, &cache));
        cache.geometry_live = 0;
        cache.geometry_budget += 1;
        assert!(!deferred.blocks(&first, &cache));
        cache.geometry_budget -= 1;
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let image = Arc::new(
            renderer
                .upload_texture(&device, &queue, 1, 1, &[255; 4])
                .unwrap(),
        );
        cache.images.insert([0; 32], (Arc::downgrade(&image), 4));
        let occupied = DeferredUpload::new(&first, &cache, "pressure".into());
        drop(image);
        cache.prune();
        assert!(!occupied.blocks(&first, &cache));
        assert!(!deferred.blocks(&first, &cache));
    }
    #[test]
    fn residency_drops_with_last_owner_and_dead_entries_are_pruned() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let image = Arc::new(
            renderer
                .upload_texture(&device, &queue, 1, 1, &[255; 4])
                .unwrap(),
        );
        let owner = Arc::clone(&image);
        let mut cache = ResidencyCache::default();
        cache.images.insert([0; 32], (Arc::downgrade(&image), 4));
        cache.budget = 4;
        assert!(cache.admit(0).is_ok());
        assert!(cache.admit(1).is_err());
        assert_eq!(mip_bytes(4, 4), 84);
        assert_eq!(mip_bytes(1, 4), 28);
        drop(image);
        cache.prune();
        assert_eq!(cache.images.len(), 1);
        drop(owner);
        assert!(cache.images[&[0; 32]].0.upgrade().is_none());
        cache.prune();
        assert!(cache.images.is_empty());
        assert!(cache.admit(4).is_ok());
    }
}
