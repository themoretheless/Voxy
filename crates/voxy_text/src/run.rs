use crate::{FontLimits, GlyphAtlas, GlyphRegion, RasterFont, RunDirection, ShapeFont, TextError};
use std::collections::{BTreeMap, btree_map::Entry};
#[derive(Clone, Copy, Debug)]
pub struct RunOptions {
    pub size: f32,
    pub direction: RunDirection,
    pub max_text_bytes: usize,
    pub max_glyphs: usize,
    pub atlas_size: [usize; 2],
    pub max_atlas_pixels: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub region: GlyphRegion,
    pub origin: [f32; 2],
    pub cluster: u32,
}
/// Immutable prepared directional run. Independent from font/scene/device owners.
#[derive(Debug)]
pub struct TextRun {
    atlas: GlyphAtlas,
    glyphs: Vec<PlacedGlyph>,
    advance: [f32; 2],
}
impl TextRun {
    #[must_use]
    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }
    #[must_use]
    pub fn glyphs(&self) -> &[PlacedGlyph] {
        &self.glyphs
    }
    #[must_use]
    pub fn advance(&self) -> [f32; 2] {
        self.advance
    }
}
/// Shaper and rasterizer always refer to the same face-zero font source.
#[derive(Debug)]
pub struct TextFont {
    shaper: ShapeFont,
    raster: RasterFont,
}
impl TextFont {
    /// # Errors
    /// Returns font parsing/size budget validation errors.
    pub fn parse(bytes: &[u8], limits: FontLimits) -> Result<Self, TextError> {
        Ok(Self {
            raster: RasterFont::parse(bytes, limits)?,
            shaper: ShapeFont::parse(bytes, limits.max_font_bytes)?,
        })
    }
    /// Prepares a complete run into private staging storage. Errors leave any
    /// caller-held prior `TextRun` intact. Baseline uses logical top-left coordinates.
    /// Whitespace advances the pen without a drawable glyph. Repeated glyph IDs
    /// share raster metrics and atlas regions within this font/size run.
    /// # Errors
    /// Returns shaping/raster/atlas caps or invalid/nonfinite placement geometry.
    pub fn prepare(
        &self,
        text: &str,
        baseline: [f32; 2],
        options: RunOptions,
    ) -> Result<TextRun, TextError> {
        if baseline.iter().any(|x| !x.is_finite()) {
            return Err(TextError::InvalidSize);
        }
        let shaped = self.shaper.shape(
            text,
            options.size,
            options.direction,
            options.max_text_bytes,
            options.max_glyphs,
        )?;
        let mut atlas = GlyphAtlas::new(
            options.atlas_size[0],
            options.atlas_size[1],
            options.max_atlas_pixels,
            options.max_glyphs,
        )?;
        let mut glyphs = Vec::with_capacity(shaped.len());
        let mut pen = [0.0_f64; 2];
        let mut cache = BTreeMap::new();
        for item in shaped {
            let cached = match cache.entry(item.id) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let bitmap = self.raster.rasterize_indexed(item.id, options.size)?;
                    let height = u32::try_from(bitmap.height).map_err(|_| TextError::Capacity)?;
                    let region = atlas.insert(&bitmap)?;
                    *entry.insert(CachedGlyph {
                        bearing: bitmap.bearing,
                        height,
                        region,
                    })
                }
            };
            let origin = placement(baseline, pen, item.offset, cached.bearing, cached.height)?;
            if let Some(region) = cached.region {
                glyphs.push(PlacedGlyph {
                    region,
                    origin,
                    cluster: item.cluster,
                });
            }
            for (value, advance) in pen.iter_mut().zip(item.advance) {
                *value += f64::from(advance);
            }
        }
        let advance = finite_pixels(pen)?;
        Ok(TextRun {
            atlas,
            glyphs,
            advance,
        })
    }
}
#[derive(Clone, Copy, Debug)]
struct CachedGlyph {
    bearing: [i32; 2],
    height: u32,
    region: Option<GlyphRegion>,
}
fn placement(
    baseline: [f32; 2],
    pen: [f64; 2],
    offset: [f32; 2],
    bearing: [i32; 2],
    height: u32,
) -> Result<[f32; 2], TextError> {
    finite_pixels([
        f64::from(baseline[0]) + pen[0] + f64::from(offset[0]) + f64::from(bearing[0]),
        f64::from(baseline[1])
            - pen[1]
            - f64::from(offset[1])
            - f64::from(bearing[1])
            - f64::from(height),
    ])
}
#[allow(clippy::cast_possible_truncation)] // Result validated after conversion.
fn finite_pixels(values: [f64; 2]) -> Result<[f32; 2], TextError> {
    let pixels = values.map(|x| x as f32);
    if pixels.iter().all(|x| x.is_finite()) {
        Ok(pixels)
    } else {
        Err(TextError::InvalidSize)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_offsets_and_descender_follow_y_down_coordinates() {
        let origin = placement([10.0, 40.0], [20.0, 3.0], [2.0, 4.0], [-1, -5], 12).unwrap();
        assert!((origin[0] - 31.0).abs() < f32::EPSILON);
        assert!((origin[1] - 26.0).abs() < f32::EPSILON);
        assert_eq!(finite_pixels([f64::MAX, 0.0]), Err(TextError::InvalidSize));
    }
}

/// Immutable run data, including exact stored pixel positions and atlas bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextRunCheckpoint {
    pub version: u32,
    pub atlas: crate::GlyphAtlasCheckpoint,
    pub glyphs: Vec<PlacedGlyphCheckpoint>,
    pub advance: [u32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedGlyphCheckpoint {
    pub origin: [u32; 2],
    pub region_origin: [usize; 2],
    pub region_size: [usize; 2],
    pub cluster: u32,
}
impl TextRun {
    pub fn checkpoint(&self) -> TextRunCheckpoint {
        let Self {
            atlas,
            glyphs,
            advance,
        } = self;
        TextRunCheckpoint {
            version: 1,
            atlas: atlas.checkpoint(),
            advance: advance.map(f32::to_bits),
            glyphs: glyphs
                .iter()
                .map(|glyph| {
                    let PlacedGlyph {
                        region: GlyphRegion { origin, size },
                        origin: position,
                        cluster,
                    } = glyph;
                    PlacedGlyphCheckpoint {
                        origin: position.map(f32::to_bits),
                        region_origin: *origin,
                        region_size: *size,
                        cluster: *cluster,
                    }
                })
                .collect(),
        }
    }
}
impl TextRunCheckpoint {
    pub fn prepare(&self, max_pixels: usize, max_glyphs: usize) -> Result<TextRun, TextError> {
        if self.glyphs.len() > max_glyphs {
            return Err(TextError::Capacity);
        }
        if self.version != 1
            || self
                .advance
                .map(f32::from_bits)
                .iter()
                .any(|v| !v.is_finite())
        {
            return Err(TextError::InvalidSize);
        }
        let atlas = self.atlas.prepare(max_pixels, max_glyphs)?;
        let dimensions = atlas.dimensions();
        let mut regions = std::collections::BTreeSet::new();
        let glyphs = self
            .glyphs
            .iter()
            .map(|glyph| {
                let origin = glyph.origin.map(f32::from_bits);
                if origin.iter().any(|v| !v.is_finite())
                    || (0..2).any(|k| {
                        glyph.region_origin[k] == 0
                            || glyph.region_size[k] == 0
                            || glyph.region_origin[k]
                                .checked_add(glyph.region_size[k])
                                .is_none_or(|end| end >= dimensions[k])
                    })
                {
                    return Err(TextError::InvalidBitmap);
                }
                regions.insert((glyph.region_origin, glyph.region_size));
                Ok(PlacedGlyph {
                    origin,
                    region: GlyphRegion {
                        origin: glyph.region_origin,
                        size: glyph.region_size,
                    },
                    cluster: glyph.cluster,
                })
            })
            .collect::<Result<Vec<_>, TextError>>()?;
        if regions.len() != self.atlas.entries {
            return Err(TextError::InvalidBitmap);
        }
        Ok(TextRun {
            atlas,
            glyphs,
            advance: self.advance.map(f32::from_bits),
        })
    }
}
/// Original face-zero source plus its raster admission policy. Accepted TextRuns
/// have independent records and are restored without re-shaping this font.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextFontCheckpoint {
    pub version: u32,
    pub bytes: Vec<u8>,
    pub max_font_bytes: usize,
    pub max_glyph_pixels: usize,
    pub max_size: u32,
}
impl TextFont {
    pub fn checkpoint(&self) -> TextFontCheckpoint {
        let Self { shaper, raster } = self;
        // Parsed font tables are derived from the same immutable source bytes.
        // Every other raster owner field must receive an explicit save decision.
        let crate::RasterFont { font: _, limits } = raster;
        let FontLimits {
            max_font_bytes,
            max_glyph_pixels,
            max_size,
        } = *limits;
        TextFontCheckpoint {
            version: 1,
            bytes: shaper.checkpoint_bytes(),
            max_font_bytes,
            max_glyph_pixels,
            max_size: max_size.to_bits(),
        }
    }
}
impl TextFontCheckpoint {
    pub fn prepare(&self, limits: FontLimits) -> Result<TextFont, TextError> {
        let max_size = f32::from_bits(self.max_size);
        if !limits.max_size.is_finite() || limits.max_size <= 0. {
            return Err(TextError::InvalidSize);
        }
        if self.bytes.len() > limits.max_font_bytes
            || self.max_font_bytes > limits.max_font_bytes
            || self.max_glyph_pixels > limits.max_glyph_pixels
            || max_size > limits.max_size
        {
            return Err(TextError::Capacity);
        }
        if self.version != 1 {
            return Err(TextError::InvalidFont);
        }
        TextFont::parse(
            &self.bytes,
            FontLimits {
                max_font_bytes: self.max_font_bytes,
                max_glyph_pixels: self.max_glyph_pixels,
                max_size,
            },
        )
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    fn font_bytes() -> Vec<u8> {
        [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .expect("Unicode TrueType font fixture required")
    }
    fn limits() -> FontLimits {
        FontLimits {
            max_font_bytes: 4 * 1024 * 1024,
            max_glyph_pixels: 16384,
            max_size: 128.,
        }
    }
    fn options() -> RunOptions {
        RunOptions {
            size: 18.5,
            direction: RunDirection::Guess,
            max_text_bytes: 4096,
            max_glyphs: 1024,
            atlas_size: [512, 256],
            max_atlas_pixels: 512 * 256,
        }
    }
    #[test]
    fn original_font_and_accepted_runs_rebuild_exact_pixels_and_continue_120_unicode_edits() {
        let bytes = font_bytes();
        let font = TextFont::parse(&bytes, limits()).unwrap();
        let saved = font.checkpoint();
        assert_eq!(saved.bytes, bytes);
        let restored_font = saved.prepare(limits()).unwrap();
        assert_eq!(saved, restored_font.checkpoint());
        for i in 0..120 {
            let text = ["office ffi", "A\u{301} café", "אבג", "سلام", "    "][i % 5];
            let mut opts = options();
            opts.size = 12.25 + (i % 7) as f32;
            let run = font.prepare(text, [-0., opts.size], opts).unwrap();
            let state = run.checkpoint();
            let accepted = state.prepare(512 * 256, 1024).unwrap();
            assert_eq!(accepted.checkpoint(), state);
            assert_eq!(
                restored_font
                    .prepare(text, [-0., opts.size], opts)
                    .unwrap()
                    .checkpoint(),
                state
            );
        }
        let run = font
            .prepare("counter", [0., 18.5], options())
            .unwrap()
            .checkpoint();
        assert!(run.prepare(512 * 256 - 1, 1024).is_err());
        assert!(run.prepare(512 * 256, 1023).is_err());
        let mut bad = run.clone();
        bad.glyphs[0].origin[0] = f32::NAN.to_bits();
        assert!(bad.prepare(512 * 256, 1024).is_err());
        bad = run.clone();
        bad.glyphs[0].region_origin[0] = usize::MAX;
        assert!(bad.prepare(512 * 256, 1024).is_err());
        bad = run;
        bad.atlas.entries += 1;
        assert!(bad.prepare(512 * 256, 1024).is_err());
        assert!(
            saved
                .prepare(FontLimits {
                    max_font_bytes: saved.bytes.len() - 1,
                    ..limits()
                })
                .is_err()
        );
        assert!(
            saved
                .prepare(FontLimits {
                    max_size: 64.,
                    ..limits()
                })
                .is_err()
        );
    }
}
