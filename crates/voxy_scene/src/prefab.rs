//! Reusable scene hierarchies with independently cloned component values.
use crate::{NodeId, SceneGraph, SceneGraphError, Transform};
use std::any::{Any, TypeId};
use std::collections::HashMap;

trait Prototype: Send + Sync {
    fn attach(&self, scene: &mut SceneGraph, id: NodeId);
}
impl<T: Any + Clone + Send + Sync> Prototype for T {
    fn attach(&self, scene: &mut SceneGraph, id: NodeId) {
        // IDs are freshly allocated by instantiate after complete validation.
        scene
            .insert_component(id, self.clone())
            .expect("fresh prefab node");
    }
}

/// Parent indices refer to earlier nodes; multiple roots are supported.
/// Components must be `Clone`; internal `NodeId` references are not remapped.
pub struct PrefabNode {
    pub parent: Option<usize>,
    pub local: Transform,
    pub name: String,
    pub active: bool,
    components: HashMap<TypeId, Box<dyn Prototype>>,
}
impl std::fmt::Debug for PrefabNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrefabNode")
            .field("parent", &self.parent)
            .field("local", &self.local)
            .field("name", &self.name)
            .field("active", &self.active)
            .field("components", &self.components.len())
            .finish()
    }
}
impl PrefabNode {
    #[must_use]
    pub fn new(parent: Option<usize>, local: Transform) -> Self {
        Self {
            parent,
            local,
            name: String::new(),
            active: true,
            components: HashMap::new(),
        }
    }

    /// Adds or replaces the prototype for one component type.
    #[must_use]
    pub fn with_component<T: Any + Clone + Send + Sync>(mut self, value: T) -> Self {
        self.components.insert(TypeId::of::<T>(), Box::new(value));
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrefabError {
    InvalidParent { node: usize, parent: usize },
    Scene(SceneGraphError),
}
impl std::fmt::Display for PrefabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "prefab error: {self:?}")
    }
}
impl std::error::Error for PrefabError {}
impl From<SceneGraphError> for PrefabError {
    fn from(value: SceneGraphError) -> Self {
        Self::Scene(value)
    }
}

