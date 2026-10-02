//! Bounded UI events feed application callbacks and the existing scene barrier.
use crate::{UiActionEvent, UiElement};
use std::collections::{BTreeMap, VecDeque};
use voxy_scene::{SceneCommands, SceneGraph, SceneId};
type Handler =
    Box<dyn FnMut(&SceneGraph, &UiActionEvent, &mut SceneCommands) -> Result<(), String> + Send>;

pub struct UiActionHandlers {
    scene: SceneId,
    events: VecDeque<UiActionEvent>,
    handlers: BTreeMap<String, Handler>,
    event_capacity: usize,
    handler_capacity: usize,
    command_capacity: usize,
}
impl std::fmt::Debug for UiActionHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiActionHandlers")
            .field("scene", &self.scene)
            .field("pending", &self.events.len())
            .field("handlers", &self.handlers.keys())
            .field("event_capacity", &self.event_capacity)
            .field("handler_capacity", &self.handler_capacity)
            .field("command_capacity", &self.command_capacity)
            .finish()
    }
}
impl UiActionHandlers {
    #[must_use]
    pub fn new(
        scene: &SceneGraph,
        events: usize,
        handlers: usize,
        commands_per_event: usize,
    ) -> Self {
        Self {
            scene: scene.identity(),
            events: VecDeque::new(),
            handlers: BTreeMap::new(),
            event_capacity: events,
            handler_capacity: handlers,
            command_capacity: commands_per_event,
        }
    }
    /// Duplicate names reject rather than silently replace application logic.
    /// # Errors
    /// Rejects invalid names, duplicate handlers and handler capacity overflow.
    pub fn register(
        &mut self,
        action: String,
        handler: impl FnMut(&SceneGraph, &UiActionEvent, &mut SceneCommands) -> Result<(), String>
        + Send
        + 'static,
    ) -> Result<(), String> {
        if action.is_empty() || action.len() > 128 || action.chars().any(char::is_control) {
            return Err("invalid UI action name".into());
        }
        if self.handlers.contains_key(&action) {
            return Err("duplicate UI action handler".into());
        }
        if self.handlers.len() >= self.handler_capacity {
            return Err("UI handler capacity exceeded".into());
        }
        self.handlers.insert(action, Box::new(handler));
        Ok(())
    }
    /// Whether application logic owns this named action.
    #[must_use]
    pub fn handles(&self, action: &str) -> bool {
        self.handlers.contains_key(action)
    }
    fn valid(scene: &SceneGraph, event: &UiActionEvent) -> Result<bool, String> {
        if !scene
            .active_in_hierarchy(event.owner)
            .map_err(|e| e.to_string())?
        {
            return Ok(false);
        }
        let Some(element) = scene
            .component::<UiElement>(event.owner)
            .map_err(|e| e.to_string())?
        else {
            return Ok(false);
        };
        if !element.valid() || !element.enabled || element.action.as_ref() != Some(&event.action) {
            return Ok(false);
        }
        let mut parent = scene.parent(event.owner).map_err(|e| e.to_string())?;
        while let Some(owner) = parent {
            if scene
                .component::<UiElement>(owner)
                .map_err(|e| e.to_string())?
                .is_some_and(|element| !element.valid() || !element.enabled)
            {
                return Ok(false);
            }
            parent = scene.parent(owner).map_err(|e| e.to_string())?;
        }
        Ok(true)
    }
    /// Queues an activation for the owning scene barrier.
    /// # Errors
    /// Rejects a foreign scene, invalid/disabled target or event capacity overflow.
    pub fn enqueue(&mut self, scene: &SceneGraph, event: UiActionEvent) -> Result<(), String> {
        if scene.identity() != self.scene {
            return Err("UI events belong to another scene".into());
        }
        if !Self::valid(scene, &event)? {
            return Err("UI action target is no longer enabled".into());
        }
        if self.events.len() >= self.event_capacity {
            return Err("UI event capacity exceeded".into());
        }
        self.events.push_back(event);
        Ok(())
    }
    /// Dispatches once at the owner-side frame barrier. Scene access is read-only;
    /// successful callback commands are admitted as one batch. Failed callbacks
    /// discard their staged scene changes. Captured application state is owned by
    /// the callback and is not rolled back on error.
    /// # Errors
    /// A foreign scene rejects the batch. Individual results report expired
    /// targets, missing handlers, callback errors and command admission failures.
    pub fn dispatch(
        &mut self,
        scene: &SceneGraph,
        commands: &mut SceneCommands,
    ) -> Result<Vec<Result<(), String>>, String> {
        if scene.identity() != self.scene {
            return Err("UI events belong to another scene".into());
        }
        if commands.scene_identity() != self.scene {
            return Err("UI command queue belongs to another scene".into());
        }
        let mut results = Vec::new();
        while let Some(event) = self.events.pop_front() {
            let result = (|| {
                if !Self::valid(scene, &event)? {
                    return Err("UI action target expired before dispatch".into());
                }
                let handler = self
                    .handlers
                    .get_mut(&event.action)
                    .ok_or_else(|| format!("unhandled UI action: {}", event.action))?;
                let mut staged = SceneCommands::new(scene, self.command_capacity);
                handler(scene, &event, &mut staged)?;
                commands.append(&mut staged).map_err(|e| e.to_string())
            })();
            results.push(result);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::{SceneCommand, Transform};
    fn fixture() -> (SceneGraph, UiActionEvent) {
        let mut scene = SceneGraph::new(2);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                UiElement {
                    origin: [0.0; 2],
                    size: [1.0; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("hide".into()),
                    text: None,
                },
            )
            .unwrap();
        (
            scene,
            UiActionEvent {
                owner,
                action: "hide".into(),
            },
        )
    }
    #[test]
    fn application_callback_runs_once_at_barrier_and_rejects_expired_or_foreign_events() {
        let (mut scene, event) = fixture();
        let mut actions = UiActionHandlers::new(&scene, 1, 1, 2);
        actions
            .register("hide".into(), |_, event, commands| {
                commands
                    .push(SceneCommand::SetActive(event.owner, false))
                    .map_err(|e| e.to_string())
            })
            .unwrap();
        assert!(actions.register("hide".into(), |_, _, _| Ok(())).is_err());
        actions.enqueue(&scene, event.clone()).unwrap();
        assert!(actions.enqueue(&scene, event.clone()).is_err());
        let mut commands = SceneCommands::new(&scene, 2);
        assert!(actions.dispatch(&scene, &mut commands).unwrap()[0].is_ok());
        assert!(scene.active_in_hierarchy(event.owner).unwrap());
        assert!(actions.dispatch(&scene, &mut commands).unwrap().is_empty());
        commands
            .apply(&mut scene)
            .unwrap()
            .into_iter()
            .for_each(|r| {
                r.unwrap();
            });
        assert!(!scene.active_in_hierarchy(event.owner).unwrap());
        scene.set_active(event.owner, true).unwrap();
        actions.enqueue(&scene, event.clone()).unwrap();
        scene.remove_component::<UiElement>(event.owner).unwrap();
        assert!(actions.dispatch(&scene, &mut commands).unwrap()[0].is_err());
        let other = SceneGraph::new(1);
        assert!(actions.enqueue(&other, event).is_err());
        assert!(actions.dispatch(&other, &mut commands).is_err());
    }
    #[test]
    fn foreign_command_queue_rejects_before_callback_and_preserves_pending_event() {
        let (mut scene, event) = fixture();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let captured = std::sync::Arc::clone(&calls);
        let mut actions = UiActionHandlers::new(&scene, 1, 1, 1);
        actions
            .register("hide".into(), move |_, event, commands| {
                captured.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                commands
                    .push(SceneCommand::SetActive(event.owner, false))
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        actions.enqueue(&scene, event.clone()).unwrap();
        let other = SceneGraph::new(1);
        let mut foreign = SceneCommands::new(&other, 1);
        assert!(actions.dispatch(&scene, &mut foreign).is_err());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        let mut commands = SceneCommands::new(&scene, 1);
        assert!(actions.dispatch(&scene, &mut commands).unwrap()[0].is_ok());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        for result in commands.apply(&mut scene).unwrap() {
            result.unwrap();
        }
        assert!(!scene.active_in_hierarchy(event.owner).unwrap());
        assert!(actions.dispatch(&scene, &mut commands).unwrap().is_empty());
    }
    #[test]
    fn failed_handler_and_full_barrier_discard_all_staged_scene_commands() {
        let (mut scene, event) = fixture();
        for fail in [true, false] {
            let mut actions = UiActionHandlers::new(&scene, 1, 1, 2);
            actions
                .register("hide".into(), move |_, event, commands| {
                    commands
                        .push(SceneCommand::SetActive(event.owner, false))
                        .map_err(|e| e.to_string())?;
                    if fail {
                        Err("application failure".into())
                    } else {
                        Ok(())
                    }
                })
                .unwrap();
            actions.enqueue(&scene, event.clone()).unwrap();
            let mut commands = SceneCommands::new(&scene, 0);
            assert!(actions.dispatch(&scene, &mut commands).unwrap()[0].is_err());
            assert!(commands.apply(&mut scene).unwrap().is_empty());
            assert!(scene.active_in_hierarchy(event.owner).unwrap());
        }
    }
}

/// Creates fresh application handlers for each Play session. Captured configuration
/// persists; state captured by newly registered callbacks belongs to that session.
pub struct UiActionSetup(Box<dyn FnMut(&mut UiActionHandlers) -> Result<(), String> + Send>);
impl std::fmt::Debug for UiActionSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiActionSetup").finish_non_exhaustive()
    }
}
impl UiActionSetup {
    pub fn new(
        setup: impl FnMut(&mut UiActionHandlers) -> Result<(), String> + Send + 'static,
    ) -> Self {
        Self(Box::new(setup))
    }
    pub fn configure(&mut self, handlers: &mut UiActionHandlers) -> Result<(), String> {
        (self.0)(handlers)
    }
}
