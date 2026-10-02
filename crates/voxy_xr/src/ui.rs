//! VR panel input adapter using the engine's existing pointer capture semantics.
use crate::QuadHit;
use voxy_ui::{PointerAction, PointerResult, PointerRouter, UiError};

/// One controller drives one primary UI pointer. Keep its identity stable;
/// cancel before switching controllers, sessions, panels or reference spaces.
#[derive(Debug, Default)]
pub struct XrPanelPointer {
    armed: bool,
}
impl XrPanelPointer {
    /// Routes a synced controller action and its located aim through the common
    /// UI router. Supply the hit calculated for this same aim/predicted frame.
    /// Action inactivity or missing position/orientation cancels capture.
    /// # Errors
    /// Rejects invalid logical extent or hit coordinates before mutation.
    pub fn update_hand(
        &mut self,
        pointer: &mut PointerRouter,
        hit: Option<QuadHit>,
        hand: &crate::HandInput,
        aim: &openxr::SpaceLocation,
        logical_extent: [f32; 2],
    ) -> Result<PointerResult, UiError> {
        let flags = openxr::SpaceLocationFlags::POSITION_VALID
            | openxr::SpaceLocationFlags::ORIENTATION_VALID;
        let available =
            hand.aim_active && hand.select.is_active && aim.location_flags.contains(flags);
        self.update(
            pointer,
            hit,
            available.then_some(hand.select.current_state),
            logical_extent,
        )
    }

    /// Maps top-left normalized panel coordinates to logical UI pixels.
    /// `select` is None if aim tracking or the select action is unavailable;
    /// that cancels capture without clicking. A tracked ray missing the panel
    /// uses `hit = None` with Some(button), retaining normal drag-out semantics.
    /// After creation/loss/cancellation, a released button must be observed
    /// before a press can capture. A held trigger during tracking recovery
    /// therefore cannot produce an unintended click.
    /// # Errors
    /// Rejects invalid extent/hit coordinates before modifying pointer state.
    pub fn update(
        &mut self,
        pointer: &mut PointerRouter,
        hit: Option<QuadHit>,
        select: Option<bool>,
        logical_extent: [f32; 2],
    ) -> Result<PointerResult, UiError> {
        if logical_extent.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || hit.is_some_and(|h| {
                h.uv.iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            })
        {
            return Err(UiError::InvalidGeometry);
        }
        let Some(down) = select else {
            return Ok(self.cancel(pointer));
        };
        let consumed = pointer
            .move_to(hit.map(|h| [h.uv[0] * logical_extent[0], h.uv[1] * logical_extent[1]]));
        if !self.armed {
            self.armed = !down;
            return Ok(PointerResult {
                consumed,
                action: PointerAction::None,
            });
        }
        Ok(if down {
            pointer.press()
        } else {
            pointer.release()
        })
    }

    /// Cancel without clicking and require a release before the next press.
    pub fn cancel(&mut self, pointer: &mut PointerRouter) -> PointerResult {
        self.armed = false;
        pointer.cancel()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_ui::{HitRegion, WidgetId};
    fn router() -> PointerRouter {
        let mut p = PointerRouter::new(1);
        p.set_regions(&[HitRegion {
            id: WidgetId(7),
            origin: [40.0, 20.0],
            size: [20.0, 10.0],
            enabled: true,
        }])
        .unwrap();
        p
    }
    fn hit() -> Option<QuadHit> {
        Some(QuadHit {
            uv: [0.5, 0.5],
            distance: 2.0,
        })
    }
    #[test]
    fn logical_mapping_click_and_drag_out() {
        let mut p = router();
        let mut adapter = XrPanelPointer::default();
        let size = [100.0, 50.0];
        adapter.update(&mut p, hit(), Some(false), size).unwrap();
        assert_eq!(
            adapter
                .update(&mut p, hit(), Some(true), size)
                .unwrap()
                .action,
            PointerAction::Press(WidgetId(7))
        );
        assert_eq!(
            adapter
                .update(&mut p, hit(), Some(true), size)
                .unwrap()
                .action,
            PointerAction::None
        );
        assert_eq!(
            adapter
                .update(&mut p, hit(), Some(false), size)
                .unwrap()
                .action,
            PointerAction::Release {
                id: WidgetId(7),
                clicked: true
            }
        );
        adapter.update(&mut p, hit(), Some(true), size).unwrap();
        assert_eq!(
            adapter
                .update(&mut p, None, Some(false), size)
                .unwrap()
                .action,
            PointerAction::Release {
                id: WidgetId(7),
                clicked: false
            }
        );
    }
    #[test]
    fn tracking_loss_cancels_and_held_recovery_never_captures() {
        let mut p = router();
        let mut adapter = XrPanelPointer::default();
        let size = [100.0, 50.0];
        assert_eq!(
            adapter
                .update(&mut p, hit(), Some(true), size)
                .unwrap()
                .action,
            PointerAction::None
        );
        adapter.update(&mut p, hit(), Some(false), size).unwrap();
        adapter.update(&mut p, hit(), Some(true), size).unwrap();
        assert_eq!(
            adapter.update(&mut p, None, None, size).unwrap().action,
            PointerAction::Cancel(WidgetId(7))
        );
        assert_eq!(
            adapter
                .update(&mut p, hit(), Some(true), size)
                .unwrap()
                .action,
            PointerAction::None
        );
        assert!(p.captured().is_none());
        adapter.update(&mut p, hit(), Some(false), size).unwrap();
        assert_eq!(
            adapter
                .update(&mut p, hit(), Some(true), size)
                .unwrap()
                .action,
            PointerAction::Press(WidgetId(7))
        );
        assert!(
            adapter
                .update(&mut p, hit(), Some(false), [f32::NAN, 50.0])
                .is_err()
        );
        assert_eq!(p.captured(), Some(WidgetId(7)));
    }
}
