//! Native colored UI controls: hover/capture/focus, activation and resize.
use glam::{Mat4, Vec2};
use std::sync::Arc;
use voxy_render::{
    RenderOutcome, SceneDraw, SceneGeometry, SceneRenderer, SceneSurface, SceneTexture,
    SceneTransform, Sprite, SpriteBatch,
};
use voxy_text::{FontLimits, RunDirection, RunOptions, TextFont, TextRun};
use voxy_ui::{
    Axis, KeyAction, LayoutItem, Length, PointerAction, ScrollPanel, WidgetId, layout_linear,
};
use voxy_ui_winit::WindowUi;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
#[derive(Debug)]
struct Resources {
    host: SceneSurface,
    renderer: SceneRenderer,
    geometry: SceneGeometry,
    texture: SceneTexture,
    transform: SceneTransform,
    labels: Vec<Label>,
}
#[derive(Debug)]
struct Label {
    run: TextRun,
    geometry: SceneGeometry,
    texture: SceneTexture,
}
impl Label {
    fn new(
        font: &TextFont,
        text: &str,
        host: &SceneSurface,
        renderer: &SceneRenderer,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let run = font.prepare(
            text,
            [0.0, 24.0],
            RunOptions {
                size: 24.0,
                direction: RunDirection::Guess,
                max_text_bytes: 128,
                max_glyphs: 32,
                atlas_size: [256, 64],
                max_atlas_pixels: 16384,
            },
        )?;
        let rgba: Vec<_> = run
            .atlas()
            .alpha()
            .iter()
            .flat_map(|alpha| [255, 255, 255, *alpha])
            .collect();
        let texture = renderer.upload_texture(host.device(), host.queue(), 256, 64, &rgba)?;
        let geometry = renderer.reserve_geometry(
            host.device(),
            run.glyphs().len() * 4,
            run.glyphs().len() * 6,
        )?;
        Ok(Self {
            run,
            geometry,
            texture,
        })
    }
    fn update(
        &mut self,
        host: &SceneSurface,
        region: voxy_ui::HitRegion,
        clip: voxy_ui::HitRegion,
        viewport: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut batch = SpriteBatch::new(self.run.glyphs().len());
        let coordinate = |value| -> Result<f32, std::num::TryFromIntError> {
            Ok(f32::from(u16::try_from(value)?))
        };
        for glyph in self.run.glyphs() {
            let size = Vec2::new(
                coordinate(glyph.region.size[0])?,
                coordinate(glyph.region.size[1])?,
            );
            let origin = Vec2::from_array(region.origin)
                + Vec2::new(16.0, (region.size[1] - 24.0).max(0.0) * 0.5)
                + Vec2::from_array(glyph.origin);
            let bounds = voxy_ui::HitRegion {
                id: region.id,
                origin: origin.to_array(),
                size: size.to_array(),
                enabled: true,
            };
            let Some(visible) = bounds.clipped(clip.origin, clip.size)? else {
                continue;
            };
            let mut sprite = Sprite::from_logical_rect(origin, size, viewport, [1.0; 4])?;
            sprite.uv_min = Vec2::new(
                coordinate(glyph.region.origin[0])? / 256.0,
                coordinate(glyph.region.origin[1])? / 64.0,
            );
            sprite.uv_max = Vec2::new(
                coordinate(glyph.region.origin[0] + glyph.region.size[0])? / 256.0,
                coordinate(glyph.region.origin[1] + glyph.region.size[1])? / 64.0,
            );
            let start = (Vec2::from_array(visible.origin) - origin) / size;
            let end = start + Vec2::from_array(visible.size) / size;
            batch.push(sprite.cropped(
                start.clamp(Vec2::ZERO, Vec2::ONE),
                end.clamp(Vec2::ZERO, Vec2::ONE),
            )?)?;
        }
        if batch.is_empty() {
            self.geometry.clear();
        } else {
            self.geometry.update(host.queue(), &batch.mesh()?)?;
        }
        Ok(())
    }
}
#[derive(Debug)]
struct App {
    window: Option<Arc<Window>>,
    resources: Option<Resources>,
    ui: WindowUi,
    toggles: [bool; 3],
    frames: u32,
    smoke: bool,
    error: Option<String>,
    font: Option<TextFont>,
    panel: Option<ScrollPanel>,
}
impl App {
    fn initialize(&mut self, events: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let window = Arc::new(
            events.create_window(
                Window::default_attributes()
                    .with_title("Voxy UI — mouse, Tab, Enter/Space")
                    .with_inner_size(winit::dpi::LogicalSize::new(640.0, 320.0)),
            )?,
        );
        self.ui.set_scale(window.scale_factor())?;
        let size = window.inner_size();
        let host = pollster::block_on(SceneSurface::new(
            Arc::clone(&window),
            size.width,
            size.height,
        ))?;
        let renderer = host.create_scene_renderer();
        let geometry = renderer.reserve_geometry(host.device(), 12, 18)?;
        let texture = renderer.upload_texture(host.device(), host.queue(), 1, 1, &[255; 4])?;
        let transform = renderer.create_transform(host.device(), Mat4::IDENTITY)?;
        let mut labels = Vec::new();
        if let Some(font) = &self.font {
            for text in ["Play / Играть", "Pause / Пауза", "Settings / Настройки"]
            {
                labels.push(Label::new(font, text, &host, &renderer)?);
            }
        }
        self.resources = Some(Resources {
            host,
            renderer,
            geometry,
            texture,
            transform,
            labels,
        });
        self.window = Some(window);
        Ok(())
    }
    fn project_regions(
        &self,
        regions: Vec<voxy_ui::HitRegion>,
    ) -> Result<Vec<(voxy_ui::HitRegion, voxy_ui::HitRegion)>, voxy_ui::UiError> {
        let mut projected = Vec::new();
        for region in regions {
            if let Some(panel) = &self.panel {
                if let Some(visible) = panel.project(region, [0.0; 2])? {
                    let offset = panel.offset();
                    let full = voxy_ui::HitRegion {
                        origin: [region.origin[0] - offset[0], region.origin[1] - offset[1]],
                        ..region
                    };
                    projected.push((full, visible));
                }
            } else {
                projected.push((region, region));
            }
        }
        Ok(projected)
    }
    fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(window) = &self.window else {
            return Ok(());
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let logical = size.to_logical::<f32>(window.scale_factor());
        let viewport = Vec2::new(logical.width, logical.height);
        let content = Vec2::new(
            viewport.x,
            if self.panel.is_some() {
                viewport.y.max(600.0)
            } else {
                viewport.y
            },
        );
        if let Some(panel) = &mut self.panel {
            panel.resize(viewport.to_array(), content.to_array())?;
        }
        let items = std::array::from_fn::<_, 3, _>(|index| LayoutItem {
            id: WidgetId(u64::try_from(index).unwrap() + 1),
            length: Length::Flex(1.0),
            enabled: true,
        });
        let regions = match layout_linear(
            [0.0; 2],
            content.to_array(),
            Axis::Vertical,
            16.0,
            12.0,
            &items,
            3,
        ) {
            Ok(regions) => regions,
            Err(voxy_ui::UiError::InsufficientSpace) => {
                self.ui.invalidate_presentation()?;
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        let projected = self.project_regions(regions)?;
        let regions: Vec<_> = projected.iter().map(|(_, visible)| *visible).collect();
        let mut batch = SpriteBatch::new(3);
        for region in &regions {
            let index = usize::try_from(region.id.0 - 1)?;
            let color = if self.ui.pointer().captured() == Some(region.id) {
                [0.9, 0.4, 0.1, 1.0]
            } else if self.ui.focus().focused() == Some(region.id) {
                [0.6, 0.3, 0.9, 1.0]
            } else if self.ui.pointer().hovered() == Some(region.id) {
                [0.2, 0.7, 0.9, 1.0]
            } else if self.toggles[index] {
                [0.2, 0.8, 0.3, 1.0]
            } else {
                [0.2, 0.3, 0.5, 1.0]
            };
            batch.push(Sprite::from_logical_rect(
                Vec2::from_array(region.origin),
                Vec2::from_array(region.size),
                viewport,
                color,
            )?)?;
        }
        if let Some(resources) = &mut self.resources {
            resources.host.resize(size.width, size.height)?;
            if batch.is_empty() {
                resources.geometry.clear();
            } else {
                resources
                    .geometry
                    .update(resources.host.queue(), &batch.mesh()?)?;
            }
            for label in &mut resources.labels {
                label.geometry.clear();
            }
            for (full, visible) in &projected {
                if let Some(label) = resources.labels.get_mut(usize::try_from(full.id.0 - 1)?) {
                    label.update(&resources.host, *full, *visible, viewport)?;
                }
            }
            let mut draws = vec![SceneDraw {
                geometry: &resources.geometry,
                texture: &resources.texture,
                transform: &resources.transform,
                overlay: true,
            }];
            draws.extend(resources.labels.iter().map(|label| SceneDraw {
                geometry: &label.geometry,
                texture: &label.texture,
                transform: &resources.transform,
                overlay: true,
            }));
            let outcome = resources.host.render_scene(&resources.renderer, &draws)?;
            self.ui
                .set_presented_regions(&regions, outcome == RenderOutcome::Presented)?;
            if outcome == RenderOutcome::Presented {
                self.frames += 1;
            }
        }
        Ok(())
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if let Err(error) = self.initialize(events) {
            self.error = Some(error.to_string());
            events.exit();
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let dispatch = self.ui.event(&event);
        if let (Some(delta), Some(panel)) = (dispatch.scroll, &mut self.panel) {
            match panel.scroll(delta) {
                Ok(_) => {
                    if let Some(window) = &self.window {
                        window.set_title(&format!("Voxy UI scroll: {:?}", panel.offset()));
                    }
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    events.exit();
                }
            }
        }
        let clicked = match (dispatch.pointer, dispatch.keyboard) {
            (PointerAction::Release { id, clicked: true }, _)
            | (_, KeyAction::Release { id, clicked: true }) => Some(id),
            _ => None,
        };
        if let Some(id) = clicked
            .and_then(|id| usize::try_from(id.0).ok())
            .and_then(|id| id.checked_sub(1))
        {
            if let Some(toggle) = self.toggles.get_mut(id) {
                *toggle = !*toggle;
            }
            if let Some(window) = &self.window {
                window.set_title(&format!("Voxy UI toggles: {:?}", self.toggles));
            }
        }
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw() {
                    self.error = Some(error.to_string());
                    events.exit();
                }
                if self.smoke && self.frames >= 60 {
                    events.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scroll = std::env::args().any(|arg| arg == "--scroll");
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    let font = std::env::args()
        .skip(1)
        .find(|arg| arg != "--smoke" && arg != "--scroll")
        .map(|path| -> Result<_, Box<dyn std::error::Error>> {
            Ok(TextFont::parse(
                &std::fs::read(path)?,
                FontLimits {
                    max_font_bytes: 8 * 1024 * 1024,
                    max_glyph_pixels: 4096,
                    max_size: 64.0,
                },
            )?)
        })
        .transpose()?;
    let mut app = App {
        window: None,
        resources: None,
        ui: WindowUi::new(3),
        toggles: [false; 3],
        frames: 0,
        smoke,
        error: None,
        font,
        panel: if scroll {
            Some(ScrollPanel::new([640.0, 320.0], [640.0, 600.0])?)
        } else {
            None
        },
    };
    EventLoop::new()?.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    if smoke {
        assert!(app.frames >= 60);
        println!("UI WINDOW PASS: {} presented native frames", app.frames);
    }
    Ok(())
}
