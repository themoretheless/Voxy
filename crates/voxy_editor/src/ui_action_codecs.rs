//! Engine-owned UI callbacks use the same opt-in save contract as game callbacks.
use voxy_gameplay::{
    GameplayError, UiActionCheckpointRegistry, UiActionEvent, UiActionHandler, UiActionHandlers,
    UiActionState,
};
use voxy_scene::{NodeId, ObjectId, SceneCommands, SceneGraph};
struct HideAction;
impl UiActionHandler for HideAction {
    fn dispatch(
        &mut self,
        _: &SceneGraph,
        event: &UiActionEvent,
        commands: &mut SceneCommands,
    ) -> Result<(), GameplayError> {
        commands.push(voxy_scene::SceneCommand::SetActive(event.owner, false))?;
        Ok(())
    }
    fn checkpoint(
        &self,
        _: &dyn Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<UiActionState, GameplayError> {
        UiActionState::new("voxy.ui.hide.v1".into(), Vec::new())
    }
}
pub(crate) fn register_handlers(actions: &mut UiActionHandlers) -> Result<(), GameplayError> {
    actions.register_checkpointable("voxy.ui.hide".into(), HideAction)
}
pub(crate) fn registry() -> UiActionCheckpointRegistry {
    let mut registry = UiActionCheckpointRegistry::default();
    registry
        .register("voxy.ui.hide.v1", |payload, _| {
            if !payload.is_empty() {
                return Err("invalid hide callback payload".into());
            }
            Ok(Box::new(HideAction))
        })
        .expect("static builtin UI codec is valid and unique");
    registry
}
