//! Presented-frame acceptance for composition authoring and isolated play.
use crate::App;
use std::path::PathBuf;
use voxy_scene::SceneDocument;
use winit::keyboard::KeyCode;
#[derive(Debug, Default)]
pub(super) struct Smoke {
    phase: u8,
    frame: u64,
    expected: Option<SceneDocument>,
}
impl App {
    fn verify_prefab_creation(
        &mut self,
        original: &SceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let panels = self.panels.as_mut().ok_or("missing rendered panels")?;
        let &(rect, _) = panels
            .regions
            .iter()
            .find(|(_, action)| *action == crate::panels::Action::CreatePrefab)
            .ok_or("missing create prefab button")?;
        let action = panels
            .hit(glam::Vec2::new(
                rect[0] + rect[2] * 0.5,
                rect[1] + rect[3] * 0.5,
            ))
            .ok_or("create button is not hit-testable")?;
        if action != crate::panels::Action::CreatePrefab {
            return Err("create button overlaps another region".into());
        }
        self.panel_action(action)?;
        if self.authoring_document()? != *original {
            return Err("asset creation changed scene".into());
        }
        if self
            .prefab_assets
            .get(self.prefab_choice)
            .is_none_or(|asset| !asset.0.starts_with("prefab-"))
        {
            return Err("created asset was not selected".into());
        }
        self.panel_action(crate::panels::Action::PlacePrefab)?;
        let placed = self.authoring_document()?;
        if placed.objects.len() != original.objects.len() + 1 {
            return Err("created template did not instantiate".into());
        }
        self.save_authoring()?;
        self.load_authoring()?;
        if self.authoring_document()? != placed {
            return Err("created asset save/load lost instance".into());
        }
        self.history_key(KeyCode::KeyZ)?;
        self.save_authoring()?;
        self.load_authoring()?;
        if self.authoring_document()? != *original {
            return Err("created asset placement undo lost scene".into());
        }
        self.selected = 0;
        println!(
            "PREFAB NATIVE PANEL CREATE PLACE SAVE UNDO PASS frames={}",
            self.frames
        );
        Ok(())
    }
    fn verify_prefab_placement(
        &mut self,
        original: &SceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let panels = self.panels.as_mut().ok_or("missing rendered panels")?;
        let &(rect, _) = panels
            .regions
            .iter()
            .find(|(_, action)| *action == crate::panels::Action::PrefabChoice)
            .ok_or("missing prefab choice button")?;
        let choice = panels
            .hit(glam::Vec2::new(
                rect[0] + rect[2] * 0.5,
                rect[1] + rect[3] * 0.5,
            ))
            .ok_or("prefab choice is not hit-testable")?;
        if choice != crate::panels::Action::PrefabChoice {
            return Err("choice button overlaps another region".into());
        }
        self.panel_action(choice)?;
        if self
            .prefab_assets
            .get(self.prefab_choice)
            .is_none_or(|asset| asset.0 != "nested")
        {
            return Err("native fixture did not select nested prefab".into());
        }
        let panels = self.panels.as_mut().ok_or("missing rendered panels")?;
        let &(rect, _) = panels
            .regions
            .iter()
            .find(|(_, action)| *action == crate::panels::Action::PlacePrefab)
            .ok_or("missing place prefab button")?;
        let action = panels
            .hit(glam::Vec2::new(
                rect[0] + rect[2] * 0.5,
                rect[1] + rect[3] * 0.5,
            ))
            .ok_or("place prefab button is not hit-testable")?;
        if action != crate::panels::Action::PlacePrefab {
            return Err("place button overlaps another region".into());
        }
        self.panel_action(action)?;
        let placed = self.authoring_document()?;
        if placed.objects.len() != original.objects.len() + 1 {
            return Err("placement lost or duplicated prefab rows".into());
        }
        self.history_key(KeyCode::KeyZ)?;
        if self.authoring_document()? != *original {
            return Err("placement undo lost original scene".into());
        }
        self.history_key(KeyCode::KeyY)?;
        if self.authoring_document()? != placed {
            return Err("placement redo changed identities".into());
        }
        self.save_authoring()?;
        self.load_authoring()?;
        if self.authoring_document()? != placed {
            return Err("placement save/load lost source links".into());
        }
        if self
            .authoring_source
            .as_ref()
            .ok_or("missing placement publication")?
            .value()
            .source
            .instances
            .len()
            != 3
        {
            return Err("placement baked the new instance".into());
        }
        self.history_key(KeyCode::KeyZ)?;
        self.save_authoring()?;
        self.load_authoring()?;
        if self.authoring_document()? != *original {
            return Err("placement cleanup undo changed original".into());
        }
        self.selected = 0;
        println!(
            "PREFAB NATIVE PANEL PLACE SAVE UNDO REDO PASS frames={}",
            self.frames
        );
        Ok(())
    }
    fn verify_prefab_revert(
        &mut self,
        original: &SceneDocument,
        edited: &SceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let panels = self.panels.as_mut().ok_or("missing rendered panels")?;
        let &(rect, _) = panels
            .regions
            .iter()
            .find(|(_, action)| *action == crate::panels::Action::RevertPrefab)
            .ok_or("missing revert instance button")?;
        let action = panels
            .hit(glam::Vec2::new(
                rect[0] + rect[2] * 0.5,
                rect[1] + rect[3] * 0.5,
            ))
            .ok_or("revert instance button is not hit-testable")?;
        if action != crate::panels::Action::RevertPrefab {
            return Err("revert instance button overlaps another region".into());
        }
        self.panel_action(action)?;
        if self.authoring_document()? != *original {
            return Err("prefab panel revert changed sibling or source identity".into());
        }
        self.save_authoring()?;
        self.load_authoring()?;
        self.history_key(KeyCode::KeyZ)?;
        if self.authoring_document()? != *edited {
            return Err("prefab panel revert undo lost overrides".into());
        }
        self.history_key(KeyCode::KeyY)?;
        if self.authoring_document()? != *original {
            return Err("prefab panel revert redo lost source pose".into());
        }
        self.history_key(KeyCode::KeyZ)?;
        self.save_authoring()?;
        println!(
            "PREFAB NATIVE PANEL REVERT UNDO REDO PASS frames={}",
            self.frames
        );
        Ok(())
    }
    fn verify_prefab_structure(
        &mut self,
        edited: &SceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.edit_key(KeyCode::Delete)?;
        self.save_authoring()?;
        let deleted = self.authoring_document()?;
        self.load_authoring()?;
        if self.authoring_document()? != deleted {
            return Err("prefab tombstone reload changed deletion".into());
        }
        self.history_key(KeyCode::KeyZ)?;
        if self.authoring_document()? != *edited {
            return Err("prefab structural undo lost linked object".into());
        }
        self.save_authoring()?;
        self.load_authoring()?;
        if self.authoring_document()? != *edited {
            return Err("prefab resurrection was baked or lost".into());
        }
        println!(
            "PREFAB NATIVE STRUCTURAL SAVE UNDO PASS frames={}",
            self.frames
        );
        Ok(())
    }
    pub(super) fn prefab_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(smoke) = self.prefab_smoke.as_ref() else {
            return Ok(false);
        };
        if self.frames < smoke.frame + 3 {
            return Ok(false);
        }
        let phase = smoke.phase;
        if self
            .graphics
            .as_ref()
            .is_none_or(|graphics| graphics.models.is_empty())
        {
            return Ok(false);
        }
        match phase {
            0 => {
                let original = self.authoring_document()?;
                self.verify_prefab_placement(&original)?;
                self.verify_prefab_creation(&original)?;
                self.edit_key(KeyCode::ArrowRight)?;
                let edited = self.authoring_document()?;
                if edited == original {
                    return Err("prefab smoke did not edit authoring".into());
                }
                self.save_authoring()?;
                self.load_authoring()?;
                if self.authoring_document()? != edited {
                    return Err("prefab save/load changed expansion".into());
                }
                self.history_key(KeyCode::KeyZ)?;
                if self.authoring_document()? != original {
                    return Err("prefab undo changed instance identity".into());
                }
                self.history_key(KeyCode::KeyY)?;
                if self.authoring_document()? != edited {
                    return Err("prefab redo lost edits".into());
                }
                self.verify_prefab_revert(&original, &edited)?;
                self.verify_prefab_structure(&edited)?;
                let (path, bytes): (PathBuf, Vec<u8>) = self.authoring_project.smoke_dependency(
                    self.authoring_source
                        .as_ref()
                        .ok_or("missing prefab source")?,
                )?;
                std::fs::write(&path, b"invalid prefab dependency")?;
                let rejected = self.load_authoring().is_err();
                // Restore even when the last-good assertion below fails.
                std::fs::write(path, bytes)?;
                if !rejected || self.authoring_document()? != edited {
                    return Err("failed prefab load replaced authoring".into());
                }
                println!(
                    "PREFAB NATIVE SAVE UNDO AND LAST GOOD PASS frames={}",
                    self.frames
                );
                let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
                smoke.expected = Some(edited);
                smoke.phase = 1;
                smoke.frame = self.frames;
            }
            1 => {
                self.load_authoring()?;
                println!(
                    "PREFAB NATIVE PRESENTED AFTER FAILURE PASS frames={}",
                    self.frames
                );
                self.toggle_play()?;
                let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
                smoke.phase = 2;
                smoke.frame = self.frames;
            }
            2 => {
                if self.simulation_ticks == 0 {
                    return Ok(false);
                }
                self.toggle_play()?;
                let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
                smoke.phase = 3;
                smoke.frame = self.frames;
            }
            _ => {
                if self.authoring_document()?
                    != *self
                        .prefab_smoke
                        .as_ref()
                        .and_then(|smoke| smoke.expected.as_ref())
                        .ok_or("missing expected authoring")?
                {
                    return Err("prefab play/stop changed authoring".into());
                }
                println!(
                    "PREFAB NATIVE PLAY STOP PASS frames={} ticks={}",
                    self.frames, self.simulation_ticks
                );
                self.prefab_smoke = None;
                return Ok(true);
            }
        }
        Ok(false)
    }
}
