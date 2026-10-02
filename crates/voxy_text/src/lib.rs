//! Bounded Unicode glyph rasterization. Shaping and layout are separate layers.
mod run;
pub use run::{PlacedGlyph, RunOptions, TextFont, TextRun};
mod shape;
pub use shape::{RunDirection, ShapeFont, ShapedGlyph};
mod atlas;
pub use atlas::{GlyphAtlas, GlyphRegion};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextError {
    InvalidFont,
    InvalidBitmap,
    InvalidSize,
    MissingGlyph,
    Capacity,
}
impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "text error: {self:?}")
    }
}
impl std::error::Error for TextError {}
/// Per-glyph pixel budget and maximum raster size; font parsing has a byte cap.
#[derive(Clone, Copy, Debug)]
pub struct FontLimits {
    pub max_font_bytes: usize,
    pub max_glyph_pixels: usize,
    pub max_size: f32,
}
/// Alpha coverage, top-to-bottom rows. Bearing is the bottom-left offset from
/// the baseline in font coordinates (positive Y upwards).
#[derive(Debug)]
pub struct GlyphBitmap {
    pub width: usize,
    pub height: usize,
    pub bearing: [i32; 2],
    pub advance: f32,
    pub alpha: Vec<u8>,
}
#[derive(Debug)]
pub struct RasterFont {
    font: fontdue::Font,
    limits: FontLimits,
}
impl RasterFont {
    /// # Errors
    /// Rejects oversized/invalid font bytes or invalid raster-size limits.
    pub fn parse(bytes: &[u8], limits: FontLimits) -> Result<Self, TextError> {
        if bytes.len() > limits.max_font_bytes {
            return Err(TextError::Capacity);
        }
        if !limits.max_size.is_finite() || limits.max_size <= 0.0 {
            return Err(TextError::InvalidSize);
        }
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|_| TextError::InvalidFont)?;
        Ok(Self { font, limits })
    }
    /// # Errors
    /// Rejects unsupported characters, invalid sizes and predicted bitmap sizes
    /// over the cap before allocating raster storage. Whitespace may be empty.
    pub fn rasterize(&self, character: char, size: f32) -> Result<GlyphBitmap, TextError> {
        if !size.is_finite() || size <= 0.0 || size > self.limits.max_size {
            return Err(TextError::InvalidSize);
        }
        let index = self.font.lookup_glyph_index(character);
        if index == 0 {
            return Err(TextError::MissingGlyph);
        }
        self.rasterize_indexed(index, size)
    }
    /// Rasterizes a glyph ID returned by a shaper using this same font face.
    /// # Errors
    /// Rejects invalid glyph ID/size and bitmap capacity before raster allocation.
    pub fn rasterize_indexed(&self, index: u16, size: f32) -> Result<GlyphBitmap, TextError> {
        if !size.is_finite() || size <= 0.0 || size > self.limits.max_size {
            return Err(TextError::InvalidSize);
        }
        if index == 0 || index >= self.font.glyph_count() {
            return Err(TextError::MissingGlyph);
        }
        let metrics = self.font.metrics_indexed(index, size);
        if metrics
            .width
            .checked_mul(metrics.height)
            .is_none_or(|pixels| pixels > self.limits.max_glyph_pixels)
        {
            return Err(TextError::Capacity);
        }
        let (metrics, alpha) = self.font.rasterize_indexed(index, size);
        Ok(GlyphBitmap {
            width: metrics.width,
            height: metrics.height,
            bearing: [metrics.xmin, metrics.ymin],
            advance: metrics.advance_width,
            alpha,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_font_and_limits_are_explicit() {
        let limits = FontLimits {
            max_font_bytes: 4,
            max_glyph_pixels: 16,
            max_size: 32.0,
        };
        assert!(matches!(
            RasterFont::parse(&[0; 5], limits),
            Err(TextError::Capacity)
        ));
        assert!(matches!(
            RasterFont::parse(&[0; 4], limits),
            Err(TextError::InvalidFont)
        ));
        assert!(matches!(
            RasterFont::parse(
                &[],
                FontLimits {
                    max_size: f32::NAN,
                    ..limits
                }
            ),
            Err(TextError::InvalidSize)
        ));
    }
}
