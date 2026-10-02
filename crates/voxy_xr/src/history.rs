//! Temporal invalidation at the runtime's predicted display time.
use crate::XrSessionEvent;

/// Resets an application-owned combined motion/exposure history on XR lifecycle
/// changes. Reference-space changes take effect at their runtime timestamp,
/// rather than when the event is polled. Recreate this owner when selecting a
/// different tracking-space type; explicit teleports/swapchain changes still
/// require clearing application history.
#[derive(Debug)]
pub struct XrHistoryReset {
    space: openxr::ReferenceSpaceType,
    pending: Vec<openxr::Time>,
    previous_predicted: Option<openxr::Time>,
}
impl XrHistoryReset {
    #[must_use]
    pub fn new(space: openxr::ReferenceSpaceType) -> Self {
        Self {
            space,
            pending: Vec::new(),
            previous_predicted: None,
        }
    }

    /// Apply an already routed session event. Lost events conservatively reset
    /// history because missed lifecycle/origin changes cannot be reconstructed.
    pub fn handle_event<T>(&mut self, event: XrSessionEvent, history: &mut Option<T>) {
        match event {
            XrSessionEvent::ReferenceSpaceChange {
                kind, change_time, ..
            } if kind == self.space => {
                if !self.pending.contains(&change_time) {
                    self.pending.push(change_time);
                }
            }
            XrSessionEvent::InstanceLoss { .. } | XrSessionEvent::EventsLost(_) => {
                self.reset(history);
            }
            XrSessionEvent::StateChanged { state, teardown }
                if teardown
                    || matches!(
                        state,
                        openxr::SessionState::READY
                            | openxr::SessionState::STOPPING
                            | openxr::SessionState::IDLE
                            | openxr::SessionState::LOSS_PENDING
                            | openxr::SessionState::EXITING
                    ) =>
            {
                self.reset(history);
            }
            _ => {}
        }
    }

    /// Call before preparing either eye at the pending frame's predicted time.
    /// `tracking_valid` must include both position and orientation for both eyes.
    /// Returns whether a reset was required, even when history was already empty.
    /// Backward predicted time invalidates history; equal time permits frame retry.
    pub fn begin_frame<T>(
        &mut self,
        predicted: openxr::Time,
        tracking_valid: bool,
        history: &mut Option<T>,
    ) -> bool {
        let before = self.pending.len();
        self.pending
            .retain(|time| time.as_nanos() > predicted.as_nanos());
        let backwards = self
            .previous_predicted
            .is_some_and(|previous| predicted.as_nanos() < previous.as_nanos());
        self.previous_predicted = Some(predicted);
        let reset = backwards || !tracking_valid || self.pending.len() != before;
        if reset {
            *history = None;
        }
        reset
    }

    /// Explicit discontinuity: session/space/swapchain replacement or teleport.
    pub fn reset<T>(&mut self, history: &mut Option<T>) {
        self.pending.clear();
        self.previous_predicted = None;
        *history = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn time(n: i64) -> openxr::Time {
        openxr::Time::from_nanos(n)
    }
    fn change(n: i64, kind: openxr::ReferenceSpaceType) -> XrSessionEvent {
        XrSessionEvent::ReferenceSpaceChange {
            kind,
            change_time: time(n),
            pose_in_previous_space: None,
        }
    }
    #[test]
    fn origin_changes_reset_at_each_effective_frame_not_poll_time() {
        let mut resets = XrHistoryReset::new(openxr::ReferenceSpaceType::LOCAL);
        let mut history = Some(([1, 2], 3));
        resets.handle_event(change(20, openxr::ReferenceSpaceType::LOCAL), &mut history);
        resets.handle_event(change(10, openxr::ReferenceSpaceType::LOCAL), &mut history);
        resets.handle_event(change(10, openxr::ReferenceSpaceType::LOCAL), &mut history);
        resets.handle_event(change(5, openxr::ReferenceSpaceType::STAGE), &mut history);
        assert!(!resets.begin_frame(time(9), true, &mut history));
        assert_eq!(history, Some(([1, 2], 3)));
        assert!(resets.begin_frame(time(10), true, &mut history));
        assert_eq!(history, None);
        history = Some(([4, 5], 6));
        assert!(!resets.begin_frame(time(19), true, &mut history));
        assert!(resets.begin_frame(time(21), true, &mut history));
        assert_eq!(history, None);
        history = Some(([7, 8], 9));
        assert!(!resets.begin_frame(time(22), true, &mut history));
        assert!(history.is_some());
    }
    #[test]
    fn tracking_and_lifecycle_reset_combined_history() {
        let mut resets = XrHistoryReset::new(openxr::ReferenceSpaceType::LOCAL);
        let mut history = Some(1);
        assert!(resets.begin_frame(time(1), false, &mut history));
        assert_eq!(history, None);
        for event in [
            XrSessionEvent::EventsLost(1),
            XrSessionEvent::InstanceLoss { loss_time: time(9) },
            XrSessionEvent::StateChanged {
                state: openxr::SessionState::STOPPING,
                teardown: false,
            },
        ] {
            history = Some(2);
            resets.handle_event(event, &mut history);
            assert_eq!(history, None);
        }
        history = Some(3);
        resets.handle_event(XrSessionEvent::InteractionProfileChanged, &mut history);
        assert_eq!(history, Some(3));
    }

    #[test]
    fn backward_time_resets_but_retry_and_future_origin_remain_valid() {
        let mut resets = XrHistoryReset::new(openxr::ReferenceSpaceType::LOCAL);
        let mut history = Some(1);
        resets.handle_event(change(30, openxr::ReferenceSpaceType::LOCAL), &mut history);
        assert!(!resets.begin_frame(time(20), true, &mut history));
        assert!(!resets.begin_frame(time(20), true, &mut history));
        assert_eq!(history, Some(1));
        assert!(resets.begin_frame(time(19), true, &mut history));
        assert_eq!(history, None);
        history = Some(2);
        assert!(!resets.begin_frame(time(29), true, &mut history));
        assert!(resets.begin_frame(time(30), true, &mut history));
        assert_eq!(history, None);
        resets.reset(&mut history);
        history = Some(3);
        assert!(!resets.begin_frame(time(1), true, &mut history));
        assert_eq!(history, Some(3));
    }
}
