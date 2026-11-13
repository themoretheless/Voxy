//! Executable game setup belongs to the host, separate from serialized resources.
//! Re-exports the platform-independent `GameHost` and `BehaviorSetup` contracts from `voxy_player`.
use std::path::Path;
pub use voxy_player::{BehaviorSetup, GameHost};

/// Run a project with application-owned codecs, behaviors and UI callbacks.
/// # Errors
/// Reports unsupported modes, invalid project data, setup and runtime failures.
pub fn run_project_with_host(
    path: &Path,
    mode: crate::ViewportMode,
    host: GameHost,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::project_runtime::run_project_host(path, mode, host)
}
