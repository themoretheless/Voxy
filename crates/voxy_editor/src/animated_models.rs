//! Scene-owner playback and deformation. Immutable attributes are shared by revision;
//! clocks, palettes and posed geometry are owned by generational scene handles.
use crate::model_playback::{ModelAnimation, ModelPlayback};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};
use voxy_animation::Pose;
use voxy_render::{
    ModelAsset, ModelGeometry, SceneGeometry, SceneMesh, SceneRenderer, SceneSkinError,
    SceneSkinInstance, SceneSkinSource, SceneSkinner,
};
use voxy_scene::NodeId;

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
    model: Arc<ModelAsset>,
    settings: ModelAnimation,
    playback: ModelPlayback,
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
#[derive(Debug)]
struct SharedSource {
    model: Weak<ModelAsset>,
    source: Weak<SceneSkinSource>,
}
#[derive(Debug)]
pub(super) struct AnimatedModels {
    skinner: Option<SceneSkinner>,
    owners: HashMap<NodeId, Owner>,
    sources: HashMap<(usize, usize), SharedSource>,
}
pub(super) struct Request {
    pub owner: NodeId,
    pub model: Arc<ModelAsset>,
    pub settings: ModelAnimation,
    pub lod: Option<Arc<voxy_render::SkinnedLodMesh>>,
    pub textures: Vec<Option<Arc<voxy_render::SceneTexture>>>,
    pub texture_storage: Vec<Arc<voxy_render::SceneTexture>>,
}

fn admit(live: u64, additional: u64, budget: u64) -> Result<(), String> {
    if live > budget || additional > budget.saturating_sub(live) {
        Err(format!(
            "animation geometry budget exceeded: live={live} additional={additional} budget={budget}"
        ))
    } else {
        Ok(())
    }
}
fn rigid_mesh(model: &ModelAsset, pose: &Pose, mesh: &SceneMesh) -> Result<SceneMesh, String> {
    let transform = model.mesh_transform(pose).map_err(|e| e.to_string())?;
    SceneMesh::new(
        mesh.vertices()
            .iter()
            .map(|v| voxy_render::SceneVertex {
                position: transform
                    .transform_point3(glam::Vec3::from_array(v.position))
                    .to_array(),
                ..*v
            })
            .collect(),
        mesh.indices().to_vec(),
    )
    .map_err(|e| e.to_string())
}
impl AnimatedModels {
    pub(super) fn texture(
        &self,
        owner: NodeId,
        primitive: usize,
    ) -> Option<&voxy_render::SceneTexture> {
        self.owners.get(&owner)?.textures.get(primitive)?.as_deref()
    }

