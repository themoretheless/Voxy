//! Engine-owned UI callbacks use the same opt-in save contract as game callbacks.
//! Delegates to `voxy_player` so standalone players and editor Play share identical builtin codecs.
use voxy_gameplay::{GameplayError, UiActionCheckpointRegistry, UiActionHandlers};

pub(crate) fn register_handlers(actions: &mut UiActionHandlers) -> Result<(), GameplayError> {
    voxy_player::register_builtin_ui_handlers(actions)
}

pub(crate) fn registry() -> UiActionCheckpointRegistry {
    voxy_player::builtin_ui_action_checkpoint_registry()
}
