//! Single-window frame host for the examples: the window lifecycle and the
//! error path that every example repeated around its own frame code.
//!
//! The host creates the window on the first resume, forwards events to the
//! example, records the first error, leaves the event loop and returns that
//! error from [`run_window_frame`]. It interprets no event itself: close,
//! resize, redraw pacing and smoke exits stay with the example, because the
//! examples differ in exactly those.
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

/// Result of one phase of a hosted example.
pub type FrameResult = Result<(), Box<dyn std::error::Error>>;

/// The phase in which a hosted example failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStage {
    /// Window or renderer creation in [`WindowFrame::start`].
    Start,
    /// Any later event, including the redraw.
    Event,
}

/// What an example supplies to [`run_window_frame`].
pub trait WindowFrame {
    /// Title, size and visibility of the window; read once.
    fn window_attributes(&self) -> WindowAttributes;

    /// Creates the surface and the scene for the new window. Runs once.
    fn start(&mut self, event_loop: &ActiveEventLoop, window: &Arc<Window>) -> FrameResult;

    /// Every window event, including close, resize and redraw.
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: &Arc<Window>,
        event: WindowEvent,
    ) -> FrameResult;

    /// Raw device input; ignored unless overridden.
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window: &Arc<Window>,
        _event: DeviceEvent,
    ) -> FrameResult {
        Ok(())
    }

    /// End of an event batch; requests the next frame unless overridden.
    fn about_to_wait(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window: &Arc<Window>,
    ) -> FrameResult {
        window.request_redraw();
        Ok(())
    }

    /// Called once with the first error, before the event loop exits.
    fn report(&self, _stage: FrameStage, _error: &str) {}
}

/// Runs `frame` in one window until it exits the event loop. `control_flow`
/// overrides the event loop default when given. Returns the first error of
/// the example, as its message.
pub fn run_window_frame<F: WindowFrame>(
    frame: &mut F,
    control_flow: Option<ControlFlow>,
) -> FrameResult {
    let event_loop = EventLoop::new()?;
    if let Some(control_flow) = control_flow {
        event_loop.set_control_flow(control_flow);
    }
    let mut host = Host {
        frame,
        window: None,
        failure: None,
    };
    event_loop.run_app(&mut host)?;
    match host.failure {
        Some(failure) => Err(failure.into()),
        None => Ok(()),
    }
}

struct Host<'a, F> {
    frame: &'a mut F,
    window: Option<Arc<Window>>,
    failure: Option<String>,
}

/// Keeps the first failure; returns its message when `result` is that failure.
fn first_failure(failure: &mut Option<String>, result: FrameResult) -> Option<&str> {
    let error = result.err()?;
    if failure.is_some() {
        return None;
    }
    Some(failure.insert(error.to_string()))
}

impl<F: WindowFrame> Host<'_, F> {
    fn settle(&mut self, event_loop: &ActiveEventLoop, stage: FrameStage, result: FrameResult) {
        if let Some(error) = first_failure(&mut self.failure, result) {
            self.frame.report(stage, error);
            event_loop.exit();
        }
    }

    fn live_window(&self) -> Option<Arc<Window>> {
        if self.failure.is_some() {
            return None;
        }
        self.window.clone()
    }
}

impl<F: WindowFrame> ApplicationHandler for Host<'_, F> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() || self.failure.is_some() {
            return;
        }
        let result = event_loop
            .create_window(self.frame.window_attributes())
            .map_err(Into::into)
            .and_then(|window| {
                let window = Arc::new(window);
                self.frame.start(event_loop, &window)?;
                self.window = Some(window);
                Ok(())
            });
        self.settle(event_loop, FrameStage::Start, result);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(window) = self.live_window() {
            let result = self.frame.window_event(event_loop, &window, event);
            self.settle(event_loop, FrameStage::Event, result);
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let Some(window) = self.live_window() {
            let result = self.frame.device_event(event_loop, &window, event);
            self.settle(event_loop, FrameStage::Event, result);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.live_window() {
            let result = self.frame.about_to_wait(event_loop, &window);
            self.settle(event_loop, FrameStage::Event, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_failure_is_kept_and_reported() {
        let mut failure = None;
        assert_eq!(first_failure(&mut failure, Ok(())), None);
        assert_eq!(failure, None);
        assert_eq!(
            first_failure(&mut failure, Err("surface lost".into())),
            Some("surface lost")
        );
        assert_eq!(first_failure(&mut failure, Err("later".into())), None);
        assert_eq!(first_failure(&mut failure, Ok(())), None);
        assert_eq!(failure.as_deref(), Some("surface lost"));
    }
}
