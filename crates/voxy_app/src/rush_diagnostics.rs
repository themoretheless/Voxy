//! Cached in-window diagnostics using Voxy's glyph rasterizer and scene overlay.
use voxy_render::{SceneGeometry, SceneMesh, SceneRenderer, SceneTexture, SceneTransform};
use voxy_text::{FontLimits, RasterFont};
const WIDTH: usize = 1200;
const HEIGHT: usize = 360;
#[derive(Debug)]
pub(crate) struct DiagnosticPanel {
    pub geometry: SceneGeometry,
    pub texture: SceneTexture,
    pub transform: SceneTransform,
    pub text: String,
}
fn font() -> Result<RasterFont, Box<dyn std::error::Error>> {
    let paths = std::env::var("VOXY_UI_FONT").into_iter().chain([
        "/System/Library/Fonts/Supplemental/Arial.ttf".into(),
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf".into(),
        "C:/Windows/Fonts/consola.ttf".into(),
    ]);
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(font) = RasterFont::parse(
                &bytes,
                FontLimits {
                    max_font_bytes: 16 * 1024 * 1024,
                    max_glyph_pixels: 4096,
                    max_size: 32.0,
                },
            ) {
                return Ok(font);
            }
        }
    }
    Err("No UI font found; set VOXY_UI_FONT to a local TTF font".into())
}
pub(crate) fn pixels(text: &str) -> Result<(Vec<u8>, usize), Box<dyn std::error::Error>> {
    let font = font()?;
    let mut pixels = vec![0u8; WIDTH * HEIGHT * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[16, 20, 29, 242]);
    }
    let mut x = 14.0f32;
    let mut baseline = 25.0f32;
    for ch in text.chars() {
        if ch == '\n' {
            x = 14.0;
            baseline += 23.0;
            continue;
        }
        let glyph = font
            .rasterize(ch, 18.0)
            .or_else(|_| font.rasterize('?', 18.0))?;
        if x + glyph.advance > WIDTH as f32 - 14.0 {
            x = 14.0;
            baseline += 23.0;
        }
        if baseline >= HEIGHT as f32 - 8.0 {
            break;
        }
        let left = x as i32 + glyph.bearing[0];
        let top = baseline as i32 - glyph.bearing[1] - glyph.height as i32;
        for y in 0..glyph.height {
            for gx in 0..glyph.width {
                let px = left + gx as i32;
                let py = top + y as i32;
                if px < 0 || py < 0 || px >= WIDTH as i32 || py >= HEIGHT as i32 {
                    continue;
                }
                let alpha = glyph.alpha[y * glyph.width + gx] as u16;
                let pixel = &mut pixels[(py as usize * WIDTH + px as usize) * 4..][..4];
                let color = if baseline <= 48.0 {
                    [130u16, 225, 235]
                } else {
                    [255u16, 190, 174]
                };
                for c in 0..3 {
                    pixel[c] = ((pixel[c] as u16 * (255 - alpha) + color[c] * alpha) / 255) as u8;
                }
            }
        }
        x += glyph.advance;
    }
    let height = (baseline as usize + 12).clamp(82, HEIGHT);
    pixels.truncate(WIDTH * height * 4);
    Ok((pixels, height))
}
impl DiagnosticPanel {
    pub fn build(
        renderer: &SceneRenderer,
        host: &voxy_render::SceneSurface,
        text: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (pixels, height) = pixels(&text)?;
        let texture = renderer.upload_texture(
            host.device(),
            host.queue(),
            WIDTH as u32,
            height as u32,
            &pixels,
        )?;
        let geometry = renderer.upload_mesh(host.device(), &SceneMesh::quad([1.0; 4]))?;
        let transform = renderer.create_transform(
            host.device(),
            glam::Mat4::from_scale_rotation_translation(
                glam::Vec3::new(1.96, 0.94 * height as f32 / HEIGHT as f32, 1.0),
                glam::Quat::IDENTITY,
                glam::Vec3::new(0.0, 0.97 - 0.47 * height as f32 / HEIGHT as f32, 0.0),
            ),
        )?;
        Ok(Self {
            geometry,
            texture,
            transform,
            text,
        })
    }
}
