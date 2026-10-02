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
