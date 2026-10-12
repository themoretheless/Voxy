//! Play mode: advancing the game, standalone start and play toggling.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    pub(crate) fn advance_game(&mut self, elapsed: f64) -> Result<(), Box<dyn std::error::Error>> {
        let mut models: BTreeMap<_, _> = if let Some(graphics) = &self.graphics {
            graphics
                .models
                .iter()
                .filter_map(|(id, resource)| {
                    resource
                        .animated_model
                        .as_ref()
                        .map(|model| (id.clone(), model.clone()))
                })
                .collect()
        } else {
            self.scene
                .components::<ModelInstance>()
                .filter_map(|(_, instance)| {
                    let resource = self.catalog.snapshot(&instance.asset)?;
                    Some((
                        instance.asset.clone(),
                        resource.value().animated.as_ref()?.clone(),
                    ))
                })
                .collect()
        };
        for (_, profile) in self.scene.components::<ModelRetarget>() {
            let source = AssetId(profile.source.clone());
            if let Some(asset) = self.catalog.snapshot(&source)
                && let Some(model) = asset.value().animated.as_ref()
            {
                models.insert(source, model.clone());
            }
        }
        self.play.animations.synchronize(&self.scene)?;
        if let Some(simulation) = &mut self.play.simulation {
            if let Some(actions) = &mut self.play.ui_actions {
                for result in actions.dispatch(&self.scene, simulation.commands())? {
                    if let Err(error) = result {
                        eprintln!("GAME UI ACTION FAILED: {error}");
                    }
                }
            }
            if let Some(physics) = &mut self.play.physics {
                physics.synchronize(&self.scene)?;
            }
            let frame_result = simulation.advance_scoped_with_animation_events(
                &mut self.scene,
                elapsed,
                animation_runtime::schedule()?,
                |system, mut access, dt| {
                    if system == "angular.step" {
                        if let Some(motion) = &mut self.play.angular_motion {
                            motion
                                .fixed_scoped(access, dt)
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                        }
                    } else if system == "character.step" {
                        access.require_write("liquid.world").map_err(|e| {
                            voxy_gameplay::GameplayFixedError::Motion(e.to_string())
                        })?;
                        let mut liquid = self.play.liquid.clone();
                        if let Some(runtime) = &mut liquid {
                            runtime
                                .tick(
                                    access.read().map_err(|e| {
                                        voxy_gameplay::GameplayFixedError::Motion(e.to_string())
                                    })?,
                                    dt,
                                    None,
                                )
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                        }
                        access
                            .require_write("animation.playback")
                            .map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                        let candidate = if let Some(physics) = &mut self.play.physics {
                            access.require_write("character.physics").map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                            access.require_write("player.input").map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                            self.play
                                .animations
                                .fixed_step(
                                    access.write().map_err(|error| {
                                        voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                                    })?,
                                    &models,
                                    physics,
                                    &mut self.play.player_input,
                                    dt,
                                )
                                .map_err(|error| match error {
                                    voxy_gameplay::CharacterTickError::Physics(error) => {
                                        voxy_gameplay::GameplayFixedError::Physics(error)
                                    }
                                    voxy_gameplay::CharacterTickError::Preparation(error) => {
                                        voxy_gameplay::GameplayFixedError::Motion(error)
                                    }
                                })?
                        } else {
                            let candidate = self
                                .play
                                .animations
                                .prepare_wall(
                                    access.read().map_err(|error| {
                                        voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                                    })?,
                                    &models,
                                    dt,
                                )
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                            if candidate.requires_pose_preparation()
                                || !candidate.motions().is_empty()
                                || !candidate.trajectories().is_empty()
                            {
                                return Err(voxy_gameplay::GameplayFixedError::Motion(
                                    "root motion requires a physics runtime".into(),
                                ));
                            }
                            access.require_write("player.input").map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                            self.play.player_input.finish_frame();
                            candidate
                        };
                        if let Some(runtime) = &mut liquid {
                            runtime
                                .publish_body_pose(access.write().map_err(|e| {
                                    voxy_gameplay::GameplayFixedError::Motion(e.to_string())
                                })?)
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                        }
                        self.play.animations = candidate;
                        self.play.liquid = liquid;
                    } else if let Some(physics) = &mut self.play.physics {
                        physics
                            .run_scoped_system(system, access, &mut self.play.player_input, dt)
                            .map_err(voxy_gameplay::GameplayFixedError::Physics)?;
                    }
                    Ok::<_, voxy_gameplay::GameplayFixedError>(
                        self.play
                            .animations
                            .take_events()
                            .into_iter()
                            .map(|(owner, event)| (owner, event.name, event.phase))
                            .collect(),
                    )
                },
            );
            if let Err(voxy_scene::SimulationStepError::System {
                completed_steps, ..
            }) = &frame_result
            {
                self.play.simulation_ticks = self
                    .play
                    .simulation_ticks
                    .saturating_add(u64::try_from(*completed_steps)?);
            }
            let frame = frame_result?;
            if let Some(audio) = self.audio.play_mut() {
                audio_play::schedule()?
                    .run_scene(&mut self.scene, |_, access| {
                        audio.advance_scoped(access, frame.time.steps)
                    })
                    .map_err(|error| error.to_string())?;
            }
            self.play.simulation_ticks = self
                .play
                .simulation_ticks
                .saturating_add(u64::try_from(frame.time.steps)?);
            for result in frame.commands {
                result?;
            }
            if let Some(physics) = &mut self.play.physics {
                physics.synchronize(&self.scene)?;
            }
        }
        Ok(())
    }

    pub(crate) fn start_standalone(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let required = self.required_cpu_model_assets();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            self.tick()?;
            for id in &required {
                if let Some(voxy_assets::AssetStatus::Failed(error)) = self.catalog.status(id) {
                    return Err(format!("game asset {} failed: {error}", id.0).into());
                }
            }
            if required
                .iter()
                .all(|id| self.catalog.snapshot(id).is_some())
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err("game resource imports timed out".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        self.validate_authoring_document(&self.authoring_document()?)?;
        self.standalone = true;
        self.toggle_play()
    }

    pub(crate) fn toggle_play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.retarget_draft.is_some() {
            return Err("apply or cancel the retarget profile edit before Play".into());
        }
        self.finish_drag(false)?;
        self.clear_panel_interactions();
        if self.audio.is_pending() {
            self.close_audio_device();
            self.update_edit_title();
            return Ok(());
        }
        self.game_ui_input.cancel();
        if let Some(expected) = self.play.playing.take() {
            self.close_play_audio();
            self.close_audio_device();
            self.play.stop(&mut self.scene)?;
            if let Some(graphics) = &mut self.graphics {
                graphics.animated_models.clear();
            }
            self.restore_authoring()?;
            if self.authoring_document()? != expected {
                return Err("Stop failed to restore authoring scene".into());
            }
        } else {
            CharacterPhysics::new(&self.scene, 128, 128)
                .with_depenetration(true)
                .validate_start(&self.scene)?;
            if let Some(runtime) = self
                .authoring
                .authoring_project
                .liquid_runtime(&self.scene)?
            {
                voxy_gameplay::validate_game_descriptors_with_liquid_runtime(
                    &self.scene,
                    128,
                    &runtime.0,
                )?;
            } else {
                voxy_gameplay::validate_game_descriptors(&self.scene, 128)?;
            }
            let audio = match self.audio.prepare_start(
                &self.scene,
                &self.authoring.authoring_project,
                self.window.is_some(),
            )? {
                audio_session::AudioStart::Pending => {
                    self.update_edit_title();
                    return Ok(());
                }
                audio_session::AudioStart::Ready(audio) => audio,
            };
            self.play.player_input = voxy_gameplay::player_input()?;
            let authoring_before_play = self.authoring_document()?;
            self.restore_authoring()?; // Detached runtime graph, fresh generational handles.
            let liquid = self
                .authoring
                .authoring_project
                .liquid_runtime(&self.scene)?;
            let mut actions = voxy_gameplay::UiActionHandlers::new(&self.scene, 128, 128, 128);
            actions.register("voxy.ui.hide".into(), |_, event, commands| {
                commands
                    .push(voxy_scene::SceneCommand::SetActive(event.owner, false))
                    .map_err(|error| error.to_string())
            })?;
            if let Some(setup) = &mut self.ui_action_setup {
                setup.configure(&mut actions)?;
            }
            let (liquid, optics) = liquid.map_or((None, Vec::new()), |(runtime, optics)| {
                (Some(runtime), optics)
            });
            self.play.liquid_optics = optics;
            self.play.liquid = liquid;
            self.play.playing = Some(authoring_before_play);
            self.authoring.animation_preview = None;
            self.play.ui_actions = Some(actions);
            let simulation = SceneSimulation::new(
                &self.scene,
                SimulationLimits {
                    fixed_step: 1.0 / 60.0,
                    max_steps: 8,
                    max_behaviors: 128,
                    max_commands: 128,
                },
            )?;
            let has_characters = self.scene.components::<CharacterBody>().next().is_some()
                || self.scene.components::<BoxCollider>().next().is_some();
            if has_characters {
                self.play.physics =
                    Some(CharacterPhysics::new(&self.scene, 128, 128).with_depenetration(true));
            }
            self.play.angular_motion =
                Some(voxy_gameplay::AngularMotionBatch::new(&self.scene, 128)?);
            self.audio.set_play(audio);
            self.play.simulation = Some(simulation);
            self.play.simulation_ticks = 0;
            self.play.animations.clear();
            self.play.simulation_time = Instant::now();
        }
        self.update_edit_title();
        Ok(())
    }

    pub(crate) fn game_key(
        &mut self,
        key: KeyCode,
        state: ElementState,
    ) -> Result<(), voxy_input::InputError> {
        if self.play.playing.is_some()
            && let Some(control) = game_control(key)
        {
            self.play.player_input.event(
                control,
                if state == ElementState::Pressed {
                    1.0
                } else {
                    0.0
                },
            )?;
            if self.trace.enabled {
                println!(
                    "GAME KEY frames={} ticks={} key={key:?} state={state:?}",
                    self.frames, self.play.simulation_ticks
                );
                self.trace.pending = true;
                self.trace.last_outcome = None;
            }
        }
        Ok(())
    }
}
