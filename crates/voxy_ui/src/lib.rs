//! Platform-neutral pointer routing over painter-ordered logical hit regions.
//! Layout, rendering and native event adapters remain separate owners.
mod scroll;
pub use scroll::ScrollPanel;
mod layout;
pub use layout::{Axis, LayoutItem, Length, layout_linear};
mod focus;
pub use focus::{FocusRouter, KeyAction};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WidgetId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRegion {
    pub id: WidgetId,
    pub origin: [f32; 2],
    pub size: [f32; 2],
    /// Disabled regions still occlude lower regions but cannot capture presses.
    pub enabled: bool,
}
impl HitRegion {
    /// Intersects this region with a logical panel rectangle. Empty/touching
    /// intersections return None; the surviving region retains widget identity.
    /// # Errors
    /// Rejects invalid source or clip geometry.
    pub fn clipped(self, origin: [f32; 2], size: [f32; 2]) -> Result<Option<Self>, UiError> {
        let clip = Self {
            origin,
            size,
            ..self
        };
        if !self.valid() || !clip.valid() {
            return Err(UiError::InvalidGeometry);
        }
        let start = std::array::from_fn(|i| self.origin[i].max(origin[i]));
        let end: [f32; 2] =
            std::array::from_fn(|i| (self.origin[i] + self.size[i]).min(origin[i] + size[i]));
        if (0..2).any(|i| end[i] <= start[i]) {
            return Ok(None);
        }
        Ok(Some(Self {
            origin: start,
            size: std::array::from_fn(|i| end[i] - start[i]),
            ..self
        }))
    }
    fn valid(self) -> bool {
        (0..2).all(|i| {
            self.origin[i].is_finite()
                && self.size[i].is_finite()
                && self.size[i] > 0.0
                && (self.origin[i] + self.size[i]).is_finite()
        })
    }
    fn contains(self, point: [f32; 2]) -> bool {
        (0..2).all(|i| point[i] >= self.origin[i] && point[i] < self.origin[i] + self.size[i])
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiError {
    Capacity,
    InvalidGeometry,
    DuplicateId,
    UnknownWidget,
    InsufficientSpace,
}
impl std::fmt::Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UI error: {self:?}")
    }
}
impl std::error::Error for UiError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerAction {
    None,
    Press(WidgetId),
    Release { id: WidgetId, clicked: bool },
    Cancel(WidgetId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerResult {
    pub consumed: bool,
    pub action: PointerAction,
}
/// One primary pointer/button. IDs must stay unique across widget lifetimes;
/// reusing an ID for a different widget while captured is caller error.
#[derive(Debug)]
pub struct PointerRouter {
    regions: Vec<HitRegion>,
    capacity: usize,
    position: Option<[f32; 2]>,
    captured: Option<WidgetId>,
    held: bool,
}
impl PointerRouter {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            regions: Vec::new(),
            capacity,
            position: None,
            captured: None,
            held: false,
        }
    }
    /// Replaces painter-ordered regions atomically; last region is topmost.
    /// Returns cancellation if the captured widget disappears or is disabled.
    /// # Errors
    /// Rejects capacity, invalid geometry and duplicate IDs before any mutation.
    pub fn set_regions(&mut self, regions: &[HitRegion]) -> Result<Option<WidgetId>, UiError> {
        if regions.len() > self.capacity {
            return Err(UiError::Capacity);
        }
        let mut ids = BTreeSet::new();
        for region in regions {
            if !region.valid() {
                return Err(UiError::InvalidGeometry);
            }
            if !ids.insert(region.id) {
                return Err(UiError::DuplicateId);
            }
        }
        let cancelled = self
            .captured
            .filter(|id| !regions.iter().any(|r| r.id == *id && r.enabled));
        if cancelled.is_some() {
            self.captured = None;
        }
        self.regions.clear();
        self.regions.extend_from_slice(regions);
        Ok(cancelled)
    }
    fn hit(&self) -> Option<HitRegion> {
        self.position.and_then(|point| {
            self.regions
                .iter()
                .rev()
                .find(|r| r.contains(point))
                .copied()
        })
    }
    #[must_use]
    pub fn hovered(&self) -> Option<WidgetId> {
        self.hit().filter(|r| r.enabled).map(|r| r.id)
    }
    #[must_use]
    pub fn captured(&self) -> Option<WidgetId> {
        self.captured
    }
    /// Coordinates are logical viewport-local pixels. None represents pointer leave.
    /// Nonfinite coordinates are treated as outside rather than routed to widgets.
    pub fn move_to(&mut self, point: Option<[f32; 2]>) -> bool {
        self.position = point.filter(|p| p.iter().all(|x| x.is_finite()));
        self.captured.is_some() || self.hit().is_some()
    }
    /// Repeated down events preserve capture and never issue another press.
    pub fn press(&mut self) -> PointerResult {
        if self.held {
            return PointerResult {
                consumed: self.captured.is_some() || self.hit().is_some(),
                action: PointerAction::None,
            };
        }
        self.held = true;
        let hit = self.hit();
        if let Some(region) = hit.filter(|r| r.enabled) {
            self.captured = Some(region.id);
            return PointerResult {
                consumed: true,
                action: PointerAction::Press(region.id),
            };
        }
        PointerResult {
            consumed: hit.is_some(),
            action: PointerAction::None,
        }
    }
    /// Release goes to the captured widget; click requires it to be topmost under
    /// the pointer at release. Dragging outside or under an overlay cancels click.
    pub fn release(&mut self) -> PointerResult {
        self.held = false;
        if let Some(id) = self.captured.take() {
            let clicked = self.hovered() == Some(id);
            return PointerResult {
                consumed: true,
                action: PointerAction::Release { id, clicked },
            };
        }
        PointerResult {
            consumed: self.hit().is_some(),
            action: PointerAction::None,
        }
    }
    /// Native focus loss clears pointer/capture and issues cancellation, never click.
    pub fn cancel(&mut self) -> PointerResult {
        self.position = None;
        self.held = false;
        let captured = self.captured.take();
        PointerResult {
            consumed: captured.is_some(),
            action: captured.map_or(PointerAction::None, PointerAction::Cancel),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn region(id: u64) -> HitRegion {
        HitRegion {
            id: WidgetId(id),
            origin: [0.0; 2],
            size: [10.0; 2],
            enabled: true,
        }
    }
    #[test]
    fn clipping_limits_hit_area_and_hides_fully_outside_regions() {
        let original = HitRegion {
            id: WidgetId(1),
            origin: [-5.0; 2],
            size: [20.0; 2],
            enabled: true,
        };
        let clipped = original.clipped([0.0; 2], [10.0; 2]).unwrap().unwrap();
        let mut router = PointerRouter::new(1);
        router.set_regions(&[clipped]).unwrap();
        router.move_to(Some([-1.0, 5.0]));
        assert!(!router.press().consumed);
        router.release();
        router.move_to(Some([5.0; 2]));
        assert_eq!(router.press().action, PointerAction::Press(WidgetId(1)));
        assert!(original.clipped([15.0; 2], [5.0; 2]).unwrap().is_none());
        assert_eq!(
            original.clipped([0.0; 2], [0.0; 2]),
            Err(UiError::InvalidGeometry)
        );
    }
    #[test]
    fn capture_drag_leave_overlap_and_focus_cancel() {
        let mut router = PointerRouter::new(3);
        router.set_regions(&[region(1), region(2)]).unwrap();
        router.move_to(Some([5.0; 2]));
        assert_eq!(router.press().action, PointerAction::Press(WidgetId(2)));
        assert_eq!(router.press().action, PointerAction::None);
        router.move_to(None);
        assert_eq!(
            router.release().action,
            PointerAction::Release {
                id: WidgetId(2),
                clicked: false
            }
        );
        router.move_to(Some([5.0; 2]));
        router.press();
        router
            .set_regions(&[region(1), region(2), region(3)])
            .unwrap();
        assert_eq!(
            router.release().action,
            PointerAction::Release {
                id: WidgetId(2),
                clicked: false
            }
        );
        router.press();
        assert_eq!(router.cancel().action, PointerAction::Cancel(WidgetId(3)));
        assert_eq!(router.release().action, PointerAction::None);
    }
    #[test]
    fn held_pointer_cannot_capture_a_new_widget_after_removal() {
        let mut router = PointerRouter::new(1);
        router.set_regions(&[region(1)]).unwrap();
        router.move_to(Some([1.0; 2]));
        router.press();
        assert_eq!(router.set_regions(&[region(2)]), Ok(Some(WidgetId(1))));
        assert_eq!(router.press().action, PointerAction::None);
        assert_eq!(router.captured(), None);
        router.release();
        assert_eq!(router.press().action, PointerAction::Press(WidgetId(2)));
    }
    #[test]
    fn disable_remove_atomic_update_and_half_open_edges() {
        let mut router = PointerRouter::new(2);
        router.set_regions(&[region(1)]).unwrap();
        router.move_to(Some([10.0, 0.0]));
        assert!(!router.press().consumed);
        router.release();
        router.move_to(Some([0.0; 2]));
        router.press();
        assert_eq!(
            router.set_regions(&[region(1), region(1)]),
            Err(UiError::DuplicateId)
        );
        assert_eq!(router.captured(), Some(WidgetId(1)));
        assert_eq!(router.set_regions(&[]), Ok(Some(WidgetId(1))));
        let mut disabled = region(2);
        disabled.enabled = false;
        router.set_regions(&[region(1), disabled]).unwrap();
        assert!(router.press().consumed);
        assert_eq!(router.captured(), None);
        let mut invalid = region(3);
        invalid.size = [f32::NAN; 2];
        assert_eq!(
            router.set_regions(&[invalid]),
            Err(UiError::InvalidGeometry)
        );
        assert_eq!(router.hovered(), None);
    }
}
