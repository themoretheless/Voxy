use voxy_world::*;

pub(crate) fn test_registry() -> BlockRegistry {
    BlockRegistry::new(vec![
        BlockDef {
            key: ResourceKey::parse("voxy:air").unwrap(),
            render: RenderKind::Invisible,
            occlusion: Occlusion::None,
            collision: CollisionShape::Empty,
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 0,
        },
        BlockDef {
            key: ResourceKey::parse("voxy:stone").unwrap(),
            render: RenderKind::Opaque,
            occlusion: Occlusion::FullCube,
            collision: CollisionShape::FullCube,
            face_materials: [MaterialId(1); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 20,
        },
    ])
    .unwrap()
}
