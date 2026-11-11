//! GPU resources shared across instances of a packaged model.
use super::{GEOMETRY_BYTES, ModelPart, PartGeometry, ProjectModel, SOURCE_BYTES, error};
use glam::Mat4;
use std::{collections::BTreeMap, sync::Arc};
use voxy_assets::{ImportInputs, ResourcePackage, SourcePath};
use voxy_model::{
    ImportedGltf, ModelAsset, ModelGeometry, ModelLimits, ModelTexture, import_project_gltf,
};
use voxy_render::{
    ObjAsset, ObjLimits, SceneMesh, SceneRenderer, SceneSkinner, SceneTexture, TextureSampling,
};
use wasm_bindgen::JsValue;

fn decode_gltf(
    package: &ResourcePackage,
    source: &SourcePath,
) -> Result<(Arc<[u8]>, ImportedGltf), JsValue> {
    let mut inputs = ImportInputs::new(34, 32 * 1024 * 1024);
    let mut read = |source: &SourcePath| {
        inputs
            .read(source.observation_id(), |id, limit| {
                package.read(id, limit.min(SOURCE_BYTES))
            })
            .map_err(|failure| format!("{failure:?}"))
    };
    let bytes = read(source).map_err(error)?;
    let imported = import_project_gltf::<Box<dyn std::error::Error>>(&bytes.bytes, |uri| {
        Ok(read(&source.sibling(uri)?)?.bytes.to_vec())
    })
    .map_err(error)?;
    Ok((bytes.bytes, imported))
}

fn parse_animated(bytes: &[u8], buffers: &[Vec<u8>]) -> Result<ModelAsset, JsValue> {
    let buffers: Vec<_> = buffers.iter().map(Vec::as_slice).collect();
    let model = ModelAsset::parse_animated(
        bytes,
        &buffers,
        ModelLimits {
            bytes: 32 * 1024 * 1024,
            vertices: 65_536,
            indices: 196_608,
            keys: 65_536,
        },
    )
    .map_err(error)?;
    if model.primitives.len() > 128 {
        return Err(error("animated primitive budget exceeded"));
    }
    Ok(model)
}

pub(super) fn load_animation_source(
    package: &ResourcePackage,
    source: &SourcePath,
) -> Result<Arc<ModelAsset>, JsValue> {
    let (bytes, imported) = decode_gltf(package, source)?;
    let ImportedGltf::Animated { buffers, .. } = imported else {
        return Err(error("retarget source must contain an animation rig"));
    };
    parse_animated(&bytes, &buffers).map(Arc::new)
}

