//! Failures of the editor's import, authoring, playback and preview tools.
#![allow(clippy::module_name_repetitions)]

/// Failures of the editor's import, authoring, playback and preview tools.
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    /// A free-form failure message.
    #[error("{0}")]
    Message(String),
    /// Scoped render extraction could not refresh.
    #[error(transparent)]
    ScopedExtraction(#[from] voxy_scene::ScopedExtractionError),
    /// The scene graph rejected a node lookup or edit.
    #[error(transparent)]
    SceneGraph(#[from] voxy_scene::SceneGraphError),
    /// A gameplay runtime call failed.
    #[error(transparent)]
    Gameplay(#[from] voxy_gameplay::GameplayError),
    /// The asset catalog rejected a request.
    #[error(transparent)]
    Asset(#[from] voxy_assets::AssetError),
    /// The import worker could not accept or finish a job.
    #[error(transparent)]
    ImportWorker(#[from] voxy_assets::ImportWorkerError),
    /// The audio mixer or decoder failed.
    #[error(transparent)]
    Audio(#[from] voxy_audio::AudioError),
    /// A scheduled system touched a resource it has no access to.
    #[error(transparent)]
    ResourceAccess(#[from] voxy_scene::ResourceAccessDenied),
    /// A scheduled system touched the scene without access.
    #[error(transparent)]
    SceneAccess(#[from] voxy_scene::SceneAccessDenied),
    /// A size or count did not fit the target integer type.
    #[error(transparent)]
    IntRange(#[from] std::num::TryFromIntError),
    /// The asset dependency graph rejected an edge.
    #[error(transparent)]
    Dependency(#[from] voxy_assets::DependencyError),
    /// The source poll worker failed.
    #[error(transparent)]
    PollWorker(#[from] voxy_assets::PollWorkerError),
    /// A fog descriptor was invalid.
    #[error(transparent)]
    Fog(#[from] voxy_scene::FogError),
    /// The renderer rejected scene geometry.
    #[error(transparent)]
    Scene(#[from] voxy_render::SceneError),
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Text input was not valid UTF-8.
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    /// An asset location was invalid.
    #[error(transparent)]
    Location(#[from] voxy_assets::LocationError),
    /// A JSON document could not be parsed or serialised.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The animator rejected a clip, rig or pose.
    #[error(transparent)]
    Animation(#[from] voxy_animation::AnimationError),
    /// The gameplay physics step failed.
    #[error(transparent)]
    Physics(#[from] voxy_gameplay::PhysicsError),
    /// A glTF document could not be parsed.
    #[error(transparent)]
    Gltf(#[from] gltf::Error),
    /// A glTF model could not be imported.
    #[error(transparent)]
    Model(#[from] voxy_render::ModelError),
    /// A skinned LOD archive was invalid.
    #[error(transparent)]
    SkinnedLod(#[from] voxy_render::SkinnedLodError),
    /// The GPU skinner rejected an upload.
    #[error(transparent)]
    SceneSkin(#[from] voxy_render::SceneSkinError),
    /// An OBJ mesh could not be parsed.
    #[error(transparent)]
    Obj(#[from] voxy_render::ObjError),
    /// An image asset could not be decoded.
    #[error(transparent)]
    ImageAsset(#[from] voxy_render::ImageAssetError),
    /// The scene camera was invalid.
    #[error(transparent)]
    Camera(#[from] voxy_render::InvalidSceneCamera),
    /// The sprite batch rejected a draw.
    #[error(transparent)]
    SpriteBatch(#[from] voxy_render::SpriteBatchError),
}

impl From<String> for EditorError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for EditorError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_owned())
    }
}

impl PartialEq for EditorError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}
