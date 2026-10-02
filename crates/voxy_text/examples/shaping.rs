//! Runtime font proof of cluster-aware shaping and glyph-ID rasterization.
use voxy_text::{FontLimits, RasterFont, RunDirection, RunOptions, ShapeFont, TextError, TextFont};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(std::env::args().nth(1).ok_or("pass local font path")?)?;
    let shaper = ShapeFont::parse(&bytes, 8 * 1024 * 1024)?;
    let raster = RasterFont::parse(
        &bytes,
        FontLimits {
            max_font_bytes: 8 * 1024 * 1024,
            max_glyph_pixels: 4096,
            max_size: 64.0,
        },
    )?;
    for text in ["AV", "office", "e\u{301}", "Привет"] {
        let run = shaper.shape(text, 32.0, RunDirection::Guess, 128, 32)?;
        assert!(!run.is_empty());
        for glyph in &run {
            assert!(text.is_char_boundary(usize::try_from(glyph.cluster)?));
            raster.rasterize_indexed(glyph.id, 32.0)?;
        }
    }
    let width = |text| -> Result<f32, TextError> {
        Ok(shaper
            .shape(text, 32.0, RunDirection::LeftToRight, 128, 32)?
            .iter()
            .map(|g| g.advance[0])
            .sum())
    };
    assert!(
        width("AV")? < width("A")? + width("V")?,
        "kerning not applied"
    );
    let composed = shaper.shape("é", 32.0, RunDirection::Guess, 128, 32)?;
    let decomposed = shaper.shape("e\u{301}", 32.0, RunDirection::Guess, 128, 32)?;
    assert_eq!(
        composed.iter().map(|g| g.id).collect::<Vec<_>>(),
        decomposed.iter().map(|g| g.id).collect::<Vec<_>>()
    );
    assert!(matches!(
        shaper.shape("AV", 32.0, RunDirection::Guess, 1, 32),
        Err(TextError::Capacity)
    ));
    assert!(matches!(
        shaper.shape("AV", 32.0, RunDirection::Guess, 128, 1),
        Err(TextError::Capacity)
    ));
    let owner = TextFont::parse(
        &bytes,
        FontLimits {
            max_font_bytes: 8 * 1024 * 1024,
            max_glyph_pixels: 4096,
            max_size: 64.0,
        },
    )?;
    let options = RunOptions {
        size: 32.0,
        direction: RunDirection::Guess,
        max_text_bytes: 128,
        max_glyphs: 32,
        atlas_size: [256, 64],
        max_atlas_pixels: 16384,
    };
    let run = owner.prepare("AV é Жg", [8.0, 48.0], options)?;
    assert!(run.glyphs().len() >= 5);
    assert!(run.advance()[0] > 50.0);
    let before = run.atlas().alpha().to_vec();
    assert!(matches!(
        owner.prepare(
            "AV é Жg",
            [8.0, 48.0],
            RunOptions {
                max_atlas_pixels: 1,
                ..options
            }
        ),
        Err(TextError::Capacity)
    ));
    assert_eq!(run.atlas().alpha(), before);
    verify_repeated_glyphs(&owner, options)?;
    println!(
        "TEXT SHAPING PASS: AV kerning, composed/decomposed equivalence, UTF-8 clusters and shaped glyph rasterization"
    );
    Ok(())
}

fn verify_repeated_glyphs(owner: &TextFont, options: RunOptions) -> Result<(), TextError> {
    // Four drawable instances must fit in space sufficient for only one raster.
    let repeated = owner.prepare(
        "A A A A",
        [0.0, 32.0],
        RunOptions {
            atlas_size: [32, 32],
            max_atlas_pixels: 1024,
            ..options
        },
    )?;
    assert_eq!(repeated.glyphs().len(), 4);
    assert_eq!(repeated.atlas().entries(), 1);
    assert!(
        repeated
            .glyphs()
            .iter()
            .all(|g| g.region == repeated.glyphs()[0].region)
    );
    assert!(
        repeated
            .glyphs()
            .windows(2)
            .all(|pair| pair[0].origin[0] < pair[1].origin[0] && pair[0].cluster < pair[1].cluster)
    );
    Ok(())
}
