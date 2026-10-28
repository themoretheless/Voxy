//! Resource diagnostics derive from the existing catalog and manifest inventory.
use crate::*;
impl App {
    pub(crate) fn selected_model_notice(&self) -> Option<panels::ResourceNotice> {
        if let Some(notice) = self.snapshot_path_notice() {
            return Some(notice);
        }
        if let Some(notice) = self.snapshot_failure_notice() {
            return Some(notice);
        }
        let node = *self.instances.get(self.selected)?;
        let model = self.scene.component::<ModelInstance>(node).ok()??;
        let retained = self.catalog.snapshot(&model.asset).is_some();
        let id: String = model.asset.0.chars().take(96).collect();
        let relink = self.model_relink.is_none()
            && self.play.playing.is_none()
            && self.resource_checkpoint.is_none()
            && self.restore_barrier.is_none()
            && matches!(self.import_recipe, InputRecipe::Manifest(_));
        if let Some(draft) = self
            .model_relink_draft
            .as_ref()
            .filter(|draft| draft.valid(self))
        {
            let tail: String = draft
                .text
                .chars()
                .rev()
                .take(90)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            let hint = self
                .model_relink_error
                .as_ref()
                .filter(|(asset, _)| asset == &model.asset)
                .map_or_else(
                    || "Project-relative path; Enter applies, Escape cancels".into(),
                    |(_, error)| error.chars().take(96).collect(),
                );
            return Some(panels::ResourceNotice {
                lines: [
                    format!("Relink model: {id}"),
                    format!("Path: {tail}|"),
                    hint,
                ],
                retry: false,
                relink,
                editing: true,
            });
        }
        if self
            .model_relink
            .as_ref()
            .is_some_and(|job| job.asset == model.asset)
        {
            return Some(panels::ResourceNotice {
                lines: [
                    format!("Relinking model: {id}"),
                    "Checking source and model compatibility in background".into(),
                    "Scene references and last good model are retained".into(),
                ],
                retry: false,
                relink: false,
                editing: false,
            });
        }
        if let Some((_, error)) = self
            .model_relink_error
            .as_ref()
            .filter(|(asset, _)| asset == &model.asset)
        {
            return Some(panels::ResourceNotice {
                lines: [
                    format!("Model relink failed: {id}"),
                    error.chars().take(96).collect(),
                    "Correct the project-relative source path and retry".into(),
                ],
                retry: false,
                relink,
                editing: false,
            });
        }
        let (title, detail, help) = if !self.available.contains(&model.asset) {
            (
                format!("Model binding missing: {id}"),
                if retained {
                    "Showing last good model; scene reference kept"
                } else {
                    "Model unavailable; scene reference kept"
                }
                .to_owned(),
                "Restore manifest binding or click Model to choose an asset",
            )
        } else if let Some(voxy_assets::AssetStatus::Failed(error)) =
            self.catalog.status(&model.asset)
        {
            (
                format!("Model import failed: {id}"),
                format!(
                    "{}{}",
                    if retained { "Last good model: " } else { "" },
                    error.chars().take(96).collect::<String>()
                ),
                "File changes trigger automatic reload; or click Retry import",
            )
        } else {
            return None;
        };
        Some(panels::ResourceNotice {
            lines: [title, detail, help.into()],
            retry: self.can_retry_selected_model(),
            relink,
            editing: false,
        })
    }
}
