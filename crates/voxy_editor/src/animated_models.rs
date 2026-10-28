//! Scene-owner playback and deformation. Immutable attributes are shared by revision;
//! clocks, palettes and posed geometry are owned by generational scene handles.
pub(crate) mod checkpoint;
use crate::EditorError;
use crate::model_playback::ModelAnimation;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};
use voxy_model::{ModelAsset, ModelGeometry};
use voxy_render::{
    SceneGeometry, SceneRenderer, SceneSkinError, SceneSkinInstance, SceneSkinSource, SceneSkinner,
};
use voxy_scene::NodeId;

#[derive(Debug)]
pub(super) struct PosedGeometry<'a> {
    pub geometry: &'a SceneGeometry,
    pub gpu_deformed: bool,
}
#[derive(Debug)]
enum Primitive {
    Skin {
        instance: SceneSkinInstance,
        source: Arc<SceneSkinSource>,
    },
    Baked(SceneGeometry),
}
impl Primitive {
    fn geometry(&self) -> &SceneGeometry {
        match self {
            Self::Skin { instance, .. } => instance.geometry(),
            Self::Baked(geometry) => geometry,
        }
    }
    fn bytes(&self) -> u64 {
        match self {
            Self::Skin { instance, .. } => instance.allocation_bytes(),
            Self::Baked(geometry) => geometry.allocation_bytes(),
        }
    }
}
#[derive(Debug)]
struct Owner {
    publication: Option<Arc<voxy_assets::ImportedAsset<crate::import::EditorAsset>>>,
    model: Arc<ModelAsset>,
    settings: ModelAnimation,
    frame: Arc<voxy_animation::AnimatorFrame>,
    ticks: u64,
    primitives: Vec<Primitive>,
    lod: Option<Arc<voxy_render::SkinnedLodMesh>>,
    textures: Vec<Option<Arc<voxy_render::SceneTexture>>>,
    // Keep residency-cache storage alive while an accepted owner's bindings live.
    texture_storage: Vec<Arc<voxy_render::SceneTexture>>,
    prepared_lod: Option<voxy_render::PreparedSkinnedLod>,
    lod_levels: HashMap<usize, LodGeometry>,
    lod_history: HashMap<u8, usize>,
}
#[derive(Debug)]
enum LodGeometry {
    Gpu(voxy_render::SceneSkinLodLevel),
    Cpu(SceneGeometry),
}
impl LodGeometry {
    fn geometry(&self) -> &SceneGeometry {
        match self {
            Self::Gpu(level) => level.geometry(),
            Self::Cpu(mesh) => mesh,
        }
    }
    fn bytes(&self) -> u64 {
        match self {
            Self::Gpu(level) => level.index_allocation_bytes(),
            Self::Cpu(mesh) => mesh.allocation_bytes(),
        }
    }
}
struct LodUpload<'a> {
    renderer: &'a SceneRenderer,
    device: &'a wgpu::Device,
    skinner: Option<&'a SceneSkinner>,
    live: u64,
    budget: u64,
}
impl LodUpload<'_> {
    fn stage(
        &self,
        state: &Owner,
        source: &voxy_render::SkinnedLodMesh,
        level: usize,
    ) -> Result<LodGeometry, EditorError> {
        if let (Some(skinner), Some(Primitive::Skin { instance, .. })) =
            (self.skinner, state.primitives.first())
        {
            Ok(LodGeometry::Gpu(skinner.create_lod_level(
                instance,
                source,
                level,
                self.live,
                self.budget,
            )?))
        } else {
            let local = source.prepare(&state.frame.skin_matrices, glam::Mat4::IDENTITY)?;
            let mesh = local
                .posed_scene_mesh(level, state.model.primitives[0].color)?
                .with_material_coordinates(
                    source
                        .mesh()
                        .vertices()
                        .iter()
                        .map(|v| v.position)
                        .collect(),
                )?;
            admit(
                self.live,
                SceneRenderer::mesh_allocation_bytes(&mesh),
                self.budget,
            )?;
            Ok(LodGeometry::Cpu(
                self.renderer.upload_mesh(self.device, &mesh)?,
            ))
        }
    }
}
#[derive(Debug)]
struct SharedSource {
    model: Weak<ModelAsset>,
    source: Weak<SceneSkinSource>,
}
#[derive(Debug)]
pub(super) struct AnimatedModels {
    authoring_frames: std::cell::RefCell<HashMap<NodeId, AuthoringFrame>>,
    skinner: Option<SceneSkinner>,
    owners: HashMap<NodeId, Owner>,
    sources: HashMap<(usize, usize), SharedSource>,
}
#[derive(Debug)]
struct AuthoringFrame {
    model: Weak<ModelAsset>,
    settings: ModelAnimation,
    frame: Arc<voxy_animation::AnimatorFrame>,
}

