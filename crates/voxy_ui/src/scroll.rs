use crate::{HitRegion, UiError};
/// Logical two-axis scroll position. Positive offset moves content left/up.
/// No inertia, overscroll, event transport or child hierarchy ownership.
#[derive(Clone, Copy, Debug)]
pub struct ScrollPanel {
    viewport: [f32; 2],
    content: [f32; 2],
    offset: [f32; 2],
}
impl ScrollPanel {
    /// # Errors
    /// Rejects nonfinite extents, nonpositive viewport or negative content size.
    pub fn new(viewport: [f32; 2], content: [f32; 2]) -> Result<Self, UiError> {
        validate(viewport, content)?;
        Ok(Self {
            viewport,
            content,
            offset: [0.0; 2],
        })
    }
    #[must_use]
    pub fn offset(&self) -> [f32; 2] {
        self.offset
    }
    /// Preserves offset where possible, then clamps to the new content bounds.
    /// # Errors
    /// Invalid geometry leaves all prior state intact.
    pub fn resize(&mut self, viewport: [f32; 2], content: [f32; 2]) -> Result<(), UiError> {
        validate(viewport, content)?;
        self.viewport = viewport;
        self.content = content;
        self.clamp();
        Ok(())
    }
    /// Scrolls by logical pixels and returns whether the offset changed.
    /// # Errors
    /// Rejects nonfinite deltas without changing state.
    #[allow(clippy::float_cmp)] // Exact finite position change drives geometry invalidation.
    #[allow(clippy::cast_possible_truncation)] // Sum clamped to finite f32 content bounds.
    pub fn scroll(&mut self, delta: [f32; 2]) -> Result<bool, UiError> {
        if delta.iter().any(|x| !x.is_finite()) {
            return Err(UiError::InvalidGeometry);
        }
        let before = self.offset;
        for (i, value) in delta.into_iter().enumerate() {
            self.offset[i] = (f64::from(self.offset[i]) + f64::from(value))
                .clamp(0.0, f64::from(self.maximum(i))) as f32;
        }
        Ok(before != self.offset)
    }
    /// Reveals a content-local rectangle with minimum movement. Oversized items
    /// align their leading edge; targets beyond content still clamp to bounds.
    /// # Errors
    /// Rejects invalid target rectangles without changing position.
    pub fn reveal(&mut self, target: HitRegion) -> Result<(), UiError> {
        if !target.valid() {
            return Err(UiError::InvalidGeometry);
        }
        for i in 0..2 {
            let end = target.origin[i] + target.size[i];
            if target.size[i] >= self.viewport[i] || target.origin[i] < self.offset[i] {
                self.offset[i] = target.origin[i];
            } else if end > self.offset[i] + self.viewport[i] {
                self.offset[i] = end - self.viewport[i];
            }
        }
        self.clamp();
        Ok(())
    }
    /// Projects a content region into viewport coordinates and clips its hit/render
    /// rectangle. The caller retains original geometry to crop texture UVs.
    /// # Errors
    /// Rejects invalid source/viewport origins or overflow in projected geometry.
    pub fn project(
        &self,
        region: HitRegion,
        viewport_origin: [f32; 2],
    ) -> Result<Option<HitRegion>, UiError> {
        if !region.valid() || viewport_origin.iter().any(|x| !x.is_finite()) {
            return Err(UiError::InvalidGeometry);
        }
        let origin =
            std::array::from_fn(|i| viewport_origin[i] + (region.origin[i] - self.offset[i]));
        HitRegion { origin, ..region }.clipped(viewport_origin, self.viewport)
    }
    fn maximum(&self, axis: usize) -> f32 {
        (self.content[axis] - self.viewport[axis]).max(0.0)
    }
    fn clamp(&mut self) {
        for i in 0..2 {
            self.offset[i] = self.offset[i].clamp(0.0, self.maximum(i));
        }
    }
}
fn validate(viewport: [f32; 2], content: [f32; 2]) -> Result<(), UiError> {
    if (0..2).all(|i| {
        viewport[i].is_finite() && viewport[i] > 0.0 && content[i].is_finite() && content[i] >= 0.0
    }) {
        Ok(())
    } else {
        Err(UiError::InvalidGeometry)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PointerRouter, WidgetId};
    #[test]
    fn bounds_resize_and_invalid_inputs() {
        let mut panel = ScrollPanel::new([100.0; 2], [300.0, 50.0]).unwrap();
        panel.scroll([500.0; 2]).unwrap();
        assert!((panel.offset[0] - 200.0).abs() < f32::EPSILON);
        assert!(panel.offset[1].abs() < f32::EPSILON);
        assert_eq!(panel.scroll([f32::NAN; 2]), Err(UiError::InvalidGeometry));
        assert_eq!(
            panel.resize([0.0; 2], [10.0; 2]),
            Err(UiError::InvalidGeometry)
        );
        assert!((panel.offset[0] - 200.0).abs() < f32::EPSILON);
        panel.resize([100.0; 2], [120.0; 2]).unwrap();
        assert!((panel.offset[0] - 20.0).abs() < f32::EPSILON);
        panel.scroll([-f32::MAX; 2]).unwrap();
        assert!(panel.offset.iter().all(|x| x.abs() < f32::EPSILON));
    }
    #[test]
    fn reveal_and_project_use_the_same_visible_hit_rectangle() {
        let mut panel = ScrollPanel::new([100.0; 2], [100.0, 300.0]).unwrap();
        let target = HitRegion {
            id: WidgetId(1),
            origin: [0.0, 180.0],
            size: [80.0, 40.0],
            enabled: true,
        };
        assert!(panel.project(target, [10.0; 2]).unwrap().is_none());
        panel.reveal(target).unwrap();
        assert!((panel.offset[1] - 120.0).abs() < f32::EPSILON);
        let visible = panel.project(target, [10.0; 2]).unwrap().unwrap();
        assert!((visible.origin[1] - 70.0).abs() < f32::EPSILON);
        let mut pointer = PointerRouter::new(1);
        pointer.set_regions(&[visible]).unwrap();
        pointer.move_to(Some([20.0, 90.0]));
        assert!(pointer.press().consumed);
        panel.resize([100.0; 2], [0.0; 2]).unwrap();
        assert!(panel.offset.iter().all(|x| x.abs() < f32::EPSILON));
    }
}
