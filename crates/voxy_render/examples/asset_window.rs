//! Compatibility entry point for the application-owned native model viewport.
use voxy_editor::{ModelSource, ViewportMode, run_model_viewport};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let first = arguments
        .first()
        .ok_or("usage: asset_window model.obj | --manifest assets.json logical-id [--smoke]")?;
    let mode = if arguments.iter().any(|arg| arg == "--smoke") {
        ViewportMode::Smoke
    } else {
        ViewportMode::Interactive
    };
    let source = if first == "--manifest" {
        ModelSource::Manifest {
            path: arguments.get(1).ok_or("missing manifest path")?.into(),
            asset: voxy_assets::AssetId(
                arguments.get(2).ok_or("missing logical asset ID")?.clone(),
            ),
        }
    } else {
        ModelSource::File(first.into())
    };
    run_model_viewport(&source, mode)
}
