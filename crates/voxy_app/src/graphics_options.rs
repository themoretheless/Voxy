//! Validate native graphics selection before creating a window or driver.
use voxy_render::{GraphicsBackend, GraphicsOptions};

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<GraphicsOptions, String> {
    let mut args = args.into_iter();
    let mut options = GraphicsOptions::default();
    let mut selected = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fallback" => options.force_fallback_adapter = true,
            "--backend" => {
                if selected {
                    return Err("--backend must be specified once".into());
                }
                selected = true;
                options.backend = match args.next().as_deref() {
                    Some("auto") => GraphicsBackend::Auto,
                    Some("metal") => GraphicsBackend::Metal,
                    Some("dx12") => GraphicsBackend::DirectX12,
                    Some("vulkan") => GraphicsBackend::Vulkan,
                    Some("gl") => GraphicsBackend::OpenGl,
                    _ => return Err("--backend expects auto|metal|dx12|vulkan|gl".into()),
                };
            }
            _ => {}
        }
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options(args: &[&str]) -> Result<GraphicsOptions, String> {
        parse(args.iter().map(ToString::to_string))
    }
    #[test]
    fn native_selection_and_other_app_flags() {
        assert_eq!(options(&[]).unwrap().backend, GraphicsBackend::Auto);
        for (name, backend) in [
            ("auto", GraphicsBackend::Auto),
            ("metal", GraphicsBackend::Metal),
            ("dx12", GraphicsBackend::DirectX12),
            ("vulkan", GraphicsBackend::Vulkan),
            ("gl", GraphicsBackend::OpenGl),
        ] {
            let selected = options(&["--gpu-terrain", "--backend", name, "--autopilot"]).unwrap();
            assert_eq!(selected.backend, backend);
            assert!(!selected.force_fallback_adapter);
        }
        assert!(options(&["--fallback"]).unwrap().force_fallback_adapter);
    }
    #[test]
    fn invalid_or_duplicate_selection_precedes_window_initialization() {
        for args in [
            vec!["--backend"],
            vec!["--backend", "nope"],
            vec!["--backend", "webgpu"],
            vec!["--backend", "metal", "--backend", "vulkan"],
        ] {
            assert!(options(&args).is_err(), "{args:?}");
        }
    }
}
