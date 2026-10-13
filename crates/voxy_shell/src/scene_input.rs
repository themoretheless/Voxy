//! Thin winit adapter; gameplay consumes named actions instead of key switches.
use voxy_input::{Binding, Control, InputMap};
use winit::keyboard::KeyCode;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneInputActions {
    pub pause: bool,
    pub toggle_world: bool,
}
#[derive(Debug)]
pub struct SceneInput {
    map: InputMap,
}
fn control(key: KeyCode) -> Control {
    // Runtime-only codes. Persist key names in a future configuration adapter.
    Control {
        device: 0,
        code: key as u32,
    }
}
impl SceneInput {
    /// # Errors
    /// Returns binding validation errors.
    pub fn new(pause: KeyCode, toggle_world: KeyCode) -> Result<Self, voxy_input::InputError> {
        let mut map = InputMap::new(2, 2);
        map.bind(
            "pause",
            vec![Binding {
                control: control(pause),
                scale: 1.0,
            }],
        )?;
        map.bind(
            "toggle_world",
            vec![Binding {
                control: control(toggle_world),
                scale: 1.0,
            }],
        )?;
        Ok(Self { map })
    }
    pub fn keyboard(&mut self, key: KeyCode, pressed: bool) -> SceneInputActions {
        // Digital values are finite and in range by construction.
        let _ = self
            .map
            .event(control(key), if pressed { 1.0 } else { 0.0 });
        let actions = SceneInputActions {
            pause: self.map.state("pause").is_some_and(|s| s.pressed),
            toggle_world: self.map.state("toggle_world").is_some_and(|s| s.pressed),
        };
        self.map.finish_frame();
        actions
    }
    pub fn focused(&mut self, focused: bool) {
        self.map.set_focused(focused);
        self.map.finish_frame();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_repeats_focus_and_custom_bindings() {
        let mut input = SceneInput::new(KeyCode::KeyP, KeyCode::KeyG).unwrap();
        assert_eq!(
            input.keyboard(KeyCode::Space, true),
            SceneInputActions::default()
        );
        assert!(input.keyboard(KeyCode::KeyP, true).pause);
        assert!(!input.keyboard(KeyCode::KeyP, true).pause);
        input.keyboard(KeyCode::KeyP, false);
        assert!(input.keyboard(KeyCode::KeyP, true).pause);
        input.focused(false);
        assert!(!input.keyboard(KeyCode::KeyG, true).toggle_world);
        input.focused(true);
        assert!(input.keyboard(KeyCode::KeyG, true).toggle_world);
    }
}
