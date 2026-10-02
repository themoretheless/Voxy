//! Isolated direct-entry native review. Logs and project files are retained.
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments: Vec<_> = std::env::args_os().skip(1).collect();
    // LaunchServices bundles cannot supply command-line arguments. Their
    // explicit review environment uses the same parser and validation path.
    if arguments.is_empty()
        && let Some(model) = std::env::var_os("VOXY_REVIEW_MODEL")
    {
        arguments.extend(["--model".into(), model]);
        if std::env::var_os("VOXY_REVIEW_LOD_SMOKE").as_deref() == Some(std::ffi::OsStr::new("1")) {
            arguments.push("--smoke".into());
        }
        if let Some(asset) = std::env::var_os("VOXY_REVIEW_MANIFEST_ASSET") {
            arguments.extend(["--manifest-asset".into(), asset]);
        }
        if let Some(scene) = std::env::var_os("VOXY_REVIEW_SCENE") {
            arguments.extend(["--scene".into(), scene]);
        }
    }
    let mut args = arguments.iter();
    let project = if args.next().map(|arg| arg.as_os_str())
        == Some(std::ffi::OsStr::new("--review-project"))
    {
        std::path::PathBuf::from(args.next().ok_or("missing review project")?)
    } else {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let project = std::env::temp_dir().join(format!(
            "voxy-native-editor-review-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&project)?;
        let log = std::fs::File::create(project.join("native.log"))?;
        let mut command = Command::new(std::env::current_exe()?);
        command
            .arg("--review-project")
            .arg(&project)
            .args(&arguments)
            .env("VOXY_EDITOR_TRACE_INPUT", "1")
            .env("VOXY_LOD_TRACE", "1")
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
        #[cfg(unix)]
        {
            // Re-enter the same direct executable, keeping LaunchServices PID
            // ownership while std performs safe environment/stdio setup.
            use std::os::unix::process::CommandExt;
            return Err(command.exec().into());
        }
        #[cfg(not(unix))]
        {
            let status = command.status()?;
            return if status.success() {
                Ok(())
            } else {
                Err("review child failed".into())
            };
        }
    };
    println!(
        "NATIVE REVIEW project={} pid={}",
        project.display(),
        std::process::id()
    );
    let mut requested_model = None;
    let mut manifest_asset = None;
    let mut requested_scene = None;
    let mut mode = voxy_editor::ViewportMode::Interactive;
    while let Some(arg) = args.next() {
        if arg == "--model" {
            if requested_model.is_some() {
                return Err("duplicate --model".into());
            }
            requested_model = Some(std::path::PathBuf::from(
                args.next().ok_or("missing --model path")?,
            ));
        } else if arg == "--smoke" {
            mode = voxy_editor::ViewportMode::LodSmoke;
        } else if arg == "--manifest-asset" {
            if manifest_asset.is_some() {
                return Err("duplicate --manifest-asset".into());
            }
            manifest_asset = Some(voxy_assets::AssetId(
                args.next()
                    .ok_or("missing manifest asset")?
                    .to_str()
                    .ok_or("manifest asset must be UTF-8")?
                    .into(),
            ));
        } else if arg == "--scene" {
            if requested_scene.is_some() {
                return Err("duplicate --scene".into());
            }
            requested_scene = Some(std::path::PathBuf::from(
                args.next().ok_or("missing scene path")?,
            ));
        } else {
            return Err(format!("unknown review argument: {}", arg.to_string_lossy()).into());
        }
    }
    if mode == voxy_editor::ViewportMode::LodSmoke && requested_model.is_none() {
        return Err("--smoke requires --model with a certified LOD recipe".into());
    }
    let model = project.join("quad.obj");
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&model)?
        .write_all(include_bytes!("../../voxy_render/examples/assets/quad.obj"))?;
    let path = match requested_model {
        Some(path) => path.canonicalize()?,
        None => model,
    };
    let source = match manifest_asset {
        Some(asset) => voxy_editor::ModelSource::Manifest { path, asset },
        None => voxy_editor::ModelSource::File(path),
    };
    let scene = project.join("scene.json");
    if let Some(source) = requested_scene {
        // Copy the seed into the retained review project; editor saves never
        // modify the user's supplied scene.
        std::fs::copy(source, &scene)?;
    }
    let result = voxy_editor::run_model_viewport_with_scene(&source, mode, Some(&scene));
    if let Err(error) = &result {
        eprintln!("NATIVE REVIEW FAILED: {error}");
    }
    result
}