pub(super) fn load_model(
    package: &ResourcePackage,
    source: &SourcePath,
    renderer: &SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    remaining_geometry: &mut u64,
    skinner: &mut Option<SceneSkinner>,
) -> Result<ProjectModel, JsValue> {
    let extension = std::path::Path::new(source.as_str())
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let upload = |mesh: &SceneMesh, remaining: &mut u64| {
        *remaining = remaining
            .checked_sub(SceneRenderer::mesh_allocation_bytes(mesh))
            .ok_or_else(|| error("browser project geometry budget exceeded"))?;
        renderer
            .upload_mesh(device, mesh)
            .map(Arc::new)
            .map_err(error)
    };
    if extension == "obj" {
        let limits = ObjLimits::default();
        let bytes = package
            .read(&source.observation_id(), limits.source_bytes)
            .map_err(error)?;
        let obj =
            ObjAsset::parse(std::str::from_utf8(&bytes).map_err(error)?, limits).map_err(error)?;
        return Ok(ProjectModel {
            parts: vec![ModelPart {
                matrix: Mat4::IDENTITY,
                geometry: PartGeometry::Static(upload(&obj.mesh, remaining_geometry)?),
                texture: None,
            }],
            animation: None,
        });
    }
    if extension != "gltf" && extension != "glb" {
        return Err(error(format!(
            "unsupported browser project model: {}",
            source.as_str()
        )));
    }
    let (bytes, imported) = decode_gltf(package, source)?;
    let (nodes, images) = match imported {
        ImportedGltf::Static { nodes, images, .. } => (nodes, images),
        ImportedGltf::Animated { buffers, images } => {
            let model = parse_animated(&bytes, &buffers)?;
            let mut textures = ProjectTextures::new(renderer, device, queue, &images);
            let mut parts = Vec::with_capacity(model.primitives.len());
            for (index, primitive) in model.primitives.iter().enumerate() {
                let morph = model.morph_source(index).map_err(error)?;
                let geometry = match (&primitive.geometry, morph) {
                    (ModelGeometry::Static(mesh), None) => {
                        PartGeometry::Static(upload(mesh, remaining_geometry)?)
                    }
                    (geometry, morph) => {
                        if skinner.is_none() {
                            *skinner = Some(SceneSkinner::new(renderer).map_err(error)?);
                        }
                        let skinner = skinner
                            .as_ref()
                            .ok_or_else(|| error("missing project skinner"))?;
                        let live = GEOMETRY_BYTES - *remaining_geometry;
                        let source = match (geometry, morph) {
                            (_, Some(morph)) => {
                                skinner.upload_morph_source(Arc::new(morph), live, GEOMETRY_BYTES)
                            }
                            (ModelGeometry::Skinned(mesh), None) => {
                                skinner.upload_source(Arc::new(mesh.clone()), live, GEOMETRY_BYTES)
                            }
                            (ModelGeometry::Static(_), None) => unreachable!(),
                        }
                        .map_err(error)?;
                        *remaining_geometry -= source.allocation_bytes();
                        PartGeometry::Skin {
                            source,
                            color: primitive.color,
                        }
                    }
                };
                parts.push(ModelPart {
                    matrix: Mat4::IDENTITY,
                    geometry,
                    texture: textures.binding(primitive.base_color_texture)?,
                });
            }
            return Ok(ProjectModel {
                parts,
                animation: Some(Arc::new(model)),
            });
        }
    };
    let mut worlds: Vec<Mat4> = Vec::with_capacity(nodes.len());
    let mut parts = Vec::new();
    let mut geometry = BTreeMap::new();
    let mut textures = ProjectTextures::new(renderer, device, queue, &images);
    for node in nodes {
        let local = node.local.matrix().map_err(error)?;
        let matrix = node.parent.map_or(local, |parent| worlds[parent] * local);
        if !matrix.is_finite() {
            return Err(error("glTF world transform overflow"));
        }
        worlds.push(matrix);
        let Some(mesh) = node.mesh else { continue };
        let key = node
            .geometry_key
            .ok_or_else(|| error("missing glTF primitive identity"))?;
        let geometry = match geometry.entry(key) {
            std::collections::btree_map::Entry::Occupied(entry) => Arc::clone(entry.get()),
            std::collections::btree_map::Entry::Vacant(entry) => {
                Arc::clone(entry.insert(upload(&mesh, remaining_geometry)?))
            }
        };
        let texture = textures.binding(node.image.map(|image| ModelTexture {
            image,
            sampling: node.sampling,
            use_mips: node.use_mips,
        }))?;
        parts.push(ModelPart {
            matrix,
            geometry: PartGeometry::Static(geometry),
            texture,
        });
    }
    Ok(ProjectModel {
        parts,
        animation: None,
    })
}

struct ProjectTextures<'a> {
    renderer: &'a SceneRenderer,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    images: &'a [voxy_render::ImageAsset],
    storage: BTreeMap<usize, SceneTexture>,
    bindings: Vec<((usize, TextureSampling, bool), Arc<SceneTexture>)>,
}

impl<'a> ProjectTextures<'a> {
    fn new(
        renderer: &'a SceneRenderer,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        images: &'a [voxy_render::ImageAsset],
    ) -> Self {
        Self {
            renderer,
            device,
            queue,
            images,
            storage: BTreeMap::new(),
            bindings: Vec::new(),
        }
    }

    fn binding(
        &mut self,
        descriptor: Option<ModelTexture>,
    ) -> Result<Option<Arc<SceneTexture>>, JsValue> {
        let Some(descriptor) = descriptor else {
            return Ok(None);
        };
        let index = descriptor.image;
        let key = (index, descriptor.sampling, descriptor.use_mips);
        if let Some((_, texture)) = self.bindings.iter().find(|(binding, _)| *binding == key) {
            return Ok(Some(Arc::clone(texture)));
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = self.storage.entry(index) {
            let image = self
                .images
                .get(index)
                .ok_or_else(|| error("missing model texture"))?;
            entry.insert(
                self.renderer
                    .upload_image_mips(
                        self.device,
                        self.queue,
                        &image.mip_chain(),
                        TextureSampling::default(),
                    )
                    .map_err(error)?,
            );
        }
        let image = &self.storage[&index];
        let levels = if descriptor.use_mips {
            image.texture().mip_level_count()
        } else {
            1
        };
        let texture = Arc::new(
            self.renderer
                .texture_binding(self.device, image, descriptor.sampling, levels)
                .map_err(error)?,
        );
        self.bindings.push((key, Arc::clone(&texture)));
        Ok(Some(texture))
    }
}