pub(super) struct Request {
    pub publication: Option<Arc<voxy_assets::ImportedAsset<crate::import::EditorAsset>>>,
    pub owner: NodeId,
    pub model: Arc<ModelAsset>,
    pub settings: ModelAnimation,
    pub frame: Option<Arc<voxy_animation::AnimatorFrame>>,
    pub lod: Option<Arc<voxy_render::SkinnedLodMesh>>,
    pub textures: Vec<Option<Arc<voxy_render::SceneTexture>>>,
    pub texture_storage: Vec<Arc<voxy_render::SceneTexture>>,
}

fn admit(live: u64, additional: u64, budget: u64) -> Result<(), EditorError> {
    if live > budget || additional > budget.saturating_sub(live) {
        Err(format!(
            "animation geometry budget exceeded: live={live} additional={additional} budget={budget}"
        ).into())
    } else {
        Ok(())
    }
}
impl AnimatedModels {
    /// Test oracle: use the existing complete CPU frame evaluator and uploader.
    #[cfg(test)]
    pub(super) fn new_cpu_reference(renderer: &SceneRenderer) -> Result<Self, EditorError> {
        let mut models = Self::new(renderer)?;
        models.skinner = None;
        Ok(models)
    }
    #[cfg(test)]
    pub(super) fn source_publications(
        &self,
    ) -> Result<Vec<Arc<voxy_assets::ImportedAsset<crate::import::EditorAsset>>>, EditorError> {
        Ok(self
            .source_publication_owners()?
            .into_iter()
            .map(|(_, p)| p)
            .collect())
    }
    pub(super) fn source_publication_owners(
        &self,
    ) -> Result<
        Vec<(
            NodeId,
            Arc<voxy_assets::ImportedAsset<crate::import::EditorAsset>>,
        )>,
        EditorError,
    > {
        let mut owners: Vec<_> = self.owners.iter().collect();
        owners.sort_by_key(|(node, _)| **node);
        owners
            .into_iter()
            .map(|(node, owner)| {
                let publication = owner
                    .publication
                    .clone()
                    .ok_or("animated GPU owner has no retained CPU publication")?;
                if publication
                    .value()
                    .animated
                    .as_ref()
                    .is_none_or(|model| !Arc::ptr_eq(model, &owner.model))
                {
                    return Err("animated GPU owner publication differs from model".into());
                }
                Ok((*node, publication))
            })
            .collect()
    }
    /// Authoring uses the bind pose plus saved overrides, without advancing any
    /// gameplay clock. Cache only the current revision/settings for each owner.
    pub(super) fn authoring_frame(
        &self,
        owner: NodeId,
        model: &Arc<ModelAsset>,
        settings: &ModelAnimation,
    ) -> Result<Arc<voxy_animation::AnimatorFrame>, EditorError> {
        if let Some(cached) = self.authoring_frames.borrow().get(&owner)
            && cached
                .model
                .upgrade()
                .is_some_and(|old| Arc::ptr_eq(&old, model))
            && &cached.settings == settings
        {
            return Ok(cached.frame.clone());
        }
        let mut frame = model.bind_frame()?;
        settings.apply_morph_weights(model, &mut frame)?;
        let frame = Arc::new(frame);
        self.authoring_frames.borrow_mut().insert(
            owner,
            AuthoringFrame {
                model: Arc::downgrade(model),
                settings: settings.clone(),
                frame: frame.clone(),
            },
        );
        Ok(frame)
    }

    /// The exact model revision and frame whose geometry was admitted for display.
    pub(super) fn accepted_frame(
        &self,
        owner: NodeId,
    ) -> Option<(&Arc<ModelAsset>, &Arc<voxy_animation::AnimatorFrame>)> {
        let accepted = self.owners.get(&owner)?;
        Some((&accepted.model, &accepted.frame))
    }

    pub(super) fn texture(
        &self,
        owner: NodeId,
        primitive: usize,
    ) -> Option<&voxy_render::SceneTexture> {
        self.owners.get(&owner)?.textures.get(primitive)?.as_deref()
    }