    pub(super) fn new(renderer: &SceneRenderer) -> Result<Self, String> {
        let skinner = match SceneSkinner::new(renderer) {
            Ok(s) => Some(s),
            Err(SceneSkinError::Unsupported) => None,
            Err(e) => return Err(e.to_string()),
        };
        Ok(Self {
            skinner,
            owners: HashMap::new(),
            sources: HashMap::new(),
        })
    }
    pub(super) fn pose_signature(&self, owner: NodeId) -> Result<Option<Vec<[u32; 16]>>, String> {
        let Some(owner) = self.owners.get(&owner) else {
            return Ok(None);
        };
        let mut playback = owner.playback.clone();
        playback.advance_with(0., |_, frame| {
            Ok(Some(
                frame
                    .skin_matrices
                    .iter()
                    .map(|matrix| matrix.to_cols_array().map(f32::to_bits))
                    .collect(),
            ))
        })
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
    ) -> Result<Option<(glam::Vec3, glam::Vec3)>, String> {
        let Some(state) = self.owners.get(&owner) else {
            return Ok(None);
        };
        let Some(source) = &state.lod else {
            return Ok(None);
        };
        let mut playback = state.playback.clone();
        let frame = playback.advance_with(0., |_, frame| Ok(frame.clone()))?;
        let posed = source
            .prepare(&frame.skin_matrices, world)
            .map_err(|e| e.to_string())?;
        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
        for position in posed.positions() {
            let p = glam::Vec3::from_array(*position);
            min = min.min(p);
            max = max.max(p);
        }
        Ok(Some((min, max)))
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
    pub(super) fn geometries_for_view(
        &self,
        owner: NodeId,
        view: u8,
    ) -> Option<impl Iterator<Item = &SceneGeometry>> {
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
                    if index == 0 {
                        selected.map_or_else(|| primitive.geometry(), LodGeometry::geometry)
                    } else {
                        primitive.geometry()
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
    ) -> Result<(), String> {
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
        let frame = state
            .playback
            .advance_with(0., |_, frame| Ok(frame.clone()))?;
        // Exact palette/world equality permits reuse across cameras and paused frames.
        // A failed replacement leaves the accepted certificate and view history intact.
        let candidate = if state.prepared_lod.as_ref().is_none_or(|posed| {
            posed.model() != world || posed.joints() != frame.skin_matrices.as_slice()
        }) {
            Some(
                source
                    .prepare(&frame.skin_matrices, world)
                    .map_err(|e| e.to_string())?,
            )
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
            let geometry = match (&self.skinner, &state.primitives[0]) {
                (Some(skinner), Primitive::Skin { instance, .. }) => LodGeometry::Gpu(
                    skinner
                        .create_lod_level(instance, source, level, live, budget)
                        .map_err(|e| e.to_string())?,
                ),
                _ => {
                    let local = source
                        .prepare(&frame.skin_matrices, glam::Mat4::IDENTITY)
                        .map_err(|e| e.to_string())?;
                    let mesh = local
                        .posed_scene_mesh(level, state.model.primitives[0].color)
                        .map_err(|e| e.to_string())?
                        .with_material_coordinates(
                            source
                                .mesh()
                                .vertices()
                                .iter()
                                .map(|v| v.position)
                                .collect(),
                        )
                        .map_err(|e| e.to_string())?;
                    admit(live, SceneRenderer::mesh_allocation_bytes(&mesh), budget)?;
                    LodGeometry::Cpu(
                        renderer
                            .upload_mesh(device, &mesh)
                            .map_err(|e| e.to_string())?,
                    )
                }
            };
            state.lod_levels.insert(level, geometry);
        }
        if let Some(candidate) = candidate {
            state.prepared_lod = Some(candidate);
        }
        state.lod_history.insert(view, level);
        Ok(())
    }
    /// One logical fixed-step clock drives all views; each owner advances at most
    /// eight pending steps per publication. Failures keep its last good frame.
    pub(super) fn synchronize(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        requests: Vec<Request>,
        ticks: u64,
        other_live: u64,
        budget: u64,
    ) -> Vec<(NodeId, String)> {
        let active: HashSet<_> = requests.iter().map(|r| r.owner).collect();
        self.owners.retain(|owner, _| active.contains(owner));
        self.sources
            .retain(|_, entry| entry.source.strong_count() > 0 && entry.model.strong_count() > 0);
        let mut errors = Vec::new();
        for request in requests {
            if let Err(error) = self.update(
                renderer,
                device,
                queue,
                request.owner,
                request.model,
                request.settings,
                request.lod,
                request.textures,
                request.texture_storage,
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
        lod: Option<Arc<voxy_render::SkinnedLodMesh>>,
        textures: Vec<Option<Arc<voxy_render::SceneTexture>>>,
        texture_storage: Vec<Arc<voxy_render::SceneTexture>>,
        ticks: u64,
        other_live: u64,
        budget: u64,
    ) -> Result<(), String> {
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
        let switch_clip = current.is_some_and(|old| old.settings.clip != settings.clip);
        let reset_clock = replace || switch_clip;
        let mut playback = if reset_clock {
            ModelPlayback::new(model.clone(), settings)?
        } else {
            let mut playback = current.unwrap().playback.clone();
            playback.set_speed(settings.speed)?;
            playback.set_root_motion_joint(settings.root_motion_joint)?;
            playback
        };
        let old_ticks = if reset_clock {
            ticks
        } else {
            current.unwrap().ticks
        };
        let steps = ticks.saturating_sub(old_ticks).min(8);
        if !reset_clock
            && !lod_changed
            && (steps == 0 || settings.speed == 0.0 || settings.clip.is_none())
        {
            let state = self.owners.get_mut(&owner).unwrap();
            state.textures = textures;
            state.texture_storage = texture_storage;
            state.playback = playback;
            state.settings = settings;
            state.ticks = old_ticks + steps;
            return Ok(());
        }
        let mut frame = playback.advance_with(0., |_, frame| Ok(frame.clone()))?;
        for _ in 0..steps {
            frame = playback.advance_with(1. / 60., |_, frame| Ok(frame.clone()))?;
        }
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
            Some(
                source
                    .prepare(&frame.skin_matrices, glam::Mat4::IDENTITY)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let mut live = other_live.saturating_add(self.allocation_bytes());
        let mut staged = Vec::with_capacity(model.primitives.len());
        for (index, primitive) in model.primitives.iter().enumerate() {
            if !replace
                && self.skinner.is_some()
                && matches!(primitive.geometry, ModelGeometry::Skinned(_))
            {
                staged.push(None);
                continue;
            }
            let result = match (&primitive.geometry, &self.skinner) {
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
                        let source = skinner
                            .upload_source(Arc::new(mesh.clone()), live, budget)
                            .map_err(|e| e.to_string())?;
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
                    let instance = skinner
                        .create_instance(
                            renderer,
                            source.clone(),
                            &frame.skin_matrices,
                            primitive.color,
                            live,
                            budget,
                        )
                        .map_err(|e| e.to_string())?;
                    live += instance.allocation_bytes();
                    Primitive::Skin { instance, source }
                }
                (geometry, _) => {
                    let mesh = match geometry {
                        ModelGeometry::Static(mesh) => rigid_mesh(&model, &frame.pose, mesh)?,
                        ModelGeometry::Skinned(mesh) => mesh
                            .posed_scene_mesh(
                                &frame.skin_matrices,
                                glam::Mat4::IDENTITY,
                                primitive.color,
                            )
                            .map_err(|e| e.to_string())?,
                    };
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
                    let mesh = mesh
                        .with_material_coordinates(coordinates)
                        .map_err(|e| e.to_string())?;
                    let bytes = SceneRenderer::mesh_allocation_bytes(&mesh);
                    admit(live, bytes, budget)?;
                    let geometry = renderer
                        .upload_mesh(device, &mesh)
                        .map_err(|e| e.to_string())?;
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
                    prepared.push(
                        skinner
                            .prepare_pose(instance, &frame.skin_matrices)
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
            for pose in &prepared {
                skinner
                    .encode_prepared_pose(queue, &mut encoder, pose)
                    .map_err(|e| e.to_string())?;
            }
        }
        queue.submit([encoder.finish()]);
        if replace {
            self.owners.insert(
                owner,
                Owner {
                    model,
                    settings,
                    playback,
                    ticks: old_ticks + steps,
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
            state.playback = playback;
            state.settings = settings;
            state.ticks = old_ticks + steps;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
