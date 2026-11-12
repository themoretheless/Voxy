//! Presented-frame acceptance for composition authoring and isolated play.
use crate::App;
use std::path::PathBuf;
use voxy_scene::SceneDocument;
use winit::keyboard::KeyCode;
#[derive(Debug, Default)]
pub(super) struct Smoke {
    phase: u8,
    frame: u64,
    play_ticks: u64,
    expected: Option<SceneDocument>,
}
impl App {
    pub(crate) fn start_prefab_smoke(&mut self) {
        self.prefab_smoke = Some(Smoke {
            phase: if std::env::var_os("VOXY_PREFAB_SMOKE_JOINT").is_some() {
                8
            } else {
                Self::prefab_smoke_entry_phase()
            },
            ..Smoke::default()
        });
        self.smoke_deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(30));
    }

    fn prefab_smoke_entry_phase() -> u8 {
        if std::env::var_os("VOXY_PREFAB_SMOKE_HIERARCHY_SEARCH").is_some() {
            6
        } else {
            0
        }
    }

    /// Clicks the presented joint controls of the first object: add with a hierarchy target
    /// pick, switch the kind, remove. The authoring document ends as it started.
    fn joint_smoke_phase(&mut self, phase: u8) -> Result<(), Box<dyn std::error::Error>> {
        use crate::{InspectorMode, joint_authoring::SCHEMA, panels::Action};
        let document = self.authoring_document()?;
        let owner = document.objects.first().ok_or("missing joint owner")?;
        let token = crate::joint_authoring::request_token(owner)?;
        let kind = owner
            .components
            .get(SCHEMA)
            .map(|joint| &joint["kind"]["type"]);
        let clicked = match phase {
            8 => {
                self.selected = 0;
                self.inspector = InspectorMode::Components(0);
                true
            }
            9 | 12 => self.invoke_presented_action(Action::Joint(token))?,
            10 => {
                let target = &document.objects.get(1).ok_or("missing joint target")?.id;
                let pick = self
                    .follow_pick
                    .as_ref()
                    .ok_or("joint pick did not start")?;
                let choice = crate::camera_authoring::choice_token(pick.key, target);
                self.invoke_presented_action(Action::FollowTargetChoice(choice))?
            }
            _ => {
                if kind.and_then(|kind| kind.as_str()) != Some("fixed") {
                    return Err("picked joint is not a weld".into());
                }
                let row = crate::component_fields::fields(owner)?
                    .iter()
                    .position(|field| field.schema == SCHEMA && field.path == "/kind/type")
                    .ok_or("picked joint has no kind row")?;
                if self.inspector == InspectorMode::Components(row / 6) {
                    self.invoke_presented_action(Action::JointKind(token))?
                } else {
                    self.inspector = InspectorMode::Components(row / 6);
                    false
                }
            }
        };
        if !clicked {
            return Ok(());
        }
        let mut next = phase + 1;
        if phase == 12 {
            if kind.and_then(|kind| kind.as_str()) != Some("hinge")
                || self.authoring_document()?.objects[0]
                    .components
                    .contains_key(SCHEMA)
            {
                return Err("joint kind switch or removal was not applied".into());
            }
            println!(
                "PREFAB NATIVE JOINT PICK KIND REMOVE PASS frames={}",
                self.frames
            );
            next = Self::prefab_smoke_entry_phase();
        }
        let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
        smoke.phase = next;
        smoke.frame = self.frames;
        Ok(())
    }

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
            .authoring
            .prefab_assets
            .get(self.authoring.prefab_choice)
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
            .authoring
            .prefab_assets
            .get(self.authoring.prefab_choice)
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
            .authoring
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
    fn invoke_presented_action(
        &mut self,
        expected: crate::panels::Action,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let panels = self.panels.as_mut().ok_or("missing rendered panels")?;
        let Some(&(rect, _)) = panels
            .regions
            .iter()
            .find(|(_, action)| *action == expected)
        else {
            return Ok(false);
        };
        let action = panels
            .hit(glam::Vec2::new(
                rect[0] + rect[2] * 0.5,
                rect[1] + rect[3] * 0.5,
            ))
            .ok_or("panel action is not hit-testable")?;
        if action != expected {
            return Err("panel action overlaps another region".into());
        }
        self.panel_action(action)?;
        Ok(true)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
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
            8..=12 => self.joint_smoke_phase(phase)?,
            6 => {
                let document = self.panel_document()?;
                let query = &document
                    .objects
                    .first()
                    .ok_or("missing search fixture")?
                    .id
                    .0;
                if !self.invoke_presented_action(crate::panels::Action::SearchHierarchy)? {
                    return Ok(false);
                }
                self.authoring_input_key(KeyCode::KeyA, Some(query))?;
                let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
                smoke.phase = 7;
                smoke.frame = self.frames;
            }
            7 => {
                let panels = self
                    .panels
                    .as_ref()
                    .ok_or("missing rendered search results")?;
                let mut matches = panels
                    .regions
                    .iter()
                    .filter_map(|(_, action)| match action {
                        crate::panels::Action::Select(index) => Some(*index),
                        _ => None,
                    });
                if matches.next() != Some(0) || matches.next().is_some() {
                    return Err("hierarchy search did not isolate the fixture object".into());
                }
                if !self.invoke_presented_action(crate::panels::Action::Select(0))? {
                    return Ok(false);
                }
                if self.selected != 0 {
                    return Err("hierarchy search result selected the wrong object".into());
                }
                println!("PREFAB NATIVE HIERARCHY SEARCH PASS frames={}", self.frames);
                let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
                smoke.phase = 0;
                smoke.frame = self.frames;
            }
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
                let (path, bytes): (PathBuf, Vec<u8>) =
                    self.authoring.authoring_project.smoke_dependency(
                        self.authoring
                            .authoring_source
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
                if self.play.simulation_ticks == 0 {
                    return Ok(false);
                }
                let play_ticks = self.play.simulation_ticks;
                self.toggle_play()?;
                let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
                smoke.play_ticks = play_ticks;
                smoke.phase = 3;
                smoke.frame = self.frames;
            }
            3 => {
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
                    self.frames, smoke.play_ticks
                );
                if self.build.project.is_none() {
                    self.prefab_smoke = None;
                    return Ok(true);
                }
                if !self.invoke_presented_action(crate::panels::Action::BuildGame)? {
                    return Ok(false);
                }
                let smoke = self.prefab_smoke.as_mut().ok_or("missing acceptance")?;
                smoke.phase = 4;
            }
            4 => match self.build.result.as_ref() {
                Some(Ok(path)) => {
                    let path = path.clone();
                    if self.build.player.is_none() {
                        println!("PREFAB NATIVE BUILD PASS {}", path.display());
                        self.prefab_smoke = None;
                        return Ok(true);
                    }
                    if !self.invoke_presented_action(crate::panels::Action::RunBuiltGame)? {
                        return Ok(false);
                    }
                    println!("PREFAB NATIVE BUILD PASS {}", path.display());
                    self.prefab_smoke
                        .as_mut()
                        .ok_or("missing acceptance")?
                        .phase = 5;
                }
                Some(Err(error)) => return Err(error.clone().into()),
                None => {}
            },
            _ => match self.build.launch.as_ref() {
                Some(Ok(status)) if status.success() => {
                    println!("PREFAB NATIVE RUN BUILT GAME PASS");
                    self.prefab_smoke = None;
                    return Ok(true);
                }
                Some(result) => {
                    return Err(format!("packaged game launch failed: {result:?}").into());
                }
                None => {}
            },
        }
        Ok(false)
    }
}
