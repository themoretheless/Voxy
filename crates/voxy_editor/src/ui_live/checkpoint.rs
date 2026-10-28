//! Composed portable UiLive participant. Native GPU staging/installation and the
//! enclosing App commit still require their own validated frame-boundary permit.
#![allow(dead_code)]
use super::{publications::*, reload_checkpoint::*, *};
use crate::EditorError;
use voxy_scene::SceneCheckpointView;

const HEADER: usize = 56;
#[derive(Clone, Copy, Debug)]
pub(crate) struct UiLiveCheckpointLimits {
    pub max_bytes: usize,
    pub max_reload_bytes: usize,
    pub publications: UiPublicationLimits,
}
#[derive(Clone, Debug)]
pub(crate) struct UiLiveCheckpoint {
    publications: UiPublicationsCheckpoint,
    reload: UiReloadCheckpoint,
}
#[derive(Debug)]
pub(crate) struct PreparedUiLive {
    pub(super) scene: voxy_scene::SceneChangeToken,
    pub(super) publications: PreparedUiPublications,
    pub(super) reload: PreparedUiReload,
}
impl PreparedUiLive {
    pub(crate) fn close(self) -> Option<std::thread::JoinHandle<()>> {
        self.reload.close()
    }
}
impl UiLive {
    /// Call at the App's save frame boundary after poll_checkpoint returned Ready.
    /// A normal main-thread frame ACK cannot interleave these immutable captures.
    pub(crate) fn checkpoint(
        &self,
        view: &SceneCheckpointView<'_>,
        limits: UiLiveCheckpointLimits,
    ) -> Result<UiLiveCheckpoint, EditorError> {
        let Some(Barrier::Ready(reload)) = &self.checkpoint else {
            return Err("UI save requires a completed reload barrier".into());
        };
        if self.worker.is_some() || self.scanning || !self.retired.is_empty() {
            return Err("UI save still has outstanding work".into());
        }
        let saved = UiLiveCheckpoint {
            publications: self.publication_checkpoint(view, limits.publications)?,
            reload: (**reload).clone(),
        };
        saved.to_bytes(limits)?;
        Ok(saved)
    }
}
impl UiLiveCheckpoint {
    pub(crate) fn to_bytes(&self, limits: UiLiveCheckpointLimits) -> Result<Vec<u8>, EditorError> {
        let publications = self.publications.to_bytes(limits.publications)?;
        let reload = self.reload.to_json(limits.max_reload_bytes)?;
        let body_len = publications
            .len()
            .checked_add(reload.len())
            .ok_or("UI save byte overflow")?;
        let size = HEADER
            .checked_add(body_len)
            .ok_or("UI save byte overflow")?;
        if size > limits.max_bytes {
            return Err("UI save byte budget".into());
        }
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(b"VOXYUIL1");
        bytes.extend_from_slice(&u64::try_from(publications.len())?.to_le_bytes());
        bytes.extend_from_slice(&u64::try_from(body_len)?.to_le_bytes());
        let mut hash = blake3::Hasher::new();
        hash.update(&publications);
        hash.update(reload.as_bytes());
        bytes.extend_from_slice(hash.finalize().as_bytes());
        bytes.extend_from_slice(&publications);
        bytes.extend_from_slice(reload.as_bytes());
        Ok(bytes)
    }
    pub(crate) fn from_bytes(
        bytes: &[u8],
        limits: UiLiveCheckpointLimits,
    ) -> Result<Self, EditorError> {
        if bytes.len() < HEADER || bytes.len() > limits.max_bytes || &bytes[..8] != b"VOXYUIL1" {
            return Err("invalid UI save header or budget".into());
        }
        let length = |range: std::ops::Range<usize>| -> Result<usize, EditorError> {
            Ok(usize::try_from(u64::from_le_bytes(
                bytes[range]
                    .try_into()
                    .map_err(|_| "invalid UI save length")?,
            ))?)
        };
        let publication_len = length(8..16)?;
        let body_len = length(16..24)?;
        let body = &bytes[HEADER..];
        if body_len != body.len()
            || publication_len > body_len
            || blake3::hash(body).as_bytes() != &bytes[24..56]
        {
            return Err("corrupt UI save".into());
        }
        let publications =
            UiPublicationsCheckpoint::from_bytes(&body[..publication_len], limits.publications)?;
        let reload = UiReloadCheckpoint::from_json(
            std::str::from_utf8(&body[publication_len..]).map_err(|_| "invalid UI reload UTF-8")?,
            limits.max_reload_bytes,
        )?;
        Ok(Self {
            publications,
            reload,
        })
    }
    /// Caller supplies the final cache state after private GPU staging. This
    /// constructs an idle watcher only after all CPU publication checks succeed.
    /// It never publishes an owner or claims a restored frame was displayed.
    pub(crate) fn prepare(
        &self,
        view: &SceneCheckpointView<'_>,
        project: &AuthoringProject,
        restored_cache: (u64, u64, u64, u64),
        limits: UiLiveCheckpointLimits,
    ) -> Result<PreparedUiLive, EditorError> {
        self.to_bytes(limits)?;
        let publications = self.publications.prepare(view, limits.publications)?;
        let reload = self.reload.prepare(project, restored_cache)?;
        Ok(PreparedUiLive {
            scene: view.graph.change_token(),
            publications,
            reload,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn limits() -> UiLiveCheckpointLimits {
        UiLiveCheckpointLimits {
            max_bytes: 100000,
            max_reload_bytes: 10000,
            publications: UiPublicationLimits {
                max_bytes: 80000,
                max_layout_bytes: 10000,
                max_elements: 128,
                max_input_bytes: 0,
                max_pixels: 0,
                max_glyphs: 0,
                max_runs: 0,
                text: ui_text::checkpoint::UiTextArchiveLimits {
                    imported: voxy_assets::ImportedArchiveLimits {
                        max_inputs: 0,
                        max_input_bytes: 0,
                        max_archive_bytes: 1000,
                    },
                    max_payload_bytes: 1000,
                    max_fonts: 0,
                    max_runs: 0,
                    max_pixels: 0,
                    max_glyphs_per_run: 0,
                    max_total_glyphs: 0,
                    font: voxy_text::FontLimits {
                        max_font_bytes: 0,
                        max_glyph_pixels: 0,
                        max_size: 128.,
                    },
                },
            },
        }
    }
    #[test]
    fn composed_save_requires_ready_barrier_preserves_generations_and_has_no_live_side_effects() {
        let scene = SceneGraph::new(4);
        let view = SceneCheckpointView::new(&scene, []).unwrap();
        let mut ui = UiLive {
            attempted: Some(SceneUiSnapshot {
                viewport: [128.; 2],
                elements: Vec::new(),
            }),
            published: Some(SceneUiSnapshot {
                viewport: [256.; 2],
                elements: Vec::new(),
            }),
            presented: Some(SceneUiSnapshot {
                viewport: [512.; 2],
                elements: Vec::new(),
            }),
            ..UiLive::default()
        };
        assert!(ui.checkpoint(&view, limits()).is_err());
        ui.begin_checkpoint().unwrap();
        assert!(ui.checkpoint(&view, limits()).is_err());
        assert!(ui.advance_checkpoint().unwrap().is_some());
        let saved = ui.checkpoint(&view, limits()).unwrap();
        let bytes = saved.to_bytes(limits()).unwrap();
        let saved = UiLiveCheckpoint::from_bytes(&bytes, limits()).unwrap();
        let project =
            AuthoringProject::new(&std::env::temp_dir(), &crate::InputRecipe::Direct).unwrap();
        let prepared = saved
            .prepare(&view, &project, (1, 2, 3, 4), limits())
            .unwrap();
        assert_eq!(prepared.publications.attempted, ui.attempted);
        assert_eq!(prepared.publications.published, ui.published);
        assert_eq!(prepared.publications.presented, ui.presented);
        assert!(prepared.reload.sources.is_none());
        assert!(prepared.reload.watcher.is_none());
        assert!(prepared.close().is_none());
        assert_eq!(
            ui.checkpoint(&view, limits())
                .unwrap()
                .to_bytes(limits())
                .unwrap(),
            bytes
        );
        assert!(ui.resume_after_checkpoint().unwrap());
        assert!(ui.checkpoint(&view, limits()).is_err());
    }
    #[test]
    fn composed_envelope_rejects_corruption_and_late_operational_failure_without_replacing_source()
    {
        let scene = SceneGraph::new(4);
        let view = SceneCheckpointView::new(&scene, []).unwrap();
        let mut ui = UiLive::default();
        ui.begin_checkpoint().unwrap();
        ui.advance_checkpoint().unwrap().unwrap();
        let bytes = ui
            .checkpoint(&view, limits())
            .unwrap()
            .to_bytes(limits())
            .unwrap();
        for cut in [0, 7, 55, bytes.len() - 1] {
            assert!(UiLiveCheckpoint::from_bytes(&bytes[..cut], limits()).is_err());
        }
        for index in [0, 8, 16, 24, bytes.len() - 1] {
            let mut bad = bytes.clone();
            bad[index] ^= 1;
            assert!(UiLiveCheckpoint::from_bytes(&bad, limits()).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(UiLiveCheckpoint::from_bytes(&trailing, limits()).is_err());
        let mut tight = limits();
        tight.max_bytes = bytes.len() - 1;
        assert!(UiLiveCheckpoint::from_bytes(&bytes, tight).is_err());
        assert!(ui.checkpoint(&view, tight).is_err());
        let publication_len =
            usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap())).unwrap();
        let mut reload: serde_json::Value =
            serde_json::from_slice(&bytes[HEADER + publication_len..]).unwrap();
        reload["version"] = serde_json::json!(99);
        let reload = serde_json::to_vec(&reload).unwrap();
        let mut body = bytes[HEADER..HEADER + publication_len].to_vec();
        body.extend_from_slice(&reload);
        let mut bad = bytes[..24].to_vec();
        bad[16..24].copy_from_slice(&u64::try_from(body.len()).unwrap().to_le_bytes());
        bad.extend_from_slice(blake3::hash(&body).as_bytes());
        bad.extend_from_slice(&body);
        let bad = UiLiveCheckpoint::from_bytes(&bad, limits()).unwrap();
        let project =
            AuthoringProject::new(&std::env::temp_dir(), &crate::InputRecipe::Direct).unwrap();
        assert!(
            bad.prepare(&view, &project, (1, 2, 3, 4), limits())
                .is_err()
        );
        assert_eq!(
            ui.checkpoint(&view, limits())
                .unwrap()
                .to_bytes(limits())
                .unwrap(),
            bytes
        );
        assert!(ui.close().is_empty());
    }
}
