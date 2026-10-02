use crate::{GlyphBitmap, TextError};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphRegion {
    pub origin: [usize; 2],
    pub size: [usize; 2],
}
/// Fixed alpha texture, append-only shelf packing with one transparent pixel
/// around every glyph. No eviction/repacking: published regions stay stable.
#[derive(Debug)]
pub struct GlyphAtlas {
    width: usize,
    height: usize,
    alpha: Vec<u8>,
    x: usize,
    y: usize,
    row_height: usize,
    entries: usize,
    max_entries: usize,
}
impl GlyphAtlas {
    /// # Errors
    /// Rejects zero dimensions, arithmetic overflow or texture pixel budget excess.
    pub fn new(
        width: usize,
        height: usize,
        max_pixels: usize,
        max_entries: usize,
    ) -> Result<Self, TextError> {
        let count = width.checked_mul(height).ok_or(TextError::Capacity)?;
        if width == 0 || height == 0 {
            return Err(TextError::InvalidSize);
        }
        if count > max_pixels {
            return Err(TextError::Capacity);
        }
        Ok(Self {
            width,
            height,
            alpha: vec![0; count],
            x: 0,
            y: 0,
            row_height: 0,
            entries: 0,
            max_entries,
        })
    }
    #[must_use]
    pub fn dimensions(&self) -> [usize; 2] {
        [self.width, self.height]
    }
    #[must_use]
    pub fn alpha(&self) -> &[u8] {
        &self.alpha
    }
    #[must_use]
    pub fn entries(&self) -> usize {
        self.entries
    }
    /// Empty glyphs (e.g. spaces) return None without using atlas space.
    /// All validation and fit checks precede mutation.
    /// # Errors
    /// Rejects malformed bitmaps, entry capacity or insufficient texture space.
    pub fn insert(&mut self, glyph: &GlyphBitmap) -> Result<Option<GlyphRegion>, TextError> {
        if glyph.width.checked_mul(glyph.height) != Some(glyph.alpha.len()) {
            return Err(TextError::InvalidBitmap);
        }
        if glyph.width == 0 || glyph.height == 0 {
            return Ok(None);
        }
        if self.entries >= self.max_entries {
            return Err(TextError::Capacity);
        }
        let width = glyph.width.checked_add(2).ok_or(TextError::Capacity)?;
        let height = glyph.height.checked_add(2).ok_or(TextError::Capacity)?;
        if width > self.width || height > self.height {
            return Err(TextError::Capacity);
        }
        let mut x = self.x;
        let mut y = self.y;
        let mut row_height = self.row_height;
        if x.checked_add(width).is_none_or(|end| end > self.width) {
            x = 0;
            y = y.checked_add(row_height).ok_or(TextError::Capacity)?;
            row_height = 0;
        }
        if y.checked_add(height).is_none_or(|end| end > self.height) {
            return Err(TextError::Capacity);
        }
        let region = GlyphRegion {
            origin: [x + 1, y + 1],
            size: [glyph.width, glyph.height],
        };
        for row in 0..glyph.height {
            let dest = (region.origin[1] + row) * self.width + region.origin[0];
            let source = row * glyph.width;
            self.alpha[dest..dest + glyph.width]
                .copy_from_slice(&glyph.alpha[source..source + glyph.width]);
        }
        self.x = x + width;
        self.y = y;
        self.row_height = row_height.max(height);
        self.entries += 1;
        Ok(Some(region))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn glyph() -> GlyphBitmap {
        GlyphBitmap {
            width: 2,
            height: 2,
            bearing: [0; 2],
            advance: 2.0,
            alpha: vec![1, 2, 3, 4],
        }
    }
    #[test]
    fn packing_gutters_and_failed_insert_are_stable() {
        let mut atlas = GlyphAtlas::new(8, 8, 64, 8).unwrap();
        for origin in [[1, 1], [5, 1], [1, 5], [5, 5]] {
            assert_eq!(atlas.insert(&glyph()).unwrap().unwrap().origin, origin);
        }
        assert_eq!(&atlas.alpha()[9..11], &[1, 2]);
        assert_eq!(&atlas.alpha()[17..19], &[3, 4]);
        assert_eq!(atlas.alpha()[0], 0);
        assert_eq!(atlas.alpha()[12], 0);
        let before = atlas.alpha().to_vec();
        assert_eq!(atlas.insert(&glyph()), Err(TextError::Capacity));
        assert_eq!(atlas.alpha(), before);
        assert_eq!(atlas.entries(), 4);
        let mut bad = glyph();
        bad.alpha.pop();
        assert_eq!(atlas.insert(&bad), Err(TextError::InvalidBitmap));
        assert_eq!(atlas.alpha(), before);
    }
    #[test]
    fn budget_empty_glyph_and_entry_limit() {
        assert!(matches!(
            GlyphAtlas::new(8, 8, 63, 1),
            Err(TextError::Capacity)
        ));
        let mut atlas = GlyphAtlas::new(8, 8, 64, 1).unwrap();
        let empty = GlyphBitmap {
            width: 0,
            height: 0,
            bearing: [0; 2],
            advance: 1.0,
            alpha: vec![],
        };
        assert_eq!(atlas.insert(&empty), Ok(None));
        atlas.insert(&glyph()).unwrap();
        assert_eq!(atlas.insert(&glyph()), Err(TextError::Capacity));
        assert_eq!(atlas.insert(&empty), Ok(None));
    }
}
