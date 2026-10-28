//! Durable anchor identity without serializing process-local scene handles.
use super::*;
use serde::{Deserialize, Serialize};
use voxy_scene::ObjectId;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportAnchorCheckpoint {
    version: u32,
    owner: ObjectId,
    local_position: [u64; 3],
    face: usize,
    sign: u64,
}
impl SupportAnchor {
    /// # Errors
    /// Rejects a foreign/deleted owner or a missing durable identity.
    pub fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<SupportAnchorCheckpoint, PhysicsError> {
        if scene.identity() != self.scene {
            return Err(PhysicsError::InvalidBody);
        }
        scene.local(self.owner)?;
        let owner = identify(self.owner)
            .filter(|id| !id.0.is_empty())
            .ok_or(PhysicsError::InvalidBody)?;
        Ok(SupportAnchorCheckpoint {
            version: 1,
            owner,
            local_position: self.local_position.to_array().map(f64::to_bits),
            face: self.face,
            sign: self.sign.to_bits(),
        })
    }
}
impl SupportAnchorCheckpoint {
    /// Rebind to the new scene without reprobing, projecting or changing coordinates.
    /// Geometry may have moved or changed since acquisition; the normal support
    /// resolver decides whether the retained anchor is still admissible next tick.
    /// # Errors
    /// Rejects invalid state, missing identities and foreign/stale resolved handles.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    ) -> Result<SupportAnchor, PhysicsError> {
        let position = DVec3::from_array(self.local_position.map(f64::from_bits));
        let sign = f64::from_bits(self.sign);
        if self.version != 1
            || self.owner.0.is_empty()
            || self.face >= 3
            || !position.is_finite()
            || ![-1., 1.].contains(&sign)
        {
            return Err(PhysicsError::InvalidBody);
        }
        let owner = resolve(&self.owner).ok_or(PhysicsError::InvalidBody)?;
        scene.local(owner)?;
        Ok(SupportAnchor {
            scene: scene.identity(),
            owner,
            local_position: position,
            face: self.face,
            sign,
        })
    }
}
