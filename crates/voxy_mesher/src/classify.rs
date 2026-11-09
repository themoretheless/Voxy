//! Directed face classification and cached per-palette-slot visibility properties.

use voxy_world::{BlockRegistry, BlockStateId, MaterialId, Occlusion, RenderKind, Sample};

use crate::types::{FaceDir, RenderLayer};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceDecision {
    Skip,
    Emit {
        material: MaterialId,
        layer: RenderLayer,
    },
}

#[inline]
#[must_use]
pub fn classify_directed_face(
    source: BlockStateId,
    neighbor: &Sample<BlockStateId>,
    face: FaceDir,
    registry: &BlockRegistry,
) -> FaceDecision {
    let Some(source_def) = registry.get(source) else {
        return FaceDecision::Skip;
    };
    let layer = match source_def.render {
        RenderKind::Invisible => return FaceDecision::Skip,
        RenderKind::Opaque => RenderLayer::Opaque,
        RenderKind::Cutout => RenderLayer::Cutout,
        RenderKind::Translucent => RenderLayer::Translucent,
    };
    if let Sample::Loaded(neighbor_id) = *neighbor
        && let Some(neighbor_def) = registry.get(neighbor_id)
    {
        if neighbor_def.occlusion == Occlusion::FullCube {
            return FaceDecision::Skip;
        }
        if source_def.translucent_interface_group.is_some()
            && source_def.translucent_interface_group == neighbor_def.translucent_interface_group
        {
            return FaceDecision::Skip;
        }
    }
    FaceDecision::Emit {
        material: source_def.face_materials[face.material_index()],
        layer,
    }
}

/// Face-classification data for one distinct block of a chunk.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SlotClass {
    /// `None` for unregistered or invisible blocks, which emit no faces.
    pub(crate) layer: Option<RenderLayer>,
    /// Registered at all; unknown neighbors never occlude.
    pub(crate) known: bool,
    pub(crate) full_cube: bool,
    pub(crate) group: Option<voxy_world::InterfaceGroupId>,
    pub(crate) materials: [MaterialId; 6],
}

impl SlotClass {
    #[inline]
    pub(crate) fn classify(registry: &BlockRegistry, block: BlockStateId) -> Self {
        let Some(definition) = registry.get(block) else {
            return Self {
                layer: None,
                known: false,
                full_cube: false,
                group: None,
                materials: [MaterialId(0); 6],
            };
        };
        let layer = match definition.render {
            RenderKind::Invisible => None,
            RenderKind::Opaque => Some(RenderLayer::Opaque),
            RenderKind::Cutout => Some(RenderLayer::Cutout),
            RenderKind::Translucent => Some(RenderLayer::Translucent),
        };
        Self {
            layer,
            known: true,
            full_cube: definition.occlusion == Occlusion::FullCube,
            group: definition.translucent_interface_group,
            materials: definition.face_materials,
        }
    }
}

/// Face decision between two cells inside the chunk, mirroring [`classify_directed_face`].
#[inline]
pub(crate) fn interior_face(
    classes: &[SlotClass],
    source_slot: u16,
    neighbor_slot: u16,
    mat_idx: usize,
) -> Option<(MaterialId, RenderLayer)> {
    let source = &classes[usize::from(source_slot)];
    let layer = source.layer?;
    if neighbor_slot == source_slot {
        if source.full_cube || source.group.is_some() {
            return None;
        }
    } else {
        let neighbor = &classes[usize::from(neighbor_slot)];
        if neighbor.known
            && (neighbor.full_cube || (source.group.is_some() && source.group == neighbor.group))
        {
            return None;
        }
    }
    Some((source.materials[mat_idx], layer))
}
