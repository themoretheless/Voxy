use crate::TextError;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunDirection {
    Guess,
    LeftToRight,
    RightToLeft,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapedGlyph {
    pub id: u16,
    /// UTF-8 byte offset of the source cluster (multiple glyphs may share it).
    pub cluster: u32,
    /// Pixels, font coordinate convention: positive Y is upwards.
    pub advance: [f32; 2],
    pub offset: [f32; 2],
}
/// One font face; shaping operates on one directional/script run. Paragraph bidi,
/// script itemization, fallback and line breaking are the caller's next layer.
#[derive(Debug)]
pub struct ShapeFont {
    bytes: Vec<u8>,
}
impl ShapeFont {
    /// # Errors
    /// Rejects invalid face zero or oversized font bytes before copying.
    pub fn parse(bytes: &[u8], max_bytes: usize) -> Result<Self, TextError> {
        if bytes.len() > max_bytes {
            return Err(TextError::Capacity);
        }
        rustybuzz::Face::from_slice(bytes, 0).ok_or(TextError::InvalidFont)?;
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }
    /// Input bytes are capped before shaping; output glyph count is capped after
    /// the shaper returns. Internal shaper allocation/work is not hard-budgeted.
    /// # Errors
    /// Rejects invalid size, byte/glyph caps and missing glyphs.
    #[allow(clippy::cast_possible_truncation)] // Font-unit positions scaled to f32 pixels.
    pub fn shape(
        &self,
        text: &str,
        size: f32,
        direction: RunDirection,
        max_bytes: usize,
        max_glyphs: usize,
    ) -> Result<Vec<ShapedGlyph>, TextError> {
        if !size.is_finite() || size <= 0.0 {
            return Err(TextError::InvalidSize);
        }
        if text.len() > max_bytes {
            return Err(TextError::Capacity);
        }
        let face = rustybuzz::Face::from_slice(&self.bytes, 0).ok_or(TextError::InvalidFont)?;
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        match direction {
            RunDirection::Guess => {}
            RunDirection::LeftToRight => buffer.set_direction(rustybuzz::Direction::LeftToRight),
            RunDirection::RightToLeft => buffer.set_direction(rustybuzz::Direction::RightToLeft),
        }
        buffer.guess_segment_properties();
        let result = rustybuzz::shape(&face, &[], buffer);
        if result.glyph_infos().len() > max_glyphs {
            return Err(TextError::Capacity);
        }
        let scale = f64::from(size) / f64::from(face.units_per_em());
        result
            .glyph_infos()
            .iter()
            .zip(result.glyph_positions())
            .map(|(info, position)| {
                let id = u16::try_from(info.glyph_id).map_err(|_| TextError::MissingGlyph)?;
                if id == 0 {
                    return Err(TextError::MissingGlyph);
                }
                let convert = |x| (f64::from(x) * scale) as f32;
                let advance = [convert(position.x_advance), convert(position.y_advance)];
                let offset = [convert(position.x_offset), convert(position.y_offset)];
                if advance.iter().chain(&offset).any(|x| !x.is_finite()) {
                    return Err(TextError::InvalidSize);
                }
                Ok(ShapedGlyph {
                    id,
                    cluster: info.cluster,
                    advance,
                    offset,
                })
            })
            .collect()
    }
}
