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

/// Exact atlas pixels and append-only shelf state. No packing is replayed on restore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphAtlasCheckpoint {
    pub version: u32,
    pub dimensions: [usize; 2],
    pub alpha: Vec<u8>,
    pub cursor: [usize; 3],
    pub entries: usize,
    pub max_entries: usize,
}
impl GlyphAtlas {
    pub fn checkpoint(&self) -> GlyphAtlasCheckpoint {
        let Self {
            width,
            height,
            alpha,
            x,
            y,
            row_height,
            entries,
            max_entries,
        } = self;
        GlyphAtlasCheckpoint {
            version: 1,
            dimensions: [*width, *height],
            alpha: alpha.clone(),
            cursor: [*x, *y, *row_height],
            entries: *entries,
            max_entries: *max_entries,
        }
    }
}
impl GlyphAtlasCheckpoint {
    pub fn prepare(&self, max_pixels: usize, max_entries: usize) -> Result<GlyphAtlas, TextError> {
        let [width, height] = self.dimensions;
        let [x, y, row_height] = self.cursor;
        if width
            .checked_mul(height)
            .is_none_or(|pixels| pixels > max_pixels || pixels != self.alpha.len())
            || self.entries > self.max_entries
            || self.max_entries > max_entries
        {
            return Err(TextError::Capacity);
        }
        if self.version != 1
            || width == 0
            || height == 0
            || x > width
            || y.checked_add(row_height)
                .is_none_or(|bottom| bottom > height)
            || (self.entries == 0 && (self.cursor != [0; 3] || self.alpha.iter().any(|v| *v != 0)))
            || (self.entries > 0 && (x < 3 || row_height < 3))
        {
            return Err(TextError::InvalidBitmap);
        }
        Ok(GlyphAtlas {
            width,
            height,
            alpha: self.alpha.clone(),
            x,
            y,
            row_height,
            entries: self.entries,
            max_entries: self.max_entries,
        })
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    #[test]
    fn restored_shelf_continues_120_insertions_without_repacking() {
        let mut original = GlyphAtlas::new(64, 64, 4096, 128).unwrap();
        let glyph = GlyphBitmap {
            width: 1,
            height: 1,
            bearing: [0; 2],
            advance: 1.,
            alpha: vec![173],
        };
        for _ in 0..7 {
            original.insert(&glyph).unwrap();
        }
        let saved = original.checkpoint();
        let mut restored = saved.prepare(4096, 128).unwrap();
        for _ in 0..120 {
            assert_eq!(
                original.insert(&glyph).unwrap(),
                restored.insert(&glyph).unwrap()
            );
            assert_eq!(original.checkpoint(), restored.checkpoint());
        }
        let mut bad = saved.clone();
        bad.cursor[1] = usize::MAX;
        assert!(bad.prepare(4096, 128).is_err());
        bad = saved.clone();
        bad.alpha.pop();
        assert!(bad.prepare(4096, 128).is_err());
        assert!(saved.prepare(4095, 128).is_err());
        assert!(saved.prepare(4096, 127).is_err());
    }
}