    pub(super) fn new(renderer: &SceneRenderer) -> Result<Self, EditorError> {
        let skinner = match SceneSkinner::new(renderer) {
            Ok(s) => Some(s),
            Err(SceneSkinError::Unsupported) => None,
            Err(e) => return Err(e.to_string().into()),
        };
        Ok(Self {
            authoring_frames: Default::default(),
            skinner,
            owners: HashMap::new(),
            sources: HashMap::new(),
        })
    }
    pub(super) fn pose_signature(
        &self,
        owner: NodeId,
    ) -> Result<Option<Vec<[u32; 16]>>, EditorError> {
        let Some(owner) = self.owners.get(&owner) else {
            return Ok(None);
        };
        Ok(Some(
            owner
                .frame
                .skin_matrices
                .iter()
                .map(|matrix| matrix.to_cols_array().map(f32::to_bits))
                .collect(),
        ))
    }
    pub(super) fn lod_selection(&self, owner: NodeId, view: u8) -> Option<(usize, u32)> {
        let state = self.owners.get(&owner)?;
        let level = *state.lod_history.get(&view)?;
        let geometry = state
            .lod_levels
            .get(&level)
            .map_or_else(|| state.primitives[0].geometry(), LodGeometry::geometry);
        Some((level, geometry.index_count()))
    }
    pub(super) fn lod_world_bounds(
        &self,
        owner: NodeId,
        world: glam::Mat4,
    ) -> Result<Option<(glam::Vec3, glam::Vec3)>, EditorError> {
        let Some(state) = self.owners.get(&owner) else {
            return Ok(None);
        };
        let Some(source) = &state.lod else {
            return Ok(None);
        };
        let frame = &state.frame;
        let posed = source.prepare(&frame.skin_matrices, world)?;
        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
        for position in posed.positions() {
            let p = glam::Vec3::from_array(*position);
            min = min.min(p);
            max = max.max(p);
        }
        Ok(Some((min, max)))
    }
    /// Evidence from the accepted palette used to update this owner's streams.
    /// Called only by opt-in diagnostics after presentation.
    pub(super) fn pose_evidence(&self, owner: NodeId) -> Option<(u64, String)> {
        let state = self.owners.get(&owner)?;
        let mut hash = blake3::Hasher::new();
        for joint in &state.frame.skin_matrices {
            for value in joint.to_cols_array() {
                hash.update(&value.to_bits().to_le_bytes());
            }
        }
        Some((state.ticks, hash.finalize().to_hex().to_string()))
    }
    pub(super) fn counts(&self) -> (usize, usize, usize) {
        let mut sources = HashSet::new();
        let mut primitives = 0;
        for owner in self.owners.values() {
            for primitive in &owner.primitives {
                if let Primitive::Skin { source, .. } = primitive {
                    primitives += 1;
                    sources.insert(Arc::as_ptr(source) as usize);
                }
            }
        }
        (self.owners.len(), primitives, sources.len())
    }
    pub(super) fn clear(&mut self) {
        self.authoring_frames.get_mut().clear();
        self.owners.clear();
        self.sources.clear();
    }
    pub(super) fn allocation_bytes(&self) -> u64 {
        let mut seen = HashSet::new();
        self.owners
            .values()
            .flat_map(|owner| &owner.primitives)
            .map(|primitive| {
                let source = match primitive {
                    Primitive::Skin { source, .. } if seen.insert(Arc::as_ptr(source) as usize) => {
                        source.allocation_bytes()
                    }
                    _ => 0,
                };
                primitive.bytes() + source
            })
            .sum::<u64>()
            + self
                .owners
                .values()
                .flat_map(|owner| owner.lod_levels.values())
                .map(LodGeometry::bytes)
                .sum::<u64>()
    }
    pub(super) fn geometries(&self, owner: NodeId) -> Option<impl Iterator<Item = &SceneGeometry>> {
        self.owners
            .get(&owner)
            .map(|state| state.primitives.iter().map(Primitive::geometry))
    }
    #[cfg(test)]
    pub(super) fn geometries_for_view(
        &self,
        owner: NodeId,
        view: u8,
    ) -> Option<impl Iterator<Item = &SceneGeometry>> {
        self.geometry_inputs_for_view(owner, view)
            .map(|inputs| inputs.map(|input| input.geometry))
    }
    pub(super) fn geometry_inputs_for_view(
        &self,
        owner: NodeId,
        view: u8,
    ) -> Option<impl Iterator<Item = PosedGeometry<'_>>> {
        self.owners.get(&owner).map(move |state| {
            let selected = state
                .lod_history
                .get(&view)
                .and_then(|level| state.lod_levels.get(level));
            state
                .primitives
                .iter()
                .enumerate()
                .map(move |(index, primitive)| {
                    if index == 0
                        && let Some(selected) = selected
                    {
                        PosedGeometry {
                            geometry: selected.geometry(),
                            gpu_deformed: matches!(selected, LodGeometry::Gpu(_)),
                        }
                    } else {
                        PosedGeometry {
                            geometry: primitive.geometry(),
                            gpu_deformed: matches!(primitive, Primitive::Skin { .. }),
                        }
                    }
                })
        })
    }
    /// Retire closed views before admission. Levels selected by any remaining
    /// view stay pinned, including last-good selections after a failed update.
    pub(super) fn retain_lod_views(&mut self, views: &[u8]) {
        for state in self.owners.values_mut() {
            state.lod_history.retain(|view, _| views.contains(view));
        }
        self.evict_unused_lod();
    }
    /// Run after all view selections and before borrowing draw geometry.
    /// Base geometry is permanently resident until owner retirement.
    pub(super) fn evict_unused_lod(&mut self) -> u64 {
        let mut released = 0;
        for state in self.owners.values_mut() {
            state.lod_levels.retain(|level, geometry| {
                let keep = state.lod_history.values().any(|selected| selected == level);
                if !keep {
                    released += geometry.bytes();
                }
                keep
            });
        }
        released
    }
    /// Selection/admission succeeds before publishing per-view history.
    pub(super) fn select_lod(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        owner: NodeId,
        view: u8,
        camera: Option<voxy_render::SceneCamera>,
        world: glam::Mat4,
        viewport: [u32; 2],
        other_live: u64,
        budget: u64,
    ) -> Result<(), EditorError> {
        let Some(camera) = camera else {
            return Ok(());
        };
        let live = other_live.saturating_add(self.allocation_bytes());
        let Some(state) = self.owners.get_mut(&owner) else {
            return Ok(());
        };
        let Some(source) = &state.lod else {
            return Ok(());
        };
        let frame = &state.frame;
        // Exact palette/world equality permits reuse across cameras and paused frames.
        // A failed replacement leaves the accepted certificate and view history intact.
        let candidate = if state.prepared_lod.as_ref().is_none_or(|posed| {
            posed.model() != world || posed.joints() != frame.skin_matrices.as_slice()
        }) {
            Some(source.prepare(&frame.skin_matrices, world)?)
        } else {
            None
        };
        let posed = candidate.as_ref().or(state.prepared_lod.as_ref()).unwrap();
        let level = posed
            .select_for_camera(
                camera,
                viewport,
                voxy_render::LodPolicy {
                    target_pixels: 1.0,
                    hysteresis: 0.15,
                },
                state.lod_history.get(&view).copied(),
            )
            .map_err(|e| format!("{e:?}"))?;
        if level > 0 && !state.lod_levels.contains_key(&level) {
            let geometry = LodUpload {
                renderer,
                device,
                skinner: self.skinner.as_ref(),
                live,
                budget,
            }
            .stage(state, source, level)?;
            state.lod_levels.insert(level, geometry);
        }
        if let Some(candidate) = candidate {
            state.prepared_lod = Some(candidate);
        }
        state.lod_history.insert(view, level);
        Ok(())
    }
    /// Publish immutable fixed-tick frames to all views. No clock advances here.
    /// GPU admission failures retain the previously rendered frame.
    pub(super) fn synchronize(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        requests: Vec<Request>,
        ticks: u64,
        other_live: u64,
        budget: u64,
    ) -> Vec<(NodeId, EditorError)> {
        let active: HashSet<_> = requests.iter().map(|r| r.owner).collect();
        self.owners.retain(|owner, _| active.contains(owner));
        self.authoring_frames
            .borrow_mut()
            .retain(|owner, _| active.contains(owner));
        self.sources
            .retain(|_, entry| entry.source.strong_count() > 0 && entry.model.strong_count() > 0);
        let mut errors = Vec::new();
        for request in requests {
            // A render-only frame may precede the first fixed tick or accepted
            // revision. Keep any prior geometry while the animation owner waits.
            if request.frame.is_none() {
                continue;
            }
            if let Err(error) = self.update(
                renderer,
                device,
                queue,
                request.owner,
                request.model,
                request.settings,
                request.frame,
                request.lod,
                request.textures,
                request.texture_storage,
                request.publication,
                ticks,
                other_live,
                budget,
            ) {
                errors.push((request.owner, error));
            }
        }
        errors
    }
    fn update(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        owner: NodeId,
        model: Arc<ModelAsset>,
        settings: ModelAnimation,
        frame: Option<Arc<voxy_animation::AnimatorFrame>>,
        lod: Option<Arc<voxy_render::SkinnedLodMesh>>,
        textures: Vec<Option<Arc<voxy_render::SceneTexture>>>,
        texture_storage: Vec<Arc<voxy_render::SceneTexture>>,
        publication: Option<Arc<voxy_assets::ImportedAsset<crate::import::EditorAsset>>>,
        ticks: u64,
        other_live: u64,
        budget: u64,
    ) -> Result<(), EditorError> {
        let frame = frame.ok_or("waiting for an accepted fixed-tick animation frame")?;
        // Playback selection belongs to the accepted owner and may use a separate
        // source rig. Rendering admits descriptor values and the target-bound frame.
        settings.validate(None, None)?;
        if frame.skin_matrices != frame.pose.skin_matrices(&model.skeleton)? {
            return Err("animation frame palette differs from pose".into());
        }
        if !textures.is_empty() && textures.len() != model.primitives.len() {
            return Err("animation material count mismatch".into());
        }
        for (index, primitive) in model.primitives.iter().enumerate() {
            if primitive.base_color_texture.is_some()
                && textures.get(index).is_none_or(Option::is_none)
            {
                return Err("animated material texture is not GPU-ready".into());
            }
        }
        let current = self.owners.get(&owner);
        let replace = current.is_none_or(|old| !Arc::ptr_eq(&old.model, &model));
        let lod_changed = current.is_none_or(|old| match (&old.lod, &lod) {
            (Some(old), Some(new)) => !Arc::ptr_eq(old, new),
            (None, None) => false,
            _ => true,
        });
        let unchanged = current.is_some_and(|old| {
            old.frame.pose == frame.pose
                && old.frame.skin_matrices == frame.skin_matrices
                && old.frame.properties == frame.properties
        });
        if !replace && !lod_changed && unchanged {
            let state = self.owners.get_mut(&owner).unwrap();
            state.textures = textures;
            state.texture_storage = texture_storage;
            state.publication = publication;
            state.frame = frame;
            state.settings = settings;
            state.ticks = ticks;
            return Ok(());
        }
        let weights = model.frame_weights(&frame)?;
        if model.has_morph() && lod.is_some() {
            return Err("morph LOD needs a morph-aware error certificate".into());
        }
        let needs_cpu = self.skinner.is_none()
            || model
                .primitives
                .iter()
                .any(|p| matches!(p.geometry, ModelGeometry::Static(_)))
            || (model.has_morph() && self.skinner.as_ref().is_some_and(|s| !s.supports_morph()));
        let baked = needs_cpu
            .then(|| model.scene_meshes_frame(&frame))
            .transpose()?;
        let prepared_lod = if let Some(source) = &lod {
            let Some(primitive) = model.primitives.first() else {
                return Err("missing skeletal LOD primitive".into());
            };
            let ModelGeometry::Skinned(mesh) = &primitive.geometry else {
                return Err("skeletal LOD requires a skin".into());
            };
            if model.primitives.len() != 1
                || mesh.vertices() != source.mesh().vertices()
                || mesh.indices() != source.mesh().indices()
                || mesh.joint_count() != source.mesh().joint_count()
            {
                return Err("skeletal LOD source differs from animation model".into());
            }
            Some(source.prepare(&frame.skin_matrices, glam::Mat4::IDENTITY)?)
        } else {
            None
        };
        let mut live = other_live.saturating_add(self.allocation_bytes());
        let mut staged = Vec::with_capacity(model.primitives.len());
        for (index, primitive) in model.primitives.iter().enumerate() {
            let gpu_skinner = self
                .skinner
                .as_ref()
                .filter(|s| weights[index].is_empty() || s.supports_morph());
            if !replace
                && gpu_skinner.is_some()
                && current
                    .is_some_and(|old| matches!(old.primitives[index], Primitive::Skin { .. }))
                && matches!(primitive.geometry, ModelGeometry::Skinned(_))
            {
                staged.push(None);
                continue;
            }
            let result = match (&primitive.geometry, gpu_skinner) {
                (ModelGeometry::Skinned(mesh), Some(skinner)) => {
                    let key = (Arc::as_ptr(&model) as usize, index);
                    let source = self.sources.get(&key).and_then(|entry| {
                        entry
                            .model
                            .upgrade()
                            .filter(|m| Arc::ptr_eq(m, &model))
                            .and_then(|_| entry.source.upgrade())
                    });
                    let source = if let Some(source) = source {
                        source
                    } else {
                        let source = if let Some(morph) = model.morph_source(index)? {
                            skinner.upload_morph_source(Arc::new(morph), live, budget)?
                        } else {
                            skinner.upload_source(Arc::new(mesh.clone()), live, budget)?
                        };
                        live += source.allocation_bytes();
                        self.sources.insert(
                            key,
                            SharedSource {
                                model: Arc::downgrade(&model),
                                source: Arc::downgrade(&source),
                            },
                        );
                        source
                    };
                    let instance = if weights[index].is_empty() {
                        skinner.create_instance(
                            renderer,
                            source.clone(),
                            &frame.skin_matrices,
                            primitive.color,
                            live,
                            budget,
                        )?
                    } else {
                        skinner.create_morph_instance(
                            renderer,
                            source.clone(),
                            &frame.skin_matrices,
                            &weights[index],
                            primitive.color,
                            live,
                            budget,
                        )?
                    };
                    live += instance.allocation_bytes();
                    Primitive::Skin { instance, source }
                }
                (geometry, _) => {
                    let mesh = baked
                        .as_ref()
                        .and_then(|meshes| meshes.get(index))
                        .ok_or("missing complete CPU animation frame")?
                        .clone();
                    let coordinates = match geometry {
                        ModelGeometry::Static(source) => {
                            source.explicit_material_coordinates().map_or_else(
                                || source.vertices().iter().map(|v| v.position).collect(),
                                <[_]>::to_vec,
                            )
                        }
                        ModelGeometry::Skinned(source) => {
                            source.vertices().iter().map(|v| v.position).collect()
                        }
                    };
                    let mesh = mesh.with_material_coordinates(coordinates)?;
                    let bytes = SceneRenderer::mesh_allocation_bytes(&mesh);
                    admit(live, bytes, budget)?;
                    let geometry = renderer.upload_mesh(device, &mesh)?;
                    live += bytes;
                    Primitive::Baked(geometry)
                }
            };
            staged.push(Some(result));
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("editor owner animation"),
        });
        if let Some(skinner) = &self.skinner {
            let primitives = if replace {
                None
            } else {
                Some(&current.unwrap().primitives)
            };
            let mut prepared = Vec::new();
            // Every primitive must validate before the first palette write.
            for (i, staged) in staged.iter().enumerate() {
                let primitive = staged
                    .as_ref()
                    .or_else(|| primitives.map(|p| &p[i]))
                    .ok_or("missing animated primitive")?;
                if let Primitive::Skin { instance, .. } = primitive {
                    prepared.push(if weights[i].is_empty() {
                        skinner.prepare_pose(instance, &frame.skin_matrices)?
                    } else {
                        skinner.prepare_morph_pose(instance, &frame.skin_matrices, &weights[i])?
                    });
                }
            }
            skinner.encode_prepared_poses_batch(queue, &mut encoder, &prepared)?;
        }
        queue.submit([encoder.finish()]);
        if replace {
            self.owners.insert(
                owner,
                Owner {
                    publication,
                    model,
                    settings,
                    frame,
                    ticks,
                    primitives: staged.into_iter().map(Option::unwrap).collect(),
                    lod,
                    prepared_lod,
                    textures,
                    texture_storage,
                    lod_levels: HashMap::new(),
                    lod_history: HashMap::new(),
                },
            );
        } else {
            let state = self.owners.get_mut(&owner).unwrap();
            for (i, primitive) in staged.into_iter().enumerate() {
                if let Some(primitive) = primitive {
                    state.primitives[i] = primitive;
                }
            }
            if lod_changed {
                state.lod_history.clear();
            }
            if lod_changed || self.skinner.is_none() {
                state.lod_levels.clear();
            }
            state.lod = lod;
            state.prepared_lod = prepared_lod;
            state.textures = textures;
            state.texture_storage = texture_storage;
            state.publication = publication;
            state.frame = frame;
            state.settings = settings;
            state.ticks = ticks;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests;
