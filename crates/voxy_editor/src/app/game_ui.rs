//! In-game UI activation and input routing while playing.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    pub(crate) fn check_game_ui(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = voxy_gameplay::SceneUiRuntime::new(128);
        ui.refresh(&self.scene, [1280.0, 720.0])?;
        let snapshot = ui.snapshot().ok_or("missing game UI snapshot")?;
        let count = snapshot.elements.len();
        let expected: Vec<_> = snapshot
            .elements
            .iter()
            .filter(|element| element.enabled && element.clip.is_some())
            .filter_map(|element| {
                element
                    .descriptor
                    .action
                    .as_ref()
                    .map(|action| voxy_gameplay::UiActionEvent {
                        owner: element.owner,
                        action: action.clone(),
                    })
            })
            .collect();
        for event in &expected {
            ui.traverse(false);
            ui.key_press();
            if ui.key_release().as_ref() != Some(event) {
                return Err("game UI keyboard target mismatch".into());
            }
        }
        let text = ui_text::UiTextPreparation::prepare_import(
            &self.scene,
            &self.authoring.authoring_project,
            [1280.0, 720.0],
        )?;
        let text = text.value();
        let meshes = ui_draw::build(ui.snapshot().ok_or("missing UI layout")?, &text.runs)?;
        let glyph_quads: usize = meshes
            .iter()
            .filter(|draw| draw.atlas.is_some())
            .map(|draw| draw.mesh.indices().len() / 6)
            .sum();
        if meshes.iter().any(|draw| {
            !ui.snapshot().is_some_and(|snapshot| {
                snapshot
                    .elements
                    .iter()
                    .any(|element| element.owner == draw.owner)
            })
        }) {
            return Err("UI mesh owner missing".into());
        }
        println!(
            "GAME UI MESH draws={} glyph_quads={glyph_quads}",
            meshes.len()
        );
        println!(
            "GAME UI TEXT fonts={} runs={} glyphs={}",
            text.font_count(),
            text.runs.len(),
            text.runs
                .iter()
                .map(|run| run.run.glyphs().len())
                .sum::<usize>()
        );
        println!("GAME UI LAYOUT elements={count} buttons={}", expected.len());
        Ok(())
    }

    pub(crate) fn queue_ui_action(&mut self, event: voxy_gameplay::UiActionEvent) {
        if let Some(actions) = &mut self.play.ui_actions
            && let Err(error) = actions.enqueue(&self.scene, event)
        {
            eprintln!("GAME UI ACTION REJECTED: {error}");
        }
    }

    pub(crate) fn game_ui_display_ready(&mut self, viewport: [f32; 2]) -> bool {
        // Headless callers have no asynchronous display; native input must use
        // the snapshot acknowledged by an actual presented frame.
        if self.window.is_none() {
            return true;
        }
        match self
            .game_ui_input
            .admit_presented(&self.scene, viewport, self.ui_live.presented())
        {
            Ok(ready) => ready,
            Err(error) => {
                eprintln!("GAME UI INPUT CANCELLED: {error}");
                false
            }
        }
    }

    pub(crate) fn activate_game_ui(
        &mut self,
        event: voxy_gameplay::UiActionEvent,
    ) -> Result<(), voxy_input::InputError> {
        if self
            .play
            .ui_actions
            .as_ref()
            .is_some_and(|actions| actions.handles(&event.action))
        {
            self.queue_ui_action(event);
            return Ok(());
        }
        if self.play.player_input.state(&event.action).is_none() {
            // UI-only actions share the same bounded named map as physical input.
            self.play.player_input.bind(&event.action, Vec::new())?;
        }
        self.play.player_input.activate(&event.action)?;
        eprintln!(
            "GAME UI ACTION owner={:?} action={}",
            event.owner, event.action
        );
        Ok(())
    }

    // Native viewport coordinates use the same bounded f32 layout as rendering.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn game_ui_key(
        &mut self,
        key: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.play.playing.is_none() && !self.standalone {
            return Ok(false);
        }
        let viewport = self.window.as_ref().map_or([1280.0, 720.0], |window| {
            let size = window.inner_size();
            [size.width as f32, size.height as f32]
        });
        if !self.game_ui_display_ready(viewport) {
            return Ok(false);
        }
        match self.game_ui_input.key(
            &self.scene,
            viewport,
            key,
            state,
            repeat,
            self.modifiers.shift_key(),
        ) {
            Ok((consumed, event)) => {
                if let Some(event) = event {
                    self.activate_game_ui(event)?;
                }
                Ok(consumed)
            }
            Err(error) => {
                eprintln!("GAME UI INPUT CANCELLED: {error}");
                Ok(false)
            }
        }
    }

    pub(crate) fn game_ui_pointer(
        &mut self,
        cursor: Vec2,
        viewport: Vec2,
        pressed: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_none() && !self.standalone {
            return Ok(());
        }
        if !self.game_ui_display_ready(viewport.to_array()) {
            return Ok(());
        }
        match self.game_ui_input.pointer(
            &self.scene,
            viewport.to_array(),
            cursor.to_array(),
            pressed,
        ) {
            Ok(Some(event)) => {
                self.activate_game_ui(event)?;
            }
            Ok(None) => {}
            Err(error) => eprintln!("GAME UI INPUT CANCELLED: {error}"),
        }
        Ok(())
    }
}
