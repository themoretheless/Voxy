//! Native editor LOD acceptance using an explicit model recipe.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: lod_viewport model.vmodel [--smoke]")?;
    let mode = if std::env::args().any(|arg| arg == "--smoke") {
        voxy_editor::ViewportMode::LodSmoke
    } else {
        voxy_editor::ViewportMode::Interactive
    };
    let arguments: Vec<_> = std::env::args().collect();
    let scene = arguments
        .iter()
        .position(|arg| arg == "--scene")
        .map(|index| arguments.get(index + 1).ok_or("missing --scene path"))
        .transpose()?;
    let source = if arguments.iter().any(|arg| arg == "--manifest") {
        voxy_editor::ModelSource::Manifest {
            path: path.into(),
            asset: voxy_assets::AssetId("lod".into()),
        }
    } else {
        voxy_editor::ModelSource::File(path.into())
    };
    voxy_editor::run_model_viewport_with_scene(&source, mode, scene.map(std::path::Path::new))
}
