//! Failures of the gameplay runtime, audio, UI and liquid systems.
#![allow(clippy::module_name_repetitions)]

/// Failures of the gameplay runtime, audio, UI and liquid systems.
#[derive(Debug, thiserror::Error)]
pub enum GameplayError {
    /// A free-form failure message.
    #[error("{0}")]
    Message(String),
    /// The scene graph rejected a node lookup or edit.
    #[error(transparent)]
    SceneGraph(#[from] voxy_scene::SceneGraphError),
    /// An asset could not be written atomically.
    #[error(transparent)]
    AtomicSave(#[from] voxy_assets::AtomicSaveError),
    /// A JSON document could not be parsed or serialised.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The audio mixer or decoder failed.
    #[error(transparent)]
    Audio(#[from] voxy_audio::AudioError),
    /// The UI runtime rejected a layout, font or action.
    #[error(transparent)]
    Ui(#[from] voxy_ui::UiError),
    /// A scheduled system touched a resource it has no access to.
    #[error(transparent)]
    ResourceAccess(#[from] voxy_scene::ResourceAccessDenied),
    /// A scheduled system touched the scene without access.
    #[error(transparent)]
    SceneAccess(#[from] voxy_scene::SceneAccessDenied),
    /// The scene simulation rejected an attachment or step.
    #[error(transparent)]
    Simulation(#[from] voxy_scene::SimulationError),
    /// A size or count did not fit the target integer type.
    #[error(transparent)]
    IntRange(#[from] std::num::TryFromIntError),
}

impl From<String> for GameplayError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for GameplayError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_owned())
    }
}

impl PartialEq for GameplayError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}
