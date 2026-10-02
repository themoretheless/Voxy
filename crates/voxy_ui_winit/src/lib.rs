//! Native window event adapter; core UI remains platform-neutral.
use voxy_ui::{FocusRouter, HitRegion, KeyAction, PointerAction, PointerRouter, UiError};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiDispatch {
    pub consumed: bool,
    pub pointer: PointerAction,
    pub keyboard: KeyAction,
    /// Logical content offset delta; the owning panel decides consumption.
    pub scroll: Option<[f32; 2]>,
}
impl Default for UiDispatch {
    fn default() -> Self {
        Self {
            consumed: false,
            pointer: PointerAction::None,
            keyboard: KeyAction::None,
            scroll: None,
        }
    }
}
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // Independent native focus/key/modifier states.
pub struct WindowUi {
    pointer: PointerRouter,
    focus: FocusRouter,
    scale: f64,
    physical_cursor: Option<[f64; 2]>,
    focused: bool,
    enter: bool,
    space: bool,
    tab: bool,
    shift: bool,
    wheel_line_pixels: f32,
    input_ready: bool,
}
impl WindowUi {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            pointer: PointerRouter::new(capacity),
            focus: FocusRouter::new(capacity),
            scale: 1.0,
            physical_cursor: None,
            focused: true,
            enter: false,
            space: false,
            tab: false,
            shift: false,
            wheel_line_pixels: 32.0,
            input_ready: false,
        }
    }
    #[must_use]
    pub fn pointer(&self) -> &PointerRouter {
        &self.pointer
    }
    #[must_use]
    pub fn focus(&self) -> &FocusRouter {
        &self.focus
    }
    /// Uses enabled hit regions as the keyboard order. Cancellation actions must
    /// be delivered by the caller when widgets disappear or become disabled.
    /// This immediately admits input; render loops should instead use
    /// `set_presented_regions` after receiving their presentation result.
    /// # Errors
    /// Returns region capacity/geometry/identity validation errors.
    pub fn set_regions(&mut self, regions: &[HitRegion]) -> Result<UiDispatch, UiError> {
        let cancelled = self.pointer.set_regions(regions)?;
        let order: Vec<_> = regions.iter().filter(|r| r.enabled).map(|r| r.id).collect();
        let keyboard = self.focus.set_order(&order)?;
        self.input_ready = true;
        Ok(UiDispatch {
            consumed: cancelled.is_some() || keyboard != KeyAction::None,
            pointer: cancelled.map_or(PointerAction::None, PointerAction::Cancel),
            keyboard,
            scroll: None,
        })
    }
    /// Publishes geometry only after the caller confirms that its frame was shown.
    /// A skipped frame cancels captures and removes all input targets.
    /// # Errors
    /// Returns region validation errors for a presented frame.
    pub fn set_presented_regions(
        &mut self,
        regions: &[HitRegion],
        presented: bool,
    ) -> Result<UiDispatch, UiError> {
        if presented {
            self.set_regions(regions)
        } else {
            self.invalidate_presentation()
        }
    }
    /// Cancels input against obsolete or unpresented geometry. Held activation
    /// keys stay tracked, so recovery cannot turn a repeat/release into a click.
    /// # Errors
    /// Propagates region validation errors.
    pub fn invalidate_presentation(&mut self) -> Result<UiDispatch, UiError> {
        let result = self.set_regions(&[])?;
        self.input_ready = false;
        self.physical_cursor = None;
        self.pointer.move_to(None);
        Ok(result)
    }
    /// # Errors
    /// Rejects nonpositive/nonfinite logical pixels per wheel line.
    pub fn set_wheel_line_pixels(&mut self, pixels: f32) -> Result<(), UiError> {
        if !pixels.is_finite() || pixels <= 0.0 {
            return Err(UiError::InvalidGeometry);
        }
        self.wheel_line_pixels = pixels;
        Ok(())
    }
    /// Converts native content-motion deltas to positive-left/up content offsets.
    /// Pixel events are physical pixels; line events use configured logical units.
    /// # Errors
    /// Rejects invalid/nonfinite input or conversion overflow.
    #[allow(clippy::cast_possible_truncation)] // Converted coordinates validated below.
    pub fn wheel_delta(&self, delta: MouseScrollDelta) -> Result<[f32; 2], UiError> {
        let values = match delta {
            MouseScrollDelta::LineDelta(x, y) => [
                -f64::from(x) * f64::from(self.wheel_line_pixels),
                -f64::from(y) * f64::from(self.wheel_line_pixels),
            ],
            MouseScrollDelta::PixelDelta(position) => {
                [-position.x / self.scale, -position.y / self.scale]
            }
        };
        let logical = values.map(|x| x as f32);
        if logical.iter().any(|x| !x.is_finite()) {
            return Err(UiError::InvalidGeometry);
        }
        Ok(logical)
    }
    /// Reprojects the retained physical cursor when the window DPI changes.
    /// # Errors
    /// Rejects nonpositive/nonfinite scale without changing routing state.
    pub fn set_scale(&mut self, scale: f64) -> Result<(), UiError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(UiError::InvalidGeometry);
        }
        self.scale = scale;
        self.update_cursor();
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)] // PointerRouter rejects nonfinite projected coordinates.
    fn update_cursor(&mut self) -> bool {
        self.pointer.move_to(
            self.physical_cursor
                .map(|p| [(p[0] / self.scale) as f32, (p[1] / self.scale) as f32]),
        )
    }
    pub fn keyboard(&mut self, code: KeyCode, down: bool) -> UiDispatch {
        if !self.focused {
            return UiDispatch::default();
        }
        let previous = self.enter || self.space;
        let keyboard = match code {
            KeyCode::Enter => {
                self.enter = down;
                KeyAction::None
            }
            KeyCode::Space => {
                self.space = down;
                KeyAction::None
            }
            KeyCode::Tab => {
                let edge = down && !self.tab;
                self.tab = down;
                if edge {
                    self.focus.traverse(self.shift)
                } else {
                    KeyAction::None
                }
            }
            _ => return UiDispatch::default(),
        };
        let held = self.enter || self.space;
        let keyboard = if held && !previous {
            self.focus.press()
        } else if previous && !held {
            self.focus.release()
        } else {
            keyboard
        };
        UiDispatch {
            consumed: self.focus.focused().is_some() || keyboard != KeyAction::None,
            keyboard,
            ..UiDispatch::default()
        }
    }
    /// Routes supported primary-pointer, activation, traversal, DPI and focus events.
    pub fn event(&mut self, event: &WindowEvent) -> UiDispatch {
        match event {
            WindowEvent::Resized(_) | WindowEvent::Occluded(true) => {
                self.invalidate_presentation().unwrap_or_default()
            }
            WindowEvent::Focused(focused) => {
                self.focused = *focused;
                if *focused {
                    return UiDispatch::default();
                }
                self.enter = false;
                self.space = false;
                self.tab = false;
                self.shift = false;
                self.physical_cursor = None;
                self.invalidate_presentation().unwrap_or_default()
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let _ = self.set_scale(*scale_factor);
                UiDispatch::default()
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift = modifiers.state().shift_key();
                UiDispatch::default()
            }
            WindowEvent::CursorMoved { position, .. } if self.focused => {
                self.physical_cursor = Some([position.x, position.y]);
                UiDispatch {
                    consumed: self.update_cursor(),
                    ..UiDispatch::default()
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.physical_cursor = None;
                UiDispatch {
                    consumed: self.update_cursor(),
                    ..UiDispatch::default()
                }
            }
            WindowEvent::MouseWheel { delta, .. } if self.focused && self.input_ready => {
                UiDispatch {
                    scroll: self.wheel_delta(*delta).ok(),
                    ..UiDispatch::default()
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } if self.focused => {
                let result = if *state == ElementState::Pressed {
                    self.pointer.press()
                } else {
                    self.pointer.release()
                };
                let keyboard = if let PointerAction::Press(id) = result.action {
                    self.focus.focus_to(Some(id)).unwrap_or(KeyAction::None)
                } else {
                    KeyAction::None
                };
                UiDispatch {
                    consumed: result.consumed,
                    pointer: result.action,
                    keyboard,
                    scroll: None,
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.keyboard(code, event.state == ElementState::Pressed)
                } else {
                    UiDispatch::default()
                }
            }
            _ => UiDispatch::default(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use voxy_ui::WidgetId;
    #[test]
    fn skipped_frames_and_resize_cancel_held_input_until_represented() {
        let mut ui = WindowUi::new(1);
        let regions = [HitRegion {
            id: WidgetId(1),
            origin: [0.0; 2],
            size: [10.0; 2],
            enabled: true,
        }];
        ui.set_presented_regions(&regions, false).unwrap();
        assert_eq!(ui.keyboard(KeyCode::Tab, true).keyboard, KeyAction::None);
        ui.keyboard(KeyCode::Tab, false);
        ui.set_presented_regions(&regions, true).unwrap();
        ui.keyboard(KeyCode::Tab, true);
        ui.keyboard(KeyCode::Tab, false);
        assert_eq!(
            ui.keyboard(KeyCode::Enter, true).keyboard,
            KeyAction::Press(WidgetId(1))
        );
        ui.physical_cursor = Some([5.0; 2]);
        ui.update_cursor();
        assert_eq!(ui.pointer.press().action, PointerAction::Press(WidgetId(1)));
        let cancelled = ui.set_presented_regions(&regions, false).unwrap();
        assert_eq!(cancelled.keyboard, KeyAction::Cancel(WidgetId(1)));
        assert_eq!(cancelled.pointer, PointerAction::Cancel(WidgetId(1)));
        assert_eq!(ui.pointer.hovered(), None);
        ui.set_presented_regions(&regions, true).unwrap();
        ui.keyboard(KeyCode::Tab, true);
        ui.keyboard(KeyCode::Tab, false);
        assert_eq!(ui.keyboard(KeyCode::Enter, true).keyboard, KeyAction::None);
        assert_eq!(ui.keyboard(KeyCode::Enter, false).keyboard, KeyAction::None);
        assert_eq!(ui.pointer.release().action, PointerAction::None);
        assert_eq!(
            ui.keyboard(KeyCode::Enter, true).keyboard,
            KeyAction::Press(WidgetId(1))
        );
        assert_eq!(
            ui.keyboard(KeyCode::Enter, false).keyboard,
            KeyAction::Release {
                id: WidgetId(1),
                clicked: true
            }
        );
        ui.event(&WindowEvent::Resized(winit::dpi::PhysicalSize::new(
            200, 200,
        )));
        assert!(!ui.input_ready);
        assert_eq!(ui.keyboard(KeyCode::Tab, true).keyboard, KeyAction::None);
        ui.set_presented_regions(&regions, true).unwrap();
        ui.event(&WindowEvent::Occluded(true));
        assert!(!ui.input_ready);
        ui.event(&WindowEvent::Occluded(false));
        assert!(!ui.input_ready);
    }
    #[test]
    fn wheel_units_match_dpi_and_drive_scroll_panel() {
        let mut ui = WindowUi::new(1);
        let at_one = ui
            .wheel_delta(MouseScrollDelta::PixelDelta(
                winit::dpi::PhysicalPosition::new(0.0, -64.0),
            ))
            .unwrap();
        ui.set_scale(2.0).unwrap();
        let at_two = ui
            .wheel_delta(MouseScrollDelta::PixelDelta(
                winit::dpi::PhysicalPosition::new(0.0, -128.0),
            ))
            .unwrap();
        for (one, two) in at_one.into_iter().zip(at_two) {
            assert!((one - two).abs() < f32::EPSILON);
        }
        ui.set_wheel_line_pixels(16.0).unwrap();
        let lines = ui
            .wheel_delta(MouseScrollDelta::LineDelta(0.0, -4.0))
            .unwrap();
        let mut panel = voxy_ui::ScrollPanel::new([100.0; 2], [100.0, 300.0]).unwrap();
        panel.scroll(lines).unwrap();
        assert!((panel.offset()[1] - 64.0).abs() < f32::EPSILON);
        assert_eq!(ui.set_wheel_line_pixels(0.0), Err(UiError::InvalidGeometry));
        assert_eq!(
            ui.wheel_delta(MouseScrollDelta::LineDelta(f32::NAN, 0.0)),
            Err(UiError::InvalidGeometry)
        );
    }
    #[test]
    fn chords_repeat_focus_loss_and_dpi_reprojection() {
        let mut ui = WindowUi::new(1);
        ui.set_regions(&[HitRegion {
            id: WidgetId(1),
            origin: [0.0; 2],
            size: [10.0; 2],
            enabled: true,
        }])
        .unwrap();
        ui.physical_cursor = Some([15.0; 2]);
        ui.update_cursor();
        assert_eq!(ui.pointer.hovered(), None);
        ui.set_scale(2.0).unwrap();
        assert_eq!(ui.pointer.hovered(), Some(WidgetId(1)));
        ui.keyboard(KeyCode::Tab, true);
        ui.keyboard(KeyCode::Tab, false);
        assert_eq!(
            ui.keyboard(KeyCode::Enter, true).keyboard,
            KeyAction::Press(WidgetId(1))
        );
        assert_eq!(ui.keyboard(KeyCode::Space, true).keyboard, KeyAction::None);
        assert_eq!(ui.keyboard(KeyCode::Enter, false).keyboard, KeyAction::None);
        assert_eq!(
            ui.keyboard(KeyCode::Space, false).keyboard,
            KeyAction::Release {
                id: WidgetId(1),
                clicked: true
            }
        );
        ui.keyboard(KeyCode::Enter, true);
        assert_eq!(
            ui.event(&WindowEvent::Focused(false)).keyboard,
            KeyAction::Cancel(WidgetId(1))
        );
        assert_eq!(ui.keyboard(KeyCode::Enter, true), UiDispatch::default());
    }
}
