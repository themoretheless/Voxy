use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BlockStateId(u32);

impl BlockStateId {
    pub const AIR: Self = Self(0);

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    #[cfg(test)]
    pub(crate) const fn from_test(value: u32) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MaterialId(pub u16);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InterfaceGroupId(pub u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderKind {
    Invisible,
    Opaque,
    Cutout,
    Translucent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Occlusion {
    None,
    FullCube,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionShape {
    Empty,
    FullCube,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceKey(Arc<str>);

impl ResourceKey {
    /// Parses a namespaced lowercase content key.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::InvalidKey`] for malformed keys.
    pub fn parse(value: impl Into<Arc<str>>) -> Result<Self, RegistryError> {
        let value = value.into();
        let Some((namespace, path)) = value.split_once(':') else {
            return Err(RegistryError::InvalidKey(value));
        };
        let valid = !namespace.is_empty()
            && !path.is_empty()
            && namespace
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && path.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'-' | b'/' | b'.')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(RegistryError::InvalidKey(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockDef {
    pub key: ResourceKey,
    pub render: RenderKind,
    pub occlusion: Occlusion,
    pub collision: CollisionShape,
    pub face_materials: [MaterialId; 6],
    pub translucent_interface_group: Option<InterfaceGroupId>,
    pub emission: u8,
    /// Integer resistance used by deterministic destruction planners.
    pub blast_resistance: u16,
}

#[derive(Clone, Debug)]
pub struct BlockRegistry {
    definitions: Arc<[BlockDef]>,
    by_key: BTreeMap<ResourceKey, BlockStateId>,
}

impl BlockRegistry {
    /// Builds an immutable session registry. The first entry must be canonical air.
    ///
    /// # Errors
    ///
    /// Rejects missing/invalid air, duplicate keys, invalid emission, and ID overflow.
    pub fn new(definitions: Vec<BlockDef>) -> Result<Self, RegistryError> {
        if definitions.is_empty() {
            return Err(RegistryError::MissingAir);
        }
        let air = &definitions[0];
        if air.key.as_str() != "voxy:air"
            || air.render != RenderKind::Invisible
            || air.occlusion != Occlusion::None
            || air.collision != CollisionShape::Empty
            || air.blast_resistance != 0
        {
            return Err(RegistryError::InvalidAir);
        }
        let mut by_key = BTreeMap::new();
        for (index, definition) in definitions.iter().enumerate() {
            if definition.emission > 15 {
                return Err(RegistryError::InvalidEmission {
                    key: definition.key.clone(),
                    emission: definition.emission,
                });
            }
            let id =
                BlockStateId(u32::try_from(index).map_err(|_| RegistryError::TooManyDefinitions)?);
            if by_key.insert(definition.key.clone(), id).is_some() {
                return Err(RegistryError::DuplicateKey(definition.key.clone()));
            }
        }
        Ok(Self {
            definitions: definitions.into(),
            by_key,
        })
    }

    #[must_use]
    pub fn get(&self, id: BlockStateId) -> Option<&BlockDef> {
        usize::try_from(id.0)
            .ok()
            .and_then(|index| self.definitions.get(index))
    }

    #[must_use]
    pub fn find(&self, key: &ResourceKey) -> Option<BlockStateId> {
        self.by_key.get(key).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    InvalidKey(Arc<str>),
    MissingAir,
    InvalidAir,
    DuplicateKey(ResourceKey),
    InvalidEmission { key: ResourceKey, emission: u8 },
    TooManyDefinitions,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid block registry: {self:?}")
    }
}

impl std::error::Error for RegistryError {}

#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_stable_dense_ids() {
        let registry = test_registry();
        assert_eq!(
            registry.get(BlockStateId::AIR).unwrap().key.as_str(),
            "voxy:air"
        );
        let stone = ResourceKey::parse("voxy:stone").unwrap();
        assert_eq!(registry.find(&stone).unwrap().get(), 1);
    }

    #[test]
    fn key_and_air_invariants_are_enforced() {
        assert!(ResourceKey::parse("No Namespace").is_err());
        assert_eq!(
            BlockRegistry::new(Vec::new()).unwrap_err(),
            RegistryError::MissingAir
        );
    }
}
