//! Failures of the demo scenes and their asset loaders.
#![allow(clippy::module_name_repetitions)]

/// Failures of the demo scenes and their asset loaders.
#[derive(Debug, thiserror::Error)]
pub enum DemoError {
    /// A free-form failure message.
    #[error("{0}")]
    Message(String),
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON document could not be parsed or serialised.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The renderer rejected scene geometry.
    #[error(transparent)]
    Scene(#[from] voxy_render::SceneError),
    /// A GPU compute program failed.
    #[error(transparent)]
    Compute(#[from] voxy_render::ComputeError),
    /// The CUDA motion backend failed.
    #[error(transparent)]
    Motion(#[from] crate::cuda_motion::MotionError),
    /// The CUDA character step failed.
    #[error(transparent)]
    Character(#[from] physics::CharacterError<voxy_gpu::CudaCollisionError>),
}

impl From<String> for DemoError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for DemoError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_owned())
    }
}

impl PartialEq for DemoError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}
