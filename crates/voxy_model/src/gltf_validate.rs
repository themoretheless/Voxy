//! glTF buffer/accessor bounds, scene mesh selection, and topological node ordering.

use super::{ModelAsset, ModelError, fail};

impl ModelAsset {
    pub(super) fn validate_gltf_buffers_and_accessors<'a>(
        gltf: &'a gltf::Gltf,
        buffers: &'a [&'a [u8]],
    ) -> Result<Vec<&'a [u8]>, ModelError> {
        let mut data = Vec::with_capacity(gltf.buffers().len());
        for buffer in gltf.buffers() {
            let bytes = match buffer.source() {
                gltf::buffer::Source::Bin => gltf.blob.as_deref(),
                gltf::buffer::Source::Uri(_) => buffers.get(buffer.index()).copied(),
            }
            .ok_or_else(|| fail("missing explicit buffer"))?;
            if bytes.len() < buffer.length() {
                return Err(fail("truncated buffer"));
            }
            data.push(&bytes[..buffer.length()]);
        }
        for accessor in gltf.accessors() {
            if accessor.sparse().is_some() {
                return Err(fail("sparse accessors unsupported"));
            }
            let view = accessor
                .view()
                .ok_or_else(|| fail("accessor without buffer view"))?;
            let stride = view.stride().unwrap_or(accessor.size());
            let end = accessor
                .count()
                .saturating_sub(1)
                .checked_mul(stride)
                .and_then(|n| n.checked_add(accessor.offset()))
                .and_then(|n| {
                    n.checked_add(if accessor.count() == 0 {
                        0
                    } else {
                        accessor.size()
                    })
                });
            if stride < accessor.size()
                || end.is_none_or(|n| n > view.length())
                || view
                    .offset()
                    .checked_add(view.length())
                    .is_none_or(|n| n > data[view.buffer().index()].len())
            {
                return Err(fail("accessor outside buffer bounds"));
            }
        }
        Ok(data)
    }

    pub(super) fn select_active_mesh_nodes<'a>(
        gltf: &'a gltf::Gltf,
        nodes: &'a [gltf::Node<'a>],
    ) -> Result<Vec<&'a gltf::Node<'a>>, ModelError> {
        // A model loads the explicit default scene (or the sole scene). Do not
        // merge unrelated alternate scenes into one visible model.
        let selected = gltf.default_scene().or_else(|| {
            (gltf.scenes().count() == 1)
                .then(|| gltf.scenes().next())
                .flatten()
        });
        if selected.is_none() && gltf.scenes().count() > 1 {
            return Err(fail("multiple scenes require an explicit default scene"));
        }
        let mut active = std::collections::HashSet::with_capacity(nodes.len());
        if let Some(scene) = selected {
            let mut pending: Vec<_> = scene.nodes().collect();
            while let Some(node) = pending.pop() {
                if active.insert(node.index()) {
                    pending.extend(node.children());
                }
            }
        } else {
            active.extend(nodes.iter().map(gltf::Node::index));
        }
        let meshes: Vec<_> = nodes
            .iter()
            .filter(|node| node.mesh().is_some() && active.contains(&node.index()))
            .collect();
        if meshes.is_empty() {
            return Err(fail("selected scene contains no mesh instance"));
        }
        Ok(meshes)
    }

    pub(super) fn topological_node_order(
        nodes: &[gltf::Node<'_>],
    ) -> Result<(Vec<Option<usize>>, Vec<usize>, Vec<usize>), ModelError> {
        let mut parents = vec![None; nodes.len()];
        for node in nodes {
            for child in node.children() {
                if parents[child.index()].replace(node.index()).is_some() {
                    return Err(fail("node has multiple parents"));
                }
            }
        }
        let mut order = Vec::with_capacity(nodes.len());
        let mut visited = vec![false; nodes.len()];
        while order.len() < nodes.len() {
            let before = order.len();
            for (index, parent) in parents.iter().enumerate() {
                if !visited[index] && parent.is_none_or(|p| visited[p]) {
                    visited[index] = true;
                    order.push(index);
                }
            }
            if before == order.len() {
                return Err(fail("cyclic node hierarchy"));
            }
        }
        let mut mapping = vec![0; nodes.len()];
        for (joint, &node) in order.iter().enumerate() {
            mapping[node] = joint;
        }
        Ok((parents, order, mapping))
    }
}
