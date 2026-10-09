//! Owner-side projection of scene components into backend-independent instances.
use crate::{NodeId, SceneGraph, SceneGraphError};
use glam::Mat4;
use std::any::Any;

/// Restricted extraction failures retain the scene failure when admission succeeds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopedExtractionError {
    AccessDenied,
    Scene(SceneGraphError),
}
impl std::fmt::Display for ScopedExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied => {
                f.write_str("extraction requires scene read and render.extraction write")
            }
            Self::Scene(error) => write!(f, "extraction failed: {error}"),
        }
    }
}
impl std::error::Error for ScopedExtractionError {}
/// The immutable frame projection has one owner and a read-only scene input.
/// # Errors
/// Returns a validation error if the built-in extraction specification is invalid.
pub fn extraction_schedule() -> Result<&'static crate::SchedulePlan, crate::ScheduleError> {
    static PLAN: std::sync::OnceLock<Result<crate::SchedulePlan, crate::ScheduleError>> =
        std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        crate::SchedulePlan::build(
            &[crate::SystemSpec {
                name: "render.extract".into(),
                phase: 0,
                after: vec![],
                access: vec![
                    crate::SystemAccess {
                        resource: "scene".into(),
                        write: false,
                    },
                    crate::SystemAccess {
                        resource: "render.extraction".into(),
                        write: true,
                    },
                ],
            }],
            1,
        )
    })
    .as_ref()
    .map_err(Clone::clone)
}
#[derive(Clone, Debug)]
pub struct ExtractedInstance<T> {
    pub owner: NodeId,
    pub world: Mat4,
    pub component: T,
}
/// A bounded published projection. Backends own their resources separately.
/// Failed extraction preserves the last complete projection; callers must handle
/// the error before drawing it because its owners may have since been deleted.
/// Component clone cost belongs to the component type, not this count budget.
#[derive(Debug)]
pub struct SceneExtraction<T> {
    instances: Vec<ExtractedInstance<T>>,
    staging: Vec<ExtractedInstance<T>>,
    capacity: usize,
}
impl<T: Any + Clone + Send + Sync> SceneExtraction<T> {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            instances: Vec::new(),
            staging: Vec::new(),
            capacity,
        }
    }
    #[must_use]
    pub fn instances(&self) -> &[ExtractedInstance<T>] {
        &self.instances
    }
    /// Refreshes through restricted scene/projection capabilities.
    /// # Errors
    /// Missing grants or resolver failures preserve the previous publication.
    pub fn refresh_scoped_with(
        &mut self,
        access: crate::SceneSystemAccess<'_>,
        mut world: impl FnMut(&SceneGraph, NodeId) -> Result<Mat4, SceneGraphError>,
    ) -> Result<(), ScopedExtractionError> {
        access
            .require_write("render.extraction")
            .map_err(|_| ScopedExtractionError::AccessDenied)?;
        let scene = access
            .read()
            .map_err(|_| ScopedExtractionError::AccessDenied)?;
        self.refresh_with(scene, |owner| world(scene, owner))
            .map_err(ScopedExtractionError::Scene)
    }
    /// Refreshes at a structural/simulation barrier, before backend submission.
    /// Includes only effectively active components, in stable scene slot order.
    /// # Errors
    /// Rejects capacity overflow or nonfinite composed transforms atomically.
    pub fn refresh(&mut self, scene: &SceneGraph) -> Result<(), SceneGraphError> {
        self.refresh_with(scene, |owner| scene.world_matrix(owner))
    }
    /// Publishes a projection using an owner-provided render pose resolver.
    /// # Errors
    /// Resolver failures and capacity overflow preserve the previous publication.
    pub fn refresh_with(
        &mut self,
        scene: &SceneGraph,
        mut world: impl FnMut(NodeId) -> Result<Mat4, SceneGraphError>,
    ) -> Result<(), SceneGraphError> {
        // Overwrite retained staging entries in place so owned component payloads
        // (asset ID strings) reuse their allocations instead of dropping and cloning.
        let mut count = 0;
        for (owner, component) in scene.active_components::<T>() {
            if count >= self.capacity {
                return Err(SceneGraphError::Capacity);
            }
            let matrix = world(owner)?;
            if !matrix.is_finite() {
                return Err(SceneGraphError::WorldOverflow);
            }
            if let Some(entry) = self.staging.get_mut(count) {
                entry.owner = owner;
                entry.world = matrix;
                entry.component.clone_from(component);
            } else {
                self.staging.push(ExtractedInstance {
                    owner,
                    world: matrix,
                    component: component.clone(),
                });
            }
            count += 1;
        }
        self.staging.truncate(count);
        std::mem::swap(&mut self.instances, &mut self.staging);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transform;
    #[derive(Clone, Debug, PartialEq)]
    struct Model(u32);
    #[test]
    fn scoped_projection_denial_and_resolver_failure_retain_publication() {
        use crate::{SchedulePlan, SystemAccess, SystemSpec};
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(owner, Model(1)).unwrap();
        let mut projection = SceneExtraction::<Model>::new(1);
        projection.refresh(&scene).unwrap();
        scene.insert_component(owner, Model(2)).unwrap();
        for grants in [
            vec![("scene", false)],
            vec![("scene", false), ("render.extraction", false)],
            vec![("render.extraction", true)],
        ] {
            let plan = SchedulePlan::build(
                &[SystemSpec {
                    name: "render.extract".into(),
                    phase: 0,
                    after: vec![],
                    access: grants
                        .into_iter()
                        .map(|(resource, write)| SystemAccess {
                            resource: resource.into(),
                            write,
                        })
                        .collect(),
                }],
                1,
            )
            .unwrap();
            let failure = plan
                .run_scene(&mut scene, |_, access| {
                    projection.refresh_scoped_with(access, |_, _| panic!("denied resolver ran"))
                })
                .unwrap_err();
            assert_eq!(failure.error, ScopedExtractionError::AccessDenied);
            assert_eq!(projection.instances()[0].component, Model(1));
        }
        extraction_schedule()
            .unwrap()
            .run_scene(&mut scene, |_, mut access| {
                assert!(access.write().is_err());
                projection.refresh_scoped_with(access, |scene, owner| scene.world_matrix(owner))
            })
            .unwrap();
        assert_eq!(projection.instances()[0].component, Model(2));
        let failure = extraction_schedule()
            .unwrap()
            .run_scene(&mut scene, |_, access| {
                projection.refresh_scoped_with(access, |_, _| Err(SceneGraphError::WorldOverflow))
            })
            .unwrap_err();
        assert_eq!(
            failure.error,
            ScopedExtractionError::Scene(SceneGraphError::WorldOverflow)
        );
        assert_eq!(projection.instances()[0].component, Model(2));
    }
    #[test]
    fn projection_tracks_inherited_motion_activity_deletion_and_reuse() {
        let mut scene = SceneGraph::new(3);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        scene.insert_component(child, Model(7)).unwrap();
        let mut projection = SceneExtraction::<Model>::new(1);
        projection.refresh(&scene).unwrap();
        assert_eq!(projection.instances()[0].owner, child);
        scene
            .set_local(
                root,
                Transform {
                    translation: glam::Vec3::X,
                    ..Transform::default()
                },
            )
            .unwrap();
        projection.refresh(&scene).unwrap();
        assert_eq!(
            projection.instances()[0].world.w_axis,
            glam::Vec4::new(1.0, 0.0, 0.0, 1.0)
        );
        scene.set_active(root, false).unwrap();
        projection.refresh(&scene).unwrap();
        assert!(projection.instances().is_empty());
        scene.remove_subtree(root).unwrap();
        let reused = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(reused, Model(9)).unwrap();
        projection.refresh(&scene).unwrap();
        assert_ne!(projection.instances()[0].owner, child);
        assert_eq!(projection.instances()[0].component, Model(9));
    }
    #[test]
    fn invalid_projection_does_not_partially_publish() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(root, Model(1)).unwrap();
        let mut projection = SceneExtraction::<Model>::new(1);
        projection.refresh(&scene).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        scene.insert_component(child, Model(2)).unwrap();
        assert_eq!(projection.refresh(&scene), Err(SceneGraphError::Capacity));
        assert_eq!(projection.instances().len(), 1);
        scene.set_active(child, false).unwrap();
        projection.refresh(&scene).unwrap();
        scene.set_active(child, true).unwrap();
        scene
            .set_local(
                root,
                Transform {
                    scale: glam::Vec3::splat(f32::MAX),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .set_local(
                child,
                Transform {
                    scale: glam::Vec3::splat(2.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        let mut projection = SceneExtraction::<Model>::new(2);
        assert_eq!(
            projection.refresh(&scene),
            Err(SceneGraphError::WorldOverflow)
        );
        assert!(projection.instances().is_empty());
    }
}