#[derive(Debug, Default)]
pub struct Prefab {
    pub nodes: Vec<PrefabNode>,
}
impl Prefab {
    /// Creates an independent hierarchy. Returned handles use prototype index order.
    /// Each prototype root attaches to `parent`; children retain their local transform.
    /// # Errors
    /// Rejects invalid topology, transforms, parent handles and insufficient capacity
    /// before changing the destination scene. Panics in user-defined Clone or Drop
    /// implementations are not recovered.
    pub fn instantiate(
        &self,
        scene: &mut SceneGraph,
        parent: Option<NodeId>,
    ) -> Result<Vec<NodeId>, PrefabError> {
        if let Some(parent) = parent {
            scene.node(parent)?;
        }
        if self.nodes.len() > scene.capacity - scene.count {
            return Err(SceneGraphError::Capacity.into());
        }
        for (index, node) in self.nodes.iter().enumerate() {
            node.local.matrix()?;
            if let Some(parent) = node.parent
                && parent >= index
            {
                return Err(PrefabError::InvalidParent {
                    node: index,
                    parent,
                });
            }
        }
        let mut ids = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let id = scene.spawn(node.parent.map(|index| ids[index]).or(parent), node.local)?;
            scene.set_name(id, node.name.clone())?;
            scene.set_active(id, node.active)?;
            for component in node.components.values() {
                component.as_ref().attach(scene, id);
            }
            ids.push(id);
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    fn translated(x: f32) -> Transform {
        Transform {
            translation: Vec3::new(x, 0.0, 0.0),
            ..Transform::default()
        }
    }
    #[test]
    fn instances_have_independent_components_and_follow_external_parent() {
        let prefab = Prefab {
            nodes: vec![
                PrefabNode::new(None, translated(2.0)).with_component(vec![100_u32]),
                PrefabNode::new(Some(0), translated(3.0)).with_component(String::from("weapon")),
            ],
        };
        let mut scene = SceneGraph::new(5);
        let parent = scene.spawn(None, translated(10.0)).unwrap();
        let a = prefab.instantiate(&mut scene, Some(parent)).unwrap();
        let b = prefab.instantiate(&mut scene, None).unwrap();
        scene.component_mut::<Vec<u32>>(a[0]).unwrap().unwrap()[0] = 5;
        assert_eq!(scene.component::<Vec<u32>>(b[0]).unwrap().unwrap(), &[100]);
        assert_eq!(
            scene
                .world_matrix(a[1])
                .unwrap()
                .transform_point3(Vec3::ZERO),
            Vec3::new(15.0, 0.0, 0.0)
        );
        scene.remove_subtree(a[0]).unwrap();
        assert_eq!(scene.component::<String>(b[1]).unwrap().unwrap(), "weapon");
    }
    #[test]
    fn prefab_metadata_preserves_local_activity_under_disabled_parent() {
        let mut root = PrefabNode::new(None, Transform::default());
        root.name = String::from("enemy");
        let mut child = PrefabNode::new(Some(0), Transform::default());
        child.name = String::from("weapon");
        child.active = false;
        let prefab = Prefab {
            nodes: vec![root, child],
        };
        let mut scene = SceneGraph::new(3);
        let parent = scene.spawn(None, Transform::default()).unwrap();
        scene.set_active(parent, false).unwrap();
        let ids = prefab.instantiate(&mut scene, Some(parent)).unwrap();
        assert_eq!(scene.name(ids[0]), Ok("enemy"));
        assert_eq!(scene.name(ids[1]), Ok("weapon"));
        assert_eq!(scene.active_self(ids[0]), Ok(true));
        assert_eq!(scene.active_in_hierarchy(ids[0]), Ok(false));
        scene.set_active(parent, true).unwrap();
        assert_eq!(scene.active_in_hierarchy(ids[0]), Ok(true));
        assert_eq!(scene.active_in_hierarchy(ids[1]), Ok(false));
    }

    #[test]
    fn invalid_templates_and_capacity_are_atomic() {
        let mut scene = SceneGraph::new(3);
        let parent = scene.spawn(None, Transform::default()).unwrap();
        let mut prefab = Prefab {
            nodes: vec![
                PrefabNode::new(None, Transform::default()),
                PrefabNode::new(Some(1), Transform::default()),
            ],
        };
        assert_eq!(
            prefab.instantiate(&mut scene, Some(parent)),
            Err(PrefabError::InvalidParent { node: 1, parent: 1 })
        );
        assert_eq!(scene.len(), 1);
        prefab.nodes[1].parent = Some(0);
        prefab.nodes[1].local.translation.x = f32::NAN;
        assert_eq!(
            prefab.instantiate(&mut scene, None),
            Err(PrefabError::Scene(SceneGraphError::InvalidTransform))
        );
        assert_eq!(scene.len(), 1);
        prefab.nodes[1].local = Transform::default();
        prefab.instantiate(&mut scene, None).unwrap();
        assert_eq!(
            prefab.instantiate(&mut scene, None),
            Err(PrefabError::Scene(SceneGraphError::Capacity))
        );
        assert_eq!(scene.len(), 3);
    }
    #[test]
    fn foreign_and_stale_external_parents_are_rejected_even_for_empty_prefabs() {
        let prefab = Prefab::default();
        let mut scene = SceneGraph::new(1);
        let foreign = SceneGraph::new(1)
            .spawn(None, Transform::default())
            .unwrap();
        assert_eq!(
            prefab.instantiate(&mut scene, Some(foreign)),
            Err(PrefabError::Scene(SceneGraphError::InvalidNode))
        );
        let stale = scene.spawn(None, Transform::default()).unwrap();
        scene.remove_subtree(stale).unwrap();
        assert_eq!(
            prefab.instantiate(&mut scene, Some(stale)),
            Err(PrefabError::Scene(SceneGraphError::InvalidNode))
        );
        assert!(prefab.instantiate(&mut scene, None).unwrap().is_empty());
    }
}
