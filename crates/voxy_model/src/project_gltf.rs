//! Shared project glTF decoding for filesystem and package resource providers.
use glam::{Mat4, Vec3};
use voxy_render::{
    ImageAsset, ImageLimits, SceneMesh, SceneVertex, TextureFilter, TextureSampling, TextureWrap,
};
use voxy_scene::Transform;

#[derive(Clone, Debug)]
pub struct ImportedNode {
    pub name: String,
    pub parent: Option<usize>,
    pub local: Transform,
    pub mesh: Option<SceneMesh>,
    pub image: Option<usize>,
    pub sampling: TextureSampling,
    pub use_mips: bool,
    pub geometry_key: Option<(usize, usize)>,
}
/// A static model or the source data required by the animated importer.
#[derive(Debug)]
pub enum ImportedGltf {
    Static {
        mesh: SceneMesh,
        nodes: Vec<ImportedNode>,
        images: Vec<ImageAsset>,
    },
    Animated {
        buffers: Vec<Vec<u8>>,
        images: Vec<ImageAsset>,
    },
}

/// Decodes a project model with a caller-owned, bounded external URI reader.
///
/// # Errors
/// Rejects malformed glTF, unsupported features, and inputs exceeding import budgets.
pub fn import_project_gltf<E>(
    bytes: &[u8],
    mut read_uri: impl FnMut(&str) -> Result<Vec<u8>, E>,
) -> Result<ImportedGltf, E>
where
    E: From<&'static str>
        + From<gltf::Error>
        + From<crate::ModelError>
        + From<voxy_render::SceneError>
        + From<voxy_render::ImageAssetError>
        + From<std::num::TryFromIntError>,
{
    let gltf = gltf::Gltf::from_slice(bytes)?;
    if gltf.extensions_required().next().is_some() {
        return Err("required glTF extensions unsupported".into());
    }
    if gltf.nodes().count() > 128 || gltf.buffers().count() > 16 || gltf.images().count() > 16 {
        return Err("glTF object budget exceeded".into());
    }
    let mut buffers = Vec::new();
    for buffer in gltf.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => gltf.blob.clone().ok_or("GLB missing BIN chunk")?,
            gltf::buffer::Source::Uri(uri) => read_uri(uri)?,
        };
        if data.len() < buffer.length() {
            return Err("truncated glTF buffer".into());
        }
        buffers.push(data);
    }
    // Reader utilities assume valid accessor ranges; check every view/accessor first.
    for view in gltf.views() {
        if view
            .offset()
            .checked_add(view.length())
            .is_none_or(|end| end > buffers[view.buffer().index()].len())
        {
            return Err("glTF view outside buffer".into());
        }
    }
    for accessor in gltf.accessors() {
        if accessor.sparse().is_some() {
            return Err("sparse accessors unsupported".into());
        }
        let view = accessor.view().ok_or("accessor missing view")?;
        let stride = view.stride().unwrap_or(accessor.size());
        let end = accessor
            .count()
            .checked_sub(1)
            .and_then(|n| n.checked_mul(stride))
            .and_then(|n| n.checked_add(accessor.offset()))
            .and_then(|n| n.checked_add(accessor.size()));
        if end.is_none_or(|end| end > view.length()) {
            return Err("glTF accessor outside view".into());
        }
    }
    let images = if gltf
        .buffers()
        .all(|buffer| matches!(buffer.source(), gltf::buffer::Source::Bin))
        && gltf
            .images()
            .all(|image| matches!(image.source(), gltf::image::Source::View { .. }))
    {
        crate::ModelAsset::decode_embedded_images(
            bytes,
            ImageLimits {
                source_bytes: 16 * 1024 * 1024,
                dimension: 2048,
                pixel_bytes: 32 * 1024 * 1024,
            },
            16,
        )?
    } else {
        let mut images = Vec::new();
        for image in gltf.images() {
            let data = match image.source() {
                gltf::image::Source::View { view, .. } => buffers[view.buffer().index()]
                    [view.offset()..view.offset() + view.length()]
                    .to_vec(),
                gltf::image::Source::Uri { uri, .. } => read_uri(uri)?,
            };
            images.push(ImageAsset::decode(
                &data,
                ImageLimits {
                    source_bytes: 16 * 1024 * 1024,
                    dimension: 2048,
                    pixel_bytes: 32 * 1024 * 1024
                        - u64::try_from(
                            images
                                .iter()
                                .map(|image: &ImageAsset| image.rgba().len())
                                .sum::<usize>(),
                        )?,
                },
            )?);
        }
        images
    };
    if images.iter().map(|image| image.rgba().len()).sum::<usize>() > 32 * 1024 * 1024 {
        return Err("decoded image budget exceeded".into());
    }
    if gltf.animations().next().is_some()
        || gltf.skins().next().is_some()
        || gltf.meshes().any(|mesh| {
            mesh.primitives()
                .any(|p| p.morph_targets().next().is_some())
        })
    {
        return Ok(ImportedGltf::Animated { buffers, images });
    }
    let raw: Vec<_> = gltf.nodes().collect();
    let mut parents = vec![None; raw.len()];
    let mut children_list = vec![Vec::new(); raw.len()];
    let mut indegree = vec![0_usize; raw.len()];
    for node in &raw {
        let parent_idx = node.index();
        for child in node.children() {
            let child_idx = child.index();
            if parents[child_idx].replace(parent_idx).is_some() {
                return Err("glTF node has multiple parents".into());
            }
            children_list[parent_idx].push(child_idx);
            indegree[child_idx] += 1;
        }
    }
    let mut ready = std::collections::BinaryHeap::with_capacity(raw.len());
    for (index, &deg) in indegree.iter().enumerate() {
        if deg == 0 {
            ready.push(std::cmp::Reverse(index));
        }
    }
    let mut order = Vec::with_capacity(raw.len());
    while let Some(std::cmp::Reverse(index)) = ready.pop() {
        order.push(index);
        for &child in &children_list[index] {
            indegree[child] -= 1;
            if indegree[child] == 0 {
                ready.push(std::cmp::Reverse(child));
            }
        }
    }
    if order.len() != raw.len() {
        return Err("cyclic glTF hierarchy".into());
    }
    let mut remap = vec![0; raw.len()];
    for (index, &raw_index) in order.iter().enumerate() {
        remap[raw_index] = index;
    }
    let mut nodes: Vec<ImportedNode> = Vec::new();
    let mut primitive_nodes = Vec::new();
    let mut worlds: Vec<Mat4> = Vec::new();
    let mut all_vertices = Vec::new();
    let mut all_normals: Option<Vec<[f32; 3]>> = Some(Vec::new());
    let mut all_indices = Vec::new();
    for raw_index in order {
        let node = &raw[raw_index];
        let (translation, rotation, scale) = node.transform().decomposed();
        let local = Transform {
            translation: Vec3::from_array(translation),
            rotation: glam::Quat::from_array(rotation),
            scale: Vec3::from_array(scale),
        };
        let matrix =
            Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation);
        if !matrix.is_finite()
            || !matrix.abs_diff_eq(Mat4::from_cols_array_2d(&node.transform().matrix()), 1e-4)
        {
            return Err("glTF transform must be finite TRS".into());
        }
        let parent = parents[raw_index].map(|parent| remap[parent]);
        let world = parent.map_or(matrix, |parent| worlds[parent] * matrix);
        worlds.push(world);
        let mut imported = ImportedNode {
            name: node.name().unwrap_or("glTF node").to_owned(),
            parent,
            local,
            mesh: None,
            image: None,
            sampling: TextureSampling::default(),
            use_mips: false,
            geometry_key: None,
        };
        if let Some(mesh) = node.mesh() {
            let count = mesh.primitives().count();
            if count == 0 {
                return Err("empty glTF mesh".into());
            }
            if raw.len() + primitive_nodes.len() + if count > 1 { count } else { 0 } > 128 {
                return Err("glTF primitive node budget exceeded".into());
            }
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles
                    || primitive.morph_targets().next().is_some()
                {
                    return Err("triangle primitives without morphs required".into());
                }
                let mut part = imported.clone();
                part.geometry_key = Some((mesh.index(), primitive.index()));
                let material = primitive.material();
                let pbr = material.pbr_metallic_roughness();
                if let Some(texture) = pbr.base_color_texture() {
                    if texture.tex_coord() != 0 {
                        return Err("base color requires TEXCOORD_0".into());
                    }
                    part.image = Some(texture.texture().source().index());
                    let sampler = texture.texture().sampler();
                    let wrap = |mode| match mode {
                        gltf::texture::WrappingMode::ClampToEdge => TextureWrap::Clamp,
                        gltf::texture::WrappingMode::Repeat => TextureWrap::Repeat,
                        gltf::texture::WrappingMode::MirroredRepeat => TextureWrap::Mirror,
                    };
                    part.use_mips = !matches!(
                        sampler.min_filter(),
                        Some(gltf::texture::MinFilter::Nearest | gltf::texture::MinFilter::Linear)
                    );
                    part.sampling = TextureSampling {
                        wrap_u: wrap(sampler.wrap_s()),
                        wrap_v: wrap(sampler.wrap_t()),
                        min_filter: match sampler.min_filter() {
                            Some(
                                gltf::texture::MinFilter::Nearest
                                | gltf::texture::MinFilter::NearestMipmapNearest
                                | gltf::texture::MinFilter::NearestMipmapLinear,
                            ) => TextureFilter::Nearest,
                            _ => TextureFilter::Linear,
                        },
                        mag_filter: if sampler.mag_filter()
                            == Some(gltf::texture::MagFilter::Nearest)
                        {
                            TextureFilter::Nearest
                        } else {
                            TextureFilter::Linear
                        },
                        mipmap_filter: Some(match sampler.min_filter() {
                            Some(
                                gltf::texture::MinFilter::NearestMipmapNearest
                                | gltf::texture::MinFilter::LinearMipmapNearest,
                            ) => TextureFilter::Nearest,
                            _ => TextureFilter::Linear,
                        }),
                        anisotropy: 1,
                    };
                }
                let position_count = primitive
                    .get(&gltf::Semantic::Positions)
                    .ok_or("missing positions")?
                    .count();
                if position_count > 65_536
                    || all_vertices.len() + position_count > 65_536
                    || primitive
                        .indices()
                        .is_some_and(|accessor| accessor.count() > 196_608)
                {
                    return Err("glTF geometry budget exceeded".into());
                }
                let reader = primitive.reader(|buffer| Some(buffers[buffer.index()].as_slice()));
                let positions: Vec<_> = reader
                    .read_positions()
                    .ok_or("missing glTF positions")?
                    .collect();
                if positions.len() > 65_536
                    || all_vertices
                        .len()
                        .checked_add(positions.len())
                        .is_none_or(|count| count > 65_536)
                {
                    return Err("glTF vertex budget exceeded".into());
                }
                let uvs: Vec<_> = reader.read_tex_coords(0).map_or_else(
                    || vec![[0.; 2]; positions.len()],
                    |uv| uv.into_f32().collect(),
                );
                if uvs.len() != positions.len()
                    || (part.image.is_some() && reader.read_tex_coords(0).is_none())
                {
                    return Err("glTF UV count mismatch or missing texture UV".into());
                }
                let colors: Vec<_> = reader.read_colors(0).map_or_else(
                    || vec![[1.; 4]; positions.len()],
                    |colors| colors.into_rgba_f32().collect(),
                );
                if colors.len() != positions.len() {
                    return Err("glTF color count mismatch".into());
                }
                let factor = pbr.base_color_factor();
                let vertices: Vec<_> = positions
                    .into_iter()
                    .zip(uvs)
                    .zip(colors)
                    .map(|((position, uv), color)| SceneVertex {
                        position,
                        uv,
                        color: std::array::from_fn(|i| color[i] * factor[i]),
                    })
                    .collect();
                let indices: Vec<_> = reader.read_indices().map_or_else(
                    || (0..vertices.len() as u32).collect(),
                    |indices| indices.into_u32().collect(),
                );
                if all_indices
                    .len()
                    .checked_add(indices.len())
                    .is_none_or(|count| count > 196_608)
                {
                    return Err("glTF index budget exceeded".into());
                }
                let authored_normals: Option<Vec<[f32; 3]>> =
                    reader.read_normals().map(Iterator::collect);
                let mut part_mesh = SceneMesh::new(vertices.clone(), indices.clone())?;
                if let Some(normals) = authored_normals.clone() {
                    part_mesh = part_mesh.with_normals(normals)?;
                }
                part.mesh = Some(part_mesh);
                let base = all_vertices.len() as u32;
                all_vertices.extend(vertices.into_iter().map(|vertex| {
                    SceneVertex {
                        position: world
                            .transform_point3(Vec3::from_array(vertex.position))
                            .to_array(),
                        ..vertex
                    }
                }));
                if let (Some(accum), Some(normals)) = (&mut all_normals, authored_normals) {
                    let normal_matrix = glam::Mat3::from_mat4(world).inverse().transpose();
                    if normal_matrix.is_finite() {
                        accum.extend(normals.into_iter().map(|n| {
                            (normal_matrix * Vec3::from_array(n))
                                .normalize_or_zero()
                                .to_array()
                        }));
                    } else {
                        all_normals = None;
                    }
                } else {
                    all_normals = None;
                }
                all_indices.extend(indices.into_iter().map(|index| index + base));
                if count == 1 {
                    imported = part;
                } else {
                    part.parent = Some(remap[raw_index]);
                    part.local = Transform::default();
                    part.name = format!("{}/primitive-{}", imported.name, primitive.index());
                    primitive_nodes.push(part);
                }
            }
        }
        nodes.push(imported);
    }
    nodes.extend(primitive_nodes);
    let mut combined_mesh = SceneMesh::new(all_vertices, all_indices)?;
    if let Some(normals) = all_normals
        && normals.len() == combined_mesh.vertices().len()
    {
        combined_mesh = combined_mesh.with_normals(normals)?;
    }
    Ok(ImportedGltf::Static {
        mesh: combined_mesh,
        nodes,
        images,
    })
}
