//! Project-scoped export of the last saved authoring state.
use std::{
    path::PathBuf,
    process::{Child, Command, ExitStatus},
    thread::JoinHandle,
};

use crate::{App, GameHost, export_project_package_with_host};

#[derive(Debug, Default)]
pub(super) struct Build {
    pub(super) project: Option<(PathBuf, PathBuf)>,
    pub(super) player: Option<PathBuf>,
    child: Option<Child>,
    pub(super) launch: Option<Result<ExitStatus, String>>,
    worker: Option<JoinHandle<Result<PathBuf, String>>>,
    pub(super) result: Option<Result<PathBuf, String>>,
}

impl App {
    pub(super) fn build_label(&self) -> Option<&'static str> {
        let (_, scene) = self.build.project.as_ref()?;
        if self.authoring.scene_path.as_ref() != Some(scene)
            || self.standalone
            || self.behavior_setup.is_some()
            || self.ui_action_setup.is_some()
        {
            return None;
        }
        Some(if self.build.worker.is_some() {
            "Building game..."
        } else {
            "Build saved game"
        })
    }

    pub(super) fn package_run_label(&self) -> Option<&'static str> {
        self.build_label()?;
        self.build.player.as_ref()?;
        self.build.result.as_ref()?.as_ref().ok()?;
        Some(if self.build.child.is_some() {
            "Game running..."
        } else {
            "Run built game"
        })
    }

    pub(super) fn run_built_game(&mut self) {
        if self.package_run_label() != Some("Run built game") || self.play.playing.is_some() {
            return;
        }
        let (Some(player), Some(Ok(package))) = (&self.build.player, &self.build.result) else {
            return;
        };
        let mode = if self.prefab_smoke.is_some() {
            "--game-native-smoke"
        } else {
            "--game"
        };
        self.build.launch = None;
        match Command::new(player)
            .arg("--package")
            .arg(package)
            .arg(mode)
            .spawn()
        {
            Ok(child) => self.build.child = Some(child),
            Err(error) => self.build.launch = Some(Err(error.to_string())),
        }
        self.update_edit_title();
    }

    pub(super) fn poll_built_game(&mut self) {
        if let Some(child) = &mut self.build.child {
            let result = match child.try_wait() {
                Ok(None) => return,
                Ok(Some(status)) => Ok(status),
                Err(error) => Err(error.to_string()),
            };
            self.build.child = None;
            self.build.launch = Some(result);
            self.update_edit_title();
        }
    }

    pub(super) fn start_project_build(&mut self) {
        if self.build.worker.is_some() {
            return;
        }
        self.build.launch = None;
        let result = self.queue_project_build();
        self.build.result = result.err().map(Err);
        self.update_edit_title();
    }

    fn queue_project_build(&mut self) -> Result<(), String> {
        if self.build_label().is_none()
            || self.play.playing.is_some()
            || self.audio.is_pending()
            || self.editor_file_status().is_some()
            || self.restore_barrier.is_some()
            || self.model_relink.is_some()
        {
            return Err("Build unavailable during the current editor operation.".into());
        }
        let (descriptor, _) = self
            .build
            .project
            .as_ref()
            .ok_or("Missing project descriptor.")?;
        let descriptor = descriptor.clone();
        let mut host = GameHost::new((*self.authoring.authoring_project.registry).clone());
        host.behavior_checkpoints = self.behavior_checkpoints.clone();
        host.ui_action_checkpoints = self.ui_action_checkpoints.clone();
        self.build.worker = Some(
            std::thread::Builder::new()
                .name("voxy-project-build".into())
                .spawn(move || {
                    let root = descriptor.parent().ok_or("Missing project directory.")?;
                    let output = (1..=1024)
                        .map(|number| {
                            root.join(if number == 1 {
                                "game.vpak".into()
                            } else {
                                format!("game-{number}.vpak")
                            })
                        })
                        .find(|path| {
                            path.symlink_metadata()
                                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
                        })
                        .ok_or("No free game package name in the project directory.")?;
                    export_project_package_with_host(&descriptor, &output, host)
                        .map_err(|error| error.to_string())?;
                    Ok(output)
                })
                .map_err(|error| error.to_string())?,
        );
        Ok(())
    }

    pub(super) fn poll_project_build(&mut self) {
        if self
            .build
            .worker
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            self.finish_project_build();
            self.update_edit_title();
        }
    }

    pub(super) fn finish_project_build(&mut self) {
        if let Some(worker) = self.build.worker.take() {
            let result = worker
                .join()
                .unwrap_or_else(|_| Err("Game build worker panicked.".into()));
            match &result {
                Ok(path) => println!("EDITOR GAME BUILT {}", path.display()),
                Err(error) => eprintln!("EDITOR GAME BUILD FAILED: {error}"),
            }
            self.build.result = Some(result);
        }
    }
}
