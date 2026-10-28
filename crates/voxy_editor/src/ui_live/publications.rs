//! Portable CPU publication participant. Workers and native presentation fences
//! belong to the enclosing UiLive save coordinator, not this archive.
#![allow(dead_code)]
use super::*;
use crate::EditorError;
use serde::{Deserialize, Serialize};
use voxy_assets::ImportedAsset;
use voxy_gameplay::SceneUiSnapshotCheckpoint;
use voxy_scene::SceneCheckpointView;
use voxy_text::TextRun;

type Text = Arc<ImportedAsset<ui_text::UiTextPreparation>>;
const HEADER: usize = 56;

#[derive(Clone, Copy, Debug)]
pub(crate) struct UiPublicationLimits {
    pub max_bytes: usize,
    pub max_layout_bytes: usize,
    pub max_elements: usize,
    pub max_input_bytes: usize,
    pub max_pixels: usize,
    pub max_glyphs: usize,
    pub max_runs: usize,
    pub text: ui_text::checkpoint::UiTextArchiveLimits,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Visible {
    layout: SceneUiSnapshotCheckpoint,
    text: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Publication {
    bytes: usize,
    // Local distinct run order -> bundle allocation identity, never content hash.
    run_ids: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    attempted: Option<SceneUiSnapshotCheckpoint>,
    published: Option<Visible>,
    presented: Option<Visible>,
    publications: Vec<Publication>,
}
#[derive(Clone, Debug)]
pub(crate) struct UiPublicationsCheckpoint {
    wire: Wire,
    archives: Vec<Vec<u8>>,
}
#[derive(Debug)]
pub(crate) struct PreparedUiPublications {
    pub(super) attempted: Option<SceneUiSnapshot>,
    pub(super) published: Option<SceneUiSnapshot>,
    pub(super) presented: Option<SceneUiSnapshot>,
    pub(super) published_text: Option<Text>,
    pub(super) presented_text: Option<Text>,
}

#[derive(Clone, Copy)]
struct Views<'a> {
    attempted: Option<&'a SceneUiSnapshot>,
    published: Option<&'a SceneUiSnapshot>,
    presented: Option<&'a SceneUiSnapshot>,
    published_text: Option<&'a Text>,
    presented_text: Option<&'a Text>,
}
impl UiLive {
    pub(crate) fn publication_checkpoint(
        &self,
        view: &SceneCheckpointView<'_>,
        limits: UiPublicationLimits,
    ) -> Result<UiPublicationsCheckpoint, EditorError> {
        // Exhaustive field inventory; these operational fields require a
        // separate drain/watch/cache coordinator before a complete UiLive save.
        let Self {
            attempted,
            published,
            presented,
            published_text,
            presented_text,
            worker: _,
            retired: _,
            deferred: _,
            sources: _,
            watcher: _,
            scanning: _,
            last_scan: _,
            checkpoint: _,
            awaiting_first_presentation: _,
        } = self;
        capture(
            Views {
                attempted: attempted.as_ref(),
                published: published.as_ref(),
                presented: presented.as_ref(),
                published_text: published_text.as_ref(),
                presented_text: presented_text.as_ref(),
            },
            view,
            limits,
        )
    }
}
impl PreparedUiPublications {
    pub(crate) fn checkpoint(
        &self,
        view: &SceneCheckpointView<'_>,
        limits: UiPublicationLimits,
    ) -> Result<UiPublicationsCheckpoint, EditorError> {
        let Self {
            attempted,
            published,
            presented,
            published_text,
            presented_text,
        } = self;
        capture(
            Views {
                attempted: attempted.as_ref(),
                published: published.as_ref(),
                presented: presented.as_ref(),
                published_text: published_text.as_ref(),
                presented_text: presented_text.as_ref(),
            },
            view,
            limits,
        )
    }
    pub(crate) fn meshes(
        &self,
    ) -> Result<(Vec<ui_draw::UiDrawMesh>, Vec<ui_draw::UiDrawMesh>), EditorError> {
        Ok((
            self.published_meshes()?,
            meshes(self.presented.as_ref(), self.presented_text.as_ref())?,
        ))
    }
    pub(crate) fn published_meshes(&self) -> Result<Vec<ui_draw::UiDrawMesh>, EditorError> {
        meshes(self.published.as_ref(), self.published_text.as_ref())
    }
}
fn meshes(
    layout: Option<&SceneUiSnapshot>,
    text: Option<&Text>,
) -> Result<Vec<ui_draw::UiDrawMesh>, EditorError> {
    match (layout, text) {
        (Some(layout), text) => ui_draw::build(layout, text.map_or(&[], |text| &text.value().runs)),
        (None, None) => Ok(Vec::new()),
        (None, Some(_)) => Err("UI resource publication has no layout".into()),
    }
}
fn capture(
    views: Views<'_>,
    view: &SceneCheckpointView<'_>,
    limits: UiPublicationLimits,
) -> Result<UiPublicationsCheckpoint, EditorError> {
    meshes(views.published, views.published_text)?;
    meshes(views.presented, views.presented_text)?;
    let mut resources: Vec<Text> = Vec::new();
    let mut pool: Vec<Arc<TextRun>> = Vec::new();
    let mut archives = Vec::new();
    let mut publications = Vec::new();
    let mut visible = |layout: Option<&SceneUiSnapshot>, text: Option<&Text>| {
        let Some(layout) = layout else {
            return Ok(None);
        };
        let text = text
            .map(|text| {
                if let Some(id) = resources.iter().position(|old| Arc::ptr_eq(old, text)) {
                    return Ok(id);
                }
                let mut local = Vec::<Arc<TextRun>>::new();
                let mut run_ids = Vec::new();
                for label in &text.value().runs {
                    if local.iter().any(|old| Arc::ptr_eq(old, &label.run)) {
                        continue;
                    }
                    let id = pool
                        .iter()
                        .position(|old| Arc::ptr_eq(old, &label.run))
                        .unwrap_or_else(|| {
                            pool.push(label.run.clone());
                            pool.len() - 1
                        });
                    local.push(label.run.clone());
                    run_ids.push(id);
                }
                publications.push(Publication { bytes: 0, run_ids });
                resources.push(text.clone());
                Ok::<_, EditorError>(resources.len() - 1)
            })
            .transpose()?;
        Ok::<_, EditorError>(Some(Visible {
            layout: capture_layout(layout, view, limits)?,
            text,
        }))
    };
    let published = visible(views.published, views.published_text)?;
    let presented = visible(views.presented, views.presented_text)?;
    admit(&resources, &pool, limits)?;
    for (text, record) in resources.iter().zip(&mut publications) {
        let archive = ui_text::checkpoint::encode(
            text,
            |node| view.reserved_identity(node).cloned(),
            limits.text,
        )?;
        record.bytes = archive.len();
        archives.push(archive);
    }
    let saved = UiPublicationsCheckpoint {
        wire: Wire {
            version: 1,
            attempted: views
                .attempted
                .map(|layout| capture_layout(layout, view, limits))
                .transpose()?,
            published,
            presented,
            publications,
        },
        archives,
    };
    saved.to_bytes(limits)?;
    Ok(saved)
}
fn capture_layout(
    layout: &SceneUiSnapshot,
    view: &SceneCheckpointView<'_>,
    limits: UiPublicationLimits,
) -> Result<SceneUiSnapshotCheckpoint, EditorError> {
    let saved = layout.checkpoint(
        |node| view.reserved_identity(node).cloned(),
        limits.max_layout_bytes,
    )?;
    saved.prepare_historical(view, limits.max_elements, limits.max_layout_bytes)?;
    Ok(saved)
}
fn admit(
    resources: &[Text],
    pool: &[Arc<TextRun>],
    limits: UiPublicationLimits,
) -> Result<(), EditorError> {
    let mut bytes = 0_usize;
    let mut glyphs = 0_usize;
    let mut pixels = 0_usize;
    for text in resources {
        for input in text.inputs().observations().values() {
            bytes = bytes
                .checked_add(
                    input
                        .as_ref()
                        .map_err(|_| "failed UI publication input")?
                        .bytes
                        .len(),
                )
                .ok_or("UI bundle input overflow")?;
        }
        for label in &text.value().runs {
            glyphs = glyphs
                .checked_add(label.run.glyphs().len())
                .ok_or("UI bundle glyph overflow")?;
        }
    }
    for run in pool {
        pixels = pixels
            .checked_add(run.atlas().alpha().len())
            .ok_or("UI bundle pixel overflow")?;
    }
    if resources.len() > 2
        || bytes > limits.max_input_bytes
        || pixels > limits.max_pixels
        || glyphs > limits.max_glyphs
        || pool.len() > limits.max_runs
    {
        return Err("UI bundle aggregate resource budget".into());
    }
    Ok(())
}

impl UiPublicationsCheckpoint {
    /// Checksummed metadata followed by the existing independent resource archives.
    /// No source IO or GPU work is performed, and no live owner is changed.
    pub(crate) fn to_bytes(&self, limits: UiPublicationLimits) -> Result<Vec<u8>, EditorError> {
        if self.wire.version != 1
            || self.archives.len() > 2
            || self.archives.len() != self.wire.publications.len()
        {
            return Err("invalid UI bundle publications".into());
        }
        let metadata = serde_json::to_vec(&self.wire)?;
        let mut size = HEADER
            .checked_add(metadata.len())
            .ok_or("UI bundle byte overflow")?;
        for (archive, publication) in self.archives.iter().zip(&self.wire.publications) {
            if archive.len() != publication.bytes
                || archive.len() > limits.text.imported.max_archive_bytes
                || publication.run_ids.len() > limits.text.max_runs
            {
                return Err("invalid UI bundle resource record".into());
            }
            size = size
                .checked_add(archive.len())
                .ok_or("UI bundle byte overflow")?;
        }
        if size > limits.max_bytes {
            return Err("UI bundle byte budget".into());
        }
        let mut body = metadata;
        let metadata_len = body.len();
        for archive in &self.archives {
            body.extend_from_slice(archive);
        }
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(b"VOXYUIP1");
        bytes.extend_from_slice(&u64::try_from(metadata_len)?.to_le_bytes());
        bytes.extend_from_slice(&u64::try_from(body.len())?.to_le_bytes());
        bytes.extend_from_slice(blake3::hash(&body).as_bytes());
        bytes.extend_from_slice(&body);
        Ok(bytes)
    }
    pub(crate) fn from_bytes(
        bytes: &[u8],
        limits: UiPublicationLimits,
    ) -> Result<Self, EditorError> {
        if bytes.len() < HEADER || bytes.len() > limits.max_bytes || &bytes[..8] != b"VOXYUIP1" {
            return Err("invalid UI bundle header or budget".into());
        }
        let metadata_len = usize::try_from(u64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| "invalid UI bundle metadata length")?,
        ))?;
        let body_len = usize::try_from(u64::from_le_bytes(
            bytes[16..24]
                .try_into()
                .map_err(|_| "invalid UI bundle body length")?,
        ))?;
        let body = &bytes[HEADER..];
        if body_len != body.len()
            || metadata_len > body_len
            || blake3::hash(body).as_bytes() != &bytes[24..56]
        {
            return Err("corrupt UI bundle".into());
        }
        let wire: Wire = serde_json::from_slice(&body[..metadata_len])?;
        if wire.version != 1 || wire.publications.len() > 2 {
            return Err("unsupported UI bundle version or publication count".into());
        }
        let mut offset = metadata_len;
        let mut archives = Vec::new();
        for publication in &wire.publications {
            if publication.bytes > limits.text.imported.max_archive_bytes
                || publication.run_ids.len() > limits.text.max_runs
            {
                return Err("UI bundle resource record budget".into());
            }
            let end = offset
                .checked_add(publication.bytes)
                .ok_or("UI bundle byte overflow")?;
            let archive = body
                .get(offset..end)
                .ok_or("truncated UI bundle publication")?;
            archives.push(archive.to_vec());
            offset = end;
        }
        if offset != body.len() {
            return Err("trailing UI bundle bytes".into());
        }
        Ok(Self { wire, archives })
    }
    pub(crate) fn prepare(
        &self,
        view: &SceneCheckpointView<'_>,
        limits: UiPublicationLimits,
    ) -> Result<PreparedUiPublications, EditorError> {
        self.to_bytes(limits)?;
        let mut used = std::collections::BTreeSet::new();
        for visible in [&self.wire.published, &self.wire.presented]
            .into_iter()
            .flatten()
        {
            if let Some(id) = visible.text {
                if id >= self.archives.len() || (used.insert(id) && id != used.len() - 1) {
                    return Err("invalid UI bundle publication reference".into());
                }
            }
        }
        if used.len() != self.archives.len() {
            return Err("unreferenced UI bundle publication".into());
        }
        let mut resources = Vec::new();
        let mut pool = Vec::new();
        for (archive, record) in self.archives.iter().zip(&self.wire.publications) {
            resources.push(Arc::new(ui_text::checkpoint::decode_historical_interned(
                archive,
                view,
                limits.text,
                &record.run_ids,
                &mut pool,
            )?));
            // Reject aggregate exhaustion after each resource, not after all fonts.
            admit(&resources, &pool, limits)?;
        }
        let visible = |saved: &Option<Visible>| {
            saved
                .as_ref()
                .map(|saved| {
                    Ok::<_, EditorError>((
                        saved.layout.prepare_historical(
                            view,
                            limits.max_elements,
                            limits.max_layout_bytes,
                        )?,
                        saved.text.map(|id| resources[id].clone()),
                    ))
                })
                .transpose()
        };
        let published = visible(&self.wire.published)?;
        let presented = visible(&self.wire.presented)?;
        let prepared = PreparedUiPublications {
            attempted: self
                .wire
                .attempted
                .as_ref()
                .map(|saved| {
                    saved.prepare_historical(view, limits.max_elements, limits.max_layout_bytes)
                })
                .transpose()?,
            published: published.as_ref().map(|(layout, _)| layout.clone()),
            published_text: published.and_then(|(_, text)| text),
            presented: presented.as_ref().map(|(layout, _)| layout.clone()),
            presented_text: presented.and_then(|(_, text)| text),
        };
        prepared.meshes()?;
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests;
