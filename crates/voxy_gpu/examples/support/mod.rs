use voxy_world::{
    BlockDef, BlockRegistry, CollisionShape, MaterialId, Occlusion, RenderKind, ResourceKey,
    TerrainPalette,
};

pub fn test_palette()
-> Result<(TerrainPalette, voxy_world::BlockStateId), Box<dyn std::error::Error>> {
    let defs = ["air", "surface", "soil", "stone", "water"]
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            Ok(BlockDef {
                key: ResourceKey::parse(format!("voxy:{key}"))?,
                render: if i == 0 {
                    RenderKind::Invisible
                } else {
                    RenderKind::Opaque
                },
                occlusion: if i == 0 {
                    Occlusion::None
                } else {
                    Occlusion::FullCube
                },
                collision: if i == 0 {
                    CollisionShape::Empty
                } else {
                    CollisionShape::FullCube
                },
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            })
        })
        .collect::<Result<Vec<_>, voxy_world::RegistryError>>()?;
    let registry = BlockRegistry::new(defs)?;
    let lookup = |key: &str| -> Result<_, Box<dyn std::error::Error>> {
        registry
            .find(&ResourceKey::parse(format!("voxy:{key}"))?)
            .ok_or_else(|| "missing block".into())
    };
    let palette = TerrainPalette {
        air: lookup("air")?,
        surface: lookup("surface")?,
        soil: lookup("soil")?,
        stone: lookup("stone")?,
    };
    let water = lookup("water")?;
    Ok((palette, water))
}
