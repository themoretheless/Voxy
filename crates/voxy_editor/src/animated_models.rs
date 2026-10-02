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
            .sum()
    }
    pub(super) fn geometries(&self, owner: NodeId) -> Option<impl Iterator<Item = &SceneGeometry>> {
        self.owners
            .get(&owner)
            .map(|state| state.primitives.iter().map(Primitive::geometry))
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
        ticks: u64,
        other_live: u64,
        budget: u64,
    ) -> Result<(), String> {
        let current = self.owners.get(&owner);
        let replace = current.is_none_or(|old| !Arc::ptr_eq(&old.model, &model));
        let switch_clip = current.is_some_and(|old| old.settings.clip != settings.clip);
        let reset_clock = replace || switch_clip;
        let mut playback = if reset_clock {
            ModelPlayback::new(model.clone(), settings)?
        } else {
            let mut playback = current.unwrap().playback.clone();
            playback.set_speed(settings.speed)?;
            playback
        };
        let old_ticks = if reset_clock {
            ticks
        } else {
            current.unwrap().ticks
        };
        let steps = ticks.saturating_sub(old_ticks).min(8);
        if !reset_clock && (steps == 0 || settings.speed == 0.0 || settings.clip.is_none()) {
            let state = self.owners.get_mut(&owner).unwrap();
            state.playback = playback;
            state.settings = settings;
            state.ticks = old_ticks + steps;
            return Ok(());
        }
        let mut frame = playback.advance_with(0., |_, frame| Ok(frame.clone()))?;
        for _ in 0..steps {
            frame = playback.advance_with(1. / 60., |_, frame| Ok(frame.clone()))?;
        }
        // Existing streams remain untouched until every primitive is preflighted.
        if !replace && self.skinner.is_some() {
            for primitive in &current.unwrap().primitives {
                if let Primitive::Skin { instance, .. } = primitive {
                    self.skinner
                        .as_ref()
                        .unwrap()
                        .validate_pose(instance, &frame.skin_matrices)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
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
            for (i, staged) in staged.iter().enumerate() {
                let primitive = staged
                    .as_ref()
                    .or_else(|| primitives.map(|p| &p[i]))
                    .ok_or("missing animated primitive")?;
                if let Primitive::Skin { instance, .. } = primitive {
                    skinner
                        .encode_pose(queue, &mut encoder, instance, &frame.skin_matrices)
                        .map_err(|e| e.to_string())?;
                }
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
                },
            );
        } else {
            let state = self.owners.get_mut(&owner).unwrap();
            for (i, primitive) in staged.into_iter().enumerate() {
                if let Some(primitive) = primitive {
                    state.primitives[i] = primitive;
                }
            }
            state.playback = playback;
            state.settings = settings;
            state.ticks = old_ticks + steps;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
