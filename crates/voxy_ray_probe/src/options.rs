//! Strict selection for the isolated experimental hardware probe.
use voxy_render::GraphicsBackend;

#[derive(Debug)]
pub(super) struct Options {
    pub backend: GraphicsBackend,
    pub require_nvidia: bool,
    pub rough_image: bool,
    pub face: bool,
}
impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, &'static str> {
        let mut result = Self {
            backend: GraphicsBackend::Auto,
            require_nvidia: false,
            rough_image: false,
            face: false,
        };
        let mut face_arguments = std::collections::BTreeSet::new();
        let mut experimental = false;
        let mut backend_set = false;
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--experimental" if !experimental => experimental = true,
                "--require-nvidia" if !result.require_nvidia => result.require_nvidia = true,
                "--rough-image" if !result.rough_image && !result.face => result.rough_image = true,
                "--face" if !result.face && !result.rough_image => result.face = true,
                "--eye-closeup"
                | "--lid-close-transition"
                | "--age-transition"
                | "--age-strong-transition"
                    if result.face => {}
                "--backend" if !backend_set => {
                    result.backend = match args.next().as_deref() {
                        Some("auto") => GraphicsBackend::Auto,
                        Some("metal") => GraphicsBackend::Metal,
                        Some("vulkan") => GraphicsBackend::Vulkan,
                        Some("dx12") => GraphicsBackend::DirectX12,
                        Some("gl") => GraphicsBackend::OpenGl,
                        _ => return Err("--backend requires auto|metal|vulkan|dx12|gl"),
                    };
                    backend_set = true;
                }
                "--face-preset" if face_arguments.insert(argument.clone()) => {
                    let path = args.next().ok_or("--face-preset requires a JSON path")?;
                    if path.is_empty() || path.starts_with("--") {
                        return Err("--face-preset requires a JSON path");
                    }
                }
                "--blink-transition"
                | "--brow-transition"
                | "--pupil-transition"
                | "--eye-closeup"
                | "--eye-side"
                | "--lid-clay"
                | "--lid-close-transition"
                | "--lid-flat-color"
                | "--lid-globes"
                | "--lid-head"
                | "--lid-surface"
                | "--mouth"
                | "--oral-above"
                | "--oral-closeup"
                | "--oral-transition"
                | "--skin-closeup"
                | "--skin-depth-transition"
                | "--skin-finish-transition"
                | "--skin-macro-light"
                | "--skin-side"
                | "--tongue"
                | "--tongue-transition"
                    if face_arguments.insert(argument.clone()) => {}
                _ => return Err("unknown, duplicate or conflicting ray-probe argument"),
            }
        }
        if !face_arguments.is_empty() && !result.face {
            return Err("face rendering arguments require --face");
        }
        if !experimental {
            return Err("use --experimental to run the isolated wgpu ray-query probe");
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Options, &'static str> {
        Options::parse(args.iter().map(|arg| (*arg).to_owned()))
    }
    #[test]
    fn selection_rejects_ignored_or_conflicting_arguments() {
        for args in [
            vec![],
            vec!["--experimental", "--backend"],
            vec!["--experimental", "--backend", "cuda"],
            vec!["--experimental", "--unknown"],
            vec![
                "--experimental",
                "--backend",
                "metal",
                "--backend",
                "vulkan",
            ],
            vec!["--experimental", "--face", "--rough-image"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        let options =
            parse(&["--experimental", "--backend", "vulkan", "--require-nvidia"]).unwrap();
        assert_eq!(options.backend, GraphicsBackend::Vulkan);
        assert!(options.require_nvidia);
        assert_eq!(
            parse(&["--experimental"]).unwrap().backend,
            GraphicsBackend::Auto
        );
    }
    #[test]
    fn face_modes_preserve_existing_render_arguments() {
        assert!(
            parse(&[
                "--experimental",
                "--face",
                "--backend",
                "metal",
                "--face-preset",
                "preset.json",
                "--skin-closeup",
                "--skin-macro-light"
            ])
            .is_ok()
        );
        assert!(parse(&["--experimental", "--lid-surface", "--eye-side", "--face"]).is_ok());
        for args in [
            vec!["--experimental", "--face", "--face-preset"],
            vec![
                "--experimental",
                "--face",
                "--face-preset",
                "--backend",
                "metal",
            ],
            vec!["--experimental", "--skin-closeup"],
            vec![
                "--experimental",
                "--face",
                "--skin-closeup",
                "--skin-closeup",
            ],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }
}
