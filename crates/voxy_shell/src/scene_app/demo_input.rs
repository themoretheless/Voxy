//! Translation of winit window events into the windowing-free demo input.
use voxy_scene_host::{DemoInput, DemoKey, DemoScroll};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

/// The demo input carried by `event`, if demos react to it.
pub(super) fn translate(event: &WindowEvent) -> Option<DemoInput> {
    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            let PhysicalKey::Code(code) = event.physical_key else {
                return None;
            };
            Some(DemoInput::Key {
                key: demo_key(code)?,
                pressed: event.state == ElementState::Pressed,
                repeat: event.repeat,
            })
        }
        WindowEvent::MouseInput {
            state,
            button: MouseButton::Left,
            ..
        } => Some(DemoInput::PrimaryButton {
            pressed: *state == ElementState::Pressed,
        }),
        WindowEvent::CursorMoved { position, .. } => Some(DemoInput::PointerMoved {
            x: position.x,
            y: position.y,
        }),
        WindowEvent::MouseWheel { delta, .. } => Some(DemoInput::Scroll(match delta {
            MouseScrollDelta::LineDelta(_, y) => DemoScroll::Lines(*y),
            MouseScrollDelta::PixelDelta(position) => DemoScroll::Pixels(position.y),
        })),
        WindowEvent::Focused(false) => Some(DemoInput::FocusLost),
        _ => None,
    }
}

pub(super) fn demo_key(code: KeyCode) -> Option<DemoKey> {
    Some(match code {
        KeyCode::ArrowLeft => DemoKey::ArrowLeft,
        KeyCode::ArrowRight => DemoKey::ArrowRight,
        KeyCode::ArrowUp => DemoKey::ArrowUp,
        KeyCode::ArrowDown => DemoKey::ArrowDown,
        KeyCode::Digit1 => DemoKey::Digit1,
        KeyCode::Digit2 => DemoKey::Digit2,
        KeyCode::Digit3 => DemoKey::Digit3,
        KeyCode::Digit4 => DemoKey::Digit4,
        KeyCode::BracketLeft => DemoKey::BracketLeft,
        KeyCode::BracketRight => DemoKey::BracketRight,
        KeyCode::KeyB => DemoKey::B,
        KeyCode::KeyC => DemoKey::C,
        KeyCode::KeyF => DemoKey::F,
        KeyCode::KeyG => DemoKey::G,
        KeyCode::KeyJ => DemoKey::J,
        KeyCode::KeyK => DemoKey::K,
        KeyCode::KeyL => DemoKey::L,
        KeyCode::KeyM => DemoKey::M,
        KeyCode::KeyP => DemoKey::P,
        KeyCode::KeyR => DemoKey::R,
        KeyCode::KeyS => DemoKey::S,
        KeyCode::KeyX => DemoKey::X,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_keys_demos_react_to_are_translated() {
        assert_eq!(demo_key(KeyCode::KeyR), Some(DemoKey::R));
        assert_eq!(demo_key(KeyCode::Digit3), Some(DemoKey::Digit3));
        assert_eq!(demo_key(KeyCode::BracketRight), Some(DemoKey::BracketRight));
        assert_eq!(demo_key(KeyCode::Escape), None);
        assert_eq!(demo_key(KeyCode::Space), None);
    }

    #[test]
    fn pointer_wheel_and_focus_events_are_translated() {
        assert_eq!(
            translate(&WindowEvent::Focused(false)),
            Some(DemoInput::FocusLost)
        );
        assert_eq!(translate(&WindowEvent::Focused(true)), None);
        assert_eq!(translate(&WindowEvent::CloseRequested), None);
    }
}
