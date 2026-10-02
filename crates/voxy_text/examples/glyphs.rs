//! Runtime font fixture; no proprietary system font is distributed in the repo.
use voxy_text::{FontLimits, GlyphAtlas, RasterFont, TextError};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a local TrueType font path")?;
    let bytes = std::fs::read(path)?;
    let limits = FontLimits {
        max_font_bytes: 8 * 1024 * 1024,
        max_glyph_pixels: 4096,
        max_size: 64.0,
    };
    let font = RasterFont::parse(&bytes, limits)?;
    let mut atlas = GlyphAtlas::new(128, 64, 8192, 16)?;
    for character in ['A', 'g', 'Ж', 'я', ' '] {
        let glyph = font.rasterize(character, 32.0)?;
        assert_eq!(glyph.alpha.len(), glyph.width * glyph.height);
        assert!(glyph.advance > 0.0);
        atlas.insert(&glyph)?;
        if character != ' ' {
            assert!(glyph.alpha.iter().any(|x| *x > 0));
        }
    }
    assert!(matches!(
        font.rasterize('A', f32::NAN),
        Err(TextError::InvalidSize)
    ));
    assert_eq!(atlas.entries(), 4);
    assert!(atlas.alpha().iter().any(|x| *x > 0));
    let tiny = RasterFont::parse(
        &bytes,
        FontLimits {
            max_glyph_pixels: 1,
            ..limits
        },
    )?;
    assert!(matches!(
        tiny.rasterize('A', 32.0),
        Err(TextError::Capacity)
    ));
    println!(
        "TEXT GLYPHS PASS: Latin/Cyrillic/whitespace raster, valid coverage, pre-raster pixel budget refusal"
    );
    Ok(())
}
