//! Unicode run preparation reuses the existing shaper/rasterizer and input owner.
use crate::{SceneUiSnapshot, UiText};
use std::{collections::BTreeMap, sync::Arc};
use voxy_assets::{AssetId, ImportInputs};
use voxy_scene::NodeId;
use voxy_text::{FontLimits, RunDirection, RunOptions, TextFont, TextRun};

/// Parses the same observed font bytes used for dependency publication.
/// # Errors
/// Rejects scoped reads, font input limits or invalid face-zero font data.
pub fn decode_ui_font_observed(
    inputs: &mut ImportInputs,
    source: AssetId,
    mut provider: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
) -> Result<TextFont, String> {
    let observed = inputs
        .read(source, |id, limit| provider(id, limit.min(4 * 1024 * 1024)))
        .map_err(|error| format!("UI font input: {error:?}"))?;
    TextFont::parse(
        &observed.bytes,
        FontLimits {
            max_font_bytes: 4 * 1024 * 1024,
            max_glyph_pixels: 16384,
            max_size: 128.0,
        },
    )
    .map_err(|error| format!("UI font: {error}"))
}
#[derive(Debug)]
pub struct PreparedUiText {
    pub owner: NodeId,
    pub origin: [f32; 2],
    pub clip: [f32; 4],
    pub source: UiText,
    /// Local baseline starts at (0,size). Equal font/content/size runs share it.
    pub run: Arc<TextRun>,
}
/// Stages all visible text with bounded runs and common Unicode shaping. Missing
/// fonts or invalid glyph/atlas admission return no partial publication. Run
/// preparation is CPU importer work, not a renderer/device callback operation.
/// # Errors
/// Rejects missing fonts, glyph/atlas/aggregate budgets and shaping errors.
pub fn prepare_ui_text<'a>(
    snapshot: &SceneUiSnapshot,
    mut font: impl FnMut(&str) -> Option<&'a TextFont>,
) -> Result<Vec<PreparedUiText>, String> {
    if snapshot.elements.len() > 128 {
        return Err("UI text element budget".into());
    }
    let mut text_bytes = 0_usize;
    for element in &snapshot.elements {
        if !element.descriptor.valid()
            || element.rect.iter().any(|value| !value.is_finite())
            || element.clip.is_some_and(|clip| {
                clip.iter().any(|value| !value.is_finite()) || clip[2] <= 0.0 || clip[3] <= 0.0
            })
        {
            return Err("invalid UI text snapshot".into());
        }
        text_bytes += element
            .descriptor
            .text
            .as_ref()
            .map_or(0, |text| text.content.len());
    }
    if text_bytes > 65536 {
        return Err("UI text source budget".into());
    }
    let mut cache = BTreeMap::new();
    let mut prepared = Vec::new();
    let mut total_glyphs = 0_usize;
    let mut atlas_pixels = 0_usize;
    for element in &snapshot.elements {
        let (Some(text), Some(clip)) = (&element.descriptor.text, element.clip) else {
            continue;
        };
        let key = (text.font.clone(), text.content.clone(), text.size.to_bits());
        let run = if let Some(run) = cache.get(&key) {
            Arc::clone(run)
        } else {
            let font = font(&text.font).ok_or_else(|| format!("missing UI font {}", text.font))?;
            let run = font
                .prepare(
                    &text.content,
                    [0.0, text.size],
                    RunOptions {
                        size: text.size,
                        direction: RunDirection::Guess,
                        max_text_bytes: 4096,
                        max_glyphs: 1024,
                        atlas_size: [512, 256],
                        max_atlas_pixels: 512 * 256,
                    },
                )
                .map_err(|error| format!("UI text: {error}"))?;
            atlas_pixels = atlas_pixels
                .checked_add(run.atlas().alpha().len())
                .ok_or("UI atlas budget overflow")?;
            if atlas_pixels > 16 * 1024 * 1024 {
                return Err("UI text aggregate budget".into());
            }
            let run = Arc::new(run);
            cache.insert(key, Arc::clone(&run));
            run
        };
        total_glyphs = total_glyphs
            .checked_add(run.glyphs().len())
            .ok_or("UI glyph budget overflow")?;
        if total_glyphs > 8192 {
            return Err("UI text aggregate budget".into());
        }
        prepared.push(PreparedUiText {
            owner: element.owner,
            origin: [element.rect[0], element.rect[1]],
            clip,
            source: text.clone(),
            run,
        });
    }
    Ok(prepared)
}
