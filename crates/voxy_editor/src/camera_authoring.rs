//! Follow camera actions reuse the hierarchy, component validation and scene history.
//! The same hierarchy target picker serves physics joints (see `joint_authoring`).
use crate::*;
use serde_json::Value;

/// Component that a pending hierarchy target pick will write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickPurpose {
    FollowCamera,
    PhysicsJoint,
}
impl PickPurpose {
    pub(crate) fn schema(self) -> &'static str {
        match self {
            Self::FollowCamera => "game.follow-camera.v1",
            Self::PhysicsJoint => crate::joint_authoring::SCHEMA,
        }
    }
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::FollowCamera => "PICK FOLLOW TARGET",
            Self::PhysicsJoint => "PICK JOINT TARGET",
        }
    }
    fn noun(self) -> &'static str {
        match self {
            Self::FollowCamera => "follow camera",
            Self::PhysicsJoint => "physics joint",
        }
    }
    fn request_token(
        self,
        object: &voxy_scene::SceneObject,
    ) -> Result<[u8; 32], serde_json::Error> {
        match self {
            Self::FollowCamera => request_token(object),
            Self::PhysicsJoint => crate::joint_authoring::request_token(object),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct FollowPick {
    owner: ObjectId,
    pub(super) key: [u8; 32],
    pub(super) purpose: PickPurpose,
    expected: Option<Value>,
}
pub(crate) fn request_token(
    object: &voxy_scene::SceneObject,
) -> Result<[u8; 32], serde_json::Error> {
    Ok(*blake3::hash(&serde_json::to_vec(&(
        &object.id,
        object.components.get("game.follow-camera.v1"),
        object.components.get("editor.camera.v1"),
    ))?)
    .as_bytes())
}
pub(crate) fn choice_token(request: [u8; 32], id: &ObjectId) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&request);
    hasher.update(id.0.as_bytes());
    *hasher.finalize().as_bytes()
}
impl App {
    /// `toggle` removes an existing component or starts picking a target for a new one;
    /// otherwise the pick retargets the existing component.
    pub(super) fn begin_target_pick(
        &mut self,
        purpose: PickPurpose,
        token: [u8; 32],
        toggle: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let noun = purpose.noun();
        if self.play.playing.is_some() {
            return Err(format!("stop play before editing a {noun}").into());
        }
        let mut document = self.authoring_document()?;
        let object = document
            .objects
            .get_mut(self.selected)
            .ok_or("missing target pick owner")?;
        if purpose.request_token(object)? != token {
            return Err(format!("{noun} changed before activation").into());
        }
        let expected = object.components.get(purpose.schema()).cloned();
        if toggle && expected.is_some() {
            object.components.remove(purpose.schema());
            return self.commit_follow_document(document);
        }
        if !toggle && expected.is_none() {
            return Err(format!("{noun} component disappeared").into());
        }
        let epoch = self
            .follow_pick_epoch
            .checked_add(1)
            .ok_or("follow picker epoch exhausted")?;
        let key = *blake3::hash(&serde_json::to_vec(&(&object.id, &expected, epoch))?).as_bytes();
        self.follow_pick_epoch = epoch;
        self.follow_pick = Some(FollowPick {
            owner: object.id.clone(),
            expected,
            key,
            purpose,
        });
        self.parenting = None;
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
    pub(super) fn choose_follow_target(
        &mut self,
        token: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop play before choosing a target".into());
        }
        let request = self
            .follow_pick
            .clone()
            .ok_or("no follow target selection in progress")?;
        self.panel_cache = None;
        let mut document = self.authoring_document()?;
        let target = document
            .objects
            .iter()
            .find(|object| choice_token(request.key, &object.id) == token)
            .ok_or("stale target selection")?
            .id
            .clone();
        self.follow_pick = None;
        let owner = document
            .objects
            .iter_mut()
            .find(|object| object.id == request.owner)
            .ok_or("target pick owner disappeared")?;
        if owner.components.get(request.purpose.schema()) != request.expected.as_ref() {
            return Err(
                format!("{} changed during target selection", request.purpose.noun()).into(),
            );
        }
        if request.purpose == PickPurpose::PhysicsJoint {
            let added = request.expected.is_none();
            let joint = self.picked_joint(&request.owner, request.expected, target)?;
            owner
                .components
                .insert(request.purpose.schema().into(), joint);
            self.commit_follow_document(document)?;
            return if added {
                self.show_joint_fields()
            } else {
                Ok(())
            };
        }
        let mut descriptor: voxy_gameplay::FollowCamera = if let Some(expected) = request.expected {
            serde_json::from_value(expected)?
        } else {
            voxy_gameplay::FollowCamera {
                target: target.clone(),
                offset: [0., 0.2, 0.],
                half_life: 0.15,
            }
        };
        descriptor.target = target;
        owner.components.insert(
            "game.follow-camera.v1".into(),
            serde_json::to_value(descriptor)?,
        );
        if !owner.components.contains_key("editor.camera.v1") {
            let mut view = self.camera.clone();
            if view.legacy {
                view.perspective = true;
            }
            view.legacy = false;
            owner
                .components
                .insert("editor.camera.v1".into(), serde_json::to_value(view)?);
        }
        self.commit_follow_document(document)
    }
    pub(crate) fn commit_follow_document(
        &mut self,
        document: SceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let next = self.validate_authoring_document(&document)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing camera authoring history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next;
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
}
